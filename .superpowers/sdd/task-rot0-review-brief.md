# Review Brief — ROT-0 顺手批

## 审查对象

- 仓库：`E:\agent-project\.worktrees\northing-rot0`（分支 feat/rot0-sweep-0821）
- 范围：`cc0eba2..63c34b2`（单 commit）
- diff 包：`.superpowers/sdd/review-package-rot0.diff`
- 实现 brief / report：`.superpowers/sdd/task-rot0-brief.md` / `task-rot0-report.md`

## 约束（本任务 spec 的精确要求）

- surfaces.md 只许改 `:50` 一行路径（外加"确定错"的同类项——report 称抽查 21 行全对，复核其抽查说法）。
- CHANGELOG：只新增 `## [Unreleased]` 段，0.2.10 及更早段落零改动；**每条 commit 锚点必须真实存在**（逐条 `git show --oneline <hash>` 或 `git log --all --oneline | rg <hash>` 核实，锚点造假/编造 = Critical）。
- TLS：reqwest features 删 `"native-tls"` 后，`cargo check --workspace` 与 `cargo check -p northhing`（MSVC wrapper）必须过；http.rs 的改动必须是语义等价（原来是默认 TLS 还是显式 native-tls？`.use_rustls_tls()` 后行为是否变化——若原来是"reqwest 默认后端自动选"，双 feature 时 native-tls 优先还是 rustls 优先？这决定行为是否真等价，给出你的判断与证据）。
- runtime-services：核销，diff 中不该有它的代码改动。
- rot-budget.json / growth 线文件不许触碰。

## 独立验证（你必须实跑）

1. `cargo check --workspace` + `cargo check -p northhing`（MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`）
2. `node scripts/check-core-boundaries.mjs` + `pnpm run check:rot`
3. CHANGELOG 锚点逐条核实（见上）
4. `rg 'native.tls|native_tls' src --glob '*.rs'` 与 `rg native Cargo.toml` 确认无残留引用

## 你的角色定位

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

## 双判决（缺一不算通过）

1. **SPEC**：对照 brief 验收逐条 PASS/FAIL + file:line 证据。
2. **QUALITY**：常规项 + 三必查（复用核查 / 无 owner 抽象 / 预算闸）。god-file 观测点：本 diff 未触及登记文件，跳过。

## Cannot verify from diff

无法判定的单独列出，禁止猜。

## 档位

Critical / Important / Minor。plan-mandated 冲突交编排者。

## 报告

`.superpowers/sdd/task-rot0-review.md`：双判决、证据、独立验证、findings。最终消息以 APPROVED / REJECTED 开头。
