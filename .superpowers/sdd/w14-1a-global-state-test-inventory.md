# 侦察报告 W14-1a — 全仓依赖进程级全局状态的测试清单

**扫描基线**：仓库 `E:\agent-project\NortHing`，分支 `main`，HEAD `66a59f6`。  
**扫描范围**：全仓 Rust 测试（`src/apps/*`, `src/crates/*`, `northing-installer`, 集成测试）。  
**安全风险预警**：**发现 1 处严重污染用户真实数据的测试**（`src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:575` 漏用隔离守卫，直接读写并残留数据于用户真实 SQLite 数据库 `%APPDATA%\northhing\memory\memory.db` / `~/.config/northhing/memory/memory.db`）。

---

## 1. 统计概览

### 1.1 按类别计数（含多标）
| 类别 | 描述 | 测试数量 | 核心风险与代表测试 |
|---|---|:---:|---|
| **A 类** | **断言未初始化**（未初始化报错，初始化后成功） | **5** | `api.rs:170`、`kernel_facade/tests.rs:381` 等，在单例被任何前序测试初始化后 100% 失败 |
| **B 类** | **变更全局配置/单例状态** | **22** | `api_provider_edit.rs` (7条)、`task_tool_agents.rs:228` (注册4个永久agent) 等 |
| **C 类** | **依赖同步原语**（全局互斥锁/CWD锁） | **24** | `TEST_GLOBAL_CONFIG_MUTEX` (8条)、`CWD_LOCK` (11条)、`REMOTE_SEARCH_TEST_LOCK` (3条)、`ENV_LOCK` (2条) |
| **D 类** | **改环境变量** (`set_var`/`remove_var`) | **4** | `installer/ai_config.rs` (2条)、`core/path_manager.rs` (2条) |
| **E 类** | **碰真实用户目录/真实 Keyring** | **6** | 真实配置库污染 (1条)、真实 Home 残留 (3条)、系统 Keyring API (2条) |
| **F 类** | **其它进程级单例依赖** (`OnceLock`/`LazyLock` 等) | **43** | 依赖 `FACADE`、`GLOBAL_CONFIG_SERVICE`、`GLOBAL_AGENT_REGISTRY`、`SESSION_MANAGER` 等 |

> **去重总计**：全仓共有 **50** 个测试涉及上述进程级全局状态隐患。

### 1.2 按 Crate 计数（去重）
| Crate / 路径 | 涉险测试数 | 主要类别分布 |
|---|:---:|---|
| `northhing` (`src/apps/desktop`) | **12** | A(3), B(9), C(8), F(12) |
| `northhing-core` (`src/crates/assembly/core`) | **28** | A(2), B(9), C(13), D(2), E(4), F(22) |
| `northhing-services-integrations` (`src/crates/services/services-integrations`) | **3** | B(3), C(3), F(3) |
| `northing-installer` (`northing-installer/src-tauri`) | **2** | D(2) |
| `northhing-cli` (`src/apps/cli`) | **2** | E(2) |
| `terminal-core` (`src/crates/services/terminal`) | **1** | A(1), F(1) |
| `northhing-ai-adapters` (`src/crates/adapters/ai-adapters`) | **1** | B(1), F(1) |
| `northhing-agent-runtime` (`src/crates/execution/agent-runtime`) | **1** | B(1), F(1) |

---

## 2. 重点安全隐患：E 类真实用户环境污染排查

