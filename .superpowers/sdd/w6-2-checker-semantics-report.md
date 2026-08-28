# W6-2 Task Report: rot 检查器语义修正（checker-semantics-rebase）

## 1. 改动概述 (Modification Summary)

- **范围**: 仅 `scripts/` 下 3 个文件：
  - `scripts/verify-rot-budget.mjs`
  - `scripts/verify-rot-budget.test.mjs`
  - `scripts/rot-budget.json`
- **目标**: 落实 D1 独立仲裁授权（APPROVE-FIX），修正 `collectRustFiles` 测量口径，将误计入生产指标的测试文件（`tests.rs` 及以 `_tests` 结尾的目录）排除，确保 `*_production` 指标仅测量生产代码。
- **红线遵守**: `scripts/rot-budget.json` 的全部 ceiling 数值保持**零改动**（逐字节一致）。

---

## 2. 规则与语义变更详情 (Rules and Semantic Change Details)

### 2.1 `scripts/verify-rot-budget.mjs`
- `collectRustFiles` 函数：
  1. 目录遍历跳过条件增加 `entry.name.endsWith('_tests')`（跳过如 `session_manager_lifecycle_tests/` 等测试专属子目录）。
  2. 文件收集条件从 `!entry.name.endsWith('_tests.rs')` 扩展为 `!entry.name.endsWith('_tests.rs') && entry.name !== 'tests.rs'`（排除 `tests.rs` 模块测试文件）。
  3. 路径段检查条件增加 `s.endsWith('_tests')`（防止任何含 `_tests` 目录段的文件被计入）。

### 2.2 `scripts/verify-rot-budget.test.mjs`
- 追加 2 条专属自测用例：
  1. `test('tests.rs file is excluded from rot budget measurement')`
  2. `test('*_tests directory files are excluded from rot budget measurement')`
- 自测试运行通过数由 9/9 增加至 11/11 全绿。

### 2.3 `scripts/rot-budget.json`
- 5 条 `grep-count` 规则的 `note` 追加语义重定基说明（注明 2026-08-28 与 D1 仲裁，对齐 `unix_epoch_inline` 既有风格）。
- 全部 ceiling 数值（502, 1089, 388, 69, 109 及 dir/god-file 规则）零改动。

---

## 3. rot 读数前后对比 (Rot Readings Before vs. After)

| 指标 (Metric) | 修正前实测读数 (Before) | 修正后实测读数 (After) | Ceiling | 状态 (Status) |
|---|---|---|---|---|
| `unwrap_production` | 518 | **469** | 502 | ✅ 合规（余量 33） |
| `expect_production` | 1106 | **937** | 1089 | ✅ 合规（余量 152） |
| `let_underscore` | 390 | **388** | 388 | ✅ 合规（顶格达标） |
| `unix_epoch_inline` | 69 | **69** | 69 | ✅ 合规 |
| `allow_dead_code` | 106 | **106** | 109 | ✅ 合规（W6-1 成果保持） |
| `dir_entries:scripts` | 42 | 42 | 42 | ✅ 合规 |
| `dir_entries:docs/design` | 1 | 1 | 1 | ✅ 合规 |
| `dir_entries:.superpowers/sdd` | 232 | 232 | 400 | ✅ 合规 |
| god_file (11 条) | 合规 | 合规 | 各自 ceiling | ✅ 全部合规 |

---

## 4. JSON Ceiling 未变 Diff 证据 (JSON Ceiling Unchanged Diff)

```diff
diff --git a/scripts/rot-budget.json b/scripts/rot-budget.json
index ca75634..fd134a6 100644
--- a/scripts/rot-budget.json
+++ b/scripts/rot-budget.json
@@ -3,31 +3,31 @@
   "kind": "grep-count",
   "pattern": "\\.unwrap\\(\\)",
   "ceiling": 502,
-  "note": "R-13, ratchet: only down"
+  "note": "R-13, ratchet: only down; baseline rebased 2026-08-28 per D1 adjudication to checker semantics (excludes tests.rs and *_tests/ dirs)"
  },
  "expect_production": {
   "kind": "grep-count",
   "pattern": "\\.expect\\(",
   "ceiling": 1089,
-  "note": "R-13, ratchet: only down"
+  "note": "R-13, ratchet: only down; baseline rebased 2026-08-28 per D1 adjudication to checker semantics (excludes tests.rs and *_tests/ dirs)"
  },
  "let_underscore": {
   "kind": "grep-count",
   "pattern": "let _ =",
   "ceiling": 388,
-  "note": "R-13, ratchet: only down"
+  "note": "R-13, ratchet: only down; baseline rebased 2026-08-28 per D1 adjudication to checker semantics (excludes tests.rs and *_tests/ dirs)"
  },
  "unix_epoch_inline": {
   "kind": "grep-count",
   "pattern": "duration_since\\([^\\n]*UNIX_EPOCH",
   "ceiling": 69,
-  "note": "T2-9 time-helper ratchet; canonical: northhing_core_types::time; only down; ceiling rebased 2026-08-21 to checker semantics (excludes tests dirs/_tests.rs)"
+  "note": "T2-9 time-helper ratchet; canonical: northhing_core_types::time; only down; ceiling rebased 2026-08-21 to checker semantics (excludes tests dirs/_tests.rs); rebased 2026-08-28 per D1 adjudication (excludes tests.rs and *_tests/ dirs)"
  },
  "allow_dead_code": {
   "kind": "grep-count",
   "pattern": "allow\\(dead_code\\)",
   "ceiling": 109,
-  "note": "R-13 allow(dead_code) ratchet: only down"
+  "note": "R-13 allow(dead_code) ratchet: only down; baseline rebased 2026-08-28 per D1 adjudication to checker semantics (excludes tests.rs and *_tests/ dirs)"
  },
```

---

## 5. 验证命令与输出原文 (Verification Results)

### 5.1 检查器自测试套件
- **命令**: `node scripts/verify-rot-budget.test.mjs`
- **输出**:
```text
✔ compliant fixture exits 0 and reports success (95.7393ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (92.2683ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (104.9408ms)
✔ registered god-file exceeding ceiling fails (5.7306ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (6.4917ms)
✔ dir-entry-count compliant fixture passes (102.251ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (102.5872ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (82.8579ms)
✔ tests.rs file is excluded from rot budget measurement (6.3979ms)
✔ *_tests directory files are excluded from rot budget measurement (5.9489ms)
✔ actual workspace rot budget passes with current manifest (309.8097ms)
ℹ tests 11
ℹ suites 0
ℹ pass 11
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 922.7877
```

### 5.2 真实工作区 Rot Budget 验证
- **命令**: `node scripts/verify-rot-budget.mjs`
- **输出**:
```text
Rot budget verification passed (5 grep rules [unwrap_production=469/502, expect_production=937/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=232/400], 11 god-file rules checked across 1342 files).
```

### 5.3 仓库 Hygiene 检查
- **命令**: `node scripts/check-repo-hygiene.mjs`
- **输出**:
```text
Repository hygiene check passed (5 content files scanned, 3548 filenames checked).
```

---

## 6. 偏离清单 (Deviations)

1. **`unwrap_production` 实际读数 469 vs Brief 预估 473**:
   - 原因：Brief 中的 473 估算基于 W6 计划编制时的旧基线；在前序任务 W6-1（commit `11a4e5e`）中，清理死代码时删除了 `src/apps/desktop/src/app_state/settings/keyring.rs` 中 4 个与已删死函数关联的测试用例（共包含 4 处 `.unwrap()`）。
   - 实测 473 - 4 = 469，完全符合预期，无异常偏差。
