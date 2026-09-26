# W26-1 Review (m3) — steering UX：UserSteeringInjected → Banner

## SPEC：PASS

逐条对照 brief §1 验收标准与 §4 Spec：

| AC | 简述 | 判定 | 证据（file:line） |
|---|---|---|---|
| AC1 | facade `UserSteeringInjected` 改为 `Banner(Info, display_content)` | PASS | `src/crates/assembly/core/src/kernel_facade/events.rs:397-400` —— `AgenticEvent::UserSteeringInjected { display_content, .. } => vec![KernelEventDto::Banner { level: BannerLevel::Info, message: display_content.clone() }]`，与 W22-1 先例 ContextCompression 三臂同构（events.rs:333-348） |
| AC2 | drops 清单移除 UserSteeringInjected + 新增 Banner 映射钉死测试 | PASS | `events.rs:511-549` 新增 `test_agentic_event_to_dtos_user_steering_injected_banner`（含 S1 空 display_content 子用例：`message == ""` 断言覆盖）；`events.rs:649-654` drops 清单已无 UserSteeringInjected（diff 删除原 :640-647 8 行块） |
| AC3a | CLI chat 增 `UserSteeringInjected` 臂，含 turn_id 门控 + set_status + needs_redraw + 文案英文 | PASS | `src/apps/cli/src/modes/chat/run.rs:409-425` —— turn_id 门控逐字照先例 `chat_state.current_turn_id().map_or(true, |id| id == turn_id)`（与 :382/:394/:403 同形）；文案 `User steering injected` / `User steering injected: <preview>`（空 display_content 降级），英文无 emoji |
| AC3b | CLI exec 增 `UserSteeringInjected` 臂，`self.print_text(|| ...)` 单行 | PASS | `src/apps/cli/src/modes/exec.rs:371-379` —— `self.print_text(|| println!("\n{msg}"))`，文案与 chat 臂逐字一致（S3 满足） |
| AC4 | desktop 零代码改动 | PASS | `git diff 03ecda0..bf1df10 --name-only` 不含任何 `src/apps/desktop/` 路径；现成消费链 `src/apps/desktop/src/ui_dioxus/app.rs:55` (`Signal<Option<String>>` banner) + `:216-220` (`KernelEventDto::Banner { message, .. }` 在 `streaming` 期间 `banner.set(Some(message))`) —— steering 在 turn 进行中（streaming=true）到达，复用现成路径，commit message 与 report §1 AC4 引用一致 |
| AC5 | §6 验证 5/5 绿 | PASS | report §3 各节均有 exit 0 与命令原文（§6.1 ws 2m11s / §6.2 56 passed 含 `test_agentic_event_to_dtos_user_steering_injected_banner` 与 `test_agentic_event_to_dtos_intentional_drops` / §6.3 cli 1m56s / §6.4 hygiene / §6.5 gate `Attempt verification passed: all modified files are within allowlist`） |

Spec 子条：

