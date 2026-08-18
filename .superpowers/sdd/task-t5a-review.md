# Review — Task T5a (move distiller pure logic into the growth crate)

BASE `2e986ce` -> HEAD `71df0dd`, branch `feat/growth-core-0804`.

Evidence base: final source files in the worktree (`parse.rs` 389 lines, `prompt.rs` 128 lines,
`distiller.rs` 416 lines, `distill/mod.rs` 4 lines, `src/agentic/AGENTS.md` 44 lines), the
pre-move implementation via `git show 2e986ce:...distiller.rs` (656 lines), and
`facts.rs:33-53` for the host enum variant names. Test counts, warning count, line counts,
file scope, dependency purity, absence of catch-all arms, and prompt byte-identity were
independently verified by the orchestrator and are taken as given (not re-run).

## Summary

This is a genuine behavior-preserving refactor. The parse loop in `parse.rs:92-148` is a
line-for-line transposition of `2e986ce:distiller.rs:360-422` with only the fact-construction
tail replaced by the neutral `DistilledFact`, and the two helpers (`strip_json_fence`,
`deserialize_keywords`) plus `RawDistilledFact` moved verbatim. No skip became a default, no
enum arm is transposed, the tolerant keywords deserializer is intact, and D15's
debug-vs-warn split is preserved at the same levels with the same message text. All 13 tests
survive with their names and input JSON, and the two assertions that could not survive the
type change were relocated to core rather than dropped.

Findings are two Minors, both documentation-level. Nothing blocks the merge.

## 1. §4 equivalence checklist, row by row

