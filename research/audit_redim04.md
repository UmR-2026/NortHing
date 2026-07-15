## 依赖与编译重审报告

> **审核角色**：依赖与编译重审员_04 
> **审核日期**：2026-06-27 
> **基线参考**：`research/audit_dim04.md` — **未找到**（本次审核为独立基线）

---

### 1. 编译状态重审

| 指标 | 当前状态 | 基线 | 变化 |
|------|----------|------|------|
| `target/debug` 大小 | **970 MB** | 180 GB | ✅ **下降 99.5%** |
| `cargo clean` 状态 | 已执行/目录重建 | 180 GB 膨胀 | ✅ **已清理** |
| `target` 目录结构 | 仅 `debug/` + `CACHEDIR.TAG` | 未记录 | ✅ 正常 |

**结论**：`target/debug` 从 180 GB 降至 970 MB，说明已执行 `cargo clean` 或增量清理，编译产物回归正常范围。

---

### 2. 编译配置更新

**Workspace `Cargo.toml` 中的 profile 配置**（第 241–254 行）：

```toml
[profile.dev]
incremental = true

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.release-fast]
inherits = "release"
lto = false
codegen-units = 16
strip = false
incremental = true
```

**发现**：

| 配置项 | 状态 | 说明 |
|--------|------|------|
| `split-debuginfo` | ❌ **未配置** | 未使用 `split-debuginfo = "unpacked"` 或 `"packed"` 来减少 target 体积 |
| `opt-level` (dev) | ❌ **未配置** | 默认 `opt-level = 0`，可考虑 `opt-level = 1` 平衡编译速度与运行时性能 |
| `target-dir` | ❌ **未配置** | 未使用 `CARGO_TARGET_DIR` 或 `.cargo/config.toml` 重定向 target 目录 |
| `sccache` | ❌ **未配置** | 未在 `.cargo/config.toml` 或环境变量中启用 `sccache` |
| 新增 profile | ⚠️ 新增 `release-fast` | 用于快速发布构建，但未验证 codegen-units=16 时的 LTO 关闭是否足够 |

---

### 3. core feature 拆分状态

**`src/crates/assembly/core/Cargo.toml` 第 162–171 行**：

```toml
[features]
# Empty default: product surface must be opted into explicitly via `features`
# on each consumer (`desktop`, `cli`, `cli-internal`, `acp`). This stops the
# `default = ["product-full"]` chain from forcing the full feature set on any
# transitive consumer that forgets to set `default-features = false`, and
# avoids the 39-version recompile cascade observed in build audits.
#
# `product-full` stays defined below as the compatibility product assembly
# boundary; do not delete it without a migration review.
default = []
```

| 检查项 | 状态 | 说明 |
|--------|------|------|
| `default` feature | ✅ **已改为 `[]`** | 原为 `default = ["product-full"]`，已清空 |
| `product-full` feature | ✅ **保留作为兼容边界** | 显式定义，需消费者主动 opt-in |
| 注释说明 | ✅ **完整** | 明确提到修复 39 版本重编译级联问题 |
| 下游依赖是否使用 `default-features = false` | 需消费者自行确认 | core 自身已修复，但 consumer 是否仍显式声明需检查 |

**结论**：`northhing-core` 的 `default` feature 拆分 **已正确实施**。这是 target 从 180 GB 降到 970 MB 的根本原因。

---

### 4. 依赖版本统一状态

**检查文件**：`northing-installer/src-tauri/Cargo.toml` 
**注意**：Workspace 的 `exclude` 列表写的是 `northhing-installer/src-tauri`（双 h），而实际目录为 `northing-installer/src-tauri`（单 h）。拼写不匹配导致 **exclude 可能未生效**，installer 实际上仍被包含在 workspace 中。

**installer 依赖 vs workspace 依赖对比**：

