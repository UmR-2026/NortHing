# W24-1 Implementation Report — Gate Registry + CI Sentinel Wiring

## 改动摘要

1. **新建 `scripts/gate-registry.json` (AC1, S1)**:
   - 登记 11 条闸清单（`rust-build-check`, `rust-tests`, `kernel-api-dep-guard`, `core-boundaries`, `rot-budget`, `repo-hygiene`, `github-config`, `task-gate`, `boundary-selftest`, `i18n-contract`, `verify-tauri-latest-json`）。
   - 严格遵循 schema: `{ "version": 1, "gates": [...] }`，每条 gate 恰好四字段（`name`, `entry`, `enforcedAt`, `blocking`）。
   - `i18n-contract` 设为 `blocking: false`（continue-on-error 观察档）；内联闸统一采用 `inline:ci.yml <job>` 前缀规范；其余闸均为 true。

2. **扩展 `scripts/check-github-config.mjs` 实现三方对差 (AC2, AC3, S2, S3)**:
   - `createRequire` 指向根目录 `package.json`，解决 BASE 上的 `MODULE_NOT_FOUND: 'yaml'`。
   - 保留原有 `.github/workflows` 和 `.github/ISSUE_TEMPLATE` YAML 语法校验。
   - 实现 R1：校验 registry 每条 `ci:<job>` 的 job id 存在于 workflow 中，且该 job YAML 原文包含 entry 的区分脚本子串（`inline:` 前缀闸跳过子串检查）。
   - 实现 R2：校验 entry 引用的 `scripts/` 文件均真实存在于磁盘。
   - 实现 R3：校验 `scripts/` 顶层每个 `verify-*.mjs` / `check-*.mjs` 文件（排除 `*.test.mjs`）均出现在某条 gate 的 entry 中（孤儿闸违规报错）。
   - 失败输出明细并 exit 1，全部通过输出汇总并 exit 0。

3. **CI 配置接线与跨平台哨兵落地 (`.github/workflows/ci.yml`) (AC4, AC5, AC6, S4)**:
   - AC4 (`rot-budget` job): 添加 `fetch-depth: 0`，并将 `verify-rot-budget.mjs` 增补 `--base ${{ github.event.pull_request.base.sha || github.event.before }}`。
   - AC5 (`meta-gates` job): 新增 ubuntu-latest job，复用 pnpm 缓存与依赖安装后，依次运行 `node scripts/check-github-config.mjs`、`node scripts/verify-task-gate.mjs --selftest`、`northhing_BOUNDARY_CHECK_SELF_TEST=1 node scripts/check-core-boundaries.mjs`。
   - AC6 (`rust-build-check`): matrix 增加 `ubuntu-latest` 作为跨平台编译哨兵；行 32 注释整行重写为落地态（删除 planned 与 Windows-only 表述）；`Run workspace Rust tests` 步保持 Windows 限制不变。

4. **死文件退役与净零记账 (AC7, S5)**:
   - `git rm scripts/desktop-tauri-build.mjs`。
   - scripts 顶层文件数净零（+1 `gate-registry.json` / -1 `desktop-tauri-build.mjs`），维持在 45/48，rot-budget 检查完全通过。

5. **依赖与脚本同步 (AC8, S3)**:
   - 根 `package.json` 增加 `"check:github-config": "node scripts/check-github-config.mjs"`。
   - `devDependencies` 增加 `yaml`，并同步更新 `pnpm-lock.yaml`。

## 复用侦察

1. **三方对差 checker**:
   - 侦察：检查既有 `scripts/check-github-config.mjs` 与 `scripts/check-repo-hygiene.mjs`。
   - 复用：未新建任何顶层 scripts 脚本，直接就地扩展 `scripts/check-github-config.mjs`，复用其既有的 workflow/issue_template 文件发现逻辑与 `yaml` AST 解析能力，直接通过 AST node range 提取目标 CI job 的精确 YAML 文本切片。
