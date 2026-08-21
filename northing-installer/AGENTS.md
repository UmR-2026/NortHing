[中文](AGENTS-CN.md) | **English**

# AGENTS.md

## Scope

This file applies to `northing-installer`. Use the top-level `AGENTS.md` for repository-wide rules.

## What matters here

`northing-installer` is a separate Tauri + React app, not part of the main Cargo workspace.

Important areas called out by the module README:

- `src-tauri/src/installer/commands.rs`: Tauri IPC and uninstall execution
- `src-tauri/src/installer/registry.rs`: Windows registry integration
- `src-tauri/src/installer/shortcut.rs`: shortcut creation
- `src-tauri/src/installer/extract.rs`: archive extraction
- `src/hooks/useInstaller.ts`: frontend installer state flow
- `src/i18n/`: installer-only strings; locale metadata is generated from
 `src/shared/i18n/contract/locales.json`

Install flow:

```text
Language Select— Options— Progress— Model Setup— Theme Setup
```

## Commands

These are command references, not the default precheck list. Use Verification
below for PR scope.

```bash
pnpm --dir northing-installer run installer:dev
pnpm --dir northing-installer run tauri:dev
pnpm --dir northing-installer run type-check
pnpm --dir northing-installer run build # React build / CI reproduction
pnpm --dir northing-installer run installer:build # packaging only
```

## Verification

Use the smallest matching check:

```bash
pnpm run i18n:audit # resource-only i18n
pnpm run i18n:generate && pnpm run i18n:contract:test && pnpm run i18n:audit
pnpm --dir northing-installer run type-check # frontend i18n/runtime
cargo check --manifest-path northing-installer/src-tauri/Cargo.toml # Tauri/Rust changes
```

Run the full installer build only for packaging, payload, native bundling,
install/uninstall flow, registry, shortcut, or extraction changes:

```bash
pnpm --dir northing-installer run type-check && pnpm --dir northing-installer run installer:build
```

If you modify uninstall flow, also validate the uninstall mode entry points described in `northing-installer/README.md`.
