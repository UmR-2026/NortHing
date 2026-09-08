# W20-1 Brief — P2-23 清账：terminal-core 非 Windows E0624 修复

## 任务标识

W20-1（清账波首单；P2-23：terminal-core 在 macOS/ubuntu 报 `error[E0624]: method deadline is private` ×2，CI run 33964321637 实证，用户 2026-09-08 拍板清挂账）。范围声明：本单只清**编译错误**挂账；非 Windows 平台的完整支持政策仍按用户 2026-09-05「Windows 限定」拍板不动，本单不恢复 CI 矩阵。

## BASE

`2b9428b`（main HEAD，代码树干净；派发时 task-gate 起点 = 本 brief 提交后的 docs commit，与 2b9428b 代码树等价，仅 docs 差异）。

## 允许文件集

- `src/crates/services/terminal/src/exec/types.rs`（修改，预期一行级可见性修复）
- `docs/status/tech-debt-ledger.md`（修改，仅 P2-23 条目 status 翻转——家规 2 同 commit）

report 写 `.superpowers/sdd/w20-1-report.md`，不进本单验收 diff。

## 背景与根因

`src/crates/services/terminal/src/exec/types.rs:195-202`：`#[cfg(unix)] impl LocalPipeControlState { fn deadline(self) }`——方法无可见性修饰 = 模块私有；调用点在 `exec/output.rs:482` 与 `:493`（跨模块）。Windows 下 impl 块与调用点同被 cfg 门控消去故编译通过；unix 目标下 E0624 ×2（两个调用点同一方法）。

**BASE 负向证据（编排者已实测复现，implementer 须复跑确认）**：
`rustup run stable-x86_64-pc-windows-msvc cargo check -p terminal-core --target x86_64-unknown-linux-gnu`
→ `error[E0624]: method deadline is private` ×2（output.rs:482/493），exit 非 0。

## 功能要求

1. 修复可见性：`fn deadline` → `pub(crate) fn deadline`（最小口径；若实现中发现第二处私有项同样被跨模块调用，一并同口径处理并在 report 说明）。
2. 修复后两个 unix 目标 check 全绿（见验证 1/2）。
3. Windows 回归零影响（验证 3/4）。

## Constraints

- 最小 diff：预期单行改动；不做顺手重构。
- commit 逐文件点名 `git add`；message 前缀 `fix(terminal-core):` + `(W20-1)` 后缀，body 引用户拍板「2026-09-08 清挂账」。
- report 中验证命令贴原文输出 + exit code；本地绝对路径用 `<REPO_ROOT>` 占位。
- 工具链纪律（AGENTS.md）：repo 目录 override 为 GNU，桌面构建用 MSVC；一律 `rustup run <tc> cargo` 形式调用。
- 收口前必跑 `node scripts/check-repo-hygiene.mjs`。
- report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
- 先读就近文档：`src/crates/services/AGENTS.md`。

## 禁区

- 禁改 output.rs 调用点（它们是对的，错的是可见性）。
- 禁动 cfg 门控结构；禁顺手「修复」其他 cfg(unix) 代码。
- 禁恢复 CI 矩阵 / 禁改 ci.yml。
- 禁改 terminal-core 其他任何文件。

## 验证

1. `rustup run stable-x86_64-pc-windows-msvc cargo check -p terminal-core --target x86_64-unknown-linux-gnu` —— 绿（BASE 复跑红 → 修复后绿，双态输出都贴）。
2. `rustup run stable-x86_64-pc-windows-msvc cargo check -p terminal-core --target aarch64-apple-darwin` —— 绿（target 已预装；若该目标环境性不可达，贴错误原文并标 DONE_WITH_CONCERNS）。
3. `rustup run stable-x86_64-pc-windows-gnu cargo check --workspace` —— 绿（Windows 回归）。
4. `rustup run stable-x86_64-pc-windows-gnu cargo test -p terminal-core` —— 绿。
5. `node scripts/check-repo-hygiene.mjs` —— 绿。

## 报告

写 `.superpowers/sdd/w20-1-report.md`，必含三节：**改动摘要**（含根因一句话）/ **验证**（5 条命令原文输出 + exit code，含 BASE 红态证据）/ **状态**（状态词结尾）。另须如实记录：cross-target check ≠ 真机构建（无链接、无 CI runner），残余风险一句话。
