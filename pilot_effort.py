"""Test 3 reasoning effort levels for step-3.7-flash."""
import os
import json
import urllib.request
import time

api_key = os.environ['STEPFUN_API_KEY']
url = 'https://api.stepfun.com/step_plan/v1/chat/completions'

def call(msg, content, max_tokens=2000, effort='medium'):
    body = json.dumps({
        'model': 'step-3.7-flash',
        'messages': [{'role': 'user', 'content': msg}],
        'max_tokens': max_tokens,
        'reasoning_effort': effort,
    }).encode('utf-8')
    req = urllib.request.Request(url, data=body, headers={'Authorization': f'Bearer {api_key}', 'Content-Type': 'application/json'}, method='POST')
    t0 = time.time()
    with urllib.request.urlopen(req, timeout=180) as resp:
        data = json.loads(resp.read().decode('utf-8'))
    print(f'  {content}: effort={effort} latency={time.time()-t0:.1f}s finish={data["choices"][0]["finish_reason"]} '
          f'completion={data["usage"]["completion_tokens"]} content_len={len(data["choices"][0]["message"]["content"])}')
    c = data['choices'][0]['message']['content']
    if c:
        print(f'    content: {repr(c[:200])}')
    return data

print('=== Test 1: 1+1, all 3 efforts ===')
for effort in ['low', 'medium', 'high']:
    call('1+1=?', f'1+1-{effort}', max_tokens=200, effort=effort)

print('\n=== Test 2: simple Q&A, low effort ===')
call('Say hello in Chinese', 'hello-cn-low', max_tokens=200, effort='low')

print('\n=== Test 3: god-object split (943 lines), all 3 efforts ===')
with open(r'E:\agent-project\northing\src\crates\execution\tool-execution\src\search\grep_search.rs', encoding='utf-8') as f:
    file_content = f.read()
prompt = f'File: grep_search.rs 943 lines. Reply with ONLY a 4-column markdown table (filename | sub-domain | line range | 1-line desc) for splitting into 4 siblings. No code. Just the table.\n\n```rust\n{file_content}\n```'
for effort in ['low', 'medium', 'high']:
    call(prompt, f'split-{effort}', max_tokens=4000, effort=effort)