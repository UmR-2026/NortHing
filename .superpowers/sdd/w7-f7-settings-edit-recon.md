# F7 Provider 编辑功能 — 只读侦察报告

> 产物类型：只读侦察（无代码改动）  
> 仓库：`E:\agent-project\NortHing` @ main HEAD=`18c0332`  
> 日期：2026-08-28  
> 输入：W4-2 壳审计 F7（`pages_settings.rs:444-488` 无 provider 编辑 UI）+ handoff 决策

---

## 1. 用户视角功能清单

### 设置页现在能干什么

| 功能 | 实现状态 | 数据来源 |
|---|---|---|
| **模型引擎**（Card 1）— 列出所有 AI 模型，点击设为默认 | ✅ 真后端 | `get_global_config()` + `list_model_configs()` → `set_default_provider()` (`pages_settings.rs:131-153, 352-406`) |
| **接入点/Provider**（Card 3）— 列出所有 provider，点击设为默认 | ✅ 真后端（仅列表+默认选择） | `get_global_config()` + `set_default_provider()` (`pages_settings.rs:434-488`) |
| **能力集/MCP**（Card 4）— 列出 MCP 服务器，toggle 启用/禁用 | ✅ 真后端 | `list_mcp_servers()` + `set_mcp_enabled()` (`pages_settings.rs:490-562`) |
| **工作区**（Card 5）— 显示当前路径 + 重新定位按钮 | ⚠️ 部分 mock | `load_app_settings()` 读路径；重新定位按钮无 handler (`pages_settings.rs:565-590`) |
| **显示模式**（Card 6）— 呼吸/双镜 toggle | ❌ TODO | `display_breath` / `display_dual_optics` 是 mock 信号，注释 `TODO(data): no AppSettings field yet` (`pages_settings.rs:592-618`) |
| **编年史/沉积/身份/准则**（Card 2/3/4 左列） | ❌ 纯展示 | 硬编码字符串，无后端接线 |
| **上下文**（Card 2 右列） | ❌ 纯展示/mock | 硬编码「全局作用域」 |

### 设置页不能干什么（F7 缺口）

- ❌ **编辑现有 provider**（改名称/URL/模型/key）
- ❌ **添加新 provider**（除新手引导 onboarding 外）
- ❌ **删除 provider**（Slint 侧有 `delete_provider` 回调，已随 Slint 删除）
- ❌ **测试 provider 连接**（从设置页内；onboarding 有独立测试流程）

---

## 2. 技术现状（file:line 锚点）

### 2.1 设置页卡片接线矩阵

| 卡片 | 数据加载 | 写操作 | facade 调用链 | 实现状态 |
|---|---|---|---|---|
| Card 1 引擎 | `get_global_config()` L131 + `list_model_configs()` L146 | `set_default_provider()` L368 | pages_settings → api.rs:139,144,149 → kernel_facade.settings | ✅ 已接线 |
| Card 3 接入点 | `get_global_config()` L131（providers 字段） | `set_default_provider()` L460 | pages_settings → api.rs:139 → kernel_facade.settings:17 | ✅ 仅列表+默认 |
| Card 4 MCP | `list_mcp_servers()` L155 | `set_mcp_enabled()` L527 | pages_settings → api.rs:154,159 → kernel_facade.settings | ✅ 已接线 |
| Card 5 工作区 | `load_app_settings()` L118 | 无 | pages_settings → app_state/settings/io.rs | ⚠️ 只读 |
| Card 6 显示 | 无 | 无 | — | ❌ TODO |

### 2.2 Kernel facade provider 相关 API 全清单

| 方法 | 签名 | file:line（facade impl） | 已接线 Dioxus API |
|---|---|---|---|
| `get_global_config` | `→ Result<GlobalConfigDto, KernelError>` | `kernel_facade/settings.rs:17` | ✅ `api.rs:139` |
| `list_model_configs` | `→ Result<Vec<AIModelConfigDto>, KernelError>` | `kernel_facade/settings.rs:47` | ✅ `api.rs:144` |
| `upsert_model_config` | `(config: AIModelConfigDto, api_key: Option<String>) → Result<(), KernelError>` | `kernel_facade/settings.rs:71` | ✅ `api.rs:189` |
| `delete_model_config` | `(id: &str) → Result<(), KernelError>` | `kernel_facade/settings.rs:173` | ❌ **未接线**（仅测试 api.rs:472） |
| `set_default_provider` | `(id: &str) → Result<(), KernelError>` | `kernel_facade/settings.rs:186` | ✅ `api.rs:149` |
| `test_provider` | `(id: &str) → Result<ProviderTestResultDto, KernelError>` | `kernel_facade/settings.rs:~200` | ✅ `api.rs:165` |
| `test_provider_config` | `(form: ProviderFormDto) → Result<ProviderTestResultDto, KernelError>` | `kernel_facade/settings.rs:~210` | ✅ `api.rs:165` |

