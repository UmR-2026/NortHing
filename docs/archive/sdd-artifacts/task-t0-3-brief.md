# Task T0-3 Brief: normalize user-visible brand marks to "NortHing"

## Source
- backend-roadmap.md T0-3; decision-register T-02 (user decision 2026-08-17): canonical product name = **NortHing（诺森）**, case-sensitive.

## Goal
Fix USER-VISIBLE brand strings that currently show "northhing" / "northing" / "NorthHing" in wrong case or broken form. Code identifiers stay lowercase and untouched.

## In scope
1. **Desktop Slint UI** (`src/apps/desktop/src/ui/**/*.slint`): display strings only. Known instances (verify, don't assume):
   - `main.slint`: `title: "northhing"` → `"NortHing"`; `app-title: "northhing v0.1.0"` → `"NortHing v0.1.0"`
   - `components/WindowChrome.slint`: `text: "northing"` → `"NortHing"`
   - Any other .slint where the string is rendered to the user. `strings.slint` already uses "NortHing" correctly — use it as the reference. Comments may be fixed when on a line you touch anyway; don't churn comment-only edits.
2. **Installer UI** (`northing-installer/src/**`): i18n locale string **VALUES** in `i18n/locales/*.json` (all locales) and any hardcoded display text in `.tsx`. Known instances: `"Launch northhing after setup"`, `"a northhing installation"`, `"Opennorthhing"` / `"Opennorthhing Model Platform"` (also missing the space — should be "Open NortHing").
   - ⚠️ NEVER rename i18n **keys** (e.g. `opennorthhing`, `northhing-dark` are identifiers — keys stay byte-identical, only values change).
3. **README.md** (repo root): title `# northhing` → `# NortHing`, and prose mentions where the word refers to the product. Do NOT touch paths (`northing-installer/README.md` link), crate names, commands, URLs.

## Explicitly out of scope
- Code identifiers, crate/package/dir names, paths, URLs, shell commands, i18n keys, Cargo.toml package names.
- `docs/archive/**`, `.superpowers/**`, `src/mobile-web/**` (frozen), `CHANGELOG.md` (historical record).
- Doc-body prose sweep in `docs/**` — deferred to rolling cleanup (housekeeping rule 1). Only README is in scope.
- Agent display name「知序」→「北」 is TH-1, NOT this task. Don't touch it.

## Editing discipline
- UTF-8 safe edits only: use the edit tool; NEVER use PowerShell Set-Content/Out-File on files containing CJK (GBK double-encoding hazard).
- Preserve each file's existing line endings.

## Verification (run and paste raw output)
1. `cargo check -p northhing` (Slint strings compile into the desktop crate)
2. `pnpm --dir northing-installer run type-check`
3. If an i18n contract/audit script covers the installer locales, run the smallest one and report; if not applicable, say so.

## Report
Write to `.superpowers/sdd/task-t0-3-report.md`. Status line first: DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED. Include a per-file change list and raw verification output.
