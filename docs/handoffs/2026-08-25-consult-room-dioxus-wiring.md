# Handoff 2026-08-25 — consult-room Dioxus 接线轮（P0 三批收口，P1 进行中，P1c 半成品在盘）

> 状态权威源：`northing/.superpowers/sdd/progress.md`（Consult-Room Dioxus 接线 Ledger 段）。
> 上一篇：`2026-08-24-consult-room-closed.md`。

## 本轮成果

处方链：v1 → 三方 judge（minimax-m3 / step-explore / ox-alpha）→ v2 → v2 复审抓 6 处 API 错位 → **v3（API 核实版）→ minimax-m3 复审 12/12 VERIFIED → 用户终裁放行**。

| 批次 | Commit | 内容 | Judge |
|---|---|---|---|
| P0a | `4889d22` | KernelToolsApi 新增 `respond_to_tool_confirmation`（契约变更，用户裁定方案 A）+ facade 实现 + `ui_dioxus/api.rs`（薄封装 + event_channel callback→mpsc） | APPROVE 0C/0I/2M |
| P0b | `0200899` | 真 `<input>`（IME 守卫）+ send/stop 合一 + TextChunk streaming 渲染 + `ensure_room_session` | APPROVE 0C/0I/4M |
| P0c | `a893a8a` | **第二契约扩展**：`ToolCallPhase::AwaitingConfirmation`（预检抓出 ConfirmationNeeded 被 `_=>vec![]` 丢弃）+ 事件映射 + approval 卡按钮接线（乐观 resolve） | APPROVE 0C/0I/3M |
| P1a | `e311cd6` | 编年史条状态驱动渐变（mixHex 衰退曲线与真值 L566 逐字节一致，Rust 侧混色绕 color-mix） | APPROVE 0C/0I/6M |
| P1b | `826ab89` | Settings 持久化：facade KernelSettingsApi 接 Card1 引擎 / Card3 接入点 / Card4 MCP + AppSettings workspace（**初轮 brief 数据源指错，fix 轮修正**） | APPROVE 0C/0I/5M |

## ⚠️ 在盘半成品（勿 git checkout 清掉）

**P1c（MCP env keyring）被 cancel 前已在工作区落下 5 文件 +599 行改动，未验证未提交**：

- `app_state/settings/keyring.rs`：`MCP_ENV_SENTINEL` + `is_mcp_env_sentinel` / `is_env_sentinel` / `make_env_sentinel` + `store_env`/`load_env`（~192 行新增）
- `app_state/settings/io.rs`：+100 行（sentinel 写入/还原路径）
- `app_state/settings/io/io_tests.rs`：+266 行测试
- `app_state/settings/mod.rs` + `types.rs`：小改

**下个 session 接续动作**：
1. `git stash list` / 直接在工作区跑验证：
   ```powershell
   $env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
   cd E:\agent-project\northing
   cargo test -p northhing --lib settings; cargo test -p northhing --lib keyring; cargo check -p northhing
   ```
2. 绿 → 补写 report + **翻转 tech-debt-ledger P1-8 为 resolved（家规 2 同 commit）** → commit → 派 minimax-m3 judge
3. 红 → 修或弃（`git checkout --` 前先把 diff 存出来）

## 剩余队列（按处方 v3 执行序）

| 批次 | 内容 | 依赖 | Brief 状态 |
|---|---|---|---|
| **P1c** | MCP env keyring | 无 | ✅ `consult-room/task-p1c-mcp-env-brief.md` 已写（含半成品） |
| P2a | F1 room 数据流（get_messages 覆盖 seed_session） | P0a | 未写 |
| P2b | B4 event queue（inherent enqueue 改 Result + Critical 跳 cap + 调用点 error! 日志） | 无 | 未写 |
| P3a | F6 onboarding 流程（页内 Step enum + test_provider_config + create_session） | P0a+P1b | 未写 |
| P3b | B3 cleanup 调度（lib.rs bootstrap 24h loop；snapshot orphan 延期） | 无 | 未写 |

## 关键工件

- 处方：`northing/.superpowers/sdd/consult-room/prescription-v3-20260825.md`（commit `9bba819` judge-verified）
- 各批 brief/report/review：`.superpowers/sdd/consult-room/task-p0[abc]-*.md` / `task-p1[ab]*.md`；`.superpowers/sdd/reports/`；`.superpowers/sdd/reviews/p0{a,b,c}-*/`、`p1{a,b}-*/`
- 台账：`northing/.superpowers/sdd/progress.md`（Consult-Room Dioxus 接线 Ledger 段）

## 运维备忘

- **环境陷阱**：GNU toolchain linker 需 `$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"` 否则 build script 链接必崩
- **模型状态**：`gemini-37-flash`（vertex）本轮首派连接失败一次后恢复；备胎 `gemini-37-flash-agy`。`judge-ox-alpha` 已改指 `openrouter/stealth/ox-alpha`（P0 前轮实证成功一次；后遇 rate-limit）。`minimax-m3` judge 连续 5 单全绿，质量稳定。
- **新注册**：`judge-ox-alpha.md`（openrouter/stealth/ox-alpha）在 `C:\Users\UmR\.config\opencode\agents\`——session 静态注册表，新 session 生效。
- **家规 2 欠债**：P1c 完成时务必同 commit 翻 P1-8（brief 已写明）。
- **judge 遗留 follow-ups**（非阻塞，终审 triage 时过一遍）：P0a M-1 报告偏离未声明 / event_channel Drop guard；P0b session 过滤 unwrap_or(true)；P0c respond 失败无 UI 回滚；P1a dblclick 位置公式差异（已声明降级）；P1b 乐观失败无 toast。
