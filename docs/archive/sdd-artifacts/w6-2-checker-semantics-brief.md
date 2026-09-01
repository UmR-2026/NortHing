# Task Brief — W6-2: rot 检查器语义修正（checker-semantics-rebase）

仓库：E:\agent-project\NortHing（main 分支）。范围仅 `scripts/` 三个文件。

## 授权链（先读）

1. 用户 2026-08-28 授权：技术细则由编排者+子代理决策闭环。
2. D1 独立仲裁：`.superpowers/sdd/w6-d1-checker-semantics-adjudication.md` = **APPROVE-FIX** + 3 附带条件。**修正的精确语义定义以该判决书为准，先读它再动手。**

## Spec

1. `scripts/verify-rot-budget.mjs` `collectRustFiles`：新增排除 ①文件名恰为 `tests.rs`；②任何目录段以 `_tests` 结尾。既有排除（`tests` 目录段、`_tests.rs` 后缀、target/node_modules）保持不变。不扩大范围（如 `test.rs` 单数、`.test.rs` 不处理——仲裁书若另有明确定义从其定义）。
2. `scripts/verify-rot-budget.test.mjs`：追加 2 条自测用例（`tests.rs` 文件被排除；`*_tests/` 目录下文件被排除），既有用例全绿。
3. `scripts/rot-budget.json`：5 条 grep-count 规则的 `note` 各追加一句语义重定基说明（格式对齐 `unix_epoch_inline` 的既有 rebase note 风格，注明 2026-08-28 + D1 仲裁）。**任何 ceiling 数值禁止改动**——diff 里 ceiling 数字必须逐字节一致。
4. 验收实测：`node scripts/verify-rot-budget.test.mjs` 绿 + `node scripts/verify-rot-budget.mjs` 退出码 0 且输出 readings：unwrap=473、expect=937、let_underscore=388、allow_dead_code=106、unix_epoch_inline 不变（≤69）。若读数与预期不符，STOP，BLOCKED 上报实际读数。
5. commit：恰好一个，消息含 `checker-semantics-rebase` 标记，对齐近期 git log 风格；不含 `.superpowers/`。

## Global Constraints（逐字遵守）

1. 分层边界：改动只在 `scripts/`；产品代码零改动。
2. SDD 禁区：禁止以任何 git 操作触碰 `.superpowers/`；report 用 write 工具写入 `.superpowers/sdd/w6-2-checker-semantics-report.md`。
3. **rot-budget 闸：本任务是闸本身的语义修正，已持 D1 仲裁授权；授权范围 = 排除规则 + note 追记 + 自测用例，超出即停。ceiling 数值零改动是硬红线。**
4. 验证最小集：上述 Spec 4 两条 node 命令 + `git diff` 自查（json 里 ceiling 数字不变）；命令与输出原文进 report。
5. 不新建无 owner 抽象；修改保持最小 diff。
6. 家规 2 doc sync：不改 crate 结构、不解 ledger 债项，无同步义务。

## 复用侦察（已完成，直接采信）

- 误计文件清单（21 个）与计数（unwrap 49 / expect 169 / let_ 2 / dead_code 0）已实测，见 W6 计划 `.superpowers/sdd/plan-2026-08-28-w6-rot-cleanup.md`。
- 语义对齐目标 = `scripts/check-repo-hygiene.mjs:90` testFilePattern 的 Rust 侧子集（`tests.rs` + `*_tests/`）。
- 预期修正后读数 = Spec 4 所列。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；需要澄清 = NEEDS_CONTEXT。
- 返回消息含：状态 / commit SHA / git show --stat / 验证命令+输出原文 / rot 读数前后对比 / json ceiling 未变的 diff 证据 / 偏离清单（无则写"无"）。
- 假汇报 = 停用：编排者将用磁盘 diff 与复跑逐条核对。
