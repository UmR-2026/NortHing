# Task T2-2p Brief — P2-21 执行：MiniApp 契约层三处 serde/wire 残留删除

> 用户已拍板（2026-08-19）：删。依据：三处全部零构造/零生产者（recon 实测 + M5 终审复核），磁盘旧数据不可能含这些值，风险≈0；MiniApp 无重建计划（对照 DialogTriggerSource::RemoteRelay/Bot 有 T5 重建才保留）。
> 来源：tech-debt-ledger P2-21、`.superpowers/sdd/task-t2-2-miniapp-recon.md` Q7。

## 工作目录
`E:\agent-project\northing`（git main；开工先 `git log --oneline -1` 实测 HEAD 并记入 report）

## 环境硬事实
- cargo 用 MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`。
- 中文编辑一律用 edit 工具。
- 并行 session 未提交改动（`memory/`、`.opencode/model-capability-notes.md`、`.handoffs/`）勿碰勿提交。

## 变更清单（全部显式授权，共 4 文件）

1. `src/crates/contracts/core-types/src/surface.rs:52`：删 `RuntimeArtifactKind::MiniApp` 变体单行（枚举其余变体 Diff/TerminalSnapshot/Preview/Usage/ReviewReport/McpManifest 全部保留，serde rename_all 属性不动）。
2. `src/crates/services/services-core/src/session/session_metadata.rs:27`：删 `SessionRelationshipKind::Miniapp` 变体单行（其余变体保留）。
3. `src/crates/services/services-core/src/session/lineage.rs:19`：`BRANCH_EXCLUDED_TAGS` 数组中摘除 `"miniapp"` 元素（`"btw", "review", "deep_review", "subagent"` 保留）。
4. `docs/status/tech-debt-ledger.md`：P2-21 条目翻 resolved（house rule 2 同 commit），resolution 注明：用户 2026-08-19 拍板删除，本任务执行，commits 见 git log T2-2p。

## 动手前强制复核（write 进 report）
- `rg -n "RuntimeArtifactKind::MiniApp" src/ tests/` 除定义行外零命中。
- `rg -n "SessionRelationshipKind::Miniapp" src/ tests/` 除定义行外零命中。
- `rg -n '"miniapp"' src/` 除 lineage.rs:19 外零命中。
- 任一复核出现额外命中 → STOP 报告，不删。

## 不做（红线）
- 不动两个枚举的其它变体与 serde 属性；不动 `DialogTriggerSource`、`RemoteSsh` 等 TH-5 保留词汇；不碰并行 session 文件；不做清单外改动与格式化。

## 验证（每条都跑，report 贴命令+输出原文）
1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace` → PASS
2. `... cargo test -p northhing-core-types` → PASS
3. `... cargo test -p northhing-services-core session` → PASS（若无匹配测试则说明并跑 `--lib`）
4. 复核 3 条 rg 输出原文；`git status --short` 仅 4 文件 + 并行 session 预存改动。

## Report
写 `.superpowers/sdd/task-t2-2p-report.md`：每项文件:行+前后摘要、验证命令+输出原文、复核输出、编译错误修在哪一层（机制层/设计层，一行一个）。假汇报=停用。不要自己 commit。最终状态 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
