# Handoff 2026-07-27 — K4a 冒烟修复收尾（session 8）

> 上一篇：`2026-07-26-session7-handoff.md`（K4a 完工）。本篇覆盖 K4a 后 GUI 冒烟与 5 个回归修复。

## 需求基线状态
- K4a 已关闭（northstar），K0 编译目标达成（3.40s ≪ 14.93s）。
- **用户决策已下：前端要换掉（FR-T 重构线）**——因此 Slint 侧 UI 渲染层问题降级，后端（kernel facade）正确性是验收重点。

## 已完成（commit 表）
| commit | 内容 |
|---|---|
| `8da4d97` | fix(desktop): stop_turn 内存查找（根因=find_session_for_turn 用 default_workspace_path 扫 list_sessions，workspace 不一致扫不到活动 session）|
| `e4791a6` | fix(desktop): 设置页测试连接显示（provider-test 状态只绑了 WelcomeView，SettingsView 链没透传）|
| `f4d4921` | fix(desktop): streaming lifecycle（Bug A 停止状态）：streaming_lifecycle.rs 新模块，4 helper + OnceLock dispatcher + turn generation 守卫 + 原子 compare_and_clear（state.rs）|
| `84e74d0` | fix(desktop): Skills/MCP 面板换 facade 数据源（旧 AppSettings.skills_enabled/mcp_servers 是死数据）+ workspace override 诚实降级（facade 无 per-workspace 数据时灰化按钮）|
| `36ba7f8`/`7d074d6`/`2c3ff66`/`bcbdd7c` | （并行 session）P2 ledger 同步 + debug-log crate-layout 回归 + 3 份审计报告 |

## 冒烟验收结论
- ✅ 发消息有回复；测试连接（llm）正常；Skills 面板列表正常；MCP 面板有项目
- ✅ **后端停止链路完整且实测**：`cancel_convergence_emits_terminal_event_when_turn_stuck` PASS（卡死 turn 也能收敛）；链路 = facade.stop_turn → cancel_dialog_turn → 取消令牌 → DialogTurnCancelled 事件
- ❌ **Slint 停止按钮仍不渲染**（is-streaming 属性链代码经 judge-lc 全量 review PASS，但运行时未见按钮；按钮位置=聊天面板顶部工具栏 ■，非输入框旁）。**因前端要换，暂缓深挖**；新前端接 `stop_turn(turn_id)` + 监听 TurnState 事件即可
- ❌ **MCP test 按钮空挂**：Slint 回调链完整但 Rust 从未注册 `on_test_mcp`，facade 无 `test_mcp_server` 方法（P2 零新增约束）。**待用户拍板**：是否破 P2 加 facade 方法（建议并入新前端设计一并做）

## 遗留 / 队列
1. K3 ROI 闸门拍板（条件已达标：编译 6.85s < 7.47s；审计建议先清 36 warning）
2. 清零 36 warning（cargo fix 可消 29 + 手动删 7 dead 符号，~40min）
3. 52+ unpushed commits 待推送
4. FR-T 前端重构开工（FR-T3a：Token 补全 + 低复杂度换绑，2-4h；阻塞面审计见 `docs/design/2026-07-22-frontend-redesign/`）
5. Skills 面板文字叠印 bug（截图可见，待立案；前端换皮时可能自然消解）
6. Boundary checker 接入 CI（~1h）
7. 占位测试资源已种：`%APPDATA%\northhing\skills\smoke-placeholder\SKILL.md`、`app.json` 的 `mcp_servers.smoke-echo`（假 server，reconnect WARN 是预期噪音；备份 `app.json.bak-smoke`）
8. 已知噪音：`.system\memory` SKILL.md 解析失败 ERROR（builtin，每次 turn 打两条，与本线无关）

## 运维变更（模型台账）
- **qw 周额度耗尽**（2026-07-27 用户告知），恢复后回首选
- **lc coder 通道退化**：调试单双连空汇报零产出；**lc judge 通道健在**（全量 review PASS 质量好）
- **m3 coder 新实证**：根因调试单能扛（R1-R3 三轮迭代收敛），其沙箱禁 git commit 需编排者代提交
- judge-m3 验收严格（R1/R2 FAIL 在并发竞态与 commit 卫生，R3 PASS 仅 hygiene 拆分）—— FAIL 轮次均真实提升质量
- 并行 session 会在同 repo 落 commit（本session 顶部被落 3 笔审计 commit，无冲突）+ 留未跟踪临时文件（boundary_result.txt、checker_*.txt、project-evaluation_20260727.md，未清）

## 操作教训（重要）
- **GUI 冒烟前必须核对 `target\debug\northhing.exe` LastWriteTime**：cargo check/test --lib 不产出 run 二进制，cold start 可能跑修复前旧 exe（本轮"修复无效"虚惊即此因）
- 后台启动：`Start-Process`/WMI `Win32_Process.Create` 会被 bash 工具会话结束收割（时灵时不灵）；**`explorer.exe <exe>` 可靠脱离**但抓不到 stdout；抓日志用前台 `& exe *> log`（阻塞本 turn）
- PowerShell `Set-Content -Encoding UTF8` 写 JSON 带 BOM → northhing config 解析炸（"expected value at line 1 column 1"）；写配置一律用 `[IO.File]::WriteAllText(..., UTF8Encoding($false))`

## Suggested skills（下一 session）
- 续 FR-T：`writing-plans`（FR-T3a 计划）→ `subagent-driven-development` + `dispatching-parallel-agents`
- 清 warning / 推 commits：机械单直接派 `coder-s37`/`coder-m27hs`
- 收尾时：`verification-before-completion`、本 skill（handoff）
