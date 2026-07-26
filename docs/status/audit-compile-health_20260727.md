# 编译健康度审计报告

> 审计日期：2026-07-27 01:49 GMT+8
> 审计员：compile-health auditor (subagent)
> 仓库：E:\agent-project\northing
> 审计目标：northhing bin 36 warning 分类 + K3 ROI 闸门输入准备

---

## 表格1：Warning 分类统计

| 类型 | 数量 | 示例文件 |
|------|------|----------|
| `unused_imports` | 23 | `app_state/mod.rs` (12), `callbacks_settings/refresh.rs` (2), `callbacks_lifecycle.rs` (2), `settings/io.rs` (1), `inspector_model_status.rs` (1), `callbacks_settings/misc.rs` (1), `callbacks_settings/provider.rs` (1), `callbacks_settings/provider_test.rs` (1), `callbacks_settings/workspace.rs` (1), `settings/mod.rs` (1) |
| `unused_variables` | 8 | `callbacks_lifecycle.rs` (7), `callbacks_settings/refresh.rs` (1) |
| `dead_code` | 5 | `sessions.rs` (1), `settings/mod.rs` (1, 3 methods), `settings/types.rs` (3) |
| **合计** | **36** | — |

> 注：`mod.rs:45` 和 `mod.rs:46` 各产生 2 个 warning（glob re-export 可见性 + unused import），故 unused_imports 统计含 2 个 `hidden_glob_reexports` 警告（归入 unused_imports 因根因相同：glob import 无效）。

---

## 表格2：Warning 按文件分布

| 文件 | Warning 数 | 类型明细 |
|------|-----------|----------|
| `app_state/mod.rs` | 14 | 12 unused_imports + 2 glob re-export |
| `app_state/callbacks_lifecycle.rs` | 9 | 2 unused_imports + 7 unused_variables |
| `app_state/settings/types.rs` | 3 | 3 dead_code |
| `app_state/callbacks_settings/refresh.rs` | 3 | 2 unused_imports + 1 unused_variable |
| `app_state/settings/mod.rs` | 2 | 1 dead_code (3 methods) + 1 unused_import (glob) |
| `app_state/sessions.rs` | 1 | 1 dead_code |
| `app_state/settings/io.rs` | 1 | 1 unused_import |
| `app_state/inspector_model_status.rs` | 1 | 1 unused_import |
| `app_state/callbacks_settings/misc.rs` | 1 | 1 unused_import |
| `app_state/callbacks_settings/provider.rs` | 1 | 1 unused_import |
| `app_state/callbacks_settings/provider_test.rs` | 1 | 1 unused_import |
| `app_state/callbacks_settings/workspace.rs` | 1 | 1 unused_import |
| **合计** | **36** | — |

**热点**：`app_state/mod.rs`（14 warnings，占 39%）和 `callbacks_lifecycle.rs`（9 warnings，占 25%）合计占 64%。

---

## 清单3：K4a 遗留 dead_code（可直接删除的）

| # | 位置 | 符号 | K4a 遗留判定 | 建议 |
|---|------|------|-------------|------|
| 1 | `sessions.rs:81` | `build_sessions_model` | **是**：K4a T23 迁移后 session 构建改走 facade DTO 路径，旧 `build_sessions_model` 接收 `SessionSummaryDto` 但调用方已切换到新的 `refresh_sessions_ui` 路径 | **删除**：已无调用方 |
| 2 | `settings/mod.rs:106` | `has_legacy_placeholders` | **是**：K4a T4 迁移后 settings 读写走 facade，legacy placeholder 检测逻辑已被 facade `get_global_config` 替代 | **删除** |
| 3 | `settings/mod.rs:210` | `upsert_mcp` | **是**：K4a T4 迁移后 MCP CRUD 走 facade `upsert_mcp_server`，旧 `AppSettings::upsert_mcp` 不再被调用 | **删除** |
| 4 | `settings/mod.rs:218` | `remove_mcp` | **是**：同上，已被 facade `delete_mcp_server` 替代 | **删除** |
| 5 | `settings/types.rs:47` | `ProviderType::display_label` | **是**：K4a 迁移后 provider 显示名通过 facade DTO 获取，本地 `display_label` 不再被 UI 调用 | **删除** |
| 6 | `settings/types.rs:140` | `SkillState::effective_in` | **是**：K4a T4 迁移后 skills 面板走 facade `list_skills`/`set_skill_enabled`，`effective_in` 的 workspace 级判断已被 facade `SkillScopeDto` 替代 | **删除** |
| 7 | `settings/types.rs:179` | `MCPServerConfig::new` | **是**：K4a 迁移后 MCP 配置构造走 facade `MCPServerDto`，本地 `MCPServerConfig::new` 不再被调用 | **删除** |

