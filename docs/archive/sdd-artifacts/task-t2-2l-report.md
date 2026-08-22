# Task T2-2l Report — MiniApp 整删 M2：assembly/core miniapp 整删

## 最终状态
**DONE**

---

## 变更文件与前后摘要

### A. core miniapp 目录整删与 lib.rs 门控摘除
1. **整删目录** `src/crates/assembly/core/src/miniapp/`（14 文件共 2,349 行全部删除）：
   - `src/crates/assembly/core/src/miniapp/mod.rs` (32 lines)
   - `src/crates/assembly/core/src/miniapp/compiler.rs` (37 lines)
   - `src/crates/assembly/core/src/miniapp/exporter.rs` (39 lines)
   - `src/crates/assembly/core/src/miniapp/host_dispatch.rs` (45 lines)
   - `src/crates/assembly/core/src/miniapp/js_worker.rs` (5 lines)
   - `src/crates/assembly/core/src/miniapp/js_worker_pool.rs` (305 lines)
   - `src/crates/assembly/core/src/miniapp/runtime_detect.rs` (3 lines)
   - `src/crates/assembly/core/src/miniapp/storage.rs` (353 lines)
   - `src/crates/assembly/core/src/miniapp/builtin/mod.rs` (638 lines)
   - `src/crates/assembly/core/src/miniapp/manager/mod.rs` (514 lines)
   - `src/crates/assembly/core/src/miniapp/manager/mgr_types.rs` (61 lines)
   - `src/crates/assembly/core/src/miniapp/manager/mgr_registry.rs` (40 lines)
   - `src/crates/assembly/core/src/miniapp/manager/mgr_runtime.rs` (126 lines)
   - `src/crates/assembly/core/src/miniapp/manager/mgr_lifecycle.rs` (401 lines)

2. `src/crates/assembly/core/src/lib.rs:17-18`:
   - 删除：`#[cfg(feature = "product-domains")] pub mod miniapp;`
   - 保留：`#[cfg(feature = "product-domains")] pub mod function_agents;` 存活门控。

### B. core Cargo.toml feature 链抽条
3. `src/crates/assembly/core/Cargo.toml` (`product-domains` feature 块):
   - 删除整行：`"northhing-services-integrations/miniapp-runtime"`
   - 修改：`"northhing-product-domains/product-full"` -> `"northhing-product-domains/function-agents"`
   - 块内其余行（`ai-adapter-runtime`、`dep:northhing-product-domains`、`northhing-services-integrations/function-agents`）完整保留。

### C. core 内残余耦合点清理
4. `src/crates/assembly/core/src/product_domain_runtime.rs`:
   - 删 use 导入：`use northhing_product_domains::miniapp::ports::{MiniAppRuntimeFacade, MiniAppStoragePort};`
   - 删方法：`pub(crate) fn miniapp_runtime_facade(storage: &dyn MiniAppStoragePort) -> MiniAppRuntimeFacade<'_>`
   - 更新模块 doc 注释（移除 MiniApp 提及），其余 function_agents 三方法完整保留。
5. `src/crates/assembly/core/src/infrastructure/app_paths/`:
   - `user_paths.rs:99-106`: 删除 `miniapps_dir()` 与 `miniapp_dir(app_id)` 两方法。
   - `init.rs:35`: 删除 `self.miniapps_dir(),` 启动建目录副作用行。
   - `path_manager.rs:9`: 文档注释中清理 `miniapps` 提及。

### D. boundary 规则同步
6. `scripts/core-boundaries/rules/source/required-rules.mjs`:
   - 移除 core Cargo.toml `product-domains` 中 `miniapp-runtime` 依赖规则与 lib.rs `pub mod miniapp` 规则；更新 `northhing-product-domains/function-agents` 规则。
   - 移除 `assembly/core/src/miniapp/{storage.rs, builtin/mod.rs, host_dispatch.rs, exporter.rs, manager/mgr_runtime.rs, manager/mgr_lifecycle.rs, manager/mod.rs, runtime_detect.rs, js_worker_pool.rs, js_worker.rs}` 规则段落。
   - 移除 `src/crates/assembly/core/src/product_domain_runtime.rs` 中 `miniapp_runtime_facade`、`MiniAppRuntimeFacade`、`MiniAppStoragePort` 规则。
7. `scripts/core-boundaries/rules/source/forbidden-rules.mjs`:
   - 移除 `src/crates/assembly/core/src/miniapp/{host_dispatch.rs, js_worker.rs, js_worker_pool.rs, storage.rs, runtime_detect.rs, manager}` 规则段落。
8. `scripts/core-boundaries/self-test.mjs`:
   - 同步更新 core Cargo.toml、core lib.rs、`product_domain_runtime.rs` 及 requiredContentContracts 列表中的 core miniapp 锚点。
   - 保留 services-integrations 与 product-domains 层的全部 miniapp 规则锚点。

---

## 编译错误分层分析
- **本批遇到的编译错误**：无（0 E0xxx 编译错误）。

---

## 验证证据（命令与输出原文）

### 1. Workspace 编译检查 (`cargo check --workspace`)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
输出原文：
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
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
warning: `northhing` (bin "northhing") generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 51s
```

### 2. Desktop 门禁检查 (`cargo check -p northhing`)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
输出原文：
```text
    Checking northhing-product-domains v0.2.10 (E:\agent-project\northing\src\crates\contracts\product-domains)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: private item shadows public glob re-export
  --> src\crates\assembly\core\src\agentic\session\mod.rs:13:1
   |