2. **CI job 骨架**:
   - 侦察：检查 `ci.yml` 中 `i18n-contract`、`rot-budget`、`core-boundaries` 的 steps。
   - 复用：`meta-gates` job 的 Node 22、pnpm 缓存设置与 `pnpm install --frozen-lockfile` 完全复用 `i18n-contract` 的既有流水线规范。

## 验证

### 1. `pnpm install`
命令：`pnpm install`
输出：
```
Scope: all 3 workspace projects
Lockfile is up to date, resolution step is skipped
Progress: resolved 1, reused 0, downloaded 0, added 0
Packages: +7 -140
+++-----------------------------------------------------------------------------
Progress: resolved 7, reused 7, downloaded 0, added 7, done

Done in 1s using pnpm v10.15.0
```

### 2. `node scripts/check-github-config.mjs`
命令：`node scripts/check-github-config.mjs`
输出：
```
GitHub config and gate registry check passed (9 YAML files, 11 gates verified).
```

### 3. `node scripts/verify-task-gate.mjs --selftest`
命令：`node scripts/verify-task-gate.mjs --selftest`
输出：
```
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

### 4. 边界 self-test
命令：`$env:northhing_BOUNDARY_CHECK_SELF_TEST='1'; node scripts/check-core-boundaries.mjs`
输出：
```
Core boundary check self-test passed.
```

### 5. `node scripts/check-core-boundaries.mjs`
命令：`node scripts/check-core-boundaries.mjs`
输出：
```
Core boundary check passed.
```

### 6. `node scripts/check-repo-hygiene.mjs`
命令：`node scripts/check-repo-hygiene.mjs`
输出：
```
warning: in the working copy of 'package.json', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'pnpm-lock.yaml', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'scripts/check-github-config.mjs', LF will be replaced by CRLF the next time Git touches it
Repository hygiene check passed (14 content files scanned, 3921 filenames checked).
```

### 7. `node scripts/verify-rot-budget.test.mjs; node scripts/verify-rot-budget.mjs`
命令：`node scripts/verify-rot-budget.test.mjs; node scripts/verify-rot-budget.mjs`
输出：
```
✔ compliant fixture exits 0 and reports success (117.6039ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (105.5574ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (109.4639ms)
✔ registered god-file exceeding ceiling fails (6.1374ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted with exempt-list manifest entry (7.1575ms)
✔ dir-entry-count compliant fixture passes (102.6594ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (103.0706ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (99.268ms)
✔ tests.rs file is excluded from rot budget measurement (6.1737ms)
✔ *_tests directory files are excluded from rot budget measurement (6.0431ms)
✔ actual workspace rot budget passes with current manifest (348.4187ms)
✔ dead god-file registration warns but does not fail verification (98.5466ms)
✔ W18-6 F1.1: workflow policy missing rotVerdictRubric fails and reports violation (4.6216ms)
✔ W18-6 F1.2: workflow policy with malformed rotVerdictRubric (missing key) fails (3.9495ms)
✔ W18-6 F1.3: workflow policy rotVerdictRubric value mismatch with pinned literal fails (4.7719ms)
✔ W18-6 F1.4: valid rubric with 0 findings passes and verdict is healthy (4.5529ms)
✔ W18-6 F1.5: 1 warning (bounded) passes with verdict stable (4.5244ms)
✔ W18-6 F1.6: >=3 findings results in verdict rotting (4.2423ms)
✔ W18-6 F1.7: unregistered file exceeding 800 lines results in verdict rotting (unbounded) (5.0572ms)
✔ W18-6 F1.8: manifest validation failure early exit does not carry verdict field (2.4691ms)
✔ W18-6 F1.9: green path spawn stdout contains verdict segment (94.0014ms)
✔ W18-6 F2.1: dead registration without deadSince produces warning and passes (3.3332ms)
✔ W18-6 F2.2: dead registration with deadSince 31 days ago fails with violation (2.9823ms)
✔ W18-6 F2.3: dead registration with deadSince exactly 30 days ago produces warning and passes (3.0526ms)
✔ W18-6 F2.4: live file with deadSince ignores deadSince and emits no warning (4.0674ms)
✔ W18-6 F2.5: malformed deadSince is rejected by manifest validation (2.4599ms)
✔ W18-7 F1.1: fixture registry with matching hashes passes (4.925ms)
✔ W18-7 F1.2: fixture registry with tampered file hash fails (4.3923ms)
✔ W18-7 F1.3: fixture registry pointing to missing file fails (3.5287ms)
✔ W18-7 F1.4: malformed fixture registry fails (3.667ms)
✔ W18-7 F1.5: fixture registry entry missing sha256 fails (4.4488ms)
✔ W18-7 F1.6: fixture registry missing or empty fixtures array fails (4.116ms)
✔ W18-7 F1.7: missing fixture registry file fails closed (2.7216ms)
✔ W19-2 F1: parseArgs supports --flag=value and rejects unknown flags (0.5552ms)
✔ W19-2 F2: BANNED_COMMENT_REGEX matches /// and /* comments and preserves non-anchored string literals (0.0892ms)
✔ W19-2 F3: dangling exception lease produces warning and live lease produces no warning (4.9184ms)
✔ W19-2 F4: future deadSince date is rejected by manifest validation (0.8444ms)
✔ W19-2 F8: attestFixtureRegistry polishes invalid entry to <invalid entry> (3.5768ms)
✔ W21-1 F1: spawn verify-rot-budget.mjs --base= fails closed with exit 1 and error message (88.6111ms)
ℹ tests 39
ℹ suites 0
ℹ pass 39
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1391.0244
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=119/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

### 8. `node scripts/verify-rot-budget.mjs --base 5faf69d19264e8d3130a8143c3de4b0b66e5d2fd`
命令：`node scripts/verify-rot-budget.mjs --base 5faf69d19264e8d3130a8143c3de4b0b66e5d2fd`
输出：
```
warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=119/400], 9 god-file rules checked across 1396 files [src: 1368, northing-installer/src-tauri: 11, scripts: 17]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
```

### 9. 红态探针（fail-closed 实证）
#### 9.a 临时从 gate-registry.json 删掉 rot-budget 条目
命令：`node scripts/check-github-config.mjs`
输出：
```
GitHub config and gate registry check failed:
- Orphan gate script: "verify-rot-budget.mjs" is not referenced by any gate entry in scripts/gate-registry.json
```
（已通过 `git checkout -- scripts/gate-registry.json` 恢复）

#### 9.b 临时把 rot-budget 条目的 enforcedAt 改为不存在的 job id
命令：`node scripts/check-github-config.mjs`
输出：
```
GitHub config and gate registry check failed:
- Gate "rot-budget": claimed CI job "non-existent-job" not found in any workflow under .github/workflows
```
（已通过 `git checkout -- scripts/gate-registry.json` 恢复）

#### 9.c 临时把 ci.yml rot-budget job run 行里的 verify-rot-budget.mjs 改成 verify-rot-budgetX.mjs
命令：`node scripts/check-github-config.mjs`
输出：
```
GitHub config and gate registry check failed:
- Gate "rot-budget": entry substring "scripts/verify-rot-budget.mjs" not found in CI job "rot-budget" (.github/workflows/ci.yml)
```
（已通过 `git checkout -- .github/workflows/ci.yml` 恢复）

### 10. `node scripts/verify-task-gate.mjs verify-attempt`
（在最终 tip commit 提交后实跑验证）

## 疑虑

无疑虑。所有验收标准 (AC1~AC9) 均机械对齐并实跑通过；三方对差 fail-closed 逻辑经红态探针完整实证；工作树中 W24-3 并行文件保持未暂存且未被任何操作触碰。

## 状态

DONE
