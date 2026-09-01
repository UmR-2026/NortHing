# Task Report — W7-1: provider 编辑/删除 API 层（Dioxus 包装 + keyring 语义）

状态：DONE

## 1. 改动清单

- **新文件** `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs` (403 行)：
  - `edit_provider_with_keyring`: 载入现有模型、解析 API key (resolve_edit_api_key fail-closed)、校验入参 (validate_provider_input)、覆盖写 keyring (非空白 key)、映射 wire format (provider_wire_format_from_str)、持久化模型配置 (upsert_model_config)。
  - `edit_provider`: 生产 keyring 包装函数。
  - `delete_provider_with_keyring`: 检查并拒绝删除默认 provider、调用 facade.delete_model_config、best-effort delete_api_key。
  - `delete_provider`: 生产 keyring 包装函数。
  - 7 例完整内联单元测试（含 MockKeyring 与 FailingKeyring）。
- **更新** `src/apps/desktop/src/ui_dioxus/api.rs` (+8 行，收口 726 行 ≤ 728 ceiling)：
  - 挂载 `#[path = "api_provider_edit.rs"] mod api_provider_edit;` 并 `pub use api_provider_edit::*;`。
  - 添加 `pub(crate) static TEST_GLOBAL_CONFIG_MUTEX` 隔离测试对 global config 的并发访问。
- **更新** `src/apps/desktop/src/app_state/settings/sync.rs` (-10 行)：
  - 删除无调用者的 `resolve_effective_api_key`（顺手清配额）。
  - `resolve_edit_api_key` 恢复活跃使用。
- **更新** `src/apps/desktop/src/app_state/settings/tests.rs` (-25 行)：
  - 移除已废弃 `resolve_effective_api_key` 对应的 4 个测试。

## 2. 字符串集比对：`infer_provider_wire_format` vs `provider_wire_format_from_str`

- **`infer_provider_wire_format(base_url, model)`**：
  - 基于 URL 与 model 字符串启发式匹配：
    - `base_url` 含 `"anthropic"` 或 `model` 以 `"claude"` 开头 → `"anthropic"`
    - `base_url` 含 `"google"`/`"gemini"` 或 `model` 以 `"gemini"` 开头 → `"gemini"`
    - 其余情况 → `"openai"`
- **`provider_wire_format_from_str(s)`**：
  - 基于 UI 显式选择的类型映射：
    - `"anthropic" | "custom-anthropic"` → `"anthropic"`
    - `"openai" | "custom-openai"` → `"openai"`
    - `"gemini"` → `"gemini"`
    - 其他未识别字符串 fallback → `"openai"`
  - **编辑流程严格使用 `provider_wire_format_from_str`**，避免因用户修改 base_url 时受 URL 启发式误判影响 wire format。

## 3. 写失败不回滚 Keyring 决策理由

- `upsert_model_config` 失败时不回滚已写入 keyring 的新 key：
  - **无害残留**：key 在 OS keyring 中仅由 provider ID 索引，未被 core 配置引用时处于孤立无害状态。
  - **避免双写复杂性与二次故障**：若在 catch/error 分支尝试回滚（删除或恢复旧 key），若回滚本身遭遇 keyring 读写失败将引入更严重的双写不一致与不可预测状态，遵循简单可靠的单向写入原则。

## 4. 编译错误定位与修复分层

- `E0583 (file not found for module api_provider_edit)`:
  - 修复层：**机制层**。`api.rs` 位于 `ui_dioxus/` 同级目录下，通过 `#[path = "api_provider_edit.rs"] mod api_provider_edit;` 显式指示模块文件路径，解决 Rust 2021 模块解析路径预期。

## 5. 验证证据

### 1. MSVC Check
命令：`& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`
输出尾部：
```
warning: `northhing` (bin "northhing") generated 54 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 2 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.42s
```

### 2. MSVC Test（含 7 例新测试，106/106 全绿）
命令：`& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib`
输出尾部：
```
test ui_dioxus::api::api_provider_edit::tests::test_delete_provider_default_provider_rejected ... ok
test ui_dioxus::api::api_provider_edit::tests::test_delete_provider_success_cleans_config_and_keyring ... ok
test ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized ... ok
test app_state::settings::io::io_tests::update_with_err_closure_does_not_write_file ... ok
test app_state::settings::io::io_tests::concurrent_loads_and_updates_preserve_all_writes ... ok
test ui_dioxus::api::api_provider_edit::tests::test_edit_provider_blank_key_inherits_existing ... ok
test ui_dioxus::api::api_provider_edit::tests::test_edit_provider_keyring_read_error_fails_closed ... ok
test ui_dioxus::api::tests::test_api_functions_fail_cleanly_before_init ... ok
test ui_dioxus::api::api_provider_edit::tests::test_edit_provider_new_key_overwrites_keyring ... ok
test ui_dioxus::api::api_provider_edit::tests::test_edit_provider_nonexistent_id_returns_error ... ok
test ui_dioxus::api::api_provider_edit::tests::test_edit_provider_validation_failure_zero_writes ... ok
test ui_dioxus::api::tests::test_persist_onboarding_provider_success_flow ... ok

test result: ok. 106 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
```

### 3. Rot Budget
命令：`node scripts/verify-rot-budget.mjs`
输出：
```
Rot budget verification passed (5 grep rules [unwrap_production=474/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=240/400], 11 god-file rules checked across 1343 files).
```

## 6. 偏离清单

无。
