# Task T0-2 Report: fix inverted bash availability check in run_script

Status: DONE

## Diff summary

Single-file, single-condition edit in
`src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/system_actions/app_control.rs`
(line 219, `"bash" =>` match arm):

```diff
-                if utilities::which_exists("bash") {
+                if !utilities::which_exists("bash") {
```

Nothing else changed — error text, hint, response shape, and all other match arms are byte-identical. No other files touched.

## Verification

### 1. `cargo check -p northhing-core`

Environment note: the `cargo` on PATH is the GNU install (`C:\Program Files\Rust stable GNU 1.95\bin\cargo.exe`) which fails on `ring`/`libsqlite3-sys` build scripts (`failed to find tool "gcc.exe"` — no gcc on PATH). Per root `AGENTS.md` ("repo dir override is GNU ... use `rustup run <tc> cargo`"), the check was run with the MSVC toolchain via rustup (MSVC is the active/default toolchain; also the one desktop builds use). This is a pre-existing environment quirk, unrelated to this change.

Command:
```
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check -p northhing-core
```

Raw output (tail; full output captured, only final lines shown):
```
   Checking northhing-agent-runtime v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-runtime)
   Checking idna_adapter v1.2.2
   Checking idna v1.1.0
   Checking url v2.5.8
   Checking legible v0.4.2
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 44.07s
```

Result: PASS (0 errors).

### 2. Existing tests for the run_script bash arm

```
rg -n "run_script" src/crates/assembly/core/tests/ -g *.rs
```
→ no matches (no output).

```
rg -n "run_script" src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/ -g *.rs | rg -i test
```
→ no matches (no output).

**No existing test module covers the run_script bash arm** (nor run_script at all in the searched locations). Per the brief, no test was written.

## Concerns

- None. The fix is the exact negation the brief specifies; behavior now matches intent (bash present → spawn; bash absent → NotAvailable error).
- Note: `grep` is unavailable in this PowerShell environment; `rg` (ripgrep) was used for the test search.