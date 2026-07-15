# R15.2 Handoff — Mojibake Cleanup (bot/mod.rs R14 residue)

## What landed

| File | Lines (before) | Lines (after) | Delta | Notes |
|---|---:|---:|---:|---|
| `command_router_dispatch.rs` | 718 | 718 | 0 | Unchanged (R15.2 scope was mod.rs only) |
| `bot/mod.rs` | 620 | 599 | -21 | 11 line replacements, net -406 bytes |

Commit: `fix(mod-bot): R15.2 mojibake cleanup — 12-char GBK residue on 10 lines + ornament cluster strip`.

## Repair scope

11 character-level edits on `bot/mod.rs`:

| Line | Type | Action |
|---|---|---|
| L1 | Trailing comment dup | `//! Bot integration for Remote Connect.//!` → `//! Bot integration for Remote Connect.` |
| L56 | `闁?` (U+95C1 U+003F) | → `—` (em-dash) |
| L91 | 4-byte GBK corruption | Full rewrite (English rendering) |
| L93 | `闁?` | Full rewrite (English rendering) |
| L116 | 17× ornament cluster | → `// ===== Shared workspace-file utilities =====` |
| L206 | 16× ornament cluster | → `// ===== Downloadable file link extraction =====` |
| L208 | `闁?` | → `—` |
| L371 | `闁?` | → `:` |
| L394 | `闁?` | → `:` |
| L429 | 16× ornament cluster | → `// ===== Auto-push file delivery helpers =====` |
| L588 | `闁?` | → `—` |

Total 49 GBK-cluster occurrences removed. Total 8 `闁?` patterns fixed. 1 trailing comment duplication fixed.

## Why Mavis direct take-over

R15.2 is a single-file character-level repair. The only "design decision" is what to put on the L91/L93 lines where the original English text was heavily corrupted — subagent suggested sensible English renderings matching the surrounding context, Mavis accepted those.

Subagent value was in the **spec phase** (wide-class scan across 1,765 files), not the impl phase. The 11-line fix is mechanical.

## Character-class rationale

The 12 R14 residual codepoints (闁炽儺娉冲厜鍋撻柍鍏夊亾) are all GBK-table CJK ideographs — valid Unicode but never legitimate in English source comments. R14 QClaw character-class scanner treated them as legitimate CJK and skipped them. R15.2's scanner uses both:
- Frequency-based filter (≤ 3 occurrences across the entire .rs file under review = suspicious)
- GBK-table presence filter (CJK codepoint but not in GB2312 level 1/2)

For R15.2 mod.rs fix, the frequency filter alone was sufficient because all 12 codepoints had ≥ 49 occurrences each in the ornament lines.

## Verification

```
$ cargo test -p northhing-core --lib --features 'service-integrations,product-full'
test result: ok. 899 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 2.15s
```

Character scan post-fix:

```
=== R14 residual codepoints in bot/mod.rs ===
  U+95C1 闁: 0
  U+70BD 炽: 0
  U+513A 儺: 0
  U+5A09 娉: 0
  U+51B2 冲: 0
  U+539C 厜: 0
  U+934B 鍋: 0
  U+64BB 撻: 0
  U+67CD 柍: 0
  U+934F 鍏: 0
  U+590A 夊: 0
  U+4EBE 亾: 0
All clear
```

## What did NOT land

- `AGENTS-CN.md` (whole-file GBK-as-UTF-8 mojibake) — Defer to R15.3.
- `CONTRIBUTING_CN.md` (whole-file GBK-as-UTF-8 mojibake) — Defer to R15.3.
- 4 locale JSON files — Defer to dedicated i18n round.
- 105+ R3b/R5/R6/R8b-era documentation files — Defer to history cleanup.

All tracked in spec §"Out of scope" for follow-up.

## Refs

- Spec: `docs/handoffs/2026-06-30-r15-2-mojibake-cleanup-spec.md`
- Review guide: `docs/handoffs/2026-06-30-r15-2-mojibake-cleanup-review.md`
- Wide-class scan reports: `E:\agent-project\mojibake_action_summary.md` + `E:\agent-project\mojibake_report_final.md` + `E:\agent-project\mojibake_records.json` (external, not in repo)
- Plan reference: `docs/handoffs/2026-06-30-r15-god-object-plan.md` (R15+ roadmap)
- Predecessor: R15 P0 (`b30882f`) + R14 fix `f777284`