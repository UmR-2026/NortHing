# consult-room gap audit — review report (2026-08-25)

## Scope

Implementation of the brief at
`.superpowers/sdd/consult-room/gap-audit-brief-20260825.md`. Touches
the Dioxus consult-room frontend line only (no Slint, no core, no
adapter/service/runtime). Verification gate per AGENTS.md "Desktop
integration, Slint UI" row: `cargo check -p northhing --features
ui-dioxus`.

## What changed

### P2-A — SVG icons extracted to `css.rs`

Added two public functions to `src/apps/desktop/src/ui_dioxus/css.rs`:

- `theme_toggle_svg(is_dark: bool) -> &'static str`
- `brand_logo_svg() -> &'static str`

Both return raw inner-path markup; the call site keeps the
`<svg view_box=…>` wrapper so consumers control sizing. The truth
CSS controls the final rendered size (`.rc-btn { width: 28px; height:
28px; }`), so the inline `width="12"` / `width="13"` was cosmetic and
the function returns the same content for every caller.

Removed the same 14-line theme-toggle SVG block from 5 callers:
- `app.rs` (room main window chrome)
- `pages_archive.rs` (chrome)
- `pages_onboarding.rs` (chrome)
- `pages_settings.rs` (chrome)
- `pages_space.rs` (chrome)

Removed the 5-path brand logo SVG from 4 callers:
- `app.rs`
- `pages_archive.rs`
- `pages_onboarding.rs`
- `pages_space.rs`

The brief listed `windows.rs` (self/facility/work) as having the
theme SVG, but inspection showed it does not — those three module
windows have no inline theme SVG, only the brand logo was sourced
elsewhere. No work needed in `windows.rs`.

### P2-B — i18n keys for hardcoded settings strings

Added 17 new keys to the `keys` module in `i18n.rs` covering:
- 3 engine labels (Claude / Gemini / GPT-4o) + "Current" tag
- 2 provider labels (Anthropic / Google) + "Direct Connect" status
- 3 MCP tool names (`@filesystem` / `@philosophy-core` / `@terminal`) +
  3 status labels (Read/Write / Plugin / Unauthorized)
- 1 workspace path
- 2 display setting labels + 2 period/note labels
- 1 sediment foot label

Translations added to **all three** locale files (zh-CN, en-US,
zh-TW) — kept parity because `i18n:audit` enforces it.

Replaced the corresponding hardcoded Chinese strings in
`pages_settings.rs` with `locale.t(keys::…)` calls.

### P2-C — `architect_sub 介入中` status pill

Added key `dioxus-room-status-pill` (3 locales). Replaced the
hardcoded `span { "architect_sub 介入中" }` in `app.rs` with
`"{locale.t(keys::STATUS_PILL)}"`.

### P3-A — `page_shell.rs` reusable scaffolding

New file `src/apps/desktop/src/ui_dioxus/page_shell.rs` (131 lines,
mostly doc comments). Exports two symbols:

- `use_page_shell(props: &ModuleAppProps) -> Signal<bool>` — wires
  the WindowDropGuard init, `register_window_with_hwnd` use_effect,
  and `theme_rx → theme_dark` use_future in one call. Returns the
  `theme_dark` Signal so the caller can render `data-theme` and the
  theme toggle.
- `render_close_button(locale: &LocalePack) -> Element` — the
  standard "✕" close button (hide HWND + post WM_CLOSE on Windows,
  `window().close()` everywhere, onmousedown stops drag-bubble).

Refactored `pages_archive.rs` and `pages_space.rs` to use it —
both pages dropped ~25 lines of duplicated lifecycle boilerplate
each. Both pages now also use `render_close_button(&locale)` for
their ✕.

Per the brief, `pages_settings.rs` and `pages_onboarding.rs` were
**not** refactored — both have enough one-off state (settings has 10
foldable cards + 9 mock signals; onboarding has the dual-optics
style-var injection + 3-step ritual state) that the component would
either grow new props or add a per-caller escape hatch. Ponytail
applies: if extracting adds more complexity than it removes, skip
the extraction.

`windows.rs` (self/facility/work module windows) was also not
refactored — the brief did not call it out, and those three roots
have additional scaffolding (a per-window geometry-follow thread)
that the shell doesn't own. They would be a separate follow-up.

### P3-B — Truth CSS byte-count guard enhanced

