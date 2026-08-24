# B2 v2 god-file split — Source recovery + C2.1 complete handoff

> **Date**: 2026-07-15 18:46 (Asia/Shanghai)
> **Session**: mvs_347da3ec15af4811ba124820fb3609d5
> **Next session pick up**: read this + read `2026-07-15-v0.1.0-roadmap.md` + read MEMORY.md §"Mavis producer-boundary" + §"Subagent model verification" + §"Verification gap" + §"Commander = full ownership"

## TL;DR

1. **Source recovery done** — northing repo rebuilt from `northing-impl-b0-smoke` snapshot (3 days old, not 1.5 months as first assumed). 30 min recovery, 3011 files / 88 MB.
2. **B2 v2 5 cycle addenda written** (4 are uncommitted in v2 worktree) — 4 QClaw HARD fixes baked into each spec.
3. **C2.1 v2 DONE** (question.rs split, 5 files, +10 net, cargo check `Finished` in 2m 26s, 0 error). C2.2-2.5 v2 addenda ready but not yet dispatched.
4. **Blocker for v2 (per cycle)**: `pnpm run i18n:generate` is MANDATORY pre-step before `cargo check` — generates 6 files required for `northhing-core` to compile (1 pre-existing dep error: `error[E0583]: file not found for module 'generated_locale_contract'` at `src/crates/assembly/core/src/service/i18n/mod.rs:5:1`).
5. **task tool parallel limit discovered**: 4 simultaneous `task` calls in 1 message all abort. 1 by 1 or 2 at a time works. Previous memory thought "无并行上限" — that was wrong for OpenCode `task` tool specifically.

## Goal (long-term)

Push northing to **0.1.0 human-usable release** (CLI/Web/Desktop/Mobile work, installable, documented, P0 bug free). GitHub push deferred to 0.1.0 done (user has never used GitHub).

## Current state

### Repo layout (post-recovery)

```
E:/agent-project/
├── northing/                                 # main repo, fresh git init
│   ├── .git/                                 # branch: main, HEAD 1b147c3
│   ├── .loop-worktrees/
│   │   └── b2-god-split-v2-20260715/         # active worktree, branch b2-god-split-v2-20260715
│   │       ├── src/apps/cli/src/ui/question.rs       # DELETED (D)
│   │       ├── src/apps/cli/src/ui/question/         # NEW (5 files, 813 lines)
│   │       │   ├── mod.rs (19), types.rs (41),
│   │       │   ├── state.rs (137), handle.rs (218), render.rs (398)
│   │       ├── src/apps/cli/src/modes/chat/input.rs  # 802 lines, NOT YET SPLIT
│   │       ├── src/apps/cli/src/main.rs              # 797 lines, NOT YET SPLIT
│   │       ├── src/apps/cli/src/acp_cli.rs           # 763 lines, NOT YET SPLIT
│   │       └── src/apps/cli/src/ui/command_palette.rs # 754 lines, NOT YET SPLIT
│   ├── docs/plans/2026-07-15-v0.1.0-roadmap.md  # 10-section plan (UNTRACKED)
│   └── (rest of source)                       # tracked, snapshot from b254db80
└── northing-impl-b0-smoke/                    # source snapshot 2026-07-12, READ-ONLY preserved
    └── (full northing source, HEAD b254db80)  # has older v0.1.0 tag at 2813b36
```

### Worktree state

`E:/agent-project/northing/.loop-worktrees/b2-god-split-v2-20260715/`:
- Branch: `b2-god-split-v2-20260715`
- HEAD: `1b147c3` (same as main, no commits yet on worktree)
- C2.1 changes uncommitted: 5 files added in `src/apps/cli/src/ui/question/`, 1 file deleted (question.rs)
- 4 cycle addenda uncommitted: `cycle-addenda/2026-07-15-c{2,3,4,5}-*.md`
- 1 modified file unrelated to C2.1: `src/apps/relay-server/static/homepage/i18n.shared.json` (was in snapshot)

### B2 v1 (LOST, do not look for it)

- B2 v1 worktree `.loop-worktrees/b2-god-split-20260715` is GONE — 12h disk accident between sessions
- 5 cycle addenda v1 + B2 plan v1 + QClaw review v1 all LOST (were untracked in deleted northing/)
- Memory: still safe in `C:/Users/UmR/.mavis/agents/mavis/memory/`
- QClaw review lives in `C:/Users/UmR/.qclaw/workspace/b2-god-split-final-review_20260715.md` (persistent, readable)
- Net effect: had to redo all 5 cycle addenda for v2, but with 4 QClaw fixes baked in (so v2 should be cleaner)

