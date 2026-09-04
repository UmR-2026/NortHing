# W15-1j Judge Review — 发送路径同型挂死修复（send/stop/approval 挪出 UI 执行器）

- 角色：独立验收 judge（skeptical 校准）
- 分支：main，BASE `80aef83` → HEAD `4f2a564`（单 commit `4f2a564`）
- diff：`git diff 80aef83..4f2a564`，补丁 `w15-1j-diff.patch` 标记 3 文件
- 验证命令我亲自跑过：`cargo check -p northhing`（0 错）；`cargo test -p northhing --lib`（165 passed）；截图存在 + 视觉核验

---

## SPEC 判决（对照 brief §1 / review package 验收标准）

| # | 条目 | 判决 | 证据 |
|---|---|---|---|
| 1 | send_action 的 ensure_room_session + submit_turn 在 turn_runtime 执行；Signal 写留 UI 侧；语义保留（空文本早退 / 无 sid 先 ensure / 成功清输入+推 Witness / 失败 maybe_set_degraded + send_error） | **PASS** | `api.rs:166-189` helper 派发到 turn_runtime worker；`app.rs:307-321` ensure+submit 整体在 worker 内 await；`app.rs:323-351` UI 侧写 session_id_signal/active_turn_id/streaming/user_input/send_error/entries/degraded；`app.rs:282-284` 空文本早退保留；`app.rs:308-313` 无 sid → ensure → new_sid 透传；`app.rs:324-335` Success 路径清输入+推 Witness（text_witness 预 clone 保留原文）；`app.rs:341-343` SubmitError 路径 maybe_set_degraded + send_error 保留 |
| 2 | stop_action 的 stop_turn 挪出；UI 侧即时清 streaming/active_turn_id 不变 | **PASS** | `app.rs:358-366` worker rt.spawn 调 stop_turn；`app.rs:367-368` UI 侧同步清 streaming + active_turn_id，与 BASE 行文位置一致；新增 `if let Ok(Err(e)) = res` 警告日志（升级点，非回归） |
| 3 | settle_approval 的 respond_to_tool_confirmation 挪出；entries 卡片 resolved/state_text 写在 UI 侧；失败保持未决 | **PASS** | `approval_card.rs:18-47` 三分支 match：Ok(Ok(())) 在 UI 侧写 entries 卡 resolved=true/state_text；Ok(Err(e)) 仅日志，卡片保持未决；Err(()) 卡片保持未决 |
| 4 | turn_runtime 为 None 时每条路径 warn 日志 + 不 panic + 不静默吞动作 | **PASS** | helper `api.rs:171-174` 统一 warn `ui_dioxus::{caller} turn_runtime handle unavailable` 返回 Err(())；三条路径全部消费 Err：send_action 写 send_error "Background runtime unavailable"（`app.rs:348-350`），stop_action 仅 UI 清状态不静默吞（worker 返回的 Err 在 `app.rs:362-364` 显式 warn），settle_approval 在 `approval_card.rs:43-45` 卡片保持未决并注释提示已日志 |
| 5 | 共享 helper 落在允许文件集内且被三处真实消费 | **PASS** | helper 定义 `api.rs:166-189`（允许文件集 ✓）；三处真实调用：`app.rs:307`（send_action）、`app.rs:361`（stop_action）、`approval_card.rs:20`（settle_approval）——grep `spawn_on_turn_runtime` 全仓命中 7 处（3 调用 + 1 注释 + 1 定义 + 1 测试 + 1 doc），无闲置消费方 |
| 6 | 运行验证：发送短消息，期间+之后 60s Responding=True、主线程不钉死；截图进 report | **PASS（编排者视觉已核验）** | 截图 `screenshots/w15-1j-desktop-final-sent.png`（104077 字节）存在；vision 核验：见证者气泡「pinping」可见、401 错误卡可见、窗口控件正常渲染、无 Not Responding 指示；report 列出 CPU 增量 0.11s（1.4687 → 1.5781）< 0.2%，60s 全程 Responding=True |
| 7 | `cargo check -p northhing` 绿 | **PASS** | 复跑确认通过：warning 全为既有 dead_code/unused_mut；0 错误；`Finished dev profile in 2.23s`（复跑，比 report 报告的 7.52s 快，缓存命中） |
| 8 | diff 只触及允许文件集；界外零触碰 | **PASS** | `git diff --stat` 仅 3 文件：`api.rs` / `app.rs` / `approval_card.rs`（+118 / -40）；`scripts/rot-budget.json` 未触碰；F1（`app.rs:67-109`）、F3 事件循环（`app.rs:127+` 的 auto-approve `api::respond_to_tool_confirmation` `app.rs:147`）、`api_events.rs`、`entry.rs`、core/services/cli/ci.yml 均零改动 |

---

## QUALITY 判决

### 复用侦察（强制）
- **报告有「复用侦察」节**：✓（`w15-1j-report.md:40-54`）
- **claim 独立验证**：
  - 「`turn_runtime()` 在 `src/apps/desktop/src/app_state/turn_runtime.rs`，在 `main.rs:77` 启动时存入」 — 验证：grep `set_turn_runtime_handle` 全仓 2 处命中（turn_runtime.rs 定义 + main.rs:77 调用），claim 属实 ✓
  - 「`tokio::sync::oneshot` 在 `api_events.rs`、`app.rs` 均有使用」 — `app.rs:76` 在 F1 范式用 `oneshot::channel()`（即 `tokio::sync::oneshot`），`api_events.rs` 用法需自行抽查，但 helper 复用 oneshot 作为 worker→UI 桥梁为既有模式的延伸 ✓
  - 「`kernel_error_message` + `maybe_set_degraded`」 — `turn_banner.rs:12-23` 定义 kernel_error_message，`turn_banner.rs:26-38` 定义 maybe_set_degraded，`app.rs:34` import，`app.rs:341-342` 复用，✓
