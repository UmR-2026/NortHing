# W22-1 Brief — P2-3 窄义版：ContextCompression 事件可见化（live 提示）

## 任务标识

W22-1（技术债清账；P2-3 窄义版：压缩事件 live 提示。历史落痕不做——那是 P2-5 波的事）。2026-09-09 triage 实证（explore 子代理，HEAD `fff7535`）。

## BASE

`fff7535`（main HEAD，工作树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit，与 fff7535 代码树等价，仅 docs 差异）。

## 背景与 BASE 证据（triage 已核实，行号为 BASE 实测）

- 事件定义：`src/crates/contracts/events/src/agentic.rs:184-213`（`ContextCompressionStarted/Completed/Failed`，`Completed` 带 token 前后计数）。
- 发射点齐全：`src/crates/assembly/core/src/agentic/execution/compress_run.rs:56-66,175-189,196-204,229-241,257-271,376,414`（auto 压缩路径真实发射）。
- **缺口 1（kernel 桥丢弃）**：`src/crates/assembly/core/src/kernel_facade/events.rs:298` 的 `agentic_event_to_dtos` 把压缩事件落入 `_ => vec![]` 丢弃。
- **缺口 2（desktop 零处理）**：desktop Dioxus 全仓 compression 零命中。
- **缺口 3（CLI 丢弃）**：`src/apps/cli/src/modes/chat/run.rs:382` 与 `src/apps/cli/src/modes/exec.rs:358` 的事件 match 落入 `_ => {}`。
- CLI `tool_cards.rs:107,522` / `theme.rs:684` 的 "ContextCompression" 卡片是**死映射**（无活生产者，见 triage），本单不管它。

## 允许文件集

- `src/crates/assembly/core/src/kernel_facade/events.rs`（修改：事件映射 + 单元测试）
- `src/apps/desktop/src/ui_dioxus/app.rs`（修改：事件环 :212 catch-all 前加 Banner 臂 + 临时 banner signal，参照同文件 degraded 模式 :183/:187）
- `src/apps/desktop/src/ui_dioxus/turn_banner.rs`（可选修改：仅当抽 helper 时动；allowlist 超列安全——未改项 task-gate 只出 warning）
- `src/apps/cli/src/modes/chat/run.rs`（修改：一处打印）
- `src/apps/cli/src/modes/exec.rs`（修改：一处打印）
- `docs/status/tech-debt-ledger.md`（修改：P2-3 条目 status 翻转——家规 2 同 commit）

report 写 `.superpowers/sdd/w22-1-report.md`，不进本单验收 diff（不 commit、不 git add）。

## 功能要求

1. **kernel 桥映射（变体钉死）**：`agentic_event_to_dtos` 为三个压缩事件加映射，**一律复用既有 `KernelEventDto::Banner { level, message }`**（kernel-api/src/events.rs:95-98，全仓零发射点无消费者冲突）。**禁加新变体**——KernelEventDto 是 FROZEN 契约（Schema §5）且 kernel-api 在本单 allowlist 外。映射钉死：
   - Started → `Banner { Info, "Context compression started" }`
   - Completed → `Banner { Info, "Context compressed: {tokens_before} → {tokens_after} tokens" }`（载荷真实字段名 tokens_before/tokens_after）
   - Failed → `Banner { Error, "Context compression failed: {reason}" }`
2. **desktop banner**：`app.rs` 事件环加 Banner 臂，显示临时 banner（参照 degraded signal 模式）。**仅在 `streaming == true` 时展示**（Banner 无 session_id，跨会话压缩不弹前台横幅——钉死口径，app.rs 已有 streaming signal）。不阻塞输入、不改动消息列表。
3. **CLI 打印**：`run.rs` 与 `exec.rs` 的 `_ => {}` 前加压缩事件分支，文案与映射 1 相同。**exec.rs 仅 `print_text`，JSON emit 不做**（窄义钉死）。**全英文输出**（日志纪律）。
4. **测试**：`agentic_event_to_dtos` 的压缩事件映射加一个单元测试（三事件 → 预期 Banner DTO）。desktop/CLI 显示侧靠 judge review（家规 4 不触发）。
5. **ledger 翻转**：P2-3 status → `resolved`（注明窄义口径：live 提示已通（kernel 桥 Banner + desktop streaming banner + CLI 打印），历史落痕归 P2-5；同 commit）。

## Constraints

- commit 逐文件点名 `git add`（禁 -A）；message 前缀 `feat(events):` + `(W22-1)` 后缀，body 注明 P2-3 窄义清账。
- report 贴原文输出 + exit code；结尾状态词。
- 行为变化集中在事件映射与显示，禁改 compress_run.rs 发射逻辑、禁改事件定义载荷（`contracts/events` 不动——若发现非动不可，BLOCKED 上交）。
- 远程兼容家规：新 DTO 走 kernel 桥即天然经 event 通道，无需远程特判；若实现中发现远程 surface 有独立事件消费点需要同步处理，BLOCKED 上交而非扩面。

## 禁区

- 禁动 `src/crates/contracts/events/`（事件定义）。
- 禁动 compress_run.rs。
- 禁做历史落痕 / system message 插入（P2-5 范围）。
- 禁动 CLI tool_cards 死映射（单独账，不在本单）。
- 禁动清单外文件。

## 验证

1. `cargo check --workspace` → 绿（BASE 处 Rust 树与 CI run 34341758420 全绿时逐字相同，无需重跑 BASE）。
2. 新增单元测试跑绿：`cargo test -p northhing-core --features product-full <test_name>`——**必须带 `--features product-full`**（kernel_facade 模块在 `#[cfg(feature = "product-full")]` 门后，默认 feature 为空，不带则测试不参与编译、绿是假绿）；report 贴测试名与结果。
3. `cargo check -p northhing`（desktop 编译闸，家规 6）。
4. CLI 侧改动以 `cargo check -p northhing-cli` 覆盖。
5. report 附：三事件 Banner 映射文案原文 + desktop streaming 门控说明 + CLI 两种输出形态（run/exec 各打印、exec 无 JSON emit）。

## skill 前置

无强相关；如参考事件流可读 `docs/architecture/` 就近文档，不因此扩展任务范围。

## 报告

写 `.superpowers/sdd/w22-1-report.md`，必含三节：**改动摘要**（含 desktop 文件选择理由）/ **验证**（5 项原文输出 + exit code）/ **状态**（状态词结尾）。