| # | Behavior | Pre-move | Post-move | Verdict |
|---|---|---|---|---|
| 1 | Max 3 facts, cap checked **before** processing each item | `:361-363` | `parse.rs:94-96` — `if facts.len() >= MAX_DISTILL_FACTS { break; }` is the first statement in the loop body, ahead of the `text` extraction | Unchanged |
| 2 | `text`: `trim()` -> skip if empty -> truncate to 300 **chars** | `:364-373` | `parse.rs:97-106` — `t.trim()`, `if t.is_empty() { continue }`, `t.chars().take(MAX_FACT_TEXT_CHARS).collect()`. Char-based, not byte-based. `None => continue` preserved | Unchanged |
| 3 | Unknown/missing `type` -> **skip item** | `:374-380` | `parse.rs:107-113` — four literal arms `user/feedback/project/reference`, then `_ => continue` | Unchanged |
| 4 | Unknown/missing `confidence` -> skip; `high`/`med`/`low` | `:381-386` | `parse.rs:114-119` — three literal arms, `_ => continue` | Unchanged |
| 5 | Unknown/missing `scope` -> skip; `workspace`/`global` | `:387-391` | `parse.rs:120-124` — two literal arms, `_ => continue` | Unchanged |
| 6 | Keywords: turn-level union, trimmed, empties dropped, order-preserving dedup, first occurrence wins | `:397-407` | `parse.rs:130-140` — identical body: `kw.trim()`, `if kw.is_empty() { continue }`, `if !keyword_set.iter().any(|k| k == &kw) { keyword_set.push(kw) }` on a `Vec<String>` (order-preserving by construction) | Unchanged |
| 7 | `was_empty_array` true **only** for a valid empty array, **false** on parse failure | `:346`, `:351` | `parse.rs:88` `raw_facts.is_empty()` on the success path; `parse.rs:81` hard-codes `was_empty_array: false` on the `Err` path. Adapter passes `outcome.was_empty_array` through unmodified (`distiller.rs:319`) | Unchanged |
| 8 | `deserialize_keywords` never returns `Err` | `:474-491` | `parse.rs:205-222` — body is byte-identical (see §4 below) | Unchanged |
| 9 | `strip_json_fence` handling of ```` ```json ```` fences | `:427-440` | `parse.rs:158-171` — identical: `trim`, strip leading ```` ``` ````, then optional `json`, `trim_start`, strip trailing ```` ``` ````, `trim().to_string()` | Unchanged |
| 10 | Prompt text byte-identical incl. 500-char assistant truncation | `:260-325` | `prompt.rs:26-84`; user part built by the same `format!`/`push_str` pair with `chars().take(MAX_ASSISTANT_TEXT_CHARS)`; adapter reassembles `Message::system(prompt.system)` then `Message::user(prompt.user)` at `distiller.rs:97-101`, same order as `:321-324` | Unchanged (byte-identity per orchestrator) |

Two second-order equivalences that the checklist does not name explicitly but that a careless
move would have broken, both verified preserved:

- **Keywords of a skipped item are discarded.** In both versions the keyword-collection block
  sits *after* the three `_ => continue` arms, so an item rejected for a bad `type`,
  `confidence`, or `scope` contributes no keywords (`2e986ce:397` vs `parse.rs:130`).
- **Keywords of items beyond the 3-fact cap are discarded.** The `break` precedes keyword
  collection in both versions, so a fourth item's keywords never enter the union.
- **All facts of one turn share a single `created_at`.** Pre-move computed `now` once at
  `:353` outside the loop; the adapter computes it once at `distiller.rs:284-287` before the
  `map`, with the same `duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)`
  expression, including the `unwrap_or(0)` fallback. A per-fact `SystemTime::now()` inside the
  closure would have been a silent drift; it was avoided.

## 2. Did any "skip" become a "default"?

**No.** All four rejection paths are literal `continue`s and none of the three neutral enums
derives or implements `Default`:

- `parse.rs:105` — `None => continue` for a missing `text`.
- `parse.rs:112`, `:118`, `:123` — `_ => continue` for `fact_type`, `confidence`, `scope`.

The enums at `parse.rs:18`, `:28`, `:36` derive only
`Debug, Clone, Copy, PartialEq, Eq` — no `Default`, so no `unwrap_or_default()` could even
compile against them. Worth noting as a near-miss the implementer did not step into:
`facts.rs:55-57` defines `default_fact_type() -> FactType::Feedback`, which exists as a serde
default for DB rows. Had the adapter reached for it, unknown-type items would have silently
become `Feedback` facts. It does not appear anywhere in `distiller.rs` (whole file read).
No fabricated metadata.

## 3. Enum mapping pairing

Host variants confirmed at `facts.rs:33-37` (`FactConfidence { High, Med, Low }`), `:41-44`
(`FactScope { Workspace, Global }`), `:48-53` (`FactType { User, Feedback, Project, Reference }`).
The adapter's three matches at `distiller.rs:300-314` are each 1:1 and correctly paired — no
transposition:

- `distiller.rs:301-303` — `High -> High`, `Med -> Med`, `Low -> Low`. The `Med` spelling
  (not `Medium`) matches both the crate variant and the host variant.
- `distiller.rs:306-307` — `Workspace -> Workspace`, `Global -> Global`.
- `distiller.rs:310-313` — `User -> User`, `Feedback -> Feedback`, `Project -> Project`,
  `Reference -> Reference`.

All three are exhaustive with no catch-all arm, so adding a variant on either side becomes a
compile error as §3.4 requires. The adapter tests are also strong enough to catch a
transposition rather than merely compiling: `adapter_parse_maps_all_enum_variants`
(`distiller.rs:357-376`) asserts the exact expected variant for all three fields on three
items covering `user/feedback/project`, `high/med/low`, and `workspace/global`, and
`adapter_parse_maps_reference_variant` (`:379-386`) covers the fourth `FactType` plus
`Global`. Every variant of all three host enums is asserted at least once against a known
input string.

## 4. The tolerant keywords deserializer

**Intact and still incapable of returning `Err` for any parseable JSON value.**
`parse.rs:205-222` is body-identical to `2e986ce:474-491`. The mechanism is unchanged and
still sound:

- `parse.rs:209` deserializes into `Option<serde_json::Value>`, which accepts *every* JSON
  value shape, so the `?` cannot fire on a type mismatch — only on input that is not valid
  JSON at all, which would already have failed the outer `from_str`.
- `parse.rs:211` — absent/`null` -> `Ok(None)`.
- `parse.rs:212-218` — array -> `filter_map(|v| v.as_str())`, so non-string elements are
  dropped and the remaining strings kept; an all-non-string array yields `Ok(Some(vec![]))`.
- `parse.rs:220` — `Some(_) => Ok(None)` catches string, number, **bool**, and object.

The bool case is covered by code (`Some(_)`) but has no dedicated whole-value test; the same
was true pre-move, and `parse_keywords_array_with_non_string_elements_facts_intact`
(`parse.rs:381`) does exercise `true` as an array element. Not a regression, so not a finding.

Also preserved: `#[serde(default, deserialize_with = "deserialize_keywords")]` at
`parse.rs:190`. Dropping the `default` would have made an absent `keywords` field an error;
it is present. The only edit is cosmetic — the derive changed from
`#[derive(Debug, serde::Deserialize)]` to `use serde::Deserialize;` + `#[derive(Debug, Deserialize)]`,
which is equivalent. As a bonus the mojibake `(brief 搂4.3 warn-only + 搂5.11)` that sat in the
pre-move doc comment at `2e986ce:452` was dropped rather than carried across.

## 5. D15 observability

Preserved at the same levels, with a single log site *moved*, not added or removed:

- Parse failure: `warn!` at `distiller.rs:281` — `"Distiller: failed to parse distilled facts JSON: {}"`
  — the message string is character-identical to `2e986ce:345`, only the interpolated binding
  changed from `e` to `err`. It fires exactly when `outcome.parse_error.is_some()`, which the
  parser sets exactly on the `serde_json::from_str` `Err` branch (`parse.rs:77-84`) — the same
  condition as before.
