# Review Brief — ROT-1（T2-9 冗余合并批 1）

## 审查对象

- 仓库：`E:\agent-project\.worktrees\northing-rot1`（分支 feat/rot1-dedup-0821）
- 范围：`30a8590..9721f75`（单 commit）
- diff 包：`.superpowers/sdd/review-package-rot1.diff`
- 实现 brief / report：`.superpowers/sdd/task-rot1-brief.md` / `task-rot1-report.md`

## 约束（本任务 spec 要求的精确值与关系）

- deep_research：agent-runtime 副本删除，`northhing_agent_runtime::deep_research::{renumber_research_report, ResearchCitationDisplayMapEntry}` 路径必须存活（tests/deep_research_contracts.rs 与 lib.rs:32 不许改）；层方向 execution→contracts 合法。
- 时间 helper：core-types `time.rs` 两个函数签名 `now_unix_ms() -> u64` / `now_unix_millis() -> i64`；转换恰好 4 处命名重复（goal_mode u64 / evidence_ledger u64 / cron i64 / metadata_store u64）；**不许转** agent-runtime cache_types、debug-log、acp、80 处内联点。
- 核销四项（ndjson / FILE_LOCKS / server bootstrap / CLI init）：无代码改动为预期形态；FILE_LOCKS 保留是预期结论（它守 backup+rename 序列）。
- 行为等价纯搬移；rot-budget.json 不许被本 diff 触碰。

## 独立验证（你必须实跑）

1. `cargo check --workspace`（MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`）
2. `cargo test -p northhing-agent-runtime --test deep_research_contracts`
3. `cargo test -p northhing-core-types`
4. `node scripts/check-core-boundaries.mjs` + `pnpm run check:rot`
5. **抽查 re-export 完整性**：在原 255 行副本（`git show 30a8590:src/crates/execution/agent-runtime/src/deep_research.rs`）里挑 3 个 pub 符号，确认经 `pub use ...::deep_research::*` 后仍可用（编译已过不代表语义等价——核对是否有原副本里有、runtime-ports 版没有的符号或行为差异，brief 说 diff 仅 5 行，核这 5 行改了什么）。
6. **重点核对 `scripts/core-boundaries/rules/source/required-rules.mjs` 的改动**：实现者主动改了边界检查规则——这是 brief 未明列的文件，判断是否确属必要（路径失效）还是越权改规则放行自己。

## 你的角色定位

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

## 双判决（缺一不算通过）

1. **SPEC**：对照实现 brief 的验收标准逐条判定 PASS/FAIL，给 file:line 证据。
2. **QUALITY**：代码质量独立判断。除常规项外，以下三条为必查项：
   - **复用核查**：实现者 report 的「复用侦察」一节是否存在且属实——抽查其声称"无既有实现/已对齐先例"的点，用 codegraph/rg 独立验证；发现复制既有能力而不复用 = Important 起评。
   - **无 owner 抽象**：diff 中每个新增抽象必须绑定当前真实消费方；投机性抽象 = Important 起评。
   - **预算闸**：diff 若触碰 `scripts/rot-budget.json` 且是上调 ceiling/放松规则，一律 SPEC FAIL。
   - **god-file 观测点**：本 diff 未触及 7 个登记文件，跳过。

## god-file 基线观测（实验第一批数据点，与 diff 无关）

顺便完成：通读以下 3 个文件（活体对照组的活跃面子集），各写一段 T0 基线健康度描述（关切构成 / 是否混杂纠缠 / 当前清晰还是纠结 / 一句依据），作为后续观测的对比锚点：
- `src/apps/desktop/src/app_state/callbacks_lifecycle.rs`（1063 行）
- `src/crates/assembly/core/src/service/agent_memory/memory_db.rs`（918 行）
- `src/crates/assembly/core/src/service/agent_memory/facts.rs`（905 行）

## Cannot verify from diff

无法从 diff 判定的项单独列出，禁止猜。

## 档位

Critical / Important / Minor。发现与 brief 原文冲突时（plan-mandated），不自行裁决，列出并交编排者。

## 报告

写到 `.superpowers/sdd/task-rot1-review.md`：双判决、逐条证据、独立验证结果、findings（带档位）、god-file T0 基线三段。最终消息以 APPROVED / REJECTED 开头。