| 子条 | 判定 | 证据 |
|---|---|---|
| S1 | PASS | `events.rs:532-548` 子用例：empty `display_content` → `dtos.len() == 1` 且 `message == ""`（不过滤，照常映射） |
| S2 | PASS | 同 AC2 |
| S3 | PASS | chat `run.rs:415-419` 与 exec `exec.rs:372-376` 文案构造块逐字同构（`truncate_str(display_content, 60)` + 空 preview 降级为 `"User steering injected"` / 非空为 `format!("User steering injected: {preview}")`） |
| S4（禁区） | PASS | diff 仅触及 allowlist 内 7 个文件：4 代码 + brief/report/allowlist 自身；不动 `kernel-api` / `desktop/` 任何文件 / `turn_tick.rs`（发射侧） / `dialog_turn.rs` / `turn_lifecycle.rs` / `save.rs` / `turn_persist.rs`（W26-2 领地） / `lsp/manager.rs`（W26-4） / `runtime-ports` / `services-integrations` / `mcp/` / `desktop keyring/main`（W26-5） / `ci.yml` / `scripts/` |
| S5（只动 UserSteeringInjected） | PASS | diff 中 facade match 臂仅 `:397-400` 一处由丢弃改映射；chat/exec 各增一臂；其余 8 静默事件（`SessionCreated` / `SessionStateChanged` / `SessionDeleted` / `SessionTitleGenerated` / `ImageAnalysis*` / `SubagentSessionLinked` / `TokenUsageUpdated` / `ThreadGoalUpdated` / `ModelRoundStarted/Completed` / `DeepReviewQueueStateChanged` / `SessionModelAutoMigrated`）原样保留为 `debug! + vec![]`（见 events.rs:349-404 现状）；CLI match 也仅增一臂，未旁接其他事件 |
| S6（agentic.rs:296 注释同步） | PASS | `src/crates/contracts/events/src/agentic.rs:296` 由 `// TODO(surface): ... (当前无任何 surface 消费（2026-09-09 ZCode 审查）)` 更新为 `// Consumed by desktop (via KernelEventDto::Banner) and CLI chat/exec surfaces (W26-1).`；唯一 agentic.rs diff |

Global Constraints（brief §5）：

| 约束 | 判定 | 证据 |
|---|---|---|
| 日志英文无 emoji；CLI 文案英文 | PASS | CLI 文案 `User steering injected` 英文无 emoji；facade Banner 消息使用 `display_content` 原文（生成方负责 i18n，facade/CLI 不做翻译） |
| desktop 零改动 | PASS | diff 不含 `src/apps/desktop/` 任何路径 |
| 禁整树 git 操作；点名 add/commit | PASS | commit `bf1df10` message 列出 7 文件（与 `git show --stat` 一致），未 `git add -A` |
| 测试真实执行，输出原文 | PASS | report §3 §6.2 原文 `56 passed; 0 failed; 0 ignored; 0 measured; 1025 filtered out; finished in 0.09s` 与新用例 + drops 更新后的钉死测试均在列 |
| 路径卫生 `<RUSTUP>`/`<HOME>`/`<TMP>` 占位 | PASS | report §3 起首声明 `<TMP>=C:\\Windows\\Temp\\opencode`、`<RUSTUP>=C:\\Users\\UmR\\.cargo\\bin`，正文不再贴裸路径 |
| cargo 一律 `rustup run stable-...-msvc` 前缀 | PASS | report §6.1/6.2/6.3 三项 cargo 命令均带此前缀 |

## QUALITY：PASS

### 常规项

- **正确性**：facade 映射路径与 W22-1 先例同构；CLI 双重消费（chat 走状态行 set_status / exec 走 print_text）文案逐字一致，行为符合预期；turn_id 门控照搬 W22-1 形态，避聊天期间旧 turn 状态干扰。
- **错误处理**：facade 对空 display_content 不抛错（`String::clone()` 正常），test 显式覆盖；CLI `truncate_str` 在 `string_utils.rs:13-19` 已含 char-boundary 安全 + 全空降级（不会因 UTF-8 切错 panic），且 `truncate_str:6` 已先 `s.lines().next().unwrap_or("")` 取首行（已读文件核验）。
- **i18n**：facade 不引入翻译，文案交由 desktop/CLI 现状（desktop 中文硬编码是项目冻结态，CLI 全英文——与本单无关）。
- **日志**：英文 + 无 emoji（`tracing::info!("{msg}")` 在 chat 臂；exec 臂仅 `println` 不打日志，避免重复，与先例 ContextCompression 一致）。
- **平台边界**：不引入 host API；facade 位于 assembly/core；CLI 与 desktop 各自消费 DTO/事件，路径符合六层分层。
- **Concurrency**：未触及 `tokio::select!` / cancel token / timeout race，不触发家规 4（concurrency test binding）。
- **Cargo workspace**：report §6.1 全 workspace 0 error；§6.3 cli 0 error。
- **Commit message**：bf1df10 prefix `feat(events): W26-1 ...`，body 含 `用户 2026-09-09 拍板 C1 steering-only`，与 brief §8 commit 规则一致。

