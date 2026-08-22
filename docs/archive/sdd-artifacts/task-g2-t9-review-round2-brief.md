# T9 Review Round 2

## Package

- Source worktree: `E:\agent-project\northing\.worktrees\growth-core-0804`
- Exact BASE: `aa53f35`
- Exact HEAD: `67a6947`
- Original brief: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-brief.md`
- Fix brief: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-fix-brief.md`
- Round 1 review: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-review.md`
- Final implementer report: `E:\agent-project\northing\.superpowers\sdd\task-t9-report.md`
- Final full diff: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-diff.patch`
- Write review only to: `E:\agent-project\northing\.superpowers\sdd\task-g2-t9-review-round2.md`

Review the entire `aa53f35..67a6947` range, not only fixer commits `0efeb29` and `67a6947`.
The source worktree is read-only. Do not edit source, commit, dispatch child agents, or rerun
the implementer's reported tests.

## Round 1 Important Findings To Close

Give an independent `CLOSED` / `OPEN` decision with file:line evidence for each:

1. **I1 stale pre-sweep state:** every confirmation must plan against live DB members, so two
   overlapping confirmations leave each topic in exactly one group and rollback-then-confirm
   cannot recreate the rolled-back group. Inspect production ordering and both new host tests.
2. **I2 destructive group-id collision:** a live id with a different member set must cause a
   warned rejection before any group write or confirm audit. Verify original rows/metadata remain
   and the regression test is not a tautology.
3. **I3 module-tree boundary hole:** production `competition_review.rs` must retain all eleven
   bans; `competition_review_tests.rs` must have an exact-file rule with all ten self-cognition
   bans and an explicitly documented `conn_locked` test-only exemption. No `allowPaths`.
4. **I4 incomplete trigger proof:** the report must include all eleven production failures, all
   ten test-file failures, a clean checker run, and no committed planted text or proof artifact.

## Additional Fixer Checks

- The final test module now contains ten host tests: seven original plus
  `two_overlapping_confirms_leave_topics_in_one_group`,
  `rollback_then_confirm_does_not_recreate_rolled_back_group`, and
  `reject_live_group_id_collision`. The report gives a real result line of 10 passed. Confirm
  the tests exercise the production `apply_competition_sweep` path and exact final DB/audit state.
- Commit `67a6947` corrected the initial versions of the I1/I2 tests. Inspect the final tests,
  not the first fixer versions.
- Worktree cleanup was independently observed after removal of untracked `boundary_errors.txt`;
  verify no proof artifact is in the commit range.
- `memory_db.rs` and `memory_db_tests.rs` must remain untouched at 999 / 1098 lines.
- `git diff --check` still reports trailing whitespace in propose.rs, route.rs, and the host test
  file. It was Round 1 Minor M1; keep or reclassify with evidence, but do not silently omit it.
- Report hygiene remains suspect: its baseline table says `competition_review: 10` although the
  pre-T9 baseline was 0; current line counts appear stale (`competition_review_tests.rs` is 353,
  not 350; verify forbidden-rules.mjs and agent_memory/mod.rs too); its final deviation still says
  cargo check timed out while the verification block records a completed 2m04s run. Classify
  these as report/spec evidence issues separately from source correctness.

## Regression Review

Confirm the fixer did not regress previously-passing requirements:

- Evidence threshold exactly 3, same-set accumulation, one evidence per set per sweep, and
  workspace-isolated pending state.
- Confirmed groups remain global without schema change.
- Double-emission at threshold still produces three propose audits plus one confirm audit.
- Rollback preserves facts and keyword weights and restores visibility.
- Prompt/parse whitelist, bad-JSON zero action, group-id sanitation, member caps, rationale
  truncation, pure crate logic, and T8 full-replacement persistence remain intact.
- No self-cognition access, `supersede`, hard retirement, T10/T11/T12/T4c overreach, or second
  `strip_json_fence` appears.
- Every touched production Rust file stays below 800 lines; surfaces.md remains correctly
  unchanged or any required update is identified.

## Global Constraints

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

Begin with exactly `SPEC PASS/FAIL` and `QUALITY PASS/FAIL`. List Round 1 I1-I4 closures first,
then any new Critical/Important findings, then residual Minor items for final triage. Every
finding needs file:line evidence. Explicitly state whether the full fixer round is closed. Use
`Cannot verify from diff` for reported test/check outputs rather than guessing.
