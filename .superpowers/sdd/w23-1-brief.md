# W23-1 Brief — 文档/注释诚实化批（A1 分层表 + D3 验证表 + shell stub 立账 + cli-internal 注释）

## 任务标识

W23-1（防腐决策包 A 组文档批；依据 = ZCode 外部审查 2026-09-09 报告 A1/D3/Unguided#1/#2 + 用户拍板 2026-09-09）。纯文档/注释单，零行为变化。

## BASE

task-gate 起点 = 本 brief 定版后的 main HEAD（brief 定版 commit = `8de38f3` 之后的 BASE 行修订；**以编排者派发正文给出的 7 位 SHA 为准**，其后不得再有非本单允许文件集的 commit）。

## 背景与 BASE 证据（编排者已实测）

- **A1**：根 `AGENTS.md` 分层表（「Layered Module Index」，AGENTS.md:21-28 区域）缺 `src/crates/support` 层（workspace members 实有 `src/crates/support/test-support` + `src/crates/support/cli-internal`，编排者已核对 Cargo.toml members）；同时 contracts 层行缺 `kernel-api`/`disposable`、services 层行缺 `debug-log` 条目。`AGENTS-CN.md` 为镜像需同步。
- **D3**：AGENTS.md Verification 表「Shared Rust logic in `core`…」行推荐「nearest focused `cargo test`」，但 `cargo test -p northhing-core --lib` 无 feature 编译即死（E0433，`ai` 模块在 `ai-adapter-runtime` feature 门后；ZCode 实测 + W22 两单已踩过并绕证）。AGENTS.md 验证表自带授权：「凡本表与实际不符的条目，以实际为准并当场修正本表」。
- **shell_safety stub 立账**：`src/crates/assembly/core/src/agentic/tools/implementations/shell_safety.rs:223-249` `guard_command_execution` 确认段为 Phase 2 stub（audit 写 "allow-stub"，:238-239 自曝 Phase 3 未接线），台账无条目——违反「已知债入账」惯例。缓解事实（须写进条目）：管线层真实确认流在位（tool_confirmation.rs / exec_retry.rs / AND 语义 process_result.rs + 默认需确认），故实际风险低但 stub 必须入账。（编排者 2026-09-20 自审更正：原 brief 误写 services-core 路径，实际在 assembly/core/agentic/tools/implementations/ 下。）
- **cli-internal**：`src/crates/support/cli-internal/src/main.rs:1-10` 模块文档 SECURITY 段宣称 token 门控，实现 :33-51 是纯长度检查（:47-50 自曝 deferred；:215-218 Validate 子命令自曝 format-only）。注释向不存在的行为许愿。

## 允许文件集

- `AGENTS.md`（修改：分层表 + 验证表两处）
- `AGENTS-CN.md`（修改：镜像同步同两处）
- `docs/status/tech-debt-ledger.md`（修改：新增 P2-25 条目）
- `src/crates/support/cli-internal/src/main.rs`（修改：仅模块文档注释，零代码行改动）

report 写 `.superpowers/sdd/w23-1-report.md`，不进验收 diff。

## 功能要求

1. **分层表修正**（AGENTS.md + AGENTS-CN.md 同改）：
   - 表中补第 7 行 support 层：`src/crates/support`，Owns = 测试支撑与内部工具（豁免生产分层约束的辅助 crate），Modules = `test-support`、`cli-internal`，Layer doc = 无（或写「无就近 AGENTS.md」）。
   - contracts 层行 Modules 补 `kernel-api`、`disposable`；services 层行 Modules 补 `debug-log`；interfaces 层行核对 `acp` 在列（已在则不动）。
   - 表下加一句：「机械对照：以根 `Cargo.toml` workspace members + `scripts/core-boundaries/rules/crate-layout.mjs` 为准」。
2. **验证表修正**（双语同改）：「Shared Rust logic in `core`…」行的最小验证列改为 `cargo check --workspace`，定向测试处注明：**`northhing-core` 的定向测试必须带 `--features product-full`**（模块在 feature 门后，裸跑编译失败 E0433——ZCode 2026-09-09 实测）。
3. **P2-25 立账**（tech-debt-ledger.md P2 段末尾追加）：`shell_safety.rs`（`src/crates/assembly/core/src/agentic/tools/implementations/shell_safety.rs`）确认闸 Phase 2 stub（log-only "allow-stub"，Phase 3 未接线）。Symptom/Evidence（shell_safety.rs:223-249）/Proposed fix（Phase 3 接 request_user_confirmation）/缓解（管线层确认流真实在位：AND 语义 + 默认需确认，08-22 基线结论已被修正）/Status: active。
4. **cli-internal 注释诚实化**：SECURITY 段改为如实口径：「capability token gate is format-only (length ≥ 32); cryptographic validation deferred — see verify_capability_token」。注释改动，零代码。

## Constraints

- commit 逐文件点名 `git add`（4 文件）；message 前缀 `docs(agents):` + `(W23-1)` 后缀，body 注明 ZCode 审查 A1/D3/U#1/U#2 + 用户拍板 2026-09-09。
- 全部改动为文档/注释；禁改任何代码行、禁动清单外文件。
- report 贴验证输出 + exit code；结尾状态词。

## 验证（编排者已 BASE `0cac642` 预跑：2/2 命令绿）

1. `node scripts/check-repo-hygiene.mjs` → exit 0（BASE 实测绿：`Repository hygiene check passed (1 content files scanned, 3894 filenames checked)`）。
2. 分层表准确性：report 附对照说明（表列 crate 集合 vs `Cargo.toml` members 25 条逐一对账）。
3. `cargo check -p northhing-cli-internal`（rustup `stable-x86_64-pc-windows-msvc`）→ exit 0（BASE 实测绿，`Finished dev profile in 20.05s`；注释改动后复跑）。
4. report 附：P2-25 条目原文 + 四处改动摘要。

## 禁区

- 禁动 ci.yml、workflow-policy.json、任何 scripts/（那些是 W23-3）。
- 禁动 contracts/events、kernel_facade（那是 W23-2）。
- 禁把分层表改成「生成物」（单一源生成是 W24+ 的事，本单只做数据修正）。

## 报告

写 `.superpowers/sdd/w23-1-report.md`，三节：**改动摘要** / **验证**（4 项）/ **状态**。
