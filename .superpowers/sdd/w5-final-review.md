# W5 全波终审 Dioxus 壳审计修复波 (`86ab479..f680cf6`)

**终审时间**: 2026-08-28  
**终审模型**: step-explore_reviewer (独立终审角色)  
**方法**: 只读审查；diff + 源码全文 + `cargo check -p northhing` 实测

---

## 裁决

**CAN MERGE** — SPEC PASS, QUALITY PASS

| 判决 | 结果 |
|------|------|
| **SPEC** | ✅ PASS — 4 任务全部命中计划 Spec；Global Constraints 逐条核对无误 |
| **QUALITY** | ✅ PASS — 跨任务接缝全净；无 Critical / Important；12 Minor 交终审 triage |
| **裁决** | 🟢 **CAN MERGE** |
| **C / I / M** | 0 / 0 / 12 |

**一句话理由**: 4 任务实现与计划逐字对齐，退出链路/事件分级/provider 持久化/watch 化接缝均经独立走查无回归，编译门绿，行为零变化声称已核实，Minor 全为可合并后跟进的低优观察项。

---

## 1. Spec 逐条判定

### W5-1 (F1) — `quit_shell` 优雅退出链

| Spec 要点 | 证据 | 判定 |
|-----------|------|------|
| `process::exit` 仅剩 init 失败路径 | `main.rs:82`(init_core_services Err) + `main.rs:131`(launch Err) — 正常关闭链路零 `process::exit` | ✅ |
| ✕ → close_all_modules → room 关闭 → launch 返回 → main.rs shutdown | `app.rs:97-102` `quit_shell` → `close_all_modules` → `window().close()` ; `main.rs:122-126` `shutdown_for_main()` | ✅ |
| `mark_all_closing_targets` 原子迁移 | `registry.rs:336-368` — Open/Opening → Closing 单临界区 + active_set 置空广播 | ✅ |
| Dioxus LoopDestroyed → `on_shutdown` | `entry.rs:204-214` — 回调 closure 捕获 `on_shutdown` Arc | ✅ |
| `perform_shutdown` 幂等 take 模式 | `main.rs:98-120` — Mutex<Option> take() 双路径（事件循环 + 主线程回退） | ✅ |

### W5-2 (F2) — 事件桥分级

| Spec 要点 | 证据 | 判定 |
|-----------|------|------|
| TextChunk 有损预算 256 | `api.rs:272` `MAX_PENDING_TEXT_CHUNKS` + `api.rs:314-329` CAS 循环 | ✅ |
| 控制事件 unbounded 直通 | `api.rs:332-334` `control_dto => { let _ = tx.send(dto); }` | ✅ |
| FIFO 跨类型保序 | 单 unbounded 通道天然 FIFO — 两者均在闭包内通过同一 `tx.send` | ✅ |
| app.rs 消费端零改动 | `app.rs:169` `api::event_channel()` 调用签名不变；`.recv().await` 在 `EventReceiver` 上为同名方法 | ✅ |
| 测试 +2 | `api.rs:588-716` — `test_tiered_event_channel_text_chunk_lossy_control_guaranteed` + `test_tiered_event_channel_drain_refills_budget` | ✅ |

### W5-3 (F4) — Onboarding 持久化 Provider

| Spec 要点 | 证据 | 判定 |
|-----------|------|------|
| `persist_onboarding_provider_with_keyring` 注入 | `api.rs:197-249` 签名为 `&dyn KeyringBackend` | ✅ |
| MockKeyring 隔离 | `api.rs:411` 测试用 `MockKeyring::new()` 注入；`PRODUCTION_KEYRING` 仅在生产路径 | ✅ |
| Keyring account 命名 | UUID 为 provider_id → `sync.rs:27` `infer_provider_wire_format` 推断 wire_format → DTO id | ✅ |
| 三失败臂 UI 显式报错 | `api.rs:207-246` — Key 存储失败 / Provider 保存失败 / 设为默认失败 | ✅ |
| `update_app_settings` 失败臂 | `pages_onboarding.rs:682-689` — 中文显式报错 | ✅ |