- Explicit `[]`: `debug!` at `distiller.rs:140-143`, unchanged from base.
- Zero valid facts from a non-empty response: `debug!` at `distiller.rs:149-152`, unchanged.

`warn` and `debug` are both still imported and both still used (`distiller.rs:20`), and the
warn/debug distinction that D15 depends on is therefore still observable from logs alone.

## 6. The 13 moved tests

All 13 are present in `parse.rs` with the **same names** and, on comparison against
`2e986ce:497-655`, the **same input JSON** strings:

`parse_valid_json_array_maps_fields` (:229), `parse_json_fence_wrap` (:243),
`parse_bad_json_returns_empty` (:253), `parse_four_items_truncates_to_three` (:263),
`parse_unknown_fact_type_skipped_valid_kept` (:275), `parse_text_over_300_chars_truncated` (:286),
`parse_empty_array_returns_empty` (:298), `parse_keywords_union_dedup_across_items` (:313),
`parse_legacy_json_without_keywords_still_works` (:325),
`parse_keywords_wrong_type_ignored_facts_intact` (:339),
`parse_keywords_wrong_type_number_ignored_facts_intact` (:357),
`parse_keywords_wrong_type_object_ignored_facts_intact` (:368),
`parse_keywords_array_with_non_string_elements_facts_intact` (:379).

Assertion strength is preserved or improved; no count assertion degraded into an
`is_empty()`:

- Value assertions kept their exact expected values: `chars().count() == 300` (:294),
  `facts.len() == 3` (:271), `keywords == vec!["pnpm", "node-18", "npm"]` (:321),
  `keywords == vec!["pnpm"]` (:386).
- Seven tests gained a `parse_error.is_none()` / `is_some()` assertion (e.g. :239, :259, :307)
  that the pre-move tests could not make — a strengthening.
- The only assertions that could not survive the type change are the two provenance checks in
  `parse_valid_json_array_maps_fields` (`2e986ce:506-507`). They were **relocated, not
  dropped**: `adapter_parse_hydrates_fact_fields` (`distiller.rs:336-337`) asserts
  `provenance.session_id == "s1"` and `provenance.turn_id == "t1"` against the same input JSON.

The §5 core-side adapter requirements are all met: non-empty id (`distiller.rs:335`), unique
ids across facts (`:353`), `created_at > 0` (`:338`), provenance (`:336-337`),
`schema_version == 1` (`:334`), and per-variant enum mapping (`:357-386`). The §5 crate-side
prompt tests exist too: `build_distill_prompt_wraps_user_input` (`prompt.rs:97`) asserts the
`<user_message>` wrapper and the absence of an `<assistant_reply>` block when there is no
assistant text, and `build_distill_prompt_truncates_assistant_text_to_500` (`prompt.rs:111`)
extracts the text *between* the tags and asserts `chars().count() == 500` on a 600-char input —
a real truncation assertion, not a length-of-whole-prompt proxy.

## 7. Dead code left behind in core

I read all 416 lines of `distiller.rs` rather than relying on the warning count (core's
`lib.rs` suppresses both `dead_code` and `unused_imports`). Nothing is stranded:

- Every import at `distiller.rs:7-20` has a live use: the `facts::` group is consumed by the
  hydration `map` (`:292-316`) and `distill_facts_from_user_message` by the fallback paths;
  `Message` at `:99-100`; `Arc`/`Duration` in signatures and the timeout; `debug`/`warn` as
  above; and all four crate-side imports (`:13-17`) are used at `:278`, `:301`, `:306`, `:310`,
  `:97`.
- The three moved constants (`MAX_DISTILL_FACTS`, `MAX_FACT_TEXT_CHARS`,
  `MAX_ASSISTANT_TEXT_CHARS`) are gone from core, and a repo-wide grep across
  `src/crates/assembly/core/src` finds no remaining reference to them or to
  `build_distillation_messages`, `RawDistilledFact`, or `deserialize_keywords`.
- `MIN_USER_INPUT_CHARS` (`:24`, used at `:64`) and `DISTILL_TIMEOUT_SECS` (`:26`, used at
  `:105` and `:118`) correctly stayed, per §3.3.
- The `serde_json::from_str` call left with the parser, and core's `distiller.rs` no longer
  names `serde` or `serde_json` anywhere.

One thing I checked because removing a helper could have broken a caller: `strip_json_fence`
is still referenced at `dream.rs:239`, but `dream.rs:267` defines its **own private copy** —
that duplication predates this commit, and `dream.rs` is explicitly out of scope (T5b). So no
caller was broken. Worth carrying into T5b: `dream.rs` should consolidate onto the crate's
`strip_json_fence` rather than keeping a third copy.

