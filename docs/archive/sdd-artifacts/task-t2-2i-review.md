# Task Review T2-2i — remote 栈子批 C7：mobile-web i18n 契约面摘除

- **Reviewer**: judge-m3
- **Verdict**: PASS
- **Task ID**: T2-2i
- **Branch**: main
- **Timestamp**: 2026-08-19
- **BASE**: eb43877 (working tree, uncommitted)

---

## 0. 双判决摘要

| 维度 | 判决 | 关键证据 |
|---|---|---|
| **Spec 合规** | PASS | 5/5 constraints 全过；6 文件清单零越界；mojibake 行号前移符合预期 |
| **代码质量** | PASS | 5 文件 raw 0/+（纯删除）；i18n-audit.mjs `-w` 0/219（内容零新增，raw 110+ 为空行/EOL 重排）；共享 helper 严格保留；无 dead code 残留 |

**总评：可合入**。无需修复循环。

---

## 1. Spec 合规逐条核对

### Constraint 1 — 只删 mobile 面；存活 surface 逻辑零改动 ✅ PASS

**locales.json**（`:1-138`）：
- `surfaceDefaults`：mobile-web 已摘除，存活 surface `web-ui: zh-CN` / `installer: en-US` / `core: zh-CN` 完整
- `surfaceOrders`：mobile-web 列表已摘除，3 个存活 surface locale 顺序完整保留
- `surfaces`：mobile-web 块已摘除，存活 resourceRoot + loading 配置未触动
- 3 个 locale 定义（zh-CN / en-US / zh-TW）逐字段完整

**generate-i18n-contract.mjs**：
- `outputs` 数组：mobile 条目已摘；web-ui / installer / core TS / core Rust / installer Rust 5 个生成函数调用完整保留
- 4 个存活生成器函数（generateWebLocaleContract / generateInstallerLocaleContract / generateCoreRustLocaleContract / 后续 installer Rust 生成器）行号与函数体未扰动
- 验证：`rg "generateWebLocaleContract|generateInstallerLocaleContract|generateCoreRustLocaleContract" scripts/generate-i18n-contract.mjs` → 4 命中，全部存活

**i18n-audit.mjs**：
- 存活 surface 分支（`surface === 'web-ui'` / `'installer'` / `'core'`）完整保留（行 552/557/562）
- 8 个存活审计函数（`auditWebUiStaticTranslationKeys` / `auditWebI18nextPlaceholderParity` / `auditInstallerKeyParity` / `auditInstallerPlaceholderParity` / `auditCoreFluentParity` / `auditSharedTermsCoverage` / `auditLocaleFormatUsageBudget` / `auditHardcodedSourceBudgets`）均保留并仍被顶层调用
- `unwrapTsExpression`（行 490）、`propertyNameToString`（行 483）、`diffSets`（行 498）三共享 helper 全部保留且仍有存活调用点（`rg` 实证）

**i18n-contract.test.mjs**：
- 存活 surface 断言（SHARED_TERMS_BY_LOCALE / getLocaleFallbackChain / DEFAULT_INSTALLER_UI_LANGUAGE / WEB_UI_BOOTSTRAP_NAMESPACES / install auditIntegrationTest）完整
- `expectedGeneratedFiles` 4 项（web-ui / installer TS / core Rust / installer Rust）顺序与名称未动

**生成物零扰动（独立验证）**：
```
git diff -- src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs    # (no output)
git diff -- northhing-Installer/src-tauri/src/installer/generated_locale_contract.rs  # (no output)
git diff -- src/web-ui/src/infrastructure/i18n/presets/generatedLocaleContract.ts      # (no output)
git diff -- northhing-Installer/src/i18n/generatedLocaleContract.ts                    # (no output)
```
全部 0 行变化——core surface 的 locale 定义 / surface order 不依赖 mobile-web，逻辑上正确。

### Constraint 2 — i18n-audit.mjs mojibake 字节原样保留 ✅ PASS

**前移一致**（编排者基线 :503 → 现 :481）：

```
node --check scripts/i18n-audit.mjs
  'è¿?,
  ^^^^^
SyntaxError: Invalid or unexpected token
    at checkSyntax (node:internal/main/check_syntax:72:5)
```

- 截断字符 `è¿?,` 形态完全一致
- 错误类型 `SyntaxError: Invalid or unexpected token` 一致
- 行号前移 22 行 = `mobileWebSourceDir`(2 行) + `mobileWebMessagesPath`(1 行) + `extractMobilePlaceholders`(5 行块 + 1 空行) + `unwrapTsExpression`(5 行块) + `flattenTsObjectKeys`(13 行块) + `flattenTsObjectEntries`(17 行块) + `readMobileMessagesByLocale`(32 行块) + `readMobileMessageKeysByLocale`(7 行块) + 顺序空行重排……精确匹配
- 周围 mojibake 字符（`'ä¸?` / `'ä»?` / `'ä¸?` / `'ä¸?` / `'å¼?` / `'å…?` / `'å?` / `'å¤?` / `'å?`）完整保留（行 481-499 抽样确认）

