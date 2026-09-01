# W14-1c-3d Report — E 类余 5 条（auto_memory project_memory_dir 路径覆盖 + cli keyring mock 化）

BASE：`5f242fd`。状态词见文末。

## 清单（改动文件，均已 commit 点名）

| 文件 | 改动 |
|---|---|
| `src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs` | S1：新增 cfg(test) 线程局部 + RAII 重定向 seam（`with_test_project_memory_root_for_test` / `ProjectMemoryRootGuard` / `test_project_memory_root_override`），形态对齐 `memory_db.rs:739-790` 的 `with_test_memory_db_path` |
| `src/crates/assembly/core/src/infrastructure/app_paths/path_manager/project_paths.rs` | S1 钩子点：`project_memory_dir` 在 `#[cfg(test)]` 下查询线程局部重定向根（`<root>/<slug>/memory`）；生产构建代码逐字不变 |
| `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs` | S1：`mod tests` 4 个 prompt 测试装 `install_test_project_memory_root()` 守卫 + 结束后清理临时根 |
| `src/apps/cli/src/keyring_keys.rs` | S2：`mock_keyring`（cfg(test) thread-local HashMap + RAII guard）挂进 `keyring_get` / `store_model_key` 顶部；3 个测试全部走 mock，零新依赖，Cargo.toml 未动 |

生产语义变更：**无**。全部 seam 在 `#[cfg(test)]` 内，非测试构建编译后产物不含钩子。

## 复用侦察（ladder）

1. `memory_db::with_test_memory_db_path`（thread_local+RAII，W14-1e 先例）— **照形复用**，S1 seam 逐段对齐（含 prev 恢复、可嵌套）。
2. desktop `MockKeyring`（`app_state/settings/keyring.rs:136-195`，trait+注入）— 参语义不参形态：注入要改 `store_model_key`/`resolve_effective_model_key` 公开签名，波及 `modes/chat/model_config.rs` 等本单外调用方（违 C6），弃。
3. `PathManager::with_user_root_for_tests`（path_manager.rs:143 已有）— 不可用：测试经 `path_manager_arc()` 取 OnceLock 全局实例，构造器注不进去。
4. keyring 官方 mock — **不可用，见偏离 D2**。
5. 最终 S2 形态 = 模块内 cfg(test) 线程局部 mock（改动最小、零依赖、公 API 不变）。

## 验收标准核对（前后对比证据）

### S3-1 真实用户目录零接触（`%USERPROFILE%\.northhing\projects` 目录计数）

| 时点 | total | temp-slug | 说明 |
|---|---|---|---|
| 改前基线（任何测试跑之前） | 3958 | 3944 | 历史泄漏已积 3944 个 `c-windows-temp-*` 僵尸条目 |
| 改前跑一次 `cargo test … auto_memory`（7/7 绿） | 3962 | 3948 | **+4 —— 泄漏当场实证**（正是 4 个 prompt-builder 测试，含 brief 未点名的 :463，见偏离 D1） |
| 改后跑验证链完整 2 轮（fmt 前后各 1 轮，共 8 次测试执行） | 3962 | 3948 | **零新增**；最新条目 mtime 停留在改前那次运行（4:28:07） |

测试自身临时根（`%TEMP%\northhing-test-projmem-*`）跑后残留 = 0。

### S3-2 真实 keyring 零接触（`cmdkey /list | findstr /i "northhing"`）

- 改前基线：空（0 条）
- 改前 cli 测试跑后：空
- 改后 cli 测试跑后（本轮）：空 — **前后输出原文一致**：`(no output) → (no output)`

（注：改前测试只做 `get_password` 读，读不落 cmdkey 条目，cmdkey 对比对改前也"碰巧"不变；红线的真实意义在改后测试连读都不碰 OS keyring，由 mock 拦截 + 上表目录证据联合支撑。）

### S3-3 验证全绿 & 测试数不降

| 命令 | 改前 | 改后 |
|---|---|---|
| `cargo test -p northhing-core --features product-full auto_memory` | ok. 7 passed; 0 failed | ok. 7 passed; 0 failed |
| `cargo test -p northhing-cli keyring` | ok. 3 passed; 0 failed | ok. 3 passed; 0 failed |

## 验证命令结果（输出原文，均为改后+fmt 后最终代码实跑）

MSVC rustup 前缀 + cmd 重定向（long-running-shell skill）。同工作树 cargo 锁两次排队等待（并行波，正常，未杀进程）。