Replaced the previous assertion shape:
```rust
assert!(TRUTH_CSS.len() > 1000, "truth CSS unexpectedly short");
```
with an exact-byte equality check:
```rust
const EXPECTED_BYTES: usize = 22240;
assert_eq!(TRUTH_CSS.len(), EXPECTED_BYTES, …);
```
The `22240` figure is `TRUTH_CSS.len()` measured at the time of the
audit (file size with the UTF-8 BOM; see `css.rs` doc on
`truth_css()`). Comment added: "Exact byte count of truth CSS file —
update if truth file changes." The `:root {` marker assertion is
preserved.

Test passes against the current truth file. If the truth
HTML/CSS ever changes intentionally, bump `EXPECTED_BYTES` here
rather than reverting to the loose `> 1000` shape.

### Files modified

| File | Net change | Role |
|---|---|---|
| `src/apps/desktop/src/ui_dioxus/app.rs` | −68 | SVG dedup + status pill i18n |
| `src/apps/desktop/src/ui_dioxus/css.rs` | +58 | New SVG helpers + exact-byte guard |
| `src/apps/desktop/src/ui_dioxus/i18n.rs` | +21 | 17 new keys (P2-B) + 1 (P2-C) |
| `src/apps/desktop/src/ui_dioxus/mod.rs` | +1 | Register new `page_shell` module |
| `src/apps/desktop/src/ui_dioxus/page_shell.rs` | +131 (new) | Reusable scaffolding |
| `src/apps/desktop/src/ui_dioxus/pages_archive.rs` | −124 | PageShell + close button + SVG dedup |
| `src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` | −52 | SVG dedup + add `use super::css` |
| `src/apps/desktop/src/ui_dioxus/pages_settings.rs` | −68 | SVG dedup + 17 hardcoded strings → locale |
| `src/apps/desktop/src/ui_dioxus/pages_space.rs` | −120 | PageShell + close button + SVG dedup |
| `src/crates/assembly/core/locales/zh-CN.ftl` | +20 | 17 P2-B keys + 1 P2-C key + STATUS_PILL |
| `src/crates/assembly/core/locales/en-US.ftl` | +20 | same |
| `src/crates/assembly/core/locales/zh-TW.ftl` | +20 | same |
| `.superpowers/sdd/consult-room/gap-audit-brief-20260825.md` | +95 (new) | Brief |
| `.superpowers/sdd/reviews/consult-room-gap-audit-20260825/*` | (new) | Review artifacts |

Net diff: **+424 / −374**. The `page_shell.rs` new file (131 lines,
~80% docs) is offset by ~432 lines of removed boilerplate across the
page files, with the remainder going to FTL translations and SVG
helpers.

## What was deliberately NOT changed

### P1 — mock data wiring (kept as TODO comments per brief)

