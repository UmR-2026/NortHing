# Review — P0c (F3-UI approval 卡接线 + 事件方向缺口补齐)

**Reviewer**: judge-m3
**Base**: `4b6a012` · **Head**: `a893a8a` · **Diff stat**: 5 source files, +120/-27, 2 task artifacts (+93/+828)
**Brief**: `task-p0c-approval-brief.md` · **Report**: `task-p0c-approval-report.md`
**Spec**: `prescription-v3-20260825.md` §F3 (P0a 桥接) + §F3-UI (P0c)

---

## 1. Constraint Verdicts

| # | Constraint | Verdict | Evidence |
|---|---|---|---|
| 1 | `ToolCallPhase` 追加 `AwaitingConfirmation`（additive, snake_case） | **PASS** | `events.rs:39-43` — `Started, Completed, AwaitingConfirmation`, `#[serde(rename_all = "snake_case")]` 保留。原两变体未删未改。derive 增 `Copy, PartialEq, Eq`（additive, 不影响 wire 兼容）。 |
| 2 | facade 映射 `ConfirmationNeeded` → `ToolCall{phase: AwaitingConfirmation}`，不发 `TurnPhase` | **PASS** | `kernel_facade/events.rs:278-292` — 新增 arm 紧贴 `_ => vec![]` 之前，仅产出 `vec![KernelEventDto::ToolCall(...)]`，无 `TurnPhase`。单测 `test_agentic_event_to_dtos_confirmation_needed_maps_to_awaiting_confirmation` 显式断言 `dtos.len() == 1`，把"不发 TurnPhase"落到测试里。 |
| 3 | facade 单测覆盖该映射 | **PASS** | `kernel_facade/tests.rs:195-221` — 断言 call_id/name/phase/session_id/turn_id/summary/detail/result_count 全字段。新增 `test_tool_call_phase_awaiting_confirmation_serde`（contracts events.rs:122-132）覆盖 snake_case tag 序列化/反序列化。 |
| 4 | `session_mock.rs MockEntry::Approval` 加 `call_id: String` | **PASS** | `session_mock.rs:32` 加 `call_id: String`；seed_session 第 74 / 82 行用 `"mock-call-1"` / `"mock-call-2"` 占位；新增 `test_seed_session_has_mock_approvals_with_call_ids` 断言两个 call_id 与 resolved 状态。 |
| 5 | app.rs：未决卡按 call_id 去重；按钮 spawn `api::respond_to_tool_confirmation` + 乐观本地 resolve；resolved=true 不绑事件 | **PASS** | `app.rs:140-159` 新增 AwaitingConfirmation arm — `entries_guard.iter().any(|e| match e { MockEntry::Approval { call_id, .. } => call_id == &tc.call_id, _ => false })` 全表扫 call_id（含已 resolved）去重。`app.rs:763-785` `handle_action` closure spawn 调用 `api::respond_to_tool_confirmation(&cid, approved)`，**仅在 `is_ok()`** 后写 `resolved = true` + state_text；resolved=true 分支保留为 `approval-card resolved`，无按钮 onclick（line 797-820）。 |
| 6 | 禁区四件：不动 ToolEventData / pipeline_pre / 协调器内部；不加 reject 文本框；不动 KernelEventDto 本体；不动 P0b send/streaming 路径 | **PASS** | (a) diffstat 仅触及 `events.rs`(kernel-api, 仅 ToolCallPhase) / `events.rs`(kernel_facade, 仅新 arm) / `tests.rs` / `session_mock.rs` / `app.rs` — pipeline_pre / 协调器 / ToolEventData 未动；(b) `render_entry` Approval 分支只渲染 approve/reject 两个 button，无 `<input>` / textarea；(c) `KernelEventDto` 枚举本体（events.rs:73-105）的变体集、字段、`#[serde(rename_all = "snake_case", tag = "kind")]` 一字未改；(d) P0b 提交的 send/stop/textChunk 流式路径（app.rs 第 130-139 行的 `TextChunk` 分支、第 510 行 `render_entries` 调用点签名仅加一个 Signal 参数）未被重写。 |

