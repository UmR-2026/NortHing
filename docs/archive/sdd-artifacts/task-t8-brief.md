# T8 — competition groups and natural suppression

## Position and baseline

- Implement only in `E:\agent-project\northing\.worktrees\growth-core-0804`.
- Branch must remain `feat/growth-core-0804`; verify `git rev-parse --show-toplevel` before editing.
- Baseline is `8b64aa8`; do not modify the main worktree source.
- Commit the implementation on the growth-core branch.
- Write the implementation report only to `E:\agent-project\northing\.superpowers\sdd\task-t8-report.md`.
- Write the complete diff only to `E:\agent-project\northing\.superpowers\sdd\task-t8-diff.patch`.
- Do not dispatch child agents. Do not edit the ledger, plan, handoff, or model notes.

## Scope

Implement G2-T8: persist competition groups, apply group normalization when a topic is
boosted, and make naturally suppressed topics stop surfacing while preserving their data and
allowing later revival. The existing pure module
`src/agentic/src/topics/competition.rs` is the starting point, not a reason to skip the task.
The existing `TopicStore` trait in `src/agentic/src/ports.rs` is also a starting contract; it
currently has no host implementation.

Do not implement T9's LLM proposal/evidence accumulation, T10 merge/dedup, T11 negation,
T12 garden rewrite, or T4c facade. Do not add any hard-retirement/supersede behavior.

## User decisions and resolved conflict

The original plan said suppression requires normalized share `<0.15` and absolute weight `<0.2`.
The user decided on 2026-08-07:

- retain both suppression gates;
- interpret absolute weight as the existing `keyword_weights.weight` cold baseline;
- suppress only when share is strictly `<0.15` and keyword weight is `<=1.0`;
- use one named constant for the `1.0` cold baseline and register it in
  `src/agentic/AGENTS.md` §4;
- do not add a second activity/heat score and do not lower the decay floor to `0.1`;
- a repeated mention raises a topic strictly above `1.0` and is the revival signal.

This decision supersedes the old `<0.2` acceptance value. Preserve the old pure-function API
only if doing so does not make the live behavior unreachable; otherwise update it and its tests
to the decided domain, with the reason documented in the report.

## Required behavior

### Competition data

Add the smallest durable representation needed for a group: group id, member topic, current
normalized share, evidence/source metadata needed by T9, and created/updated timestamps. Use
the existing SQLite `MemoryDb` and its `CREATE TABLE IF NOT EXISTS` startup batch; support old
databases without a destructive migration. Do not create a second database or a second topic
weight table.

The host adapter must implement the existing `TopicStore` group methods or a narrowly
equivalent port adjustment. Keep storage errors warn-only at the growth host boundary. Do not
invent an LLM or hardcoded group proposal source in T8: T9 owns proposal and evidence logic.
If no groups exist in a fresh database, normal topic behavior remains unchanged.

### Normalization and boost

- Group shares always sum to 1 within the existing epsilon, including zero-sum and malformed
  input handling covered by the pure module.
- Boosting a member raises its share and squeezes every sibling; no member is deleted.
- Boost delta remains capped at `0.15` and invalid values remain safe no-ops.
- Boosting a topic outside a group must preserve current non-group keyword-weight behavior.
- Do not double-boost or double-decay a topic in one finalized turn.
- A later boost of a suppressed topic must revive it without reconstructing or deleting facts.

### Retrieval suppression

Use both gates: group share `<0.15` and live keyword weight `<=1.0`. Equality at either
threshold is active unless the other gate is strictly satisfied according to the selected
comparison. Suppression must not delete facts, change fact status, or call any hard-retirement
API. It should affect retrieval visibility/priority through the existing search path, and must
not make unrelated or ungrouped facts disappear.

If the current fact schema cannot associate a fact with topics without introducing an
unreviewed second truth source, use the existing keyword/FTS relation and document the exact
association and its limits. Do not silently claim stronger semantics than the storage proves.

## Required tests

Add focused tests at the nearest existing locations. At minimum prove:

1. group schema is present after opening a fresh DB and remains usable after reopening an old
   DB;
2. shares sum to one, boost rise causes sibling fall, zero division is safe, and malformed
   values are handled safely;
3. cold baseline `<=1.0` plus share `<0.15` suppresses, exact boundary behavior is pinned, and
   a weight `>1.0` remains active even with a low share;
4. suppressed topics remain stored and a later boost revives them;
5. an ungrouped topic follows the existing boost/decay path unchanged;
6. no fact row is deleted or hard-retired by group maintenance;
7. persistence round-trip preserves group membership, shares, source/evidence metadata, and
   timestamps;
8. malformed/empty groups do not panic and storage failures remain warn-only where the host
   adapter is involved.

If a requested end-to-end retrieval assertion cannot be made from current fact/topic data,
state that limitation explicitly in the report and provide the strongest hermetic test the
current schema supports. Do not make a test pass by adding an unproven fixture-only shortcut.

## Boundaries and constraints

Copy these global constraints verbatim into the report's compliance section:

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

Also obey repository and nearest `AGENTS.md` files, especially the growth crate permission
matrix and core decomposition rules. Use English-only source comments and logs.

## Verification required in report

The implementer must run and report commands plus relevant output, with the required PATH
prefix:

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo test -p northhing-core --features product-full memory_db
cargo test -p northhing-core --features product-full growth_adapter
cargo check -p northhing-core --features product-full
node scripts/check-core-boundaries.mjs
```

Run `cargo check -p northhing` too if the changed dependency/API surface reaches desktop
assembly. Report exact counts, warnings, and any environmental blocker. Do not run `cargo fmt`.

## Report requirements

Report:

- exact files and symbols changed;
- the schema and migration strategy;
- how the live `[1.0,5.0]` topic weight and cold-baseline suppression semantics are enforced;
- how ungrouped topics and existing search behavior are preserved;
- test commands and output;
- any deviation, unresolved limitation, or `Cannot verify from diff` item with evidence.