## Done this session (2026-07-15, 17:00-18:46)

1. **Source recovery** from `northing-impl-b0-smoke/`:
   - 4 stale entries trashed from `E:/agent-project/northing/` (target-shared, tests, tools, `{}`)
   - 3011 files / 88 MB robocopy from snapshot
   - `git init` + initial commit `1b147c3` "snapshot: from northing-impl-b0-smoke 2026-07-12 (HEAD b254db80)"
   - LongCat subagent verified project identity: AGENTS.md match, 5 god files LOC all match
   - `cargo check -p northhing-cli` baseline: PASS 0 error, 1517 warnings (1382 dead_code historical)
2. **Worktree v2 created**: `git worktree add .loop-worktrees/b2-god-split-v2-20260715 -b b2-god-split-v2-20260715 main`
3. **5 cycle addenda v2 written** in worktree `cycle-addenda/`:
   - `2026-07-15-c1-question.md` (C2.1)
   - `2026-07-15-c2-input.md` (C2.2)
   - `2026-07-15-c3-main.md` (C2.3)
   - `2026-07-15-c4-acp.md` (C2.4)
   - `2026-07-15-c5-palette.md` (C2.5)
4. **0.1.0 roadmap plan** written: `E:/agent-project/northing/docs/plans/2026-07-15-v0.1.0-roadmap.md` (10 sections, 1 critical path + 9 parallel streams)
5. **C2.1 v2 dispatched + completed**:
   - 1 LongCat subagent, 15 min, 0 M3 take-over
   - Output: question.rs (803) → question/ dir (5 files, 813 lines, +10 net)
   - `pnpm run i18n:generate` pre-step (CRITICAL — see blocker below)
   - `cargo check -p northhing-cli` → `Finished dev profile [unoptimized + debuginfo] target(s) in 2m 26s` ✅
   - 4 QClaw HARD fixes baked into spec, subagent wrote correctly first time, 0 Mavis direct edit
   - 8 `pub(super)` helpers in state.rs confirmed (mirrors C1 v1 QClaw pattern)
6. **2 stale cron deleted** (per "Cron self-cleanup discipline"): check-chat-render-v2 + check-p2-p3

## In progress (paused for new session)

- **C2.2 v2 dispatch blocked by system limit** — tried 4 parallel `task` calls in 1 message, all got "Tool execution aborted". Need to fall back to 1-by-1 or 2-at-a-time.
- 4 cycle addenda (C2.2-2.5) all written, ready to dispatch sequentially

## Blocker (pre-existing dep, Mavis commander responsibility per user 04:36)

**`pnpm run i18n:generate` is MANDATORY pre-step before any `cargo check -p northhing-cli`** in this snapshot.

- Generates 6 i18n contract files (locales + base)
- Without it, `northhing-core` fails with `error[E0583]: file not found for module 'generated_locale_contract'` at `src/crates/assembly/core/src/service/i18n/mod.rs:5:1`
- This dep error is **pre-existing in snapshot** (v0.1.0 tag at 2813b36 also has it; not introduced by B2)
- Mavis commander decision (per user 2026-07-15 04:36 push-back "不会因为技术债出现突然说不是这个session引入的所以就直接pass"): **fix the dep at the source** (run `pnpm run i18n:generate` before every cargo check) rather than declare it out of scope
- Subagent MUST include `pnpm run i18n:generate` as first cargo check step in their workflow
- Or: investigate `src/crates/assembly/core/src/service/i18n/mod.rs:5` and add `#[allow(missing_module)]` / `#[path]` attribute / proper build.rs — but `pnpm run i18n:generate` is the lighter path

## Next steps (Mavis session resume)

1. **M3 take-over 1-2 min** before resuming: check worktree is intact, no stale Mavis direct edit leaked in
2. **C2.2 dispatch** (input.rs split, 802 → 5 files, 1 by 1, NOT 4 parallel):
   - Reuse the prompt template from this session (4 mandatory items: workdir + resolver + return format + max_tokens)
   - Cycle addendum: `E:/agent-project/northing/.loop-worktrees/b2-god-split-v2-20260715/cycle-addenda/2026-07-15-c2-input.md`
   - Bake-in fix from QClaw v1: mouse.rs `use super::super::{ChatExitReason, ChatMode, NonKeyEventOutcome};`