---

## 2. Skeptical Check Verdicts

### 2.1 serde 兼容性（additive 变体的双向兼容）
- **前向兼容**（新 binary 读旧数据）：`Started` / `Completed` 仍在；旧数据无新变体 → 无问题。✓
- **反向兼容**（旧 binary 读新数据）：默认 serde derive 收到未知 variant tag `awaiting_confirmation` → 反序列化失败。但本项目场景里 KernelEventDto 是 **in-memory 事件流**（kernel_facade events.rs `subscribe_events` callback 模型，非持久化），且 kernel 与 desktop 同包同版本发布。生产中"新 kernel + 旧 desktop"混合部署不存在。
- **风险等级**：低。**Minor** — 可在 `ToolCallPhase` 上加 `#[serde(other)]` 或 `#[default]` 作为纵深防御，但当前无需。

### 2.2 去重逻辑（重连 / 重放）
- `entries_guard.iter().any(...)` 是 **全表扫**，不区分 `resolved` 状态。即便同一 call_id 已有 resolved 卡，新事件仍被丢弃 → 防止 resolved 卡被同一 call_id 重新刷出"未决"覆盖。✓
- 但同一 call_id 在同一会话内若出现两次 AwaitingConfirmation（kernel bug 或重放），第二次会被静默吞掉。如果预期是"再次询问用户"则与设计冲突；如果预期是"幂等忽略"则与设计一致。spec §F3-UI 仅说"乐观更新"，无显式语义。call_id 由 kernel 在 tool execution 状态机内生成，**正常路径下每个 AwaitingConfirmation 对应唯一 call_id**。✓ 当前实现是合理的。

### 2.3 render_entry 签名变化（Signal<Vec<MockEntry>> 传入）的爆炸半径
- `grep render_entry` 全仓仅 4 处匹配，全部在 `app.rs`：1 个调用点（line 510）+ render_entries 定义（line 719）+ render_entries 内部 1 处转发（line 727）+ render_entry 定义（line 732）。无第三方调用方。✓ 爆炸半径封闭。

### 2.4 乐观更新的失败路径（respond 失败时是否回滚）
- 实现：`is_ok()` 才写 `resolved=true`；失败时不动 entries。
- 用户感知：按钮按下后若失败，**卡片停留在未决状态、按钮仍在**，用户可再点。无 toast / 日志 / 计数。
- 评估：spec §F3-UI 第 97 行原文"成功后本地把该条 resolved 置 true（乐观更新）"——spec 显式只要求成功路径，无失败反馈要求。**与 spec 一致**。但用户体验上是个空白。
- **Minor**：建议后续 P1+ 增加 `tracing::warn!` 或 toast 反馈，不在本 P0c 范围。

### 2.5 文档同步义务（house rule 2）
- AGENTS.md 家规 2："changing crate structure (add/remove crate, move paths) requires updating `docs/status/surfaces.md`"。
- 本次仅在既有 `ToolCallPhase` 枚举上 **追加一个变体**，未增删 crate、未移动路径。**不触发** surfaces.md 同步。
- `grep "ToolCallPhase|kernel-api/events|kernel_facade/events" surfaces.md` 零命中——surfaces.md 当前不列 enum 级契约。✓ 无需补救。

