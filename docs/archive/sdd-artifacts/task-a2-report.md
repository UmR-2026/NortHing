# Task A2 Report — topics/extract.rs

## Status

**DONE**

## File stats

- **File**: `src/agentic/src/topics/extract.rs`
- **Lines**: 497 actual (426 per `Measure-Object -Line`; within the 800-line limit)
- **Commit**: `68d3909 feat(growth): add dependency-free topic extraction`

## Verification commands

### `cargo check -p northhing-agentic-growth`

```
   Compiling proc-macro2 v1.0.106
   Compiling quote v1.0.46
   Compiling unicode-ident v1.0.24
   Compiling serde_core v1.0.228
   Compiling zmij v1.0.21
   Compiling thiserror v2.0.18
   Compiling serde_json v1.0.150
    Checking once_cell v1.21.4
   Compiling serde v1.0.228
    Checking itoa v1.0.18
    Checking pin-project-lite v0.2.17
    Checking memchr v2.8.3
    Checking tracing-core v0.1.36
   Compiling syn v2.0.118
   Compiling tracing-attributes v0.1.31
   Compiling thiserror-impl v2.0.18
   Compiling serde_derive v1.0.228
   Compiling async-trait v0.1.89
    Checking tracing v0.1.44
    Checking northhing-agentic-growth v0.2.10 (E:\agent-project\northing\.worktrees\growth-a2\src\agentic)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 13.80s
```

### `cargo test -p northhing-agentic-growth`