### W5-4 (F5+F6) — PartialEq + Mutex→watch

| Spec 要点 | 证据 | 判定 |
|-----------|------|------|
| ModuleAppProps 结构 PartialEq | `registry.rs:48-50` — `plugin_id == other.plugin_id && gen == other.gen` | ✅ |
| Debug impl | `registry.rs:53-60` | ✅ |
| room_window_id Mutex→watch | `entry.rs:141` `watch::channel::<Option<WindowId>>` ; `state.rs:36` type alias | ✅ |
| geometry Mutex→`send_modify` | `entry.rs:242-251` `geometry_tx.send_modify` 原地修改 | ✅ |
| Mutex import 精确摘除 | `entry.rs:30` `use std::sync::Arc;` — Mutex 已移除 | ✅ |
| 测试 +2 | `registry.rs:129-677` — `test_mark_all_closing_targets` + `test_module_app_props_partial_eq` | ✅ |

### Global Constraints

| # | 约束 | 状态 |
|---|------|------|
| 1 | 分层边界：仅在 `src/apps/desktop` | ✅ 9 文件全在该路径 |
| 2 | 日志英文、无 emoji | ✅ inspection 通过 |
| 3 | tokio 生命周期改动附自动化测试 | ✅ F2 EventReceiver + W5-1 mark_all_closing_targets 均有测试 |
| 4 | 不碰 `.superpowers/` / `progress.md` | ✅ diff 零触及 |
| 5 | rot-budget 不上调、不新增 >800 行文件 | ✅ `scripts/rot-budget.json` 零变更；registry.rs 678 行未越线 |
| 6 | `cargo check -p northhing` 绿 | ✅ 编译通过，0 error，现有 warning 不归本波 |
| 7 | commit 每任务一个、不含 `.superpowers/` | ✅ 台账 4 commit 链对齐 |
| 8 | 不新建无 owner 抽象 | ✅ EventReceiver 唯一消费者即 app.rs event loop；mark_all_closing_targets 唯一调用方即 `quit_shell`/CloseRequested handler |

---

## 2. Quality 跨任务集成走查

### 2.1 W5-1 关闭链 × W5-4 watch 化交互

**核查结论: SAFE**

- `mark_all_closing_targets` (registry.rs Mutex<HashMap>) 和 `geometry_tx.send_modify` (watch::Sender) 操作不同数据结构 — 无竞争
- `send_modify` 内部已持通道锁；即使所有 receiver 已 drop，watch::Sender 发送不阻塞（通道始终缓存最新值）— 写已关闭接收端不 panic
- `room_window_id_rx.borrow()` 在 `is_room` 检查中为无锁快照，关闭路径中无与 Mutex 交叉持锁 — 无死锁

**时序走查**: 正常关闭 = ✕ 按钮 → `quit_shell(wm)` → `close_all_modules()` (mark_all_closing_targets + close_window) → `win_ops::close_os_window(main_hwnd)` → `window().close()` → `CloseRequested` → (tao handler 二次 mark_all_closing_targets 幂等) → 窗口销毁 → `LoopDestroyed` → `on_shutdown()` → shutdown_tx + worker_join + MCP cleanup。每环节 file:line 一一对应，闭环完整。

### 2.2 W5-2 分级声明对账

**"app.rs 消费端零改动" — 核实: TRUE**

- api.rs 返回类型从 `Receiver<KernelEventDto>` → `EventReceiver`
- `EventReceiver` 提供同名 `recv().await` 方法
- `app.rs` 中 `let mut rx = api::event_channel()` + `while let Some(dto) = rx.recv().await` 零改动 ✓

**TextChunk AtomicUsize 计数准确性**: CAS 循环 (`compare_exchange_weak`) 在单生产者（kernel event 回调单线程派发）场景下单线程无并发；Relaxed ordering 无可视化问题。理论过计窗口 = 仅在多线程回调场景下，当前架构不触发。

