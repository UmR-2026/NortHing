# Task T2-2m Report — MiniApp 整删 M3：services-integrations miniapp 整删

## 1. 基本信息
- **任务编号**：T2-2m
- **目标**：services-integrations 层 miniapp 子系统整删（代码、feature 门控、boundary 规则、就近文档）
- **基准提交**：HEAD = `6d6b86c`
- **执行状态**：`DONE`

---

## 2. 变更清单与前后摘要

### A. miniapp 目录与 lib.rs
1. **整删目录** `src/crates/services/services-integrations/src/miniapp/`（11 个文件，共 2,989 行）：
   - `builtin_io.rs`
   - `host_dispatch.rs`
   - `mod.rs`
   - `storage.rs`
   - `storage_app_io.rs`
   - `storage_drafts.rs`
   - `storage_imports_io.rs`
   - `storage_port.rs`
   - `storage_tests.rs`
   - `worker.rs`
   - `worker_pool.rs`
2. **`src/crates/services/services-integrations/src/lib.rs`**（:27-28）：
   - 删除了 `#[cfg(feature = "miniapp-runtime")] pub mod miniapp;` 两行。

### B. Cargo.toml feature 摘除（[dependencies] 未触碰）
3. **`src/crates/services/services-integrations/Cargo.toml`**：
   - 删除了 `miniapp-runtime` feature 块定义（含 `base64`、`northhing-product-domains/miniapp`、`northhing-services-core`、`dep:northhing-product-domains`、`dirs`、`reqwest`、`uuid`、`which`）。
   - `product-full` feature 列表中摘除了 `"miniapp-runtime"` 项。
   - `[dependencies]` 依赖项完全保持不变（无孤立依赖，均由 mcp / remote-ssh-concrete / workspace-search / function-agents 共享）。

### C. boundary 规则同步
4. **`scripts/core-boundaries/rules/feature-rules.mjs`**：
   - 从 `services-integrations` 的 7 个 optional dependency 的 `ownerFeatures` 中移除 `'miniapp-runtime'`：
     - `base64`：`['mcp', 'remote-ssh-concrete']`
     - `northhing-product-domains`：`['function-agents']`
     - `northhing-services-core`：`['git', 'mcp', 'workspace-search', 'remote-ssh-concrete']`
     - `dirs`：`['remote-ssh-concrete']`
     - `reqwest`：`['mcp']`
     - `uuid`：`['remote-ssh-concrete']`
     - `which`：`['workspace-search']`
   - 从 `ownerCrateFeatureAssemblyRules` 中 `services-integrations` 的 `requiredProductFullFeatures` 数组中移除 `'miniapp-runtime'`。
   - 保留 product-domains 规则（:86-88 与 :151 留待 M4）。
5. **`scripts/core-boundaries/rules/source/forbidden-rules.mjs`**：
   - 删除了 `src/crates/services/services-integrations/src/miniapp/host_dispatch.rs` 对应的禁令规则。
6. **`scripts/core-boundaries/rules/source/required-rules.mjs`**：
   - 删除了 4 个针对 `services-integrations/src/miniapp/` 的规则块（`builtin_io.rs`、`host_dispatch.rs`、`worker.rs`、`worker_pool.rs`）。
7. **`scripts/core-boundaries/self-test.mjs`**：
   - 删除了 7 个针对 `services-integrations/src/miniapp/` 的契约测试条目（`storage.rs`、`storage_imports_io.rs`、`storage_tests.rs`、`builtin_io.rs`、`host_dispatch.rs`、`worker.rs`、`worker_pool.rs`）。

### D. 就近文档与注释同步
8. **`src/crates/services/services-integrations/AGENTS.md`**（:34-37）：
   - 删除了 MiniApp runtime IO 职责段落。
9. **`src/crates/services/AGENTS.md`**（:7, :15, :22）与 **`src/crates/services/AGENTS-CN.md`**（:5, :12, :17）：
   - 清理了涉及 MiniApp runtime/import IO 的措辞，中英文版本保持同步。
10. **`src/crates/services/services-integrations/src/announcement/types.rs`**（:180）：
   - 文档注释示例由 `feature_v1_3_0_miniapp` 更新为 `feature_v1_3_0_demo`（顺手清配额，确保 `services-integrations` 代码与注释无 miniapp 残留）。

---

## 3. 验证证据（命令与输出原文）

### 1. Workspace 编译检查
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
输出：
```text
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
...
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 53.38s
```
结果：**PASS**

### 2. Feature 组合抽验
```powershell
# 2.1 默认 feature 检查
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations
```
输出：
```text
    Checking northhing-core-types v0.2.10 (E:\agent-project\northing\src\crates\contracts\core-types)
    Checking northhing-events v0.2.10 (E:\agent-project\northing\src\crates\contracts\events)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.49s
```
结果：**PASS**

