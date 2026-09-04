# W15-1j Brief — 发送路径同型挂死修复（send/stop/approval 挪出 UI 执行器）

## 1. 来源与验收标准（逐字）

来源 = W15-1i report 遗留节 + 编排者队列（用户实测反馈「点击输入后卡死」）：

> `send_action`（`app.rs:274` 附近）目前仍直接在 UI 执行器内同步 await 内核调用，需在下一单依本单 F1 相同方案迁移至 `turn_runtime`。

机制背景（同 W15-1i，直接采信）：dioxus 0.8.0-alpha.1 混合循环下，任一撞上「永不完成 Pending」的内核 await 会被 42k/s 重 poll → 主线程 busy-spin → 窗口 Not Responding。W15-1i 已修的 F1 是同一形态；本单修用户触发路径。根因动检报告：`.superpowers/sdd/reports/startup-hang-trace-report.md`；F1 已验收的修法范式：`app.rs:67-109`（turn_runtime + oneshot + Signal 留 UI 侧）。

验收标准（逐条可机械核对）：
1. `send_action`（`app.rs:271-303`）的 `ensure_room_session` + `submit_turn` 内核链在 `turn_runtime()` worker rt 上执行，结果（纯数据）经 oneshot/JoinHandle 回灌；所有 Signal 写（session_id_signal/active_turn_id/streaming/user_input/send_error/entries/degraded）在 UI 侧完成；现有语义逐一保留（空文本早退、无 sid 时先 ensure、submit 成功后清输入+推 Witness 条目、失败时 `kernel_error_message` + `maybe_set_degraded` + send_error）。
2. `stop_action`（`app.rs:306-316`）的 `stop_turn` 同样挪出；UI 侧即时清 streaming/active_turn_id 的现有行为不变。
3. `settle_approval`（`approval_card.rs:18-41`）的 `respond_to_tool_confirmation` 挪出；entries 卡片的 resolved/state_text 写在 UI 侧；失败时保持卡片未决的现有语义不变。
4. `turn_runtime()` 为 None 时每条路径都有 warn 日志 + 定义良好的行为（不 panic、不静默吞用户动作）。
5. 三处同型改造若引入共享 helper，helper 必须落在本 brief 允许文件集内且被全部三处真实消费。
6. 运行验证：debug 构建后运行 app，**真实发送一条短消息**，发送期间与之后 60s 窗口 `Responding=True`、主线程不钉 100% 单核；窗口截图（发送后状态）路径进 report。判定标准是「窗口不死」，回复内容不作要求（模型不可达/报错也算通过，只要 UI 活着且错误可见）。
7. `cargo check -p northhing` 绿（桌面合并门）。

## 2. 编排者预检结论（直接采信，勿重复侦察）

| 事实 | 位置（已核实） |
|---|---|
| send_action 现状：dioxus `spawn(async move { ... })`，内核链内联 UI 执行器 | `src/apps/desktop/src/ui_dioxus/app.rs:271-303` |
| stop_action 现状：同上，`api::stop_turn` 内联 | `app.rs:306-316` |
| settle_approval 现状：`async fn` 被三处 dioxus `spawn` 调用，`respond_to_tool_confirmation` 内联 | `src/apps/desktop/src/ui_dioxus/approval_card.rs:18-41`（调用点 :123/:131/:141） |
| 已验收范式（F1）：`turn_runtime()` → `rt.spawn` → oneshot 回灌 → UI 侧写 Signal | `app.rs:67-109` |
| `turn_runtime()` getter（pub(crate)） | `src/apps/desktop/src/app_state/turn_runtime.rs:18` |
| F3 事件循环内还有一处内联 `respond_to_tool_confirmation`（auto-approve 路径，`app.rs:127` 附近） | **本单界外**（流式循环改造是另一形态，留 follow-up） |

codegraph blast radius（编排者代查）：三个改造点全是 room 窗口私有闭包/私有 fn，无外部调用方；`api::` 各函数签名不动 → 零外溢。

## 3. 复用侦察（强制）

