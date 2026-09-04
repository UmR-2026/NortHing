# Review — W15-1i（桌面启动挂死修复：core IO 超时降级 + F1 挪出 UI 执行器）

- 分支：`main`，BASE `9b41eac` → HEAD `f2f3819`（2 commits：`2472cff` services-core + `f2f3819` desktop）
- diff：`git diff 9b41eac..f2f3819`（4 文件 / +168 / −33）
- brief：`.superpowers/sdd/w15-1i-brief.md`
- report：`.superpowers/sdd/reports/w15-1i-report.md`
- 截图（实现者已落盘 + 编排者已视觉验证）：`screenshots/w15-1i-desktop-70s.png`（正常渲染，"northing" chrome / 知序 / "它正在命名自己"）

## 双判决

### 1. SPEC（对照 brief §1 逐条）

| # | 验收条目 | 判定 | 证据（file:line） |
|---|---|---|---|
| 1 | `read_optional` 的 `fs::metadata` + `fs::read_to_string` 包 timeout（5s），超时转显式错误，调用方降级语义不变 | **PASS** | `json_store.rs:120` (`io_timeout(path, "metadata", timeout, fs::metadata(...))`)、`json_store.rs:133`（`io_timeout(path, "read_to_string", timeout, fs::read_to_string(...))`）；默认常量 `JSON_FILE_IO_TIMEOUT: Duration = Duration::from_secs(5)` 在 `json_store.rs:19`；超时映射 `ErrorKind::TimedOut` 在 `json_store.rs:32-35`；既有 `ReadMetadata` / `Read` 错误变体（`json_store.rs:46-56`）保留调用方 `metadata_store` 走 `SessionState::Idle` 兜底语义 |
| 2 | `write_bytes_atomic` 各 fs 操作包 timeout，超时接入既有重试与 PermissionDenied 降级 | **PASS** | `create_dir_all` 包裹于 `json_store.rs:184`；`write_temp` 于 `json_store.rs:195`；`remove_temp` 于 `json_store.rs:204`；`fallback_overwrite` 于 `json_store.rs:226`；`replace_file_from_temp` 内 `rename_initial` / `remove_target` / `rename_replace` 于 `json_store.rs:274, 279, 286`；`is_retryable_write_error` 已含 `ErrorKind::TimedOut`（`json_store.rs:295`），无缝接入既有 5 次重试；`PermissionDenied` 降级直写分支保留于 `json_store.rs:221-233` |
| 3 | 新增超时测试确定性触发（pending() 注入或极小 timeout），无早退绿；既有 json_store 测试保持绿 | **PASS** | 4 个超时测试用 `std::future::pending::<std::io::Result<_>>()` + `Duration::from_millis(10)`：`json_store_contracts.rs:111-125, 127-141, 143-157, 159-173`，每个都断言 `err.kind() == TimedOut` 且消息文案含 op 名 + 超时时长（不是仅 `is_err`）；第 5 个测试 `json_store_timed_out_is_retryable`（`json_store_contracts.rs:175-179`）验证 `TimedOut` 被 `is_retryable_write_error` 认作可重试。**实测 10/10 全部通过**（本轮独立跑 `cargo test -p northhing-services-core json_store`，0.02s）。`cargo check --workspace` 通过（仅既有 warning，无新 warning） |
| 4 | F1 内核链改在 worker rt 上执行，结果经 oneshot 回灌；Signal 写全部在 UI 侧；缓存命中/settings warn/get_messages warn/entries 非空才 set 等语义保留；turn_runtime() 为 None 时有 warn 日志 | **PASS** | `app.rs:71-74` 取 handle + `None` 时 `tracing::warn!`；`app.rs:76-84` 创建 oneshot + `rt.spawn(...)` 派发 `ensure_room_session` + `get_messages` 内核链；`app.rs:86-109` 在 UI 侧 `rx.await`，所有 `session_id_signal.set`（`app.rs:88`）、`entries.set`（`app.rs:94`）均在 UI 线程。语义保留核：`messages_to_entries` 仅在非空时 set（`app.rs:93-95`）；`Err` 路径两个独立 warn（`app.rs:98, 104`）；新增 channel 关闭兜底 warn（`app.rs:107`）；缓存命中由 `ensure_room_session` 内部保证（无外部变化）。oneshot 已存在于仓内 23 处先例（`acp:92`、`cdp_client:176`、`process_protocol:286`、`ask_user_question_tool:192` 等），模式正确 |
| 5 | `entry.rs` 注释与 trace 报告新结论一致（busy-wake + 冻结 timer；sleeping future 平反） | **PASS** | `entry.rs:179-190` 新注释明确：(a) sleeping use_future 由实验 D/E 平反无害；(b) 真 poison = 高频（~42k/s）轮询 Pending 的 busy-wake future + tokio 时间驱动冻结 + 饿死 tao 消息泵；(c) geometry publishing 必须脱离 timer 轮询。内容与 `startup-hang-trace-report.md` 实验矩阵 D/E/G 与"修复方向建议 3"完全一致 |
| 6 | 运行验证证据：Responding=True、CPU 数值、截图路径 | **PASS** | report §运行验证数值与截图证据：构建走 `rustup run ... cargo build -p northhing`（启动前）；PID 44968 三点采样：t=0s Responding=True/CPU 406ms；t=30s Responding=True/CPU 453ms；t=70s Responding=True/CPU 1.359s/70s（≈2% 单核均值，远低于原 100% 满转）。截图 `screenshots/w15-1i-desktop-70s.png` 已落盘且本评审目视确认窗口正常渲染（"northing" chrome / 知序 / "它正在命名自己"），无白屏/无响应标识 |
| 7 | diff 只触及允许文件集 | **PASS** | `git diff --name-only 9b41eac..f2f3819` = `{src/crates/services/services-core/src/json_store.rs, src/crates/services/services-core/tests/json_store_contracts.rs, src/apps/desktop/src/ui_dioxus/app.rs, src/apps/desktop/src/ui_dioxus/entry.rs}` — 与 brief §8 允许集逐项匹配 |
| 8 | 界外零触碰（send_action / api_events / kernel_facade / ci.yml 等） | **PASS** | `send_action` 在新文件仍位于 `app.rs:280-324`，未触碰；`api_events.rs` 未在 diff 中；`kernel_facade/**` 未在 diff 中；`.github/workflows/ci.yml` 未在 diff 中；`scripts/rot-budget.json` 未在 diff 中 |

