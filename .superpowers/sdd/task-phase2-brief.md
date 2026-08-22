# Task PHASE-2 Brief — 棘轮扩展（4 新计数条目 + crate 准入守卫 + checker 读数输出）

## 来源与验收标准

来源：GLM-5.3 咨询方案 Phase 2（编排者修正版）+ 本日流程校准教训（拧 ceiling 必须用 checker 读数）。

**验收**：Spec 1-4 落地 + 验证输出进 report。

## 编排者预检结论（2026-08-21 用 checker 口径实测，直接采信）

| 新指标 | 基线（ceiling） | 说明 |
|---|---|---|
| `allow_dead_code` | **111** | grep-count，pattern `allow\(dead_code\)`，R-13 家族 |
| `dir_entries:scripts` | **45** | 新 kind：目录条目计数 |
| `dir_entries:docs/design` | **3** | 同上（PHASE-1A 归档后现状） |
| `dir_entries:.superpowers/sdd` | **400** | 同上；**刻意留 5 余量**——每任务自然产 4-6 工件，见顶触发归档循环（非只降指标，是防沉积 cap；manifest note 里写明这个语义差异） |

C1（password_vault/keyring/mcp::auth unwrap 清零）**已核销**：T2-5（2026-08-20）实证三目标生产区零 unwrap，本任务不重做。

## 复用侦察（强制）

读：`scripts/verify-rot-budget.mjs` 全文 + `verify-rot-budget.test.mjs` + `scripts/core-boundaries/rules/crate-layout.mjs` + `crate-rules.mjs`（crate 断言先例）+ core-boundaries 自测结构。report 写「复用侦察」节。

## Spec（必须全部满足）

1. **checker 扩展**（`verify-rot-budget.mjs`）：
   - 新增 kind `dir-entry-count`：统计指定目录（相对仓库根，可含 `.` 开头的隐藏目录）顶层条目数（文件+目录，二选一写明）；目录不存在 = 违规（防"删了目录反而变绿"）。
   - 通过输出行追加各 grep 规则的**实测读数**（如 `passed (unwrap=511/513, expect=1093/1093, ...)`）——这是拧 ceiling 的防呆口径。
2. **rot-budget.json 新增 4 条目**（值逐字用预检表；note 写明语义：前三个 only-down；sdd 为 cap-and-archive）。
3. **crate 准入守卫**：在 core-boundaries 规则族中新增——根 Cargo.toml workspace members 里每个 crate 必须在 `docs/status/surfaces.md` 有行（按 crate 名或路径匹配，匹配规则写明）；`.superpowers/sdd` 等豁免不进 surfaces 的非产品成员若有，列豁免常量数组带注释。挂进 check-core-boundaries.mjs 主检查；**自测必须有"构造一个未登记 crate 的 fixture → 变红"用例**。
4. **文档同步**：surfaces.md 若因本任务发现漏登成员，同 commit 补登（不许为了让 gate 变绿而删成员）；家规 7 条目在 AGENTS.md/CN 追加一句"dir-entry-count 指标的 sdd 条目是 cap-and-archive 语义"。
5. 不顺手碰：i18n 冻结区、产品代码逻辑、既有 ceiling 数值（编排者收口拧）。

## Global Constraints（逐字遵守）

- 输出/注释 English-only、无 emoji。
- 自测覆盖：新 kind 的违规/合规两路 + crate 准入的红绿两路。
- 历史事故禁令：checker 计数口径变更必须与"编排者预检基线"一致（本会话已有一次口径失真事故）；非 ASCII 用 edit 工具。

## 验证（命令 + 输出都要进 report）

1. `node scripts/verify-rot-budget.mjs`（贴带读数的输出）
2. `node scripts/verify-rot-budget.test.mjs`（贴输出）
3. `node scripts/check-core-boundaries.mjs`（含自测，如有）
4. `pnpm run check:rot`
5. `cargo check --workspace`（MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`）
6. `git diff --stat`

## 报告

`.superpowers/sdd/task-phase2-report.md`：Spec 逐条、复用侦察节、两个"写明"点（目录计数口径/匹配规则）、验证输出尾部、偏离声明。最后消息以状态词开头。

## 派发元信息

- BASE `fe91147`；worktree `E:\agent-project\.worktrees\northing-p2`（分支 `feat/phase2-ratchet-0821`）
- commit message 后缀 `(PHASE-2)`；只 stage 你改的文件。