**结论**：全部 7 个 dead_code 符号均为 K4a 迁移遗留，旧 API 已被 facade 完全替代，可直接删除。无 feature-gated 代码的 dead_code。无需要保留的 dead_code。

---

## K3 ROI 闸门评估输入

### 1. 编译时间对比

| 指标 | K4a 前（基线） | K4a 后（K0 实测） | 当前实测（2026-07-27） |
|------|---------------|-------------------|----------------------|
| touch 增量 `cargo check -p northhing` | — | 3.40s (judge-m3) | **6.85s** |
| touch 增量 `cargo check --workspace` | — | 3.40s | **84.0s** |
| clean build `cargo check -p northhing` | — | — | **67.76s** |
| K4a 设计目标 | — | min(30s, 14.93×0.5)=7.47s | — |

**分析**：
- K4a 设计时 K0 基线 3.40s 远超目标 7.47s，判定"编译收益超额达成"。
- 当前 touch 增量 6.85s 仍低于 7.47s 目标，但比 K0 实测的 3.40s 有约 2× 膨胀。
- 膨胀原因推测：K4a 完工后（2026-07-26）至本次审计（2026-07-27）期间可能有新代码增量（T5 清扫项等后续 commit），或编译缓存状态差异。
- **K3 闸门判定**：编译收益目标仍达成（6.85s < 7.47s target），但余量从 4.07s 缩减至 0.62s，需关注趋势。

### 2. 当前 Warning 数量趋势

| 时间点 | Warning 数 | 来源 |
|--------|-----------|------|
| K4a 前 | 未记录 | — |
| K4a 后（K0） | 未记录 | — |
| 当前（2026-07-27） | 36 (northhing bin) + 1 (w4_repro) + 20 (northhing-core lib) + 4 (services-integrations) + ~17 (Slint UI padding) | 本次实测 |

**分析**：
- northhing bin 的 36 个 warning 全部为 K4a 迁移直接产物（unused imports 因 import 路径切换、dead_code 因旧 API 废弃）。
- 这些 warning 不影响功能正确性，但表明 K4a 迁移后的"清扫"步骤（T5 grep 守卫）未覆盖 import 清理。
- **趋势**：若不清理，随代码增长 warning 数只会增不会减。

### 3. Desktop crate 对 northhing-core 的依赖切断状态

| 检查项 | 状态 | 详情 |
|--------|------|------|
| `Cargo.toml` 中 `northhing-core` 依赖 | **仍存在** | `northhing-core = { path = "../../crates/assembly/core", default-features = false, features = ["product-full"] }` |
| 代码面 `northhing_core::` 引用 | **21 行残留** | 全部命中 K4a §6 豁免清单 |
| 豁免分类 | ✅ 合规 | `kernel_facade()` 手柄调用（17 行）、`shutdown_mcp_servers`（1 行）、`w4_repro.rs`（5 行）、`state.rs coordinator()`（2 行，§12 缺口 5 豁免）、`mcp_adapter.rs KernelFacade`（1 行，D2-A' 保留） |
| `cargo tree -p northhing-kernel-api` 禁止依赖 | **零命中** | 对 `rmcp\|git2\|axum\|tower-http\|reqwest\|northhing-core` 零命中 ✅ |

**判定**：
- **Cargo 依赖未切断**：desktop 仍依赖 `northhing-core`（按 K4a §6 修订口径，composition-root 手柄 `kernel_facade()` 住在 core 内 + w4_repro 豁免，**保留依赖是设计决策，不是遗漏**）。
- **代码面解耦达标**：21 行残留全部命中豁免清单，无违规引用。
- **facade 依赖链干净**：`kernel-api` 不传递 `northhing-core` 及重依赖。

### 4. Kernel-API Facade 覆盖度

| 域 | Facade 方法 | Desktop 需求覆盖 |
|----|-----------|-----------------|
| coordination (turn) | `submit_turn`, `stop_turn`, `get_turn_state` | ✅ 全覆盖 |
| session | `create_session`, `list_sessions`, `get_session`, `delete_session`, `rename_session`, `get_messages`, `create_branch`, `archive_session`, `get_session_metadata` 等 | ✅ 全覆盖 |
| events | `subscribe_events`, `unsubscribe_events`, `emit_backend_event` | ✅ 全覆盖 |
| bootstrap | `init_core` | ✅ 全覆盖 |
| settings/config | `get_global_config`, `update_global_config`, `list_model_configs`, `upsert_model_config`, `delete_model_config`, `set_default_provider` | ✅ 全覆盖 |
| MCP | `list_mcp_servers`, `upsert_mcp_server`, `delete_mcp_server`, `get_mcp_status` | ✅ 全覆盖 |
| skills | `list_skills`, `get_skill`, `set_skill_enabled`, `load_skill_overrides`, `resolve_skill_default_enabled` 等 | ✅ 全覆盖 |
| provider test | `test_provider`, `test_provider_config` | ✅ 全覆盖 |
| tools | `list_tools`, `register_tool` | ✅ 全覆盖 |
| usage | `generate_session_usage`, `render_usage_markdown`, `get_token_usage` | ✅ 全覆盖 |
| agents | `list_agents`, `list_subagents` | ✅ 全覆盖 |
| memory | `list_episodes` 等 | ✅ 全覆盖 |
| platform | `open_terminal`, `analyze_image`, `get_core_health` 等 | ✅ 全覆盖 |
| debug_log | **无 facade 方法** | ⚠️ 已按 D1 决策拆为 `northhing-debug-log` 微 crate（T5 已落地） |
| shutdown_mcp_servers | **无 facade 方法** | ⚠️ 豁免保留直连 core（§6 豁免清单①） |
| set_actor_runtime / coordinator() | **无 facade 方法** | ⚠️ 豁免保留直连 core（§12 缺口 5，待 P2 评审） |

