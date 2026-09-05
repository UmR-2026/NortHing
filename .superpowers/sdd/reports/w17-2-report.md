# W17-2 Implementation Report: Hygiene Fetch Depth Fix & Packaging Workflows Narrowed to Windows

## 1. Modifications by File and Line

- **Change 1: `.github/workflows/ci.yml:157-160`**
  - Added `with: fetch-depth: 2` to the `actions/checkout@v4` step in `repo-hygiene` job.
  - Added comment documenting the rationale: shallow clone (`fetch-depth: 1`) leaves `HEAD^1` unavailable, triggering full-repo fallback scan as evidenced in CI run 33982832690.
  - Zero other changes in `ci.yml`.

- **Change 2: `scripts/check-repo-hygiene.mjs:52-54`**
  - Added fail-loud `console.warn` when both `localChangedFiles.length === 0` and `committedChangedFiles.length === 0`, signaling that the fallback scan over all tracked files is active.
  - Output message: `WARNING: full-repo scan fallback active — HEAD^1 unavailable or no local changes; scan scope is ALL tracked files`.
  - Zero changes to validation logic, rules, skip lists, or patterns.

- **Change 3: Packaging Workflows Windows-Only Narrowing**
  - `.github/workflows/nightly.yml`:
    - Lines 74-77: Removed `ubuntu-latest`, `ubuntu-24.04-arm`, `macos-15`, and `macos-15-intel` legs, retaining solely `windows-latest` (`windows-x64`, target `x86_64-pc-windows-msvc`, command `pnpm run installer:build`).
    - Lines 84-86: Removed unused `Install Linux system dependencies (Tauri bundler)` step.
    - Line 195: Cleaned `publish-nightly` release file pattern to only target `release-assets/**/*northhing-installer.exe`.
  - `.github/workflows/cli-package.yml`:
    - Lines 86-90: Replaced non-Windows legs (`macos-15`, `macos-15-intel`, `ubuntu-latest`, `ubuntu-24.04-arm`) with `windows-latest` (`windows-x64`, target `x86_64-pc-windows-msvc`, `can_smoke_test: true`).
    - Lines 98-101: Added `Setup OpenSSL (Windows, prebuilt)` step and removed Linux system dependencies step.
    - Lines 128-130: Added executable resolution (`if [[ -f "${BIN}.exe" ]]; then BIN="${BIN}.exe"; fi`) for smoke test.
    - Lines 146-150: Handled `.exe` staging in `dist-cli` directory.

- **Change 4: `docs/status/tech-debt-ledger.md:253-260`**
  - Added tech debt ledger entry `P2-24` recording the ~170 historical files with absolute paths exposed under full-repo fallback scan, marked `deferred`, with proposed direction for future sanitation or exemption rules.

---

## 2. Reviewer-53 Finding Fix (Important-1): i18n Contract Generation on Windows Packaging Legs

- **Issue**:
  - `northhing-core` unconditionality includes `pub mod generated_locale_contract;`.
  - The generated file `generated_locale_contract.rs` is gitignored (`.gitignore`), so fresh checkouts fail with `error[E0583]: file not found for module 'generated_locale_contract'` unless pre-generated (as evidenced in CI run 33846866557).
  - Both `nightly.yml` (Windows leg) and `cli-package.yml` (new Windows leg) previously lacked the `Generate i18n locale contract` step before building.

- **Parity with `ci.yml:52-57`**:
  - Reference step in `ci.yml:52-57` (and `ci.yml:87-93`):
    ```yaml
    - name: Generate i18n locale contract
      shell: bash
      run: |
        node scripts/generate-i18n-contract.mjs
        test ! -d northhing-Installer && test -f northing-installer/src/i18n/generatedLocaleContract.ts
      # generated_locale_contract.rs is gitignored; northhing-core fails E0583 without it
    ```
  - Both packaging workflows now adopt this exact step verbatim under `shell: bash`, positioned after repository checkout and before cargo build invocations.

- **Fix Applied**:
  - `.github/workflows/nightly.yml:137-143`: Added `Generate i18n locale contract` step in `package` job immediately before `Build desktop app`.
  - `.github/workflows/cli-package.yml:114-120`: Added `Generate i18n locale contract` step in `build` job immediately before `Build northhing-cli`.

---

## 3. Retained Ubuntu Job Justifications

As specified in W17-2 brief: any job containing cargo/Rust build or test steps must run on Windows; pure orchestration, notification, or scheduling jobs may retain `ubuntu-latest`.

1. **`nightly.yml` -> `check-changes` (`runs-on: ubuntu-latest`)**:
   - **Nature**: Pure scheduling / orchestration check.
   - **Reason**: Inspects repository commit recency via `git log -1`, `jq`, and `date` to decide whether to trigger the build. Contains no compilation, no cargo commands, and no Rust dependencies.

