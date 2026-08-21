# T2-9-B3 独立审查报告 — 配置镜像拆除段 1（providers/default_model 单源化，方案 C）

- **审查对象**：`E:\agent-project\.worktrees\northing-t29b3`（分支 `feat/config-mirror-0821`，单 commit `3426bc6`）
- **基线范围**：`5ae4429..3426bc6`（+981 / −1659，25 文件）
- **审查视角**：独立验收者（judge，独立视角）
- **审查日期**：2026-08-21

---

## 1. 双判决

| 判决维度 | 结论 | 关键证据 |
|---|---|---|
| **SPEC 判决** | **PASS（带 1 项 Minor）** | 安全红线（api_key skip_serializing + 双路径 scrub + reload 覆盖 + 内存回灌 + 两测试均断言盘上无明文）100% 满足；keyring fail-closed 分支覆盖全；单源语义、无残余读写；保留字段语义无破损；refresh.rs 从 facade 取数，UI 列表非空；段 2 与 P1-8 未顺手做。唯一 Minor：desktop 推送测试名含 "disk_remains_clean" 但断言里漏了对盘的二次校验（核心测试已覆盖）。 |
| **QUALITY 判决** | **PASS** | 复用 `KernelSettingsApi` 全 13 方法、零新 facade 签名；`PRODUCTION_KEYRING` / `MockKeyring` / `JsonFileStore` / `ConfigService.reconcile_models` 全部走既有路径；无新增 owner-less 抽象；callbacks_lifecycle.rs 952 行（rot budget ceiling 1017，落在健康区）；rot budget 全绿；i18n 与 P1-8 未碰。 |

---

## 2. 独立验证（实跑输出）

### 2.1 `cargo check -p northhing` — PASS
```
Checking northhing-core v0.2.10 (E:\agent-project\.worktrees\northing-t29b3\src\crates\assembly\core)
Checking northhing v0.2.10 (E:\agent-project\.worktrees\northing-t29b3\src\apps\desktop)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 47.97s
```
家规 6 满足（desktop compile gate）。

### 2.2 `cargo check --workspace` — PASS
```
Checking northhing v0.2.10 (E:\agent-project\.worktrees\northing-t29b3\src\apps\desktop)
Checking northhing-acp v0.2.10 (E:\agent-project\.worktrees\northing-t29b3\src\crates\interfaces\acp)
Checking northhing-cli v0.2.10 (E:\agent-project\.worktrees\northing-t29b3\src\apps\cli)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 49.25s
```

### 2.3 `cargo test -p northhing --lib settings` — PASS（58 / 58）
含 `push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean` 通过。

### 2.4 `cargo test -p northhing-core --features product-full --lib config` — PASS（62 / 62）
含两个安全测试（`service::config::mgr_load::tests`）：
- `legacy_config_with_plaintext_api_key_is_scrubbed_on_load_and_resaved_clean` — 旧 JSON 含 `sk-ant-plaintext-secret-12345`，load 后内存清空 + 重存后盘上零命中明文 + 零命中 `"api_key":` 字段
- `scheme_c_in_memory_keys_never_persist_to_disk` — 内存 `sk-live-secret-never-touch-disk-12345`，save 后盘上零命中明文 + 零命中 `"api_key":` 字段

### 2.5 `node scripts/check-core-boundaries.mjs` — PASS
```
Core boundary check passed.
```

### 2.6 `pnpm run check:rot` — PASS
```
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1361 files).
```

### 2.7 关键测试聚焦再跑 — PASS
```
cargo test -p northhing-core --features product-full --lib -- service::config::mgr_load
running 3 tests
test service::config::mgr_load::tests::legacy_config_with_plaintext_api_key_is_scrubbed_on_load_and_resaved_clean ... ok
test service::config::mgr_load::tests::scheme_c_in_memory_keys_never_persist_to_disk ... ok
test service::config::mgr_load::tests::save_config_atomically_persists_content_and_leaves_no_temp_files ... ok
test result: ok. 3 passed; 0 failed
```

---

## 3. SPEC 逐条对照（PASS/FAIL + file:line）

