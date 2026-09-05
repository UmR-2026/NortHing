# W16-5 Report: Unify Review Verdict Vocabulary and Extend metaRatchetPaths

## 改动摘要

1. **I-1 Unified Review Verdict Vocabulary**:
   - Updated Housekeeping Rule 8.3 in `AGENTS-CN.md` to establish `reviewVerdicts` in `scripts/workflow-policy.json` as the sole vocabulary:
     - `3. 审查结论以 \`scripts/workflow-policy.json\` 的 \`reviewVerdicts\` 为唯一词表（当前为 APPROVE / APPROVE_WITH_CONCERNS / CANNOT_VERIFY / BLOCKED / FAIL）；CANNOT_VERIFY 按 \`scripts/workflow-policy.json\` 的 \`cannotVerifyPolicy\` 分级...`
   - Updated Housekeeping Rule 8.3 in `AGENTS.md` synchronously:
     - `3. Review verdicts use \`reviewVerdicts\` in \`scripts/workflow-policy.json\` as the sole vocabulary (currently APPROVE / APPROVE_WITH_CONCERNS / CANNOT_VERIFY / BLOCKED / FAIL); CANNOT_VERIFY is tiered per \`cannotVerifyPolicy\` in \`scripts/workflow-policy.json\`...`
   - Both files modified at exactly one location, preserving subsequent `CANNOT_VERIFY` tiering semantics.

2. **I-2 Extended metaRatchetPaths in Workflow Policy**:
   - Added 4 critical gate and wiring files to `metaRatchetPaths` in `scripts/workflow-policy.json` while keeping the original 4 intact:
     - `scripts/check-repo-hygiene.mjs`
     - `scripts/check-core-boundaries.mjs`
     - `scripts/check-github-config.mjs`
     - `package.json`

## 验证

### 1. Policy Validation (`node scripts/verify-task-gate.mjs validate-policy`)
Exit code: 0
```text
Policy validation passed: E:\agent-project\northing\scripts\workflow-policy.json
```

### 2. Task Gate Selftest (`node scripts/verify-task-gate.mjs --selftest`)
Exit code: 0
```text
[PASS] negative fixture a: replay W15-1l real incident (detected out-of-bounds pages_archive.rs)
[PASS] negative fixture b: invalid git revision rejected
[PASS] negative fixture c: missing required section in brief rejected
[PASS] negative fixture d: unapproved exemption phrase rejected
[PASS] negative fixture e: prejudging reviewer phrase in prose rejected
[PASS] negative fixture f: bad policy missing required field rejected
[PASS] negative fixture g: policy enum mismatch rejected
[PASS] positive fixture 1: complete 8-file allowlist passes
[PASS] positive fixture 2: allowlist with unfulfilled file passes with warning
[PASS] positive fixture 3: w16-1-brief.md passes validate-brief
[PASS] positive fixture 4: default workflow-policy.json passes validate-policy
Selftest passed: 11 fixtures passed (7 negative, 4 positive).
```

### 3. Repository Hygiene (`node scripts/check-repo-hygiene.mjs`)
Exit code: 0
```text
Repository hygiene check passed (5 content files scanned, 3828 filenames checked).
```

### 4. Rot Budget Verification (`node scripts/verify-rot-budget.mjs`)
Exit code: 0
```text
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=370/388, unix_epoch_inline=69/69, allow_dead_code=104/109], 3 dir rules [dir_entries:scripts=44/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=58/400], 6 god-file rules checked across 1368 files).
```

## 状态

DONE