### 2. QUALITY

#### 复用核查（§3 必查项）

- **report「复用侦察」节**：存在（`w15-1i-report.md` 第 44-57 行），列了 `tokio::time::timeout`（30+ 散装）、`tokio::sync::oneshot`（acp/cdp_client/lsp 等）、`turn_runtime()`、`api_events.rs:101-109` 的 Handle 模式。
- **独立验证**：
  - `tokio::time::timeout` 在 services-core 内：`json_store.rs:30`、`process_manager.rs:10`、`diff/service.rs:7`（非仓内 IO timeout helper）。
  - `tokio::sync::oneshot` 仓内 23 处独立确认（grep 已列），模式与 `app.rs:76` 一致。
  - `turn_runtime()` 实现 `src/apps/desktop/src/app_state/turn_runtime.rs:12-20`，`set_turn_runtime_handle` 在 `main.rs:77` 装配。`app.rs:71` 调用正确。
  - `api_events.rs:101-109` 的 Handle 模式 = `Handle::try_current() + handle.spawn`（已目视确认）。本次实现用了不同模式（`OnceLock<Handle>` getter + `rt.spawn`），更适配 worker 跨线程回灌的用例，不构成"复制既有能力不复用"问题。
- **结论**：复用侦察属实，无复制既有能力问题。