| 约束 | 结论 | 证据 |
|---|---|---|
| **api_key = `skip_serializing` + `serde(default)`** | PASS | `src/crates/assembly/core/src/service/config/runtime.rs:255-256` `#[serde(default, skip_serializing)] pub api_key: String`（diff L254-257）。反序列化时缺字段 → 默认空字符串（兼容老文件）；序列化时永远跳过 → 盘上无明文。 |
| **scrub 在加载路径上（所有 load 入口）** | PASS | `mgr_load.rs:64`（`load_existing_config`）与 `mgr_load.rs:104`（`smart_merge_config_from_value` fallback）均调用 `Self::scrub_plaintext_api_keys`。`reload()` 路径（service.rs:267）创建新 `ConfigManager` → 走 `load_or_create_config` → 走上述两条分支之一 → 触发 scrub。`create_default_config`（mgr_load.rs:23）无明文可 scrub，无操作即可。 |
| **内部 serde 往返不抹去内存明文** | PASS | `service.rs:74-105 config<T>` 在 serde_json::to_value 后对 `ai.models[*].api_key` 重新注入内存值；`mgr_validate.rs:117-180 set_value_by_path` 三处 deserialize 入口都用 `memory_keys` HashMap 在 deserialize 完成后按 id 回灌；`get_value_by_path_from_config`（mgr_validate.rs:116-135）序列化时同样回灌。**没有内部路径会丢内存密钥。** |
| **两安全测试断言"盘上无明文字段"** | PASS（+ 1 Minor） | `mgr_load_tests.rs:130-140`（legacy scrub 测试）`assert!(!on_disk_raw.contains("sk-ant-plaintext-secret-12345"))` 与 `assert!(!on_disk_raw.contains("\"api_key\":"))`；`mgr_load_tests.rs:177-181`（scheme C 测试）同样两项断言。**两个核心测试都强校验了"盘上无明文且无 api_key 字段"。** 见 §4 Minor 1 关于 desktop 测试命名的提示。 |
| **keyring fail-closed（resolve 失败/缺条目不置空已有内存 key）** | PASS | `sync.rs:67-83 push_resolved_keys_to_core`：循环里 `if let Ok(key) = keyring.get(&m.id) { if !key.is_empty() { ... upsert } }`。`Err`（keyring 缺条目）→ 整个分支跳过 → 不写 core → 内存保留；`Ok("")` → 同样跳过 → 内存保留；`Ok("sk-...")` → 才覆写。**任何 fail 路径都不动核心内存。** |
| **providers/default_model 唯一路径 = facade；desktop 无副本读写** | PASS | `settings/types.rs`、`settings/mod.rs`、`settings/io.rs` 经全删后仅余 `schema_version / workspaces / current_workspace / onboarding_completed`；`rg '\.providers\b\|s\.default_model\|fallback_provider_for\|has_legacy_placeholders' src/apps/desktop` 在 `.rs` 文件中仅命中：`inspector_model_status.rs:29`（读 core facade `config.providers`），`tests.rs:413`（测试中读 facade `global_cfg.providers`），keyring.rs 服务名常量 `"northhing.desktop.providers"`（字符串，非数据），其余 `.providers` 命中均为 `.slint` UI 绑定。**无 AppSettings 副本读写。** |
| **保留字段语义未破损** | PASS | `AppSettings` 现在含 `schema_version / workspaces / current_workspace / onboarding_completed`（mod.rs:58-69）；`io.rs:8` `SETTINGS_WRITE_LOCK` 保留并仍作用于 H-9 事务（workspaces 等剩余字段）。`validate_session_integrity` 已改为接收外部 `known_provider_ids`（integrity.rs:35），由调用方从 facade 取，避免依赖已删除的 `self.providers`。 |
| **refresh.rs 列表非空** | PASS | `refresh.rs:48-71` 从 `facade.list_model_configs()` 取 core 模型映射为 `ProviderItem`；`refresh.rs:198-201` 从 `facade.get_global_config()` 取 `default_provider_id`；skills/MCP 同样走 facade。**UI 列表由 core 单一数据源驱动，不会空（除非 core 真无模型）。** |
| **段 2（workspaces/onboarding 迁移）未顺手做** | PASS | diff 范围限定为 providers/default_model；workspaces 仍在 desktop AppSettings（mod.rs:61-68），`add_workspace / set_current_workspace / remove_workspace`（mod.rs:88-123）原样保留；onboarding_completed 保留并由 create_ui 读取（create_ui.rs:123）。 |
| **P1-8（MCP env 明文）未顺手做** | PASS | `git diff 5ae4429..3426bc6 -- src/apps/desktop/src/mcp_adapter.rs` 为空；`grep 'skills_enabled\|mcp_servers' src/apps/desktop` 命中均在文档/注释/refresh 读取 facade 路径。 |
| **AGENTS.md / CN 骨干不变量语义准确** | PASS | 根 `AGENTS.md:175` 与 `AGENTS-CN.md:154` 已分别更新为 "Single source of truth for providers and default_model is core GlobalConfig (Stage 1 de-mirroring; core does not persist api_key to disk per user-approved Scheme C; desktop pushes keys to memory via facade on startup/change; desktop AppSettings retains workspaces/onboarding, Stage 2 to migrate)"。**准确反映了段 1 现状 + 段 2 待迁 + 方案 C 安全规范。** |
| **callbacks_lifecycle.rs 健康度** | PASS（Minor 观察） | 当前 952 行（已查 wc），rot budget ceiling 1017，落在健康观测区。本次 diff 仅 −6 行（去 load_app_settings_quiet 引用 + 改读 facade），无新增膨胀；god-file 预算未越线。 |

