# Review Package — W15-1j（发送路径同型挂死修复：send/stop/approval 挪出 UI 执行器）

- 分支：`main`，BASE `80aef83` → HEAD `4f2a564`（单 commit）
- diff：`git diff 80aef83..4f2a564`，补丁 = `.superpowers/sdd/packages/w15-1j-diff.patch`（3 文件：api.rs / app.rs / approval_card.rs）
- brief：`.superpowers/sdd/w15-1j-brief.md`（验收标准 §1、Global Constraints §5、界外 §4）
- report：`.superpowers/sdd/reports/w15-1j-report.md`

## 任务一句话

W15-1i 修了启动路径 F1，但 send_action / stop_action / settle_approval 仍在 dioxus UI 执行器内联 await 内核链——同型挂死源（用户实测「点击输入后卡死」）。本单把这三条用户触发路径挪到 `turn_runtime` worker rt（抽了共享 helper `spawn_on_turn_runtime`，落 api.rs），Signal 写留 UI 侧。

## 验收标准（逐条判 PASS/FAIL，对应 brief §1）

1. send_action 的 ensure_room_session + submit_turn 在 turn_runtime 执行，结果纯数据回灌，Signal 写全在 UI 侧；语义保留（空文本早退/无 sid 先 ensure/成功清输入+推 Witness/失败 maybe_set_degraded + send_error）。
2. stop_action 的 stop_turn 挪出；UI 侧即时清 streaming/active_turn_id 不变。
3. settle_approval 的 respond_to_tool_confirmation 挪出；卡片 resolved/state_text 写在 UI 侧；失败保持未决。
4. turn_runtime 为 None 时每条路径有 warn 日志 + 不 panic + 不静默吞动作。
5. 共享 helper 落在允许文件集内且被三处真实消费。
6. 运行验证：真实发送一条消息，期间+之后 60s Responding=True、主线程不钉死；截图路径在 report。
7. `cargo check -p northhing` 绿。
8. diff 只触及允许文件集（app.rs / approval_card.rs / api.rs）；界外零触碰（F1/F2/F3 事件循环/entry.rs/core/cli/ci.yml）。

## Global Constraints（逐字）

- 禁止新增依赖。
- 禁整树 git 操作；测试真实执行贴输出原文。
- 运行验证只读观察 + 一条无害短消息；不得删改 `~/.northhing`、`~/AppData/Roaming/northhing` 下任何文件。
- 日志英文无 emoji。

## 重点质询（供 skeptical 校准，不是预判结论）

- helper `spawn_on_turn_runtime` 的单元测试（report 称 `test_spawn_on_turn_runtime_behavior`）是否真执行——它如何在不依赖桌面前提下构造 turn_runtime？检查是否有早退路径（如 None 时直接 return 当 ok）。
- send 失败路径：worker 侧 Err / 通道关闭时 UI 状态是否无残留（streaming 卡 true 等）。
- report 称 `cargo test -p northhing --lib` 165 passed——与 diff 对得上才采信。
- 编排者已视觉验证 `screenshots/w15-1j-desktop-final-sent.png`：窗口渲染正常、witness 气泡「pinping」可见、assistant 401 错误卡可见（模型无 key 属预期，brief 判定标准=窗口不死+错误可见）。
- 实现者拍了 10 张截图（多次发送尝试）——只 w15-1j-desktop-final-sent.png 是验收证据，其余为过程产物。

## 背景（非判据）

- 根因动检报告：`.superpowers/sdd/reports/startup-hang-trace-report.md`。
- W15-1i 已验收范式：`app.rs:67-109`。
