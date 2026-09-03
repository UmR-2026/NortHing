# Review — W15-1g（list_workspace_tree 符号链接检查顺序修复）

- 分支：`fix/w15-1g-symlink-fence-order`，BASE `6cbebbb` → HEAD `ea2882c`
- commit：`ea2882c887696f7d57c508ac3ae3aaa2d75c17b6`（单 commit，message 严格匹配 brief 模板）
- diff 文件集：`platform.rs` + `.superpowers/sdd/reports/W15-1g-report.md`（与允许集完全一致）

---

## SPEC 逐条判决

| # | 验收标准 | 判决 | 证据 |
|---|---|---|---|
| 1 | 修复后 `cargo test -p northhing-core w9_6`（实现者补 `--features product-full`）本地绿 | PASS | 我独立跑 `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-core w9_6 --features product-full`，12/12 全绿；含目标用例 `list_tree_skips_symlink_to_outside_target ... ok`。输出与 report §"验证命令 + 输出原文" 命令 1 完全一致 |
| 2 | CI 转绿（由编排者推分支后观测） | 不在审查范围 | brief 明确"不在审查范围"。报告无伪证风险 |
| 3 | diff 只触及允许文件集（platform.rs + report） | PASS | `git diff 6cbebbb..ea2882c --name-only` 仅两行：`platform.rs` + `W15-1g-report.md`；working tree 另有 5 个 WIP 文件未提交、未 staged（review package 已声明忽略） |
| 4 | `metadata failed` 错误语义保留；`is_within`/`resolve_within_workspace`/`pick_workspace_root` 零改动；测试文件零改动 | PASS | diff hunk 仅 `@@ -316,15 +316,10 @@` 与 `@@ -335,6 +330,14 @@` 两个区块，均落在 `list_workspace_tree` 循环体内。`is_within`(138-147)、`resolve_within_workspace`(66-133)、`pick_workspace_root`(1-54) 函数体未在 diff 中出现。`tests.rs` diff 为空（0 行） |
| 5 | Spec 子项 1：符号链接判定在 `is_within` 之前 | PASS | diff 第 318 行起先 `let meta = match tokio::fs::symlink_metadata(&p)...`，第 329-332 行 `if meta.file_type().is_symlink() { continue }`，然后第 333-339 行才是 `is_within` 围栏 |
| 6 | Spec 子项 2：`metadata failed` 仍返回 Runtime 错误 | PASS | 原 platform.rs:323-326 错误块逐字搬移：`return Err(KernelError::Runtime(format!("metadata {} failed: {e}", p.display())))`；错误消息模板未改 |
| 7 | Spec 子项 3：`is_within`/`resolve_within_workspace`/`pick_workspace_root` 零改动 | PASS（已在 #4 举证） | — |
| 8 | Spec 子项 4：测试文件零改动 | PASS | `git diff 6cbebbb..ea2882c -- src/crates/assembly/core/src/kernel_facade/tests.rs` 为空 |
| 9 | Global Constraint：符号链接既不被列出也不被跟随 | PASS | 见下文"安全敏感复核"段 |

---

## 安全敏感复核（特别核查点）

顺序对调后逐行核对（platform.rs:318-352）：

1. `let p = entry.path();` — 拿候选路径
2. `tokio::fs::symlink_metadata(&p)` — **不跟随链接**（用词正确）
3. `if meta.file_type().is_symlink() { continue }` — 符号链接跳过
4. `if !is_within(&workspace_root, &p) { return Err(...) }` — 围栏
5. `out.push(FileTreeEntryDto { ... })` — 非符号链接条目入栈
6. `if is_dir && depth < depth_limit { stack.push(...) }` — 目录递归

逐项判决：

- **(a) 符号链接不出现于列举结果**：✅ `continue` 早于 `out.push` 与 `stack.push`。符号链接既不被加入结果向量也不进入后续递归。
- **(b) 符号链接不被 canonicalize 跟随**：✅ 围栏之前已通过 `continue` 跳出；围栏本身 `is_within` 仅对**非符号链接**条目调用。`tokio::fs::symlink_metadata` 自身不跟随链接（标准 lstat 语义），故 `escape_link` 的目标 `/tmp/.../readme.md` 永远不会被触碰——正确保住"既不列出也不跟随"。
- **`symlink_metadata`（不跟随）vs `metadata`（跟随）用词正确**：✅ diff 全文 `tokio::fs::symlink_metadata`，无 `tokio::fs::metadata`（后者会跟随链接，破坏安全语义）。注释新加段落明文："`symlink_metadata` does not follow links, so this must run before `is_within`: its `canonicalize` would resolve an escaping symlink and trip the fence instead of skipping."——语义与行为对齐。