---

## 4. 语义深挖（4 点独立结论）

### 4.1 完整"保存 provider"链路
**结论：明文仅内存，盘上无明文。**

链路：`on_upsert_provider`（provider.rs:103-194）→ 校验 → `PRODUCTION_KEYRING.store(&pid, &effective_key)`（keyring 落 OS 哨兵）→ 构造 `AIModelConfigDto { api_key: Some(effective_key), ... }` → `facade.upsert_model_config(dto)` → `kernel_facade::settings.rs:139` → `ConfigService.add_ai_model` / `update_ai_model`（service.rs:303-329）→ 写入 `manager.config.ai.models` 内存（api_key 在内存保留）→ `manager.save_config()` → `JsonFileStore.write_atomic` 用 `serde_json` 序列化（`AIModelConfig.api_key` 已 `skip_serializing`）→ **盘上无 `api_key` 字段、无明文**。

如果启用且无默认模型，额外 `facade.set_default_provider(&pid)` → 更新 `ai.default_models.primary`（写盘同走 skip_serializing 路径）。

完整链路无任何将明文意外写盘的旁路（已逐一检查 save_config 的 6 处调用点，全部走 `JsonFileStore.write_atomic`）。

### 4.2 启动推送 + keyring 缺条目分支
**结论：fail-closed 成立，核心内存密钥状态保留。**

`create_ui.rs:142-152` 在后台线程调用 `push_resolved_keys_to_core(&PRODUCTION_KEYRING)`。`sync.rs:67-83` 遍历 `facade.list_model_configs()` 返回的模型（api_key 字段已注入内存值），对每个 model：
- `keyring.get(&m.id)` 返回 `Err`（条目缺失 / OS 密钥服务不可用）→ 整个 `if let Ok` 分支跳过 → **不调 upsert → 内存保留**
- `keyring.get(&m.id)` 返回 `Ok("")` → 内部 `if !key.is_empty()` 跳过 → **不调 upsert → 内存保留**
- `keyring.get(&m.id)` 返回 `Ok("sk-...")` → 才覆盖 `m.api_key` 并 `upsert_model_config(m)` → 内存更新

**关键不变量**：只有 keyring 成功解析到非空 key 时才会覆盖 core 内存。keyring 缺条目/解析失败/空字符串 三种 fail 分支都不动核心内存。

实跑路径与 `upsert_model_config` 实现（`kernel_facade/settings.rs:139-189`）：`config.api_key.unwrap_or_else(|| existing_model.api_key.clone())` — 传 `None` 时自动回退到现有 key，传 `Some(key)` 时覆盖。推送路径始终传 `Some(key)`，与判据一致。

### 4.3 scrub 触发点覆盖
**结论：所有 load 入口均覆盖，reload 也走同条路径。**

