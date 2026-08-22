# T9 — competition recognition (judge-mom LLM proposal + evidence + rollback)

## Position and baseline

- Implement only in `E:\agent-project\northing\.worktrees\growth-core-0804`.
- Branch must remain `feat/growth-core-0804`; verify `git rev-parse --show-toplevel` before editing.
- Baseline is `aa53f35`; do not modify the main worktree source.
- Commit the implementation on the growth-core branch.
- Write the implementation report only to `E:\agent-project\northing\.superpowers\sdd\task-t9-report.md`.
- Write the complete diff only to `E:\agent-project\northing\.superpowers\sdd\task-t9-diff.patch`.
- Do not dispatch child agents. Do not edit the ledger, plan, handoff, or model notes.

## Scope

Implement G2-T9 (plan §5, D7 hybrid competition recognition): a judge-mom routine that asks
the in-process LLM to propose competition relations between topics, accumulates consistent
evidence across sweeps (`N=3`, same workspace), puts confirmed groups into effect, and supports
rollback. Every step writes audit rows to `fact_reviews` with `reviewer="judge-mom"` and actions
`propose_competition` / `confirm_competition` / `rollback_competition`. Bad JSON means zero actions.

Do not implement T10 merge/dedup, T11 negation, T12 garden rewrite, or the T4c facade. Do not add
any hard-retirement/supersede behavior. Do not modify `dream.rs` beyond what the boundary-rule
ritual requires (it must remain byte-identical in its production body).

## User decisions (2026-08-07, fixed, do not reopen)

1. **Cross-group convergence = forced single membership at confirm time.** When a group is
   confirmed, each of its members is removed from every other pre-existing group. A group thus
   reduced to fewer than 2 members is dissolved (full-replacement save with an empty member
   list). Remaining members of a shrunk group are renormalized to sum 1.0. From then on "one
   topic belongs to at most one group" is a system invariant. The T8 defenses (write side first
   group by `group_id` order in `growth_adapter.rs`, read side max share in
   `load_competition_share_map`) stay in place as unreachable-in-steady-state defenses; do not
   remove or "simplify" them.
2. **Evidence accumulates per workspace; confirmed groups take effect globally.** Pending
   evidence is keyed per workspace (`judge_mom` KV); identical proposals from different
   workspaces must not add up. Confirmed groups carry no workspace key and take effect globally,
   consistent with the shipped T8 table and the global `keyword_weights`. Do not add a
   `workspace_key` column. Do not count evidence by parsing `fact_reviews` rows (free-text
   `reason` is fragile); `fact_reviews` is the audit trail only.

## Architecture (fixed by pre-check scouting; deviate only with a documented reason)

### Crate side — `src/agentic/src/review/`

Fill the T1 shell `route.rs` and add one new module `propose.rs`. All pure: no IO, no logging,
no `SystemTime::now()` (timestamps are parameters). Reuse `llm_output::strip_json_fence` (the
single copy — do not create a third one) and `review::verdict::MAX_REASON_CHARS`.

`propose.rs`:

- Neutral input types (no core `Fact`, no rusqlite): a topic snapshot `(name, weight)` list and
  an existing-group snapshot `(group_id, member topics)` list.
- `build_competition_prompt(topics, existing_groups) -> (system: String, user: String)`:
  system prompt carries the instructions; user content wraps the topic list (topics derive from
  user text) in `<user_message>` tags per the global injection-defense constraint. Cap the topic
  list the host will supply; the builder itself must also truncate defensively.