3. **C2.3-2.5 dispatch** (1 by 1 sequential OR 2-at-a-time):
   - 3 addenda in worktree, each ~5-7KB
   - Mavis writes 4 prompt variants, dispatches 1 at a time
   - Total LongCat: ~60-80 min for 4 cycles
4. **Phase C3: cargo fmt cleanup** (156 files, subagent 1-2h)
5. **Phase C4: pre-existing turn_batch B-3 test fix** (30-60min subagent)
6. **Phase C5: `cargo test --workspace`** (1-2h subagent parallel, target: 0 fail)
7. **Phase C6: HANDOFF.md + README.md update** (Mavis 30-60min)
8. **Phase C7: re-tag v0.1.0 at clean HEAD** (Mavis 15min)
9. **Parallel streams A-I** (run any time):
   - A: dead_code 1382 → 0 (M3 take-over bulk fix, 1-2h)
   - B: doc audit (Mavis, scan for stale untracked)
   - D: cross-platform smoke test (subagent)
   - E: installer build (subagent)
   - F: e2e test on CLI/Web/Desktop/Mobile (subagent)
   - G: rustdoc completion (subagent)
   - I: docs cleanup (Mavis, ASCII-only per mojibake prevention)

## Critical context (so new session doesn't repeat mistakes)

### Done criterion (QClaw 2026-07-15 04:30 lesson)

**`cargo check -p <target>` output MUST contain `Finished`** — NOT just exit 0, NOT just "0 errors in target files". The literal substring `Finished` in the output line is the only valid done signal.

### Subagent model verification (B2 C1 abort lesson)

**Verify via resolver ONLY**:
```bash
py C:/Users/UmR/.mavis/bin/agent-model-resolver.py --agent general --show-fallback-chain
```
Expected: `effective: longcat/LongCat-2.0 (L1)`.

**DO NOT verify via `mavis session list` or `mavis session info`** — those show Mavis root sessions (M3), not the OpenCode-native `task` tool session, which uses `general` agent config (`~/.mavis/agents/general/opencode/opencode.json` `model: longcat/LongCat-2.0`).

If `effective: minimax/MiniMax-M3` → ABORT immediately, report `MODEL_MISMATCH_ABORT`, do not start work with M3.

### Subagent prompt essentials (4 mandatory items)

1. **Workdir** = `E:/agent-project/northing/.loop-worktrees/b2-god-split-v2-20260715/` (use `workdir` parameter, never `cd`)
2. **Model verify** via resolver (above)
3. **Return format** = Report only, no commit, final "Report" section with effectiveModel / Work done / 8 self-check / Visibility / Out-of-scope / Blockers
4. **LongCat max_tokens** ≥ 4096

### Hard rules (per B2 plan + C2.1 lessons)

- Edit/Write tools for .rs files (NOT PowerShell Set-Content/Add-Content/Get-Content which corrupts UTF-8)
- Do NOT commit (Mavis commander commits after user review)
- 156 pre-existing cargo fmt warnings are out of scope — DO NOT run `cargo fmt`
- Bash PATH prefix for cargo: `$env:Path = "C:\msys64\mingw64\bin;" + $env:Path`
- Touch ONLY the cycle's target file/dir (e.g. C2.2 = `modes/chat/input*` only)

### Pre-step (Mavis commander, not subagent)

**Before dispatching any cycle subagent**, verify `pnpm run i18n:generate` was run in the worktree (i18n files are not tracked by git, must be regenerated on every worktree creation):
```bash
cd <workdir>; pnpm run i18n:generate
```
Subagent should also run it as first cargo-check step (defense-in-depth).

### mavis binary context

- 0.51 binary 0.51.82 self-contained turn dispatch (works)
- 0.47 daemon 0.47.0 dead (metadata API broken, turn dispatch OK)
- mavis 0.50 was the broken version (lessons in MEMORY.md are 0.50-specific, qualified)
- OpenCode `task` tool works on 0.51 (proven with C2.1 success)

## Files (state of repo)