2. **`nightly.yml` -> `publish-nightly` (`runs-on: ubuntu-latest`)**:
   - **Nature**: Pure asset aggregation and GitHub Release publishing.
   - **Reason**: Downloads build artifacts via `actions/download-artifact`, inspects assets with `find`, purges previous nightly release via GitHub CLI (`gh release delete`), and publishes a new release using `softprops/action-gh-release`. Contains no cargo/Rust build or test execution.

3. **`cli-package.yml` -> `prepare` (`runs-on: ubuntu-latest`)**:
   - **Nature**: Pure metadata resolution.
   - **Reason**: Resolves release tag and version strings from git event metadata or `package.json` using bash and `jq`. Contains no compilation or Rust toolchain dependencies.

4. **`cli-package.yml` -> `upload-release-assets` (`runs-on: ubuntu-latest`)**:
   - **Nature**: Pure release aggregation and webhook dispatch.
   - **Reason**: Aggregates CLI tarballs, computes `SHA256SUMS`, attaches files to GitHub release, and issues repository dispatch curl call to Homebrew tap. Contains no cargo/Rust build or test execution.

---

## 4. Verification Evidence

### Verification 1: Shallow Clone Fallback Warning Reproduction
- **Procedure**: Cloned repo with `--depth 1` into a temporary directory outside workspace (`C:\WINDOWS\TEMP\opencode\shallow-test`), copied modified `scripts/check-repo-hygiene.mjs`, amended into the shallow HEAD so working tree is clean and `HEAD^1` does not exist (`git rev-parse --verify HEAD^1` fails), executed `node scripts/check-repo-hygiene.mjs`, and verified exit code and output. Deleted temporary directory after test.

- **Command & Output**:
```text
$ git rev-parse --verify HEAD^1
fatal: Needed a single revision

$ node scripts/check-repo-hygiene.mjs
WARNING: full-repo scan fallback active — HEAD^1 unavailable or no local changes; scan scope is ALL tracked files
Repository hygiene check failed:
- .agents/skills/northhing-onboarding/SKILL.md:117 contains a local absolute path.
- .agents/skills/northhing-v3-workflow/SKILL.md:28 contains a local absolute path.
- .agents/skills/northhing-v3-workflow/SKILL.md:32 contains a local absolute path.
- .agents/skills/northhing-v3-workflow/SKILL.md:215 contains a local absolute path.
- .opencode/model-capability-notes.md:85 contains a local absolute path.
...
- docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md:705 contains a local absolute path.

$ cmd /v:on /c "node scripts\check-repo-hygiene.mjs > nul 2>&1 & echo EXIT=!ERRORLEVEL!"
EXIT=1
```

### Verification 2: Local Working Tree Hygiene Check (Normal Scope)
- **Command**:
```bash
node scripts/check-repo-hygiene.mjs
```
- **Output**:
```text
Repository hygiene check passed (4 content files scanned, 3839 filenames checked).
```

### Verification 3: Rot Budget Verification
- **Command**:
```bash
node scripts/verify-rot-budget.mjs
```
- **Output**:
```text
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=44/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=60/400], 6 god-file rules checked across 1368 files).
```

### Verification 4: Workflow YAML Syntax Validation
- **Command**:
```bash
python -c "import yaml
files = ['.github/workflows/ci.yml', '.github/workflows/nightly.yml', '.github/workflows/cli-package.yml']
for f in files:
    with open(f, 'r', encoding='utf-8') as stream:
        yaml.safe_load(stream)
    print(f'{f}: YAML syntax valid')
"
```
- **Output**:
```text
.github/workflows/ci.yml: YAML syntax valid
.github/workflows/nightly.yml: YAML syntax valid
.github/workflows/cli-package.yml: YAML syntax valid
```

### Verification 5: Task Gate Allowlist Verification
- **Command (Initial Commit `56b752f..46c9f53`)**:
```bash
node scripts/verify-task-gate.mjs verify-attempt --base 56b752f --tip 46c9f53 --allowlist C:\WINDOWS\TEMP\opencode\allowlist.txt
```
- **Output**:
```text
Attempt verification passed: all modified files are within allowlist.
```

- **Command (Fix Commit `46c9f53..9aa5762`)**:
```bash
node scripts/verify-task-gate.mjs verify-attempt --base 46c9f53 --tip 9aa5762 --allowlist C:\WINDOWS\TEMP\opencode\allowlist-fix.txt
```
- **Output**:
```text
Attempt verification passed: all modified files are within allowlist.
```

---

## 5. Git Commit History

1. `46c9f53`: `ci: hygiene fetch-depth fix + fail-loud fallback + packaging workflows windows-only (W17-2)`
   - 5 files: `.github/workflows/ci.yml`, `.github/workflows/cli-package.yml`, `.github/workflows/nightly.yml`, `docs/status/tech-debt-ledger.md`, `scripts/check-repo-hygiene.mjs`
2. `9aa5762`: `fix(ci): add i18n contract generation to windows packaging legs (W17-2)`
   - 2 files: `.github/workflows/cli-package.yml`, `.github/workflows/nightly.yml`

---

DONE