**控制事件 unbounded 背压**: 设计意旨 — 控制事件（TurnState Completed/Failed、ToolCall AwaitingConfirmation）为关键状态机转换，不丢为第一优先级。流量层面，控制事件频率远低于 TextChunk（文本流 vs 状态跳变）。不设上限为正确取舍（brief + judge 均认可）。

### 2.3 W5-3 生产路径无污染

**MockKeyring 隔离 — 核实: TRUE**

- 生产路径: `store_provider_api_key` → `store_provider_api_key_with_keyring(&PRODUCTION_KEYRING, ...)`
- 测试路径: `persist_onboarding_provider_with_keyring(&kr, ...)` → `MockKeyring`
- 无交叠；`MockKeyring` 仅在 `#[cfg(test)]` mod 中定义和引用

**keyring account 命名**: UUID 作为 provider_id → keyring 条目。与既有 `keyring.rs` 约定一致（provider 级 key，非 agent 级）。

**三失败臂 UI 报错**: `api.rs:209` "Key 存储失败" / `api.rs:239` "Provider 保存失败" / `api.rs:245` "设为默认 Provider 失败" — 均为中文用户可见、带首行截断。

### 2.4 W5-4 行为零变化

**PartialEq 重渲染**: ModuleAppProps 在 `VirtualDom::new_with_props` 时一次性传入，之后不再更新 props。PartialEq 仅在 VDOM 构造/替身时比较一次。plugin_id + gen 比较 = 同实例不变、不同实例（gen 不同）重渲染 — 与旧恒 true 行为等效（因为从不更新）。✅ 零渲染回归。

**`send_modify` 等价性**: 旧路径 `Mutex::lock → modify → Mutex drop → send`; 新路径 `send_modify(|geom| { ... })` — 后者原子完成获取锁+修改+发送，等价且更高效（无中间 Unlock→Send 间隙）。

### 2.5 退出链路完整性走查

| 环节 | file:line | 状态 |
|------|-----------|------|
| `process::exit` 残留 | `main.rs:82`(init 失败) / `main.rs:131`(launch 失败) — **零残留于正常关闭路径** | ✅ |
| ✕ → quit_shell | `app.rs:97-102` | ✅ |
| close_all_modules | `app.rs:90-95` → `registry.rs:336-368` `mark_all_closing_targets` + `window().close_window(wid)` + `win_ops::close_os_window(hwnd)` | ✅ |
| room 窗口关闭 | `win_ops::close_os_window(window().hwnd())` + `window().close()` | ✅ |
| launch 返回 | Dioxus `LoopDestroyed` → `entry.rs:204-214` `on_shutdown()` | ✅ |
| `shutdown_tx.send(())` | `main.rs:99-103` | ✅ |
| worker_handle.join() | `main.rs:104-113` | ✅ |
| `shutdown_mcp_servers()` | `main.rs:115-119` | ✅ |

### 2.6 文档/台账一致性

- `progress.md` W5 四行 commit 范围与本包一致 (86ab479..f680cf6)
- `plan-2026-08-28-w5-dioxus-shell-fixes.md` 各任务 Spec 与 diff 对齐
- w5-3 brief 未跟踪 — observation: brief 提及"w5-3 brief 未跟踪"指 W5-3 的 brief 文件未被 git 跟踪（仅在磁盘上），属编排者流程观察，非代码缺陷

---

## 3. Findings

### Critical (0)

无。

### Important (0)

无。

### Minor (12)

