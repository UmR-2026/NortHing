# W10-1 Judge Review (api.rs split, commit 078af44)

**Verdict: PASS** (SPEC ✅ + QUALITY ✅, 0C / 0I / 1M)
**Date:** 2026-08-29
**Base / Head:** `8fe83ef` → `078af44` (single commit, 5 files, +583/-546)

---

## 双判决总览

| 维度 | 判决 | 证据 |
|---|---|---|
| SPEC | ✅ PASS | 5 项重点核查全部通过；Global Constraints 8/8 满足 |
| QUALITY | ✅ PASS | 复用既有 sibling-module 模式；零新抽象；预算闸逐行等价；god-file 观测点合规 |
| Critical | 0 | — |
| Important | 0 | — |
| Minor | 1 | 报告中"55→52 warnings / 删 3 个 unused imports"数字与 diff 实测不符（实际移走 13 个 import），不影响代码正确性 |

---

## 重点核查项逐项

### 1. 纯位移逐块核对（settings wrapper / event bridge / memory wrapper）

| 块 | 源位置（api.rs 原） | 目的地 | 函数等价 |
|---|---|---|---|
| settings wrapper | L44-214 | `api_settings.rs` | ✅ `get_global_config`/`list_model_configs`/`set_default_provider`/`list_mcp_servers`/`set_mcp_enabled`/`list_skills`/`set_skill_enabled`/`test_provider_config`/`store_provider_api_key*`/`upsert_model_config`/`persist_onboarding_provider*` 逐函数 diff 字节级一致（仅改 `super::super::app_state::settings::*` → `crate::app_state::settings::*` 缩进路径，可接受） |
| event bridge | L216-316 | `api_events.rs` | ✅ `MAX_PENDING_TEXT_CHUNKS=256`、`EventReceiver`、`create_event_bridge`、`event_channel` 整段照搬；预算闸逻辑（CAS-loop + drop / unbounded 控制通道）逐行等价 |
| memory wrapper | L320-332 | `api_memory.rs` | ✅ `list_facts`、`search_facts` 全文等价 |

**pub use re-export 面校验**（brief 要求抽查 3 个调用点）：

| 调用点 | 路径 | 验证 |
|---|---|---|
| `app.rs:284` | `api::submit_turn` | ✅ 走 `pub use super::api_*` 透传，零改动 |
| `pages_settings.rs:116,180` | `super::api::list_model_configs` | ✅ 零改动 |
| `pages_memory.rs:111,140,162` | `api::list_facts` / `api::search_facts` | ✅ 零改动 |
| `app.rs:106` | `api::event_channel` | ✅ 零改动 |

全工作树 `rg "ui_dioxus::api::(submit_turn|list_model_configs|list_facts|search_facts|event_channel|create_event_bridge)"` 仅命中迁移目标模块内引用，对外接口零破损。

### 2. TEST_GLOBAL_CONFIG_MUTEX 归位（brief 重点 2）

- 定义迁移：`api.rs` → `api_settings.rs`（`#[cfg(test)] pub(crate) static TEST_GLOBAL_CONFIG_MUTEX`）
- re-export：`api.rs:21 pub use super::api_settings::*;` —— Rust `pub use` 是同一 static 的别名（不复制），故 `crate::ui_dioxus::api::TEST_GLOBAL_CONFIG_MUTEX` 与 `crate::ui_dioxus::api_settings::TEST_GLOBAL_CONFIG_MUTEX` 指向**同一 `tokio::sync::Mutex<()>` 实例**
- 现有引用路径 `use crate::ui_dioxus::api::TEST_GLOBAL_CONFIG_MUTEX;`（`api_provider_edit.rs:156`）保持不变，编译通过 ✅
- 串行化覆盖：8 处 `_guard = TEST_GLOBAL_CONFIG_MUTEX.lock().await;`（6 in `api_provider_edit.rs` + 2 in `api_settings.rs`），全部共享同一 mutex ✅