```
   Compiling windows-link v0.2.1
   Compiling parking_lot_core v0.9.12
   Compiling scopeguard v1.2.0
   Compiling cfg-if v1.0.4
   Compiling smallvec v1.15.2
   Compiling pin-project-lite v0.2.17
   Compiling once_cell v1.21.4
   Compiling itoa v1.0.18
   Compiling memchr v2.8.3
   Compiling bytes v1.12.1
   Compiling serde_core v1.0.228
   Compiling zmij v1.0.21
   Compiling tokio-macros v2.7.0
   Compiling thiserror v2.0.18
   Compiling windows-sys v0.61.2
   Compiling lock_api v0.4.14
   Compiling tracing-core v0.1.36
   Compiling tracing v0.1.44
   Compiling parking_lot v0.12.5
   Compiling serde_json v1.0.150
   Compiling serde v1.0.228
   Compiling mio v1.2.1
   Compiling socket2 v0.6.4
   Compiling tokio v1.52.3
   Compiling northhing-agentic-growth v0.2.10 (E:\agent-project\northing\.worktrees\growth-a2\src\agentic)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 13.77s
     Running unittests src\lib.rs (target\debug\deps\northhing_agentic_growth-f6dc5dbd6f97d99a.exe)

running 16 tests
test error::tests::error_display_includes_context ... ok
test topics::extract::tests::ascii_case_normalized_to_lowercase ... ok
test topics::extract::tests::duplicate_tokens_are_deduplicated ... ok
test topics::extract::tests::long_cjk_topic_is_truncated_by_char_count ... ok
test topics::extract::tests::ascii_stopwords_are_filtered ... ok
test topics::extract::tests::empty_input_yields_empty_result ... ok
test topics::extract::tests::at_most_max_topics_returned ... ok
test topics::extract::tests::same_input_produces_same_output_twice ... ok
test topics::extract::tests::cjk_stopwords_are_filtered ... ok
test topics::extract::tests::only_punctuation_yields_empty_result ... ok
test topics::extract::tests::only_whitespace_yields_empty_result ... ok
test topics::extract::tests::pure_ascii_filters_stopwords_and_short_tokens ... ok
test topics::extract::tests::pure_cjk_keeps_contiguous_run_as_one_topic ... ok
test topics::extract::tests::pure_digit_tokens_are_filtered ... ok
test topics::extract::tests::mixed_cjk_ascii_contains_both_kinds ... ok
test topics::extract::tests::short_words_and_single_chars_yield_empty ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

**Result: 16 passed, 0 failed.**

## Test expectations (§3 of brief)

| # | Test name | Input | Expected output (actual hardcoded assertion) |
|---|-----------|-------|---------------------------------------------|
| 1 | `pure_ascii_filters_stopwords_and_short_tokens` | `"I prefer pnpm for dependency install"` | `vec!["prefer", "pnpm", "dependency"]` — `I` is <3 chars filtered, `for` is stopword filtered, `prefer`/`pnpm`/`dependency` kept as first 3 (cap) |
| 2 | `pure_cjk_keeps_contiguous_run_as_one_topic` | `"用户偏好使用中文回复"` | `vec!["用户偏好使用中文回复"]` — no delimiters, kept as a single CJK run (no segmentation) |
| 3 | `mixed_cjk_ascii_contains_both_kinds` | `"以后依赖安装都用 pnpm，不要用 npm"` | `vec!["以后依赖安装都用", "pnpm", "不要用"]` — CJK run + ASCII token `pnpm` + CJK run `不要用`; `npm` excluded by cap at MAX_TOPICS=3 |
| 4a | `ascii_stopwords_are_filtered` | `"the quick brown fox"` | `vec!["quick", "brown", "fox"]` — `the` filtered as stopword |
| 4b | `cjk_stopwords_are_filtered` | `"我们 吃饭"` | `vec!["吃饭"]` — `我们` filtered as CJK stopword |
| 5 | `short_words_and_single_chars_yield_empty` | `"a b c 的 了"` | `vec![]` — all are single chars < min threshold |
| 6 | `pure_digit_tokens_are_filtered` | `"2026 18"` | `vec![]` — pure ASCII digits discarded |
| 7 | `long_cjk_topic_is_truncated_by_char_count` | `"这是一个超过三十个字符的中文测试字符串用于验证截断功能是否正常工作"` (33 chars) | Single topic truncated to 24 chars (`MAX_TOPIC_CHARS`), verified by `chars().count() == MAX_TOPIC_CHARS` and exact string match of first 24 chars |
| 8 | `duplicate_tokens_are_deduplicated` | `"pnpm pnpm PNPM"` | `vec!["pnpm"]` — case-folded dedup, single result |
| 9 | `at_most_max_topics_returned` | `"rust python javascript golang typescript"` | `vec!["rust", "python", "javascript"]` — 5 valid → capped to first 3 |
| 10a | `empty_input_yields_empty_result` | `""` | `vec![]` |
| 10b | `only_punctuation_yields_empty_result` | `",.?!"` | `vec![]` |
| 10c | `only_whitespace_yields_empty_result` | `"   \t\n  "` | `vec![]` |
| 11 | `same_input_produces_same_output_twice` | `"rust python javascript"` | `extract_topics` called twice, results compared with `assert_eq!` — deterministic |
| 12 | `ascii_case_normalized_to_lowercase` | `"PNPM"` | `vec!["pnpm"]` — uppercase normalized to lowercase |
| 13 | `ascii_connector_chars_survive_inside_tokens` | `"用 node-18 跑 src/agentic 的 C++ 好"` | `vec!["node-18", "src/agentic", "c++"]` — connector chars preserved inside tokens; single CJK chars filtered |
| 14 | `connector_chars_stripped_from_ends` | `"pnpm. --flag"` | `vec!["pnpm", "flag"]` — trailing/leading connectors stripped |

## Git log and status

```
$ git log --oneline -1
2816b47 feat(growth): add dependency-free topic extraction