| 依赖 | installer 版本 | workspace 版本 | 统一状态 |
|------|---------------|----------------|----------|
| `tauri` | `2` | `2.11` | ❌ 不匹配 |
| `tauri-plugin-dialog` | `2` | `2.7` | ❌ 不匹配 |
| `tauri-build` | `2` | `2.6` | ❌ 不匹配 |
| `serde` | `1` | `1.0` | ⚠️ 语义兼容但写法不统一 |
| `serde_json` | `1` | `1.0` | ⚠️ 语义兼容但写法不统一 |
| `tokio` | `1` | `1.52` | ⚠️ 语义兼容但写法不统一 |
| `tokio-stream` | `0.1` | `0.1.18` | ⚠️ 语义兼容但写法不统一 |
| `dirs` | `5.0` | `6.0` | ❌ **MAJOR 版本— 突** |
| `zip` | `0.6` | `4.6` | ❌ **MAJOR 版本— 突**（且 `0.6` 是较新 major） |
| `reqwest` | `0.12` | `0.13.4` | ❌ **MAJOR 版本— 突** |
| `urlencoding` | `2` | `2.1` | ⚠️ 语义兼容但写法不统一 |
| `futures` | `0.3` | `0.3.31` | ⚠️ 语义兼容但写法不统一 |
| `eventsource-stream` | `0.2` | `0.2.3` | ⚠️ 语义兼容但写法不统一 |
| `anyhow` | `1.0` | `1.0` | ✅ 一致 |
| `log` | `0.4` | `0.4` | ✅ 一致 |
| `flate2` | `1.0` | `1.0` | ✅ 一致 |
| `chrono` | `0.4` | `0.4` | ✅ 一致（但 features 未继承） |
| `tar` | `0.4` | **未定义** | ⚠️ 新增依赖 |
| `norththing-ai-adapters` | `path = "..."` | **未定义** | ⚠️ 独立路径引用 |

**额外发现**：installer 自身还定义了独立的 `[profile.release]` 和 `[profile.release-fast]`，与 workspace 的 profile 配置 **重复且不完全一致**（`opt-level = "z"` vs workspace `opt-level = 3`）。由于 installer 的 `exclude` 拼写错误，这些 profile 可能与 workspace — 突。

**结论**：❌ **未统一**。installer 几乎完全未使用 `workspace = true` 继承依赖版本，且存在多组 major 版本— 突（`dirs` `5.0` vs `6.0`、`zip` `0.6` vs `4.6`、`reqwest` `0.12` vs `0.13.4`）。建议迁移为 `workspace = true` 并修正 exclude 拼写。

---

### 5. 边界泄露修复状态

**grep 结果**：CLI 仍大量直接引用 `northhing_core::agentic::core::...`、`northhing_core::agentic::coordination::...`、`northhing_core::agentic::events::...`、`northhing_core::agentic::tools::...`、`norththing_core::infrastructure::...`、`norththing_core::service::config::...` 等内部模块。

**具体泄露分布**（按文件统计）：

| 文件 | 泄露类型 | 行数示例 |
|------|----------|----------|
| `acp_cli.rs` | `service::config`, `infrastructure` | 120, 126, 128, 421, 424, 427 |
| `chat_state.rs` | `agentic::core::message`, `agentic::core::strip_prompt_markup` | 9, 12 |
| `main.rs` | `infrastructure::ai`, `service::config` | 371, 373, 379, 383, 444 |
| `agent/agentic_system.rs` | `infrastructure::ai`, `service::config` | 3, 4 |
| `agent/core_adapter.rs` | `agentic::coordination`, `agentic::core`, `agentic::events`, `agentic::tools` | 12, 15, 16, 340 |
| `management.rs` | `infrastructure`, `service::config` | 7, 8, 15, 20, 74 |
| `root_handlers.rs` | `agentic::core::message`, `agentic::coordination`, `infrastructure`, `service::config` | 176, 179, 189, 200, 239, 270, 336, 341 |
| `modes/exec.rs` | `infrastructure` | 425 |
| `modes/chat.rs` | `agentic::tools`, `service::config`, `infrastructure` | 46, 54, 854, 1809, 2016, 2018, 2031, 2053, 2100, 2102, 2283, 2737, 3455, 3496, 3547, 3610 |
| `ui/startup.rs` | `agentic::coordination`, `agentic::tools`, `service::config` | 41, 42, 50, 1285, 1287, 1402, 1446, 1494, 1551, 2129, 2131, 2143, 2165 |