## 8. Crate purity and AGENTS.md

A grep of `src/agentic/src/distill` for `uuid|SystemTime|UNIX_EPOCH|tracing|warn!|debug!|rusqlite|northhing_core|Instant::now`
returns exactly one hit: `parse.rs:4`, inside a `//!` doc comment describing the host's job.
No clock access, no logging, no host types, no id generation in crate code. `distill/mod.rs`
exposes `pub mod parse; pub mod prompt;` and `lib.rs:11` already declared `pub mod distill`,
which is why no `lib.rs` edit was needed. The three constants are `pub` as §3.3 requires
(`parse.rs:12`, `:15`, `prompt.rs:9`).

`AGENTS.md` §4 replaced the "Parameters will be filled by subsequent tasks" stub (confirmed
against `2e986ce:src/agentic/AGENTS.md`) with the three parameters at their correct values
3 / 300 / 500, plus a note that the host keeps `MIN_USER_INPUT_CHARS` and
`DISTILL_TIMEOUT_SECS` locally. Substantively correct; see Minor 1 for the header defect.

## Findings

### Minor 1 — `src/agentic/AGENTS.md:35` — parameter table's third column is mislabeled `Owner` but contains the meaning

The header row is `| Parameter | Value | Owner |`, yet the three data rows put the parameter's
*meaning* in that column ("max facts per turn", "max chars per fact text", "max chars of
assistant reply included as prompt context"). Since the parameter name is already fully
qualified (`distill::parse::MAX_DISTILL_FACTS`), the owner is redundant and the meaning is what
was actually wanted — brief §3.3 asks for "their values and meaning". This matters only because
this table is the crate's authoritative parameter registry per §4; a wrong header invites the
next task to append an actual owner into the meaning column and drift the format.

Smallest fix: rename the header cell.

```
| Parameter | Value | Meaning |
```

### Minor 2 — `src/agentic/src/distill/parse.rs:65` (and `:157`) — unterminated ```` ``` ```` fence in rustdoc on public API

`/// Tolerates ```json fence wrapping. All fields are optional in the` opens a Markdown code
fence that is never closed inside the doc comment, so rustdoc renders the remaining four lines
of that doc block (the "Unknown enum values cause the entry to be skipped", "Max 3 facts", and
keywords/`was_empty_array` paragraphs) as a `json` code block instead of prose. `parse.rs:157`
has the same construct on the private `strip_json_fence`. The text is a verbatim carry-over
from `2e986ce:329` and `:426`, so this is not a regression — but it moved from a private
function in an application crate to the **documented public entry point of a library crate**,
where the rendered docs are the API surface, and it is now duplicated because
`distiller.rs:265` kept its copy of the same sentence. It does not fail the build: the info
string's first word is `json`, so rustdoc does not treat it as a Rust doctest.

Smallest fix: escape the backticks in the two crate-side doc comments, e.g.

```
/// Tolerates ` ```json ` fence wrapping. All fields are optional in the
```

or reword to "Tolerates triple-backtick json fence wrapping."

## Non-blocking observations (no action required)

- `distiller.rs:284-287` computes `now` unconditionally, including on the parse-failure and
  empty-array paths where the `map` produces nothing; pre-move it was computed only after a
  successful `from_str`. One wasted `SystemTime::now()` on a cold error path, zero behavioral
  difference. I deliberately do **not** recommend moving it into the `map` closure — that would
  break the shared-`created_at`-per-turn property verified in §1.
- Report line 67 states the core `distill` filter was 37 before and 31 after
  (37 - 13 moved + 7 new = 31), while brief §7.2 anticipated "13 now". The brief's 13 counted
  the tests inside `distiller.rs`; the `distill` filter matches more test names than that. The
  report's arithmetic is internally consistent and matches the orchestrator's independently
  verified 31, so this is a brief-side estimate, not an implementer error.
- The report's "Concerns" entry (crate parse tests conceptually overlapping the 7 adapter
  tests) is accurate and benign: the crate tests pin parse semantics, the adapter tests pin
  hydration and enum pairing. The overlap is the boundary being tested from both sides, which
  is what caught nothing here only because nothing was wrong. Keep them.

## Cannot verify

Nothing material. Two items are delegated rather than independently re-derived by me, per the
review brief's explicit instruction not to re-run them: (a) the prompt system-string
byte-identity (2886 chars) and the 154 / 31 / 8 / 12 test counts and 19-warning baseline, which
the orchestrator confirmed; and (b) actual test execution — my judgement of the tests is by
reading their bodies, not by observing them pass. My §1 row 10 verdict therefore rests on the
orchestrator's byte comparison plus my own reading that `prompt.rs:80-84` and
`distiller.rs:97-101` reproduce `2e986ce:315-324`'s construction and message order.

SPEC: PASS
QUALITY: PASS
