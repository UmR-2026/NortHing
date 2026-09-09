# W21-3 Report — workflow-policy.json 审查材料清单字段

## 改动摘要

- 修改 `scripts/workflow-policy.json`，在顶层 `reportRequiredSections` 字段后新增 `reviewPackageRequired` 字段：
  ```json
  "reviewPackageRequired": {
    "default": ["brief", "diff", "report"]
  },
  ```
- 缩进严格对齐既有风格（2 空格），符合 JSON 标准；未触碰其他任何字段或代码文件（仓内零代码改动）。
- 提交：`e52318f`（`ci(workflow-policy): add reviewPackageRequired field (W21-3)`），单文件点名 `git add`，不含 `-A`。

## 验证

### 1. `node scripts/verify-task-gate.mjs validate-policy`

```
$ node scripts/verify-task-gate.mjs validate-policy
Policy validation passed: E:\agent-project\northing\scripts\workflow-policy.json
```
Exit code: `0`

### 2. `node scripts/verify-rot-budget.mjs`

```
$ node scripts/verify-rot-budget.mjs
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=90/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```
Exit code: `0`（verdict: rotting 与 BASE 等价，7 条 warning 属设计内）

### 3. `node scripts/verify-task-gate.mjs --selftest`

```
$ node scripts/verify-task-gate.mjs --selftest
[PASS] negative fixture a: replay W15-1l real incident (detected out-of-bounds pages_archive.rs)
[PASS] negative fixture b: invalid git revision rejected
[PASS] negative fixture c: missing required section in brief rejected
[PASS] negative fixture d: unapproved exemption phrase rejected
[PASS] negative fixture e: prejudging reviewer phrase in prose rejected
[PASS] negative fixture f: bad policy missing required field rejected
[PASS] negative fixture g: policy enum mismatch rejected
[PASS] negative fixture h: rename source path omitted from allowlist rejected
[PASS] positive fixture 1: complete 8-file allowlist passes
[PASS] positive fixture 2: allowlist with unfulfilled file passes with warning
[PASS] positive fixture 3: w16-1-brief.md passes validate-brief
[PASS] positive fixture 4: default workflow-policy.json passes validate-policy
[PASS] positive fixture 5: rename with both source and destination in allowlist passes
[PASS] negative fixture i: brief mentions 续单 without BASE line fails
[PASS] positive fixture 6: brief mentions 后续单 with ## BASE title passes
[PASS] positive fixture 7: parseArgs supports --flag=value, -h, and rejects unknown flags
Selftest passed: 16 fixtures passed (9 negative, 7 positive).
```
Exit code: `0`

### 4. `node -e "JSON.parse(...)"` 合法性验证与新字段内容原文

```
$ node -e "const fs = require('fs'); const policy = JSON.parse(fs.readFileSync('scripts/workflow-policy.json', 'utf8')); console.log('Parsed successfully, reviewPackageRequired:'); console.log(JSON.stringify(policy.reviewPackageRequired, null, 2));"
Parsed successfully, reviewPackageRequired:
{
  "default": [
    "brief",
    "diff",
    "report"
  ]
}
```
Exit code: `0`

## 状态

DONE
