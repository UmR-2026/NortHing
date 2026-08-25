# Task P1b-fix Brief — F5 Settings 持久化补全（数据源修正）

> 修正 P1b 的覆盖缺口。原 brief 把数据源错指为 AppSettings；实际 engine/provider/MCP 在 **facade `KernelSettingsApi`**（P0a 已建立 api.rs 通路模式）。
> Base: 当前 main tip（P1b 已落 workspace_path 接线，保留）。

## 背景

P1b 只接了 `workspace_path`（AppSettings），其余 8 个 toggle 留了 `// TODO(data)`。正确数据源是 facade settings API（已核实存在，`contracts/kernel-api/src/settings.rs:136-187`）：

- `get_global_config() -> GlobalConfigDto { providers, default_provider_id, ... }`
- `list_model_configs() -> Vec<AIModelConfigDto>`（字段：id/provider_id/model/display_name/enabled/...）
- `upsert_model_config(config, api_key)` / `delete_model_config(id)` / `set_default_provider(id)`
- `list_mcp_servers() -> Vec<MCPServerDto>`（含 `enabled: Option<bool>`）
- `upsert_mcp_server(config)` / `delete_mcp_server(id)`

## 步骤

### 1. `api.rs` 加 settings 薄封装（沿用 P0a 模式，纯薄封装）

```rust
pub async fn get_global_config() -> Result<GlobalConfigDto, KernelError>
pub async fn list_model_configs() -> Result<Vec<AIModelConfigDto>, KernelError>
pub async fn set_default_provider(id: &str) -> Result<(), KernelError>
pub async fn list_mcp_servers() -> Result<Vec<MCPServerDto>, KernelError>
pub async fn set_mcp_enabled(server: MCPServerDto, enabled: bool) -> Result<(), KernelError>
  // 内部：server.enabled = Some(enabled); facade.upsert_mcp_server(server)
```

### 2. `pages_settings.rs` 接线

- **模型引擎卡**（Card 1）：加载时 `list_model_configs()` → 渲染真实列表（替换 3 条 mock 行）；点击行 → `set_default_provider(&model.id)` + 乐观 Signal 更新。列表为空 → 显示现有 mock 行并保留 TODO 注释。
- **接入点卡**（Card 3）：`get_global_config()` 的 `providers` 渲染；active 判定 = `default_provider_id`；点击 → `set_default_provider(&provider.id)`。
- **能力集卡**（Card 4）：`list_mcp_servers()` 渲染；toggle → `set_mcp_enabled(server, !current)` + 乐观更新。
- **显示模式卡**（Card 6 呼吸/双光学）：AppSettings 无字段，**保留 mock + TODO**（不动）。
- 加载统一在页面 `use_future` 启动时拉一次；失败 warn + 保持现有 mock 展示（fail-open）。
- 乐观更新 + 失败仅 `tracing::warn!` 不回滚（与 P1b 语义一致）。

### 3. 不做

- 不动 io.rs/keyring.rs；不动 display 两开关；不加 api_key 输入；不碰其他页面。

## 验证（必跑并贴输出）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cd E:\agent-project\northing
cargo check -p northhing --features ui-dioxus
cargo test -p northhing --features ui-dioxus --lib ui_dioxus
```

报告：`.superpowers/sdd/reports/task-p1b-fix-settings-report.md`（status + files + 接线/保留清单 + 验证输出原文 + 偏离声明）。