$ git status --short
(clean — no output)
```

## Deviations from brief (original)

No spec deviations in the original implementation, except:
- The brief §2.2 rule 1 and rule 2 had a contradiction: `is_ascii_punctuation` treats `-`, `_`, `.`, `+`, `/` as delimiters, but rule 2 explicitly lists those characters as allowed inside ASCII tokens (with examples `node-18`, `src/agentic`, `C++`). The original implementation followed rule 1 (delimiter list), which broke the rule 2 examples. This was fixed in the review round (see below).

---

## Review Fix Round

### Important fix: S-1 — Connector characters split by `is_ascii_punctuation`

**Root cause:** `is_delimiter` used `c.is_ascii_punctuation()` which returns `true` for `-`, `_`, `.`, `+`, `/`. The brief §2.2 rule 2 lists these as allowed inside ASCII tokens (`node-18`, `src/agentic`, `C++`).

**Fix applied:**
1. In `is_delimiter`, excluded connector chars from ASCII punctuation: `c.is_ascii_punctuation() && !matches!(c, '-' | '_' | '.' | '+' | '/')`
2. Added `trim_connector_chars()` function that strips leading/trailing `-`, `_`, `.`, `+`, `/` from ASCII tokens, with a fallback: if stripping would shrink below `MIN_ASCII_TOKEN_CHARS` but the original is long enough, keep the original (preserves tokens like `c++`).
3. Updated `process_segment` to apply trimming before length/pure-digit/stopword checks.
4. Simplified ASCII token branch predicate: `segment.chars().all(is_ascii_token_char)` (removed redundant `is_ascii_alphanumeric()` per Q-3).
5. Updated doc comments on `extract_topics` and `is_delimiter` to reflect the connector trim step.

### Minor fixes

| Finding | Action taken |
|---------|-------------|
| **Q-1** (report line count 365 vs actual 497) | Updated report to show 497 actual lines (426 per `Measure-Object -Line`) |
| **Q-2** (CJK chars in `//` line comments in tests) | Rewrote two comments in English: line 291 (mixed test) and line 321 (CJK stopwords test) |
| **Q-3** (redundant `is_ascii_alphanumeric()` in predicate) | Collapsed to `segment.chars().all(is_ascii_token_char)` — same behavior, less code |

### New tests added

| Test name | Input | Expected output | Purpose |
|-----------|-------|-----------------|---------|
| `ascii_connector_chars_survive_inside_tokens` | `"用 node-18 跑 src/agentic 的 C++ 好"` | `vec!["node-18", "src/agentic", "c++"]` | Verify `-`, `/`, `+` survive inside tokens |
| `connector_chars_stripped_from_ends` | `"pnpm. --flag"` | `vec!["pnpm", "flag"]` | Verify trailing `.` and leading `--` are stripped |

### Verification (review fix)

```
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
```

Output:

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.61s
     Running unittests src\lib.rs (target\debug\deps\northhing_agentic_growth-f6dc5dbd6f97d99a.exe)

running 18 tests
test error::tests::error_display_includes_context ... ok
test topics::extract::tests::ascii_case_normalized_to_lowercase ... ok
test topics::extract::tests::ascii_connector_chars_survive_inside_tokens ... ok
test topics::extract::tests::ascii_stopwords_are_filtered ... ok
test topics::extract::tests::at_most_max_topics_returned ... ok
test topics::extract::tests::cjk_stopwords_are_filtered ... ok
test topics::extract::tests::connector_chars_stripped_from_ends ... ok
test topics::extract::tests::duplicate_tokens_are_deduplicated ... ok
test topics::extract::tests::empty_input_yields_empty_result ... ok
test topics::extract::tests::long_cjk_topic_is_truncated_by_char_count ... ok
test topics::extract::tests::mixed_cjk_ascii_contains_both_kinds ... ok
test topics::extract::tests::only_punctuation_yields_empty_result ... ok
test topics::extract::tests::only_whitespace_yields_empty_result ... ok
test topics::extract::tests::pure_ascii_filters_stopwords_and_short_tokens ... ok
test topics::extract::tests::pure_cjk_keeps_contiguous_run_as_one_topic ... ok
test topics::extract::tests::pure_digit_tokens_are_filtered ... ok
test topics::extract::tests::same_input_produces_same_output_twice ... ok
test topics::extract::tests::short_words_and_single_chars_yield_empty ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```
cargo check -p northhing-agentic-growth
```

Output:

```
     Checking northhing-agentic-growth v0.2.10 (E:\agent-project\northing\.worktrees\growth-a2\src\agentic)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.84s
```

**Result: 18 passed, 0 failed; cargo check clean.**

### Commit (amended)

```
2816b47 feat(growth): add dependency-free topic extraction
```

`git status --short`: clean, only `src/agentic/src/topics/extract.rs` in commit.

### Remaining items from review

All 3 Minor items were addressed (report line count corrected, CJK comments rewritten in English, redundant predicate simplified). No items left open.
