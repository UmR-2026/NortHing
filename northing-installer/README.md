# northhing Installer

A fully custom, branded installer for northhing â?built with **Tauri 2 + React** for maximum UI flexibility.

## Why a Custom Installer?

Instead of relying on the generic NSIS wizard UI from Tauri's built-in bundler, this project provides:

- **100% custom UI** â?React-based, with smooth animations, dark theme, and brand consistency
- **Modern experience** â?Similar to Discord, Figma, and VS Code installers
- **Full control** â?Custom installation logic, right-click context menu, PATH integration
- **Cross-platform potential** â?Same codebase can target Windows, macOS, and Linux

## Common tasks

### Install dependencies

```bash
pnpm install
```

Production installer builds call workspace desktop build scripts, so root dependencies are required.

### Run in dev mode

```bash
pnpm run tauri:dev
```

### Build the full installer

```bash
pnpm run installer:build
```

Use this as the release entrypoint. `pnpm run tauri:build` does not prepare validated payload assets for production.

### Build installer only

```bash
pnpm run installer:build:only
```

`installer:build:only` requires an existing valid desktop executable in the expected target output path.

## Architecture

```
northing-installer/
âââ src-tauri/                 # Tauri / Rust backend
â?  âââ src/
â?  â?  âââ main.rs            # Entry point
â?  â?  âââ lib.rs             # Tauri app setup
â?  â?  âââ installer/
â?  â?      âââ commands.rs    # Tauri IPC commands
â?  â?      âââ extract.rs     # Archive extraction
â?  â?      âââ registry.rs    # Windows registry (uninstall, context menu, PATH)
â?  â?      âââ shortcut.rs    # Desktop & Start Menu shortcuts
â?  â?      âââ types.rs       # Shared types
â?  âââ capabilities/
â?  âââ icons/
â?  âââ Cargo.toml
â?  âââ tauri.conf.json
âââ src/                       # React frontend
â?  âââ pages/
â?  â?  âââ LanguageSelect.tsx # First screen language picker
â?  â?  âââ Options.tsx        # Path picker + install options
â?  â?  âââ Progress.tsx       # Install progress + confirm
â?  â?  âââ ModelSetup.tsx     # Optional model provider setup
â?  â?  âââ ThemeSetup.tsx     # Theme preview + finish
â?  âââ components/
â?  â?  âââ WindowControls.tsx # Custom titlebar
â?  â?  âââ Checkbox.tsx       # Styled checkbox
â?  â?  âââ ProgressBar.tsx    # Animated progress bar
â?  âââ hooks/
â?  â?  âââ useInstaller.ts    # Core installer state machine
â?  âââ styles/
â?  â?  âââ global.css         # Base styles
â?  â?  âââ variables.css      # Design tokens
â?  â?  âââ animations.css     # Keyframe animations
â?  âââ types/
â?  â?  âââ installer.ts       # TypeScript types
â?  âââ App.tsx
â?  âââ main.tsx
âââ scripts/
â?  âââ build-installer.cjs    # End-to-end build script
âââ index.html
âââ package.json
âââ vite.config.ts
âââ tsconfig.json
```

## Installation flow

```
Language Select â?Options â?Progress â?Model Setup â?Theme Setup
       â?            â?         â?           â?             â?
   choose UI      path +     run real    optional AI     save theme,
    language      options    install      model config    launch/close
```

## Development

### Prerequisites

- Node.js 18+
- Rust (latest stable)
- pnpm

### Setup

```bash
pnpm install
```

### Repository Hygiene

Keep generated artifacts out of commits. This project ignores:

- `node_modules/`
- `dist/`
- `src-tauri/target/`
- `src-tauri/payload/`

### Dev Mode

Run the installer in development mode with hot reload:

```bash
pnpm run tauri:dev
```

### Uninstall Mode (Dev + Runtime)

Key behavior:

- Install phase creates `uninstall.exe` in the install directory.
- Windows uninstall registry entry points to `"<installPath>\\uninstall.exe" --uninstall "<installPath>"`.
- Launching with `--uninstall` opens the dedicated uninstall UI flow.
- Launching `uninstall.exe` directly also enters uninstall mode automatically.

Local debug command:

```bash
npx tauri dev -- -- --uninstall "D:\\tmp\\example-install-dir"
```

Core implementation:

- Launch arg parsing + uninstall execution: [commands.rs](src-tauri/src/installer/commands.rs)
- Uninstall registry command: [registry.rs](src-tauri/src/installer/registry.rs)
- Uninstall UI page: [Uninstall.tsx](src/pages/Uninstall.tsx)
- Frontend mode switching and state: [useInstaller.ts](src/hooks/useInstaller.ts)

## Build

### Full release build

```bash
pnpm run installer:build
```

Release artifacts embed payload files into the installer binary, so runtime installation does not depend on an external `payload` folder.

### Full fast build

```bash
pnpm run installer:build:fast
```

### Installer-only build

```bash
pnpm run installer:build:only
```

If payload validation fails, the build exits with an error.

### Installer-only fast build

```bash
pnpm run installer:build:only:fast
```

### Output

Default release output:

```text
src-tauri/target/release/northhing-installer.exe
```

Fast build output:

```text
src-tauri/target/release-fast/northhing-installer.exe
```

## Customization guide

### Changing the UI Theme

Edit [variables.css](src/styles/variables.css). Colors, spacing, and animations are controlled by CSS custom properties.

### Adding Install Steps

1. Add a new step key to `InstallStep` in [installer.ts](src/types/installer.ts)
2. Create a new page component in [src/pages](src/pages)
3. Add the step to the `STEPS` array in [useInstaller.ts](src/hooks/useInstaller.ts)
4. Add the page render case in [App.tsx](src/App.tsx)

### Modifying Install Logic

- **File extraction** â?[extract.rs](src-tauri/src/installer/extract.rs)
- **Registry operations** â?[registry.rs](src-tauri/src/installer/registry.rs)
- **Shortcuts** â?[shortcut.rs](src-tauri/src/installer/shortcut.rs)
- **Tauri commands** â?[commands.rs](src-tauri/src/installer/commands.rs)

### Adding Installer Payload

Place the built northhing application files in `src-tauri/payload/` before building the installer. The build script handles this automatically.
During `cargo build`, the payload directory is packed into an embedded zip inside `northhing-installer.exe`.

## Integration with CI/CD

Add to your GitHub Actions workflow:

```yaml
- name: Build Installer
  run: |
    cd northing-installer
    pnpm install
    pnpm run installer:build:only

- name: Upload Installer
  uses: actions/upload-artifact@v4
  with:
    name: northing-installer-exe
    path: northing-installer/src-tauri/target/release/northhing-installer.exe
```