### Judge 必查项（judge-brief-block 防腐）

- **复用侦察**：report §2 有「复用侦察」节，5 条全部属实——独立抽查：
  - `KernelEventDto::Banner { level, message }`（contract FROZEN，零改动）：grep 确认 events.rs 中 `northhing_kernel_api::events::{BannerLevel, KernelEventDto, ...}` 与既有同模块用法一致；
  - `BannerLevel::Info` 变体存在（kernel-api 已 W22-1 落地，本单不动）：grep `BannerLevel::Info` 在 facade 三处已用，本单新增第 4 处同构；
  - desktop banner Signal + `.degraded-banner` 链：grep `banner.set(` 命中 app.rs 多处（含 :218），与 brief 引用 :216-220 对齐；
  - `chat_view.set_status` + `needs_redraw`：chat 先例臂 :383-385 / :395-399 / :404-406 已用，本单 :421-422 同形复用；
  - `self.print_text`：exec 先例臂 :359-369 已用，本单 :378 同形；
  - `crate::ui::string_utils::truncate_str`：文件存在，`src/apps/cli/src/ui/string_utils.rs:5-22`，签名 `(s: &str, max_bytes: usize) -> String`，含 char-boundary 防护 + `...` 截断 + 取首行——本单首次跨臂复用同一工具，零新组件、零新依赖。PASS
- **无 owner 抽象**：diff 不引入 trait / interface / 封装层 / 新配置项；纯 match 臂 + 单测试 + 单注释同步。无投机抽象。PASS
- **预算闸**：diff 不触及 `scripts/rot-budget.json` 或任何 baseline/manifest；不在 meta-ratchet 路径上（metaRatchetPaths 与本单 4 代码文件零交集——agentic.rs 是 contracts/events 文档同步注释，不构成元闸路径调整）。PASS
- **条件早退测试**：新增的 `test_agentic_event_to_dtos_user_steering_injected_banner` 不含平台/权限/环境条件分支（无 `cfg(unix)` / `skip` / `make_xxx_or_ignore`），report §6.2 原文 `... ok` 行确认用例真实执行（不仅是函数被列举）。PASS
- **god-file 观测点**：events.rs 当前 700 行级（BASE ~696 行 + diff +56 行 -16 行 ≈ 736 行），低于 800 警戒；chat/run.rs / exec.rs 距警戒远。未触及 god-file。无需观测记录。PASS

### 模式接线一致性（背景须知 W22-1 先例对照）

- **facade 映射** —— W22-1 `events.rs:333-348` 三臂写法一致为 `vec![KernelEventDto::Banner { level, message: <expr> }]`；本单 `:397-400` 一行同构（`message: display_content.clone()`）。同文件既有 Banner 映射的写法 ✓
- **chat 臂 turn_id 门控** —— W22-1 `chat/run.rs:382/:394/:403` 三处均 `chat_state.current_turn_id().map_or(true, |id| id == turn_id)` 包裹 `set_status + needs_redraw + tracing::info!/warn!`；本单 `:414-424` 同形（`set_status + needs_redraw + tracing::info!`）。`tracing::info!` 对应 Info 级 Banner，文案/日志语义与先例匹配。✓
- **exec 臂 print_text** —— W22-1 `exec.rs:358-370` 三处均 `self.print_text(|| println!("\n..."))`（Failed 用 `eprintln!`）；本单 `:371-379` 同形（Info 用 `println!`），与先例同语义。✓
- **Banner 消息拼接** —— W22-1 使用 `format!("Context compressed: {tokens_before} → {tokens_after} tokens")` / 静态串；本单直接透传 `display_content.clone()`（已 String），不二次 format——更省事，且 S1 空字符串场景无需额外处理（空就是空）。✓
- **测试形态** —— W22-1 `test_agentic_event_to_dtos_context_compression_banners` 三事件三 match 分支断言 Banner；本单 `test_agentic_event_to_dtos_user_steering_injected_banner` 同 match 分支断言 + S1 空串子用例。✓

