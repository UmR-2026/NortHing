# W16-4 Brief：theme.rs 行数预算内修复（unsafe O_NONBLOCK + 死代码腾行）

- 任务标识：W16-4
- 波次计划：`E:\agent-project\NortHing\.superpowers\sdd\plan-2026-09-05-w16-trusted-core.md`
- 来源：D-synthesis §9 拍板 1（theme.rs 不升 ceiling，活体对照试验田）+ deep audit `docs/reviews/deep-audit-2026-09-05-lsp-manager-theme.md`
- BASE：`559cd6f`（main HEAD）
- 目标文件：`src/apps/cli/src/ui/theme.rs`（当前 989 行 = ceiling 989，**零余量**）

## 背景（一句话）

deep audit 坐实该文件 5 项 rot-evidence，其中最重的是 unsafe 块无 SAFETY 注释 + `fcntl(F_SETFL)` 恢复调用返回值被 `let _` 丢弃（失败则 stdin 永久残留 O_NONBLOCK，后续所有 stdin 读取损坏）。用户拍板：ceiling 钉死 989，修复必须在行数预算内完成——本单同时是"闸能否逼出带修复的净减"的实验数据点。

## 允许文件集（diff 越出 = judge Critical）

1. `src/apps/cli/src/ui/theme.rs`

禁区：其它一切文件（含 rot-budget.json——本单**不动** ceiling）。

## 修复清单（四项，全部做）

1. **unsafe 块（约 L164-194，`#[cfg(unix)]` 的 `detect_terminal_appearance` 内）**：
   - 块顶加 `// SAFETY:` 注释，说明为什么对每个 fd 的 fcntl 调用是安全的（fd 有效性前提、flags 读写语义、无并发冲突前提）；
   - **消除 O_NONBLOCK 泄漏**：恢复调用 `libc::fcntl(fd, libc::F_SETFL, flags)`（约 L193）的返回值不再 `let _` 丢弃——检查返回值，失败时 `tracing::warn!`（English-only）记录；成功路径零行为变化。
2. **删死 API** `load_opencode_theme_json`（约 L728，audit 坐实无调用方、带 allow 标注）——连函数带其 `#[allow(dead_code)]` 标注删除。
3. **删两个误标 `#[allow(dead_code)]`**：`StyleKind`（约 L637）与 `OpencodeThemeJson.defs`（约 L700）——audit 已坐实为活符号，删标注不删代码；删除后编译不得产生 dead_code warning（若产生，说明 audit 判断错误，停手报 NEEDS_CONTEXT）。
4. **修正两条陈旧注释**：L635 附近 StyleKind 的 reason 注释（与 30+ 调用方现实矛盾，改写为真实用途）；L215 附近 parse_osc_color 的 "not yet wired" 注释（已接入 unix 分支，改写为真实状态）。

## 硬约束

- **净行数 ≤0**：删代码腾出的行数 ≥ SAFETY 注释与错误处理新增的行数。report 贴 `rg -c "^" src/apps/cli/src/ui/theme.rs` 前后对比。
- 除 fcntl 恢复调用的错误处理外，**零运行时行为变化**；颜色数学零触碰；RSX/UI 零触碰。
- 若四项做完净行数 >0，先在本文件内继续做死代码/冗余腾行（仅限 audit 已指认的项），仍无法达标则报 DONE_WITH_CONCERNS 并列出剩余行数缺口——**禁止**为凑行数删除有意义的注释或压行重排。

## 验证（命令 + 输出原文进 report）

```text
<USERPROFILE>/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing-cli
<USERPROFILE>/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing-cli theme
node scripts/verify-rot-budget.mjs
```

- 编排者已在 BASE 预跑：`cargo check -p northhing-cli` 绿（1 warning 基线），rot 绿。
- **cfg(unix) 覆盖声明（钉死）**：本机 MSVC 工具链不对 `#[cfg(unix)]` 块做语义检查（仅语法解析）。report 必须显式声明这一点，并说明 unix 块语义正确性由 CI `rust-build-check (ubuntu-latest)` 的 `cargo check --workspace` 兜底；实现者须逐行自审 unsafe 块的类型与 API 用法正确性并在 report 给出自审结论。若能 `rustup target add x86_64-unknown-linux-gnu` 且 cross-check 通过，作为加分项记录；失败不阻塞。
- rot 读数要求：`allow_dead_code` 计数 −2（106→104），god-file theme.rs 条目 989 不触发（行数 ≤989）。

## 报告

写到 `E:\agent-project\NortHing\.superpowers\sdd\reports\w16-4-report.md`（不入本 commit）：改动摘要（四项逐条 file:line 前后对照）/ 行数前后对比 / 验证输出原文 / cfg(unix) 覆盖声明与 unsafe 自审结论 / 结尾状态词。

## 派发元信息

- commit 规则：`git add src/apps/cli/src/ui/theme.rs`；message：`fix(cli): theme.rs unsafe SAFETY + O_NONBLOCK leak + dead code removal, net-zero lines (W16-4)`。
- skill 前置阅读：`E:\agent-project\.opencode\skills\rust-skills\unsafe-checker\SKILL.md` 与 `E:\agent-project\.opencode\skills\rust-skills\m15-anti-pattern\SKILL.md`——遵循其中与本任务相关的约定，不因此扩展任务范围。

## Global Constraints（摘编自计划）

1. 零新依赖；日志 English-only。
2. 验证命令输出原文进 report（命令 + exit code）。
3. commit 逐文件点名，禁 `git add -A`。
4. report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
