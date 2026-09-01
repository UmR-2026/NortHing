DONE

## Per-file Change List

1. **`src/apps/desktop/src/ui/main.slint`**:
   - `title`: `"northhing"` → `"NortHing"`
   - `app-title`: `"northhing v0.1.0"` → `"NortHing v0.1.0"`

2. **`src/apps/desktop/src/ui/components/WindowChrome.slint`**:
   - Watermark text: `"northing"` → `"NortHing"`

3. **`northing-installer/src/i18n/locales/en.json`**:
   - `errors.installPath.directoryMustBeEmptyOrNorthhing`: `"a northhing installation."` → `"a NortHing installation."`
   - `options.launchAfterInstall`: `"Launch northhing after setup"` → `"Launch NortHing after setup"`
   - `options.existingInstallTitle`: `"Existing northhing installation detected"` → `"Existing NortHing installation detected"`
   - `model.providers.opennorthhing.name`: `"Opennorthhing"` → `"Open NortHing"`
   - `model.providers.opennorthhing.description`: `"Opennorthhing Model Platform"` → `"Open NortHing Model Platform"`
   - `uninstall.title`: `"Uninstall northhing"` → `"Uninstall NortHing"`
   - `uninstall.subtitle`: `"removes northhing"` → `"removes NortHing"`
   - *(Note: All i18n keys kept byte-identical as required).*

4. **`northing-installer/src/i18n/locales/zh.json`**:
   - `errors.installPath.directoryMustBeEmptyOrNorthhing`: `"northhing 安装"` → `"NortHing 安装"`
   - `options.launchAfterInstall`: `"安装后启动 northhing"` → `"安装后启动 NortHing"`
   - `options.existingInstallTitle`: `"检测到本机已安装 northhing"` → `"检测到本机已安装 NortHing"`
   - `model.providers.opennorthhing.name`: `"Opennorthhing"` → `"Open NortHing"`
   - `model.providers.opennorthhing.description`: `"Opennorthhing 大模型平台"` → `"Open NortHing 大模型平台"`
   - `uninstall.title`: `"卸载 northhing"` → `"卸载 NortHing"`
   - `uninstall.subtitle`: `"将移除 northhing"` → `"将移除 NortHing"`

5. **`northing-installer/src/i18n/locales/zh-TW.json`**:
   - `errors.installPath.directoryMustBeEmptyOrNorthhing`: `"northhing 安裝"` → `"NortHing 安裝"`
   - `options.launchAfterInstall`: `"安裝後啟動 northhing"` → `"安裝後啟動 NortHing"`
   - `options.existingInstallTitle`: `"檢測到本機已安裝 northhing"` → `"檢測到本機已安裝 NortHing"`
   - `model.providers.opennorthhing.name`: `"Opennorthhing"` → `"Open NortHing"`
   - `model.providers.opennorthhing.description`: `"Opennorthhing 大模型平台"` → `"Open NortHing 大模型平台"`
   - `uninstall.title`: `"解除安裝 northhing"` → `"解除安裝 NortHing"`
   - `uninstall.subtitle`: `"將移除 northhing"` → `"將移除 NortHing"`

6. **`src/shared/i18n/resources/shared/en-US/terms.json`**, **`zh-CN/terms.json`**, **`zh-TW/terms.json`**:
   - `"product.name"`: `"northhing"` → `"NortHing"`
   - `"product.remote"`: `"northhing Remote"` → `"NortHing Remote"`
   - `"connectionMethods.northhingServer"`: `"northhing Server"` → `"NortHing Server"`

7. **`northing-installer/src/pages/LanguageSelect.tsx`**:
   - `alt="northhing"` → `alt="NortHing"`
   - Heading text: `northhing` → `NortHing`

8. **`northing-installer/src/pages/ThemeSetup.tsx`**:
   - Error string: `'Failed to launch northhing'` → `'Failed to launch NortHing'`

9. **`northing-installer/index.html`**:
   - `<title>Install northhing</title>` → `<title>Install NortHing</title>`

10. **`README.md`**:
    - Top heading: `# northhing` → `# NortHing`

11. **`northing-installer/src/i18n/generatedLocaleContract.ts`** & shared contract files:
    - Updated contract types via `pnpm run i18n:generate`.

---

## Verification Output

### 1. `cargo check -p northhing`
Executed using the MSVC toolchain (`rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing`):
```
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 15s
```

### 2. `pnpm --dir northing-installer run type-check`
```
> northhing-installer@0.2.10 type-check E:\agent-project\northing\northing-installer
> tsc --noEmit
```

### 3. `pnpm run i18n:generate`
```
> northhing@0.2.10 i18n:generate E:\agent-project\northing
> node scripts/generate-i18n-contract.mjs

[i18n:generate] Wrote 6 generated i18n contract file(s).
```

### 4. `pnpm run i18n:audit` / `i18n:contract:test` note
`pnpm run i18n:contract:test` was run; as noted in `AGENTS.md` ("`[missing: src/web-ui — absent in v0.1.0]`", "`[frozen: i18n engineering]`"), full contract testing fails due to absent `src/web-ui`. Contract generation (`pnpm run i18n:generate`) and frontend type checks (`pnpm --dir northing-installer run type-check`) were executed as the authoritative checks for installer locales.
