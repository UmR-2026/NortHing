# Task T2-2n Report — MiniApp 整删 M4：product-domains miniapp 整删（含内置 6 件套资产 + 专测）

## 1. 基础信息
- 任务：Task T2-2n (MiniApp 子系统整删 M4)
- 工作目录：`E:\agent-project\northing`
- 实测 HEAD：`62163f6 sdd: T2-2m ledger line + brief/report/review/diff artifacts`
- 最终状态：`DONE`

---

## 2. B6 依赖独占性复核（删除前 rg 证据）
执行命令：
```powershell
git grep -n -E "dirs::|sha2::|which::" src/crates/contracts/product-domains/
```
输出原文：
```text
src/crates/contracts/product-domains/src/miniapp/builtin.rs:10:use sha2::{Digest, Sha256};
src/crates/contracts/product-domains/src/miniapp/permission_policy.rs:113:                    dirs::home_dir().into_iter().collect()
src/crates/contracts/product-domains/src/miniapp/runtime.rs:45:        which::which(name).ok()
```
结论：`dirs`、`sha2`、`which` 三个 optional 依赖在 `product-domains` 内完全独占于 `src/miniapp/` 模块，在 `function_agents` 及 crate 其它部分无任何使用，确认满足独占性删除条件。

---

## 3. 变更清单与文件明细

### A. miniapp 目录与专测整删
1. **整删目录** `src/crates/contracts/product-domains/src/miniapp/`（16 个 .rs 共 3,885 行 + `builtin/assets/` 6 件套应用资产共 55,889 行，含 ppt-live 27,805 行 bundle）。
2. **整删专测** `src/crates/contracts/product-domains/tests/` 下 6 个 miniapp 专测文件及目录：
   - `tests/builtin_and_ports.rs` (deleted)
   - `tests/compiler_export_storage_and_runtime.rs` (deleted)
   - `tests/host_routing_and_lifecycle_helpers.rs` (deleted)
   - `tests/permissions_and_bridge.rs` (deleted)
   - `tests/runtime_facade_and_customization.rs` (deleted)
   - `tests/common/mod.rs` (deleted)
   - 保留 `tests/function_agent_contracts.rs`。
3. **`src/crates/contracts/product-domains/src/lib.rs`**：
   - 删除 lines 7-8：`#[cfg(feature = "miniapp")] pub mod miniapp;`

### B. Cargo.toml 与依赖配置
4. **`src/crates/contracts/product-domains/Cargo.toml`**：
   - 删除 `dirs`、`sha2`、`which` optional 依赖行。
   - 删除 `miniapp = ["dirs", "sha2", "which"]` feature 行。
   - `product-full` feature 由 `["miniapp", "function-agents"]` 改为 `["function-agents"]`。
5. **`Cargo.lock`**：
   - 由 cargo 自动同步收敛（不手工编辑）。

### C. i18n-audit 扫描挂点
6. **`scripts/i18n-audit.mjs`**：
   - 仅删除 `createLocaleFormatScanSpecs` 中 `core-miniapp` spec 5 行（lines 3566-3570）：
     ```javascript
         {
           surface: 'core-miniapp',
           root: path.join(root, 'src', 'crates', 'contracts', 'product-domains', 'src', 'miniapp', 'builtin', 'assets'),
           predicate: (file) => file.endsWith('.js'),
         },
     ```
   - 严格遵守 mojibake 红线，文件其余所有字节 byte-preserved。

### D. Boundary 规则同步
7. **`scripts/core-boundaries/rules/feature-rules.mjs`**：
   - `product-domains` optional dependencies 由 `dirs`/`sha2`/`which` 清空为 `[]`。
   - `ownerCrateFeatureAssemblyRules` 中 `product-domains` 的 `requiredProductFullFeatures` 摘除 `'miniapp'`，保留 `['function-agents']`。
8. **`scripts/core-boundaries/rules/source/forbidden-rules.mjs`**：
   - `product-domains` 规则中移除 `Command::new` 的 `allowPaths`（`runtime.rs`）例外，恢复为全域禁用。
9. **`scripts/core-boundaries/rules/source/required-rules.mjs`**：
   - 删除 `product-domains` 层 11 个 miniapp 模块契约规则块及 `builtin.rs` 契约规则块。
10. **`scripts/core-boundaries/self-test.mjs`**：
    - 移除 `product-domains` optional dependency 校验（`dirs`/`sha2`）。
    - 移除 `product-domains` `Command::new` 的 `allowPaths` 校验。
    - 移除 `manifestContractChecks` 中 12 个 miniapp 文件的契约锚点。

### E. 就近文档同步
11. **`src/crates/contracts/product-domains/AGENTS.md`** 与 **`AGENTS-CN.md`**：
    - 摘除 `miniapp` 特征与归属职责段落描述，中英文同步。

---

## 4. 编译错误分层处理
- 遇到的编译错误：0 个。
- 原因说明：M1-M3 已断开上层引用，所有删除与变更一次性自洽通过。

---

