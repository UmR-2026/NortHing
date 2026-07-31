---
name: northhing-slint-desktop
description: Use for designing, implementing, debugging, or reviewing Northhing's native Slint desktop UI, Rust callbacks, window behavior, and visual acceptance
---

# Northhing Slint Desktop

Use this skill only for the native Slint application in `src/apps/desktop`. Northhing's shipped desktop is package `northhing`; it is not Tauri, Electron, or a web surface.

This is project-specific guidance written from Northhing's current architecture and Slint 1.17 documentation. Repository-local `AGENTS.md` files and the active task specification remain authoritative.

## Start Here

Before editing:

1. Read root `AGENTS.md` as the current repository guide, then read the nearest directory-level `AGENTS.md` or `AGENTS-CN.md` for every path in scope. Treat `AGENTS-CN.md` as a translation aid; if it conflicts with `AGENTS.md`, `package.json`, or `Cargo.lock`, use the latter sources.
2. For architecture-sensitive work, also read `README.md` and `CONTRIBUTING.md`.
3. Read the active plan or handoff. For FR-T5, read `docs/superpowers/plans/2026-07-29-fr-t5-settings-drawers.md`.
4. Confirm the resolved Slint version in `Cargo.lock`; do not infer behavior only from `Cargo.toml` ranges.
5. If CodeGraph is configured and its index is current, use it for symbol, caller, and impact analysis. Otherwise use LSP, Grep, and direct source inspection without blocking the task.
6. Inspect the rendered state and existing callback path before proposing changes.

If documentation conflicts, prefer the nearest current directory guide, then root `AGENTS.md`, `package.json`, and `Cargo.lock` over translated, archived, or stale planning documents.

## Project Boundaries

- UI markup and components: `src/apps/desktop/src/ui/`.
- Rust desktop state and callbacks: `src/apps/desktop/src/app_state/`.
- Follow the existing kernel/API and adapter boundaries documented by the nearest guide. Do not add a new direct path into core for convenience.
- Core `GlobalConfig` is the sole runtime configuration source. Do not add another desktop-readable config file.
- Keep business state and persistence in Rust. Slint owns presentation state and forwards user actions through callbacks.
- Do not introduce web, Tauri, frozen-surface, or installer abstractions into a Slint-only task.
- Avoid unrelated refactors. Above 1000 lines, split a production Rust file or add the repository-approved `// allow-god-file` justification. For files explicitly called out by the active plan, such as `callbacks_lifecycle.rs`, follow the stricter plan-specific split rule.

## UI Thread Discipline

A Slint property update from a non-event-loop thread can be lost without a useful failure. Every background completion, stream event, timer bridge, or async callback that changes UI state must enter the Slint event loop.

- Reuse the existing helpers around `slint::invoke_from_event_loop` where possible.
- Capture `Weak` UI handles rather than extending the window lifetime with a strong handle.
- Keep blocking I/O, provider calls, file writes, and long computation off the UI thread.
- For cancellation, timeout, or concurrent state changes, add a focused automated test. Do not rely only on a screenshot.

## Callback Contract

For every callback added or touched:

1. Locate the declaration and every call site in `.slint` files.
2. Locate the Rust `on_<callback>` registration.
3. Trace success, error, cancellation, and persistence paths.
4. Verify loading/disabled state returns to a usable state on every terminal path.
5. Remove or disable controls that have no implemented handler; do not ship clickable dead ends.

Slint kebab-case callbacks map to Rust snake_case registration names. Models exposed as `[T]` map to `ModelRc<T>`; retain and mutate a `VecModel` when live row updates are required.

For the current FR-T5 milestone, treat these as explicit contract checks:

- Identity generation and save must have Rust handlers and persistence.
- Markdown export must gather the intended session messages, open the existing native save dialog pattern, and report write failures.
- Session settings must have a defined destination or be visibly disabled.
- The `/` skill picker must connect to the existing skill registry instead of maintaining a second list.

## Slint Layout Rules

- Apply `padding` and `spacing` to layout elements, not arbitrary rectangles.
- Use `preferred-height`/`preferred-width` deliberately inside `Flickable` and nested layouts.
- Do not set `x` or `y` on an item managed by a layout unless intentionally overriding layout placement.
- Preserve units: lengths need `px`/`rem`, durations need `ms`, and angles need `deg`.
- Use Slint string interpolation syntax, not JavaScript or template-literal syntax.
- Bind only properties that should remain reactive. An unnecessary explicit binding can override a useful component or layout default.
- Put animations on the property that changes and verify the transition in motion; a single screenshot cannot prove animation quality.

When a layout behaves unexpectedly, reduce it to the smallest component and check preferred size, fill behavior, explicit geometry, and parent constraints before changing unrelated tokens.

## Input, Focus, and Overlays

- Use `FocusScope` and the appropriate capture/bubble path for keyboard shortcuts.
- Ensure text input keeps normal editing behavior; global shortcuts must not consume unrelated keys.
- A `/` picker must define opening, filtering, keyboard navigation, selection, Escape, and focus restoration.
- Keep popovers within the window bounds and provide a deterministic dismissal path.
- Use accessible menu primitives where the interaction is semantically a menu.
- Verify keyboard-only operation, not just pointer clicks.