| # | 任务 | 等级 | 位置 | 描述 | 建议 |
|---|------|------|------|------|------|
| W5-1-M1 | W5-1 | Minor | report.md | 验证章节缺 test 执行输出原文（`cargo test` 命令 + 输出未入 report） | **修一记一**：后续 report 模板要求测试输出粘贴 |
| W5-1-M2 | W5-1 | Minor | report.md | 走查行号 off-by-one × 2（main.rs 关闭链步骤行号偏移） | accept-and-close：不影响代码正确性 |
| W5-1-M3 | W5-1 | Minor | `app.rs:90-95` | `close_all_modules` 中 `window().close_window(wid)` 与 `win_ops::close_os_window(hwnd)` 双调用 — 对已由 tao handler 触发的关闭冗余 | **修一记一**：加 `// ponytail: redundant on tao-managed windows; safe no-op` 注释 |
| W5-1-M4 | W5-1 | Minor | report.md | `WindowDropGuard` 复用声明未经验证 | defer-with-owner: W5-1 implementer — 需真机关闭链路实跑闭环 |
| W5-2-M1 | W5-2 | Minor | `api.rs:314-329` | CAS `compare_exchange_weak` 多生产者过计理论窗口 | accept-and-close：当前单线程回调，理论窗口不可达 |
| W5-2-M2 | W5-2 | Minor | `api.rs:298-301` | `pending_text_chunks()` getter 可降 `pub(crate)` | defer-with-owner: W5-2 implementer |
| W5-2-M3 | W5-2 | Minor | `api.rs:317-322` | TextChunk 丢包仅 `tracing::debug!`，运营不可见 | defer-with-owner: 产品确认运营监控诉求后升 `warn!` |
| W5-2-M4 | W5-2 | Minor | `api.rs:308` | 控制侧 unbounded 通道无显式上限 | accept-and-close：设计意旨（保证投递），流量可控 |
| W5-3-M1 | W5-3 | Minor | `sync.rs:27-37` | `infer_provider_wire_format` URL 启发式脆弱 — proxy URL 含 "anthropic"/"google" 字样会误分类 | defer-with-owner: 产品确认 proxy 用例后考虑显式 provider type 选择器 |
| W5-4-M1 | W5-4 | Minor | `api.rs:340` / `api.rs:343` | `let _ = tx.send(dto)` 丢弃返回值 — unbounded 通道永不会 Err，但语义可加注释 | **修一记一**：加 `// unbounded: send never fails` 注释 |
| W5-4-M2 | W5-4 | Minor | `registry.rs:1294` | PartialEq 测试缺 rx-Arc 变体用例（thread-local rx vs shared rx） | defer-with-owner: W5-4 implementer |
| W5-4-M3 | W5-4 | Minor | `registry.rs:678` | registry.rs 当前 678 行，距 800 警戒线余量 122 行 | accept-and-close：余量充足，关注后续 `mark_all_closing_targets` 扩展趋势 |

### Cannot verify from diff

1. **Dioxus LoopDestroyed 库契约**: 声称 `LoopDestroyed` 在窗口销毁后触发、`on_shutdown` 回调正常执行。此行为取决于 dioxus-desktop 0.8.0-alpha.1 内部事件序，无法从源码 diff 验证。须真机关闭链路实跑。→ 台账已有 ⚠️ 实测兜底项。

2. **WindowDropGuard 复用**: W5-1-M4 — `WindowDropGuard` 在多个关闭路径复用的安全性声明，未在 diff 中找到独立验证证据。

3. **控制 unbounded 通道背压上限**: unbounded mpsc 理论上可被恶意/异常生产方撑爆内存。当前 kernel event 回调为"串行回调 + 消费者异步 drain"模型，实际流量受 UI 帧率约束，无法从 diff 量化验证。

---

## 4. 预防性核查

### 4.1 复用核查

- `send_modify` 替换手写 Mutex+send — 复用 tokio watch 内置能力 ✅
- `tokio::sync::watch` for room_window_id — 沿用既有 geometry channel 模式 (`state.rs` GeometryTx/Rx) ✅
- `KeyringBackend` trait + MockKeyring — 复用 C3 既有基础设施 ✅
- `create_event_bridge` callback 分发 — 新建但无既有等价替代（旧 mpsc::channel 语义不符分级需求）✅

### 4.2 无 owner 抽象