#### 无 owner 抽象（§3 必查项）

**Important 1**：diff 中新增两个 `pub` 公开方法 **无外部 owner**：
- `JsonFileStore::read_optional_timeout`（`json_store.rs:113-117`，`pub async fn`）：全仓 grep 仅 1 个调用方——自己的非超时 shim `read_optional`（`json_store.rs:110`）。**测试代码用的是 `io_timeout` 直接调用，不是这个方法**。
- `JsonFileStore::write_bytes_atomic_timeout`（`json_store.rs:174-179`，`pub async fn`）：全仓 grep 仅 1 个调用方——自己的非超时 shim `write_bytes_atomic`（`json_store.rs:171`）。**测试代码未触及**。

report 第 9 行声称这两个变体「供自定义超时及自动化测试驱动」，但 `json_store_contracts.rs` 5 个新测试全部走 `io_timeout` 直调（第 113/129/145/161 行），不调用这两个 `_timeout` 变体。所谓「测试驱动」的消费方不存在。

这违反：
- brief §3：「动手前用 rg/codegraph 确认」+ 投机性抽象 = Important 起评
- AGENTS.md Housekeeping rule 0（YAGNI ladder rung 1: "Does it need to exist at all?")
- Ponytail 规则："No unrequested abstractions: no interface with one implementation"

修复建议：去掉 `_timeout` 公开变体，把 `read_optional` / `write_bytes_atomic` 直接实现超时（即不要先包成两个函数再 thin-call，节省一层间接）。

#### god-file 观测点（§3 必查项）

- `json_store.rs` BASE 231 行 → HEAD 269 行（+38），仍 < 800 行门槛，无需观察。
- `app.rs` BASE 738 行 → HEAD 756 行（+18），仍 < 800 行门槛，无需观察。
- 未触及 rot-budget 登记的超 800 行文件。

#### 预算闸（§3 必查项）

- diff 未触碰 `scripts/rot-budget.json` 或任何 baseline。

#### 条件早退测试（§3 必查项）

- 4 个超时测试均无平台/权限/环境条件早退。注入的 `std::future::pending()` 永远 Pending，`tokio::time::timeout(10ms, pending)` 必须真的等 10ms 后才返回 `Err(Elapsed)`，断言 `err.kind() == TimedOut` 实际验证了 Elapsed 路径。
- 实际耗时 0.02s / 10 tests ≈ 5ms / 4 个超时测试 = 与 10ms 单测超时量级一致，pending() 确实等到了 Elapsed。
- **真实跑通，无早退绿**。

## Cannot verify from diff

- **运行时数值**：单次 70s 采样（修复前为 4/4 确定性必挂，70s Responding=True + CPU 1.35s + 截图正常已构成强证据），但长时间稳定性（> 5 min）未观察。Brief §1.6 仅要求 60–90s。
- **截图时序**：截图在 t=70s 时，F1 内核调用应早已完成；无法从单张静态截图判定 F1 内核调用与 oneshot 回灌的端到端时延。但 brief 不要求 F1 时延指标。
- **report 声称的 `replace_file_from_temp` `dead_code` 警告**：独立验证当前 `cargo check --workspace` 无此 warning，且 BASE 版本（`9b41eac`）中该函数已被 `write_bytes_atomic` 调用，不存在 "twin `_timeout` suffix function" 的中间态——report 此节为不实描述（详见下方 Minor）。

## Findings

### Critical

无。

### Important

**I1（无 owner 抽象）**：`JsonFileStore::read_optional_timeout`（`json_store.rs:113-117`）与 `JsonFileStore::write_bytes_atomic_timeout`（`json_store.rs:174-179`）均为 `pub async fn`，但全仓无任何外部消费方，唯一的调用方是它们各自非超时的 shim 包装。report 称其「供自定义超时及自动化测试驱动」不属实——5 个测试全部直接调 `io_timeout`，未触碰这两个变体。违反 brief §3「动手前用 rg/codegraph 确认 + 投机性抽象 = Important」。建议删除这两个公开方法，让 `read_optional` / `write_bytes_atomic` 直接承载 `DEFAULT_TIMEOUT` 常量（少一层间接、零 API 表面膨胀）。

