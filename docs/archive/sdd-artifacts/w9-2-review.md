# W9-2 Review — Memory Browser Panel (read-only)

**Verdict: CAN MERGE** (re-review of fix commit `d02502e`; both Important
issues resolved, no regressions. Remaining items are Minor → terminal triage.)

The implementation correctly fulfills the spec at the contract and facade layers
and respects the TH-3 read-only philosophy. The two Important issues from the
first review (silent export failure, duplicate theme tracking) are fixed
correctly and minimally in `d02502e`. Tests remain smoke-only and don't
validate DTO field mapping (Minor, M-1).

---

## Critical

_None._

The TH-3 philosophy hard constraint is honored: zero edit/delete/forget
mutation entry points in the UI. Only `list_facts` / `search_facts` are
called; the JSONL export writes to `<config_dir>/northhing/exports/...`, not
to `MemoryDb`. No method on the new trait surface mutates memory.

---

## Important

### I-1. Export writes silently fail and still display the path

`pages_memory.rs:229,246`:

```rust
let _ = std::fs::create_dir_all(&dir);
...
let _ = std::fs::write(&path, content);
export_path.set(path.to_string_lossy().to_string());
```

If `create_dir_all` or `write` fails (permission denied, disk full, AV
locking, sandbox), the user still sees a green "exported to <path>" notice
with no error. The brief says "完成后 UI 显示导出路径" — "完成后" implies
"after the write succeeds". This should route failures through `error_msg`
instead of `export_path`, and only set `export_path` on success. Pattern
match `Result` and update the right signal. Trivial fix.

### I-2. Duplicated theme-tracking block regresses `use_page_shell`

`pages_memory.rs:69-88` discards the `Signal<bool>` returned by
`use_page_shell(&props)` (line 69) and then reimplements the exact same
`use_signal` + `use_future` pair that `use_page_shell` already sets up
internally (page_shell.rs:88-102). Result:

- Two `use_future`s subscribe to `theme_rx.changed()` simultaneously.
- The shell's signal is created and updated, but never read — wasted
  work on every theme tick.