- `parse_competition_proposals(json, allowed_topics) -> Vec<CompetitionProposal>`:
  - Tolerate ```json fences via `strip_json_fence`; bad JSON returns an empty vec (zero actions),
    mirroring `parse_verdicts`' swallow-and-continue discipline.
  - Item shape: `{"action": "propose"|"rollback", "group_id": "...", "members": [...], "rationale": "..."}`
    (`reason` also acceptable for rollback; pick one canonical field name and document it).
    Unknown actions are skipped, never defaulted.
  - `propose` validation: members must be a JSON array of strings; each member must match the
    `allowed_topics` whitelist case-insensitively and is then replaced by the canonical
    whitelist string (anti-hallucination: the LLM may only reference topics it was shown);
    members deduplicated; a proposal with fewer than 2 or more than `MAX_PROPOSAL_MEMBERS`
    valid members is dropped entirely (not truncated).
  - `group_id` sanitized to lowercase ASCII `[a-z0-9-]`, max `MAX_GROUP_ID_CHARS`; if nothing
    valid remains, drop the proposal.
  - `rationale` truncated char-wise (not bytes) at `MAX_REASON_CHARS`; include a multibyte
    fixture test that locks char-based truncation (T5b M2 lesson).
- Evidence accumulation, pure and port-free:
  - `PendingProposal { member_set: Vec<String> /* sorted, deduped */, group_id: String,
    evidence: u64, rationale: String, first_seen_ms: u64, last_seen_ms: u64 }`, serializable
    (host persists it as JSON in the `judge_mom` KV).
  - Consistency key is the normalized member SET, not the `group_id` (the LLM may name the same
    set differently across sweeps; the latest sweep's sanitized `group_id` wins).
  - `record_proposals(pending, proposals, existing_groups, required, now_ms) -> Vec<ReviewDecision>`:
    - A `propose` whose member set already equals a live group's member set is a no-op (skip, no
      evidence, no audit emission from the decision list).
    - One sweep contributes at most one evidence per distinct member set (dedupe within batch).
    - Reaching `required` (`COMPETITION_EVIDENCE_REQUIRED = 3`) removes the pending entry and
      emits `ReviewDecision::Confirm { group_id, members, evidence }`.
    - `rollback` for a live `existing_groups` id emits `ReviewDecision::Rollback { group_id }`
      **immediately, single-shot** (rollback is the safe direction: it lifts suppression and
      deletes no data); unknown ids are skipped. Rollback needs no evidence threshold.
    - Cap the pending list at `MAX_PENDING_PROPOSALS` (evict least-recently-updated).
- New crate constants (register all in `src/agentic/AGENTS.md` §4 with meaning):
  `COMPETITION_EVIDENCE_REQUIRED = 3`, `MAX_PROPOSAL_MEMBERS = 6`, `MAX_GROUP_ID_CHARS = 40`,
  `MAX_PENDING_PROPOSALS = 32`.

`route.rs` (replace the shell; this is the "routing review verdicts into relation actions" the
T1 skeleton declared for T9):

- `plan_confirmation(all_existing_members: &[CompetitionMember], confirmed_group_id, confirmed_members,
  live_weights: &[(String, f64)], evidence: u64, source: &str, now_ms) -> Vec<(String /* group_id */, Vec<CompetitionMember>)>`
  — pure re-slot planner implementing user decision 1:
  - remove each confirmed member from every other group; renormalize shrunk groups with the
    existing `normalize` on their current shares; groups dropping below 2 members get an empty
    planned write (dissolve);
  - the confirmed group's initial shares come from `normalize` over the members' live keyword
    weights (missing weight = the `1.0` baseline); every planned member row carries
    `evidence_count = evidence`, `source = source`, correct created/updated stamps (preserve
    `created_at_ms` for rows that already exist in this group);
  - output is one full-replacement write per affected group (including the confirmed group and
    every dissolved group). No IO, no logging.

### Host side — core

New file `src/crates/assembly/core/src/service/agent_memory/competition_review.rs` (sibling of
`judge_memory.rs`; declared from `agent_memory/mod.rs`). Mirror `dream.rs::run_dream_sweep`'s
structure exactly (client resolution → open DB → cadence gate → load inputs → LLM call with
timeout → parse → apply → persist state), warn-only at every step:

- `pub(crate) async fn run_competition_review(workspace_root: &std::path::Path)`:
  - `resolve_memory_llm_client().await`; `None` = return.
  - Open `default_memory_db_path()`; on error warn + return (do NOT set the gate, same as dream).
  - Cadence gate: `judge_mom` KV key `competition_review_last_at` via the existing
    `get_judge_state`/`set_judge_state` helpers; interval constant
    `COMPETITION_REVIEW_INTERVAL_MS = 24 * 60 * 60 * 1000` (host-side, dream precedent).
  - Inputs: top `COMPETITION_REVIEW_TOP_K = 20` keyword weights (new query, see below), all
    competition-group members (existing `load_all_competition_members`), pending state JSON from
    `judge_mom` key `competition_pending:<workspace_key>` where `workspace_key =
    workspace_root.to_string_lossy()` (same derivation as dream). Malformed pending JSON: warn,
    treat as empty (never fail the sweep).
  - Fewer than 2 topics in the input: set the gate and return (nothing to judge).
  - LLM call with `tokio::time::timeout(Duration::from_secs(COMPETITION_REVIEW_LLM_TIMEOUT_SECS))`,
    `= 15` (dream precedent); on failure/timeout/empty text: warn, set the gate, return (dream's
    exact anti-hammering pattern).
  - Parse via the crate; empty parse = zero actions (still set the gate).
  - Apply decisions:
    - `Confirm`: run `plan_confirmation`, then for each planned write call
      `save_competition_group(group_id, members)` (an empty member list deletes the group — T8
      behavior, reuse it); per-write failure is warn-only and does not abort the remaining writes.
      Audit: one `confirm_competition` row. If the re-slot dissolved or shrank other groups,
      mention that in the confirm row's `reason` (<=200 chars).
    - `Rollback`: `save_competition_group(group_id, &[])` + one `rollback_competition` row.
    - Each accepted `propose` (new or reinforced pending) also gets one `propose_competition`
      audit row, so the evidence trail is human-reconstructible without reading the KV blob.
  - Audit rows use `FactReview { id: Uuid::new_v4(), fact_id: format!("competition:{group_id}"),
    reviewer: "judge-mom", action, reason: Some(...), created_at: now_ms }` via the existing
    `record_fact_review`. The synthetic `competition:` prefix is the convention for non-fact
    audit rows (the column is `NOT NULL`); document it in the module doc comment.
  - Persist pending state and the gate key; then one `info!` summary line (dream precedent).
- Call site: `turn_persist_facts.rs`, immediately after the existing `run_dream_sweep` call
  (currently around line 237), one line:
  `crate::service::agent_memory::run_competition_review(&workspace_path_buf).await;`
  plus the `pub(crate) use` re-export in `agent_memory/mod.rs`. This inherits the known
  "only on fact-producing turns" coupling; T12 owns decoupling — do not fix it here.

### The one new SQL query

`list_top_keyword_weights(&self, limit: usize) -> NortHingResult<Vec<(String, f64)>>` —
`impl MemoryDb` inside the existing access module `memory_db/competition_groups.rs` (it already
holds the T8 group SQL and already uses `conn_locked` internally; this keeps every raw-connection
use inside the two sanctioned access modules).

**Hard capacity constraints (from pre-check; violating them is a Critical):**
- `memory_db.rs` is at **999/1000** lines. Do not add, remove, or modify a single line of it.
- `memory_db_tests.rs` is at 1098 lines. Add **no** tests there.
- `growth_adapter.rs` (473) and `turn_persist_facts.rs` (385) have headroom but only get the
  changes described above.
- The new flow file `competition_review.rs` must not call `conn_locked` (see boundary rules
  below); all SQL goes through `MemoryDb` methods / the `judge_memory.rs` helpers.

### Boundary rules (T7a standard, mandatory)

Add a `forbiddenContentRules` group in `scripts/core-boundaries/rules/source/forbidden-rules.mjs`
for `src/crates/assembly/core/src/service/agent_memory/competition_review.rs` with the same
pattern set T7a put on `judge_memory.rs` (self-cognition symbol ban + `\bconn_locked\b` ban;
copy the T7a group's structure and adapt the messages). Prove every pattern fires: temporarily
plant a violation in the new file, run `node scripts/check-core-boundaries.mjs`, observe the
expected error, then restore. Record the proof in the report. Do not add `allowPaths`.

## Required tests

Crate (`cargo test -p northhing-agentic-growth`):
1. parse: valid mixed batch; bad JSON → empty; unknown action skipped; member not in whitelist
   dropped; case-insensitive member match substituted with the canonical topic; <2 and >6 member
   proposals dropped; duplicate members deduped; group_id sanitize/drop; rationale char-truncation
   with a multibyte fixture.
2. evidence: 2 consistent sweeps → no confirm; 3rd → `Confirm` with the latest group_id; same set
   under a different proposed group_id still accumulates; overlapping-but-not-equal sets accumulate
   separately; already-in-effect set is a no-op; pending cap eviction; rollback of a live group
   emits immediately; rollback of unknown id is a no-op.
3. route: re-slot removes members from other groups and renormalizes; <2-member leftover groups
   are dissolved (empty planned write); initial shares derive from live weights with the 1.0
   baseline for missing weights; pre-existing rows keep their `created_at_ms`; planned writes
   cover exactly the affected groups.

Host (`cargo test -p northhing-core --features product-full`, new test file
`competition_review_tests.rs` — NOT `memory_db_tests.rs`):
4. **证据不足不生效**: two sweeps' worth of identical proposals → no group rows exist.
5. **达阈值生效**: third identical sweep → group rows exist with `evidence_count = 3`, source
   `judge-mom`, and audit rows `propose_competition` ×3 + `confirm_competition` ×1.
6. **回滚恢复**: after a confirm makes a member suppressed (construct the shares/weights so the
   two-gate suppression fires via the existing helpers), a rollback deletes the group and the
   suppression helpers report the topic active again.
7. **跨 workspace 不串**: two identical proposals under workspace key A plus one under key B →
   no confirm; the two pending blobs are independent.
8. Re-slot end-to-end: topic in confirmed group G1; a different set containing the same topic
   reaches N=3 → G1 loses the topic (or is dissolved), the new group holds it, and
   `load_all_competition_members` shows the topic in exactly one group.
9. Bad LLM JSON → zero audit rows, zero group rows, gate still advanced.
10. warn-only: force a storage failure on one path (e.g. read-only DB handle pattern already
    used in existing tests, or the smallest hermetic fault you can support) and show the sweep
    completes without propagating an error. If a hermetic fault is not practically constructible,
    say so in the report instead of faking coverage.

Structure the host routine so decisions/pending/gate/apply are testable without a real LLM
(extract the post-LLM part into a synchronous inner function the tests can drive directly, the
way T5b extracted `apply_verdicts`). Do not make network calls in tests.

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
- Coding curfew：03:00 后不派实现单。

Also obey repository and nearest `AGENTS.md` files, especially the growth crate permission
matrix (judge-mom: no self-cognition, no hard retirement) and the core decomposition rules.
Use English-only source comments and logs.

## Verification required in report

Run and report commands plus relevant output, with the required PATH prefix, from the worktree:

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo test -p northhing-core --features product-full competition_review
cargo test -p northhing-core --features product-full growth_adapter
cargo test -p northhing-core --features product-full memory_db
cargo test -p northhing-core --features product-full prompt_injection
cargo check -p northhing-core --features product-full
node scripts/check-core-boundaries.mjs
```

Report exact test counts (including the pre-change baseline counts so the delta is visible),
warning counts (baseline is 19), and line counts of every touched file via `(Get-Content).Count`.
If `docs/status/surfaces.md` enumerates crate modules, update it in the same commit; otherwise
note that it does not. Do not run `cargo fmt`.

## Report requirements

Report:

- exact files and symbols changed, with line counts;
- the prompt shape (system + user skeleton) and the full parse/evidence/route decision tables;
- how each user decision is enforced and which tests prove it;
- the boundary-rule trigger proof (planted violation → checker output → restore);
- test commands and output;
- any deviation, unresolved limitation, or `Cannot verify from diff` item with evidence.
  All mechanism claims need file:line evidence; if you cannot verify something, write
  "not verified" instead of inferring it.
