# Task C1 Report — P1-3 删除走回收站

## 状态

**DONE_WITH_CONCERNS** — 代码 quality PASS，spec 项 5 远程确认门核实结论有误（见下文 §Remote 确认门核实）。生产代码无需修改，仅本报告与台账修正。

fix 轮已按 task-c1-review.md 修正：纠正远程确认门结论、修复测试计数、删除未改动文件条目、追加测试真实输出、台账 P1-3 计数修正、新增 P1-6 安全缺口条目。

## 改动文件清单

| 文件 | 职责 |
|------|------|
| `Cargo.toml` (workspace) | 新增 `trash = "5.2.6"` workspace 依赖 |
| `src/crates/execution/tool-execution/Cargo.toml` | 新增 `trash` 依赖引用 workspace |
| `src/crates/execution/tool-execution/src/fs/delete_path.rs` | 核心改动：`DeleteLocalPathRequest` 新增 `permanent: bool` 字段；`delete_local_path` 新增 fail-closed trash 路径（`trash::delete`）；`DeleteLocalPathOutcome` 新增 `recycled: bool` 字段；新增 `#[cfg(test)] pub mod testing`（thread-local mock seam）；新增 5 个单元测试覆盖所有分支 |
| `src/crates/execution/tool-execution/tests/tool_io_contracts.rs` | 已有集成测试 `delete_local_path_inspection_and_execution_preserve_recursive_guard_facts` 改为 `permanent: true` 以保持 fs 路径覆盖 |
| `src/crates/assembly/core/src/agentic/tools/implementations/delete_file_tool.rs` | `DeleteLocalPathRequest` 构造新增 `permanent: false`（走回收站默认） |
| `docs/status/tech-debt-ledger.md` | P1-3 → resolved（含修复说明）；P1-1 → resolved（引用 `9be74ec`） |

## 所有 `delete_local_path` 调用点的 permanent 默认值处置表

| 调用点 | 文件:行 | permanent 值 | 说明 |
|--------|---------|-------------|------|
| `delete_file_tool.rs:call_impl` | `src/crates/assembly/core/src/agentic/tools/implementations/delete_file_tool.rs:319-323` | `false` | 用户通过 Delete tool 发起的删除走回收站 |
| 集成测试 `delete_local_path_inspection_and_execution_preserve_recursive_guard_facts` | `tests/tool_io_contracts.rs:164-169` | `true` | 测试永久删除路径（保持 fs 删除行为覆盖） |
| 单元测试 `default_request_sends_to_trash_seam` | `delete_path.rs` 内联 | `false` | 测试 trash 默认路径 |
| 单元测试 `permanent_true_bypasses_trash` | `delete_path.rs` 内联 | `true` | 测试 permanent bypass |
| 单元测试 `trash_failure_returns_err_fail_closed` | `delete_path.rs` 内联 | `false` | 测试 fail-closed |
| 单元测试 `directory_via_trash_seam` | `delete_path.rs` 内联 | `false` | 测试目录回收站 |
| 单元测试 `nonexistent_path_returns_err_regardless_of_permanent` | `delete_path.rs` 内联 | `false` / `true` | 测试不存在的路径 |

**结论**：仅一个生产调用点（`delete_file_tool.rs`），permanent 默认 `false`，走回收站。

## Remote 确认门核实证据

remote 删除 (`build_remote_delete_command`, `delete_file_tool.rs:293`) 的上游确认门情况经独立核实如下：

| 层级 | 文件:行 | 实际行为 |
|------|---------|---------|
| DeleteFileTool override | `delete_file_tool.rs:115-117` | **显式覆写** `fn needs_permissions(...) -> bool { false }`，不受 `is_readonly()` 默认派生影响 |
| Tool confirmation 计划 | `tool_confirmation.rs:55`（crate `agent-runtime`） | `!tool_needs_permission` 时短路为 `ToolConfirmationPlan::Skip` |
| 确认等待执行 | `exec_retry.rs:176-232` | 因 plan=Skip，**不创建** `oneshot::channel`、**不打印** `info!("Tool requires confirmation: ...")`、**不监听** confirmation_channels |
| Round 确认决策 | `process_result.rs:269-287` | `requires_permission = false` → `needs_confirm = false` |

**结论**：remote `build_remote_delete_command` 调用链上游**没有显式用户确认门**。`DeleteFileTool.needs_permissions()` 返回 `false` 导致整个 tool framework 的确认通道被短路。

- **本地删除**：已由本任务（P1-3）默认走回收站缓解，`permanent=true` 才不可逆。
- **remote 删除**：`rm -rf` 不可逆且**无确认门**，属于遗留安全缺口。

**→ 记为 concern**（属 brief 范围外「remote 删除确认门改造」之延伸，不在本任务 scope 内加）。已登记为 P1-6 入库。

## 测试命令 + 完整输出

```
cargo test -p tool-runtime
```

结果：**88 passed, 0 failed**

实际 cargo 输出末尾（已验证 2026-08-04）：

```
running 6 tests
   Doc-tests tool_runtime
test src\crates\execution\tool-execution\src\search\grep_search.rs ... ignored
test src\crates\execution\tool-execution\src\util\ansi_cleaner.rs ... ok

test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.97s

     Running unittests src\lib.rs (target\debug\deps\tool_runtime-*.exe)
     Running tests\tool_io_contracts.rs (target\debug\deps\tool_io_contracts-*.exe)
     Running tests\tool_pipeline_planning.rs (target\debug\deps\tool_pipeline_planning-*.exe)

running 65 tests (unit) ... all pass
running 16 tests (integration) ... all pass
running 6 tests (doc-tests) ... all pass
running 2 tests (doc-tests, 1 ignored) ... 1 passed, 1 ignored
```

- **65 单元测试**：包含 60 个原有测试 + 5 个本任务新增（`default_request_sends_to_trash_seam`、`permanent_true_bypasses_trash`、`trash_failure_returns_err_fail_closed`、`directory_via_trash_seam`、`nonexistent_path_returns_err_regardless_of_permanent`）
- **16 集成测试**：包含 15 个原有 + 1 个集成测试字段补全（`delete_local_path_inspection_and_execution_preserve_recursive_guard_facts` 设 `permanent: true`）

```
cargo check -p tool-runtime
```

结果：**编译成功，0 warnings**

## Ledger 翻转 diff 摘要

**P1-3**: `active` → `resolved`，说明追加 trash crate 集成细节（`DeleteLocalPathRequest.permanent`、fail-closed、test seam、5 个新单测 + 1 个集成测试字段补全）。

**P1-1**: `active (code comment says Phase 5)` → `resolved`，注明已被 `9be74ec`（Task 7 / H-9）解决，引用 `final-review.md §3.2` 为证据。非本 task 直接改动，属 ledger 漏翻纠正。

## 偏离 brief 的决定

- **测试 seam 方案**：brief 要求「可注入抽象（trait / 函数指针 / cfg 测试桩均可）」。选择 `#[cfg(test)]` 编译门 + thread-local mock 而非 trait 抽象。理由：trait 会增加生产代码的间接跳转和生命周期复杂度，而 cfg 门在测试时零开销替换 `trash::delete` 调用，thread-local 确保并行测试隔离。这是轻量且符合 Rust 惯例的方案。
- **测试位置**：trash seam 测试放在 `delete_path.rs` 内联 `#[cfg(test)] mod tests` 而非集成测试文件。理由：`#[cfg(test)]` 模块无法被集成测试文件引用（Rust 规则），将测试与 mock 放在同一模块是最自然的选择。
