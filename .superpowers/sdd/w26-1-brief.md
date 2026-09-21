# W26-1 Brief — steering UX：UserSteeringInjected → Banner（单 judge）

## 1. 来源与验收标准（逐字）

Plan `.superpowers/sdd/plan-2026-09-10-w23-w26-closure-and-decoupling.md` §1：
> "W26-1 steering UX（UserSteeringInjected → Banner，W22-1 模式复用）[单 judge]"

用户拍板（2026-09-09，台账在案）："C1 只修 steering UX（UserSteeringInjected→Banner 模式复用，其余 8 事件维持静默）"——**只动这一个事件**，其它静默事件不许顺带接。

### 验收标准（机械可核对）

- AC1: `kernel_facade/events.rs` 中 `AgenticEvent::UserSteeringInjected` 从显式丢弃（:397-400 `vec![]`）改为产出 `vec![KernelEventDto::Banner { level: BannerLevel::Info, message }]`，message = 事件的 `display_content` 字段值。
- AC2: 钉死测试更新：`test_agentic_event_to_dtos_intentional_drops`（W23-2 落地）中 UserSteeringInjected 移出"故意丢弃"清单；新增/改写一条断言其映射为 Banner(Info, display_content) 的用例。
- AC3: CLI chat（`modes/chat/run.rs`）match 增 `AgenticEvent::UserSteeringInjected` 臂（在 `_ => {}` 之前），按 W22-1 先例 `chat_view.set_status(...)` + `needs_redraw`，**含 turn_id 门控照先例**（:382/:394/:403 的 current_turn_id 比较）；CLI exec（`modes/exec.rs`）增臂，用 `self.print_text(|| ...)` 输出一行。**两处文案为英文**（CLI 界面现状全英文，W22-1 先例英文；中文硬编码仅是 desktop 现状），两臂文案一致。
- AC4: desktop 零代码改动——`app.rs:216-220` 已消费 `KernelEventDto::Banner`（streaming 期间显示），本单复用该路径（report 须引用 app.rs:216-220 现状证据）。
- AC5: §6 验证命令全绿，输出原文进 report。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `9222bbc42c2b0a13961dd8e63686c3c804abe698`（= origin/main）。

| 事实 | 证据 |
|---|---|
| `UserSteeringInjected` 定义（agentic.rs:297-304）：字段 session_id/turn_id/round_index/steering_id/content/**display_content** | 侦察实证 |
| facade 丢弃点 events.rs:397-400（debug! + vec![]）；W23-2 已把 14+9 变体全具名化、`_ =>` 已清除——本单是把其中一个具名臂从丢弃改映射 | 侦察 + W23-2 台账 |
| `KernelEventDto::Banner { level: BannerLevel, message: String }`（kernel-api/events.rs:95-98）；`BannerLevel::Info` 变体存在（:47-53）——FROZEN 契约零改动，纯复用 | 侦察实证 |
| W22-1 映射先例：events.rs:333-348（ContextCompression→Banner）+ 测试 :451-509 | 侦察实证 |
| desktop 现状：app.rs:55 banner Signal、:216-220 streaming 中消费 Banner、:517-521 `.degraded-banner` DOM——steering 在 turn 进行中（streaming=true）到达，路径现成 | 侦察实证 |
| CLI chat 消费 AgenticEvent（非 DTO），:381-409 有 ContextCompression set_status 先例，:411 有 `_ => {}`；exec :358-370 先例 + :372 `_ => {}`，print_text :457-461 | 侦察实证 |
| 发射点 turn_tick.rs:460-471（emit_event Normal 优先级） | 侦察实证 |
| 仓规：core 定向测试须带 `--features product-full`（裸跑 E0433） | AGENTS.md 验证表注记 |
| 非 meta-ratchet（本单文件集与 metaRatchetPaths 零交集）→ 单 judge | workflow-policy.json |

## 3. 复用侦察（强制）

- Banner DTO / BannerLevel / desktop banner Signal / CLI set_status / print_text 全部复用，零新组件。
- report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

- S1: facade 映射按 AC1；display_content 为空字符串时的行为：照常映射（不过滤——发射方已决定注入即显示）。
- S2: 测试按 AC2。
- S3: CLI 两臂按 AC3；chat 文案建议形态 `已插入引导：<display_content 截断合理长度>`，exec 一行打印同义——具体文案实现者定，但两臂文案须一致。
- S4: 禁区：kernel-api（FROZEN 契约不动）、desktop 全部文件、turn_tick.rs（发射侧不动）、dialog_turn.rs / turn_lifecycle.rs / save.rs / turn_persist.rs（W26-2 领地）、lsp/manager.rs（W26-4）、runtime-ports + services-integrations + mcp 相关 + desktop keyring/main（W26-5）、ci.yml 与 scripts/ 全部。
- S5: 其它静默事件一律不碰（拍板原文"其余 8 事件维持静默"）——diff 里出现第二个事件的接线 = SPEC FAIL。
- S6: `src/crates/contracts/events/src/agentic.rs:296` 的过时注释「当前无任何 surface 消费（2026-09-09 ZCode 审查）」在本单落地后即为假——顺手更新该行注释为落地态（唯一允许的 agentic.rs 改动；注释-现实同步仓规）。

## 5. Global Constraints（逐字遵守）

- 日志英文无 emoji；CLI 文案英文（CLI 界面现状全英文）；desktop 侧零改动。
- 禁整树 git 操作；只点名 add/commit。
- 测试真实执行，report 贴输出原文。
- 路径卫生（D-2）：report 输出原文的本机路径段一律 `<RUSTUP>`/`<HOME>` 占位。
- cargo 一律 `rustup run stable-x86_64-pc-windows-msvc cargo ...` 前缀。

## 6. 验证（命令 + 输出原文进 report）

1. `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace` → 0 error
2. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full kernel_facade` → 绿（含新映射用例与更新后的 drops 钉死测试）
3. `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing-cli` → 0 error
4. `node scripts/check-repo-hygiene.mjs` → exit 0
5. `node scripts/verify-task-gate.mjs verify-attempt --base 9222bbc42c2b0a13961dd8e63686c3c804abe698 --tip <本单最后 commit sha> --allowlist .superpowers/sdd/w26-1-allowlist.txt` → exit 0（自建 allowlist 含自身；**窗口内若出现其它 W26 单文件，停手报编排者复跑，不得私扩 allowlist**）

## 7. 报告

路径 `.superpowers/sdd/w26-1-report.md`。章节：改动摘要 / 复用侦察 / 验证（命令+输出原文）/ 疑虑 / 状态（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）。

## 8. 派发元信息

- BASE: `9222bbc42c2b0a13961dd8e63686c3c804abe698`
- 允许文件集：
  - `src/crates/assembly/core/src/kernel_facade/events.rs`
  - `src/crates/contracts/events/src/agentic.rs`（仅 :296 注释一行，S6）
  - `src/apps/cli/src/modes/chat/run.rs`
  - `src/apps/cli/src/modes/exec.rs`
  - `.superpowers/sdd/w26-1-brief.md` / `w26-1-report.md` / `w26-1-allowlist.txt`
- 禁区：§4-S4 所列 + 其它未列出文件。
- commit 规则：点名 git add；前缀 `feat(events): W26-1`，body 注「用户 2026-09-09 拍板 C1 steering-only」；允许多 commit。
- 并行声明：W26-2/3/4/5 同波全并行，文件集不相交（矩阵见波首 ledger/派发记录）；同工作树 cargo 锁等待可容忍。
