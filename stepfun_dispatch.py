#!/usr/bin/env python3
"""
Step-3.7-flash dispatcher for R44-R49 LOW/MED 难度 god-object splits.
- Reads STEPFUN_API_KEY from env (set by Mavis session, NEVER in file)
- Calls https://api.stepfun.com/step_plan/v1/chat/completions
- Reasoning model: needs max_tokens=4000
- Outputs: git mv + Edit operations via subprocess (NOT direct file write)
- Single-task mode: pass file path + target structure, get patch
"""
import os
import sys
import json
import subprocess
import time
import argparse
import re
from pathlib import Path

API_KEY = os.environ.get('STEPFUN_API_KEY')
BASE_URL = os.environ.get('STEPFUN_BASE_URL', 'https://api.stepfun.com/step_plan/v1')
MODEL = os.environ.get('STEPFUN_MODEL', 'step-3.7-flash')

if not API_KEY:
    print('ERROR: STEPFUN_API_KEY env var not set', file=sys.stderr)
    sys.exit(2)

REPO_ROOT = Path(r'E:\agent-project\northing')

def call_stepfun(prompt, system, max_tokens=4000, max_retries=3):
    """Call step-3.7-flash chat completion. Returns content string."""
    import urllib.request
    import urllib.error

    body = json.dumps({
        'model': MODEL,
        'messages': [
            {'role': 'system', 'content': system},
            {'role': 'user', 'content': prompt},
        ],
        'max_tokens': max_tokens,
    }).encode('utf-8')

    req = urllib.request.Request(
        f'{BASE_URL}/chat/completions',
        data=body,
        headers={
            'Authorization': f'Bearer {API_KEY}',
            'Content-Type': 'application/json',
        },
        method='POST',
    )

    for attempt in range(1, max_retries + 1):
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                data = json.loads(resp.read().decode('utf-8'))
                content = data['choices'][0]['message']['content']
                finish = data['choices'][0]['finish_reason']
                usage = data['usage']
                return {
                    'content': content,
                    'finish_reason': finish,
                    'usage': usage,
                    'attempt': attempt,
                }
        except urllib.error.HTTPError as e:
            err_body = e.read().decode('utf-8', errors='replace')
            print(f'  HTTP {e.code} (attempt {attempt}/{max_retries}): {err_body[:200]}', file=sys.stderr)
            if e.code in (429, 500, 502, 503, 504):
                time.sleep(2 ** attempt)
                continue
            raise
        except (urllib.error.URLError, TimeoutError) as e:
            print(f'  Network error (attempt {attempt}/{max_retries}): {e}', file=sys.stderr)
            time.sleep(2 ** attempt)
            continue

    raise RuntimeError(f'Failed after {max_retries} attempts')


def read_file_lines(path: Path):
    """Read file, return list of lines (UTF-8)."""
    with open(path, 'r', encoding='utf-8') as f:
        return f.read().splitlines()


def count_lines(path: Path):
    """Canonical wc-l: ReadAllLines().Count, NOT Measure-Object -Line."""
    return len(read_file_lines(path))


def get_git_head_file(rel_path: str):
    """Get file content from current HEAD (uncommitted changes preserved if path is on disk)."""
    abs_path = REPO_ROOT / rel_path
    if abs_path.exists():
        return read_file_lines(abs_path)
    return None


