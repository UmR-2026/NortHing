# Task Report — W12-1: 会话全文搜索后端（contracts + assembly/core）

## 1. 状态词
**DONE**

## 2. Commit SHA
`ca38f88cf64976b407848045ed970527df5a94e6`

## 3. Git Show Stat
```text
commit ca38f88cf64976b407848045ed970527df5a94e6
Author: Mavis <mavis@northhing.local>
Date:   Mon Aug 31 22:19:59 2026 +0800

    feat(core): implement session fulltext search backend (W12-1)

 .../assembly/core/src/kernel_facade/session.rs     | 127 ++++++++-
 .../assembly/core/src/kernel_facade/tests.rs       | 300 +++++++++++++++++++++
 src/crates/contracts/kernel-api/src/lib.rs         |   4 +-
 src/crates/contracts/kernel-api/src/session.rs     |  18 ++
 4 files changed, 446 insertions(+), 3 deletions(-)
```

## 4. 三条验证命令输出尾部

### 命令 1: `cargo check --workspace`
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check --workspace
```
输出尾部：
```text
warning: `northhing` (bin "northhing") generated 59 warnings (2 duplicates) (run `cargo fix --bin "northhing" -p northhing` to apply 9 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 26.01s
```
(0 errors)

### 命令 2: `cargo test -p northhing-core --features product-full session`
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full session
```
输出尾部：
```text
test agentic::persistence::session_branch::tests::branch_session_copies_turns_snapshots_and_lineage_metadata ... ok
test agentic::persistence::metadata_subhandlers::tests::list_session_metadata_page_returns_visible_top_level_page_with_children ... ok

test result: ok. 158 passed; 0 failed; 1 ignored; 0 measured; 913 filtered out; finished in 0.46s

     Running tests\context_profile.rs (target\debug\deps\context_profile-6c25f13a8520e02e.exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.00s
     Running tests\git_contracts.rs (target\debug\deps\git_contracts-842e439f5fda151a.exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
     Running tests\product_assembly.rs (target\debug\deps\product_assembly-6ba7f867e85e9989.exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
     Running tests\remote_mcp_streamable_http.rs (target\debug\deps\remote_mcp_streamable_http-a53af564d41b5386.exe)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
```

### 命令 3: `node scripts/verify-rot-budget.mjs`
```powershell
node scripts/verify-rot-budget.mjs
```
输出尾部：
```text
Rot budget verification passed (5 grep rules [unwrap_production=477/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=358/400], 6 god-file rules checked across 1364 files).
```

## 5. 复用侦察
- **查阅符号**：`rebuild_messages_from_turns`、`persistence_manager.list_sessions`、`default_workspace_path`、`summary_to_dto`、`coordinator().get_messages`、`system_time_to_ms_i64`。
- **复用情况**：
  1. `coordinator().session_manager().persistence_manager.list_sessions(Path::new(&workspace_path))`：复用既有持久化会话元数据列表扫描逻辑。
  2. `coordinator().get_messages(&summary.session_id)`：复用既有消息获取逻辑（底层自动处理持久化与内存转录重建）。
  3. `crate::kernel_facade::helpers::default_workspace_path()`：复用既有默认工作区解析 helper。
  4. `crate::kernel_facade::helpers::system_time_to_ms_i64()`：复用既有时间戳映射逻辑。
- **新写等价物**：零新写等价物。

## 6. 偏离清单
无。

## 7. 编译错误修复记录
1. `E0599`: `PathManager::with_user_root_for_tests` 关联函数不存在（机制层：测试代码引用了旧的私有测试辅助方法；修在测试层：统一使用标准构造器 `PathManager::new()`）。
2. `E0063`: `ModelRoundData` 缺少部分 telemetry 结构体字段（机制层：测试 fixture 手工构造结构体未填全字段；修在测试层：编写 `create_test_model_round` 集中构造测试模型交互轮次）。