结论：安全性质未削弱，仅调整先后。

---

## QUALITY（独立判断）

### 复用侦察
- report §"复用侦察"存在且属实（platform.rs:24-32 段），对齐到同文件 `resolve_within_workspace`(66-133) 的"先判符号链接属性、再谈 canonicalize 围栏"模式。我比对 doc 注释（:56-65）："`absolute()` does not follow symlinks... We re-fence using `std::fs::canonicalize` (which DOES follow symlinks) and additionally reject any user path whose `symlink_metadata` reports a symlink"——与本次 list 端的设计意图一致，方向相同（用户路径直接拒绝，文件系统条目仅跳过）。✅
- `meta` 变量在搬移后被 `meta.is_dir()` (341)、`meta.len()` (343) 继续复用——未引入第二份元数据查询，无 IO 浪费。✅

### 无 owner 抽象（owner abstraction）
- 无新增 trait、interface、wrapper、helper；diff 净 +11/-8 = 仅一处顺序对调 + 4 行注释。✅

### 预算闸（rot-budget / baseline 类）
- `platform.rs` 行数：BASE 370 → HEAD 373（+3）。**未触 800 行阈值**，未在 `scripts/rot-budget.json` `god_file:` 清单内登记。✅
- rot-budget 无上调、无数值修改。✅

### god-file 健康度观测
- `platform.rs` 373 行，远低于 800；模块结构清晰（helpers 56-147、impl 167+），无折叠/重复信号。✅

### 设计意图的注释质量
- 新加的 3 行注释（:320-322）解释了**为什么**而不是**做了什么**（避免 canonicalize 把 escape symlink 解析出界再触发围栏），对未来读者定位"为什么这里没有先 is_within"非常关键。✅

### 错误消息模板
- "metadata {} failed: {e}" 逐字保留；"entry escaped workspace: {}" 逐字保留；英文无 emoji；符合 `src/crates/LOGGING.md` 默认。✅

### i18n / 日志面
- 未新增错误消息、未新增日志——无新增 i18n key 负担。✅

### 跨任务接口（无）
- 本任务为单点顺序修复，无跨任务接口耦合。

### 旁路文件 / working tree 状态
- 工作树有 5 个 WIP 修改（`session_subhandlers.rs`、`kernel_facade/session.rs`、`service/workspace/accessors.rs`、`json_store.rs`、`metadata_store.rs`）未提交，与本任务文件集不相交。review package 明示忽略，符合 AGENTS.md "外延小债务可在同 commit 顺手清"但此处不属于"顺手清"语境（与本任务无关的 WIP）。✅

---

## Findings

无 Critical、无 Important、无 Minor。

（diff 净 +11/-8，单点顺序对调；无重复逻辑、无注释腐烂、无命名异味、无循环复杂度上升。）

---

## Cannot verify from diff

下列项需 CI 实测，diff 不能判定：

1. CI windows 真符号链接路径下 `list_tree_skips_symlink_to_outside_target` 真正转绿（而非走 `make_symlink_or_ignore` 早退）。本机因无 `SeCreateSymbolicLinkPrivilege`，本地 12 个 w9_6 用例绿是早退绿——`list_tree_skips_symlink_to_outside_target` 本地路径与 CI runneradmin 路径行为差异详见 brief §"编排者预检结论"。编排者负责推分支后观测。
2. CI ubuntu/macos job 是否仍然全绿（diff 仅触动 Rust 标准库 API 的调用顺序，理论上与平台无关；但 cross-platform 实测属 CI 职责）。
3. 邻接测试 `read_file_rejects_symlink_to_outside_target` 走的是 `read_workspace_file` 路径（`resolve_within_workspace`），未在 diff 中触及——逻辑上不受影响，但 CI 全量回归是最终判定。

---

## APPROVE

结论：**APPROVE**

判决摘要：
- SPEC：9/9 PASS（含所有 brief 与 review package 列出的验收标准与 Spec 子项）
- QUALITY：复用侦察属实 / 无 owner 抽象 / 预算闸无触发 / god-file 健康
- 安全复核：符号链接既不出现于列举结果也不被跟随；`symlink_metadata`/`metadata` 用词正确
- Findings：无
- Cannot verify：3 项已分别列出，均属编排者/CI 责任，diff 本身无可打回之处

实现者报告内容与 diff 一一对得上，无伪证；遗留 caveat（CI 真链接路径验证、cross-platform job）已如实标注。