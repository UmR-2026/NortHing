# Task Report — W7-2: 设置页 provider 编辑弹窗 UI（F7 功能面）

状态：DONE

## 1. 改动清单

- **新文件** `src/apps/desktop/src/ui_dioxus/pages_settings_provider_edit.rs` (501 行):
  - `ProviderEditModal` 编辑弹窗组件：名称 / 类型下拉 (SUPPORTED_PROVIDER_TYPES) / Base URL / 模型 / API Key (password 输入，占位提示"留空 = 保持不变") / 启用状态 toggle。
  - 类型切换联动：类型变更时若 base_url 为空或为已知旧默认值，自动填充新类型默认 base_url (`default_base_url_for_type` / `is_known_default_url`)。
  - 三失败臂显式中文报错：测试失败 (`✗ 测试失败: ...`)、保存失败 (`保存失败: ...`)、删除被拒 (`删除失败: ...`)。
  - 两段式删除确认：点击删除进入确认态（显示红字提示与确认/取消按钮），确认后执行 `delete_provider`。
  - 密钥继承语义：留空时回落至 `PRODUCTION_KEYRING.get(id)` 进行测试/保存。
  - 3 例内联单元测试（类型映射覆盖、默认 URL 映射、已知默认 URL 判定）。
- **更新** `src/apps/desktop/src/ui_dioxus/pages_settings.rs` (+45 行，收口 776 行 ≤ 791 ceiling):
  - Card 3 (接入点 PROVIDER) 每行添加独立「编辑」小按钮，带 `e.stop_propagation()` 避免触发默认服务切换。
  - 添加 `editing_provider` 信号与 `refresh_providers` 刷新闭包，挂载 `ProviderEditModal`。
- **更新** `src/apps/desktop/src/ui_dioxus/mod.rs` (+1 行):
  - 注册 `mod pages_settings_provider_edit;` 模块。
- **硬红线完全遵守**：
  - `app.rs`: 零触碰
  - `api.rs`: 零触碰
  - `css.rs`: 零触碰（内联样式覆盖弹窗布局与微调）
  - `pages_onboarding.rs`: 零触碰

## 2. 编译错误定位与修复分层

- `E0369 (binary operation == cannot be applied to type ProviderConfigDto)`:
  - 修复层：**设计层**。`ProviderEditModalProps` 作为 Dioxus 0.8 组件属性，手动为 `ProviderEditModalProps` 实现 `PartialEq`（比对 provider 字段及回调），避免跨 crate 修改 `kernel-api` 的 DTO 契约。
- `E0599 (method get exists for struct Lazy<ProductionKeyring> but trait bounds not satisfied)`:
  - 修复层：**机制层**。引入 `KeyringBackend` trait 到作用域 (`use crate::app_state::settings::KeyringBackend;`)，使 `PRODUCTION_KEYRING.get(&id)` 正常解引用调用。

## 3. 验证证据

### 3.1 MSVC Check
命令：`& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`
输出尾部：
```
warning: `northhing` (bin "northhing") generated 44 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.58s
```
Warnings 变化：54 → 44（成功消费 W7-1 挂载的 `api_provider_edit` 导出，减少 10 个未消费警告，符合 ≤50 门禁）。

### 3.2 MSVC Test (109/109 全绿)
命令：`& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib`
输出尾部：
```
test ui_dioxus::pages_settings_provider_edit::tests::test_default_base_url_mapping ... ok
test ui_dioxus::pages_settings_provider_edit::tests::test_is_known_default_url ... ok
test ui_dioxus::pages_settings_provider_edit::tests::test_supported_provider_types_coverage ... ok
...
test ui_dioxus::api::api_provider_edit::tests::test_delete_provider_default_provider_rejected ... ok
test ui_dioxus::api::api_provider_edit::tests::test_delete_provider_success_cleans_config_and_keyring ... ok
test ui_dioxus::api::api_provider_edit::tests::test_edit_provider_blank_key_inherits_existing ... ok
test ui_dioxus::api::api_provider_edit::tests::test_edit_provider_keyring_read_error_fails_closed ... ok
test ui_dioxus::api::api_provider_edit::tests::test_edit_provider_new_key_overwrites_keyring ... ok
test ui_dioxus::api::api_provider_edit::tests::test_edit_provider_nonexistent_id_returns_error ... ok
test ui_dioxus::api::api_provider_edit::tests::test_edit_provider_validation_failure_zero_writes ... ok

test result: ok. 109 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
```

### 3.3 Rot Budget
命令：`node scripts/verify-rot-budget.mjs`
输出：
```
Rot budget verification passed (5 grep rules [unwrap_production=474/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=249/400], 11 god-file rules checked across 1344 files).
```

### 3.4 截图验收清单（已存 `.superpowers/sdd/`，不 commit）

1. `E:\agent-project\NortHing\.superpowers\sdd\w7-2-shot-1.png`: 设置页 Card 3 (接入点 PROVIDER) 现状，展示默认激活项与每行右侧「编辑」按钮。
2. `E:\agent-project\NortHing\.superpowers\sdd\w7-2-shot-2.png`: 编辑弹窗打开态，字段齐全（名称、类型下拉、Base URL、模型、API Key 带"留空 = 保持不变"占位提示、启用复选框）及操作按钮（删除、测试连接、保存、✕ 关闭）。
3. `E:\agent-project\NortHing\.superpowers\sdd\w7-2-shot-3.png`: 两段式删除确认态，展示红色警告"确定删除该服务？此操作不可撤销。"与"取消"/"确定删除"操作项。
4. `E:\agent-project\NortHing\.superpowers\sdd\w7-2-shot-4.png`: 失败臂报错态，展示测试连接失败时的顶部红色错误横幅与原因。

## 4. 偏离清单

无。
