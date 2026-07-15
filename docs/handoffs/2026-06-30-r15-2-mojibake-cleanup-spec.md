# R15.2 Spec — Mojibake Debt Cleanup (R14 worker residue)

> **Status**: drafted at R15 close (`b30882f`)
> **Author**: Mavis (after subagent wide-class scan + manual verification)
> **User mode**: personal pace, side project
> **Scope**: tight — only R14 refactor commit's introduced mojibake

## Background

R14 worker (`split_command_router.py` running on Chinese Windows) read UTF-8 source files with the system default `gbk` encoding and wrote back as UTF-8. The encoding round-trip corrupted every em-dash `—` (and every other non-ASCII punctuation) into a `U+95C1 U+003F` ("闁?") pattern, plus generated 17× ornament clusters of `闁冲厜鍋撻柍鍏夊亾` on comment-divider lines.

R14 fix commit `f777284` (`fix(fmt+imports+mojibake): R14 cargo fmt fixup + minor observations`) repaired the most common mojibake markers (`—`, `…`, control chars 0x80-0x9F), but its character-class scanner was too narrow — it missed the 12-character set of GBK-table CJK ideographs that survived in `bot/mod.rs`.

## Scope (tight, R14 only)

### In scope

1. `src/crates/assembly/core/src/service/remote_connect/bot/mod.rs` — 10 mojibake lines + 1 trailing-comment duplication on L1.
   - 8 lines with `闁?` (U+95C1 + U+003F) — replaced with `—` (em-dash) or `:` depending on context.
   - 3 lines with 17× ornament cluster of `闁冲厜鍋撻柍鍏夊亾` — replaced with clean ASCII header `// ===== TITLE =====`.
   - L91 and L93 had multi-byte corruption (`闁炽儺娉?line per` and `闁?they were`) — replaced with the clean English rendering matching the surrounding context.

### Out of scope (deferred to R15.3+)

1. **`AGENTS-CN.md` (31 lines, 42 mojibake chars)** — Whole-file GBK-as-UTF-8 mojibake, not character-level replaceable. Original Chinese is GBK-encoded and was double-decoded. Needs either full re-translation from the English `AGENTS.md` or a GBK→UTF-8 transcode against an external source. **Defer to R15.3**.
2. **`CONTRIBUTING_CN.md` (18 lines, 24 mojibake chars)** — Same as above. **Defer to R15.3**.
3. **4 locale JSON files** (`northing-installer/src/i18n/locales/{zh,zh-TW,en}.json` + `src/apps/relay-server/static/homepage/i18n.json`) — User-facing i18n strings with U+FFFD + `?` placeholders. R14 unrelated. **Defer to a dedicated i18n round**.
4. **105+ R3b/R5/R6/R8b-era documentation files** (`docs/PROJECT_STATE.md`, `docs/sdlc-harness/*.md`, etc.) — Whole-file mojibake from earlier split scripts. **Defer to R-X.Y history cleanup, not R15.2**.
5. **Other repo crates** — R14 only touched `bot/`. Other crates have no R14 residual.

## Why tight scope

User explicitly asked for "R14 范围" cleanup. R14 commit `ed35b81` only added mojibake to `bot/mod.rs` (verified via `git log -S "闁冲厜鍋撻"`). Other files with mojibake predate R14 and are separate cleanup rounds.

## Character-class evidence (R14 worker 漏扫)

