# R15.2 Review Guide — R14 worker mojibake cleanup on `bot/mod.rs`

## Summary

Single-file character-level mojibake repair. Closes the R14 worker residue in `bot/mod.rs` — 8 `闁?` patterns (em-dash corruption) + 3 ornament clusters (49 GBK CJK ideographs each repeated 16-17×) + 1 trailing comment duplication. All from R14 refactor commit `ed35b81`'s introduction; R14 fix `f777284` missed the GBK-table character class.

| Metric | Before R15.2 | After R15.2 | Delta |
|---|---:|---:|---:|
| R14 residual codepoints in `bot/mod.rs` | 60 | **0** | **-60** |
| Ornament cluster occurrences | 49 | 0 | -49 |
| `闁?` patterns | 8 | 0 | -8 |
| Trailing `//!` duplication | 1 | 0 | -1 |
| File size (bytes) | 20,871 | **20,465** | **-406** |
| `cargo test -p northhing-core --lib --features 'service-integrations,product-full'` | 899/0/1 | **899/0/1** | unchanged |

Commit: pending `fix(mod-bot): R15.2 mojibake cleanup`.

## What to review

### Per-line diff

| Line | Repair |
|---|---|
| L1 | Drop duplicated trailing `//!` |
| L56 | `闁?` → `—` (em-dash, matches surrounding doc-comment style) |
| L91 | English rendering: `[Thinking] blocks are forwarded to the user, one line per ThinkingEnd event` (was: heavily GBK-corrupted, intent guessed from context — `[Thinking] are forwarded to the user, one per ThinkingEnd` was the original sense) |
| L93 | English rendering: `These messages are too noisy for IM channels` (was: `mode 闁?they were too noisy`) |
| L116 | Strip 17× `闁冲厜鍋撻柍鍏夊亾` ornament cluster, keep title. Replaced with `// ===== Shared workspace-file utilities =====` |
| L206 | Strip 16× ornament, keep title. `// ===== Downloadable file link extraction =====` |
| L208 | `闁?` → `—` |
| L371 | `闁?` → `:` (the original `Phase 1:` was structurally required for the doc-comment) |
| L394 | `闁?` → `:` |
| L429 | Strip 16× ornament, keep title. `// ===== Auto-push file delivery helpers =====` |
| L588 | `闁?` → `—` |

### Critical observations (please verify)

1. **L91/L93 English rendering is a best-guess.** The original text was English source comments but R14 worker mangled the em-dash separators. The repair chose clean English that matches the surrounding doc-comment style and the intent of the original sentences (verbose-mode tool-call forwarding too noisy for IM). **If the original text was meaningfully different (e.g. contained numbers, references, specific feature names), please flag and supply the correct text.**

2. **No semantic changes** to logic. All 11 edits are on doc comments or comment dividers — no executable code touched. The 3 ornament-cluster lines were pure decoration; the 8 `闁?` patterns were corruption in doc comments.

3. **UTF-8 write verified.** The fix was done via Python script with `encoding='utf-8'` (no BOM). File still has Windows CRLF line endings (matches git autocrlf).

4. **No iron-rule violations introduced.** All changes are in `//` and `///` comment lines; no production code touched.

5. **No cargo fmt noise.** Diff scope is exactly the 11 lines; no other lines touched.

6. **Scope decision: R14-only.** AGENTS-CN.md and CONTRIBUTING_CN.md have similar mojibake but it's whole-file GBK double-decoding (not character-level replaceable) and predates R14 (was already present in R13 era commits). Deferred to R15.3.

## Refs

- Spec: `docs/handoffs/2026-06-30-r15-2-mojibake-cleanup-spec.md`
- Handoff: `docs/handoffs/2026-06-30-r15-2-mojibake-cleanup-handoff.md`
- Subagent wide-class scan: `E:\agent-project\mojibake_action_summary.md` + `E:\agent-project\mojibake_records.json` (5.9MB raw, external — not in repo)
- Predecessor: R15 P0 `b30882f` + R14 fix `f777284`

## Questions for reviewer

1. Is the L91/L93 English rendering acceptable, or should R15.2 defer those two lines pending git archaeology to recover the original text?
2. The ASCII replacement for the ornament cluster uses `// ===== TITLE =====` — is this the project's preferred divider style, or should it match a more conventional `// ----- TITLE -----` or `// *** TITLE ***` pattern? (No project-wide divider style was located; this was Mavis's best guess.)
3. Should the 5 R14-era pre-existing handoff docs that are untracked in git (`docs/handoffs/2026-06-28-*.md`) be reviewed for mojibake too? (Quick scan: all 4 are clean of R14 residual codepoints.)
4. AGENTS-CN.md / CONTRIBUTING_CN.md whole-file mojibake — do you want a R15.3 spec now, or leave it as a known-debt entry in handoff?
5. The 4 i18n locale JSON files (63/59/1/22 mojibake chars respectively) — is R15.3 the right round, or should it be a dedicated i18n round given the user-facing impact?

## Sign-off request

Please verify:

- [ ] 0 R14 residual codepoints in `bot/mod.rs` (12-char class)
- [ ] 3 ornament clusters removed
- [ ] 899/0/1 tests unchanged
- [ ] UTF-8 clean write (no BOM, no Windows-1252 leakage)
- [ ] Diff scope exactly 1 file, 11 line replacements
- [ ] L91/L93 English rendering matches surrounding context

APPROVE / REJECT + score + minor observations.