**核心泄露模式**：

1. **CLI 直接实例化 core 内部服务**：`norththing_core::service::config::initialize_global_config()`、`get_global_config_service()` 等
2. **CLI 直接操作 agentic 内部类型**：`ConversationCoordinator`、`SessionConfig`、`EventQueue`、`MessageContent` 等
3. **CLI 直接访问基础设施**：`try_get_path_manager_arc()`、`AIClientFactory` 等
4. **CLI 直接调用工具层**：`get_user_input_manager()`、`get_global_tool_registry()` 等

**`core_adapter.rs` 的注释（第 1–4 行）**自评为：
> "Adapts northhing-core's Agentic system to CLI's Agent interface."

但实际上仍直接导入 `northhing_core::agentic::coordination::ConversationCoordinator`、`northhing_core::agentic::core::SessionConfig`、`northhing_core::agentic::events::EventQueue` 等内部类型，未通过任何公共 adapter/ports 层隔离。

**结论**：❌ **边界泄露未修复**。CLI 与 `northhing-core` 之间仍保持直接耦合，所有上次发现的内部模块穿透路径均仍然存在。建议引入 `norththing-cli-internal` crate 或 `norththing-runtime-ports` 作为边界层。

---

### 6. 综合评分（更新）

| 维度 | 权重 | 得分 | 说明 |
|------|------|------|------|
| 编译产物清理 | 20% | 10/10 | 180 GB → 970 MB，彻底清理 |
| core feature 拆分 | 25% | 10/10 | `default = []` 已正确实施，附详细注释 |
| 编译配置优化 | 15% | 4/10 | 无 `split-debuginfo`，无 `sccache`，无 `target-dir` 重定向；`dev` profile 无 `opt-level` 调优 |
| 依赖版本统一 | 20% | 2/10 | installer 未使用 workspace 继承，存在 3 组 major 版本— 突，exclude 拼写错误 |
| 边界泄露修复 | 20% | 1/10 | 零修复，所有泄露路径仍然存在 |

### 总分：5.0 / 10（及格线以下）

> **改善项**：target 清理 + core feature 拆分（+5.5 分） 
> **恶化/未改善项**：边界泄露、依赖版本、编译配置（-0.5 分，因 installer 新增独立 profile 和版本— 突）

---

### 建议修复清单（优先级排序）

1. **P0 — 边界泄露**：将 CLI 中所有 `northhing_core::agentic::core::...`、`agentic::coordination::...`、`agentic::events::...`、`agentic::tools::...`、`infrastructure::...`、`service::config::...` 的引用，迁移到 `northhing-runtime-ports` 或 `northhing-cli-internal` 提供的公共 API。`core_adapter.rs` 应仅依赖端口层，而非 core 内部模块。

2. **P1 — installer 依赖统一**：将 `northing-installer/src-tauri/Cargo.toml` 中的所有依赖改为 `workspace = true` 继承；修正 workspace 根 `exclude` 中 `northhing-installer` → `northing-installer` 的拼写错误；移除 installer 中重复的 `[profile.release]` / `[profile.release-fast]`，统一使用 workspace profile。

3. **P2 — 编译配置增强**：
 - 在 `.cargo/config.toml` 中添加 `target-dir = "target"` 或 `CARGO_TARGET_DIR`（如需统一路径）
 - 在 `.cargo/config.toml` 中启用 `sccache`：`[build] rustc-wrapper = "sccache"`
 - 在 `[profile.dev]` 中添加 `split-debuginfo = "unpacked"`（Linux/macOS）以减少 target 体积
 - 考虑 `[profile.dev] opt-level = 1` 提升 debug 构建运行时性能

4. **P3 — 下游验证**：检查 `desktop`、`cli`、`cli-internal`、`acp` 等 consumer 是否仍显式使用 `default-features = false` 引用 `northhing-core`，确保 feature 拆分真正生效。