| Codepoint | Char | Name | R14 residual sites |
|---|---|---|---|
| U+95C1 | 闁 | CJK UNIFIED IDEOGRAPH-95C1 | bot/mod.rs (50×), AGENTS-CN.md (×), CONTRIBUTING_CN.md (×) |
| U+70BD | 炽 | CJK UNIFIED IDEOGRAPH-70BD | bot/mod.rs L91 |
| U+513A | 儺 | CJK UNIFIED IDEOGRAPH-513A | bot/mod.rs L91 |
| U+5A09 | 娉 | CJK UNIFIED IDEOGRAPH-5A09 | bot/mod.rs L91 |
| U+51B2 | 冲 | CJK UNIFIED IDEOGRAPH-51B2 | ornament cluster |
| U+539C | 厜 | CJK UNIFIED IDEOGRAPH-539C | ornament cluster |
| U+934B | 鍋 | CJK UNIFIED IDEOGRAPH-934B | ornament cluster |
| U+64BB | 撻 | CJK UNIFIED IDEOGRAPH-64BB | ornament cluster |
| U+67CD | 柍 | CJK UNIFIED IDEOGRAPH-67CD | ornament cluster |
| U+934F | 鍏 | CJK UNIFIED IDEOGRAPH-934F | ornament cluster |
| U+590A | 夊 | CJK UNIFIED IDEOGRAPH-590A | ornament cluster |
| U+4EBE | 亾 | CJK UNIFIED IDEOGRAPH-4EBE | ornament cluster |

All 12 codepoints are valid GBK characters but invalid choices for English source comments. R14 QClaw character-class scanner treated them as legitimate CJK and skipped them.

## Repair strategy (character-level)

| Line | Original (UTF-8 corrupted) | Repaired |
|---|---|---|
| L1 | `//! Bot integration for Remote Connect.//!` | `//! Bot integration for Remote Connect.` |
| L56 | `/// Persisted bot connection 闁?saved to disk...` | `/// Persisted bot connection — saved to disk...` |
| L91 | `/// `[Thinking] 闁炽儺娉?line per `ThinkingEnd`) are forwarded...` | `/// `[Thinking]` blocks are forwarded to the user, one line per `ThinkingEnd` event.` |
| L93 | `/// mode 闁?they were too noisy for IM channels...` | `/// These messages are too noisy for IM channels...` |
| L116 | `// 闁冲厜鍋撻柍鍏夊亾 Shared workspace-file utilities 闁冲厜...(×16)` | `// ===== Shared workspace-file utilities =====` |
| L206 | `// 闁冲厜鍋撻柍鍏夊亾 Downloadable file link extraction 闁...(×15)` | `// ===== Downloadable file link extraction =====` |
| L208 | `/// Extensions that are source-code / config files 闁?excluded...` | `/// Extensions that are source-code / config files — excluded...` |
| L371 | `// Phase 1 闁?protocol-prefixed links...` | `// Phase 1: protocol-prefixed links...` |
| L394 | `// Phase 2 闁?markdown hyperlinks...` | `// Phase 2: markdown hyperlinks...` |
| L429 | `// 闁冲厜鍋撻柍鍏夊亾 Auto-push file delivery helpers 闁...(×15)` | `// ===== Auto-push file delivery helpers =====` |
| L588 | `/// assistant-mode replies silently dropped attachments 闁?see` | `/// assistant-mode replies silently dropped attachments — see` |

## Implementation method

- Direct take-over by Mavis (no subagent dispatch).
- Python script `E:\agent-project\.mavis\tmp-r15-2\fix_mod_rs.py` does the character-level repair with strict UTF-8 IO.
- All 11 replacements verified in dry-run before write.
- File written with `Path.write_text(..., encoding='utf-8')` (no BOM, no PowerShell pipeline).

## Verification

- `cargo check -p northhing-core --tests --features 'service-integrations,product-full'` clean (only pre-existing dead-code warnings).
- `cargo test -p northhing-core --lib --features 'service-integrations,product-full'` → 899/0/1 unchanged.
- Character-class scan: 0 R14 residual codepoints remaining in `bot/mod.rs`.
- Diff scope: 1 file, 11 line replacements, net `-406 bytes`.

## Why no plan YAML / subagent dispatch

Same logic as R15 P0: this is a single-file 11-line character-level repair. No design decisions to defer. The subagent was used at the spec stage for the wide-class scan (which is what R15.2 needed fresh-context for), and Mavis executes the repair itself.

## Sign-off criteria for R15.2

- [ ] 0 R14 residual codepoints (12-char set) in `bot/mod.rs`
- [ ] 3 ornament clusters removed, ASCII headers in place
- [ ] 899/0/1 tests unchanged
- [ ] No unrelated diff (no cargo fmt noise, no character drift outside the 11 lines)
- [ ] UTF-8 clean write (verified via byte-level re-read)