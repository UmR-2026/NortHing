# W14-1c-3d Brief — E 类余 5 条（auto_memory 路径覆盖 ×3 + cli keyring mock ×2）

> 来源：`w14-1b-arbitration.md` §3 附带 + 步骤 9/10。BASE：`5f242fd`。
> 前置：W14-1e 已给 auto_memory 全部测试补了 `with_test_memory_db_path` 守卫；本单补的是**另一个**泄漏面（project_memory_dir 真实 home）。

## 预检结论（已磁盘核实）

- `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs`：测试 `:430/:482/:505` 区域体内有 `crate::infrastructure::path_manager_arc().project_memory_dir(&workspace)`（:436/:487/:510）——走**真实用户目录**。已有守卫模式参照同文件 `with_test_memory_db_path`（memory_db.rs:762/769 的 RAII 重定向）。
- `src/apps/cli/src/keyring_keys.rs`（120 行）：测试在 :96 `#[cfg(test)] mod tests`（:100/:107/:115 三个）；实现直接用 `keyring::Entry`（:17/:30），测试会读写**真实 OS keyring**。keyring crate 有官方 mock（`keyring::mock`，需 feature）；desktop 侧已有 `MockKeyring` 先例（`src/apps/desktop/src/app_state/settings/`，可参不可抄——不同 crate）。

## Spec

- S1：`infrastructure/app_paths/path_manager.rs` 加 RAII 测试重定向 seam `with_test_project_memory_root_for_test(...)`（形态对齐 `with_test_memory_db_path`；同 crate module 测试用 `#[cfg(test)] pub`），并让 auto_memory.rs 的 :436/:487/:510 三处测试用它把 project_memory_dir 重定向到临时目录。
- S2：cli `keyring_keys.rs` 测试改走 mock keyring（keyring crate 的 mock feature 或最小抽象，选改动最小的；crate Cargo.toml 加 dev-dependency/feature 允许）。验收：跑测试前后 `cmdkey /list | findstr /i "northhing"` 输出不变。
- S3：验证全绿；测试数不降；真实用户目录与真实 keyring 零接触（用 evidence 证明：测试前后 `%APPDATA%\northhing` 相关目录 mtime 截图式记录 or cmdkey 对比）。

## Constraints

C1 可见性按仲裁补遗规则（同 crate cfg(test) pub；跨 crate/tests = pub + doc(hidden)）。C2 不动 FACADE/global_scheduler/六层；不改生产代码路径语义（seam 只在 test 构建生效）。C3 `let _ =` 闸 371/388。C4 git 只点名 add。C5 以实际代码为准，偏离记 report。C6 **并行波**：别动非本单文件；编译错来自别人（registry / deep_review / remote_ssh / ci.yml）就等；禁杀进程。

## 验证

MSVC rustup 前缀 + cmd 重定向。
1. `cargo check -p northhing-core --features product-full` + `cargo check -p northhing-cli`（crate 名先查 Cargo.toml）（0 error）
2. `cargo test -p northhing-core --features product-full auto_memory`（全绿）
3. `cargo test -p northhing-cli keyring`（全绿）
4. `cmdkey /list | findstr /i "northhing"` 前后对比原文

## 报告

`.superpowers/sdd/w14-1c-3d-report.md`：清单 / 输出原文 / 复用侦察 / 偏离节 / 状态词。完成后自行 commit（message 含 W14-1c-3d）。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