### Minor

**M1（report 不实描述）**：report「编译与告警处理」节声称「warning: associated function replace_file_from_temp is never used」被设计层修复，并提到「消除冗余的 `_timeout` 后缀孪生函数」。事实：(a) `cargo check --workspace` 在 BASE 与 HEAD 均无此 warning（BASE 版本该函数已被 `write_bytes_atomic` 调用，HEAD 加 `timeout` 参数后调用方不变）；(b) diff 中不存在任何 `_timeout` 后缀孪生函数，仅是给现有函数加了 `timeout: Duration` 参数。建议 report 删除此节或改写为"无新增 warning"。

**M2（report「复用侦察」描述精度）**：report 第 56 行称 `io_timeout` 为「私有/模块辅助函数」，但实际可见性是 `pub`（`json_store.rs:21` 无修饰符 = `pub`）。与 `read_optional_timeout` / `write_bytes_atomic_timeout` 的可见性一致（无修饰符 = `pub`），但与 report 自述「私有」矛盾。若保留 I1 修复方案（删掉 `_timeout` 变体），`io_timeout` 仍宜保持 `pub(crate)`（避免 crate 外引用）。

## 总结判

**APPROVE**（带 1 项 Important）

理由：SPEC §1 八条逐项满足，核心修复（core IO timeout + F1 worker 离场）双管齐下闭环，运行验证 CPU/Responding 数值与 trace 报告根因一致。Important I1（无 owner 抽象）不影响功能正确性，可在下一单（建议 W15-1j 或合并 W15-1i'）作为清理 follow-up 修复；不阻塞 W15-1 §7#11 三张截图重拍的完整闭环。Minor M1/M2 是 report 描述问题，不阻塞合并。

> 编排者可选路径：(A) 直接合并 + 提交 W15-1j follow-up 修 I1；(B) 打回 implementer 在本单内修 I1（diff 改动很小：删 18 行 + 改 2 行函数体）。

---

## I1 修复重审

- 修复 commit：`d1d31b8 refactor(services-core): drop unused timeout variants (W15-1i review I1)`
- 修复 base→head：`976ad9d..d1d31b8`
- 修复报告：`.superpowers/sdd/reports/w15-1i-report.md`「I1 修复记录」节（line 193-296）
- 验证态度：skeptical 验收，逐条对照 + 独立 grep + 亲跑 `cargo check --workspace`，不重跑 fixer 已报的 10/10 测试

### 验收点逐条核验

