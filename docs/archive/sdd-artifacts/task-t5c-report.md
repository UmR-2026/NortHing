# Task t5c: Fix vacuous test assertions in auto_memory.rs

Worktree: `E:\agent-project\northing\.worktrees\growth-core-0804` (branch `feat/growth-core-0804`)
File changed: `src/crates/assembly/core/src/service/agent_memory/auto_memory.rs` (only file changed)

## Production format string confirmed

`auto_memory.rs:304`:

```rust
format!("\n\n# Remembered facts\n\n{}", items)
```

This is the exact rendered section marker the assertions now check for. The new prose in the "Auto-captured facts vs. your memory files" section (line 251) writes the phrase inline as ``The `# Remembered facts` block ...``, which does NOT contain the `\n\n# Remembered facts\n\n` form, so the tightened assertions are not tripped by the prose.

## Assertion edits

### Edit 1 - `prompt_injection_without_facts_excludes_remembered_facts_section`
`auto_memory.rs:490-491`

```rust
        assert!(
            !prompt.contains("\n\n# Remembered facts\n\n"),
            "Prompt should NOT contain '# Remembered facts' section when no facts exist"
        );
```

Comment restored (line 488): `// Should NOT contain the remembered facts section`

### Edit 2 - `prompt_injection_degrades_when_facts_file_unreadable`
`auto_memory.rs:533-534`

```rust
        assert!(
            !prompt.contains("\n\n# Remembered facts\n\n"),
            "Prompt should omit '# Remembered facts' when facts.jsonl is unreadable"
        );
```

Comment restored (line 531): `// Prompt build must still succeed and simply omit the facts section.`

The vacuous `!prompt.contains("- I prefer pnpm")` assertions (which could never fail) were replaced with assertions on the exact rendered section marker. Both messages now describe the section absence, and both reworded comments were restored to match the original intent. The four added prompt sections, their wording, their placement, and the new test `prompt_includes_all_four_memory_guidance_additions` are untouched.

## Non-vacuity proof

For each tightened assertion, the assertion was temporarily inverted to `prompt.contains("\n\n# Remembered facts\n\n")` with message `"PROOF: inverted assertion to demonstrate non-vacuity"`, the test was run and FAILED, then reverted.

### Proof 1 - `prompt_injection_without_facts_excludes_remembered_facts_section`

```
running 1 test
test service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section ... FAILED

failures:

---- service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section stdout ----

thread 'service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section' (20828) panicked at src\crates\assembly\core\src\service\agent_memory\auto_memory.rs:489:9:
PROOF: inverted assertion to demonstrate non-vacuity
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1226 filtered out; finished in 0.05s

error: test failed, to rerun pass `-p northhing-core --lib`
```

### Proof 2 - `prompt_injection_degrades_when_facts_file_unreadable`

```
running 1 test
test service::agent_memory::auto_memory::tests::prompt_injection_degrades_when_facts_file_unreadable ... FAILED
thread 'service::agent_memory::auto_memory::tests::prompt_injection_degrades_when_facts_file_unreadable' (54112) panicked at src\crates\assembly\core\src\service\agent_memory\auto_memory.rs:532:9:
PROOF: inverted assertion to demonstrate non-vacuity

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1226 filtered out; finished in 0.05s
error: test failed, to rerun pass `-p northhing-core --lib`
```

Both tests fail when the assertion is inverted, proving the assertions are NOT vacuous. Both edits were reverted immediately after.

## Verification (full raw output)

All commands run from the worktree root with `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`.

### 1. `cargo test -p northhing-core --features product-full auto_memory` - 8 tests

```
test service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_facts_includes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_select_facts_budget_limit ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 1219 filtered out; finished in 0.10s
```

### 2. `cargo test -p northhing-core --features product-full prompt_injection` - 4 tests

```
running 4 tests
test service::agent_memory::auto_memory::tests::prompt_injection_degrades_when_facts_file_unreadable ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_facts_includes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_select_facts_budget_limit ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 1223 filtered out; finished in 0.10s
```

### 3. `cargo check -p northhing-core --features product-full` - warning baseline

```
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
```

19 warnings, unchanged from baseline (the count of `^warning` lines is 20, the extra line being the cargo summary line "generated 19 warnings"). No increase.

### 4. `node scripts/check-core-boundaries.mjs` - exit 0

```
Core boundary check passed.
EXIT=0
```

### 5. Line count of `auto_memory.rs`

```
LINES=675
```

## Commit

```
commit 2e986cec63da1d1ea34f5cc870b493835d68e4da
Author: Mavis <mavis@northhing.local>
Date:   Thu Aug 6 17:19:53 2026 +0800

    feat(memory): add four guidance sections to agent memory prompt and tighten remembered-facts section assertions

 .../core/src/service/agent_memory/auto_memory.rs   | 84 +++++++++++++++++++++-
 1 file changed, 82 insertions(+), 2 deletions(-)
```

The commit contains ONLY the intended changes to the one file: 82 insertions, 2 deletions. No whole-file line-ending churn (working copy has CRLF; `.gitattributes` normalizes `*.rs` to LF on commit, and the diff shows only the 4 added prompt sections + 1 new test + the 2 assertion-line replacements).

`git status --short` after commit: clean (empty). Nothing under `.superpowers/` was committed.