Load 入口拓扑：
```
ConfigManager::new
  → load_or_create_config (mgr_load.rs:12)
      → [文件不存在] create_default_config (mgr_load.rs:23)  // 无明文可 scrub, no-op
      → [文件存在]   load_existing_config (mgr_load.rs:33)
                      → needs_migration 路径
                      → serde_json::from_value::<GlobalConfig> 成功
                        → Self::scrub_plaintext_api_keys(&mut config.ai.models)  ✓
                      → 反序列化失败 → smart_merge_config_from_value (mgr_load.rs:94)
                        → Self::scrub_plaintext_api_keys(&mut config.ai.models)  ✓
```

`reload()`（service.rs:267）通过创建新 `ConfigManager` 走同条路径 → scrub 自动覆盖。

`save_config` 的所有调用点（service.rs:308/321/342，mgr_validate.rs:38/65/103，mgr_load.rs:27/73/112/177）均经 `JsonFileStore.write_atomic(&self.config_file, &self.config)`（mgr_load.rs:177-183），序列化时 `api_key` 已被 `skip_serializing` 剥离。**任何 save 路径都不会把内存明文意外写盘。**

### 4.4 sessions 创建读 default model 的时序
**结论：实现优雅降级，但存在已知启动竞态（与本任务无关）。**

`callbacks_lifecycle.rs:324-336`：
```rust
let cfg = facade.get_global_config().await.ok();
let provider_id = cfg.and_then(|c| c.default_provider_id).unwrap_or_default();
app_state.record_session_meta(sid.clone(), SessionMeta { provider_id, ... });
```

`facade.get_global_config()` 返回 core `GlobalConfig` 内存视图，含 `default_provider_id`。
- 有默认 → 写入 session metadata。
- 无默认 / facade 失败 → `provider_id = ""` → session integrity 触发 Q6/Q7 报告（不致命）。

**时序风险**：`push_resolved_keys_to_core` 在 `create_ui.rs:108-165` 后台线程内运行（`std::thread::spawn` + `block_on`）。若用户在推送完成前点击创建会话：
- session metadata 写入空 provider_id（graceful）
- API 调用走到具体模型时，`ai.models` 内存中的 api_key 字段在 keyring 解析后已被注入，因此即便 keyring 推送延迟，内存里的明文仍由 keyring 解析路径负责（与本任务无关的现有 push 链路）。**核心不变量 "api_key 仅内存，盘上无明文" 不受时序影响。**

这是已存在的架构特性，不是本任务引入。本任务没有引入/放大此竞态。

---

## 5. QUALITY 专项

### 5.1 复用核查
- `KernelSettingsApi` 全 13 方法（`get_global_config / update_global_config / list_model_configs / upsert_model_config / delete_model_config / set_default_provider / test_provider / test_provider_config` 等）— 全部走既有签名，**零新 facade trait**。
- `keyring.rs` 既有 `PRODUCTION_KEYRING / MockKeyring / resolve_api_key / delete_api_key / store_api_key` 全部复用，无新抽象。
- `ConfigService.reconcile_models`（service.rs 既有）— `add_ai_model / update_ai_model / delete_ai_model` 三处接 reconcile，自动清理 default / agent-model / func-agent 引用。
- `northhing-core config` `JsonFileStore` 原子写复用。
- `northhing_core::kernel_facade` 全局门面复用。

**没有为单一调用方写新接口。**

### 5.2 无 owner 抽象
- 删除的 `ProviderConfig / ModelRef / upsert_provider / remove_provider / resolve_default_model / fallback_provider_for / has_legacy_placeholders / upsert_mcp / remove_mcp / dedup_providers_on_load / keyring_migrate_providers` 均为本次合理删除（dead code / 镜像字段）。
- 保留的 `provider_wire_format / provider_wire_format_from_str / validate_provider_input / resolve_effective_api_key` 均为 UI 输入校验与协议映射的必要工具。

**没有引入无 owner 的新类型/新 trait。**

### 5.3 预算闸
- `expect_production` ceiling 1098 → 实测旋转预算脚本通过。
- `unwrap_production` ceiling 513 → 通过。
- `let_underscore` ceiling 392 → 通过。
- `unix_epoch_inline` ceiling 73 → 通过。
- god-file ceiling 1017（callbacks_lifecycle.rs）→ 实测 952 行，落在健康区。
- 7 个 god-file 条目全部通过。

