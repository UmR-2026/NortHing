# T9 Review Package

## Package

- Source worktree: `E:\agent-project\northing\.worktrees\growth-core-0804`
- Exact BASE: `aa53f35`
- Exact HEAD: `5d85c13`
- Original brief: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-brief.md`
- Implementer report: `E:\agent-project\northing\.superpowers\sdd\task-t9-report.md`
- Full diff: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-diff.patch`
- Write review only to: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-review.md`

Review the complete `aa53f35..5d85c13` range, including implementation commit `bc2012b`
and fixer commit `5d85c13`. The source worktree is read-only for review. Do not edit source,
commit, dispatch child agents, or rerun the implementer's reported tests. Use the report as
test evidence; independently inspect the code and diff.

## Prior Fixer Findings

The first implementation report claimed `185 passed`, but an independent run found a
deterministic failure in `review::propose::tests::evidence_accumulates_and_confirms`:
the third sweep emitted two decisions while the test asserted one. The fixer changed the
test to require exactly one `ProposeAccepted { evidence: 3 }` and one `Confirm`, and claimed
the host audit test asserts exactly three `propose_competition` rows plus one
`confirm_competition` row. Independently verify both the production semantics and these exact
assertions. Do not accept the report's prior false output without source evidence.

The fixer also restored the accidental four-space indentation of the existing
`load_competition_share_map` function. Confirm the final diff leaves only the intended
`list_top_keyword_weights` addition in `competition_groups.rs`.

## Spec Review

Verify against the original brief and the plan T9 text, including the user rulings:

- Evidence threshold is exactly `N=3`; same normalized member set counts even if the LLM
  changes `group_id`; one sweep contributes at most one evidence for a set.
- Evidence is isolated by workspace through `judge_mom` KV keys; proposals from different
  workspaces cannot reach confirmation together.
- Confirmed groups are global and must not gain a `workspace_key` column.
- Confirm confirmation uses forced single membership: confirmed topics are removed from every
  other group, remaining groups are renormalized, and groups with fewer than two members are
  dissolved.
- Rollback is single-shot, removes only the competition group rows, preserves facts and
  keyword weights, and writes the required audit row.
- Audit rows use reviewer `judge-mom`, actions `propose_competition`, `confirm_competition`,
  and `rollback_competition`; the synthetic `competition:<group_id>` fact id is documented
  and consistently applied.
- Bad JSON and invalid proposal fields produce zero actions; topic whitelist, member count,
  group id validation, deduplication, and char-based rationale truncation are real gates.
- The host routine is warn-only, has a cadence gate, does not block the turn path, and does
  not read or write self-cognition.
- T10 merge/dedup, T11 negation, T12 garden rewrite, and T4c facade are not implemented early.
- T8's `CompetitionGroupStore`, `CompetitionMember`, explicit group id, metadata, and
  full-replacement save are reused rather than duplicated.

## Quality Review

Inspect all ten changed files and focus on these risks:

1. `review/propose.rs`: pending-state transitions, duplicate proposals, confirmation ordering,
   rollback behavior, malformed JSON handling, and whether the double-emission design can
   duplicate or lose audit rows.
2. `review/route.rs`: preservation of metadata and timestamps, normalization after re-slot,
   empty/single-member group handling, duplicate topic inputs, and deterministic output.
3. `competition_review.rs`: cadence/error paths, pending-state persistence ordering, LLM timeout
   behavior, workspace key construction, audit failure handling, and absence of raw connection
   access or self-cognition access.
4. `competition_groups.rs`: SQL ordering, top-weight query limits, existing T8 retrieval helper
   equivalence, and the absence of whitespace-only churn after the fixer.
5. `turn_persist_facts.rs`: call placement relative to facts, episode/facts behavior, and
   whether the new sweep can cause an unexpected duplicate or nested path.
6. `forbidden-rules.mjs`: exact path semantics, complete self-cognition/`conn_locked` coverage,
   and whether the planted violation proof actually targets the new production file.
7. `competition_review_tests.rs`: tests are not fixture-only tautologies, exact audit counts are
   asserted, cross-workspace isolation is real, rollback proves visibility recovery, and the
   mutex-poisoning test cannot contaminate other tests.

Check the following non-negotiable capacity facts from the diff/report:

- `memory_db.rs` remains exactly 999 lines and is not changed.
- `memory_db_tests.rs` remains exactly 1098 lines and is not changed.
- Every touched production Rust file remains below 800 lines.
- `docs/status/surfaces.md` is correctly unchanged or the report gives a file-based reason.
- No emoji, `supersede` semantics, or new hard-retirement path appears in the review route.
- `git diff --check` cleanliness must be assessed. The orchestrator observed trailing-whitespace
  diagnostics in the implementation range; classify their severity and identify exact files
  and lines if still present.

## Global Constraints

Copy and assess these plan constraints verbatim:

- 成长路径**永远 warn-only**：失败只 `tracing::warn!`，绝不向 `turn_persist` 传播、绝不阻塞主流程。
- **judge-mom 无作废权**：唯一硬作废入口是 `negation.rs`（D8）；园丁/评审路径出现 `supersede` = 违规（边界脚本拦）。
- **管家对自我认知库无权**（D9）：编译期不可见 + 负向测试 + 边界规则三重保证。
- 权重系统三道闸：组内归一化、单次 boost 上限、越界钳制；所有参数集中在 crate 常量并记入 crate AGENTS.md（禁散落魔法数）。
- LLM 输出不可信：严格 JSON + 字段白名单 + 长度截断（text ≤300 / reason ≤200）+ 用户内容包 `<user_message>`、指令只认 system。
- 配置单一事实源 = core `GlobalConfig`（`service/config/memory.rs`）；禁第二份运行时可读配置。
- 决策纯函数、IO 只在 executor/adapter；crate 自测零磁盘零网络。
- 生产 `.rs` < 800 行；>1000 必须拆或带 `// allow-god-file` 理由。
- 日志 English-only、无 emoji（gemini-36-flash 有 emoji 惯性前科 → 交付后机械扫描）。
- **不裸 `cargo fmt`**（两次污染前科）；用 `pnpm run fmt:rs` 或手工对齐。
- cargo 命令带 `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`；core 测试必带 `--features product-full`。
- 远程 workspace：读侧 query-aware 注入已跳过远程，写侧沿用现状（设计稿 M4 已知限制），本轮不扩大。
- implementer 只 commit 范围内文件；crate 结构变动同 commit 更新 `docs/status/surfaces.md`。
- Coding curfew：03:00 后不派实现单。

## Required Output

Begin with exactly `SPEC PASS/FAIL` and `QUALITY PASS/FAIL`. Give separate decisions for
spec compliance and code quality. Findings must be ordered Critical, Important, Minor and
include file and line references. Explicitly state whether the fixer round is closed. List
residual Minor items for final triage if no Critical/Important remain. Use `Cannot verify from
diff` for facts that cannot be established without rerunning tests; do not guess. Every claim
about a mechanism must include file:line evidence.
