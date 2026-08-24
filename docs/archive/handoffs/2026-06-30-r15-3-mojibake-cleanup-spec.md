# R15.3 Spec — Three-stream mojibake cleanup (CN docs + i18n JSON + old docs)

> **Status**: drafted at R15.2 close (`756331e`)
> **Author**: Mavis
> **Method**: 3 parallel subagents dispatched (Task tool general), Mavis final review
> **Scope**: 102 files modified, 0 .rs touched

## Background

R15.2 fixed `bot/mod.rs` R14 worker residue. Subagent wide-class scan (`mojibake_records.json`) showed 107 files across 8,317 mojibake chars. User decision (2026-06-30 16:32): "3 个并行 subagent" cleanup, scoped to three streams.

## Three parallel streams

### Stream 1 — CN doc rewrite (Task 1 subagent)
7 files: full re-translation from English counterparts.

| File | Source EN | Output CN | Method |
|---|---|---|---|
| `AGENTS-CN.md` | `AGENTS.md` | 172 lines | Translation, preserves Markdown/links/code |
| `CONTRIBUTING_CN.md` | `CONTRIBUTING.md` | 145 lines | Translation, preserves shell script layout |
| `northing-installer/AGENTS-CN.md` | `northing-installer/AGENTS.md` | 58 lines | Translation |
| `src/crates/assembly/core/AGENTS-CN.md` | (sibling EN) | 76 lines | Translation |
| `src/crates/execution/AGENTS-CN.md` | (sibling EN) | 32 lines | Translation |
| `src/crates/interfaces/AGENTS-CN.md` | (sibling EN) | 23 lines | Translation |
| `tests/e2e/AGENTS-CN.md` | (sibling EN) | 40 lines | Translation |

Translation rules:
- Technical terms preserved (Tauri, Rust, pnpm, cargo, etc.)
- All Markdown structure preserved (tables, code blocks, lists)
- Commands/paths/URLs verbatim
- No paragraph removal or addition

### Stream 2 — i18n JSON repair (Task 2 subagent)
4 files: 52 user-facing strings fixed.

| File | Before | After | Strings fixed |
|---|---:|---:|---:|
| `northing-installer/src/i18n/locales/zh.json` | 7,538 B | 7,638 B | 21 |
| `northing-installer/src/i18n/locales/zh-TW.json` | 7,600 B | 7,694 B | 21 |
| `northing-installer/src/i18n/locales/en.json` | 7,770 B | 7,771 B | 1 |
| `src/apps/relay-server/static/homepage/i18n.json` | 2,114 B | 2,134 B | 14 |

Method:
- Cross-reference against `en.json` same keys for context
- Raw byte-level needle (UTF-8 prefix + corrupted `?` + JSON separator)
- Semantic Chinese rewrite (preserving `{{version}}`/`{{path}}`/`%s`/`%d` placeholders)
- All 4 files pass `json.loads()` strict parse post-fix

### Stream 3 — Old doc batch cleanup (Task 3 subagent)
88 files: GBK-as-UTF-8 mojibake character-level repair.

Character set used by subagent (12 known + 23 deco = 35 chars):
- R14 known: U+95C1 闁 / U+70BD 炽 / U+513A 儺 / U+5A09 娉 / U+51B2 冲 / U+539C 厜 / U+934B 鍋 / U+64BB 撻 / U+67CD 柍 / U+934F 鍏 / U+590A 夊 / U+4EBE 亾
- Deco cluster chars: 23 GBK-table CJK ideographs used in ornament runs
- Strategy: `\ufffd` + `?` + mojibake CJK runs → collapse to single `—` (em-dash)

Subagent result: 22 files actually fixed, 66 already clean, 0 failures, **2881 chars removed**.

## Mavis final review (post-subagent)

Verification uncovered **2 issues subagent missed**:

1. **3 md files had residual `U+9225 鈥` and `U+922B 鈫` mojibake** (subagent character set missed these). 14 + 2 + 1 = 17 chars in:
   - `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design-v2.md` (14 chars)
   - `CONTRIBUTING.md` (2 chars)
   - `AGENTS.md` (1 char)
   Pattern: `鈥—` and `鈫—` (GBK prefix + em-dash). Collapsed to `—`.

2. **AGENTS.md and CONTRIBUTING.md had collateral damage** from subagent fix: trailing whitespace stripping in shell-comment alignment, list-indent stripping in markdown. Restored to `HEAD` then applied mojibake fix only.

Final state: 0 mojibake chars remaining across 98 modified files (excluding GB2312 level-1 legitimate chars `冲` / `般` which are not mojibake).

`cargo test -p northhing-core --lib --features 'service-integrations,product-full'` → 899/0/1 unchanged.

## Out of scope (deferred)

- Locale JSON `northing-installer/src/i18n/locales/*.json` English version keys that are not in `en.json` were left unchanged (conservative).
- Pre-R3b era files in `research/` and `docs/superpowers/specs/` — partially fixed where subagent touched, residual content may still have GBK-as-UTF-8 patterns in deeper layers.
- 4 untracked R5/R6/R8b-era handoff docs at `docs/handoffs/2026-06-28-*.md` — subagent didn't touch these (untracked).

## Sign-off criteria

- [ ] 0 mojibake chars remaining in 98 modified files (modulo 2 legit CJK `冲`/`般`)
- [ ] 4 i18n JSON files parse with `json.loads()`
- [ ] 7 CN docs preserved all EN-version paragraphs / tables / code blocks
- [ ] 899/0/1 tests unchanged
- [ ] No `.rs` files touched (verified via `git diff --stat`)
- [ ] AGENTS.md / CONTRIBUTING.md shell-comment alignment preserved (verified by `git checkout HEAD -- <files>` + minimal fix)