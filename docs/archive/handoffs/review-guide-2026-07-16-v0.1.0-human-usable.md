# QClaw Review Guide — v0.1.0-human-usable Release

> Review scope: Full v0.1.0-human-usable release review (all changes since v0.1.0 tag at facc9c3).
> HEAD: `9ac3757` (v0.1.0-human-usable tag).
> Reviewer: QClaw (14-dimensional adversarial review pattern).
> Goal: APPROVE or REJECT v0.1.0-human-usable for push to GitHub.

---

## Focus Areas

### FA-1: AGENTS.md Restoration (`8e61991`)

Review: `src/apps/cli/` → root `AGENTS.md` (203 lines)

Key invariants to verify:
- [ ] Line 1 contains `[中文](AGENTS-CN.md) | **English`** bilingual link
- [ ] "Layered Module Index" table has all 6 layers (Interfaces → Contracts)
- [ ] "Common Commands" section has ≥ 20 commands (install/dev/check/test/build)
- [ ] "Verification table" has ≥ 10 rows (change type → minimum verification)
- [ ] "Global Rules" has i18n + logging + Tauri + platform + remote + agent loop sections
- [ ] "Architecture" section references `docs/architecture/core-decomposition.md`
- [ ] "Agent-doc priority" rule present at end
- [ ] No session-specific notes, no "%APPDATA%" references, no "Mavis" references
- [ ] AGENTS-CN.md exists as Chinese translation

### FA-2: Syntect Optionalization (`612677a`)

Review: `src/apps/cli/Cargo.toml` + `src/apps/cli/src/ui/syntax_highlight.rs`

Key invariants to verify:
- [ ] `[dependencies]` section: `syntect` and `syntect-tui` have `optional = true`
- [ ] `[features]` section: `default = ["syntax-highlight"]`
- [ ] `[features]` section: `syntax-highlight = ["dep:syntect", "dep:syntect-tui"]`
- [ ] `syntax_highlight.rs`: `#[cfg(feature = "syntax-highlight")]` mod for syntect impl
- [ ] `syntax_highlight.rs`: `#[cfg(not(feature = "syntax-highlight"))]` mod for plain fallback
- [ ] Public API (`highlight_code`, `highlight_bash_command`, `highlight_bash_output`, `highlight_code_with_line_numbers`) available in BOTH configs
- [ ] No crash when feature disabled (graceful plain-text fallback)
- [ ] `cargo check -p northhing-cli --no-default-features` → Finished
- [ ] `cargo check -p northhing-cli` (default) → Finished

### FA-3: Telemetry Test Fix (`612677a`)

Review: `src/crates/execution/agent-dispatch/tests/telemetry_test.rs`

Key invariants to verify:
- [ ] `all_const_flags_default_off_in_phase_1` asserts only USE_ONESHOT_DISPATCHER, USE_ACTOR_IPC, USE_DISPATCHER_IPC
- [ ] `USE_LIGHTWEIGHT_ACTOR` removed from assertion (A2 activated since e5ae9b1)
- [ ] Comment explains the A2 activation reason
- [ ] `USE_LIGHTWEIGHT_ACTOR` removed from import (no unused import warning)
- [ ] `cargo test -p northhing-agent-dispatch --test telemetry_test` → all pass

### FA-4: Documentation Consistency

Review: AGENTS.md, HANDOFF.md, README.md, release notes

Key invariants to verify:
- [ ] HANDOFF.md §0 HEAD = `c1b0c27` (or newer, matching actual HEAD)
- [ ] HANDOFF.md §0 "next session's job" references v0.1.0-human-usable tag + GitHub push (not old C5 fix)
- [ ] HANDOFF.md §0 test metrics include "932/933 PASS"
- [ ] README.md L19 has "v0.1.0 human-usable (2026-07-15)" or similar date
- [ ] README.md module directory listing includes `src/crates/cli-internal/`, `src/crates/test-support/`, `src/apps/relay-server/`
- [ ] Release notes exist at `docs/releases/2026-07-16-v0.1.0-human-usable.md`
- [ ] Release notes in both Chinese and English
- [ ] No outdated references (Mavis, C5 "blocked", old commit hashes)

