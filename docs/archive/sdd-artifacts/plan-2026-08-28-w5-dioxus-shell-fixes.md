# W5 计划：Dioxus 壳审计修复（2026-08-28）

来源：`.superpowers/sdd/w4-2-dioxus-shell-review.md`（step-explore_reviewer 壳级审计，1C/3I/3M）。
用户拍板范围：F1 + F2 + F4 + F5 + F6。F3（几何跟随线程）按审计建议搁置等 dioxus 上游；F7（provider 编辑 UI，L）留下一波/产品决策。
背景：Slint 壳已于 W4-1 物理删除，Dioxus 是唯一壳；F1 不修则 ✕ 关窗绕过优雅退出，实测进程项必挂。

## Global Constraints（全波通用，reviewer 注意力透镜逐字复制）

1. 分层边界：改动只在 `src/apps/desktop`；其它 crate 零改动。
2. 日志纪律：新增日志一律英文、无 emoji，带关键上下文字段。
3. 并发测试绑定（家规④）：触碰 tokio 任务生命周期/取消/关闭顺序的改动必须随附至少一个自动化测试；无法自动化处由编排者在 brief 里显式豁免并说明理由。
4. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/`。
5. rot-budget：不上调任何 ceiling；不新增 >800 行文件。
6. 验证最小集：`cargo check -p northhing` + 本任务指定的聚焦测试；命令与输出原文进 report。
7. commit 规则：每任务恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物。
8. 不新建无 owner 抽象；优先复用既有通道/设施（brief 里已点名）。

## Task 1 (W5-1): F1 — quit_shell 走优雅退出，禁 process::exit

审计原文（w4-2 F1）：`app.rs:763-765` `quit_shell()` 调 `std::process::exit(0)`，由 room chrome ✕ 按钮（app.rs:433-436）触发。后果：WindowDropGuard 不跑、几何跟随线程被 OS 强杀、worker 线程（tokio runtime + MCP servers + cleanup scheduler）永远收不到 main.rs 的 shutdown 信号。优雅退出路径存在且正确（`shutdown_tx.send(())` → worker 退出 → `shutdown_mcp_servers()`），但从 ✕ 不可达。修复方向：用关窗信号替代 process::exit，让控制流回到 `launch()` 再回 main.rs。

编排者裁定（钉死）：

- 目标语义：✕ → 关闭 room + 全部 module 窗（走 `ShellWindowManager` 现有关闭路径）→ `ui_dioxus::launch()` 返回 → main.rs 的 `shutdown_tx.send(())` + `shutdown_mcp_servers()` 正常执行。
- 实现路径由实现者按现有代码选（关 room 窗触发既有退出链，或经 manager 广播关闭），但必须满足：不再出现 `std::process::exit` 于正常退出路径（init 失败的 exit(1) 保留）；退出后 MCP 子进程被清理。
- 测试豁免说明：窗口关闭链路难自动化；若实现中抽出了可测的信号/状态函数，为其附一个单测；纯 wiring 部分豁免（编排者事后真机实测兜底，对应实测清单 6/7 项）。

Spec：
1. `quit_shell` 不再调 `process::exit(0)`；✕ 触发完整优雅退出链（room + module 窗关闭 → launch 返回 → main shutdown 路径）。
2. `rg "process::exit" src/apps/desktop/src` 仅剩 init 失败路径。
3. 验证集全绿；report 附退出链路的 file:line 走查说明（每个环节如何接力）。

## Task 2 (W5-2): F2 — 事件通道关键事件不丢（TurnState/ToolCall 与 TextChunk 分级）

审计原文（w4-2 F2）：`api.rs:191-193` 把 kernel 1024 广播桥到 `mpsc::channel(256)` 用 `try_send`，满载静默丢。丢 TextChunk = 文案缺口（可接受）；丢 ToolCall(AwaitingConfirmation) = 审批卡消失；丢 TurnState::Completed/Failed/Cancelled = 流式标志永不复位、草稿永不提交、UI 永久卡"生成中"。

编排者裁定（钉死）：

- 方向：**控制事件与数据事件分级**。`TextChunk` 保持有损（try_send，满了可丢）；`TurnState` / `ToolCall`（及任何影响状态机/审批的事件）必须保证投递——实现者选择最小机制（独立 unbounded 控制通道，或满载时控制事件阻塞/重试路径），并在 report 说明选择与代价。
- 不许用"无限加大 256 缓冲"当修复（不解决根因）。
- Spec：
  1. TextChunk 之外的事件类型不再因通道满载而丢失（给出机制与 file:line）。
  2. 附自动化测试：塞满有损通道后 TurnState 事件仍到达消费者（测试设施由实现者按 crate 内现有模式选）。
  3. 消费者循环（app.rs:158-253）的流式复位语义不变。

## Task 3 (W5-3): F4 — onboarding 持久化 provider 配置

审计原文（w4-2 F4）：`pages_onboarding.rs:672-705` 测连通性（test_provider_config）→ key 存 keyring（account "onboarding"）→ `create_session(model_name: "default")`，但从不创建/持久化 `ProviderConfigDto`（无 upsert_provider_config / 无 set_default_provider）。后果：引导完成后全局配置无 provider，会话创建空转，用户面对空设置页。

编排者裁定（钉死）：

- 修复：`test_provider_config` 成功后，从表单字段构造 ProviderConfigDto → `kernel_facade().upsert_provider_config(...)` → 设为默认 provider（核实 facade 上的真实 API 名，report 引用）→ 再 create_session。keyring account "onboarding" 的 key 要与持久化的 provider 关联（或改存到 provider 对应 account，按 keyring 既有约定——先读 `app_state/settings/keyring.rs` 的 account 命名规则再定）。
- 失败语义：persist 失败 → 不推进到下一步，错误展示在 onboarding UI（不静默）。
- Spec：
  1. 引导完成后 `list_providers` 能看到新 provider 且为默认；create_session 不再因缺 default provider 空转。
  2. 各失败臂（测试失败/persist 失败/设默认失败）有明确 UI 错误，不静默吞。
  3. 附聚焦测试或注明无法自动化的理由（UI spawn 块可豁免，但持久化序列若抽成函数则必须测）。

## Task 4 (W5-4): F5 + F6 — PartialEq hack 与 entry.rs Mutex 收口

审计原文（w4-2 F5/F6）：
- F5 `registry.rs:39-43`：`impl PartialEq for ModuleAppProps { fn eq(...) -> bool { true } }` 恒 true，prop 变更永不触发重渲染；当前靠 watch channel 绕行。修复：实现真实 PartialEq（比较影响渲染的字段）或加注释说明故意为之——实现者选正确且懒的那个，注释为下限。
- F6 `entry.rs:139-140`：`room_window_id` / `latest_geometry` 两处 `std::sync::Mutex` 跨线程共享（tao 事件处理器 + use_effect）。当前无跨 await 持锁，是 footgun 非 bug。修复：`room_window_id` 改 `tokio::sync::watch`（单写多读天然契合）；`latest_geometry` 若 watch 化不干净则保留 Mutex + `ponytail:` 注释（注明上限与升级路径）。禁止引入新框架。

Spec：
1. F5：真实 PartialEq 或注释落地（report 说明选择）。
2. F6：room_window_id 走 watch 或等效无锁/弱锁机制；latest_geometry 处置有明确理由。
3. 验证集 + `cargo test -p northhing` 全绿。

## 终审

全波完成后 review-package <wave-base>..HEAD 派终审（reviewer 档位按 diff 规模定）。wave-base = W5-1 派发前 HEAD。
