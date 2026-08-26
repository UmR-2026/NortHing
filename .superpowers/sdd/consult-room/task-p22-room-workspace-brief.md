# Task P22 — P2-22 修复：room 会话按持久化工作区解析 + 双建 TOCTOU + 孤儿 i18n 键

来源：终审 finding 2/3/7 → 台账 **P2-22**（`docs/status/tech-debt-ledger.md`）。consult-room v3 波后首个跟进批。

## 现状 bug（已核实）

`ui_dioxus/api.rs::ensure_room_session()`（api.rs:62-74）：
1. `list_sessions()` 走 facade 硬限定 **CWD** 默认工作区（kernel_facade/session.rs:38-49）——onboarding 以用户工作区创建的会话永远不在候选集；
2. 自身 create 用 `workspace_path: None` → 又落到 CWD；
3. startup future 与 send_action 两处并发调用非原子 list→create（毫秒级双建窗口）。

修复所需接口**全部现成**：facade `list_sessions_all_workspaces()`（session.rs:71-106，组按最近访问排序、CWD 组兜底在末尾）目前 UI 层零调用。

## 改动（仅 `ui_dioxus/api.rs` + `i18n.rs` 一行删除）

### ① 纯函数选择器 + ensure 重写（api.rs）

```rust
/// Pick the room session from workspace-grouped summaries.
/// Preferred workspace hit wins; otherwise the first group that has any
/// session (groups are ordered most-recent-access first by the facade);
/// `None` means "create fresh".
fn pick_room_session<'a>(
    groups: &'a [WorkspaceSessionsDto],
    preferred_workspace: Option<&str>,
) -> Option<&'a SessionSummaryDto>
```

判定表（实现按此逐行）：

| 输入 | 行为 |
|---|---|
| `preferred=Some(ws)` 且存在 `workspace_path == ws` 的组且该组 sessions 非空 | 取该组第一条（facade 排序 = 最近优先） |
| `preferred=Some(ws)` 但无匹配组或组空 | 返回 `None`（→ 调用方在 preferred 工作区新建；**不**回落其它工作区的旧会话） |
| `preferred=None` | 第一个非空组的第一个会话；全空 → `None` |

公开 `ensure_room_session()` 重写：

1. 进程级缓存串行化（同时修 TOCTOU）：
   ```rust
   static ROOM_SESSION_CACHE: tokio::sync::Mutex<Option<String>> = tokio::sync::Mutex::const_new(None);
   ```
   lock 后命中缓存直接返回 clone；未命中才走解析。**Err 不缓存**（允许重试）。加一行 ponytail 注释：缓存进程生命周期有效，delete/archive 后需重启才换房——升级路径=接 delete_session 时失效。
2. 解析 preferred 工作区：`crate::app_state::settings::load_app_settings().await`
   - Ok(s) → `s.current_workspace.or_else(|| s.workspaces.first().map(|w| w.path.clone()))`（P3a 只 add_workspace 未设 current，故 first 即 onboarding 工作区）
   - Err(e) → `tracing::warn!` 一行 + preferred=None（零配置用户保持 CWD 旧行为）
3. `api::list_sessions_all_workspaces()` 新薄包装（与既有同风格）→ `pick_room_session` 命中即缓存+返回。
4. 未命中 → `create_session(SessionConfigDto { workspace_path: preferred.clone(), agent_type: "agentic".into(), model_name: "default".into(), name: Some("诊室".into()) })` → 缓存 + 返回。
5. 既有 `test_api_functions_fail_cleanly_before_init` 与 `test_ensure_room_session_fails_cleanly_when_uninitialized` 必须原样通过（settings 读真文件成功与否不影响最终 Err 判定）。

### ② 单测（api.rs tests mod）

针对 `pick_room_session` 纯函数 ≥4 条：preferred 命中 / preferred 落空返回 None / 无 preferred 取第一非空组 / 全空返回 None。fixture 手搓 `WorkspaceSessionsDto { workspace_path, sessions: vec![SessionSummaryDto{..}] }`（字段见 kernel-api/src/session.rs:24-42）。

### ③ 孤儿 i18n 键删除

删 `i18n.rs:387` `pub const ONBOARDING_BTN_COMPLETE`（5d2d22c 移除了最后使用者，终审 finding 7）。若删除触发任何生成契约/i18n 校验报错：**还原并如实报告**，不要顺手改生成器。

## 禁区

- **不动 core / kernel_facade / contracts**（all-workspaces 接口已存在，纯消费）。
- 不动 `submit_turn`（TurnInputDto.workspace_path 保持 None——会话已携带工作区，turn 级不重复传；如内核实际需要 turn 级路径，那是另一个立项）。
- 不动 app.rs（P2a M1 的 entries.set 启动竞态窗口本轮**裁定不修**：订阅流只推订阅后的新事件、启动期无运行中回合 → 无真实触发路径；理由写进 report，ledger 注记保留）。
- 不动 pages_* UI。
- 缓存不加失效机制（注释声明天花板即可）。

## 复用侦察（必填进 report）

- 全仓确认 `list_sessions_all_workspaces` 除 trait/facade 定义外零消费者（应无既有 picker 可复用）。
- `load_app_settings` 在 ui_dioxus 的引用先例（P3a 的 `super::super::app_state::settings` 路径模式），import 方式保持一致。
- 是否已有等价"选最新会话"helper（app_state/sessions.rs 的 Slint 侧可能有类似逻辑，仅参考不可跨层引用）。

## 验证（report 必贴命令+尾部输出）

```
cargo check -p northhing
cargo check -p northhing --tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib ui_dioxus
```

## Report

写 `.superpowers/sdd/reports/task-p22-room-workspace-report.md`：改动清单（file:line）、判定表逐行落点、缓存天花板声明、验证尾部、偏离及理由。