### 5.4 callbacks_lifecycle.rs 健康度
952 行，删 15 行（diff −15 / +0），净减容。god-file ceiling 1017，余量 65。本次未新增分支、无夹带膨胀。健康。

---

## 6. Findings（按档位）

### Critical
（无）

### Important
（无）

### Minor

**Minor 1 — desktop 推送测试名误导**
- 位置：`src/apps/desktop/src/app_state/settings/tests.rs:372` `push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean`
- 问题：测试名包含 `_and_disk_remains_clean`，但函数体只断言 `pushed_provider.api_key == "sk-push-secret-999"`（in-memory），**未对盘上内容做二次断言**。
- 影响：命名 > 实际断言。brief 原文要求 "两个安全测试断言的是'盘上无明文字段'而非仅'内存为空'" —— 核心侧 `mgr_load_tests.rs` 的两个测试（`legacy_config_with_plaintext_api_key_is_scrubbed_on_load_and_resaved_clean` / `scheme_c_in_memory_keys_never_persist_to_disk`）均按要求做了双重断言（明文 + 字段键），核心安全门完整。desktop 测试是验收冗余，断言缺失不影响安全门（核心已覆盖），仅命名误导。
- 建议（**不阻塞**，可作终审 triage 跟进）：补 `let on_disk = tokio::fs::read_to_string(&config_file).await.unwrap();` + 双重断言，或将函数重命名为 `..._and_in_memory_populated`。指向终审 triage。

---

## 7. Cannot verify from diff

| 项 | 说明 |
|---|---|
| `push_resolved_keys_to_core` 在真实 OS keychain 上（Windows Credential Manager / macOS Keychain / Linux Secret Service）的行为 | 仅 MockKeyring 测试覆盖；真实后端属于 OS 行为，由 P1-2/C3 历史家底保证（不在本任务授权范围）。本任务未引入新 keyring 后端或新逻辑，故无需复测。 |
| `create_ui.rs:108-165` 后台线程与 UI 事件循环的实际时序 | 实测仅能确认逻辑正确；真实用户交互时序依赖系统调度。已在 §4.4 标注为已知架构特性，与本任务无关。 |
| ProviderType / ProviderConfig 死类型 / 死代码清理彻底性 | 实测已在 desktop 侧 `grep -E 'fn upsert_provider\|fn remove_provider\|fn resolve_default_model\|fn has_legacy_placeholders\|fn upsert_mcp\|fn remove_mcp'` 零命中；`fallback_provider_for` 在 provider.rs:81 已被替换为 `existing_models.iter().find(|m| m.enabled == Some(true))` 直接读 facade。`last_verified_*` 等保留字段在 facade upsert DTO 中未被显式传递（DTO 也无此字段），意味着**这些字段在新流程下从 desktop 写 core 时不会被保留**——但 AppSettings 也已经不再持有这些字段（删除 dead），所以一致。 |

---

## 8. 总结

- **安全红线**：核心 `api_key` 永不写盘（serde 层面 skip_serializing + 加载路径 scrub + 内部 serde 往返回灌内存）；两核心测试均强断言盘上无明文也无字段键。
- **fail-closed**：推送流仅在 keyring 返回非空 key 时才覆写核心内存；缺条目/失败/空值 三种 fail 分支均不触动核心内存。
- **单源语义**：desktop `AppSettings` 移除 providers/default_model/skills_enabled/mcp_servers，UI 列表改从 facade 取，零残余读写。
- **保留字段**：workspaces/current_workspace/onboarding_completed/schema_version 保留；H-9 单写者事务保留并仍在 io_tests 中覆盖。
- **段 2 / P1-8 / i18n**：未顺手碰。
- **文档**：AGENTS.md / AGENTS-CN.md 骨干不变量条目语义准确。

**唯一 Minor**：desktop 推送测试命名大于断言（核心已覆盖安全断言，不影响放行；指向终审 triage）。

---

**APPROVED** — 单 Minor 不阻塞放行；指向终审 triage 跟进。