- The page regresses the abstraction that `page_shell.rs` was created to
  centralize ("Every Dioxus module window repeats the same lifecycle
  boilerplate... refactor pages_archive.rs and pages_space.rs to use it").

Other module pages (`pages_archive.rs:110`, `pages_space.rs:147`) do
`let mut theme_dark = use_page_shell(&props);` and then `let theme_class = if theme_dark() ...`. The new page should follow that exact pattern.

Functionally harmless (the user-visible `class` reacts correctly because
the local signal also updates), but it's a code smell and a regression of
the page_shell contract. Fix to the canonical pattern.

---

## Minor

### M-1. Tests cover smoke only — DTO field mapping untested

`tests.rs:564-573` adds two `is_ok()` tests:

```rust
async fn test_list_facts_returns_ok() {
    let facade = KernelFacade::new();
    let result = facade.list_facts(None).await;
    assert!(result.is_ok(), ...);
}

async fn test_search_facts_returns_ok() {
    let facade = KernelFacade::new();
    let result = facade.search_facts("anything", None, Some(5)).await;
    assert!(result.is_ok(), ...);
}
```

The brief §5 explicitly says "DTO 转换...若有纯函数则测之". The
enum-to-String mapping in `kernel_facade/memory.rs` (4 match arms per
method, 3 enums × 2 methods = 6 mappings) is the most fragile part of the
diff and is completely untested. The existing pattern
`test_list_episodes_dto_fields_are_correct` (tests.rs:521) checks actual
field values; the new tests should follow suit:

- insert a fact with `FactScope::Global`, `FactConfidence::High`,
  `FactType::Feedback` via the test DB
- call `list_facts(None)`
- assert the DTO round-trips with `"global"`, `"high"`, `"feedback"`
  strings and `session_id` / `turn_id` from provenance

The diff already touches `tests.rs`; adding one more test is in-scope.

### M-2. UI hardcodes `workspace_slug = None`

`pages_memory.rs:115, 156, 190` calls `api::list_facts(None)` /
`api::search_facts(..., None, ...)`. The brief §1 says workspace semantics
mirror `get_facts`: `Some` = global + workspace, `None` = global only.

This is **semantically faithful to `None` semantics** but means the
panel never shows workspace-scoped facts — a usability gap. The
standalone panel has no `workspace_slug` in `ModuleAppProps` today, so
hardcoding `None` is the only option without scope creep. Documented in
the brief's "Resolved ambiguities" only via "no global handle" — the
workspace scope isn't actually resolved. Recommend either:

- Adding `workspace_slug: Option<String>` to `ModuleAppProps` (out of
  scope for W9-2)
- Or explicitly noting in the page comment that this panel is
  global-only (one line)

### M-3. Canonical time helper not used (continues rot-budget overage)

`pages_memory.rs:221-224`:

```rust
let now = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_secs())
    .unwrap_or(0);
```

The rot-budget json says: `unix_epoch_inline` ceiling 69, with note
"T2-9 time-helper ratchet; canonical: `northhing_core_types::time`; only
down". The canonical helper `northhing_core_types::time::now_unix_ms()`
exists. Pre-existing rot was 70/69; this PR adds +1 → 71/69.

Per the review brief: "Pre-existing rot violations... are NOT from this
PR" — but adding new occurrences when already over the ceiling is a
continuation. The fix is one-liner: replace the inline expression with
`northhing_core_types::time::now_unix_ms() / 1000`. The brief's
constraint #9 already requires "遇编译错误先加载对应 rust skill, 禁止无
脑 clone/unwrap". This is a similar reflex: reach for the canonical
helper.

### M-4. UI passes hardcoded `Some(20)` search limit

`pages_memory.rs:156` calls `api::search_facts(&query, None, Some(20))`.
The facade default is `unwrap_or(20)` already (memory.rs:104), so
passing `Some(20)` is redundant. Not a bug — just minor duplication. Per
the brief's "facade 层: 纯 passthrough", the default belongs to the
facade; the UI could simply pass `None` and let the facade pick 20.

### M-5. Duplicated `FactItem::from(FactDto)` mapping

`pages_memory.rs` repeats the `d.id, d.text, d.scope, ...` mapping in
three places (lines 117-128, 159-168, 194-204). A trivial `impl
From<FactDto> for FactItem` would cut 30 lines and reduce drift risk.
Ponytail-lazy: skip unless a fourth call site appears.

### M-6. Stale-fetch race on rapid Enter / search

When the user types and hits Enter multiple times in quick succession,
multiple `spawn` calls fire concurrently and the last-to-resolve wins
when writing `facts.set(...)`. Dioxus panels commonly tolerate this. Not
flagging as Important because the worst case is "old results flash
briefly" — and there's no debounce mechanism that wouldn't introduce a
separate UX surprise. Document only if you're being thorough.

---

## Spec compliance — point by point

| Spec | Status | Notes |
|---|---|---|
| `FactDto` (snake_case serde, fields align Fact, enums flattened) | ✓ | All 8 fields match Fact struct (schema_version correctly omitted per brief + impl comment). |
| `KernelMemoryApi::list_facts(Option<&str>) -> Result<Vec<FactDto>, KernelError>` | ✓ | |
| `KernelMemoryApi::search_facts(&str, Option<&str>, Option<u32>)` | ✓ | limit default 20 in facade. |
| Facade passthrough via auto_memory path | ✓ | `MemoryDb::open(&default_memory_db_path())` — identical pattern to `auto_memory.rs:302`. No global handle. |
| Error mapping style aligns with list_episodes | ✓ | `KernelError::Runtime(format!("...: {}", e))` — same as list_episodes. |
| Desktop api wrappers ≤40 lines growth | ✓ | 17 lines added. |
| UI read-only list + search + JSONL export | ✓ | See I-1 for export error handling. |
| Export to `<config_dir>/northhing/exports/memory-<unix_ts>.jsonl` | ✓ | Line 225-230, ts is unix_secs. |
| Empty/error Chinese states | ✓ | "暂无记忆事实", "未找到匹配的记忆", "加载失败: ...", "搜索失败: ...". |
| Mount: avoid net add to app.rs | ✓ | Plugin registered in `registry.rs:138-145`, trigger button added in `windows.rs:541-553` (next to existing settings button — same pattern). mod.rs gets `mod pages_memory;`. |
| CSS reuse | ✓ | Uses `css::truth_css()` + `css::OVERLAY_CSS`. New CSS classes (`mem-toolbar`, `mem-search`, `mem-row`, `mem-empty`, `mem-loading`, `mem-error`, `mem-export-path`, `mem-scope`, `mem-conf`, `mem-type`, `mem-time`, `mem-btn-clear`, `mem-btn-export`) are appended to `OVERLAY_CSS` somewhere (out-of-diff; need to verify exists or is added). |
| Tests ≥1 per facade method | ✓ | 2 tests added. (See M-1 for DTO gap.) |
| No ceiling raises in rot-budget | ✗ | Adds +1 to `unix_epoch_inline` (71/69), +2 to `let_underscore`. Pre-existing already over ceilings per review brief. (See M-3.) |
| New files <800 lines | ✓ | pages_memory.rs = 347 lines. |
| contracts behavior-light (DTO + trait methods only) | ✓ | No impl details leaked upward. |
| logs English no emoji | ✓ | N/A — no logs added. Error strings are user-facing Chinese (intentional, brief §i18n frozen). |
| SDD ban: no `.superpowers/` in commit | ✓ | Verified via git show --stat — only the 9 in-scope files. |
| Single commit | ✓ | git log shows one commit c80227b. |
| TH-3 read-only: zero edit/delete/forget entry | ✓ | Only `list_facts`/`search_facts`/`std::fs::write` to disk. No `delete_fact`/`update_fact` called. |

---

## Cannot verify from diff

- **UI rendering on screen**: cannot run the desktop app here. CSS classes
  referenced by the new HTML must exist in `css::OVERLAY_CSS`; brief
  says "css.rs 余量 0 零触碰". Need spot-check that the new classes
  (`mem-toolbar`, `mem-row`, `mem-empty`, `mem-loading`, `mem-error`,
  `mem-export-path`, `mem-scope`, `mem-conf`, `mem-type`, `mem-time`,
  `mem-btn-clear`, `mem-btn-export`, `mem-search`, `station-head`)
  either already exist in `css.rs` (presumably `OVERLAY_CSS` constant)
  or were added in the PR. Diff did NOT show changes to `css.rs`, so
  they must already exist. If they don't, this is a runtime crash, not
  a build error — **please confirm**.

- **`cargo check --workspace`** and **`cargo test -p northhing --lib`** /
  **`test -p northhing-core --features product-full memory`** outputs:
  not in this review per the review-brief scope (you have these in the
  implementer's report).

- **`node scripts/verify-rot-budget.mjs`**: would need to be run by the
  implementer / CI. From the diff alone, the new file adds known rot;
  pre-existing is already over.

- **`w9-2-shot-1.png`**: not provided here.

---

## Suggested fixes (small, in-scope)

1. **I-1**: in `do_export`, match the `Result` from `fs::write` and
   route failure into `error_msg.set(format!("导出失败: {}", e))`,
   leaving `export_path` unset. Wrap with `let content = ...;
   match std::fs::write(&path, content) { Ok(_) => export_path.set(...), Err(e) => error_msg.set(...), }`. 5-line fix.

2. **I-2**: replace lines 69-88 with the canonical pattern (matches
   `pages_archive.rs:110`):

   ```rust
   let mut theme_dark = use_page_shell(&props);
   let class = if theme_dark() { "dark" } else { "light" };
   ```

   Then delete the local `use_signal` + `use_future` block. Net diff:
   -20 lines, no functional change. Even if theme_dark is unused
   elsewhere in the page, capturing it from the shell signals
   "this page is theme-aware" and prevents a future contributor from
   re-regressing.

3. **M-1**: add `test_list_facts_dto_fields_round_trip` that inserts a
   known fact and asserts the DTO field strings match. ~30 lines.

4. **M-3**: replace inline `duration_since(UNIX_EPOCH)` with
   `northhing_core_types::time::now_unix_ms() / 1000`. (Requires the
   `northhing_core_types` dep in `apps/desktop`'s `Cargo.toml` —
   verify; if not present, this is a separate small dep-add. If dep
   add is out of scope, leave the inline call and document it as
   intentional rot for W9-2 with a `// ponytail:` comment naming the
   ceiling and upgrade path, then file a follow-up.)

---

## Summary

- **Spec**: faithful at all layers.
- **Philosophy (TH-3)**: honored.
- **Bugs**: one Important (silent export failure → misleading UI) and
  one Important (theme future duplication). Both fixable in <10 lines
  total.
- **Tests**: smoke-only; DTO mapping untested.
- **Rot**: pre-existing overage continues; one inline UNIX_EPOCH added
  despite canonical helper.
- **Net recommendation**: NEEDS FIXES — apply I-1 and I-2, then merge.
  Optional: M-1 (DTO test) and M-3 (canonical time helper) for
  hygiene.

---

## Re-review of fix commit `d02502e` (2026-08-29)

Diff: `git diff c80227b..d02502e` — only `pages_memory.rs`, +10/-23.

### I-1 (silent export failure) — FIXED ✓

```rust
if let Err(e) = std::fs::create_dir_all(&dir) {
    error_msg.set(format!("创建导出目录失败: {}", e));
    return;
}
...
if let Err(e) = std::fs::write(&path, content) {
    error_msg.set(format!("导出失败: {}", e));
} else {
    export_path.set(path.to_string_lossy().to_string());
}
```

- `create_dir_all` failure → `error_msg` + early `return` (skips the
  write) ✓
- `fs::write` failure → `error_msg`, `export_path` left unset ✓
- `export_path` only set on success ✓
- Chinese error strings consistent with the rest of the page (i18n
  frozen) ✓
- No logging-discipline issue: these are user-facing UI strings, not
  `tracing` log output ✓

### I-2 (duplicate theme tracking) — FIXED ✓

```rust
pub fn memory_app_root(props: ModuleAppProps) -> Element {
    let theme_dark = use_page_shell(&props);
    let class = if theme_dark() { "dark" } else { "light" };
    ...
```

- 19-line reimplementation removed; now uses the `Signal<bool>` the
  shell already returns — matches the canonical pattern in
  `pages_archive.rs:110` / `pages_space.rs:147` ✓
- `use_page_shell` is the first hook in the function (required by
  page_shell.rs contract "call this exactly once at the top ... before
  any other hooks") ✓
- Single theme future active; no redundant signal ✓

### Regressions introduced by the fix — none

- Hook order preserved (shell call is first statement).
- `props.theme_rx` no longer directly referenced — correct, shell reads
  it internally.
- Export flow behavior otherwise unchanged (empty-list early return,
  timestamp, JSONL shape all intact).

### Pre-existing nit (not from this fix)

- `use std::rc::Rc;` (line 9) is unused — no `Rc::` usage anywhere in
  `pages_memory.rs`. It was already unused in the original commit
  `c80227b` (the removed theme block never used `Rc` either), so this
  fix neither introduced nor removed it. Desktop crate has no
  `deny(warnings)`, so it is a warning, not a compile failure. Flag for
  terminal triage / opportunistic cleanup.

### Remaining Minor items (unchanged, non-blocking)

M-1 (DTO field mapping untested), M-2 (workspace `None` hardcode),
M-3 (inline UNIX_EPOCH instead of `northhing_core_types::time`),
M-4 (`Some(20)` redundant), M-5 (triplicated FactItem mapping),
M-6 (stale-fetch race). These were explicitly non-blocking in the first
review and remain so.

### Cannot verify from diff (unchanged)

- UI rendering on screen; existence of the new CSS classes in
  `css::OVERLAY_CSS` (css.rs not in either commit).
- `cargo check --workspace` / desktop tests / rot-budget script output
  (in implementer's report / CI).

### Re-review verdict

**CAN MERGE.** Both Important findings are resolved correctly and
minimally with no regressions. The unused `Rc` import and the six Minor
items are routed to terminal triage.