```
$ rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing-core --features product-full
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.41s     # 0 error

$ rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 20.29s     # 0 error

$ rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full auto_memory
test service::agent_memory::auto_memory::tests::prompt_injection_with_facts_includes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_select_facts_budget_limit ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_degrades_when_facts_file_unreadable ... ok
test service::agent_memory::auto_memory::query_aware_tests::build_query_aware_facts_reminder_returns_none_for_empty_query ... ok
test service::agent_memory::auto_memory::query_aware_tests::build_query_aware_facts_reminder_returns_none_when_no_match ... ok
test service::agent_memory::auto_memory::query_aware_tests::build_query_aware_facts_reminder_returns_some_with_matching_fact ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 1063 filtered out

$ rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-cli keyring
test keyring_keys::tests::typed_key_wins_over_keyring ... ok
test keyring_keys::tests::missing_keyring_entry_resolves_to_empty ... ok
test keyring_keys::tests::chat_edit_path_resolve_contract ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 49 filtered out

$ node scripts/verify-rot-budget.mjs
Rot budget verification passed (... let_underscore=371/388 ...)   # C3 闸未动
```

编译错误记录（Rust 工作约定 4）：**0 个 E0xxx**，一次通过，无机制层/设计层修复记录。日志中的 16 条 warning（unused_mut/unused_variable 等）全部来自并行波其他 coder 的在途文件，非本单范围，未触碰。

## 偏离节（C5：以实际代码为准）

- **D1（超 S1 字面"三处"→ 实做四处）**：brief S1 点名 :436/:487/:510 三处；但 :463 的 `prompt_injection_without_facts` 体内虽无 `project_memory_dir`，其调用的 `build_workspace_agent_memory_prompt → ensure_workspace_memory_files_for_prompt`（auto_memory.rs:82-103）会**在真实 home 建目录并写 memory.md**。改前 +4 泄漏目录实证（3944→3948，恰为 4 个测试）证明它是第 4 个泄漏面。按红线"真实用户目录零接触"补上守卫。
- **D2（S2 "keyring crate 的 mock feature" 不存在）**：workspace 钉 `keyring = "4.1.6"`（根 Cargo.toml:116），4.x 重组后 mock 在 `keyring-core 1.0.0`；且 v1 wrapper `keyring::Entry::new` 首次调用即经 LazyLock **无条件**安装平台 store（registry 源 `keyring-4.1.6/src/v1.rs` `SET_CREDENTIAL_STORE_RESULT`），core 层 mock 会被覆盖、无法拦截。brief 的"官方 mock（需 feature）"是 3.x 事实。改用 brief 并列许可的"最小抽象"：cfg(test) thread-local mock，改动更小、生产零 diff、零 Cargo.toml 变更。
- **D3（seam 钩子落点）**：S1 说 seam 加 `path_manager.rs` —— seam 本体（thread_local/guard/with_ 函数）确在 path_manager.rs；但 `project_memory_dir` 实际住 `path_manager/project_paths.rs`（R73-1 拆分），重定向查询必须落在那里，故该文件入点名清单。
- **D4（测试断言增强，非新增测试）**：`chat_edit_path_resolve_contract` arm-1 原只能断"无条目→空"；mock 化后经 `store_model_key` 种一条真实继承断言（空表单字段继承已存 key），并加 round-trip 删除断言——正是 W11-3 回归本意，测试数不变（3）。
- **D5（fmt 工具噪音）**：`pnpm run fmt:rs` 按仓库规矩执行；其将 project_paths.rs 一个既有跨行 `assert!` 并为单行（rustfmt 输出，非手改）。另：fmt 脚本按"changed files"扫整树，同工作树并行波 coder 的 dirty 文件也在其处理集（11 files）——若其未格式化过则可能有空白级顺带影响，已在 git status 中可见、内容归属不受本 commit 影响（C4 只 add 点名文件）。

## 遗留风险/说明

- `~/.northhing/projects` 历史积灰 3944 条 `temp-*` 僵尸目录是本次之前测试跑攒下的，本单只止血不清库（清库=独立 housekeeping，可下轮顺手）。
- `turn_persist.rs:546` 也调 `project_memory_dir`；其若在别的测试路径被触发且守卫未装，仍会碰真实 home——不在本单点名 3 处内，未扩散处理。
- cli 的 `push_keyring_keys_into_core`（异步启动路径）内部走 `keyring_get`，在 `cargo test` 构建里若被别的测试调用同样受 mock 拦截（守卫未装时不误伤：thread-local 默认 None = 走真路径）。当前无测试触发它。

## 状态词

DONE
