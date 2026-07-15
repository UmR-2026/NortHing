"""Test low effort + larger max_tokens for god-object split."""
import os
import json
import urllib.request
import time

api_key = os.environ['STEPFUN_API_KEY']
url = 'https://api.stepfun.com/step_plan/v1/chat/completions'

with open(r'E:\agent-project\northing\src\crates\execution\tool-execution\src\search\grep_search.rs', encoding='utf-8') as f:
    file_content = f.read()

# Truncate to 1500 lines for prompt
lines = file_content.splitlines()
if len(lines) > 1500:
    file_content = '\n'.join(lines[:1500])

prompt = f"""File: grep_search.rs (943 lines, full code below). Propose splitting into 4 sibling files (mod.rs + 3 siblings). Reply with ONLY:
1. Sub-domain table: sibling filename | line range | 1-line description
2. mod.rs full content (facade with re-exports)
3. Each sibling's full content (complete .rs file)
4. git mv + edit commands to apply

NO explanations outside the artifacts. Reply in Chinese (mixed English for code identifiers OK).

```rust
{file_content}
```"""

# Test low effort + max_tokens 8000, 16000
for mt, eff in [(8000, 'low'), (16000, 'low'), (32000, 'low'), (8000, 'medium')]:
    body = json.dumps({
        'model': 'step-3.7-flash',
        'messages': [{'role': 'user', 'content': prompt}],
        'max_tokens': mt,
        'reasoning_effort': eff,
    }).encode('utf-8')
    req = urllib.request.Request(url, data=body, headers={'Authorization': f'Bearer {api_key}', 'Content-Type': 'application/json'}, method='POST')
    t0 = time.time()
    try:
        with urllib.request.urlopen(req, timeout=300) as resp:
            data = json.loads(resp.read().decode('utf-8'))
        latency = time.time() - t0
        finish = data['choices'][0]['finish_reason']
        ctn = data['usage']['completion_tokens']
        rt = data['usage']['completion_tokens_details']['reasoning_tokens']
        content = data['choices'][0]['message']['content']
        print(f'\nmax_tokens={mt} effort={eff} latency={latency:.1f}s finish={finish} completion={ctn} reasoning={rt} content_len={len(content)}')
        if content:
            print(f'content (first 500):\n{content[:500]}')
    except Exception as e:
        print(f'\nmax_tokens={mt} effort={eff} ERROR: {e}')