SYSTEM_PROMPT = """You are a Rust refactoring assistant specialized in god-object split.

CRITICAL RULES (R40-R49 review lessons):
1. NEVER use _lost_methods.rs placeholder
2. NEVER use part1.rs/part2.rs/part3.rs mechanical naming
3. Sub-domain naming: init.rs, state.rs, dispatch.rs, types.rs, helpers.rs, lifecycle.rs, etc.
4. mod.rs ≤ 600 lines, only `mod sibling;` declarations + `pub use super::*;` re-export
5. Each sibling ≤ 800 lines
6. Fields use `pub(super)` for cross-sibling access
7. Wildcard re-export: `pub use super::*;` in mod.rs
8. Preserve all `pub use` re-export paths (no cross-crate module reference changes)

OUTPUT FORMAT (mandatory):
1. Read input file structure
2. Reply with sibling list: each sibling's filename + line range + sub-domain name
3. Reply with mod.rs full content (re-export facade)
4. Reply with each sibling's full content
5. Reply with `git mv` commands to apply

Use the simplest possible split. Do not over-engineer. Reply in Chinese (mixed English for code identifiers is fine)."""


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--file', required=True, help='Relative path to god-object file from repo root')
    parser.add_argument('--crate', required=True, help='Crate name (e.g. northhing-core)')
    parser.add_argument('--sub-domain-hint', default='', help='Hint for sub-domain decomposition')
    parser.add_argument('--max-tokens', type=int, default=4000)
    parser.add_argument('--output', help='Save full response to file')
    args = parser.parse_args()

    target = REPO_ROOT / args.file
    if not target.exists():
        print(f'ERROR: file not found: {target}', file=sys.stderr)
        sys.exit(1)

    line_count = count_lines(target)
    print(f'File: {args.file}')
    print(f'Crate: {args.crate}')
    print(f'Line count: {line_count} (canonical wc-l)')

    # Read file content (truncate if too large to avoid token limit)
    lines = read_file_lines(target)
    max_input_lines = 1500  # roughly 60k tokens
    if len(lines) > max_input_lines:
        # send first + last 500 lines + middle sample
        head = lines[:500]
        tail = lines[-500:]
        mid_start = (len(lines) - 500) // 2
        middle = lines[mid_start:mid_start + 500]
        content = '\n'.join(head) + f'\n\n... [TRUNCATED {len(lines) - 1500} lines] ...\n\n' + '\n'.join(middle) + '\n\n... [TRUNCATED] ...\n\n' + '\n'.join(tail)
        print(f'Truncated to {max_input_lines} lines (sent head+mid+tail)')
    else:
        content = '\n'.join(lines)

    user_prompt = f"""# Task: split god-object {args.file} ({line_count} lines) into facade + N sibling

## Crate
`{args.crate}` (use `cargo check -p {args.crate}`)

## Iron rules (mandatory)
1. NO `_lost_methods.rs` placeholder — must assign every method to a sub-domain sibling
2. NO `part1.rs`/`part2.rs` mechanical naming — use sub-domain names (init/state/dispatch/types/helpers/lifecycle/...)
3. mod.rs ≤ 600 lines: only `mod sibling;` + `pub use super::*;` re-export + necessary struct definition
4. Each sibling ≤ 800 lines
5. Fields `pub(super)` for cross-sibling struct field access
6. Preserve all `pub use` re-export paths (consumer `use crate::path::...` must still work)
7. Wildcard re-export: `pub use super::*;` in mod.rs

## Sub-domain hint
{args.sub_domain_hint or '(none — analyze file structure yourself)'}

## File content
```rust
{content}
```

## Required output (reply in this order)
1. **Sibling plan** (table):
   | sibling filename | sub-domain | line range | brief description |
2. **mod.rs full content** (facade with mod declarations + re-exports)
3. **Each sibling's full content** (complete .rs file)
4. **git mv commands** to apply the split
5. **Verification commands**: `cargo check -p {args.crate}` + `cargo fmt -p {args.crate} -- --check`

Be specific, concrete, and complete. Include ALL `use` statements and `pub` modifiers."""

    print(f'\nCalling {MODEL}...')
    t0 = time.time()
    result = call_stepfun(user_prompt, SYSTEM_PROMPT, max_tokens=args.max_tokens)
    dt = time.time() - t0
    print(f'Done in {dt:.1f}s')
    print(f'Tokens: prompt={result["usage"]["prompt_tokens"]} completion={result["usage"]["completion_tokens"]} total={result["usage"]["total_tokens"]}')
    print(f'Finish: {result["finish_reason"]}')
    print(f'Attempts: {result["attempt"]}')

    output_text = result['content']
    if args.output:
        out_path = REPO_ROOT / args.output
        out_path.parent.mkdir(parents=True, exist_ok=True)
        with open(out_path, 'w', encoding='utf-8') as f:
            f.write(output_text)
        print(f'Output saved to {out_path}')

    # Print to stdout
    print('\n' + '=' * 80)
    print(output_text)
    print('=' * 80)


if __name__ == '__main__':
    main()