## Theme and Visual Language

Northhing's redesign uses `RedesignTheme` and the generated redesign token system.

- Do not reintroduce `MaterialTheme` into migrated surfaces.
- Do not import generic `Palette` or standard-widget styling merely because generic Slint guidance recommends it.
- Use existing semantic color, typography, spacing, radius, air, halo, and representative-color tokens.
- Do not add raw hex colors in FR-T5 UI work. If a required token is missing, update the designated token source/generator instead of scattering literals.
- Verify light and dark modes independently.
- Reuse existing icon assets. Do not use unsupported Unicode glyphs as icons; tofu boxes are release defects.
- Preserve the product's calm, low-chrome visual language rather than default platform or Material styling.

For settings work, the HTML prototypes under `docs/design/2026-07-22-frontend-redesign/prototypes/` are the visual source of truth, but rendered Slint behavior and existing functionality must both be preserved.

## FR-T5 Execution Order

Follow the dependency order in the active plan:

1. Unify the settings shell and pages.
2. Prove real window expansion with a small Rust/Slint POC before replacing drawers.
3. Move Skills/MCP/theme responsibilities to settings and redefine the outer drawer.
4. Fix glyph and small visual regressions.
5. Close Identity, export, and session-settings callback gaps.

Do not implement full drawer expansion before the POC demonstrates:

- the main content does not shift;
- each side expands by the intended width;
- both drawers can coexist;
- minimum size rules remain valid;
- frameless window controls and resize behavior still work;
- animation does not introduce visible jitter.

If the native API cannot satisfy these conditions reliably, report the evidence and use the plan's explicit instant-resize fallback rather than hiding the problem with an in-window overlay.

## Documentation and MCP Safety

- Use CodeGraph for impact and callback-path analysis only when the project MCP is available and its index is current; otherwise continue with LSP, Grep, and direct source inspection.
- Use the existing Context7 Slint documentation source when available and query for the resolved 1.17.x behavior. Do not register another documentation service or guess element, property, or window APIs.
- Do not attach the upstream remote Slint docs MCP; Context7 already covers documentation without adding another remote trust boundary.
- Do not install `slint-lsp`, `slint-viewer`, npm/Python dev packages, or binaries as part of this skill.
- Do not execute floating `latest`, `curl | tar`, unpinned `cargo install`, or installs from a moving branch.
- The embedded Slint UI-control MCP is disabled by default. A future POC requires explicit task scope, loopback-only binding, a temporary process, non-sensitive test data, and teardown after verification.

## Native Visual Verification

Compilation is necessary but cannot establish visual correctness.

1. Build or run the actual native desktop application.
2. Capture the real `northhing` window.
3. Read the image and compare it with the design source of truth.
4. Trigger the target interaction.
5. Capture and inspect the resulting state.
6. Repeat for light/dark mode and relevant empty/loading/error states.

On the current Windows workspace, use the reviewed helpers when available:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ..\.opencode\tools\shot-window.ps1 -ProcessName northhing -OutFile <absolute-png-path>
powershell -NoProfile -ExecutionPolicy Bypass -File ..\.opencode\tools\click-window.ps1 -ProcessName northhing -X <window-x> -Y <window-y>
```

Coordinate injection is allowed only for the local test window. Do not use it against unrelated applications. Static screenshots cannot verify animation; use multiple frames or direct observation for transitions.

The acceptance matrix should cover only states affected by the change, selected from:

- light and dark themes;
- main, settings, archive, and onboarding routes;
- left/right/both drawers;
- empty, loading, success, error, and disabled states;
- pointer and keyboard interaction;
- narrow, default, and expanded window sizes.

## Verification Gate

Run the smallest checks that prove the change, with fresh output:

```text
pnpm run fmt:rs                 # only when Rust changed
pnpm run desktop:check
```

Use the MSVC command from the active FR-T5 plan when the default Windows toolchain is not the validated one. Add focused desktop tests whenever behavior, concurrency, cancellation, persistence, or callback state changes.

Before completion, require all applicable evidence:

- zero compile errors and no new warnings;
- focused tests pass;
- every touched Slint callback has a Rust handler or an intentional disabled state;
- no new raw hex values or Material theme dependencies in migrated UI;
- screenshots of each affected visual state have been inspected;
- failures and unverified states are reported explicitly.

Do not run frozen Web/i18n checks as a substitute for desktop verification. Run i18n commands only when the task actually changes the shared locale contract or resources.

## Review Checklist

- Scope matches the active task and no frozen surface changed.
- UI state has one clear owner.
- Background updates enter the event loop.
- Callback paths include failure and cancellation.
- Theme tokens replace literals.
- Focus and keyboard behavior are intentional.
- Window behavior is tested on the native app.
- The visual result was inspected, not inferred from source.
- Commands and evidence are included in the handoff.