## 5. 验证命令与输出原文

### 验证 1: workspace 整体编译检查
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
输出：
```text
warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
note: the name `prompt_cache` in the type namespace is supposed to be publicly re-exported here
  --> src\crates\assembly\core\src\agentic\session\mod.rs:34:9
   |
34 | pub use facade::*;
   |         ^^^^^^^^^
note: but the private item here shadows it
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: `#[warn(hidden_glob_reexports)]` on by default

warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
warning: `northhing` (bin "northhing") generated 5 warnings
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.80s
```

### 验证 2: product-domains 默认/无 feature 测试
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-product-domains --no-default-features
```
输出：
```text
   Compiling northhing-product-domains v0.2.10 (E:\agent-project\northing\src\crates\contracts\product-domains)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.04s
     Running unittests src\lib.rs (target\debug\deps\northhing_product_domains-4f8d57850b79b600.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests\function_agent_contracts.rs (target\debug\deps\function_agent_contracts-018591c5707251b6.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests northhing_product_domains

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 验证 3: product-domains function-agents 特征测试
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-product-domains --features function-agents
```
输出：
```text
   Compiling syn v2.0.118
   Compiling serde_derive v1.0.228
   Compiling tracing-attributes v0.1.31
   Compiling tracing v0.1.44
   Compiling serde v1.0.228
   Compiling northhing-product-domains v0.2.10 (E:\agent-project\northing\src\crates\contracts\product-domains)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.68s
     Running unittests src\lib.rs (target\debug\deps\northhing_product_domains-2fcedcf92ac78ab1.exe)

running 8 tests
test function_agents::common::tests::rejects_missing_or_invalid_json ... ok
test function_agents::common::tests::extracts_json_from_common_ai_response_wrappers ... ok
test function_agents::common::tests::repairs_unescaped_quotes_inside_json_strings ... ok
test function_agents::git_func_agent::utils::tests::commit_ai_response_policy_extracts_json_and_maps_domain_errors ... ok
test function_agents::startchat_func_agent::utils::tests::work_state_ai_prompt_uses_product_domain_template ... ok
test function_agents::startchat_func_agent::utils::tests::work_state_ai_response_policy_extracts_json_and_maps_domain_errors ... ok
test function_agents::git_func_agent::utils::tests::commit_ai_prompt_uses_product_domain_template_and_truncation_policy ... ok
test function_agents::git_func_agent::context_analyzer::tests::detects_rust_library_context ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\function_agent_contracts.rs (target\debug\deps\function_agent_contracts-b44f72de814c6f3a.exe)

running 18 tests
test git_commit_options_preserve_existing_defaults ... ok
test function_agent_runtime_facade_preserves_empty_staging_error ... ok
test function_agent_ports_keep_ai_and_git_boundaries_explicit ... ok
test function_agent_runtime_facade_honors_disabled_git_state_boundary_and_preserves_time_info ... ok
test function_agent_runtime_facade_generates_commit_message_from_ports ... ok
test startchat_git_status_helpers_preserve_porcelain_contract ... ok
test git_function_agent_utils_preserve_change_classification ... ok
test git_function_agent_commit_prompt_preparation_preserves_truncation_boundary ... ok
test function_agent_runtime_facade_builds_work_state_from_ports_without_surface_logic ... ok
test function_agent_json_helpers_parse_ai_payloads_without_core_runtime ... ok
test git_function_agent_prompt_helpers_preserve_ai_contract ... ok
test git_function_agent_summary_helpers_preserve_commit_shape ... ok
test git_function_agent_diff_truncation_preserves_legacy_marker ... ok
test startchat_options_preserve_existing_defaults ... ok
test startchat_complete_analysis_parser_preserves_defaults_and_limits ... ok
test startchat_prompt_helpers_preserve_ai_contract ... ok
test git_function_agent_analysis_parser_preserves_defaults_and_required_title ... ok
test startchat_action_helpers_preserve_limits_and_defaults ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests northhing_product_domains

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### 验证 4: core-boundaries 检查（含 self-test）
```powershell
node scripts/check-core-boundaries.mjs
```
输出：
```text
Core boundary check passed.
```

### 验证 5: i18n-audit.mjs 语法检查（修改前后比对）
修改前：
```powershell
node --check scripts/i18n-audit.mjs
```
输出：
```text
E:\agent-project\northing\scripts\i18n-audit.mjs:481
  'è¿?,
  ^^^^^

SyntaxError: Invalid or unexpected token
    at checkSyntax (node:internal/main/check_syntax:72:5)

Node.js v24.19.0
```

修改后：
```powershell
node --check scripts/i18n-audit.mjs
```
输出：
```text
E:\agent-project\northing\scripts\i18n-audit.mjs:481
  'è¿?,
  ^^^^^

SyntaxError: Invalid or unexpected token
    at checkSyntax (node:internal/main/check_syntax:72:5)