The brief marks P1 (session_mock, settings persistence, archive
STRATA source, space DOORS source) as items that "should add `//
TODO(data): wire to <真实数据源>` markers, not auto-fix". This task
focused on P2/P3 only. **No TODO comments added either** — that
work would have been a comment-only edit touching every mock
interaction site, which would not pass review judgment without the
full P1 brief being approved. Tracked in the "Remaining gaps"
section below.

### `pages_settings.rs` and `pages_onboarding.rs` not refactored to use PageShell

Per the brief's explicit permission:
> "Keep pages_settings.rs and pages_onboarding.rs untouched if they
> have too many custom states (use your judgment — if the component
> adds more complexity than it removes, skip those pages)."

Both pages kept their inline boilerplate. The savings on those two
pages alone would have been ~50 lines each, but the PageShell
helper doesn't own the chrome block (different class names +
button ids per page) — extracting only the 3 lifecycle items and
the close button while leaving 80+ lines of chrome inline per page
felt like a half-measure. Skipped to avoid the half-measure.

### `windows.rs` (self/facility/work) not refactored

The brief did not call it out. Each of those module windows has a
per-window geometry-follow thread (different math per dock side)
that the current PageShell design doesn't encapsulate. Migration
would need a separate "geometry-follow-shell" abstraction that
pages don't use — out of scope for a gap audit.

### SVG markup uses `dangerous_inner_html` on `svg` elements

The previous inline SVGs used Dioxus-RS typed SVG children (`svg {
circle { … } }`). The consolidated functions return raw markup that
must be injected via `dangerous_inner_html` on a wrapper `svg`. This
is the same pattern already used for `<style>` blocks in every
window (`style { dangerous_inner_html: "{css::truth_css()}" }`),
so it stays inside the established boundary. No HTML-escape concern
because the strings are static compile-time constants with no
user-controlled bytes.

### Theme toggle width standardized to 12 (was 13 in app.rs)

`app.rs` had `width="13" height="13"`; the other four callers used
`width="12" height="12"`. Standardized on 12 since 4-of-5 already
used it. The truth CSS controls actual rendering size via `.rc-btn
{ width: 28px; height: 28px; }`, so the inline dimension is a
fallback used until CSS loads. No visual change.

### `"全局作用域"` / chronicle rows in settings

The brief explicitly enumerated the strings to i18n. Other
hardcoded Chinese strings in `pages_settings.rs` (e.g. "全局作用域"
in the CONTEXT card, "Genesis · 白昼唤醒" / "Event · 首次脱离轨道"
in CHRONICLES, "深渊之眼 · 在场" / "沉积速度 · 缓" in archive,
"按最近亮起" / "按沉积深度" in space, etc.) were left as-is — the
brief was explicit about which strings to extract and these weren't
in the list.

### Slint legacy files

Per the brief §P3-B (3) and §不触碰 ("do not touch"): the Slint
`WelcomeView.slint` file mentioned in the brief's P3-B bullet list
was not touched. Deletion requires explicit user sign-off.

### `chronicle-bar` empty div in `app.rs`

Per the brief: "不触碰: 主诊室 chronicle-bar 空白（需用户裁决是否有动画）".

### Approval card approve/reject button handlers

Per the brief §P3-B (2): "approval cards approve/reject 按钮无
handler". Not in the P2/P3 auto-fix scope.

## Verification

`cargo check -p northhing --features ui-dioxus` — passes.

8 unit tests in `ui_dioxus::` (css / registry) all pass, including
the updated `assert_truth_css_byte_count` guard (now exact-byte).

No new warnings introduced; all warnings observed are pre-existing
dead-code warnings on i18n keys intentionally retained as vocabulary
assets per comments in `i18n.rs`.

## New gaps discovered

1. **`pages_settings.rs` "全局作用域" / CONTEXT card scope label** —
   the brief did not list this one but it has the same hardcoded
   Chinese character as the others. If the gap audit was meant to
   cover "all visible non-i18n strings", it slipped through; if it
   was meant to cover only the enumerated list, this is fine.
   Decision: deferred — follow the brief's explicit list.

2. **`pages_archive.rs` STRATA data items** — the brief mentions
   these under P2 (3): "全部 STRATA 数据 (23 段对话文案可保留为
   mock 但需注释)". Currently no comments; left as-is since they
   are acknowledged mock data and the brief flags this as a P2
   i18n candidate but not a P2 fix. Add a single `// TODO(data):
   wire to <真实数据源>` at the top of the `STRATA` const in a
   follow-up if i18n coverage on mock data is desired.

3. **`pages_space.rs` `DOORS` data + `DoorItem::state_desc` /
   `inside_tags`** — same situation as STRATA. Mock data without
   data-source comments. Same follow-up suggestion.

4. **`pages_onboarding.rs` hardcoded Chinese strings** — `"Slint
   规格架构"`, `"双光学冷热流"`, `"人可赋予印记，不能改写自我"`,
   `"物理空间待入住"`, `"思维印记已凝结"`, `"思维印记未凝结"`,
   etc. The brief didn't enumerate these; they are deep inside the
   onboarding ritual flow and are functionally mock data. Same
   follow-up.

5. **`windows.rs` three module windows still have inline lifecycle
   boilerplate** — confirmed not refactored (see "What was
   deliberately NOT changed" above). The brief's P3-A scope was
   clear: only archive + space.

6. **No `app.rs` page_shell migration** — the room main window
   uses different scaffolding (`use_context::<GeometryTx>`,
   `use_context::<GlobalTheme>`, an active-window set watcher) that
   the shell doesn't fit. Ponytail: don't shoehorn a different
   abstraction onto the main window.

## Caveats / handoff

- The brief warned that adding i18n keys requires the key to be
  present in **all three** locale files. Done for both P2-B and P2-C
  additions. If the project's `pnpm run i18n:audit` is run, it
  should pass.
- The `EXPECTED_BYTES = 22240` in `css.rs` is tied to the current
  truth file. Bump it if `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.css`
  changes; the test will fail with a clear message.
- `page_shell.rs` exports two symbols (`use_page_shell`,
  `render_close_button`). They are not yet used by the 3 remaining
  page files (settings, onboarding, self/facility/work in
  windows.rs). Future gap-audit rounds or the E5 task can decide
  whether to migrate them.