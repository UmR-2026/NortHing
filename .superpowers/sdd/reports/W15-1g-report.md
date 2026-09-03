# W15-1g Report: list_workspace_tree 符号链接检查顺序修复

分支：`fix/w15-1g-symlink-fence-order`（BASE `6cbebbb`）
改动文件：仅 `src/crates/assembly/core/src/kernel_facade/platform.rs`（允许文件集内）

## 改动摘要

`list_workspace_tree` 循环体内，把 `tokio::fs::symlink_metadata` + `is_symlink() → continue` 块移到
`is_within` 围栏检查**之前**（原 platform.rs:320 围栏先于 :328 符号链接判定）。根因与处方按 brief 预检段逐字执行：
`is_within`（:138-147）对 candidate 调 `std::fs::canonicalize`，会跟随符号链接 → 工作区内指向工作区外的
`escape_link` 被解析出界 → 围栏在跳过逻辑之前开火返回 `Validation("entry escaped workspace")`，
CI windows（runneradmin 有符号链接权限、真建链）测试红。

安全性质核对（Spec 逐条）：

1. ✅ 符号链接判定现发生在 `is_within` 之前（diff 见下）。
2. ✅ `symlink_metadata` 失败仍返回 `KernelError::Runtime("metadata ... failed")`，块体逐字搬移，错误语义未动。
3. ✅ `is_within` / `resolve_within_workspace` / `pick_workspace_root` 函数体零改动（diff 只触及 `list_workspace_tree` 循环体）。
4. ✅ 测试文件未动。
5. ✅ 符号链接依旧既不被列出（continue 先于 `out.push`）也不被跟随（判定用不跟随链接的 `symlink_metadata`）；非符号链接条目照旧逐项过围栏后才入栈/输出。围栏未被削弱，仅调整先后。

净 diff：`platform.rs` +19/-15（一个代码块位置对调 + 4 行原因注释，无其它文件）。

## 复用侦察

- `platform.rs` 内现成的"符号链接属性先于 canonicalize 围栏生效"的模式 = `resolve_within_workspace`
  （platform.rs:113-126）：用不跟随链接的 `symlink_metadata` 判符号链接、直接拒绝，其 doc 注释（:56-65）
  写明设计意图——canonicalize 会跟随链接所以词面围栏不够，符号链接属性必须单独判。本次改动即把
  `list_workspace_tree` 对文件系统来源条目的处理对齐到同一"先判符号链接属性、再谈 canonicalize 围栏"的语义
  （差别：list 场景条目非用户提供，跳过即可，与兄弟测试预期一致）。
- `list_workspace_tree` 内原 `symlink_metadata` 块（含 `metadata ... failed → Runtime` 错误构造）逐字复用，未新写任何辅助函数/类型；未新增依赖；未新增 i18n/日志面（错误消息模板原样保留，英文无 emoji）。
- 结论：无需新代码，纯顺序对调，是本文件内最短合法改法。

## 验证命令 + 输出原文

本机无 `SeCreateSymbolicLinkPrivilege`（brief 预检实测），两个符号链接测试本地走
`make_symlink_or_ignore`（tests.rs:1111）早退路径返回 ok——**本地绿是必要条件不是充分条件，
该用例的真正验证靠 CI windows job（编排者负责推分支观测）**。

**偏差说明（brief 验证命令缺 feature，基线即不可编译）**：`northhing-core` 的 Cargo.toml `default = []`，
按 brief 原文跑 `cargo test -p northhing-core w9_6` 在未改动的基线上就报 3 个既有 feature-gate 编译错
（`skill_watch.rs:12/13` 需 `product-full`、`config/service.rs:359` 需 `ai-adapter-runtime`，均非本次触及文件）。
CI 用 `cargo test --locked --workspace`（ci.yml:96）经 feature 统一隐式开启 product-full，故 CI 基线是绿的。
本地最小等价复现 = 显式补 `--features product-full`：

```powershell
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core w9_6 --features product-full
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --lib --features product-full
```

命令 1（12 个 w9_6 用例全绿，含目标用例）：

```
running 12 tests
test kernel_facade::tests::w9_6_file_tree::path_fence_rejects_escape_segments ... ok
test kernel_facade::tests::w9_6_file_tree::read_file_rejects_symlink_to_outside_target ... ok
test kernel_facade::tests::w9_6_file_tree::list_tree_rejects_parent_dir_escape ... ok
test kernel_facade::tests::w9_6_file_tree::read_file_with_explicit_workspace_root_uses_that_fence ... ok
test kernel_facade::tests::w9_6_file_tree::list_tree_skips_symlink_to_outside_target ... ok
test kernel_facade::tests::w9_6_file_tree::list_tree_lists_direct_children ... ok
test kernel_facade::tests::w9_6_file_tree::read_file_rejects_too_large ... ok
test kernel_facade::tests::w9_6_file_tree::list_tree_rejects_absolute_path ... ok
test kernel_facade::tests::w9_6_file_tree::read_file_rejects_escape ... ok
test kernel_facade::tests::w9_6_file_tree::list_tree_rejects_non_absolute_workspace_root ... ok
test kernel_facade::tests::w9_6_file_tree::list_tree_with_explicit_workspace_root_uses_that_fence ... ok
test kernel_facade::tests::w9_6_file_tree::read_file_round_trip_within_cap ... ok
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1058 filtered out; finished in 0.03s
```

命令 2（全量 lib 回归）：

```
test result: ok. 1069 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.83s
```

## 编译错误处置

- 未改动代码本身零编译错误（一次通过）。
- 基线 feature-gate 错（E0433 ×3，机制层：命令行补 `--features product-full`，不改任何代码）——如上偏差说明。

## 工作树状态备注（非本任务改动）

- brief 称"src 干净"，实际派发时工作树有 5 个 src 文件未提交改动（`session_subhandlers.rs`、
  `kernel_facade/session.rs`、`service/workspace/accessors.rs`、`json_store.rs`、`metadata_store.rs`）
  + `progress.md`。与本任务文件集不相交，未 staged、未触碰；本地验证是在含这些 WIP 的树上跑的
  （它们编译通过且 1069 全绿，无干扰）。
- `rustfmt` 组件在 stable-x86_64-pc-windows-msvc 工具链未安装，未擅自安装；改动为纯块搬移，缩进/风格与
  周边逐字一致。

## 遗留风险

- 目标用例本地仅早退绿（无符号链接权限），修复对"真符号链接"路径的效果以 CI windows 为准（编排者观测）。
- 行为变化面 = `list_workspace_tree` 对文件系统条目"符号链接从可能报 Validation 错 → 静默跳过"，与测试钉死的
  预期及 desktop/MCP/ACP 目录列举用途一致；`read_workspace_file` 路径（走 `resolve_within_workspace`）未动。

DONE
