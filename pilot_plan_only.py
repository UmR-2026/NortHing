"""Pilot test: plan-only prompt for step-3.7-flash on grep_search.rs 943 lines."""
import os
import json
import urllib.request
import time

api_key = os.environ['STEPFUN_API_KEY']
url = 'https://api.stepfun.com/step_plan/v1/chat/completions'

with open(r'E:\agent-project\northing\src\crates\execution\tool-execution\src\search\grep_search.rs', encoding='utf-8') as f:
    file_content = f.read()

body = json.dumps({
    'model': 'step-3.7-flash',
    'messages': [
        {'role': 'system', 'content': 'You are a Rust refactoring planner. Reply with ONLY a 4-column markdown table listing each sibling file, its sub-domain, line range, and 1-line description. No code. No explanation. Just the table.'},
        {'role': 'user', 'content': f"""File: grep_search.rs 943 lines. Sub-domain hint: search_engine, search_query, search_state, search_types.

```rust
{file_content}
```

Reply with just the table."""}
    ],
    'max_tokens': 2000
}).encode('utf-8')

req = urllib.request.Request(url, data=body, headers={'Authorization': f'Bearer {api_key}', 'Content-Type': 'application/json'}, method='POST')
t0 = time.time()
with urllib.request.urlopen(req, timeout=180) as resp:
    data = json.loads(resp.read().decode('utf-8'))
print(f'latency: {time.time()-t0:.1f}s')
print(f'finish: {data["choices"][0]["finish_reason"]}')
print(f'completion_tokens: {data["usage"]["completion_tokens"]}')
print(f'reasoning_tokens: {data["usage"]["completion_tokens_details"]["reasoning_tokens"]}')
print('---')
content = data['choices'][0]['message']['content']
print(f'content_len: {len(content)}')
print(content)