**关键发现：`delete_model_config` 在 facade 层已实现，但 Dioxus `api.rs` 没有暴露一个 `delete_provider` 包装函数。**

### 2.3 Onboarding 调用序列

```
pages_onboarding.rs:135-181  (run_test_provider)
  → ProviderFormDto { provider_id:"onboarding", base_url, api_key, model, provider_type }
  → api::test_provider_config()

pages_onboarding.rs  (submit flow, around line 230+)
  → api::persist_onboarding_provider(model, base_url, api_key, agent_name)
    → api::persist_onboarding_provider_with_keyring(keyring, model, base_url, api_key, agent_name)
      → 1. store_provider_api_key_with_keyring(keyring, provider_id, api_key)
      → 2. upsert_model_config(model_dto, Some(api_key))
      → 3. set_default_provider(provider_id)
```

字段集：`model` + `base_url` + `api_key` + `agent_name`（→ `display_name`）。  
`provider_type` 由 `infer_provider_wire_format(base_url, model)` 自动推导（`sync.rs:27-37`）。

### 2.4 Keyring Sentinel 机制

| 概念 | 位置 | 说明 |
|---|---|---|
| `API_KEY_SENTINEL = "__kr__"` | `keyring.rs:56` | 磁盘上的占位符，表示 key 在 OS keyring 中 |
| `is_keyring_sentinel()` | `keyring.rs:64` | 判断是否为 sentinel |
| `store_api_key()` | `keyring.rs:219` | 写 OS keyring，返回 sentinel |
| `delete_api_key()` | `keyring.rs:233` | 从 OS keyring 删除（best-effort，missing 不报错） |
| `resolve_edit_api_key()` | `sync.rs:16` | **编辑语义**：空白 incoming ← 保留 stored keyring key；非空则覆盖 |
| `resolve_effective_api_key()` | `sync.rs:5` | 兄弟函数，签名不同（`Option<&str>` vs `anyhow::Result<String>`） |
| `prepare_settings_for_save()` | `io.rs:89` | 序列化前将 api_key 替换为 sentinel |
| `push_resolved_keys_to_core()` | `sync.rs:53` | 启动时从 keyring 读 key → 写 core 内存 |

**Dead code 确认：** `resolve_edit_api_key` + `resolve_effective_api_key` 在 Dioxus 侧无调用者（Slint 删除后 orphan）。handoff 已记录：「I1 修复随 Slint 回调层已删」。

### 2.5 MockKeyring 测试设施

| 设施 | 位置 | 说明 |
|---|---|---|
| `MockKeyring` struct | `keyring.rs:139` | `HashMap` 后端，所有 build 可用 |
| `MockKeyring::new()` | `keyring.rs:146` | 构造 |
| `MockKeyring::seed()` | `keyring.rs:151` | 预填充 |
| `assert_contains()` | `keyring.rs:159` | 断言含某 key |
| `assert_not_contains()` | `keyring.rs:167` | 断言不含某 key |
| 已有测试 | `api.rs:378-476` | `test_api_functions_fail_cleanly_before_init` + `test_persist_onboarding_provider_success_flow` |

---

## 3. 缺口清单（编辑功能要新建的，按依赖排序）

### G1：Dioxus API 层缺少 delete/edit 包装函数

**现状：** `api.rs` 有 `upsert_model_config`（L189）、`set_default_provider`（L149）、`test_provider_config`（L165），但 **没有** `delete_provider` 和 **没有** `edit_provider`（= 带 keyring 集成的 upsert 编辑流）。