**Facade 方法总数**：55 个 async fn + 1 个 sync fn（`strip_prompt_markup`）= 56 个公开方法（设计文档记录 53 满额，实测有 55 async + 1 sync = 56，差异可能因 T23q/T4p DTO 补缺时新增辅助方法）。

**覆盖判定**：facade 完整覆盖 desktop 产品面需求。3 个缺口（debug_log / shutdown / actor_runtime）均已按 K4a 设计决策豁免，不影响 K3 闸门。

---

## 建议

### 应立即修复（低风险、高收益）

| 优先级 | 类型 | 范围 | 预计工作量 | 理由 |
|--------|------|------|-----------|------|
| P0 | dead_code | 删除清单3 全部 7 个符号 | 15 min | K4a 遗留死代码，无调用方，直接删除，零风险 |
| P1 | unused_imports | 清理 `app_state/mod.rs` 的 12 条无效 import | 10 min | K4a 迁移后 import 路径切换残留，`cargo fix` 可自动处理 29/36 条 |
| P1 | unused_imports | 清理 `callbacks_lifecycle.rs` 的 2 条无效 import | 5 min | 同上 |
| P2 | unused_variables | `callbacks_lifecycle.rs` 7 个 `app_state` 参数加 `_` 前缀 | 5 min | 这些 callback 注册函数暂不需要 `app_state`，但保留参数为未来扩展；加 `_` 前缀即可 |
| P2 | unused_imports | 清理 `callbacks_settings/` 下 6 条无效 import | 5 min | `SharedString`/`ComponentHandle`/`ModelRef` 在迁移后不再使用 |

### 可忽略（无需修复）

| 类型 | 范围 | 理由 |
|------|------|------|
| Slint UI padding warnings | ~17 条 `.slint` 文件 | Slint 编译器对非布局元素的 padding 属性发出，是 UI 框架行为，不影响编译结果 |
| `northhing-core` lib 的 20 个 warning | core 内部 | 不在本次审计范围（desktop crate），需 core 侧独立清理 |
| `services-integrations` 的 4 个 deprecated warning | `rmcp`/`sse_stream` 依赖 | 第三方库 API 弃用警告，需等依赖升级 |
| `w4_repro.rs` 的 1 个 unused_variable | `input` 参数 | D3 豁免的 dev bin，不阻塞 |

### 一键修复建议

```powershell
cd E:\agent-project\northing
# 自动修复 29/36 条（unused_imports + unused_variables 的 _ 前缀建议）
cargo fix --bin "northhing" -p northhing --allow-dirty
# dead_code 需手动删除（cargo fix 不会删函数）
```

---

## K3 ROI 闸门汇总

| 闸门输入 | 状态 | 数据 |
|---------|------|------|
| 编译时间对比 | ✅ 达标 | 当前 6.85s < 目标 7.47s（K4a 前 14.93s），但余量缩小至 0.62s |
| Warning 数量趋势 | ⚠️ 需关注 | 36 个 warning 全为 K4a 迁移产物，未清理；建议 K3 启动前清零 |
| Desktop → core 依赖切断 | ✅ 按设计口径达标 | Cargo 依赖保留（设计决策），代码面 21 行全命中豁免，facade 依赖链零禁止依赖 |
| Facade 覆盖度 | ✅ 完整 | 56 方法覆盖 desktop 全部产品面需求；3 个缺口已按设计豁免 |
| **K3 闸门总判定** | **条件达标** | 编译收益仍在目标内；建议先清理 36 warning 再启动 K3（约 40 min 工作量） |

> **编者注**：K4a 设计文档 §13 已明确"编译目标已在 K4a 达成，K3（kernel 下沉）符合'降级为有空再做'条件"。本次审计确认编译收益仍维持（6.85s < 7.47s），K3 可按低优先级排期。但 warning 堆积是技术债信号，建议在 K3 启动前清零以降低迁移噪音。

---

*报告结束*