动手前查：W15-1i 在 `app.rs:67-109` 留下的 turn_runtime+oneshot 范式可否抽成三处共用的私有 helper（这是**复用既有模式**，优先于各写一份）；`tokio::sync::oneshot` 先例（`api_events.rs` 等）。report 必须有「复用侦察」一节：查了哪些符号、复用了什么、新写等价物逐条给理由。无此节 = 未完成。

## 4. Spec（必须全部满足）

对应 §1 验收标准 1-6，逐条即 Spec。**判断点（已授权）**：
- helper 抽不抽、放 app.rs 还是 api.rs：自裁，但三处必须真实消费它（若抽）；不抽则三处各自内联同范式也可，report 说明选择理由。
- 回灌用 oneshot 还是 JoinHandle：自裁。
- worker 侧失败/通道关闭的兜底：不 panic、有 warn 日志、UI 不残留假状态（如 streaming 卡 true）。

**明确界外（不要碰，越界即 judge Critical）**：
- F3 事件循环（`app.rs:107+` 的 event_channel 消费循环，含 :127 auto-approve 内联 await）——同型但另一改造形态，留 follow-up。
- F1（`app.rs:67-109`，刚验收过）、F2、`api_events.rs`、`entry.rs`、core/services 一切文件、cli、ci.yml。
- 不重构、不顺手清理无关代码。

## 5. Global Constraints（逐字遵守）

- 禁止新增依赖（tokio oneshot / turn_runtime 全现成）。
- 禁整树 git 操作：禁止 `git restore .` / `git checkout .` / `git stash` / `git add -A`，只许点名文件 add/commit。
- 测试必须真实执行：report 贴验证命令真实输出原文；环境阻断须明示交编排者补跑，不得自报 DONE。
- 运行验证是对真实 app 的只读观察 + 一次真实发消息：不得删除/修改 `~/.northhing`、`~/AppData/Roaming/northhing` 下任何文件；发送内容用无害短文本（如「ping」）。
- 日志英文无 emoji。

## 6. 验证（命令 + 输出原文都要进 report）

仓库根 `E:\agent-project\NortHing`：

```
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo build -p northhing
```

（编排者已在 BASE `80aef83` 预跑第一条，基线绿。）

运行验证（Spec 6）：`Start-Process target\debug\northhing.exe`（detached，**绝不直接跑会阻塞 shell**；PTY 写命令注意路径用正斜杠、cmd 要 CR 不要 LF）→ 等 ~20s 加载 → 用 powerskills/desktop 或键盘输入发送一条短消息 → 之后 60s 采样 `(Get-Process northhing).Responding` 与 CPU → `C:\WINDOWS\TEMP\opencode\win-shot.ps1` 拍窗口截图 → `Stop-Process -Name northhing` 收掉。

## 7. 报告

写到 `E:\agent-project\NortHing\.superpowers\sdd\reports\w15-1j-report.md`。含：改动摘要、Spec 逐条自核、复用侦察节、每个编译错误修在哪一层（机制层/设计层，一行一个）、验证命令+输出原文、运行验证数值与截图路径、遗留问题。结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 8. 派发元信息

- BASE commit：`80aef83`（main 当前 HEAD）。
- **允许文件集**（diff 越出 = judge Critical）：
  - `src/apps/desktop/src/ui_dioxus/app.rs`
  - `src/apps/desktop/src/ui_dioxus/approval_card.rs`
  - `src/apps/desktop/src/ui_dioxus/api.rs`（仅当共享 helper 选择落这里）
- 禁区：其它一切文件。
- commit 规则：点名 `git add`；message：`fix(desktop): ... (W15-1j)`。
- 长命令纪律：cargo 一律 PTY/重定向；`run_detached` 本机有静默死前科。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。

## Skill 前置阅读（约束输入，不是需求输入）

- `E:\agent-project\.opencode\skills\rust-skills\m07-concurrency\SKILL.md`（async/执行器边界）
- `E:\agent-project\.opencode\skills\long-running-shell\SKILL.md`（Windows 下 cargo/长命令纪律）

遵循其中与本任务相关的约定，不因此扩展任务范围。
