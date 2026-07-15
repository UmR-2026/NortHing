"""Step-3.7-flash planner: low effort, small prompt, just sub-domain plan."""
import os
import json
import urllib.request
import time

api_key = os.environ['STEPFUN_API_KEY']
url = 'https://api.stepfun.com/step_plan/v1/chat/completions'

def plan_only(file_path, output_path, effort='low'):
    with open(file_path, encoding='utf-8') as f:
        content = f.read()
    lines = content.splitlines()
    if len(lines) > 800:
        # Truncate to first 600 + last 100
        content = '\n'.join(lines[:600] + ['', f'... [TRUNCATED {len(lines)-700} lines] ...', ''] + lines[-100:])

    prompt = f"""Analyze this Rust file and output ONLY a markdown table with 4 columns: sibling filename | sub-domain | line range (estimate) | 1-line description. NO code, NO explanation, just the table. Then a 2nd table: mod.rs contents summary (which sibling modules + which re-exports).

File: {os.path.basename(file_path)} ({len(lines)} lines, truncated to 800 lines below)

```rust
{content}
```"""
    body = json.dumps({
        'model': 'step-3.7-flash',
        'messages': [{'role': 'user', 'content': prompt}],
        'max_tokens': 16000,
        'reasoning_effort': effort,
    }).encode('utf-8')
    req = urllib.request.Request(url, data=body, headers={'Authorization': f'Bearer {api_key}', 'Content-Type': 'application/json'}, method='POST')
    t0 = time.time()
    with urllib.request.urlopen(req, timeout=180) as resp:
        data = json.loads(resp.read().decode('utf-8'))
    latency = time.time() - t0
    finish = data['choices'][0]['finish_reason']
    ctn = data['usage']['completion_tokens']
    content_out = data['choices'][0]['message']['content']
    print(f'  effort={effort} latency={latency:.1f}s finish={finish} completion={ctn} content_len={len(content_out)}')
    if content_out:
        with open(output_path, 'w', encoding='utf-8') as f:
            f.write(content_out)
        print(f'  saved to {output_path}')
        print(f'  preview: {content_out[:200]}')
    else:
        print('  ERROR: empty content')

if __name__ == '__main__':
    import sys
    file_path = sys.argv[1]
    output_path = sys.argv[2]
    effort = sys.argv[3] if len(sys.argv) > 3 else 'low'
    plan_only(file_path, output_path, effort)