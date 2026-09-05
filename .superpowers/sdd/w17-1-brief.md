# W17-1 Brief：CI Windows-only 矩阵 + 去除 cli 警告 + 跨平台伤情挂账

- 任务标识：W17-1
- BASE：`1c9ac2f`（main HEAD，工作树干净）
- 来源：用户拍板 2026-09-05「本来就是 windows 限定的，不考虑其他平台」+ W16 终审 I-3（ci.yml 60 连红根因 = terminal-core E0624 在 macos/ubuntu 编译失败）+ 用户要求去除 `northhing-cli` 的 1 warning 基线

## 允许文件集（越界 = judge Critical）

1. `.github/workflows/ci.yml`
2. `src/apps/cli/src/ui/question/mod.rs`
3. `docs/status/tech-debt-ledger.md`

禁区：其它一切文件。

## 改动一：ci.yml rust-build-check 矩阵收窄为 Windows-only

- `rust-build-check` job 的 `strategy.matrix.os`：删除 `ubuntu-latest` 与 `macos-15` 两行，仅留 `windows-latest`。
- 删除 `Install Linux system dependencies (Tauri)` 步骤整块（矩阵收窄后其 `if runner.os == 'Linux'` 永不触发，留之是死配置）。
- `Setup OpenSSL (Windows, prebuilt)` 步骤保留（其 `if runner.os == 'Windows'` 条件保留或去掉均可，以保持 diff 最小为准）。
- 在该 job 上方或 matrix 附近加一行注释说明：`# Windows-only per user decision 2026-09-05; non-Windows builds currently broken (terminal-core E0624), see tech-debt-ledger`（措辞可微调，事实必须含：拍板日期 + 伤情出处）。
- **只允许上述删除/注释，不得重排、重命名或改动 ci.yml 任何其它内容**（其余 job：rust-tests-serial / kernel-api-clean / core-boundaries / rot-budget / repo-hygiene / i18n-contract 全部保持原样）。
- 本文件在 `workflow-policy.json` 的 `metaRatchetPaths` 内：本单已带用户拍板（Windows-only），审查走双 judge 车道（编排者安排，实现者无需操作）。

## 改动二：去除 northhing-cli 的 unused_imports 警告

- `src/apps/cli/src/ui/question/mod.rs:15`：`pub use types::{QuestionAction, QuestionData, QuestionOption, QuestionPrompt};` 中删除 `QuestionData` 与 `QuestionOption` 两个名字。
- 编排者已取证：仓内唯一消费方 `question.rs` 走 `super::types::QuestionData` / `super::types::QuestionOption` 直接路径，不经该重导出；types.rs 本体不动。
- 验证目标：`cargo check -p northhing-cli` 输出 **0 warning**（当前基线 1 warning）。

## 改动三：tech-debt-ledger 挂账

- `docs/status/tech-debt-ledger.md` 新增一条（先读该文件既有条目格式，就近模仿编号与字段）：
  - 症状：非 Windows 平台构建失败——`terminal-core` 在 macos/ubuntu 报 `error[E0624]: method deadline is private` ×2（CI run 33964321637，2026-09-05）；
  - 处置：`deferred`——按用户 2026-09-05「Windows 限定」拍板挂起；若未来恢复跨平台支持需先修此项；
  - 关联：ci.yml 矩阵同日收窄为 windows-only（W17-1）。

## 验证（输出原文进 report）

```text
<USERPROFILE>/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing-cli
node scripts/verify-rot-budget.mjs
node scripts/check-repo-hygiene.mjs
```

（编排者已在 BASE 预跑第一条：绿 + 1 warning 基线。验收要求 0 warning。）

## 提交（两个 commit）

1. `ci: windows-only build matrix per user decision 2026-09-05 (W17-1)` —— `ci.yml` + `tech-debt-ledger.md`
2. `fix(cli): drop unused question re-exports, zero-warning baseline (W17-1)` —— `mod.rs`

逐文件点名 add，禁 `git add -A`。

## 报告

`.superpowers/sdd/reports/w17-1-report.md`（不入 commit）：三改动逐条 file:line / 验证输出原文 / 结尾状态词。

## Global Constraints

1. 零新依赖；输出 English-only。2. 验证输出原文进 report。3. ci.yml 改动仅限指定删除/注释。4. 结尾状态词 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
