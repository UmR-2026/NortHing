# Review Brief: Task T2-1（CI 补齐）

## 审查对象
未提交工作区改动，限以下 3 个文件（工作区另有 2 个并行 session 的既有改动 ` .opencode/model-capability-notes.md` / `memory/northhing.md`，**不在审查范围**）：

```powershell
git -C E:\agent-project\northing diff -- .github/workflows/ci.yml docs/architecture/backend-roadmap.md docs/status/tech-debt-ledger.md
```

diff 快照：`E:\agent-project\northing\.superpowers\sdd\task-t2-1-diff.md`
实现报告：`E:\agent-project\northing\.superpowers\sdd\task-t2-1-report.md`

## 需求来源（spec 合规判据）
`E:\agent-project\northing\.superpowers\sdd\task-t2-1-brief.md` —— 以其中 Required changes（3 条）为逐条核对清单。

## Constraints（逐字摘自任务书，逐条核对）
- 不新增、不删除、不跳过任何既有测试；不为让 CI 变绿而给测试加 #[ignore]——发现 OS 相关失败如实上报（NEEDS_CONTEXT 或 DONE_WITH_CONCERNS 列出清单）。
- 不动 `.github/workflows/` 里其它文件；不动 GitHub 仓库设置。
- i18n-contract 预存失败（24 个）**不在本单范围**（i18n engineering 冻结中，CI 无 i18n job）；不要试图在本单修它。

## 双判决要求
1. **spec 合规**：Required changes 3 条 + Constraints 3 条逐条 PASS/FAIL。
2. **代码质量**：ci.yml YAML 结构正确性（缩进/条件表达式/matrix 语义）、`if: matrix.os == 'ubuntu-latest'` 在 GitHub Actions 的合法性与实际效果、文档修改的准确性（roadmap/ledger 新表述是否与 ci.yml 实际内容一致）。

## 特别核对点（不预判结论，仅提示必须亲自打开证据）
- ci.yml 改动后的完整 job 结构：`git diff` 之外应 `read` 该文件改动段上下文，确认 step 拼接无断裂。
- tech-debt-ledger.md P2-15 翻 resolved 是否合规：家规 2「解债→同 commit 翻 ledger 状态」；核对 P2-15 条目原文，判断 T2-1 是否足以关掉它（注意区分"代码缺陷"与"流程门"两部分）。
- 报告中声称 `cargo check --workspace` PASS——可抽查 target 目录新产物或用 `git log` 之外手段佐证；报告文字本身不构成证据。
- 报告 §3 测试盘点表（约 2507 测试）属于调查性内容，抽查 2-3 个 crate 的 `#[test]` 计数是否合理即可，不必全量复核。

## 输出
写 `E:\agent-project\northing\.superpowers\sdd\task-t2-1-review.md`，按模板：Strengths / Issues（Critical/Important/Minor，每条 file:line + 为什么重要 + 怎么修）/ Recommendations / Assessment（双判决分行给出：spec-compliance: PASS/FAIL；code-quality: PASS/FAIL；ready-to-merge: Yes/No/With fixes）。遵循 Evidence Discipline 与 Fail-closed verdict 条款（Cannot verify from diff > 0 则不得给 Yes，逐条列出）。
