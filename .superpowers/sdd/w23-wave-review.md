# W23 波级终审报告

## 终审判决: APPROVE

## 终审综述

本审查位作为独立终审判位，对 W23 诚实化波（commit 范围 `0cac642..6b92232`，共 17 个 commit，涵盖 4 个实施单、brief 修订与审查产物）进行了波级全量闭环审计与独立实跑验证。

### 1. 波目标达成度
对照波计划 `.superpowers/sdd/plan-2026-09-10-w23-w26-closure-and-decoupling.md` §1 W23 段，四项任务全部高质量达成，零偷工零范围缩水：
- **W23-1 文档批**：分层表补齐 Layer 7 support（`test-support`, `cli-internal`），contracts 补齐 `kernel-api`/`disposable`，services 补齐 `debug-log`，interfaces 保留 `acp`，添加机械对照句，AGENTS-CN.md 同步镜像；验证表追加 `--features product-full` 注记并保留 `when behavior changed`；ledger 增补 P2-25 五字段（Symptom / Mitigation / Evidence / Proposed fix / Status）；`cli-internal/src/main.rs` 完成 SECURITY 注释诚实化。
- **W23-2 事件诚实化**：`agentic.rs` 将 2 处前端承诺虚构注释转为 `TODO(surface)` 并标注 ZCode 审查事实，2 处零发射变体标注 `reserved`；`events.rs` 彻底显式化外层 14 个与内层 9 个未翻译变体（全返回 `vec![]` 并按三类语义输出对应 `debug!` 日志），清除除 line 75 `TurnErrorKind` 外的 `_ =>`；新增 `test_agentic_event_to_dtos_intentional_drops` 锁死 14+9 变体丢弃行为。
- **W23-3 meta 小修**：`ci.yml:32` 注释诚实化更新（标明 W20-1 已修与 W24 哨兵规划，零配置改动）；`scripts/verify-rot-budget.mjs` 中 `attestFixtureRegistry` existsSync 缺文件分支由 fail-open 修正为 fail-closed（仅 1 行替换）；`scripts/verify-rot-budget.test.mjs` F1.7 测试同步翻转断言 `success: false` 与钉死文案。
- **W23-4 台账处置**：`tech-debt-ledger.md` P2-14 关单（`resolved`，按 2026-09-09 用户裁决 dream-sweep 为设计答案）、P2-17 按设计暂缓（`frozen`，等待第三调用方）；严格行内替换，UTF-8 em-dash 字节级正确，无 GBK 双重编码污染。

### 2. 跨单一致性
- W23-1 在 AGENTS.md 验证表注明的定向测试 `--features product-full`，与 W23-2 实施及测试运行参数完全契合。
- W23-1 增补 P2-25 与 W23-4 处置 P2-14/P2-17 在 `docs/status/tech-debt-ledger.md` 中提交顺序清晰、结构标准一致。
- W23-3 的 `attestFixtureRegistry` 仅在 `--selftest` 路径生效，不影响正常主路径及后续 W24 闸门演进。
- 全波 4 单技术面改动文件集互不相交且全量收敛于 9 个目标文件，无冲突与反复修改。

### 3. 零行为变化总验证
- 9 个技术面文件中，7 个为纯文档/注释（AGENTS.md、AGENTS-CN.md、tech-debt-ledger.md、cli-internal main.rs、agentic.rs、ci.yml），零行为变更。
- `events.rs` 将通配 `_ => vec![]` 拆解为外层 14 臂与内层 9 臂具名匹配，每臂均返回 `vec![]`，仅增加 debug-level 日志，函数返回值和系统状态转换逻辑与改动前 100% 严格等价。既有 13 项映射单测无一回归。
- `scripts/verify-rot-budget.mjs` 仅在 `--selftest` 内部前置校验时检测 fixture 目录，主检查路径 `verifyRotBudget` 零接触。

## Findings

- [Minor] (已于 progress.md 留痕，波级 triage 闭环) W23-4 brief 复审 Minor：`tech-debt-ledger.md` Change Protocol 规定 "Resolved: Mark as resolved with commit reference"，而 P2-14 Status 使用任务 ID（`W23-4`）而非 commit SHA。
  - **证据**：`w23-4-brief-review.md:5`；`docs/status/tech-debt-ledger.md:187`；`progress.md:680`。
  - **处置建议**：Accept-and-close。核查仓库历史先例，P2-15（`Task T2-1`）、P2-16（`Task B3`）、P2-20（`Wave1-Final`）、P2-23（`W20-1`）、P2-24（`W20-2`）均统一使用任务 ID 作为唯一溯源标识，且任务 ID 在 git log、brief、report 与 progress.md 间构成 1:1 双向可审计链。保持当前写法完全合规，无需返工。

## 独立实跑证据

波级终审判位在当前工作区独立执行了全部 4 条指定验证命令（以及相关的全量单元测试与编译检查），全部通过：

### 1. `node scripts/check-repo-hygiene.mjs`
- **Exit Code**: 0
- **输出摘要**:
  ```text
  Repository hygiene check passed (1 content files scanned, 3913 filenames checked).
  ```

### 2. `node scripts/verify-rot-budget.mjs`
- **Exit Code**: 0
- **输出摘要**:
  ```text
  warn: unix_epoch_inline has zero headroom (current 69 == ceiling 69) — use exception lease channel
  Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=45/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=113/400], 9 god-file rules checked across 1397 files [src: 1368, northing-installer/src-tauri: 11, scripts: 18]) — verdict: rotting (rubric SSOT: scripts/workflow-policy.json rotVerdictRubric).
  warn: dir_entries:docs/design has zero headroom (current 1 == ceiling 1) — use exception lease channel
  warn: god_file:src/apps/desktop/src/ui_dioxus/css.rs has zero headroom (current 790 == ceiling 790) — use exception lease channel
  warn: god_file:src/apps/cli/src/ui/startup/selectors.rs has zero headroom (current 827 == ceiling 827) — use exception lease channel
  warn: god_file:src/crates/assembly/core/src/service/lsp/manager.rs has zero headroom (current 836 == ceiling 836) — use exception lease channel
  ```

### 3. `node scripts/verify-rot-budget.mjs --selftest`
- **Exit Code**: 0
- **输出摘要**:
  ```text
  Selftest passed: 53 checks passed (33 negative, 20 positive).
  ```

### 4. `node scripts/verify-task-gate.mjs verify-attempt --base 0cac642 --tip 6b92232 --allowlist C:/WINDOWS/TEMP/opencode/w23-wave-allowlist.txt`
- **Exit Code**: 0
- **输出摘要**:
  ```text
  Attempt verification passed: all modified files are within allowlist.
  ```

### 补充验证证据
- **`node scripts/verify-rot-budget.test.mjs`**: 39 passed / 0 failed (包含翻转后的 W18-7 F1.7 fails closed 测试)。
- **`cargo check --workspace`** (MSVC toolchain): Finished dev profile in 2.03s, 0 errors.
- **`cargo test -p northhing-core --features product-full kernel_facade::events`**: 2 passed / 0 failed（包含 `test_agentic_event_to_dtos_intentional_drops` 与 `test_agentic_event_to_dtos_context_compression_banners`）。