### 3. flaky 测试声明真伪判定（brief 重点 3，关键判据）

**复现方案**：MSVC `cargo +stable-msvc test -p northhing --lib --no-fail-fast` 跑 8 次全量套件。

| 实测轮次 | 全量结果 | `test_delete_provider_default_provider_rejected` |
|---|---|---|
| 1 | 140 passed | ok |
| 2 | **139 passed, 1 failed** | **FAILED** |
| 3 | **139 passed, 1 failed** | **FAILED** |
| 4 | 140 passed | ok |
| 5 | 140 passed | ok |
| 6 | 140 passed | ok |
| 7 | 140 passed | ok |
| 8 | 140 passed | ok |

**单跑隔离**：6/6 通过。

**真伪结论**：
- 实现者声称"全量跑 flaky、单跑过"——**事实属实**，本轮实测 8 次套件 2 次失败
- 实现者声称"pre-existing 与本次拆分无关"——**判定为真 pre-existing**：
  - mutex 身份在拆分前后均为同一 `pub(crate) static`（`pub use` 不复制）
  - 拆分未改动 `api_provider_edit.rs` 任何测试代码（git diff 验证该文件 0 行变更）
  - 拆分未改动任何 mutex 使用点（8 处全部分布在原文件，未触任何逻辑）
- 失败根因（推测）：`GlobalConfigManager.initialize` 并发初始化窗口（参见 commit `6574b01` 修复 6 个 pre-existing 测试的同类问题）。与 W10-1 拆分**无因果关系**。

**判定**：按 brief 处置（"真 pre-existing 记观察项；拆分引入 = Important"）→ **记观察项，不打 Important**。观察项建议：

> **观察项 O-1（pre-existing，不阻塞合并）**：`test_delete_provider_default_provider_rejected` 在全量套件中以 ~25% 概率失败。根因疑似 `GlobalConfigManager.initialize` 跨测试并发窗口；现有 `TEST_GLOBAL_CONFIG_MUTEX` 仅覆盖 `*_with_keyring` 路径，未覆盖 `kernel_facade().upsert_model_config(...)` 自身。建议下轮（W10-3 全量测试）排查 6574b01 修复面是否完全覆盖。

### 4. 事件桥 W5-2 分级逻辑逐行等价

| 项 | 原 api.rs | 新 api_events.rs | 等价 |
|---|---|---|---|
| `MAX_PENDING_TEXT_CHUNKS` | 256 | 256 | ✅ |
| TextChunk CAS-loop 上界 | `current >= MAX_PENDING_TEXT_CHUNKS` → drop | 同 | ✅ |
| 控制事件通道 | unbounded_channel | unbounded_channel | ✅ |
| `EventReceiver::recv` 计数回收 | `pending_text_chunks.fetch_sub(1, Relaxed)` | 同 | ✅ |
| `event_channel` runtime fallback | `tokio::runtime::Handle::try_current` → spawn；否则 `std::thread::spawn` 兜底 | 同 | ✅ |
| 测试 `test_tiered_event_channel_text_chunk_lossy_control_guaranteed` | 356 chunks + TurnState::Completed + ToolCall | 同步搬家 | ✅ |
| 测试 `test_tiered_event_channel_drain_refills_budget` | 256 + drop + 10 consume + 10 refill + TurnState::Failed | 同步搬家 | ✅ |
| 测试 `test_event_channel_returns_receiver` | drop(rx) | 同步搬家 | ✅ |

事件桥 3 个测试均通过（实测全量套件 ok）。

### 5. Spec / Constraints 逐条

