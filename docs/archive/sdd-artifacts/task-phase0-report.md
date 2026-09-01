# Task PHASE-0 Report — 断源批（i18n 幽灵 / god-file 注释 / 守卫插电）

## 1. 复用侦察

- **`ci.yml` 结构**：`rust-build-check` 矩阵作业在 Rust 编译前执行 `node scripts/generate-i18n-contract.mjs`（由于指定 `shell: bash`，在 Linux/macOS/Windows runner 均以 Bash 语法执行）；静态检查作业（`core-boundaries`、`rot-budget`）均运行在 `ubuntu-latest` 上使用 `node-version: "22"`。
- **`check-repo-hygiene.mjs` 规则**：第 215 行正则 `/(^|[-_])review[-_]?prompt\.(txt|md)$/i` 拦截临时 review prompt 文件；`.agents/skills/lightweight-agent-execution/review-prompt.md` 命中该规则。将其重命名为 `reviewer-prompt.md` 避开正则（`reviewer` 后接 `-prompt` 不匹配 `review` 前缀）。
- **`generate-i18n-contract.mjs` 生成目标**：
  1. `src/web-ui/src/infrastructure/i18n/presets/generatedLocaleContract.ts`
  2. `northing-installer/src/i18n/generatedLocaleContract.ts`（原写 `northhing-Installer`）
  3. `src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs`
  4. `northing-installer/src-tauri/src/installer/generated_locale_contract.rs`（原写 `northhing-Installer`）

## 2. Spec 落地清单

1. **Spec 1: i18n 路径修复与断言**
   - `scripts/generate-i18n-contract.mjs` 第 15、23 行将 `northhing-Installer` 修正为 `northing-installer`。
   - 验证无遗留 `northhing-Installer/` 目录或跟踪文件。
   - 执行 `node scripts/generate-i18n-contract.mjs` 生成 4 个合约文件到正确目标路径。
   - 在 `.github/workflows/ci.yml` 的 `Generate i18n locale contract` 步骤追加断言：`test ! -d northhing-Installer && test -f northing-installer/src/i18n/generatedLocaleContract.ts`。
2. **Spec 2: 移除 allow-god-file 头注释**
   - `src/apps/cli/src/ui/theme.rs`：删除首行 `// allow-god-file: 972L ...`。
   - `src/apps/desktop/src/app_state/callbacks_lifecycle.rs`：删除首行 `// allow-god-file: 917L ...`。
3. **Spec 3: review-prompt.md 重命名**
   - 使用 `git mv` 将 `.agents/skills/lightweight-agent-execution/review-prompt.md` 重命名为 `reviewer-prompt.md`。
   - 同步修改 `.agents/skills/lightweight-agent-execution/SKILL.md` 第 68、111 行的两处引用。
4. **Spec 4: CI 插电**
   - 在 `.github/workflows/ci.yml` 新增 `repo-hygiene` 作业运行 `node scripts/check-repo-hygiene.mjs`（硬门）。
   - 新增 `i18n-contract` 作业运行 `pnpm run i18n:contract:test:ci` 并设置 `continue-on-error: true`，注明观察位说明注释（English-only）。
5. **Spec 5: 边界约束遵守**
   - 未修改 `.opencode/model-capability-notes.md`、i18n audit 其余工程文件或任何产品运行时业务代码。

## 3. 验证输出

### 验证 1: i18n 生成与路径断言
```powershell
node scripts/generate-i18n-contract.mjs && pwsh -Command "Write-Output ('Test-Path northhing-Installer: ' + (Test-Path northhing-Installer)); Write-Output ('Test-Path northing-installer/src/i18n/generatedLocaleContract.ts: ' + (Test-Path northing-installer/src/i18n/generatedLocaleContract.ts))"
```
**输出**：
```
[i18n:generate] Wrote 4 generated i18n contract file(s).
Test-Path northhing-Installer: False
Test-Path northing-installer/src/i18n/generatedLocaleContract.ts: True
```

### 验证 2: 仓库卫生检查
```powershell
node scripts/check-repo-hygiene.mjs
```
**输出**：
```
Repository hygiene check passed (7 content files scanned, 3351 filenames checked).
```

### 验证 3: 核心边界与代码腐化预算
```powershell
node scripts/check-core-boundaries.mjs && pnpm run check:rot
```
**输出**：
```
Core boundary check passed.

> northhing@0.2.10 check:rot E:\agent-project\.worktrees\northing-phase0
> node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs

✔ compliant fixture exits 0 and reports success (99.7111ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (97.4302ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (112.7825ms)
✔ registered god-file exceeding ceiling fails (5.5636ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (7.3166ms)
✔ actual workspace rot budget passes with current manifest (352.0586ms)
ℹ tests 6
ℹ suites 0
ℹ pass 6
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 680.9358
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1363 files).
```

### 验证 4: Workspace 编译检查
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
**输出**：
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 16s (0 errors)
```

### 验证 5: Git Diff 统计
```bash
git log -1 --stat
```
**输出**：
```
commit 9df49b28d6ab25adc1d73e7d84a7e4064a6b8cac
Author: Mavis <mavis@northhing.local>
Date:   Sat Aug 22 02:07:50 2026 +0800

    chore(ci): fix i18n generator paths, remove stale god-file comments, and wire CI hygiene and i18n checks (PHASE-0)

 .agents/skills/lightweight-agent-execution/SKILL.md    |  4 +--
 .../{review-prompt.md => reviewer-prompt.md}       |  0
 .github/workflows/ci.yml                           | 40 +++++++++++++++++++++-
 scripts/generate-i18n-contract.mjs                 |  4 +--
 src/apps/cli/src/ui/theme.rs                       |  1 -
 .../desktop/src/app_state/callbacks_lifecycle.rs   |  1 -
 6 files changed, 43 insertions(+), 7 deletions(-)
```

## 4. 偏离与遗留 Caveat

- **无偏离**：完全按照 Task PHASE-0 Brief 要求执行并完成验收。
- **Caveat**：`pnpm run i18n:contract:test:ci` 处于 CI 观察位（`continue-on-error: true`），其 24 项预存失败归属于当前冻结的 T2-3 / Web UI 表面，待后续解冻后转为硬门。