13 | pub(crate) mod prompt_cache;
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
warning: `northhing` (bin "northhing") generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 30s
```

### 3. 架构边界与 Self-Test 检查 (`check-core-boundaries.mjs`)
```powershell
node scripts/check-core-boundaries.mjs
```
输出原文：
```text
Core boundary check passed.
```

### 4. 功能等价抽验 (`cargo test -p northhing-core --lib --features product-full function_agents`)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --lib --features product-full function_agents
```
输出原文：
```text
   Compiling northhing-product-domains v0.2.10 (E:\agent-project\northing\src\crates\contracts\product-domains)
   Compiling northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
   Compiling northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: `northhing-core` (lib test) generated 19 warnings (run `cargo fix --lib -p northhing-core --tests` to apply 18 suggestions)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 46s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-08e260be7d818dec.exe)

running 8 tests
test function_agents::port_adapters::tests::core_product_domain_runtime_owner_constructs_function_agent_git_adapter ... ok
test function_agents::runtime_services::tests::parse_commit_response_preserves_product_domain_response_policy ... ok
test function_agents::runtime_services::tests::parse_complete_analysis_preserves_product_domain_response_policy ... ok
test function_agents::port_adapters::tests::git_adapter_builds_commit_snapshot_from_existing_core_git_services ... ok
test function_agents::port_adapters::tests::git_adapter_startchat_snapshot_matches_legacy_empty_state_when_not_git_repo ... ok
test function_agents::port_adapters::tests::git_adapter_commit_snapshot_keeps_staged_diff_and_unstaged_count_separate ... ok
test function_agents::port_adapters::tests::git_adapter_startchat_snapshot_preserves_git_state_when_diff_has_no_head ... ok
test function_agents::port_adapters::tests::git_adapter_builds_startchat_snapshot_without_changing_git_semantics ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1004 filtered out; finished in 1.44s
```

佐证抽验：`cargo check -p northhing-core --features product-full`：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-core --features product-full
```
输出原文：
```text
    Checking northhing-runtime-ports v0.2.10 (E:\agent-project\northing\src\crates\contracts\runtime-ports)
    Checking northhing-events v0.2.10 (E:\agent-project\northing\src\crates\contracts\events)
    Checking northhing-product-domains v0.2.10 (E:\agent-project\northing\src\crates\contracts\product-domains)
    Checking northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Checking terminal-core v0.2.10 (E:\agent-project\northing\src\crates\services\terminal)
    Checking northhing-agent-stream v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-stream)
    Checking northhing-runtime-services v0.2.10 (E:\agent-project\northing\src\crates\execution\runtime-services)
    Checking northhing-product-capabilities v0.2.10 (E:\agent-project\northing\src\crates\assembly\product-capabilities)
    Checking northhing-agent-tools v0.2.10 (E:\agent-project\northing\src\crates\execution\tool-contracts)
    Checking northhing-agent-dispatch v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-dispatch)
    Checking northhing-agent-runtime v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-runtime)
    Checking northhing-ai-adapters v0.2.10 (E:\agent-project\northing\src\crates\adapters\ai-adapters)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 51s
```

### 5. 收束自检
- `rg -l -i "miniapp" src/crates/assembly/core/`：**0 命中**（输出为空，完全归零）。
- `rg -i "miniapp" scripts/core-boundaries/` 命中计数：
  - 改前：**474**
  - 改后：**293**（减少 181 处 core 层规则，保留 services-integrations 与 product-domains 层全部锚点）

### 6. Git 状态核对 (`git status --short`)
```text
 M .opencode/model-capability-notes.md
 M memory/northhing.md
 M scripts/core-boundaries/rules/source/forbidden-rules.mjs
 M scripts/core-boundaries/rules/source/required-rules.mjs
 M scripts/core-boundaries/self-test.mjs
 M src/crates/assembly/core/Cargo.toml
 M src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs
 M src/crates/assembly/core/src/infrastructure/app_paths/path_manager/init.rs
 M src/crates/assembly/core/src/infrastructure/app_paths/path_manager/user_paths.rs
 M src/crates/assembly/core/src/lib.rs
 D src/crates/assembly/core/src/miniapp/builtin/mod.rs
 D src/crates/assembly/core/src/miniapp/compiler.rs
 D src/crates/assembly/core/src/miniapp/exporter.rs
 D src/crates/assembly/core/src/miniapp/host_dispatch.rs
 D src/crates/assembly/core/src/miniapp/js_worker.rs
 D src/crates/assembly/core/src/miniapp/js_worker_pool.rs
 D src/crates/assembly/core/src/miniapp/manager/mgr_lifecycle.rs
 D src/crates/assembly/core/src/miniapp/manager/mgr_registry.rs
 D src/crates/assembly/core/src/miniapp/manager/mgr_runtime.rs
 D src/crates/assembly/core/src/miniapp/manager/mgr_types.rs
 D src/crates/assembly/core/src/miniapp/manager/mod.rs
 D src/crates/assembly/core/src/miniapp/mod.rs
 D src/crates/assembly/core/src/miniapp/runtime_detect.rs
 D src/crates/assembly/core/src/miniapp/storage.rs
 M src/crates/assembly/core/src/product_domain_runtime.rs
?? .handoffs/handoff-g2-t9-2026-08-07.md
?? .superpowers/sdd/task-t2-2l-brief.md
```
（工作区除并行 session 预存文件外，仅包含本批清单变更）

---

## 偏离与遗留说明
- 无任何计划外偏离。未做 git commit。