**需要新建：**
- `delete_provider(provider_id: &str) → Result<(), KernelError>` — 包装 `kernel_facade().delete_model_config()` + `delete_api_key()`
- `persist_edit_provider_with_keyring(keyring, id, name, type_str, base_url, api_key, model, enabled)` — 复用 Slint callback 的 edit 语义（`resolve_edit_api_key`）

### G2：Dioxus API 层缺少 edit 流的 keyring 集成

**现状：** `persist_onboarding_provider_with_keyring`（api.rs:197）是 new provider 专用。编辑需要：
1. 读当前 keyring key（`keyring.get(id)`）
2. 如果用户留空 key 字段 → 继承已有的（Slint 用 `resolve_edit_api_key`）
3. 如果用户填了新 key → 覆盖
4. `upsert_model_config(dto, Some(effective_key))` + `delete_api_key`（如果 key 被清空）

### G3：设置页缺少 provider 编辑弹窗/表单 UI

**现状：** Card 3（`pages_settings.rs:434-488`）只有列表 + 点击设为默认。需要新增：
- 每行添加编辑按钮（复用 Slint 的「编辑弹窗」模式，或 Dioxus 浮层）
- 编辑表单字段集（见下方 §4）

### G4：`infer_provider_wire_format` URL 启发式在编辑路径的影响

**风险：** `infer_provider_wire_format`（`sync.rs:27-37`）的启发式规则：
- URL 含 "anthropic" 或 model 以 "claude" 开头 → `"anthropic"`
- URL 含 "google"/"gemini" 或 model 以 "gemini" 开头 → `"gemini"`
- 其他 → `"openai"`

**问题：** 用户在编辑时改了 URL（例如从 anthropic 切到 openai compatible 代理），但 model 名仍含 "claude" → 自动推断可能产生非预期 wire format。Slint 侧用 `provider_wire_format_from_str` 反转 UI 的 `ProviderType` enum → wire format。编辑表单应提供 **显式 provider_type 选择器**（下拉框），不依赖 URL 启发式推断。

### G5：`delete_model_config` 不清理 keyring + 无引用完整性检查

**现状：** facade `delete_model_config`（`kernel_facade/settings.rs:173-184`）只删 core 配置，**不**删 keyring。Slint callback（`callbacks_settings/provider.rs:register_delete_provider_callback`）做了两步：
1. `facade.delete_model_config(&pid)`
2. `delete_api_key(&PRODUCTION_KEYRING, &pid)`
3. 引用完整性检查（session metadata 中引用了该 provider 的会话 → 提示用户 + 自动 fallback）

Dioxus 侧需重建这个三步流程。

---

## 4. 编辑表单字段集

### 4.1 字段对照（onboarding + Slint 历史 + DTO 能力）

| 字段 | onboarding (`persist_onboarding_provider`) | Slint (`register_upsert_provider_callback`) | DTO (`AIModelConfigDto`) | Dioxus 设置页显示 | 编辑表单需要？ |
|---|---|---|---|---|---|
| `id` | 自动生成 UUID | 编辑时传已有 id | `id: String` | ✅ 已有 | 隐藏（不可改） |
| `name` / `display_name` | `agent_name` → display_name | `name` 参数 | `display_name: Option<String>` | ✅ 已有 | ✅ 可编辑 |
| `provider_type` | `infer_provider_wire_format(url, model)` | `type_str` → `provider_wire_format_from_str` | `provider_id: String` | ✅ 已有（显示） | ✅ 下拉选择 |
| `base_url` | 用户输入 | 用户输入 | `base_url: Option<String>` | ❌ 不显示 | ✅ 可编辑 |
| `api_key` | 用户输入 | 用户输入 | 不存 DTO（通过 `api_key: Option<String>` 参数传入） | ❌ 不显示 | ✅ 可编辑（password 字段） |
| `model` | 用户输入 | 用户输入 | `model: String` | ❌ 不显示 | ✅ 可编辑 |
| `enabled` | `true` | `enabled` 参数 | `enabled: Option<bool>` | ❌ 不显示 | ✅ toggle |
| `category` | `"general_chat"`（硬编码） | 硬编码 | `category: Option<String>` | — | ❌ 不需要编辑 |
| `capabilities` | `["text_chat"]` | 硬编码 `["text_chat","function_calling"]` | `capabilities: Option<Vec<String>>` | — | ❌ 不需要编辑 |
| `auth` | `"api_key"` | 硬编码 | `auth: Option<String>` | — | ❌ 不需要编辑 |
| `max_tokens` | `None` | `None` | `max_tokens: Option<u32>` | — | ⏸ 预留 |
| `temperature` | `None` | `None` | `temperature: Option<f64>` | — | ⏸ 预留 |