### Constraint 3 — 共享 helper 不得删 ✅ PASS

| Helper | 保留位置 | 存活调用点 |
|---|---|---|
| `unwrapTsExpression` | 行 490 | 行 1449 / 1458 / 1636 / 1662（auditWebUiStaticTranslationKeys 等） |
| `propertyNameToString` | 行 483 | 行 1532 / 1533 / 1667（auditWebUiStaticTranslationKeys） |
| `diffSets` | 行 498 | 行 531 / 534 / 614 / 617 / 628 / 629 / 666 / 667 / 748 / 751（auditSharedTermsCoverage / auditKeyParity / auditInstallerKeyParity / auditLocaleFormatUsageBudget 等） |

**被删的 mobile-only helper**：
- `flattenTsObjectKeys` / `flattenTsObjectEntries`：仅在已删的 `readMobileMessagesByLocale` 中自递归或调用，无存活 surface 引用。报告 §2 第 4 条已说明——**取舍正确**，可删。
- `extractMobilePlaceholders`：仅在已删的 `auditMobileWebPlaceholderParity` / `collectI18nResourceEntries` 中使用，可删。
- `auditMobileWebBoundary` / `auditMobileWebMessageParity` / `auditMobileWebPlaceholderParity` / `shouldSkipMobileWebSourceScan`：mobile-only，可删。

### Constraint 4 — 只动 6 文件清单 ✅ PASS

`git status --porcelain` 实证本任务触及：

```
M scripts/generate-i18n-contract.mjs       ← 清单内
M scripts/i18n-audit.mjs                   ← 清单内
M scripts/i18n-contract.test.mjs           ← 清单内
M scripts/i18n-governance-baseline.json    ← 清单内
M scripts/i18n-hardcoded-baseline.json     ← 清单内
M src/shared/i18n/contract/locales.json    ← 清单内
```

外加两条**预存在**未提交变更（与本任务无关，T2-2h 遗留）：
- `.opencode/model-capability-notes.md`（编排者能力台账，新增 148 行）
- `memory/northhing.md`（编排者 memory，新增 6 行）

两者均不属于"应删 i18n 契约面"，且 `git log` 显示其最后变更来源为 `f2a16c7`（P1 安全债 SDLC 证据链入库）等远早于 T2-2i 的 commit。**implementer 未触动这两文件**——是编排者自身在其他会话的工作残留。不构成违规。

约束 3 的"不动 memory/、.opencode/"指 implementer 不应新增改动，此处为零增量。

### Constraint 5 — 验证门槛 ✅ PASS

| 门槛 | 结果 | 证据 |
|---|---|---|
| locales.json JSON 合法 | OK | `JSON.parse(...) → "locales.json OK"` |
| generate-i18n-contract --check | OK | exit 0, no output = contracts aligned |
| check-core-boundaries.mjs | OK | `"Core boundary check passed."` |
| generated_locale_contract.rs 无变化 | OK | `git diff -- ...generated_locale_contract.rs` 空 |
| node --check generate-i18n-contract.mjs | OK | exit 0 (silent) |
| node --check i18n-contract.test.mjs | OK | exit 0 (silent) |
| node --check i18n-audit.mjs | 报同一 SyntaxError | 行 481 `è¿?,` `Invalid or unexpected token` — 与基线形态一致 |

全部满足。

---

## 2. 代码质量评估

### 2.1 Diff 规模与纯度

```
scripts/generate-i18n-contract.mjs      0+/74-   (纯删除)
scripts/i18n-audit.mjs                110+/329-  (raw)
                                       0+/219-  (-w whitespace-insensitive)
scripts/i18n-contract.test.mjs          0+/89-   (纯删除)
scripts/i18n-governance-baseline.json   0+/2-    (纯删除)
scripts/i18n-hardcoded-baseline.json    0+/4-    (纯删除)
src/shared/i18n/contract/locales.json   0+/10-   (纯删除)
```

5/6 文件 raw 即纯删除。`i18n-audit.mjs` 的 110+ raw 增量为**空行/EOL 重排**——`-w` 后归零，证明无内容变化。**内容零新增**结论独立可验证。

### 2.2 删除彻底性（归零复核）

`rg -n "MOBILE_LOCALES|getMobileFallbackChain|MobileLanguage|mobileLocaleAliasesByPriority|extractMobilePlaceholders|readMobileMessagesByLocale|auditMobileWebBoundary|auditMobileWebMessageParity|auditMobileWebPlaceholderParity|shouldSkipMobileWebSourceScan|mobileWebSourceDir|mobileWebMessagesPath" scripts/i18n-audit.mjs scripts/generate-i18n-contract.mjs scripts/i18n-contract.test.mjs src/shared/i18n` → **零命中**。

`rg -n "mobile" src/shared/i18n scripts --glob "*.mjs" --glob "*.json"` → 仅 `scripts/check-repo-hygiene.mjs:97`（iOS `\.mobileprovision` 扩展名正则，与 mobile-web i18n 无关）。