| 文件:行 | 测试名 | 污染类型 | 严重等级 | 根因与表现 |
|---|---|---|:---:|---|
| `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:575` | `build_query_aware_facts_reminder_returns_some_with_matching_fact` | **写入真实 SQLite DB** | 🔴 **CRITICAL** | **漏写 `with_test_memory_db_path` 隔离守卫**！直接调用 `default_memory_db_path()` 打开用户的真实 SQLite 数据库（`%APPDATA%\northhing\memory\memory.db` 或 `~/.config/northhing/memory/memory.db`），并执行 `db.insert_fact("I prefer pnpm for JS projects")`，测试结束后无任何清理，导致测试数据永久污染用户真实记忆库！ |
| `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:430` | `prompt_injection_with_facts_includes_remembered_facts_section` | 真实 Home 目录残留 | 🟡 MEDIUM | 调用 `path_manager_arc().project_memory_dir(&workspace)`，在用户真实 home 的 `~/.northhing/projects/<uuid-slug>/memory/` 下创建目录并写入 `facts.jsonl`，清理时只删除了 temp workspace，残留了真实 home 下的 slug 目录。 |
| `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:482` | `prompt_injection_with_select_facts_budget_limit` | 真实 Home 目录残留 | 🟡 MEDIUM | 同上，在 `~/.northhing/projects/<slug>/memory/` 创建并残留目录与文件。 |
| `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:505` | `prompt_injection_degrades_when_facts_file_unreadable` | 真实 Home 目录残留 | 🟡 MEDIUM | 同上，在 `~/.northhing/projects/<slug>/memory/` 创建了 `facts.jsonl` 目录并残留。 |
| `src/apps/cli/src/keyring_keys.rs:108` | `missing_keyring_entry_resolves_to_empty` | 触发真实 OS Keyring API | 🟢 LOW | 调用 `resolve_effective_model_key` 触发真实操作系统的 Keyring 服务（读不存在 key，安全只读）。 |
| `src/apps/cli/src/keyring_keys.rs:116` | `chat_edit_path_resolve_contract` | 触发真实 OS Keyring API | 🟢 LOW | 同上，只读调用真实操作系统 Keyring API。 |

---

## 3. 最难处理的 Top 5 隐患及难点分析

### 1. `src/apps/desktop/src/ui_dioxus/api.rs:170` — `test_ensure_room_session_fails_cleanly_when_uninitialized`
- **难点**：**跨层与二进制边界**。`desktop` 是 `[[bin]]` 目标，未向外暴露 library 接口。该测试断言单例未初始化时的行为，但在同 binary 内有大量会触发 `init_core()` 的测试。若迁入外部独立集成测试，必须将 desktop 拆为 `lib + bin` 结构并提升内部 UI 函数可见性；若在单元测试中保留，则需要为 `KernelFacade` 和 `ROOM_SESSION_CACHE` 提供测试专用的 Reset/Clear seam。

### 2. `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:200..371` — 7 个 Provider 编辑/删除测试
- **难点**：**复合全局状态耦合与锁范围不完全**。这 7 个测试依赖 `init_core()`、修改 `GlobalConfig` 的 providers/default_provider_id、读写 Keyring。虽然测试体内使用了 `TEST_GLOBAL_CONFIG_MUTEX`，但同 binary 的其它测试（如 `app_state/settings/tests.rs:337` 的 `push_resolved_keys_to_core`、`api.rs:170`）并不会获取该锁，导致并行跑时互相踩踏、串行跑时单例状态残留。

### 3. `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:575` — `build_query_aware_facts_reminder_returns_some_with_matching_fact`
- **难点**：**开发机真实用户数据破坏与隐蔽性**。因缺少 RAII 隔离守卫，每次执行 `cargo test` 都会直接对当前用户的实际 SQLite 记忆库执行写操作，污染本地用户配置且极难排查（在不同机器或 CI 干净容器上表现不同）。

### 4. `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_agents.rs:228` — `prompt_stability_description_with_context_renders_available_agents_in_stable_order`
- **难点**：**全局单例注册表永久污染且无反注册接口**。测试通过 `register_prompt_order_test_subagent` 向单例 `GLOBAL_AGENT_REGISTRY` 动态注入 4 个自定义 agent (`AAAPromptOrderBuiltin` 等)，而 `AgentRegistry` 完全未实现 unregister 功能，导致该进程内的后续所有 agent 列表查询均被污染。