Node.js v24.19.0
```
结果：前后严格报同一个 SyntaxError（第 481 行），证明修改未扩大或引入其它语法损伤。

### 验证 6: 收束与残留自检
1. `git grep -l -i "miniapp" src/crates/contracts/`
   输出：
   ```text
   src/crates/contracts/core-types/src/surface.rs
   ```
   （说明：`surface.rs` 中为 `RuntimeArtifactKind::MiniApp` serde 死变体，按 brief/recon 明确归属 M5 处理，本批保持不碰）。

2. `git grep -i miniapp scripts/core-boundaries | Measure-Object -Line`
   - 修改前行数：222
   - 修改后行数：0

### 验证 7: desktop 编译检查（MSVC）
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
输出：
```text
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
warning: `northhing` (bin "northhing") generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 53.32s
```

### 验证 8: git status --short
```powershell
git status --short
```
输出：
```text
 M .opencode/model-capability-notes.md
 M Cargo.lock
 M memory/northhing.md
 M scripts/core-boundaries/rules/feature-rules.mjs
 M scripts/core-boundaries/rules/source/forbidden-rules.mjs
 M scripts/core-boundaries/rules/source/required-rules.mjs
 M scripts/core-boundaries/self-test.mjs
 M scripts/i18n-audit.mjs
 M src/crates/contracts/product-domains/AGENTS-CN.md
 M src/crates/contracts/product-domains/AGENTS.md
 M src/crates/contracts/product-domains/Cargo.toml
 M src/crates/contracts/product-domains/src/lib.rs
 D src/crates/contracts/product-domains/src/miniapp/bridge_builder.rs
 D src/crates/contracts/product-domains/src/miniapp/builtin.rs
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/coding-selfie/index.html
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/coding-selfie/meta.json
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/coding-selfie/style.css
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/coding-selfie/ui.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/coding-selfie/worker.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/divination/index.html
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/divination/meta.json
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/divination/style.css
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/divination/ui.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/divination/worker.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/gomoku/index.html
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/gomoku/meta.json
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/gomoku/style.css
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/gomoku/ui.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/gomoku/worker.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/.gitignore
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/build-northhing.mjs
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/bundle.json
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/dist/ui.bundle.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/esm_dependencies.json
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/index.html
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/meta.json
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/source_manifest.json
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/deck-ai.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/export-bundle-entry.mjs
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/export-deck-browser.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/export-deck-host.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/export-format-icons.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/export-html.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/export-slide-browser.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/flat-select.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/html2pptx-dom-core.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/i18n.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/northhing-backend-adapter.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/pptx-element-export.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/pptx-html-build.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/render.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/sanitize-slide-html.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/state.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/style-presets.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/src/vendor/ppt-export.bundle.mjs
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/style.css
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/ui.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/ppt-live/worker.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/pr-review/index.html
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/pr-review/meta.json
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/pr-review/style.css
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/pr-review/ui.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/pr-review/worker.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/regex-playground/index.html
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/regex-playground/meta.json
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/regex-playground/style.css
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/regex-playground/ui.js
 D src/crates/contracts/product-domains/src/miniapp/builtin/assets/regex-playground/worker.js
 D src/crates/contracts/product-domains/src/miniapp/compiler.rs
 D src/crates/contracts/product-domains/src/miniapp/customization.rs
 D src/crates/contracts/product-domains/src/miniapp/draft.rs
 D src/crates/contracts/product-domains/src/miniapp/exporter.rs
 D src/crates/contracts/product-domains/src/miniapp/host_routing.rs
 D src/crates/contracts/product-domains/src/miniapp/lifecycle.rs
 D src/crates/contracts/product-domains/src/miniapp/mod.rs
 D src/crates/contracts/product-domains/src/miniapp/permission_policy.rs
 D src/crates/contracts/product-domains/src/miniapp/ports.rs
 D src/crates/contracts/product-domains/src/miniapp/runtime.rs
 D src/crates/contracts/product-domains/src/miniapp/runtime_facade.rs
 D src/crates/contracts/product-domains/src/miniapp/storage.rs
 D src/crates/contracts/product-domains/src/miniapp/types.rs
 D src/crates/contracts/product-domains/src/miniapp/worker.rs
 D src/crates/contracts/product-domains/tests/builtin_and_ports.rs
 D src/crates/contracts/product-domains/tests/common/mod.rs
 D src/crates/contracts/product-domains/tests/compiler_export_storage_and_runtime.rs
 D src/crates/contracts/product-domains/tests/host_routing_and_lifecycle_helpers.rs
 D src/crates/contracts/product-domains/tests/permissions_and_bridge.rs
 D src/crates/contracts/product-domains/tests/runtime_facade_and_customization.rs
?? .handoffs/handoff-g2-t9-2026-08-07.md
?? .superpowers/sdd/task-t2-2n-brief.md
?? .superpowers/sdd/task-t2-2n-report.md
```

---

## 6. 偏离与遗留说明
- 无任何计划外偏离。
- `Cargo.lock` 由 cargo 自动收敛。
- 未 commit，等待编排者与 reviewer 审查。