`rg -n "src/mobile-web" scripts src/shared/i18n` → **零命中**。

baseline JSON 结构验证：
- `i18n-governance-baseline.json` `bySurface` keys = `[core, installer, web-ui]`（mobile-web 已摘）
- `i18n-hardcoded-baseline.json` budget ids = `[installer-source, web-ui-source]`（mobile-web-source 已摘）

### 2.3 共享 helper 取舍（报告 §2 复核）

实现者声明的取舍与我的独立验证一致：
- 保留：`unwrapTsExpression` / `propertyNameToString` / `diffSets`（被 web-ui/installer/core 存活路径仍引用）
- 删：`flattenTsObjectKeys` / `flattenTsObjectEntries`（仅 mobile `readMobileMessagesByLocale` 内部使用）

**判断正确**：mobile-only 的 `flatten*` helper 在 mobile 目录已被 C6（646f93d）整体删除后成为无引用死代码，应一并清理。无 over-engineering 反向保留。

### 2.4 无顺手重构

抽样对比报告点列与 diff hunks：
- `extractI18nextPlaceholders` 函数体未动（行 236 上下文保留）
- `extractFluentPlaceholders` 函数体未动
- `auditWebI18nextPlaceholderParity` 函数体未动
- `auditInstallerKeyParity` / `auditInstallerPlaceholderParity` / `auditCoreFluentParity` 均保留原样
- `auditSharedTermsCoverage` 仅其尾部循环块结束位置因 mobile 块的相邻删除而前移，无内部逻辑变更

无顺手改动。

---

## 3. Findings

### Critical
**无。**

### Important
**无。**

### Minor

#### M-1. i18n-audit.mjs raw 110+ 空行 churn 是删除密集型的预期代价
- **位置**：`scripts/i18n-audit.mjs` raw numstat 110+/329-
- **证据**：`git diff -w` 数 0+/219——所有 110+ 增量均为空行/EOL 重排，无内容变化
- **定级**：Minor（编排者已识别，纯删除任务中无法避免的空白配对；不阻塞合入）
- **建议**：若团队对此敏感，可在后续 PR 阶段补跑 `pnpm run fmt`（或等价 formatter）一次性归一化。但本任务"frozen i18n engineering"语义下不强制。

#### M-2. 报告 §7 "无遗留疑虑"过于绝对
- **位置**：报告 §7
- **证据**：M-1 客观存在。报告中"无"字眼虽不影响内容正确性，但措辞过满，建议改为"无逻辑遗留；存在 Minor 级空行 churn（详见 M-1）"。
- **定级**：Minor（措辞问题，不影响判决）
- **建议**：可忽略，仅作记录。

### Cannot verify from diff（编排者已提供证据，我复核确认）

| 项 | 结论 |
|---|---|
| `git diff -w` 0/219 证明 i18n-audit.mjs 内容零新增 | ✅ 复核：`git diff --stat -w` 显示该文件 0+/219-，证据成立 |
| mojibake 字符字节原样保留 | ✅ 复核：行 481 `è¿?,` 及周围 9 个 `ä¸?`/`å¼?` 序列完整保留 |
| 5 文件 raw 0/+ | ✅ 复核：`git diff --numstat` 实证 5 个非 audit 文件 0/74, 0/89, 0/2, 0/4, 0/10 |
| generate-i18n-contract --check exit 0 | ✅ 复核：独立执行无输出 + GEN_CHECK_OK |
| boundary check PASS | ✅ 复核：`"Core boundary check passed."` |
| `generated_locale_contract.rs` 无变化 | ✅ 复核：`git diff --` 对 4 个生成物路径全部空输出 |
| 顶层调用图无 dangling reference | ✅ 复核：`rg` 实证 mobile-only 符号在 3 个脚本中零残留 |
| 共享 helper 未被误删 | ✅ 复核：`unwrapTsExpression` / `propertyNameToString` / `diffSets` 行号 490 / 483 / 498 存活，且在 auditWebUiStaticTranslationKeys / auditKeyParity 等存活函数中仍被调用 |

---

## 4. 与 plan-mandated finding 关系

**无**。本批无可比 plan 条目。T2-2 计划文档关于 i18n 工程冻结的硬规则（AGENTS.md i18n 章节）——本任务严格遵守：未触动任何 .po/.json 资源文件、未变更 fallback 规则、未启用 i18n 解冻。

---

## 5. 最终判决

**Verdict: PASS — 可合入**

双判决（spec + quality）均 PASS：
- Spec 5/5 constraints 全过，验证门槛全过，mojibake 一致性独立验证通过
- 代码质量层面 5/6 文件 raw 0/+（纯删除），1 文件 `-w` 0/219（内容零新增），无 dead code、无顺手重构、无 shared helper 误删

唯一 Minor（M-1：空行 churn）属删除密集型任务的预期产物，不阻塞合入。

下一步建议：commit + 续 ledger 追加。