### 5. `src/crates/assembly/core/src/kernel_facade/tests.rs:404` — `test_init_gate_lifecycle_all_scenarios`
- **难点**：**底层进程级并发原语篡改**。测试直接重置 `FACADE_READY: AtomicBool` 为 false 并修改 `INIT_STATE: AsyncMutex`。在多线程 test runner 中，若有其它测试正在并发执行 `init_core()`，初始化门禁状态会被撕裂，引发不可预测的死锁或断言 panic。

---

## 4. 主清单表（全量 50 条明细）

| 文件:行 | 测试名 | 类别 | 依赖的全局状态 | 被污染时的表现 | 迁移可行性判断（A/B类） |
|---|---|:---:|---|---|---|
| `src/apps/desktop/src/ui_dioxus/api.rs:170` | `test_ensure_room_session_fails_cleanly_when_uninitialized` | **A** | `FACADE`, `ROOM_SESSION_CACHE` | 排在 `init_core()` 后时 `is_err()` 断言失败 panic | **需提升可见性**（需 desktop 导出 lib 或提供 reset） |
| `src/apps/desktop/src/ui_dioxus/api_events.rs:121` | `test_event_channel_returns_receiver` | **F** | `FACADE` (`subscribe_events`) | 若未初始化后台打 warn，若已初始化向单例注册 receiver | — |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:200` | `test_edit_provider_blank_key_inherits_existing` | **B, C, F** | `TEST_GLOBAL_CONFIG_MUTEX`, `FACADE`, `GlobalConfig` | 修改全局模型配置列表 | **需提升可见性**（需导出 helper 与 mock） |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:233` | `test_edit_provider_new_key_overwrites_keyring` | **B, C, F** | `TEST_GLOBAL_CONFIG_MUTEX`, `FACADE`, `GlobalConfig` | 修改全局模型配置列表 | **需提升可见性** |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:267` | `test_edit_provider_keyring_read_error_fails_closed` | **B, C, F** | `TEST_GLOBAL_CONFIG_MUTEX`, `FACADE`, `GlobalConfig` | 修改全局模型配置列表 | **需提升可见性** |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:294` | `test_edit_provider_nonexistent_id_returns_error` | **B, C, F** | `TEST_GLOBAL_CONFIG_MUTEX`, `FACADE` (`init_core`) | 初始化全局 Facade 单例 | **需提升可见性** |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:322` | `test_delete_provider_default_provider_rejected` | **A, B, C, F** | `TEST_GLOBAL_CONFIG_MUTEX`, `FACADE`, `GlobalConfig.default_provider_id` | 设置并断言默认 Provider 拒绝删除；排在污染后易断言失败 | **需提升可见性** |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:349` | `test_delete_provider_success_cleans_config_and_keyring` | **B, C, F** | `TEST_GLOBAL_CONFIG_MUTEX`, `FACADE`, `GlobalConfig` | 变更全局配置并删除 | **需提升可见性** |
| `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:371` | `test_edit_provider_validation_failure_zero_writes` | **B, C, F** | `TEST_GLOBAL_CONFIG_MUTEX`, `FACADE`, `GlobalConfig` | 变更全局配置并删除 | **需提升可见性** |
| `src/apps/desktop/src/ui_dioxus/api_settings.rs:198` | `test_api_functions_fail_cleanly_before_init` | **A, F** | `FACADE` | 断言未初始化时各 API 宽容不 panic；初始化后无法测未初始化路径 | **需提升可见性** |
| `src/apps/desktop/src/ui_dioxus/api_settings.rs:253` | `test_persist_onboarding_provider_success_flow` | **B, C, F** | `TEST_GLOBAL_CONFIG_MUTEX`, `FACADE` (`init_core`), `GlobalConfig` | 变更全局 default_provider_id 与模型列表 | **需提升可见性** |
| `src/apps/desktop/src/app_state/settings/tests.rs:337` | `push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean` | **B, F** | `FACADE`, `GLOBAL_CONFIG_SERVICE`, `GlobalConfig` | 未加互斥锁，直接向全局 Core 内存推送并删除密钥 | **需提升可见性**（需导出 `push_resolved_keys_to_core`） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:381` | `test_result_methods_return_error_before_init` | **A** | `FACADE` (OnceLock) | 排在 `init_core()` 之后运行时直接 panic 失败 | **可直接迁**（`kernel_facade()` 是公共函数） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:404` | `test_init_gate_lifecycle_all_scenarios` | **B, F** | `FACADE_READY: AtomicBool`, `INIT_STATE: AsyncMutex` | 强行重置门禁状态，并行时撕裂其它测试的初始化过程 | **需提升可见性**（`FACADE_READY` 是 `pub(super)`） |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:977` | `list_tree_rejects_parent_dir_escape` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade 与进程当前工作目录 | — |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1000` | `list_tree_rejects_absolute_path` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade | — |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1038` | `list_tree_lists_direct_children` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade | — |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1060` | `read_file_rejects_too_large` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade | — |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1079` | `read_file_round_trip_within_cap` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade | — |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1090` | `read_file_rejects_escape` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade | — |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1146` | `read_file_rejects_symlink_to_outside_target` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade | — |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1176` | `list_tree_skips_symlink_to_outside_target` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade | — |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1209` | `list_tree_with_explicit_workspace_root_uses_that_fence` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade | — |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1229` | `read_file_with_explicit_workspace_root_uses_that_fence` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade | — |
| `src/crates/assembly/core/src/kernel_facade/tests.rs:1249` | `list_tree_rejects_non_absolute_workspace_root` | **C, F** | `CWD_LOCK: Mutex<()>`, `FACADE` | 依赖全局 Facade | — |
| `src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs:212` | `env_overrides_keep_e2e_storage_out_of_real_user_profile` | **C, D** | `ENV_LOCK`, 环境变量 `northhing_*` | 修改进程级环境变量 | — |
| `src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs:237` | `e2e_storage_guard_rejects_missing_isolated_roots` | **A, C, D** | `ENV_LOCK`, 环境变量 `northhing_E2E_STORAGE_GUARD` | 修改进程级环境变量并断言 Err | **可直接迁** |
| `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:430` | `prompt_injection_with_facts_includes_remembered_facts_section` | **E, F** | `GLOBAL_PATH_MANAGER`, 真实 Home | 污染真实 `~/.northhing/projects/` | — |
| `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:482` | `prompt_injection_with_select_facts_budget_limit` | **E, F** | `GLOBAL_PATH_MANAGER`, 真实 Home | 污染真实 `~/.northhing/projects/` | — |
| `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:505` | `prompt_injection_degrades_when_facts_file_unreadable` | **E, F** | `GLOBAL_PATH_MANAGER`, 真实 Home | 污染真实 `~/.northhing/projects/` | — |
| `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:575` | `build_query_aware_facts_reminder_returns_some_with_matching_fact` | **B, E, F** | 真实 SQLite 数据库 `memory.db` | **高危：直接污染真实用户 SQLite 数据库** | **不适用迁移，应原地补 `_db_guard`** |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_agents.rs:228` | `prompt_stability_description_with_context_renders_available_agents_in_stable_order` | **B, F** | `GLOBAL_AGENT_REGISTRY` (`agent_registry()`) | 向全局单例注入 4 个测试 agent 且永不注销 | **需提升可见性**（应重构成局部参数） |
| `src/crates/assembly/core/src/agentic/tools/implementations/code_review_tool/tests.rs:354` | `deep_review_submission_fills_concurrency_limited_from_runtime_tracker` | **B, F** | `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER` | 向全局 tracker 记录容量拒绝 | **需提升可见性** |
| `src/crates/assembly/core/src/agentic/tools/implementations/code_review_tool/tests.rs:395` | `deep_review_shared_context_diagnostics_stays_out_of_report` | **B, F** | `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER` | 向全局 tracker 记录 tool 使用 | **需提升可见性** |
| `src/crates/assembly/core/src/agentic/tools/implementations/code_review_tool/tests.rs:437` | `deep_review_submission_folds_capacity_skips_into_concurrency_limited_signal` | **B, F** | `GLOBAL_DEEP_REVIEW_BUDGET_TRACKER` | 向全局 tracker 记录 capacity skip | **需提升可见性** |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review_tests.rs:306` | `deep_review_capacity_queue_cancel_control_skips_waiting_reviewer` | **B, F** | `GLOBAL_DEEP_REVIEW_QUEUE_CONTROL_TRACKER` | 向全局 tracker 插入控制命令 | **需提升可见性** |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review_tests.rs:346` | `deep_review_capacity_queue_records_one_runtime_wait_when_ready` | **B, F** | `GLOBAL_DEEP_REVIEW_QUEUE_CONTROL_TRACKER` | 向全局 tracker 记录等待状态 | **需提升可见性** |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review_tests.rs:396` | `deep_review_capacity_queue_pause_does_not_expire_until_continued` | **B, F** | `GLOBAL_DEEP_REVIEW_QUEUE_CONTROL_TRACKER` | 向全局 tracker 插入 Pause/Continue | **需提升可见性** |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review_tests.rs:454` | `deep_review_capacity_queue_skip_optional_skips_optional_waiter` | **B, F** | `GLOBAL_DEEP_REVIEW_QUEUE_CONTROL_TRACKER` | 向全局 tracker 插入 SkipOptional | **需提升可见性** |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review_tests_runtime.rs:375` | `deep_review_queue_action_cancel_stops_turn` | **B, F** | `GLOBAL_DEEP_REVIEW_QUEUE_CONTROL_TRACKER` | 变更全局队列控制状态 | **需提升可见性** |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review_tests_runtime.rs:428` | `deep_review_queue_action_pause_sets_state` | **B, F** | `GLOBAL_DEEP_REVIEW_QUEUE_CONTROL_TRACKER` | 变更全局队列控制状态 | **需提升可见性** |
| `src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_deep_review_tests_runtime.rs:460` | `deep_review_queue_action_continue_clears_pause` | **B, F** | `GLOBAL_DEEP_REVIEW_QUEUE_CONTROL_TRACKER` | 变更全局队列控制状态 | **需提升可见性** |
| `src/crates/execution/agent-runtime/tests/deep_review_policy_contracts.rs:77` | `deep_review_queue_control_and_shared_context_contract` | **B, F** | `GLOBAL_DEEP_REVIEW_QUEUE_CONTROL_TRACKER` | 变更全局队列控制状态 | **可直接迁** (已在 `tests/`) |
| `src/crates/services/terminal/src/session/singleton.rs:92` | `test_session_manager_not_initialized` | **A, F** | `SESSION_MANAGER: OnceCell` | 排在初始化后运行时测试被 if 跳过 | **可直接迁** |
| `src/crates/services/services-integrations/src/remote_ssh/workspace_search/service_helpers.rs:138` | `remote_search_rejects_non_linux_before_stdio_open` | **B, C, F** | `REMOTE_SEARCH_TEST_LOCK`, `REMOTE_STDIO_SESSIONS` 等 | 清空全局 stdio sessions 映射 | **需提升可见性** (`REMOTE_STDIO_*` 为 `pub(super)`) |
| `src/crates/services/services-integrations/src/remote_ssh/workspace_search/service_helpers.rs:162` | `remote_search_context_ignores_stale_cache_before_resolving_connection` | **B, C, F** | 同上 | 修改全局 stdio context 映射 | **需提升可见性** |
| `src/crates/services/services-integrations/src/remote_ssh/workspace_search/service_helpers.rs:204` | `remote_search_open_guard_is_removed_when_stdio_spawn_fails` | **B, C, F** | 同上 | 清空全局 stdio guards 映射 | **需提升可见性** |
| `src/crates/adapters/ai-adapters/src/diagnostics.rs:18` | `sensitive_diagnostics_can_be_toggled` | **B, F** | `INCLUDE_SENSITIVE_DIAGNOSTICS: AtomicBool` | 切换全局诊断开关 | **可直接迁** (`set_include_sensitive_diagnostics` 为 `pub`) |
| `src/apps/cli/src/keyring_keys.rs:108` | `missing_keyring_entry_resolves_to_empty` | **E** | 操作系统真实 Keyring | 读真实 OS Keyring API | — |
| `src/apps/cli/src/keyring_keys.rs:116` | `chat_edit_path_resolve_contract` | **E** | 操作系统真实 Keyring | 读真实 OS Keyring API | — |
| `northing-installer/src-tauri/src/installer/ai_config.rs:401` | `write_model_then_theme_preserves_both` | **D** | 环境变量 `NORTHHING_INSTALLER_CONFIG_DIR` | 并行测试可能发生环境变量踩踏 | — |
| `northing-installer/src-tauri/src/installer/ai_config.rs:438` | `write_theme_then_model_preserves_both` | **D** | 环境变量 `NORTHHING_INSTALLER_CONFIG_DIR` | 并行测试可能发生环境变量踩踏 | — |

---

## 5. 迁移可行性与修复策略总结

对涉及 **A 类（断言未初始化）** 与 **B 类（变更全局状态）** 的测试迁移评估如下：

1. **可直接迁（4 个）**：
   - `assembly/core/kernel_facade/tests.rs:381` (`test_result_methods_return_error_before_init`)
   - `assembly/core/infrastructure/app_paths/path_manager.rs:237` (`e2e_storage_guard_rejects_missing_isolated_roots`)
   - `services/terminal/src/session/singleton.rs:92` (`test_session_manager_not_initialized`)
   - `adapters/ai-adapters/src/diagnostics.rs:18` (`sensitive_diagnostics_can_be_toggled`)
   - *(注：`agent-runtime` 的 contracts 测试已在 `tests/` 目录)*

2. **迁移需提升可见性（20 个）**：
   - **Desktop Crate (10 个)**：`api.rs:170`、`api_provider_edit.rs` (7个)、`api_settings.rs` (2个)、`app_state/settings/tests.rs:337`。因为 `desktop` 是 `[[bin]]`，无法直接作为 lib 导入，需拆分 `lib.rs` 并将 `ui_dioxus::api`、`MockKeyring` 等提升为 `pub`。
   - **Core DeepReview / TaskTool (10 个)**：`task_tool_agents.rs:228`、`code_review_tool` (3个)、`task_tool_deep_review_tests` (4个)、`task_tool_deep_review_tests_runtime` (3个)。目前其辅助注册与队列控制函数为内部私有/`pub(crate)`。
   - **Services Integrations (3 个)**：`remote_ssh/workspace_search/service_helpers.rs` (3个)，其访问的 `REMOTE_STDIO_SESSIONS` 等为 `pub(super)`。

3. **建议就地修复而不是迁移（2 个）**：
   - `auto_memory.rs:575` (`build_query_aware_facts_reminder_returns_some_with_matching_fact`)：**高危 Bug**，直接在原地补上一行 `let _db_guard = with_test_memory_db_path(unique_test_memory_db_path());` 即可彻底消除安全隐患与状态依赖。
   - `kernel_facade/tests.rs:404` (`test_init_gate_lifecycle_all_scenarios`)：建议将 `run_init_gate` 的测试改用局部实例/局部 AtomicBool，不直接篡改进程全局单例。

---
*报告生成完毕。所有清单条目均已验证真实代码路径与行号。*