| Constraints（plan 第 25-34 行） | 满足 |
|---|---|
| C1 只动 `src/apps/desktop` | ✅ 5 文件全部在 `src/apps/desktop/src/ui_dioxus/` |
| C2 日志英文无 emoji；零新增日志 | ✅ diff 无 `tracing::info!`/`log::`/`println!` 新增 |
| C3 SDD 禁区：禁 `.superpowers/` 操作、禁 `progress.md`、禁整树 git 操作 | ✅ commit 内无 `.superpowers/`；本次 judge 操作仅 `git show`/`git diff` 读路径 |
| C4 rot-budget 不上调 ceiling | ✅ `node scripts/verify-rot-budget.mjs` passed |
| C5 验证最小集：`check -p northhing` + `test -p northhing --lib` + rot | ✅ 三项均绿（见下方 § Verification） |
| C6 commit：每任务恰好一个；不含 `.superpowers/` | ✅ single commit `078af44`，5 文件（4 .rs + 1 mod.rs） |
| C7 行为零变化铁律 | ✅ 纯位移，逐块字节级等价（仅缩进路径微调） |
| C8 遇编译错误先加载对应 rust skill | ✅ 本轮无编译错误 |

| Spec 任务目标 | 满足 |
|---|---|
| api.rs 799 → ≤450 | ✅ 实际 266（≤450） |
| 抽出三组（settings/event/memory） | ✅ |
| `mod api_provider_edit;` 保留 | ✅ |
| 模式对齐 sibling module + re-export | ✅ |
| TEST_GLOBAL_CONFIG_MUTEX 跟随 + 跨模块共享（实现者选位置 + report 说明） | ✅ 归位 `api_settings.rs` + `pub use super::api_settings::*;` 保路径 + report §TEST_GLOBAL_CONFIG_MUTEX Placement 说明 |

---

## Verification（最小集实测证据）

| 命令 | 结果 |
|---|---|
| `cargo +stable-msvc test -p northhing --lib test_delete_provider_default_provider_rejected --no-fail-fast` | `1 passed; 0 failed`（隔离单跑） |
| `cargo +stable-msvc test -p northhing --lib --no-fail-fast` ×8 | 见 §3 flaky 表（8 轮中 6 绿 2 红，证实 pre-existing） |
| `node scripts/verify-rot-budget.mjs` | `Rot budget verification passed (5 grep rules ..., 3 dir rules ..., 6 god-file rules checked across 1361 files).` |

---

## 防腐校准自检（brief 第 21 行）

| 项 | 状态 |
|---|---|
| 复用核查（既有的 sibling-module + pub use 模式 = `api_provider_edit` 先例） | ✅ 完全复用，零新模式 |
| 无 owner 抽象（未引入新 trait / interface / config wrapper） | ✅ 纯 split，无新抽象 |
| 预算闸（TextChunk 256 / unbounded control 双重通道） | ✅ 数值与逻辑字节级保留 |
| god-file 观测点：api.rs 266、api_settings.rs 292 | ✅ 两条均 ≤800，无 review 压力 |

---

## Findings 清单

| ID | 等级 | 文件 | 内容 |
|---|---|---|---|
| M-1 | Minor | `w10-1-api-split-report.md` §Warnings Notes | 报告称"55 → 52 warnings (removed 3 split-related unused imports)"；实测 diff 移除 13 个 import 行（`KernelAgentsApi, SkillInfoDto, SkillScopeDto, KernelEventDto, KernelEventsApi, FactDto, KernelMemoryApi, AIModelConfigDto, GlobalConfigDto, KernelSettingsApi, MCPServerDto, ProviderFormDto, ProviderTestResultDto` + `AtomicUsize, Ordering, Arc` 三个 std）。数字不准，但不影响代码正确性 — 不需打回 |

---

## Cannot Verify（brief 第 21 行明令禁止猜测）

无。所有 SPEC / QUALITY 项均有 diff 字节比对或实测输出支撑。

---

## 终审决议

**PASS — CAN MERGE**。

- 单 commit `078af44`，纯位移零行为变化
- god-file 防御（api.rs 799 → 266）达标，下轮 W10-2 `windows.rs` 拆分接续可按相同模式复用
- O-1 flaky 观察项建议 W10-3 全量测试时一并排查 6574b01 修复面

下一站：W10-2 `windows.rs` 拆分（plan Task 2），模式可复用本轮 sibling + `pub use` 形态。