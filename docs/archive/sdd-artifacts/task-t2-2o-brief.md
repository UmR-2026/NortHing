# Task T2-2o Brief — MiniApp 整删 M5：顶层 MiniApp/ + 文档收口（无 serde 变更）

> 批次权威文档：`.superpowers/sdd/task-t2-2-miniapp-recon.md`（Q4 文档面 / Q8-M5）。
> 前置 M1-M4 已并 main（a930c93 / 980c879 / 111938e / b094075）：代码面 miniapp 已删光，`rg -i miniapp scripts/core-boundaries` 已归零。
> **用户决策悬空**：core-types `RuntimeArtifactKind::MiniApp`（surface.rs:52）、services-core `SessionRelationshipKind::Miniapp`（session_metadata.rs:27）、lineage.rs:19 `"miniapp"` tag——三处 serde/wire 残留**本批不动**，登记 P2-21 待用户拍板（decide 超时，默认保守路径）。
> 本批 = 顶层 MiniApp/ 目录整删 + 全部文档收口 + 台账登记 + 终扫。

## 工作目录
`E:\agent-project\northing`（git main；开工先 `git log --oneline -1` 实测 HEAD 并记入 report）

## 环境硬事实（必读）
- cargo 一律 MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`。
- 中文文件编辑一律用 edit 工具（PowerShell 写非 ASCII 会 GBK 双重编码）。
- 工作区并行 session 未提交改动（`memory/`、`.opencode/model-capability-notes.md`、`.handoffs/`）勿碰勿提交。
- 行号来自 recon（基线 3702baf），漂移时以符号搜索为准；不一致则 STOP 报告。

## 变更清单（全部显式授权）

### A. 顶层 MiniApp/ 整删
1. 整删 `MiniApp/` 目录（7,953 行：`Skills/miniapp-dev/` 4 md 696 行 + `Demo/git-graph/` ≈6,028 行 + `Demo/icon-design-system/` ≈1,229 行）。recon 实测无任何脚本/配置引用（唯一代码提及 miniapp-dev 的 miniapp_init_tool.rs 已在 M1 删除）。

### B. 文档收口（house rule 2，同 commit）
2. `docs/status/surfaces.md:22`：MiniApp UI Frozen 行——删除该行（面已不存在；surfaces.md 是现状登记表，已删面不留行）。
3. 根 `AGENTS.md`：:26,35,176,179 的 MiniApp/miniapp 提及清理。⚠️ :179 附近骨架不变量行含 "MiniApp string-mode commands containing shell metacharacters are rejected"——**guard_command_execution 本体保留**，只摘除该句中的 MiniApp 分句/措辞，shell 安全不变量其余文字一字不动。
4. 根 `AGENTS-CN.md`：:25,34,137,140 同义清理，与英文版语义对齐。
5. `README.md:43`：Frozen-experimental 枚举中去掉 `MiniApp UI`（保留 CLI / server / SDLC harness）。
6. `docs/tech-debt-cleanup-guide.md:12,75,115`：miniapp 提及同步清理。
7. `docs/architecture/backend-roadmap.md`：
   - :85 SW1-1 行：标注随 MiniApp 整删关闭（moot）。
   - :96 依赖关系行：MiniApp 整删已执行（更新"删除前唯一要求…"措辞为已完成事实）。
   - :117 MiniApp host 行：host 已删，更新/移除该行。
   - :167 T2-2 行：MiniApp 子系统整删部分标 done（M1-M5 commits 区间）——**此行可整行标完成**（remote 栈 + MiniApp 两半均毕）。
   - :185 T2-5 行：unwrap 治理清单中的 `miniapp::manager` 目标已不存在，从清单摘除（T2-5 行其余目标保留）。
   - :216 T3-5 行（已划掉）：补一句关闭回链（随 T2-2 M 批完成）。
   - :247 MiniApp 第三方生态行：该行前提（T1-1+T3-5）已随整删关闭，更新措辞或标注失效。
   - :190-206 PCS-3 语义段：**保留不动**（自足设计依据，明确不回溯旧码）。
8. `docs/status/decision-register.md:40` P-14 行：补执行回链（T2-2 M1-M5，commits 区间，本批 commit 占位——report 里留 TODO 占位词 `<this-commit>`，编排者收口时回填，或写"M5 commit 见 git log T2-2o"均可）。
9. `docs/status/tech-debt-ledger.md`：新增 **P2-21** 条目——MiniApp 契约层三处 serde/wire 残留（core-types surface.rs:52 `RuntimeArtifactKind::MiniApp`、services-core session_metadata.rs:27 `SessionRelationshipKind::Miniapp`、services-core lineage.rs:19 `"miniapp"` tag）零构造零生产者，删除有旧数据反序列化兼容风险，2026-08-19 用户决策超时未拍板，悬置待决（来源：T2-2 MiniApp recon Q7）。

### C. 终扫（verify-only）
10. `rg -n -i "miniapp|mini_app|mini-app" --glob '!docs/archive/**' --glob '!docs/handoffs/**' --glob '!docs/superpowers/**' --glob '!.superpowers/**' --glob '!memory/**' --glob '!research/**' --glob '!target/**' --glob '!docs/migration-2026-07-16/**'` → 剩余命中应只含：契约层三处（已登记 P2-21）、`tool_call_accumulator.rs:150` 测试串（见 D）、roadmap PCS-3 语义段（自足保留）、注释/历史措辞。逐条列出并归类；发现代码面新残留 = STOP 报告。
11. `rg -i miniapp scripts/core-boundaries` → 应已归零（M4 达成），复核确认。

### D. 测试残留（授权）
12. `src/crates/execution/agent-stream/src/tool_call_accumulator.rs:150`：测试用例表 `("InitMiniApp", "Markdown Viewer")` 一行删除（引用的工具已不存在；若该行删除导致测试语义变化而非纯用例行删除，STOP 报告）。

## 不做（红线）
- 不动三处 serde/wire 残留（P2-21 悬置）。
- 不动 roadmap :190-206 PCS-3 段；不改 docs/archive、docs/handoffs、research、migration 历史文档。
- 不改 guard_command_execution 本体与骨架不变量其余文字。
- 不碰并行 session 文件；不做清单外改动与无关格式化。

## 验证（每条都跑，report 贴命令+输出原文）
1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace` → PASS
2. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing` → PASS（desktop 门禁）
3. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-agent-stream` → PASS（D 项改动）
4. `node scripts/check-core-boundaries.mjs` → PASS
5. C 项两条终扫输出原文。
6. `git status --short` → 仅本批清单文件 + 并行 session 预存改动。

## Report
写 `.superpowers/sdd/task-t2-2o-report.md`：每项文件:行+前后摘要、验证命令+输出原文、终扫归类表、编译错误修在哪一层（机制层/设计层，一行一个）、偏离说明。假汇报=停用。不要自己 commit。最终状态 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