### FA-5: Code Quality & Safety

Key invariants to verify:
- [ ] No new `unsafe` blocks in CLI or core crates
- [ ] No new `unwrap()` or `expect()` in production code (test code exempt)
- [ ] No path traversal vulnerabilities in file operations
- [ ] All `#[cfg(feature = "syntax-highlight")]` properly paired with `#[cfg(not(...))]` fallback
- [ ] `cargo clippy -p northhing-core --lib` runs without crashing (warnings OK, errors NOT)
- [ ] Dead code warning count ≤ 160 (baseline 151 from bbd0b5b)

### FA-6: Test Coverage

Commands to run:
```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo test --workspace --exclude northhing 2>&1 | Select-String "test result:"
cargo test -p northhing --lib 2>&1 | Select-String "test result:"
cargo test -p northhing-cli --lib 2>&1 | Select-String "test result:"
```

Expected:
- [ ] `--workspace --exclude northhing`: all pass EXCEPT pre-existing turn_batch fail
- [ ] `-p northhing --lib`: all pass (80/80 or similar)
- [ ] `-p northhing-cli --lib`: all pass (34/34 or similar)
- [ ] No new test failures vs pre-review baseline

### FA-7: Git Hygiene & Tag

- [ ] `git status` → working tree clean
- [ ] `git tag -l` shows both `v0.1.0` and `v0.1.0-human-usable`
- [ ] `git log --oneline -10` shows clean commit messages
- [ ] No untracked junk files (test-*.txt, target-test/, .loop-worktrees/)
- [ ] `9ac3757` is tagged as v0.1.0-human-usable

### FA-8: End-User Readiness (面向无代码能力用户)

Key invariants to verify:
- [ ] README has clear installation instructions
- [ ] README has bilingual support or links to Chinese version
- [ ] README clearly states v0.1.0 limitations (desktop GUI pending, etc.)
- [ ] Release notes explain "what's new" in non-technical language
- [ ] Clippy warning count reasonable (< 50 new warnings)
- [ ] No crash-on-startup in CLI default build

### FA-9: Slint/Desktop (informational — known limitation)

- [ ] `cargo check -p northhing` (Slint desktop) → compiles OK
- [ ] Documented as known limitation that workspace-level test excludes Slint due to MSYS2 GCC
- [ ] Desktop GUI not claimed as fully verified in v0.1.0

### FA-10: Post-v0.1.0 Drift Check

- [ ] Roadmap in `docs/plans/2026-07-16-v0.1.0-human-usable-roadmap.md` is accurate (C5 = resolved)
- [ ] HANDOFF §11 pointers still reference existing docs
- [ ] No broken symlinks or references to deleted files

---

## Severity Classification

| Level | Definition | Action |
|---|---|---|
| CRITICAL | Blocks release — broken compile, test regression, data loss risk | MUST FIX before tag |
| Major | Significantly impacts user experience or code quality | SHOULD FIX before tag |
| Minor | Style, naming, minor doc inconsistency | Can ship, fix in next iteration |
| Observation | For future consideration | Logged only |

## 9 Verification Steps

1. `cargo check -p northhing-cli --no-default-features` → output contains `Finished`
2. `cargo check -p northhing-cli` (default) → output contains `Finished`
3. `cargo test --workspace --exclude northhing` → all pass except known turn_batch fail
4. `git status` → `nothing to commit, working tree clean`
5. `git tag -l` → both `v0.1.0` and `v0.1.0-human-usable` present
6. `wc -l AGENTS.md` → 203 lines
7. `grep "syntax-highlight" src/apps/cli/Cargo.toml` → 3+ matches
8. `grep "USE_LIGHTWEIGHT_ACTOR" src/crates/execution/agent-dispatch/tests/telemetry_test.rs` → 0 matches
9. Read HANDOFF.md §0 — HEAD and "next session's job" reflect current state

## Final Verdict

```
Score: X/10
Verdict: APPROVE | REJECT
Blockers (if any):
```

---

*Reviewer: QClaw*
*Date: 2026-07-16*
*Scope: v0.1.0-human-usable full release review*
