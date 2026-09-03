# Task W15-1g: 修复 list_workspace_tree 符号链接检查顺序导致的 CI windows 测试红

## 来源与验收标准（逐字）

CI run 33675024521（@34d0553）两个 windows job 同挂一个测试，失败原文（gh 日志逐字）：

```
thread 'kernel_facade::tests::w9_6_file_tree::list_tree_skips_symlink_to_outside_target' (1908) panicked at src\crates\assembly\core\src\kernel_facade\tests.rs:1183:14:
root listing must succeed even when symlinks exist: Validation("entry escaped workspace: \\\\?\\C:\\Users\\runneradmin\\AppData\\Local\\Temp\\northhing-w9-6-workspace-listing-4140-...\\escape_link")
test result: FAILED. 1068 passed; 1 failed; 1 ignored
```

验收标准：
1. 修复后 `cargo test -p northhing-core w9_6` 本地绿。
2. 推分支后 CI windows 两个测试 job 该用例转绿（编排者用 gh 观测，你不用等 CI）。
3. diff 只触及允许文件集。

## 编排者预检结论（直接采信）

**根因已定位**（编排者 + codegraph 实锤，勿重新侦察）：

- `src/crates/assembly/core/src/kernel_facade/platform.rs` `list_workspace_tree` 循环体内顺序错误：
  - :320 先做围栏检查 `is_within(&workspace_root, &p)`，失败即 `return Err(Validation("entry escaped workspace"))`；
  - :328-337 才取 `symlink_metadata` 并 `is_symlink() → continue` 跳过。
- `is_within`（platform.rs:138-147）对 candidate 调 `std::fs::canonicalize`——**canonicalize 跟随符号链接** → escape_link 解析到工作区外 → 围栏在符号链接跳过逻辑之前开火。
- 本地全绿是假象：本机无 SeCreateSymbolicLinkPrivilege（编排者实测 `New-Item -ItemType SymbolicLink` → DENIED），测试在 `make_symlink_or_ignore`（tests.rs:1111）早退；CI runneradmin 有权限 → 真建链 → 触发 bug。该测试在 CI 上从未真正绿过（本地绿 = 跳过）。

**处方**：把 :328-337 的 `symlink_metadata` + `is_symlink() → continue` 移到 :319-325 围栏检查**之前**。安全性质不变：符号链接依旧既不被跟随也不被列出；非符号链接条目照旧逐项围栏。注意 `symlink_metadata` 不跟随链接，正是这里需要的语义。meta 变量后续 :338 `meta.is_dir()` 继续复用即可。

- blast radius：`list_workspace_tree` 的 callers = kernel facade DTO 面（desktop/MCP/ACP 目录列举），行为变化仅"符号链接从报错变跳过"——这正是该测试与兄弟测试（`read_file_rejects_symlink_to_outside_target` 走不同路径，不受影响）钉死的预期。
- 不许动 `is_within` 本身（其它调用点依赖其 canonicalize 语义）。

## 复用侦察（强制）

- 查 `platform.rs` 内是否已有"先取 symlink_metadata 再围栏"的现成顺序样例（如 read_workspace_file 的实现），有则对齐其模式。
- report 必须有「复用侦察」一节。无此节 = 未完成。

## Spec（必须全部满足）

1. `list_workspace_tree` 循环体内：符号链接判定（symlink_metadata + is_symlink → continue）发生在 `is_within` 围栏检查之前。
2. `metadata failed` 的错误返回语义保持（symlink_metadata 失败仍返回 Runtime 错误）。
3. 不改 `is_within`、`resolve_within_workspace`、`pick_workspace_root`。
4. 不改测试文件（测试定义的是正确期望，是实现错了）。

## Global Constraints

- 安全敏感路径：符号链接必须仍然既不被列出也不被跟随——只允许调整检查顺序，禁止削弱围栏。
- cargo 一律 `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo ...`；长命令 PTY + cmd 重定向；不 kill 任何非 northhing 进程。
- 家规：就近 AGENTS.md 优先（src/crates/assembly/core/AGENTS.md）。

## 验证（命令 + 输出原文进 report）

```powershell
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core w9_6
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --lib
```

说明：本机无符号链接权限，`list_tree_skips_symlink_to_outside_target` 本地只会走早退路径——本地绿是必要条件不是充分条件，最终验收靠 CI（编排者负责推分支观测）。report 里如实标注这一点。

## 报告

- 路径：`.superpowers/sdd/reports/W15-1g-report.md`
- 内容：改动摘要 / 复用侦察节 / 验证命令+输出原文 / 编译错误处置（预期无）/ 结尾状态词 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 派发元信息

- BASE commit：`6cbebbb`（请先 `git checkout -b fix/w15-1g-symlink-fence-order` 再动手；完成后 commit 到该分支，commit message 格式 `fix(core): check symlink before workspace fence in list_workspace_tree (W15-1g)`）
- **允许文件集**：`src/crates/assembly/core/src/kernel_facade/platform.rs`、报告文件（新建）
- 禁区：`is_within`/`resolve_within_workspace`/`pick_workspace_root` 函数体、测试文件、其它 crate
- 推送由编排者执行，你不要 push

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