| # | 验收条目 | 判定 | 证据（file:line / 命令） |
|---|---|---|---|
| 1 | 两个 pub 变体已删除（`read_optional_timeout` / `write_bytes_atomic_timeout`） | **PASS** | `git diff --stat 976ad9d..d1d31b8` = 单文件 `src/crates/services/services-core/src/json_store.rs` `+2/-17`；diff hunk 删除两个 `pub async fn ..._timeout(...)` 完整函数体；当前 `json_store.rs` grep 零源内命中 |
| 2 | `read_optional` / `write_bytes_atomic` 直接承载 `DEFAULT_TIMEOUT`（5s 常量）驱动 `io_timeout` | **PASS** | `json_store.rs:19` `pub const JSON_FILE_IO_TIMEOUT: Duration = Duration::from_secs(5);`；`json_store.rs:107` `pub const DEFAULT_TIMEOUT: Duration = JSON_FILE_IO_TIMEOUT;`；`json_store.rs:109-110` `read_optional` 起手 `let timeout = Self::DEFAULT_TIMEOUT;`；`json_store.rs:113, 126` `io_timeout(path, ..., timeout, fs::{metadata,read_to_string}(...))`；`json_store.rs:163-164` `write_bytes_atomic` 起手 `let timeout = Self::DEFAULT_TIMEOUT;`；`json_store.rs:169, 180, 189, 211` 同模式 `io_timeout(..., timeout, fs::{create_dir_all,write,remove_file,write}(...))` |
| 3 | 全仓无残留引用（`rg "read_optional_timeout\|write_bytes_atomic_timeout"` 源内零命中） | **PASS** | 仓内 grep 命中仅 `.superpowers/sdd/{packages/w15-1i-review.md, w15-1i-brief.md, reports/w15-1i-report.md, reports/w15-1i-review.md}`（评审/brief/report 描述原文），**`.rs` 源零命中**；`tests/json_store_contracts.rs` 未触及（验证点 4 同证） |
| 4 | 测试未改动且输出（10/10 绿）与 diff 对得上 | **PASS** | `git log -- src/crates/services/services-core/tests/json_store_contracts.rs` 末次 = `2472cff`（W15-1i 主单），**非** I1 修复 `d1d31b8`；`git diff 2472cff..d1d31b8 -- ...json_store_contracts.rs` 空输出；fixer 报告 `cargo test -p northhing-services-core json_store` 原文 10 测试名（`json_store_timed_out_is_retryable` + 5 既有 + 4 `io_timeout_*`）= `10 passed; 0 failed; 0 ignored; 0 measured`，与 brief §1.3 + 原 review 表 #3 测试集一一对应 |
| 5 | `cargo check --workspace` 绿 | **PASS** | 本轮亲跑 `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace` = `Finished dev profile in 1.87s`（无 error）；亲跑 `cargo check -p northhing-services-core` = `Finished dev profile in 7.65s`，**无 warning**（与 fixer 报告一致；既有 warning 仅来自 `apps/desktop` 与 `apps/cli`，与本次改动无关） |
| 6 | diff 未越出 json_store.rs（无顺手改其它东西） | **PASS** | `git diff --name-only 976ad9d..d1d31b8` = 单文件 `src/crates/services/services-core/src/json_store.rs`；hunk 仅含 (a) `read_optional` 体外两行删除 + 体内 `let timeout = Self::DEFAULT_TIMEOUT;` 行内新增；(b) `write_bytes_atomic` 体外三行删除 + 体内 `let timeout = Self::DEFAULT_TIMEOUT;` 行内新增。无格式漂移、无空白行新增、无注释改动、无 import 增删 |

### 残留 Minor 复核

- **M1（report 不实描述 `replace_file_from_temp dead_code`）**：原 review 已指出，与 I1 修复无关，本轮 fix 未触及——本 Minor 仍属 implementer 原 report 描述问题，**不阻塞合并**。
- **M2（`io_timeout` 可见性 `pub` vs `pub(crate)`）**：原 review 标"若保留 I1 修复方案，io_timeout 仍宜 `pub(crate)`"。I1 已修复后本 Minor 变得 actionable——`json_store.rs:21` 仍为 `pub async fn io_timeout`，而外部消费方仅 `services-core/tests/json_store_contracts.rs:1`（同 crate 内引用即足够）。**但不在 I1 修复范围内**（属下一单清理），fixer 选择不动是对的——避免 scope creep。

### 总判更新

- 原 verdict：`APPROVE` 带 `1 Important（I1）+ 2 Minor（M1/M2）`
- I1 修复后 verdict：**APPROVE `0C / 0I / 2M`**（M1/M2 同前，均为原 report 描述精度问题，不阻塞合并；M2 actionable，留作下一单清理 follow-up）

W15-1i 整体 APPROVE 通过，I1 修复已闭环。建议下一单（任意后续轮次）顺手处理 M2（`io_timeout` 改 `pub(crate)` 一字之差），M1 仅文档勘误。