### 4.2 编辑表单字段集（推荐最小集）

```
┌─────────────────────────────────────────────┐
│ 编辑 AI 服务                    [✕ 关闭]    │
├─────────────────────────────────────────────┤
│  名称    [________________________]          │
│  类型    [▼ anthropic ▼]                     │
│  Base URL [________________________]         │
│  模型    [________________________]          │
│  API Key  [________________________]  (留空=不变) │
│  ☑ 启用                                        │
│                                               │
│  [测试连接]  [保存]  [删除]                    │
└─────────────────────────────────────────────┘
```

**交互语义：**
- API Key 留空 = 继承 keyring 中已有 key（不自 Slint 的 `resolve_edit_api_key`）
- 清空 API Key = 从 keyring 删除该 key（更新后 key 为空）
- 类型变更 → 自动填充默认 base_url（可改写）
- 保存 → `upsert_model_config(dto, Some(effective_key))` + 如果之前有 keyring 且新 key 为空则 `delete_api_key`

---

## 5. 复用清单

| 已有能力 | 来源 | F7 可直接复用？ |
|---|---|---|
| `KernelSettingsApi::upsert_model_config(config, api_key)` | `kernel_facade/settings.rs:71` | ✅ 核心写入路径 |
| `KernelSettingsApi::delete_model_config(id)` | `kernel_facade/settings.rs:173` | ✅ 删 core 配置 |
| `KernelSettingsApi::test_provider_config(form)` | facade + `api.rs:165` | ✅ 测试连接 |
| `KernelSettingsApi::set_default_provider(id)` | facade + `api.rs:149` | ✅ 设为默认 |
| `store_api_key(keyring, provider_id, plaintext)` | `keyring.rs:219` | ✅ 写 keyring |
| `delete_api_key(keyring, provider_id)` | `keyring.rs:233` | ✅ 删 keyring key |
| `resolve_edit_api_key(stored, incoming)` | `sync.rs:16` | ⚠️ Dead code，需解 `#[allow(dead_code)]` 并迁到 api 层 |
| `resolve_effective_api_key(stored, incoming)` | `sync.rs:5` | ⚠️ 同上（兄弟函数） |
| `validate_provider_input(name, type, url, key, model)` | `sync.rs:72` | ✅ 已接线到 Slint；Dioxus 侧未使用，可直接复用 |
| `infer_provider_wire_format(url, model)` | `sync.rs:27` | ✅ onboarding 在用；编辑流程建议用显式下拉替代 |
| `provider_wire_format_from_str(s)` | `sync.rs:40` | ✅ UI enum → wire format 映射 |
| `MockKeyring` + 断言辅助 | `keyring.rs:139-173` | ✅ 测试直接用 |
| `ProviderConfigDto` / `AIModelConfigDto` | `kernel-api/src/settings.rs` | ✅ DTO 已正确定义 |
| `ProviderFormDto` | `kernel-api/src/settings.rs:118` | ✅ 用于测试连接 |
| `test_persist_onboarding_provider_success_flow` | `api.rs:434` | 📝 测试模板，编辑流程可仿写 |
| Slint `register_upsert_provider_callback` | `callbacks_settings/provider.rs`（已删，git 可查） | 📖 参考实现（edit key 继承语义） |
| Slint `register_delete_provider_callback` | 同上 | 📖 参考实现（三步删除流程） |

---

## 6. 风险 / 坑

### R1：`resolve_edit_api_key` 是 dead code — 需先复活再引用

`sync.rs:16` 的 `resolve_edit_api_key` 当前 `#[allow(dead_code)]`，Dioxus 侧无调用者。F7 编辑流程必须依赖此语义（空白 key → 保留已有）。**做法：去掉 `#[allow(dead_code)]`，在 `api.rs` 新增 `edit_provider_with_keyring` 时复用。**

### R2：`infer_provider_wire_format` 启发式不可靠用于编辑