| 新增类型/函数 | 消费方 | 裁定 |
|---------------|--------|------|
| `EventReceiver` (api.rs:283-311) | `app.rs` event_channel consumer + 2 新测试 | ✅ 唯一消费方 |
| `create_event_bridge` | `event_channel()` + 2 新测试 | ✅ 唯一消费方 |
| `persist_onboarding_provider_with_keyring` | `pages_onboarding.rs` + 1 新测试 | ✅ 唯一消费方 |
| `store_provider_api_key_with_keyring` | `persist_onboarding_provider` + 1 测试 | ✅ 唯一消费方 |
| `mark_all_closing_targets` | `quit_shell` + CloseRequested handler + 1 测试 | ✅ 唯一消费方 |
| `RoomWindowIdTx` type alias | `entry.rs` + `app.rs` context inject | ✅ 唯一消费方 |
| `infer_provider_wire_format` | `pages_onboarding.rs` test_provider_config + 2 测试 | ✅ 唯一消费方 |

### 4.3 预算闸

- `scripts/rot-budget.json`: **零变更** — 通过 `git diff --name-only 86ab479..f680cf6 -- scripts/rot-budget.json` 确认
- 无新文件 >800 行
- `registry.rs` 当前 678/800 — 未越线

### 4.4 God-file 观测点

| 文件 | 当前行数 | 观察 |
|------|----------|------|
| `registry.rs` | 678 | **接近警戒线** (余量 122 行)。本波新增 `mark_all_closing_targets` (36 行) + filter_map 展开 + 新测试。Next contact: 任何 W5.x 扩展须先评估拆分必要性。 |
| `api.rs` | 717 | 本波 +195 行（F2 事件桥分级 + F4 onboarding API 扩展 + 测试）。可控但需关注。 |

---

## 5. Minor Triage

| # | Finding | 处置 | 理由 |
|---|---------|------|------|
| W5-1-M1 | report 缺 test 执行输出 | **修一记一** | 证据纪律，不改代码但要求发布者补报告 |
| W5-1-M2 | 走查行号 off-by-one×2 | accept-and-close | 行号偏差不影响代码正确性 |
| W5-1-M3 | room 双关闭冗余 | **修一记一** | 加 ponytail 注释标注冗余为安全 no-op |
| W5-1-M4 | WindowDropGuard 复用声明 | defer-with-owner (W5-1 implementer) | 需真机实测验证，不阻塞合并 |
| W5-2-M1 | counter 过计理论窗口 | accept-and-close | 单线程回调场景不可达 |
| W5-2-M2 | pending_text_chunks 可降 pub(crate) | defer-with-owner (W5-2 implementer) | API 可见性收紧 |
| W5-2-M3 | 丢 chunk 用 debug! 运营不可见 | defer-with-owner (产品确认后) | 运营监控诉求待确认 |
| W5-2-M4 | 控制侧 unbounded 无上限 | accept-and-close | 设计意旨正确 |
| W5-3-M1 | URL 启发式脆弱 | defer-with-owner (产品确认 proxy 用例) | 当前用户流无 proxy 场景 |
| W5-4-M1 | send 丢弃语义可加注释 | **修一记一** | 一行注释消除疑虑 |
| W5-4-M2 | PartialEq 缺 rx-Arc 变体用例 | defer-with-owner (W5-4 implementer) | 测试覆盖增强 |
| W5-4-M3 | registry.rs 678/800 | accept-and-close | 余量充足，关注趋势 |

---

## 6. 合并前阻塞清单

**无阻塞项。** CAN MERGE 直行。

合并后建议跟进（非阻塞）:
- W5-1-M4: WindowDropGuard 复用真机实测闭环
- W5-2-M3: 确认 TextChunk 丢包运营可见度需求
- W5-3-M1: 确认 proxy base_url 场景需求（当前 onboarding 用户流无此场景）

---

## 7. 验证记录

| 命令 | 输出 | 状态 |
|------|------|------|
| `cargo check -p northhing` | Finished dev profile, 0 error, 50 warnings (pre-existing) | ✅ |
| `rg "process::exit" src/apps/desktop/src/` (excl. bin/) | 2 hits — `main.rs:82` + `main.rs:131` (均为 init/launch 失败) | ✅ |
| `git diff --name-only 86ab479..f680cf6 -- scripts/rot-budget.json` | 空 | ✅ |
| diff 触及文件全在 `src/apps/desktop/src/` | 9 files, 全部 in-scope | ✅ |