### 2.6 其他二级检查（顺手）
- **hardcoded 中文状态文本** (`"已授权操作"` / `"已拒绝操作"`，app.rs:806/810)：与 AGENTS.md "v0.1.0 status: Desktop UI uses hardcoded Chinese. i18n engineering is frozen" 一致。✓ 不算偏离。
- **P0a `respond_to_tool_confirmation` 测试**（kernel_facade tests.rs `test_respond_to_tool_confirmation_returns_runtime_err_before_init`）本次仍在 37/37 通过，未被新代码破坏。✓
- **P0b 既有 `test_event_channel_returns_receiver`** 仍在 ui_dioxus 12/12 通过。✓
- **`Confirmed` / `Rejected` 后续事件**（`ToolEventData::Confirmed` / `Rejected`，agentic.rs:372-379）：kernel 会在 `respond_to_tool_confirmation` 成功后发这两个事件；当前 facade 走 `_ => vec![]` 静默丢弃，UI 仅靠本地乐观 resolve。这是 spec §F3-UI 接受的设计（"乐观更新"），但若未来需要"以 kernel 状态为准"的强一致，需补映射。**Minor**，超出本任务范围。
- **derive 变更**：`Copy, PartialEq, Eq` 是为 `app.rs:141` 的 `tc.phase == ToolCallPhase::AwaitingConfirmation` 比较与 test 的 `assert_eq!` 服务。`Copy` 对纯 enum 是零成本；`PartialEq, Eq` 不引入运行时开销。✓ 合理。

---

## 3. 双判决

### SPEC 判决
- 6/6 constraint PASS。
- spec §F3-UI 全部要点（resolved==false 接线 / resolved==true 不绑事件 / 无 reject 文本框）落地。
- spec §F3（P0a 已落）相关 `respond_to_tool_confirmation` 路径未被破坏，37 + 12 测试仍全绿。

### QUALITY 判决
- 最小 diff：5 源文件 / +120/-27，加 `Signal<Vec<MockEntry>>` 参数透传到 render_entry 是 Dioxus 0.x 惯用法；未引入新结构体、新 helper、未发明抽象。
- 测试覆盖：3 个新单测（contracts serde round-trip / facade 映射断言 / seed call_id 断言），分别覆盖 3 个改动层。无冗余。
- 命名：`AwaitingConfirmation` 与 spec 描述对齐；handler closure `handle_action(approved, status)` 简洁。
- 失败路径：仅 spec-compliant，缺少 UX 反馈——已记录为 Minor。
- `Signal<Vec<MockEntry>>` 通过 render 树透传：Dioxus 0 中 `Signal` 句柄可 `clone()`，实现里 `let mut entries = entries;` 是为了把 `Signal` 移进 `spawn(async move)`；手法正确，无 shadow 风险（`Signal` 的写方法在 `Write` 阶段接管，不需要重 shadow 阻止误读）。
- 编译 / 测试：4 项验证全绿，时长合理（51s + 3m08s + 0.09s + 0.00s），warning 全部 pre-existing（与 base 对齐，无新增）。

---

## 4. Findings

### Critical
*(none)*

### Important
*(none)*

### Minor
- **M1**: `respond_to_tool_confirmation` 失败时无 UI 反馈（app.rs:769-782）——用户重试无感知。建议补 `tracing::warn!` 或错误条。spec 不要求；超出 P0c 范围，记入终审 triage。
- **M2**: `ToolCallPhase` 反序列化在收到未知 variant 时硬失败。建议加 `#[serde(other)]` 或 default arm 做纵深防御（kernel↔UI 版本漂移场景）。当前 in-memory + 同版本发布下风险为 0；列为纵深建议。
- **M3**: `Confirmed` / `Rejected` 后续事件在 facade 被 `_ => vec![]` 吞掉。当 kernel 内部状态与 UI 乐观态分歧时（罕见：用户批准但 tool 内部 timeout 触发 reject），UI 显示"已授权"但实际未执行。当前 spec 不要求双向校正；记录在案。

---

## 5. Final Verdict

**APPROVE**

- SPEC 判决：PASS（6/6）
- QUALITY 判决：PASS
- Verification：4/4 命令成功（`cargo check --workspace` 51.28s · `cargo check -p northhing --features ui-dioxus` 3m08s · `cargo test -p northhing-core --features product-full kernel_facade` 37 passed · `cargo test -p northhing --features ui-dioxus --lib ui_dioxus` 12 passed）
- Tests：3 新增单测全部 green，pre-existing 49 tests 全保留
- 0 Critical · 0 Important · 3 Minor（M1/M2/M3 均超出本任务范围，指向终审 triage）

无修复循环需求。下一步：ledger 追加 `Task P0c: complete`，触发终审（review-package MERGE_BASE HEAD）。