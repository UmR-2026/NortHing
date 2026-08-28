# W6-1 Task Report: allow_dead_code 清账（128 → 106 ≤ 109）

## 1. 改动概述 (Modification Summary)

- **范围**: `src/apps/desktop`（settings/keyring, settings/types, settings/tests, ui_dioxus/i18n, ui_dioxus/registry）及 `src/crates/assembly/core/locales/*.ftl`。
- **目标**: 清理生产无引用的死代码与误标注的 `#[allow(dead_code)]`，净削减 `allow(dead_code)` 计数 ≥19（从基线 128 降至 ≤109）。
- **实测落点**: `allow_dead_code` 计数从 **128** 降至 **106**（净删 **22** 处），达成指标且低于 ceiling 109。

---

## 2. 站点行动表执行详情 (Action Table Execution)

### A. 删码 + 删标注 (True Dead Code Deletions)
1. `ui_dioxus/i18n.rs` const `DECK_WITNESS_NOTE`: 删除 const + 3 份 .ftl 对应词条。
2. `ui_dioxus/i18n.rs` const `VLABEL_INNER`: 删除 const + 3 份 .ftl 对应词条。
3. `ui_dioxus/i18n.rs` const `VLABEL_OUTER`: 删除 const + 3 份 .ftl 对应词条。
4. `ui_dioxus/i18n.rs` const `INNER_HEAD_TITLE`: 删除 const + 3 份 .ftl 对应词条。
5. `ui_dioxus/i18n.rs` const `INNER_SECTION_ENGINE_TITLE`: 删除 const + 3 份 .ftl 对应词条。
6. `ui_dioxus/i18n.rs` const `INNER_SECTION_ENGINE_EM`: 删除 const + 3 份 .ftl 对应词条。
7. `ui_dioxus/i18n.rs` const `INNER_SECTION_CONTEXT_TITLE`: 删除 const + 3 份 .ftl 对应词条。
8. `ui_dioxus/i18n.rs` const `INNER_SECTION_CONTEXT_EM`: 删除 const + 3 份 .ftl 对应词条。
9. `app_state/settings/keyring.rs` fn `resolve_api_key`: 删除函数、清理模块 doc 注释引用、删除 4 个专属测试用例。
10. `app_state/settings/types.rs` `impl ProviderType { default_base_url, default_models }`: 删除 impl 块；`settings/tests.rs` 对应测试用例同步清理。
11. `app_state/settings/types.rs` `ProviderConfig::new`: 删除 impl 块及未使用的 `use uuid::Uuid;`；`settings/tests.rs` 对应测试同步清理。
12. `app_state/settings/types.rs` struct `ModelRef`: 全仓零引用，删除 struct 定义及标注。
13. `ui_dioxus/registry.rs` fn `register_window`: 删除函数；对应测试改用生产路径 `register_window_with_hwnd`。
14. `ui_dioxus/registry.rs` fn `mark_closing`: 删除函数；对应测试改用生产路径 `mark_closing_target`。
15. `ui_dioxus/registry.rs` fn `get_window_id`: 删除函数；对应测试改用 `get_window_target`。
16. `ui_dioxus/registry.rs` fn `get_hwnd`: 删除函数；对应测试改用 `get_window_target`。

### B. 仅删标注 (Mis-annotated Un-annotations)
1. `ui_dioxus/i18n.rs` const `INNER_HEAD_FACILITY_TITLE`: 移除 `#[allow(dead_code)]`（在 `ui_dioxus/windows.rs:307, 483` 生产调用，见偏离清单）。
2. `app_state/settings/keyring.rs` fn `is_keyring_sentinel`: 移除 `#[allow(dead_code)]`（生产链 `api.rs` -> `store_api_key`）。
3. `app_state/settings/keyring.rs` fn `is_env_sentinel`: 移除 `#[allow(dead_code)]`（`io.rs` 活跃引用）。
4. `app_state/settings/keyring.rs` fn `make_env_sentinel`: 移除 `#[allow(dead_code)]`（`io.rs:101` 活跃调用）。
5. `app_state/settings/keyring.rs` fn `store_api_key`: 移除 `#[allow(dead_code)]`（`api.rs:175` 活跃调用）。
6. `app_state/settings/types.rs` enum `MCPTransport`: 移除 `#[allow(dead_code)]`（作为 `MCPServerConfig.transport` 活跃类型，试删后 cargo check 无警告）。

### C. i18n .ftl 同步
- `src/crates/assembly/core/locales/zh-CN.ftl`、`zh-TW.ftl`、`en-US.ftl` 同步移除了 8 个已删 const 的词条。
- `pnpm run i18n:audit` 审计前后均为 11 处 pre-existing 失败，失败数零增长。

### D. 禁止删除项防护
- `API_KEY_SENTINEL` / `MCP_ENV_SENTINEL` / `MockKeyring` + 其 `impl` 保持原样未动。
- `ProviderType` / `ProviderConfig` 结构体定义（serde/磁盘格式）保留。
- `state.rs` 中的 `is_dark` / `toggle` 保持原样未动。

---

## 3. 偏离清单 (Deviations)

1. **`INNER_HEAD_FACILITY_TITLE` 从 A 表重分类至 B 表**:
   - 行动表 A 中原计划删除 `INNER_HEAD_FACILITY_TITLE`。
   - 侦察核实：`INNER_HEAD_FACILITY_TITLE` 在 `src/apps/desktop/src/ui_dioxus/windows.rs` 第 307 行（`w2-group-label`）与第 483 行（`station-head facility w2-head`）为正在运行的生产 UI 引用，属于误标 `#[allow(dead_code)]`。
   - 执行：保留该 const 及 3 份 `.ftl` 词条，仅删除其 `#[allow(dead_code)]` 标注。计数贡献同样为 -1。

---

## 4. 验证命令与输出原文 (Verification Results)

### 4.1 Rot Budget 验证
- 命令: `node scripts/verify-rot-budget.mjs`
- 输出:
```text
unwrap_production: current 518 exceeds ceiling 502 — split, reduce, or register a justified manifest entry (raising a ceiling requires user sign-off)
expect_production: current 1106 exceeds ceiling 1089 — split, reduce, or register a justified manifest entry (raising a ceiling requires user sign-off)
let_underscore: current 390 exceeds ceiling 388 — split, reduce, or register a justified manifest entry (raising a ceiling requires user sign-off)
Rot budget verification failed with 3 violation(s).
```
- 指标明细: `allow_dead_code` 从 **128** 降至 **106**（不再在 violations 中，低于 ceiling 109）。

### 4.2 i18n 审计
- 命令: `pnpm run i18n:audit`
- 输出:
```text
[i18n:audit] Failed with 11 error(s) and 0 warning(s).
```
（基线为 11 处 pre-existing 错误，前后一致，零新增）。

### 4.3 Cargo Check (MSVC)
- 命令: `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`
- 输出:
```text
warning: `northhing` (bin "northhing") generated 50 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 23.22s
```
（0 error，50 warnings 与基线 50 warnings 完全一致，无任何新增 warning）。

### 4.4 Cargo Test (MSVC)
- 命令: `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib`
- 输出:
```text
test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s
```
（全部 103 个测试通过，0 failed）。

### 4.5 Workspace 检查
- 命令: `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check --workspace`
- 输出:
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 55s
```

---

## 5. 编译错误分析分层 (Error Layer Trace)

- 本任务无生命周期/并发/所有权编译错误（0 E0xxx 错误），改动属于机制层清理（死代码/误标注属性移除及生产接口替换）。
