# Task Brief — Audit I3：growth 蒸馏移出 turn 完成事件临界路径

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r2-core.md` Finding 3（Important）：

> Enqueue `DialogTurnCompleted` and fire the watchdog oneshot **before** the growth hooks, or spawn `append_episode_log_entry`/`append_facts_entry` onto a separate task so they don't sit on the completion path.

**编排者裁定（钉死）**：选前者（同任务内重排），**不起新任务**。审计已实证："The turn is already persisted and the scheduler already notified (that happens earlier in `persist_completed_dialog_turn`)"——finalize 只做 growth（episode log + facts 蒸馏 + dream sweep），不挡完成语义。

验收标准（逐条可机械核对）：

1. `DialogTurnCompleted` 入队与 watchdog oneshot `tx.send` 均发生在 `finalize_persisted_turn_in_workspace_if_needed(...).await` **之前**。
2. finalize 本身逻辑零改动（参数、语义、顺序不变，仅调用点后移）。
3. 失败路径（`persist_failed_dialog_turn` 分支）与成功路径产出 `workspace_turn_status` 的形状不变。
4. `cargo check --workspace` 与聚焦测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-27 @ c48e4a9 实时核实：

| 事实 | 锚点 |
|---|---|
| 当前顺序：finalize `.await` → `if let Some(completed_event) = workspace_turn_status.1 { enqueue }` → `tx.send(workspace_turn_status)`（move 语义） | `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs:352-368` |
| finalize 末位参数是 `workspace_turn_status.0.clone()`（:361）→ 重排后须在 `tx.send` move 前先 clone | 同上 |
| `tx` = 外层 watchdog `tokio::select!` 的 oneshot（:372-374 `result = rx`），send 越早 watchdog 越早退出 | `:371-392` |
| finalize/growth 内含 LLM 调用：distill 15s + dream 15s 超时上限（warn-only 有界） | `turn_persist.rs:472,561`；`service/agent_memory/distiller.rs:27`、`dream.rs:24` |
| 该块整体在 spawned task 内（:350-351 注释），finalize 后移不改变任务归属 | `:350-369` |
| growth 蒸馏是 growth-core 线功能（并行 session 的领土），本任务只动调用顺序，**不碰 growth 实现** | 编排者边界注记 |

## 3. 复用侦察（强制）

查 `finalize_persisted_turn_in_workspace_if_needed` 签名与全部调用点（预期仅此一处）；查 `coordination/tests/` 现有 turn 完成测试 harness。report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. **重排**（`sub_handle_out.rs:352-368`，最小 diff）：

```rust
if let Some(ref completed_event) = workspace_turn_status.1 {
    let _ = event_queue.enqueue(completed_event.clone(), None).await;
}
let turn_status_for_finalize = workspace_turn_status.0.clone();
let _ = tx.send(workspace_turn_status);
Self::finalize_persisted_turn_in_workspace_if_needed(
    session_manager.as_ref(),
    &session_id_clone,
    &turn_id_clone,
    turn_index,
    &effective_agent_type_clone,
    &user_input_for_workspace,
    session_workspace_path.as_deref(),
    session_storage_path_for_finalize.as_deref(),
    turn_status_for_finalize,
    user_message_metadata_clone,
)
.await;
```

   （参数以现场签名为准逐一对应；上方为骨架不是逐字稿。）

2. **顺序注释**：在 enqueue 块上方加两行注释说明"growth finalize 在完成事件与 watchdog 信号之后运行（审计 I3：蒸馏 LLM 最长 30s 不应挡 UI 完成态）"。英文。
3. **测试处置**：先查 `coordination/tests/` 是否存在能跑到该路径的 turn 完成 harness：
   - 有 → 扩展一个断言（完成事件到达不被 finalize 阻塞路径破坏）；
   - 无现实 harness（finalize 需 workspace + LLM，不可注入）→ 不加测试，report 用一节说明勘察结论（家规 4 判定：本 diff 不改 `select!`/cancellation token/timeout race 任何一行，仅事件顺序前移，条款不适用）。
   两种情况都接受，禁止硬造无断言价值的测试。
4. `workspace_turn_status` tuple 的 move/clone 边界按骨架处理；不许顺带改其它行。

## 5. Global Constraints（逐字遵守）

- 只动 `sub_handle_out.rs` 一个文件（+ 可能的 `coordination/tests/` 扩展）。
- 禁止碰 `turn_persist.rs` / `distiller.rs` / `dream.rs` / growth 任何实现（并行 session 领土）。
- 禁止起新 tokio task / 新 channel（编排者已裁定同任务内重排）。
- 日志只许英文、无 emoji。
- sub_handle_out.rs 现 410 行，改后远低于 800。
- 只 commit 代码与（若有）测试文件——**禁止 commit `.superpowers/sdd/progress.md`**；report 文件可 commit。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content（GBK 双重编码事故史）。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --lib dialog_turn
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --lib coordination
```

report 里每条命令贴完整输出尾部（含 test result 行）。report 无输出原文 = 假汇报嫌疑。

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\audit-fix-i3-growth-critical-path-report.md`：实现内容 / 复用侦察节 / 测试勘察结论节 / 每个编译错误最终修在哪一层（机制层/设计层，一行一个）/ 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`c48e4a9`（派发前 HEAD）
- 禁区文件：`turn_persist.rs`、`distiller.rs`、`dream.rs`、`service/agent_memory/**`、`.superpowers/sdd/progress.md`
- commit 规则：conventional commits（如 `fix(core): ...`），不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作（本会话既定流程）

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
