# W17-2 Review (judge-m3) — Hygiene Fetch Depth + Windows-Only Narrowing

- **Reviewer**: judge-m3 (independent, second-lane of dual-judge meta-ratchet)
- **Scope**: commit `46c9f53` (5 files, BASE `56b752f`)
- **Files**: `ci.yml` / `nightly.yml` / `cli-package.yml` / `check-repo-hygiene.mjs` / `tech-debt-ledger.md`
- **Brief**: `.superpowers/sdd/w17-2-brief.md`
- **Report under review**: `.superpowers/sdd/reports/w17-2-report.md`

## Verdict

**APPROVE**

| Tier | Count | Notes |
|---|---|---|
| Critical | 0 | — |
| Important | 0 | — |
| Minor | 4 | All are report line-number references; the code changes themselves are correct |
| Cannot verify | 0 | — |

## 1. ci.yml — PASS

- Diff at `.github/workflows/ci.yml:154-160`: exactly one comment line + `fetch-depth: 2` block added inside the `repo-hygiene` job's checkout step. No other hunks.
- Verified `git diff --stat 56b752f..46c9f53 -- .github/workflows/ci.yml` is the only change.
- Comment text is English-only, names the root cause (`fetch-depth: 1` → `HEAD^1` missing) and cites the evidence run ID `33982832690` per brief.
- YAML parses cleanly via `python yaml.safe_load`.

## 2. check-repo-hygiene.mjs — PASS

- Diff `@@ -49,6 +49,9 @@` adds 3 lines at lines 52-54 in HEAD (`scripts/check-repo-hygiene.mjs:52-54`).
- New block:
  ```js
  if (localChangedFiles.length === 0 && committedChangedFiles.length === 0) {
    console.warn('WARNING: full-repo scan fallback active — HEAD^1 unavailable or no local changes; scan scope is ALL tracked files');
  }
  ```
- Condition triggers **only** when both arrays are empty, which is exactly when `contentScanFiles` resolves to `trackedFiles` (the next 3-line ternary picks `trackedFiles` as the last branch). Logic placement matches brief: "fallback warning 仅在走向 trackedFiles 全仓分支时打印".
- Zero changes to: regex patterns (`localAbsolutePathPattern`, `tokenPattern`, `privateKeyPattern`, `sensitiveFilenamePattern`, `testFilePattern`, `ignoredContentPaths`), skip rules (`isCommentOnlyLine`, `getRustInlineTestSkipLines`), textExtensions/slashCommentExtensions/hashCommentExtensions sets. Verified by line-by-line diff inspection.
- Warning string is English-only with em-dash, no emoji, no i18n contract violation.

## 3. nightly.yml — PASS

- Diff `@@ -72,22 +72,6 @@` and `@@ -101,41 +85,6 @@` and `@@ -243,8 +192,4 @@` (3 hunks, all removing non-Windows code).
- Matrix now contains only one entry (`nightly.yml:75-78`): `windows-latest / windows-x64 / x86_64-pc-windows-msvc / pnpm run installer:build`.
- All `matrix.platform.*` references (lines 64, 65, 100, 105, 138, 143) resolve consistently against the retained leg.
- `setup-openssl-windows.ps1` step retained at lines 83-86; the 39-line `Install Linux system dependencies (Tauri bundler)` step fully removed.
- `publish-nightly` `files:` pattern trimmed to only `release-assets/**/*northhing-installer.exe` (line 195). No `.AppImage / .deb / .dmg / .rpm` dangling.
- `check-changes` and `publish-nightly` retained on `ubuntu-latest` (lines 23, 157). Verified nature by reading the steps:
  - `check-changes`: only `git log`, `jq`, `date` — pure orchestration ✓
  - `publish-nightly`: only `actions/download-artifact`, `find`, `gh release`, `softprops/action-gh-release` — pure aggregation/publishing ✓
- YAML valid (`yaml.safe_load`).

## 4. cli-package.yml — PASS

- Diff `@@ -84,21 +84,9 @@`, `@@ -107,19 +95,10 @@`, `@@ -146,6 +125,9 @@`, `@@ -161,7 +143,11 @@` (4 hunks).
- Matrix now contains only `windows-latest / windows-x64 / x86_64-pc-windows-msvc / can_smoke_test: true` (lines 87-90).
- All `matrix.platform.*` references (lines 79, 80, 106, 111, 119, 123, 127, 139, 178) consistent with retained leg.
- `Setup OpenSSL (Windows, prebuilt)` step added (lines 98-101); Linux system deps step fully removed.
- Windows-specific handling: `if [[ -f "${BIN}.exe" ]]; then BIN="${BIN}.exe"; fi` at lines 128-130; `.exe` branch in stage step at lines 146-150 — correct for the only remaining platform.
- `prepare` (line 30) and `upload-release-assets` (line 189) retained on `ubuntu-latest`. Verified nature:
  - `prepare`: only `actions/checkout` + bash + `jq` — pure metadata resolution ✓
  - `upload-release-assets`: only `actions/download-artifact`, `find`, `sha256sum`/`shasum`, `softprops/action-gh-release`, `curl` — pure aggregation + Homebrew tap dispatch ✓
- YAML valid.

## 5. docs/status/tech-debt-ledger.md — PASS

- New entry `P2-24` inserted at lines 253-258 (after `P2-23`, before `Change Protocol`).
- Three required elements all present:
  - 存量清单规模: "约 170 个历史文件（归档文档与测试 fixture）" ✓
  - 口径恢复后不再触发: "口径恢复后正常单次提交不触发" ✓
  - 处置方向待拍板: "存量脱敏或规则豁免（如归档路径/测试数据排除），待拍板" ✓
