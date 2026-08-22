# Review — Task T7a (boundary rules for the permission matrix)

You are reviewing ONE commit on branch `feat/growth-core-0804`.

- Repo: `E:\agent-project\northing` (main checkout) / worktree `E:\agent-project\northing\.worktrees\growth-core-0804`
- BASE: `9f261cd`  HEAD: `9a9fb8a`
- Diff: `E:\agent-project\northing\.superpowers\sdd\task-t7a-diff.md`
- Spec (the ONLY source of requirements): `E:\agent-project\northing\.superpowers\sdd\task-t7a-brief.md`
- Implementer report: `E:\agent-project\northing\.superpowers\sdd\task-t7a-report.md`

Read the brief in full before the diff. Read files from the **worktree** path above.

## Two verdicts required (both must be explicit)

1. **Spec compliance** — does the commit do exactly what the brief §3 requires, no more, no less?
2. **Code quality** — is it correct, honest, and maintainable?

## What this task is

It adds three boundary-rule groups to `scripts/core-boundaries/rules/source/forbidden-rules.mjs`
and relocates one Rust test. It deliberately changes **no production behavior**.

## Constraints copied verbatim from the plan's Global Constraints

- Rust production files must stay under 800 lines.
- `cargo fmt` is forbidden in this project.
- English-only, no emoji, in code and comments.
- Core tests must run with `--features product-full`.
- Warning policy is warn-only; the core warning baseline is **19** and must not increase.
- `northhing-core`'s `lib.rs:3-4` carries `#![allow(dead_code)]` and `#![allow(unused_imports)]`,
  so an unchanged warning count does NOT prove absence of dead code — judge dead code by reading.

## Highest-value things to check (in priority order)

1. **Do the rules actually constrain anything?** A pattern with a misspelled symbol, or a
   `path` that does not cover the intended file, is a silent no-op that looks like
   enforcement. Cross-check every regex's symbol against the real source. In particular
   confirm the symbol names exist as written: `load_self_cognition`, `append_self_cognition`,
   `count_self_cognition`, `migrate_identity_into_self_cognition`, `resolve_identity_path`,
   `SelfCognitionRow`, `SelfCognitionDbStore`, `init_self_cognition_store`,
   `SelfCognitionStore`, `conn_locked`, `set_blob`, `boost_keyword`, `insert_fact`,
   `supersede_fact`.
2. **Rule-list placement semantics.** Groups A and B were added to `forbiddenContentRules`
   (exact-file paths) and group C to `forbiddenContentUnderRules` (directory subtree). Verify
   against `scripts/core-boundaries/checker.mjs` that each group is in the list whose matching
   semantics it needs. A directory rule placed in the exact-path list (or vice versa) would
   silently match nothing.
3. **Is the relocated test weaker than before?** `dream_payload_never_contains_self_cognition_sentinel`
   moved from `dream.rs` into the new `dream_d9_tests.rs`. It must be a pure relocation: same
   name, same sentinel, and BOTH assertions — the "sentinel absent" assertion AND the
   "fact text present" anti-vacuity assertion. Losing the second one would turn the test
   into one that passes even if the payload were empty.
4. **Is the `allowPaths` exception minimal and honest?** Group C whitelists
   `system_prompt_tests.rs` for the `SelfCognitionDbStore` pattern only. Check that no
   whitelist is broad enough to void its own rule (e.g. whitelisting `dream.rs` inside a rule
   whose whole purpose is constraining `dream.rs`), and that the recorded exception for
   `init_self_cognition_store` in the group `reason` describes reality.
5. **Scope discipline.** Only these files may change: `forbidden-rules.mjs`, `dream.rs`,
   and the new `dream_d9_tests.rs`. Any production behavior change, any edit to
   `checker.mjs` / `self-test.mjs` / other rule modules, or any `supersede`-related rule
   (explicitly deferred to T12 by brief §2.3) is a finding.
6. **Rule messages.** Each `message` should tell a future author the invariant and what to do
   instead, not merely that they tripped a lint. Weak or copy-pasted-wrong messages are Minor.

## Already independently verified by the orchestrator — do NOT re-run these

- `node scripts/check-core-boundaries.mjs` exit 0 on the committed tree.
- Test counts: dream 7, self_cognition 19, system_prompt 21, memory_db 28, auto_memory 7,
  growth_adapter 30, `northhing-agentic-growth` 139. Core warnings 19.
- The relocated sentinel string is byte-identical to the pre-move version.
- Rule firing spot-checks: injecting `load_self_cognition` + `conn_locked` into
  `judge_memory.rs` fails the checker with the intended messages; injecting
  `SelfCognitionDbStore` + `boost_keyword` into `system_prompt.rs` fails too, proving the
  `allowPaths` does not exempt the production file. Worktree is clean.

Review by reading. Do not re-run the suite; the report plus the above is your evidence base.

## Output format

For each finding: **Critical / Important / Minor**, the `file:line`, what is wrong, why it
matters, and the smallest correct fix. If you cannot determine something from the diff alone,
put it under a separate heading **"Cannot verify from diff"** rather than guessing — do not
state that a file or symbol does not exist unless you actually opened the path and confirmed.

End with the two verdicts, each as `PASS` or `FAIL`, on their own lines:
`SPEC: PASS|FAIL`
`QUALITY: PASS|FAIL`