```powershell
# 2.2 remote-ssh / remote-ssh-concrete 组合检查
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations --features remote-ssh,remote-ssh-concrete
```
输出：
```text
    Checking northhing-core-types v0.2.10 (E:\agent-project\northing\src\crates\contracts\core-types)
    Checking northhing-events v0.2.10 (E:\agent-project\northing\src\crates\contracts\events)
    Checking northhing-services-core v0.2.10 (E:\agent-project\northing\src\crates\services\services-core)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.15s
```
结果：**PASS**

```powershell
# 2.3 --no-default-features 检查（Cargo.toml 中 default = []）
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations --no-default-features
```
输出：
```text
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.63s
```
结果：**PASS**

### 3. Core Boundaries 检查
```powershell
node scripts/check-core-boundaries.mjs
```
输出：
```text
Core boundary check passed.
```
结果：**PASS**

### 4. 存活测试回归
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-services-integrations --lib
```
输出：
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.78s
     Running unittests src\lib.rs (target\debug\deps\northhing_services_integrations-6a0c6f977a74cf60.exe)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```
全 feature 集测试（`--features product-full`）回归结果：
```text
running 47 tests (lib unit tests) -> ok. 47 passed; 0 failed
running 4 tests (announcement_contracts) -> ok. 4 passed; 0 failed
running 18 tests (config_and_server_lifecycle) -> ok. 18 passed; 0 failed
running 3 tests (context_enhancer_and_catalog) -> ok. 3 passed; 0 failed
running 9 tests (dynamic_tools_and_runtime) -> ok. 9 passed; 0 failed
running 2 tests (file_watch_contracts) -> ok. 2 passed; 0 failed
running 3 tests (function_agent_contracts) -> ok. 3 passed; 0 failed
running 10 tests (git_contracts) -> ok. 10 passed; 0 failed
running 7 tests (remote_ssh_contracts) -> ok. 7 passed; 0 failed
running 4 tests (request_builders_and_adapters) -> ok. 4 passed; 0 failed
running 3 tests (tool_names_and_protocol) -> ok. 3 passed; 0 failed
running 3 tests (workspace_search_contracts) -> ok. 3 passed; 0 failed
总计：110 passed; 0 failed
```
结果：**PASS**

### 5. 收束自检
```powershell
rg -l -i "miniapp" src/crates/services/services-integrations/
```
输出：`0 命中`

```powershell
rg -l -i "miniapp" src/crates/services/
```
输出：
```text
src/crates/services/services-core\AGENTS.md
src/crates/services/services-core\src\session\session_metadata.rs
src/crates/services/services-core\src\session\lineage.rs
```
（注：`services-core` 下的 3 处为 M5 范围，按 brief 明确不触碰）

```powershell
rg -i "miniapp" scripts/core-boundaries | Measure-Object -Line
```
输出：
- 改前：`293`
- 改后：`222`（减少 71 行，仅保留 product-domains 层规则，待 M4 清理）

### 6. Git 工作区状态
```powershell
git status --short
```
输出：
```text
 M .opencode/model-capability-notes.md
 M memory/northhing.md
 M scripts/core-boundaries/rules/feature-rules.mjs
 M scripts/core-boundaries/rules/source/forbidden-rules.mjs
 M scripts/core-boundaries/rules/source/required-rules.mjs
 M scripts/core-boundaries/self-test.mjs
 M src/crates/services/AGENTS-CN.md
 M src/crates/services/AGENTS.md
 M src/crates/services/services-integrations/AGENTS.md
 M src/crates/services/services-integrations/Cargo.toml
 M src/crates/services/services-integrations/src/announcement/types.rs
 M src/crates/services/services-integrations/src/lib.rs
 D src/crates/services/services-integrations/src/miniapp/builtin_io.rs
 D src/crates/services/services-integrations/src/miniapp/host_dispatch.rs
 D src/crates/services/services-integrations/src/miniapp/mod.rs
 D src/crates/services/services-integrations/src/miniapp/storage.rs
 D src/crates/services/services-integrations/src/miniapp/storage_app_io.rs
 D src/crates/services/services-integrations/src/miniapp/storage_drafts.rs
 D src/crates/services/services-integrations/src/miniapp/storage_imports_io.rs
 D src/crates/services/services-integrations/src/miniapp/storage_port.rs
 D src/crates/services/services-integrations/src/miniapp/storage_tests.rs
 D src/crates/services/services-integrations/src/miniapp/worker.rs
 D src/crates/services/services-integrations/src/miniapp/worker_pool.rs
?? .handoffs/handoff-g2-t9-2026-08-07.md
?? .superpowers/sdd/task-t2-2m-brief.md
?? .superpowers/sdd/task-t2-2m-report.md
```

---

## 4. 编译错误分层修复记录
- 本批次编译与测试过程中**未发生编译错误（0 E0xxx errors）**，无须修补。

---

## 5. 偏离与遗留说明
- **偏离说明**：无。严格执行清单 A~D，未触碰 `[dependencies]`、`product-domains` 以及 `services-core` 中的 M5 范围。
- **未提交状态**：按规则未执行 `git commit`，等待编排者与评审者处理。

---

## 6. 最终状态
**`DONE`**
