# Judge Brief — 处方 v3 复审（API 错位修复验证）

## 任务

复审 `prescription-v3-20260825.md`。v3 的唯一目的是修复你在 v2 复审中抓出的 6 处 API 错位 + 1 个决策点（F3）。**不重审设计方向**（三方已达成共识），只验证：

1. v2 报告「Changes needed」清单逐项是否落实
2. v3 引用的签名与源码是否一致（**逐条 grep 验证，不接受处方自述**）
3. v3 是否引入新错位/新过度设计

## v2 → v3 待验证修正清单

| # | v2 问题 | v3 声称的修法 | 验证锚点 |
|---|---|---|---|
| 1 | `confirm_tool` facade 不存在 | KernelToolsApi 新增 `respond_to_tool_confirmation(tool_id, approved, reason)` → facade 路由 `coordinator.confirm_tool/reject_tool`（coordinator_session.rs:219 pub） | 读 coordinator_session.rs:219 确认 pub + 签名 |
| 2 | `submit_turn(text:&str)` 错 | 构造 `TurnInputDto{session_id,text,mode,policy,source,workspace_path}` → 返回 `outcome.turn_id` | turn.rs:12-20 DTO 字段 + turn.rs:80 签名 |
| 3 | `cancel_turn` 名/型双错 | `stop_turn(&TurnId)`，TurnId=String | turn.rs:6, 84 |
| 4 | `subscribe_events()->Stream` 错 | callback 模型 → mpsc(256) → use_future 消费 | kernel_facade/events.rs:41-44 签名 |
| 5 | `persist_app_settings` 不存在 | `update_app_settings(FnOnce(&mut AppSettings)->Result<T>)` 闭包（io.rs:54） | io.rs:54 签名 |
| 6 | `test_provider_connection` 不存在 | `test_provider_config(ProviderFormDto)`（settings.rs:183），form 态 | settings.rs:179/183 双签名 |
| 7 | `create_session()` 无参错 | `create_session(SessionConfigDto) -> SessionId` | session.rs:237 |
| 8 | B3 snapshot orphan 挂 session 删除不可行 | 收窄：FileSnapshotSystem per-workspace（service.rs:36），orphan 清理延期 | snapshot/service.rs:36 |
| 9 | B4 trait/inherent 矛盾 | inherent enqueue 改 Result；trait StreamEventSink::enqueue 签名不动，impl 内 error! 日志 | queue.rs enqueue 定义点（inherent vs trait）|
| 10 | B2 归属错（services-integrations） | 改放 desktop `keyring.rs`（KeyringBackend 本体在 desktop） | keyring.rs 位置 + KeyringBackend 归属 |
| 11 | F4 Signal 来源/mixHex/兼容缺失 | 新 Signal 声明 + truth 衰退曲线 t=0.18+0.82·(i/(n-1)) + Rust 侧混色绕开 color-mix | truth HTML 548-584 行对照衰退公式 |
| 12 | 家规 4 测试要求 | B4 加 1 单测（Critical 跳 cap）；F3 facade 加 1 单测（未初始化 Err）；B3 判定不触发家规 4 | 判定是否成立 |

## 约束

- 每条给 VERIFIED / MISMATCH / NEW ISSUE + 一句证据（file:line）
- 不做设计方向重审（B1 单文件 api.rs / 不建 event_bus / F4 状态驱动渐变 = 三方共识，不再议）
- 编译性错误不归你（实现期的事）
- 末尾总判：`READY FOR USER REVIEW` / `NEEDS FIXES: <清单>`

## 输出

写 `E:\agent-project\northing\.superpowers\sdd\reviews\consult-room-prescription-v3-review\report.md`（含本 brief 副本 brief.md）。
