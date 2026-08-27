# Task Brief — Audit I2: 坏 session_state.json 毒化会话列表（skip-and-continue）

## 1. 来源与验收标准（逐字）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r2-core.md` Finding 2（Important）：

> In `list_sessions`, treat a failed state load as `None` → `SessionState::Idle` with a `warn!` (skip-and-continue), matching the existing `.unwrap_or(SessionState::Idle)` intent. Optionally make `list_sessions_all_workspaces` tolerate a single failing workspace (return its group empty + warn) instead of aborting the whole listing.

编排者裁定：两项都做（"Optionally" 项纳入——P22 的 room 会话解析依赖 `list_sessions_all_workspaces`，单工作区元数据损坏不应全灭）。

验收标准（逐条可机械核对）：

1. 单个 session 的 state 加载失败（parse error / IO error）不再使 `list_sessions` 返回 `Err`；该 session 以 `SessionState::Idle` 进列表，并有一条 `warn!`（英文、含 session_id、不含敏感内容）。
2. 单个 workspace 的 `list_sessions` 失败不再使 `list_sessions_all_workspaces` 整体 `Err`；该 workspace 以空 sessions 组进结果，并有一条 `warn!`。
3. 回归测试：元数据正常 + state 文件损坏 → `list_sessions` 返回 `Ok`，该会话在列且 state 为 `Idle`。
4. `cargo check --workspace` 与聚焦测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

2026-08-26 @ 64fba6f 实时核实：

| 事实 | 锚点 |
|---|---|
| 毒化点 1：`load_stored_session_state(workspace_path, &metadata.session_id).await?` 逐 session 传播错误；`.map(...).unwrap_or(SessionState::Idle)` 已处理 None（缺失），只差把 Err 当 None | `src/crates/assembly/core/src/agentic/persistence/session_subhandlers.rs:303-307` |
| `load_stored_session_state` 返回 `NortHingResult<Option<StoredSessionState>>`；parse 失败 = `Err`（经 `json_store.rs` `read_optional` 的 `Err(Deserialize)`） | `session_subhandlers.rs:123`；`src/crates/services/services-core/src/json_store.rs:113-116` |
| 毒化点 2：`list_sessions(...).await.map_err(...)?` 在 per-workspace 循环里，任一失败整体中止 | `src/crates/assembly/core/src/kernel_facade/session.rs:90-96` |
| `list_sessions_all_workspaces` 返回 `Vec<WorkspaceSessionsDto { workspace_path, sessions }>`；空组 = `sessions: Vec::new()` | `kernel_facade/session.rs:97-103` |
| 测试基建现成：`TestWorkspace::new()` + `PersistenceManager::new(workspace.path_manager())` + `standard_metadata(...)` + `save_session_metadata`；同文件已有 `list_sessions` 测试两个 | `session_subhandlers.rs:440-501`（tests 模块内），文件现 537 行 |
| state 文件可用 `save_stored_session_state` 先写合法值再覆盖垃圾字节来造损坏（或自行定位 session 目录下的 state json） | `session_subhandlers.rs:269, 283-286` |
| 文件已 `use tracing::...info!`（:294 有 `info!` 调用）；加 `warn!` 需确认导入 | `session_subhandlers.rs:294` |
| P22 依赖链：诊室 `ensure_room_session` → facade `list_sessions_all_workspaces` | ledger P2-22（已 resolved，本任务是其连带防护） |

## 3. 复用侦察（强制）

动手前查：`sanitize_runtime_state` / `unwrap_or(SessionState::Idle)` 现有语义；同文件 tests 模块的 `TestWorkspace`/`standard_metadata` 复用；report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. **`list_sessions` 容错**（`session_subhandlers.rs:303-307`）：`load_stored_session_state(...).await` 改 `match`：`Ok(value)` → 维持现有 `.map(sanitize).unwrap_or(Idle)`；`Err(e)` → `warn!`（含 session_id 与错误）+ `SessionState::Idle`。列表照常推进。
2. **`list_sessions_all_workspaces` 容错**（`kernel_facade/session.rs:90-96`）：per-workspace `list_sessions` 失败 → `warn!`（含 workspace_path 与错误）+ 推入 `WorkspaceSessionsDto { workspace_path, sessions: Vec::new() }`，循环继续。不提取新抽象，inline 改。
3. **回归测试**（`session_subhandlers.rs` tests 模块，TDD）：元数据正常保存 + state 文件写入垃圾字节 → `manager.list_sessions(workspace.path()).await` 返回 `Ok`；断言该会话在列表中且 `state == SessionState::Idle`。
4. **`list_sessions_all_workspaces` 容错不加新单测**（编排者已裁定：该路径测试需全 kernel 初始化 harness，代价远超 6 行 inline 改动；审计原文亦标注 Optionally）——reviewer 按代码审查判，report 里用一段文字说明此决定的出处（本行）。

判断点（已授权，不许上报）：warn! 的具体措辞（英文、含 id/path、无敏感内容）；测试函数命名按文件惯例。

## 5. Global Constraints（逐字遵守）

- 禁止改 `services-core/src/json_store.rs` 的 `read_optional` 语义（全局改动爆炸半径太大，本任务只修消费侧）。
- 禁止改 `load_stored_session_state` / `save_stored_session_state` 的签名与语义。
- 禁止给 `kernel_facade/tests.rs` 加测试（该文件是 rot 观测组登记成员，ceiling 823 已钉死）。
- 日志只许英文、无 emoji。
- 本任务不涉 `tokio::select!` / cancellation / timeout race —— 家规 4 不适用。
- session_subhandlers.rs 现 537 行 / kernel_facade/session.rs 现 195 行，改后均须远低于 800。
- i18n frozen，不许碰 locale 资源。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content（GBK 双重编码事故史）。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --lib session_subhandlers
```

report 里每条命令贴完整输出尾部（含 test result 行）。report 无输出原文 = 假汇报嫌疑。

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\audit-fix-i2-session-list-report.md`：实现内容 / 复用侦察节 / 每个编译错误最终修在哪一层（机制层/设计层，一行一个）/ 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`64fba6f`（派发前 HEAD）
- 禁区文件：`src/crates/services/services-core/**`、`kernel_facade/tests.rs`、`session_subhandlers.rs` 与 `kernel_facade/session.rs` 中本任务外的现有逻辑
- commit 规则：conventional commits（如 `fix(core): ...`），不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作（本会话既定流程）

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
