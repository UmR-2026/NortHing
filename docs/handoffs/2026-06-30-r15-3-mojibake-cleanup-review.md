# R15.3 Review Guide — Three-stream mojibake cleanup (102 files)

## Summary

Three parallel subagents cleaned up mojibake debt across the repo. Mavis final review found and fixed 2 subagent gaps before commit.

| Metric | Value |
|---|---:|
| Modified files | **102** (7 CN + 4 JSON + 88 old docs + 3 Mavis final sweep) |
| `.rs` files touched | **0** |
| Cargo test baseline | 899/0/1 unchanged |
| Total mojibake chars removed | ~3,400 |
| Final residue (excl. legit CJK) | **0** |
| Subagent gaps caught by Mavis | 2 |

## What to review

### Stream 1 — CN doc rewrite (7 files)

Translation fidelity: did the subagent preserve all paragraphs, tables, code blocks from the English source? Spot-check by reading 1-2 CN docs side-by-side with their EN counterparts.

| File | EN source | Lines before/after |
|---|---|---|
| `AGENTS-CN.md` | `AGENTS.md` | 14252B → 11448B |
| `CONTRIBUTING_CN.md` | `CONTRIBUTING.md` | 9558B → 7624B |
| 5 sub-AGENTS-CN.md | respective EN | various |

### Stream 2 — i18n JSON (4 files)

Did the subagent accurately translate each `zh.json` / `zh-TW.json` string from the `en.json` reference? The 52 fixed strings are user-facing; mistranslations would be visible to installers.

`json.loads()` validation: all 4 files parse strictly.

### Stream 3 — Old doc batch (88 files)

The character-level repair used a 35-character CJK mojibake set + `\ufffd` + `?` pattern. Each mojibake run collapsed to single `—` (em-dash). Verify a few files (top 5 by char reduction: `PROMPT_LOADER_ARCHITECTURE.md` 424, `MiniApp/SKILL.md` 357, `sdlc-harness/implementation-plan.md` 324, `MiniApp/design-playbook.md` 213, `MiniApp/api-reference.md` 205) read sensibly.

### Mavis final sweep (2 fixes)

1. **`docs/superpowers/specs/2026-06-17-v3-prompt-loader-design-v2.md`** — 14 chars `鈥—` / `鈫—` collapsed to `—`. Verify reading still makes sense (it should — pattern was always "em-dash + sentence").
2. **`AGENTS.md` + `CONTRIBUTING.md`** — restored to `HEAD` (collateral damage in shell-comment alignment + markdown list indent), then applied minimal `鈥?` → `—` fix. Verify shell-comment alignment matches HEAD (16-space alignment on `# full hot-reload:` etc.).

## Critical observations (please verify)

1. **No `.rs` files touched.** Verified via `git diff --stat -- '*.rs'`. All changes are in `.md` and `.json`.
2. **`cargo test` baseline preserved.** 899 passed, 0 failed, 1 ignored — exactly R14 baseline.
3. **2 false-positive CJK chars are legitimate.** `冲` (U+51B2, "conflict") and `般` (U+822C, "general") appear in 3 files (top-level `AGENTS-CN.md` L171, `CONTRIBUTING_CN.md` L91, `zh.json` L14, `zh-TW.json` L14) as part of normal Chinese phrasing. These are GB2312 level-1 chars, not mojibake. Subagent's character class scanner flagged them initially; Mavis confirmed legitimacy.
4. **CN docs translated from EN.** If user wants original Chinese wording preserved (where GBK source is still recoverable), this round picked the safe path (re-translate from EN). Alternative would require external GBK source — not feasible.
5. **JSON repairs are semantic, not byte-perfect.** Where UTF-8 bytes were corrupted, subagent rewrote strings based on `en.json` context. Each replacement is a best-effort translation; if a translation is wrong, please flag the specific JSON key.
6. **No new mojibake introduced by Mavis sweep.** Verified: 0 files with increased mojibake char count after Mavis fix.

## Refs

- Spec: `docs/handoffs/2026-06-30-r15-3-mojibake-cleanup-spec.md`
- Handoff: `docs/handoffs/2026-06-30-r15-3-mojibake-cleanup-handoff.md`
- Predecessor: R15.2 (`756331e`) + R15 P0 (`b30882f`) + R14 fix `f777284`

## Questions for reviewer

1. Are the 7 CN doc translations acceptable, or should we attempt GBK-source recovery instead?
2. Are the 52 i18n JSON string translations accurate?
3. The 3 md files with `鈥—` / `鈫—` mojibake — is collapsing to `—` the right replacement, or should it be different (e.g. `:`, `,`)?
4. Should the 4 untracked R5/R6/R8b-era handoff docs at `docs/handoffs/2026-06-28-*.md` be cleaned up in a follow-up round?
5. Should deeper-layer GBK patterns in `research/` and `docs/superpowers/specs/` (out of scope for this round) be a dedicated R-X.Y round?

## Sign-off request

Please verify:
- [ ] 102 files modified, 0 `.rs` touched
- [ ] 0 mojibake residues in modified files (modulo 2 legitimate `冲`/`般` chars)
- [ ] `cargo test` baseline 899/0/1
- [ ] CN docs preserve all EN source structure
- [ ] i18n JSON strings semantically accurate (cross-reference `en.json`)
- [ ] AGENTS.md / CONTRIBUTING.md shell-comment alignment matches HEAD (16-space)

APPROVE / REJECT + score + minor observations.