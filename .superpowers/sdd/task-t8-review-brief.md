# Review request — T8 competition groups

## Review package

- Source worktree: `E:\agent-project\northing\.worktrees\growth-core-0804`
- Base: `8b64aa8` (use this exact BASE; do not substitute `HEAD~1`)
- Head: `99d82dd`
- Implementer brief: `E:\agent-project\northing\.superpowers\sdd\task-t8-brief.md`
- Implementer report: `E:\agent-project\northing\.superpowers\sdd\task-t8-report.md`
- Diff: `E:\agent-project\northing\.superpowers\sdd\task-t8-diff.patch`
- Write review only to: `E:\agent-project\northing\.superpowers\sdd\task-t8-review.md`

This is a 1004-line, 9-file change. Read the full files in the source worktree and inspect
the actual base-to-head diff. Do not edit source, commit, dispatch another agent, or change
the plan/ledger/model notes. Do not rerun the implementer's already-reported test suite;
review semantics and evidence instead. If a report claim is not supported, mark it explicitly.

## Required independent verdicts

Start with `SPEC PASS/FAIL` and `QUALITY PASS/FAIL`. Both are required. Classify every finding
as Critical, Important, or Minor, with file and line. Only Critical/Important findings trigger
a fixer round.

## User ruling to enforce

The original `<0.2` absolute threshold is superseded. The live keyword-weight domain is
`[1.0,5.0]`, and suppression requires normalized share strictly `<0.15` AND live keyword
weight `<=1.0`. There is no second activity score and no restored `0.1` floor. A repeated
mention must raise the topic strictly above `1.0` and revive it. This decision must be present
in behavior, tests, parameter registration, and report.

## Spec review priorities

1. **Persistence and migration**: verify the new schema is created on every open, old DBs are
   non-destructively upgraded, group member identity is stable, full replacement is atomic,
   empty-group behavior is intentional, and all T9 metadata (evidence/source/timestamps)
   round-trips without loss.
2. **Port boundary**: verify the growth crate remains IO-free and core owns SQLite access;
   `CompetitionGroupStore` is minimal, object-safe, and does not accidentally remove a
   contract needed by T9/T10. Confirm the host adapter is the only concrete storage path.
3. **Normalization and mutation**: verify every group boost goes through the reviewed pure
   normalization, preserves all members, clamps delta, handles malformed/zero groups, and
   cannot double-boost or double-decay a finalized turn. Check behavior when a topic belongs
   to multiple groups, when a group is absent, and when a weight row is absent.
4. **Suppression in retrieval**: inspect the exact `search_facts` flow. Confirm suppression is
   visibility-only, does not change fact status/delete rows, applies both gates with exact
   boundary semantics, does not suppress ungrouped facts, does not suppress a fact when any
   independent matching topic remains warm, and does not make a malformed storage read fail
   the main path.
5. **Revival**: verify a repeated mention actually changes the live weight above `1.0` and
   that a previously suppressed topic becomes retrievable without rewriting its fact/group
   identity. Confirm the tests prove the production path rather than only pure helpers.
6. **Warn-only and security**: all storage failures are swallowed/logged at the host edge;
   no `supersede`, status mutation, deletion, LLM, or self-cognition access was introduced;
   logs/source are English-only and no emoji were added.

## Quality review priorities

- Look for SQL transaction/locking mistakes, partial writes, stale shares, nondeterministic
  ordering, silent `unwrap`/error swallowing that loses durable data, and schema compatibility
  hazards.
- Check whether `search_facts` remains correct for empty queries, CJK/FTS matching, workspace
  scoping, multiple query terms, and the existing BM25/two-layer score path.
- Check whether the new storage module and tests stay under the production 800-line rule.
- Check whether report counts and claims match the diff and actual commit.
- Check if the implementation accidentally does T9/T10/T12 work or leaves a dead API that
  cannot support the next task.
- Do not report formatting-only nits or re-litigate the user's threshold decision.

## Global constraints (exact)

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
- Coding curfew: 03:00 后不派实现单。

## Output

Structure the review as:

1. Two verdicts up front.
2. Findings ordered Critical → Important → Minor, each with file/line and evidence.
3. A short spec coverage matrix for the eight required test behaviors.
4. `Cannot verify from diff` for anything unresolved, without guessing.
5. Residual risk and whether a fixer round is required.