用户在编辑时改了 URL 或 model 名，自动推断可能产生错误 wire format。Slint 编辑流程用 `provider_wire_format_from_str` 将用户显式选择的 `ProviderType` 映射为 wire format。**Dioxus 编辑表单必须有显式 provider_type 下拉框**（anthropic / openai / gemini / custom-openai / custom-anthropic），不依赖 URL 启发式。

### R3：`delete_model_config` 不删 keyring — 需手动补两步

facade 层只删 core 配置。Slint 的 delete callback 显式做了 `delete_api_key` + 引用完整性检查。Dioxus edit/delete 流程必须：
1. `kernel_facade().delete_model_config(id)`
2. `delete_api_key(keyring, id)`
3. 可选：引用完整性提示（有会话用此 provider → 影响 N 个会话）

### R4：I1 波修复的「编辑不抹 key」语义需在 Dioxus 侧重建

Slint 侧通过 `resolve_edit_api_key(PRODUCTION_KEYRING.get(&pid), &pkey)` 实现。Dioxus 侧此逻辑为 dead code。F7 实现时需将这段逻辑从 `sync.rs` 复活并接入编辑流程。注意 I1 修复附带：keyring 读失败 → **fail-closed**（拒绝保存，不等同于「清空 key」）。

### R5：`AIModelConfigDto` 与 `ProviderConfig` 的字段落差

核心用 `AIModelConfigDto`（无 `api_key` 字段，key 走独立参数），旧的 `ProviderConfig`（`types.rs:27-48`）有 `api_key: String` 字段（plaintext in app.json，Scheme C 已废弃）。设置页 Card 3 读取的是 `ProviderConfigDto`（kernel-api 层，也有 `api_key` 缺失）。编辑表单必须认识到：
- 读：`ProviderConfigDto` 含 `provider_type`（string），不含 `api_key`
- 写：`AIModelConfigDto` + 独立 `api_key: Option<String>` 参数

**意味着：编辑表单加载时，API key 从 keyring 读（`keyring.get(id)`），不在 DTO 中。**

### R6：W5-3 已知坑 — `infer_provider_wire_format` URL 启发式

同上 R2。W5-3 的 `upsert_model_config` 在 onboarding 流程中用了 `infer_provider_wire_format` 推导 `provider_id` 字段。如果用户在编辑时变更了 base_url，且不显式选 provider_type，wire format 可能不一致。

### R7：测试覆盖缺口

- `MockKeyring` 可用 ✅
- 但 `edit_provider_with_keyring` 不存在 → 需新建测试
- key 继承语义（留空 = 不变）需独立测试覆盖
- delete keyring entry 的 cleanup 需测试

---

## 7. 是否发现"已有编辑能力半成品"

**是，但半成品在 Slint 侧，已随 Slint 删除。**

| 半成品 | 状态 | 说明 |
|---|---|---|
| `callbacks_settings/provider.rs` | ❌ 已删（`707e414`） | 含 `register_upsert_provider_callback`（编辑+新增）和 `register_delete_provider_callback`（删除+keyring cleanup） |
| `callbacks_settings/provider_test.rs` | ❌ 已删（`707e414`） | `register_test_provider_callback` |
| `resolve_edit_api_key` | ⚠️ Dead code | `sync.rs:16`，函数体正确但无调用者 |
| `resolve_effective_api_key` | ⚠️ Dead code | `sync.rs:5`，兄弟函数 |
| `validate_provider_input` | ⚠️ 未接线 Dioxus | `sync.rs:72`，Slint 在用，Dioxus 侧未暴露 |
| Dioxus `pages_settings.rs` Card 3 | ⚠️ 只读列表 | 424-488 行，无编辑入口 |

**结论：** F7 不是从零开始——Slint 的完整编辑流程（含 keyring 交互、key 继承、校验、facade 写入）已在 `707e414^` 中存在且实测可用。核心逻辑（`upsert_model_config` + keyring 操作 + `resolve_edit_api_key` 语义）可以直接移植到 Dioxus `api.rs`。真正要从零新建的是：
1. Dioxus 侧 API 包装（`api.rs` 新增 2-3 个函数）
2. 设置页弹窗/表单 UI（`pages_settings.rs` 新增 modal + 字段）
3. 测试（复用 `MockKeyring`）