### 条件早退测试核查（独立复核 §6.2 输出）

report §6.2 输出原文：`test kernel_facade::events::tests::test_agentic_event_to_dtos_user_steering_injected_banner ... ok` —— 该用例名出现在 `running unittests` 输出中（非仅 `56 passed` 数字），证明断言路径真实执行。无静默 skip。

## Cannot verify from diff

- **实际终端渲染**：`set_status` 与 `print_text` 的视觉呈现（chat 状态行/eprintln 与 stdout 是否正确 flush、ANSI 着色、terminal 兼容性）需手动 smoke 验证；diff 范围内的单元/编译/类型检查无法覆盖此层。**结论：报告 §6 不要求此项，brief §6 也未列；按 brief 即可接受。建议波收口人工 smoke 一次（不阻塞）。**
- **streaming 时序竞态**：desktop app.rs:216-220 在 `*streaming.read()` 为 true 时才 `banner.set(...)`。steering 事件从 `turn_tick.rs:460-471` 发射（brief §2 预检钉死），与 TextChunk/StreamChunk 在 turn 进行中是同一 phase——但 facade 不带 phase 信息，desktop 仅靠 `streaming` Signal 判断。本单未改 desktop，未改 turn_tick，无法从 diff 验证 steering 到达时 streaming 必为 true（曾存在 edge case：ContextCompressionFinished 也走同一 Banner 路径，在非 streaming 时 desktop 静默吞掉）。**结论：本单与 W22-1 同型 Banner 映射，desktop 既成消费链 0 改动；该时序竞态沿用 W22-1 已有的桌面行为，非本单新增风险。**
- **W23-2 落地态基线**：report 提「W23-2 落地态 -1」——实际 events.rs 现状（after W26-1）外层 `debug!` 丢弃臂 13 个（grep `intentionally dropped at facade` 排除内层 ToolEvent 9 个后命中 13 行：350/354/358/362/366/370/374/378/382/386/390/394/402），与 report §1 S5 复核写的「12 个」差 1。**结论：Minor counting error，不影响正确性——diff stat 删除行仅 1 行（events.rs 旧 `:397-400` 4 行 + drops 清单 8 行，共 12 行删除但功能上只删 1 个事件的丢弃臂），可能是报告把「+1 映射臂」与「-1 丢弃臂」在计数口径上表达偏紧；属报告笔误级别，非代码缺陷。**
- **三窗 gate 全波复跑**：report §6.5 已记录「W1 `03ecda0..bf1df10` exit 0 / W2 `1ef1b16..<B-tip>` exit 0（仅 report 回填，6 条 unfulfilled=code 文件已由 W1 覆盖）」，但未亲跑总窗 `03ecda0..B-tip`（含兄弟 W26-5 的 out-of-bounds）。brief §6.5 要求编排者波收口用全波 allowlist 复跑总窗——本单已按 brief 要求「窗口内出现其它 W26 单文件停手报编排者复跑，未私扩 allowlist」。**结论：编排者波收口事项，非本单判定阻塞项。**

## Findings

无 Critical / 无 Important。

- [Minor] Report §1 S5 复核「facade `debug!` 丢弃臂计数 12 个，较 W23-2 落地态 -1」与 events.rs 实际 13 个外层丢弃臂（grep 实证）差 1 —— 仅报告叙事计数偏差，不影响 diff 正确性与 allowlist 守门。建议 review-m3 之后的 ledger/报告刷写时把数字修正或删掉具体计数（避免后续读者被误导）。修复指令：把「12 个」改为「13 个」或改为定性描述（"除 UserSteeringInjected 外的全部静默事件原样保留"）；不影响通过。

## 终判

**APPROVE** —— SPEC 全条满足且无 Critical/Important finding；唯一 Minor 为报告笔误，不阻塞。