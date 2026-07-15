"""1+1 test for step-3.7-flash."""
import os
import json
import urllib.request
import time

api_key = os.environ['STEPFUN_API_KEY']
url = 'https://api.stepfun.com/step_plan/v1/chat/completions'

# Test 1: 1+1 with step_plan
body = json.dumps({
    'model': 'step-3.7-flash',
    'messages': [{'role': 'user', 'content': '1+1=?'}],
    'max_tokens': 100
}).encode('utf-8')
req = urllib.request.Request(url, data=body, headers={'Authorization': f'Bearer {api_key}', 'Content-Type': 'application/json'}, method='POST')
t0 = time.time()
with urllib.request.urlopen(req, timeout=30) as resp:
    data = json.loads(resp.read().decode('utf-8'))
print(f'=== /step_plan/v1 ===')
print(f'latency: {time.time()-t0:.1f}s')
print(f'finish: {data["choices"][0]["finish_reason"]}')
print(f'completion_tokens: {data["usage"]["completion_tokens"]}')
content = data['choices'][0]['message']['content']
print(f'content ({len(content)} chars): {repr(content)}')
reasoning = data['choices'][0]['message'].get('reasoning_content', '') or ''
print(f'reasoning_content ({len(reasoning)} chars): {repr(reasoning[:300])}')

# Test 2: same with /v1 (standard)
url2 = 'https://api.stepfun.com/v1/chat/completions'
try:
    req2 = urllib.request.Request(url2, data=body, headers={'Authorization': f'Bearer {api_key}', 'Content-Type': 'application/json'}, method='POST')
    with urllib.request.urlopen(req2, timeout=30) as resp:
        data2 = json.loads(resp.read().decode('utf-8'))
    print(f'\n=== /v1 ===')
    print(f'finish: {data2["choices"][0]["finish_reason"]}')
    print(f'completion_tokens: {data2["usage"]["completion_tokens"]}')
    content2 = data2['choices'][0]['message']['content']
    print(f'content ({len(content2)} chars): {repr(content2)}')
except urllib.error.HTTPError as e:
    print(f'\n=== /v1 HTTP {e.code}: {e.read().decode()[:200]} ===')