- **新写等价物理由**：3 处用户交互路径需共享 worker 派发模板，提取 helper 避免 20+ 行模版代码在 3 处复制——理由属实，非投机抽象 ✓

### 无 owner 抽象
- `spawn_on_turn_runtime`：3 个真实调用方（app.rs:307、app.rs:361、approval_card.rs:20），无悬空消费者 ✓
- `SendOutcome` enum：定义在 spawn 闭包内（`app.rs:295-305`），仅 send_action 局部使用，scope 正确 ✓

### 预算闸
- `scripts/rot-budget.json` 未修改，diff 越界检查通过 ✓

### 条件早退测试（必查项）
- `test_spawn_on_turn_runtime_behavior`（`api.rs:296-304`）：**只执行了早退路径**。
  - 测试逻辑：调 helper，然后 if `turn_runtime().is_none()` 断言 Err / else 断言 Ok(42)。
  - 真实情况：在 `cargo test` 单元测试环境下，`set_turn_runtime_handle()` 从不被调用（仅 main.rs:77 在生产启动时调），所以 `turn_runtime()` 始终返回 None → 必然走 `assert!(res.is_err())` 分支。
  - 这意味着 **`rt.spawn` + oneshot 回灌路径未被自动化测试覆盖**——只有 turn_runtime handle unavailable 的早退分支被测。
  - 我亲自跑 `cargo test -p northhing --lib spawn_on_turn_runtime` 单测，确实走的是早退分支（输出 1 passed，filter 出 164）。
  - **判定**：命名 "behavior" 误导，但行为上不会导致 SPEC 失败——rt.spawn/oneshot 路径靠手动运行验证（report §运行验证，60s 60+0 钉死采样已证）。属于「测试机会未抓」而非「测试造假」。→ **Minor**：建议补一个 `#[tokio::test(flavor = "multi_thread")]` 显式 `set_turn_runtime_handle()` 跑通的测试。

### god-file 观测点
- `app.rs` HEAD 共 847 行（`read` 工具 line-numbering；PowerShell `Measure-Object -Line` 因 CRLF/空行口径报 801，以 Read 工具 line 数为准），超过 AGENTS.md §3 800 行警戒线。
- 当前 `scripts/rot-budget.json` 未将 `app.rs` 登记为 god-file。
- review package 已注明「app.rs 本单后 847 行、超 800 未注册——这是已知的闸红项，有独立 slim 单排队处理；你的职责是记录该事实，不因它 REJECT 本单行为正确性」。
- 本 PR 对 app.rs 净增 71 - 15 = 56 行，主要来自 SendOutcome enum 与 spawn_on_turn_runtime 三处调用适配——重构本单无法避免（这就是为啥该文件本就需要独立 slim 单）。
- **判定**：记录事实，不阻 APPROVE（与 review package 指引一致）。→ **Minor（记账）**

### 编译/设计层
- 本单无 E0xxx 错误；编译一次过 ✓
- `turn_id` 在 stop_action 中双层 `async move` 嵌套捕获——Rust 编译器接受，因内层仅取 `&turn_id`（borrow）而外层不直接使用 turn_id。cargo check 通过 ✓
- `let degraded = degraded;`（`app.rs:291`）与 `let existing_sid = session_id_signal();`（`app.rs:292`）是把 Signal/可变变量送进 spawn 闭包的标准 dioxus 模式，与 BASE 行文兼容 ✓

---

## Cannot verify from diff

无。helper 的真实 worker 行为靠运行验证（已通过），早退路径靠单元测试（已通过），cargo check 已通过。所有验收项可判定。

---

## 判决

### APPROVE

### Findings

**Critical**：无

**Important**：无

**Minor（记账/不阻塞）**：
1. **测试覆盖盲点（M-test）**：`test_spawn_on_turn_runtime_behavior`（`api.rs:296-304`）在单元测试环境下仅执行早退分支（`turn_runtime()` 永远为 None），未覆盖 `rt.spawn` + oneshot 回灌核心路径。helper 行为靠 report 运行验证（60s CPU 采样 0.11s 增量）证明正确。**建议下次提 PR 时补一个 `set_turn_runtime_handle()` 显式设置 Handle 的 happy-path 测试**，但本单不阻塞。
2. **app.rs god-file 仍未登记（M-god）**：`app.rs` HEAD 847 行（> 800 警戒线），`scripts/rot-budget.json` 未注册。review package 已注明由独立 slim 单处理，本 PR 因重构必然推动行数（净 +56）。**记账，不阻 APPROVE**。建议编排者在 slim 单排期中跟催一次。

### 总评
- SPEC：8/8 PASS
- QUALITY：复用侦察属实、helper 无悬空消费者、预算闸未越、测试覆盖盲点记账、god-file 记账
- 实现者提交的 diff 严格落在允许文件集内（api.rs/app.rs/approval_card.rs），界外零触碰
- 运行验证：60s 窗口 CPU 增量 0.11s、Responding=True、见证者 + 错误卡同时可见、模型无 key 报 401 属预期（brief 明确「窗口不死 + 错误可见」为判定标准）
- 报告命令与输出与 diff 严格对得上（165 passed 含 5 个 api.rs 测试含新 helper 测试）
- 单 commit、message 符合 `fix(desktop): ... (W15-1j)` 约定、点名 add（commit 4f2a564 内未触 `git add -A` 痕迹）

**结论：本单同型挂死修复行为正确、文件集纪律严格、运行验证可信。APPROVE。**