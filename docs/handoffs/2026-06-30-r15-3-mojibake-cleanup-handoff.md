# R15.3 Handoff — Three-stream mojibake cleanup

## Summary

| Stream | Subagent | Files modified | Chars reduced |
|---|---|---:|---:|
| 1 — CN doc rewrite | general | 7 | 383 (whole-file rewrite, size Δ) |
| 2 — i18n JSON repair | general | 4 | ~145 (semantic rewrite) |
| 3 — Old doc batch | general | 88 | 2,881 |
| Mavis final sweep | (Mavis) | 3 + 2 revert | 17 (鈥/鈫 residues + AGENTS.md/CONTRIBUTING.md revert+fix) |
| **Total** | | **102** | **~3,400** |

## What landed

### Stream 1 — CN docs (Task 1 subagent)
All 7 CN-version docs rewritten from their English counterparts, preserving all Markdown structure, tables, code blocks, lists, commands, paths, URLs.

Notable: top-level `AGENTS-CN.md` 14,252 → 11,448 bytes (-20%); `CONTRIBUTING_CN.md` 9,558 → 7,624 bytes (-20%).

### Stream 2 — i18n JSON (Task 2 subagent)
52 user-facing strings restored across 4 JSON files. All 4 files pass `json.loads()` strict parse post-fix. CRLF line endings preserved.

The pattern was UTF-8 multi-byte CJK trailing byte replaced by `0x3F` (`?`), making the file invalid UTF-8. Task 2 subagent used raw-byte needle anchoring (UTF-8 prefix + `?` + JSON separator `,\r\n`) and semantic Chinese rewrite from `en.json` cross-reference.

### Stream 3 — Old doc batch (Task 3 subagent)
22 files actually fixed (66 already clean). Total 2,881 chars removed across 88 files in scope. Master script `E:\agent-project\.mavis\tmp-r15-3\fix_old_docs.py` documents the strategy.

### Mavis final review sweep (post-subagent)
**Found and fixed 2 subagent gaps**:

1. **3 md files with residual `U+9225 鈥` / `U+922B 鈫` mojibake** that subagent's character set missed. 17 chars total. Fixed by collapsing `鈥—` / `鈫—` patterns to `—`.
2. **AGENTS.md and CONTRIBUTING.md had collateral damage** (trailing whitespace stripping + list-indent stripping in markdown). Restored to `HEAD` and applied minimal `鈥?` → `—` fix only.

## Verification

```
cargo test -p northhing-core --lib --features 'service-integrations,product-full'
test result: ok. 899 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.18s
```

Character-class residue scan across 98 modified files: **0 mojibake chars remaining** (excluding 2 GB2312 level-1 legitimate chars `冲` and `般` which appear in 3 files as part of legitimate Chinese phrasing like "冲突" / "一般").

## What did NOT land

- Locale JSON strings not inferable from `en.json` were left unchanged (conservative).
- Deep GBK patterns in `research/` and `docs/superpowers/specs/` partially fixed; deeper layers may have residual GBK-as-UTF-8 that requires Chinese-NLU inference (out of scope for character-level fix).
- 4 untracked pre-existing handoff docs at `docs/handoffs/2026-06-28-*.md` (R5/R6/R8b era) — subagent didn't touch these as they're not in `git ls-files`.

## Refs

- Spec: `docs/handoffs/2026-06-30-r15-3-mojibake-cleanup-spec.md`
- Review guide: `docs/handoffs/2026-06-30-r15-3-mojibake-cleanup-review.md`
- Subagent wide-class scan reports (predecessor): `E:\agent-project\mojibake_action_summary.md` + `E:\agent-project\mojibake_records.json`
- Subagent master fix script: `E:\agent-project\.mavis\tmp-r15-3\fix_old_docs.py` + `REPORT.md` + `verify.py` + `post_check.py`
- Mavis verify scripts: `E:\agent-project\.mavis\tmp-r15-3\verify_mavis.py` + `find_residual.py` + `find_residual2.py` + `fix_鈥_mojibake.py`
- Predecessor: R15.2 (`756331e`) + R15 P0 (`b30882f`)
- Plan reference: `docs/handoffs/2026-06-30-r15-god-object-plan.md`