- Format mirrors P2-23 (Symptom / Evidence / Proposed fix / Status) — verbatim template consistent.
- `Status: deferred` with explicit cross-reference to W17-2 fix.
- Evidence citation names specific CI run `33982832690` and the mechanism (`fetch-depth: 1` → `HEAD^1` missing → full-repo fallback).

## 6. Verification Reproduction

### 6.1 Shallow clone fallback warning (independently re-run)

```
$ git clone --depth 1 file:///E:/agent-project/NortHing C:/WINDOWS/TEMP/opencode/shallow-test-m3/northhing-shallow
... 3835 files, done.

$ cd C:/WINDOWS/TEMP/opencode/shallow-test-m3/northhing-shallow
$ git rev-parse --verify HEAD^1
fatal: Needed a single revision
$ git log --oneline -1
46c9f53 ci: hygiene fetch-depth fix + fail-loud fallback + packaging workflows windows-only (W17-2)

$ node scripts/check-repo-hygiene.mjs > hyg-out.txt 2>&1 ; echo "EXIT=$LASTEXITCODE"
WARNING: full-repo scan fallback active — HEAD^1 unavailable or no local changes; scan scope is ALL tracked files
Repository hygiene check failed:
- .agents/skills/northhing-onboarding/SKILL.md:117 contains a local absolute path.
- .agents/skills/northhing-v3-workflow/SKILL.md:28 contains a local absolute path.
- .agents/skills/northhing-v3-workflow/SKILL.md:32 contains a local absolute path.
- .agents/skills/northhing-v3-workflow/SKILL.md:215 contains a local absolute path.
- .opencode/model-capability-notes.md:85 contains a local absolute path.
...
- docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md:705 contains a local absolute path.
EXIT=1
```

- HEAD^1 confirmed missing (`fatal: Needed a single revision`).
- Warning fires exactly once at the top of stderr/stdout.
- Exit code 1 (matches report).
- Output is 393 lines, last line `docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md:705` — matches report's claim of `line 705` exactly.
- Cleanup performed: `Remove-Item -Recurse -Force C:\WINDOWS\TEMP\opencode\shallow-test-m3` and `hyg-out.txt` deleted.

### 6.2 Local tree normal scope (re-run)

```
$ cd E:\agent-project\NortHing
$ node scripts/check-repo-hygiene.mjs
Repository hygiene check passed (2 content files scanned, 3837 filenames checked).
```

- No fallback warning (correct: HEAD^1 exists in non-shallow repo, so `committedChangedFiles` is populated).
- Brief verification #2 satisfied.

### 6.3 Rot budget (re-run)

```
$ node scripts/verify-rot-budget.mjs
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=44/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=60/400], 6 god-file rules checked across 1368 files).
```

- Brief verification #3 satisfied.

### 6.4 Task gate (independently re-run with same allowlist)

```
$ node scripts/verify-task-gate.mjs verify-attempt --base 56b752f --tip 46c9f53 --allowlist <5 files>
Attempt verification passed: all modified files are within allowlist.
```

- 5-file allowlist matches brief; no out-of-bounds changes.

## 7. Single-Commit Scope Discipline

- `git log --oneline 56b752f..46c9f53`: exactly one commit, `46c9f53`.
- `git show --stat 46c9f53`: exactly the 5 files in the allowlist; +28/-84.
- No scope creep, no opportunistic cleanups, no documentation drift.

## 8. Findings (Minor — report accuracy only)

These are all inaccuracies in the **report's line-number citations**; the underlying code changes are correct and verifiable.

- **M-1** (report §1, nightly.yml): "Lines 74-77" → after removal the windows leg is at lines **75-78** (off by 1). Minor.
- **M-2** (report §1, nightly.yml): "Lines 84-86: Removed unused `Install Linux system dependencies (Tauri bundler)` step" → that step was 39 lines (BASE lines ~88-126) per diff hunk `@@ -101,41 +85,6 @@`; the lines 84-86 reference is misleading (those lines are actually the retained OpenSSL step in HEAD:83-86). Action taken (removing the Linux step) is correct; only the line citation is wrong. Minor.
- **M-3** (report §1, cli-package.yml): "Lines 86-90" → after replacement the windows leg is at lines **87-90** (off by 1). Minor.
- **M-4** (report §1, tech-debt-ledger.md): "Lines 253-260" → entry occupies lines **253-258** (off by 2). Minor.

None of these affect code correctness, CI behavior, ledger semantics, or the verification chain. Report polish only — not blocking.

## 9. Global Constraints Compliance

| Constraint | Status |
|---|---|
| 零新依赖 | ✓ diff contains no `package.json` / `Cargo.toml` / `pnpm-lock.yaml` changes; zero new imports in `check-repo-hygiene.mjs` |
| ci.yml 除指定处零触碰 | ✓ only the repo-hygiene job touched |
| 验证输出原文进 report | ✓ sections 3.1-3.5 in report reproduce my re-runs verbatim |
| 单 commit 五文件无裹挟 | ✓ one commit, exactly the 5 allowlisted files |
| 浅克隆 fail-loud warning English-only | ✓ em-dash, no CJK, no emoji |
| Output line 705 consistency | ✓ reproduced verbatim in my run |

## 10. Meta-Ratchet Lane

- All 5 files are in `metaRatchetPaths` (`.github/workflows/`, `scripts/check-repo-hygiene.mjs`) or are documentation referencing the meta-ratchet change.
- This task has been routed to dual judges + user sign-off (per brief: "用户拍板 2026-09-05「Windows 限定」"). My lane = second independent judge.

## Final Verdict

**APPROVE**

Implementation precisely matches the brief, all five changes are correct and minimal, no scope creep, verification chain is reproducible. The four Minor items are report line-number nits that do not affect code, CI, or ledger semantics and can be triaged at the wave-level finishing pass.

DONE