| Path | State | Notes |
|---|---|---|
| `E:/agent-project/northing/` | main repo | branch `main`, HEAD `1b147c3` |
| `E:/agent-project/northing/.loop-worktrees/b2-god-split-v2-20260715/` | worktree | branch `b2-god-split-v2-20260715`, HEAD `1b147c3`, C2.1 uncommitted |
| `E:/agent-project/northing/src/apps/cli/src/ui/question.rs` | **DELETED** | in worktree only |
| `E:/agent-project/northing/src/apps/cli/src/ui/question/` | **5 NEW files** | in worktree, uncommitted (mod=19, types=41, state=137, handle=218, render=398) |
| `E:/agent-project/northing/src/apps/cli/src/modes/chat/input.rs` | 802 lines, uncommitted | not yet split |
| `E:/agent-project/northing/src/apps/cli/src/main.rs` | 797 lines, uncommitted | not yet split |
| `E:/agent-project/northing/src/apps/cli/src/acp_cli.rs` | 763 lines, uncommitted | not yet split |
| `E:/agent-project/northing/src/apps/cli/src/ui/command_palette.rs` | 754 lines, uncommitted | not yet split |
| `E:/agent-project/northing/docs/plans/2026-07-15-v0.1.0-roadmap.md` | UNTRACKED | 10-section plan |
| `E:/agent-project/northing/.loop-worktrees/b2-god-split-v2-20260715/cycle-addenda/` | 5 files UNTRACKED | C2.1-C2.5 addenda |
| `E:/agent-project/northing-impl-b0-smoke/` | READ-ONLY | source snapshot, do not touch |
| `C:/Users/UmR/.qclaw/workspace/b2-god-split-final-review_20260715.md` | readable | v1 QClaw review (4 HARD, 9 MINOR, 6.5/10) — read for lessons, not for current work |
| `C:/Users/UmR/.mavis/agents/mavis/memory/MEMORY.md` | readable | cross-project memory, ~478 lines |
| `C:/Users/UmR/.mavis/agents/mavis/memory/mavis-runtime.md` | readable | L1 detail, includes B2 lessons |
| `C:/Users/UmR/.mavis/bin/agent-model-resolver.py` | readable | subagent model verifier |

## Lessons to remember (already in MEMORY.md)

- §"Mavis producer-boundary (2026-07-15)" — Mavis NOT write .rs code, subagent does
- §"Subagent model verification (2026-07-15)" — use resolver not session list
- §"B2 god-split 5 cycle 1-by-1 sequential pattern" — 1 worktree, 5 cycle, 1 by 1
- §"Verification gap (2026-07-15 B2 QClaw, CRITICAL)" — done criterion = `Finished` in output
- §"Commander = full ownership, no out-of-scope pass" — fix dep chain, don't pass
- §"R75 守則 self-imposed misread" — Mavis-authored doc should be committed, not untracked
- §"mavis binary 0.51 binary 0.51.82 self-contained" — 0.50-specific lessons don't apply to 0.51

## Lessons to add next session (NEW from this session)

1. **Task tool parallel limit**: 4 simultaneous `task` calls in 1 message all abort. Use 1 by 1 or 2-at-a-time.
2. **`pnpm run i18n:generate` pre-step**: MANDATORY before `cargo check -p northhing-cli` (6 generated files, dep error if missing).
3. **Source code safety**: Untracked docs in northing/ at risk during disk accidents. Commit important Mavis-authored docs ASAP, or have external backup.
4. **Snapshot recovery feasibility**: 3-day-old snapshot viable for 30-min recovery vs 100+ hours rebuild from scratch. Keep `northing-impl-b0-smoke/` around as insurance.
5. **v0.1.0 tag at 2813b36 is "tech preview"** — user wants real "human-usable" release, more work needed.

## Quick resume commands (for new Mavis session)

```powershell
# 1. Check worktree state
cd E:/agent-project/northing/.loop-worktrees/b2-god-split-v2-20260715
git status --short
git log --oneline -3

# 2. Run i18n pre-step (CRITICAL)
pnpm run i18n:generate

# 3. Verify baseline still compiles
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing-cli 2>&1 | Select-String "Finished" | Select-Object -First 1

# 4. Read cycle addenda
Get-Content cycle-addenda/2026-07-15-c2-input.md

# 5. Dispatch C2.2 subagent via task tool (1 by 1, NOT 4 parallel)
```

## Sign-off

C2.1 v2 done. C2.2-2.5 v2 ready to dispatch. Critical fix paths: resolver verification, pnpm pre-step, Finished criterion, 1-by-1 dispatch (not 4 parallel). M3 quota OK if new session starts in next 1-2 hours (current session used ~30min).
