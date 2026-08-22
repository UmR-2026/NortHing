# Review Brief: Task T2-2a（死代码删除第一批，≈11.3k rs 行）

## 审查对象
未提交工作区改动，BASE = `e65d98e`（main HEAD）。本批改动即全部 working-tree diff（141 文件，+160/-11,960；其中 `.opencode/model-capability-notes.md`、`memory/northhing.md` 两个文件是并行 session 既有改动，**不在审查范围**——`git diff` 时用 pathspec 排除或直接读 diff 快照文件）。

```powershell
git -C E:\agent-project\northing diff -- . ':(exclude).opencode/model-capability-notes.md' ':(exclude)memory/northhing.md'
```

diff 快照：`E:\agent-project\northing\.superpowers\sdd\task-t2-2a-diff.md`（含全部改动，自查时排除上述两文件段）
实现报告：`E:\agent-project\northing\.superpowers\sdd\task-t2-2a-report.md`
需求来源（spec 判据）：`E:\agent-project\northing\.superpowers\sdd\task-t2-2a-brief.md`（删除清单 D1-D6 + Constraints + Verification）
侦察附件：`E:\agent-project\northing\.superpowers\sdd\task-t2-2a-recon.md`

## Constraints（逐字摘自任务书，逐条核对）
- 不 commit、不 push（编排者统一收口）；改动留在工作区
- 文档同步硬规则：crate 删除与 surfaces.md / 各 AGENTS.md / boundary 规则文件的同步必须在**同一工作区改动集**里
- 排除项勿碰：`tool-provider-groups`、`harness`、`judge_gate`、`remote_connect`、`mobile-web`、`miniapp`、`relay-*`、`tests/e2e/`
- 勿碰并行 session 资产：`memory/`、`.graph/`、`.opencode/model-capability-notes.md`、`.superpowers/sdd/` 里其它 task-* 文件、前端 session 相关文件
- 若某项复核发现实际有引用：**跳过该项**，报告标注，不要强行删

## 双判决要求
1. **spec 合规**：D1-D6 逐项（删了什么/同步了什么/复核 grep 是否真跑了）+ Constraints 5 条。
2. **代码质量**：这是纯删除+配置同步批。重点是**完整性**（该删的删干净了吗——残留引用/残留声明）与**不多删**（排除项、活代码、pulldown-cmark 保留）。

## 特别核对点（亲自打开证据，不预判结论）
- **编译门禁声明核实**：报告称 `cargo check --workspace` PASS（3m54s）。你可独立复跑 `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace`（增量应 <1min）或至少核查 target 新产物时间戳；报告文字不构成证据。
- **boundary 规则同步完备性**：`scripts/core-boundaries/rules/` 四文件（crate-layout.mjs / crate-rules.mjs / feature-rules.mjs / self-test.mjs）——webdriver 条目是否删干净；self-test.mjs 是否引用了被删 crate 的 manifest 原文（侦察 §2 曾指出 self-test 硬编码 tool-packs manifest——tool-packs 本批**保留**，但若 self-test 也有 webdriver/pcc 相关断言则需同步）；复跑 `node scripts/check-core-boundaries.mjs` 确认绿。
- **残留引用扫描**：`rg -i "webdriver" src scripts --glob '!**/target/**'`、`rg -i "insights" src/crates --glob "*.rs"`、`rg "plan.compliance"` 全仓——判定残留命中是"无关语义"还是"漏删"。
- **排除项完整**：tool-provider-groups / harness / judge_gate / remote_connect / mobile-web / miniapp / relay-* / tests/e2e 全部零改动（diff 应无命中）。
- **活代码安全**：`src/crates/assembly/core/src/agentic/session/`（8.6k 行活代码）未被误删；`src/apps/cli/Cargo.toml:43` pulldown-cmark 引用仍有效（根 workspace 声明保留）。
- **AGENTS-CN.md 同步**：implementer 主动同步了根 AGENTS-CN.md 与 adapters/AGENTS-CN.md（brief 未列，属顺手清配额精神）——核对中文镜像与英文版改动一致、无错删。
- **Cargo.lock**：删除 crate 后 lock 已再生（`cargo metadata` PASS 佐证）；lock diff 中被移除的包是否确为被删 crate 及其独有依赖（抽查 2-3 个）。
- **dev.cjs / copy_reference.cjs**：编辑后的脚本语法完整性（node --check 或结构阅读）。
- 报告行数对账（11,331 rs 行净删）与 diff --stat 是否吻合。

## 输出
写 `E:\agent-project\northing\.superpowers\sdd\task-t2-2a-review.md`，按模板：Strengths / Issues（Critical/Important/Minor，每条 file:line + 为什么重要 + 怎么修）/ Recommendations / Assessment（spec-compliance: PASS/FAIL；code-quality: PASS/FAIL；ready-to-merge: Yes/No/With fixes）。遵循 Evidence Discipline 与 Fail-closed verdict（Cannot verify from diff > 0 不得给 Yes，逐条列出）。
