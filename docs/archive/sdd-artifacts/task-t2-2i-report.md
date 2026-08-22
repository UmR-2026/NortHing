# Task Report T2-2i — remote 栈子批 C7：mobile-web i18n 契约面摘除

- **Status**: DONE
- **Task ID**: T2-2i
- **Branch**: main
- **Timestamp**: 2026-08-19

---

## 1. 逐文件摘除点清单

### (1) `src/shared/i18n/contract/locales.json`
- `surfaceDefaults`: 摘除 `"mobile-web": "en-US"`
- `surfaceOrders`: 摘除 `"mobile-web": ["zh-CN", "zh-TW", "en-US"]`
- `surfaces`: 摘除 `"mobile-web"` 块（`resourceRoot: "src/mobile-web/src/i18n"`, `loading: "surface-minimal"`）
- 存活 surface（`web-ui` / `installer` / `core`）零改动，JSON 解析校验合法。

### (2) `scripts/generate-i18n-contract.mjs`
- `outputs` 数组：摘除 `src/mobile-web/src/i18n/generatedLocaleContract.ts` 输出目标配置
- 契约生成函数：摘除 `generateMobileLocaleContract` 及其内部全部生成逻辑（`MOBILE_LOCALES`、`UNKNOWN_LANGUAGE_FALLBACK_CHAIN`、`mobileLocaleAliasesByPriority`、`MobileLanguage` 类型、`DEFAULT_LANGUAGE`、`SHARED_TERMS_BY_LOCALE`、`isMobileLanguage`、`resolveMobileLanguage`、`getNextMobileLanguage`、`getMobileLanguageShortName`、`getMobileFallbackChain`）
- 生成目标文件数由 5 个调整为 4 个（`web-ui`、`installer` TS/Rust、`core` Rust）

### (3) `scripts/i18n-audit.mjs`
- 路径常量：摘除 `mobileWebSourceDir`、`mobileWebMessagesPath`
- 占位符解析：摘除 `extractMobilePlaceholders`
- AST 读取函数：摘除 `flattenTsObjectKeys`、`flattenTsObjectEntries`、`readMobileMessagesByLocale`、`readMobileMessageKeysByLocale`
- 契约资源根审计：摘除 `auditSurfaceResourceRoots` 中的 `mobile-web` 分支
- 边界与一致性审计：摘除 `auditMobileWebBoundary`、`auditMobileWebMessageParity`、`auditMobileWebPlaceholderParity`
- 资源全量收集：摘除 `collectI18nResourceEntries` 中针对 `mobile-web` 消息树的扫描与 `pushResourceEntry`
- 扫描跳过判断：摘除 `shouldSkipMobileWebSourceScan`
- 格式扫描配置：摘除 `shouldSkipLocaleFormatSourceScan` 中针对 `I18nProvider.tsx` 的豁免，摘除 `createLocaleFormatScanSpecs` 中 `mobile-web` 条目（保留 `core-miniapp` 与其他存活 surface）
- 硬编码预算：摘除 `auditHardcodedSourceBudgets` 中 `mobile-web-source` 规格
- 顶层执行点：摘除 `auditMobileWebBoundary()`、`auditMobileWebMessageParity()`、`auditMobileWebPlaceholderParity()` 调用
- **Mojibake 字节原样保留**：`zhTwSameTextScriptSignals` 未作任何改动

### (4) `scripts/i18n-contract.test.mjs`
- `expectedGeneratedFiles`：摘除 `'src/mobile-web/src/i18n/generatedLocaleContract.ts'`
- 运行时断言：摘除 `shared i18n terms are consumed by each product surface runtime` 中的 `mobile-web` 检查
- 回退链断言：摘除 `frontend runtimes use generated locale defaults and fallback chains` 中的 `getMobileFallbackChain` 检查
- 审计逻辑断言：摘除 `auditMobileWebMessageParity`、`auditMobileWebPlaceholderParity`、`extractMobilePlaceholders` 正则匹配断言
- 集成测试块：摘除 `auditIntegrationTest('mobile-web uses shared terms for stable shared concept labels', ...)` 整块用例
- 契约资源对齐测试：摘除 `i18n contract locales and resources align with contract` 中的 `mobile-web` 分支

### (5) `scripts/i18n-governance-baseline.json`
- `budgets.sharedTermDuplicates.bySurface`：摘除 `"mobile-web": 0`
- `budgets.l10nQualityCandidates.bySurface`：摘除 `"mobile-web": 0`

### (6) `scripts/i18n-hardcoded-baseline.json`
- `budgets` 数组：摘除 `{ "id": "mobile-web-source", "maxCjkLines": 0 }`

---

## 2. 共享 Helper 取舍说明

在 `scripts/i18n-audit.mjs` 的 AST 分析函数中：
1. `unwrapTsExpression`：被存活的 `auditWebUiStaticTranslationKeys`（行 1650/1659/1837/1863 等）以及选项解构多处直接引用，**属于共享 helper，严格保留**。
2. `propertyNameToString`：被 `auditWebUiStaticTranslationKeys`（行 1733/1734/1868 等）引用，**属于共享 helper，严格保留**。
3. `diffSets`：被全 surface 审计函数共享引用，**严格保留**。
4. `flattenTsObjectKeys` / `flattenTsObjectEntries`：仅在已删的 `readMobileMessagesByLocale` 中自递归或调用，无存活 surface 引用，**随 mobile 读取逻辑一并摘除**。

---

## 3. 生成器输出变化核对

- 运行 `node scripts/generate-i18n-contract.mjs` 输出：`[i18n:generate] Wrote 4 generated i18n contract file(s).`
- 运行 `node scripts/generate-i18n-contract.mjs --check`：无输出，退出码 0，表明存活 4 个生成文件已与契约完全对齐。
- `src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs` 内容对比：`git diff` 为 0 改动（因为 `core` 的 surface order 与 locale 定义未受 mobile-web 影响）。

---

## 4. Mojibake 一致性证明

`scripts/i18n-audit.mjs` 的 pre-existing mojibake 语法损伤字节原样保留。

- **编辑前**：
  ```
  E:\agent-project\northing\scripts\i18n-audit.mjs:503
    'è¿?,
    ^^^^^
  SyntaxError: Invalid or unexpected token
  ```
- **编辑后**（因前面删除 `mobileWebSourceDir`、`mobileWebMessagesPath`、`extractMobilePlaceholders` 等行，行号前移）：
  ```
  E:\agent-project\northing\scripts\i18n-audit.mjs:481
    'è¿?,
    ^^^^^
  SyntaxError: Invalid or unexpected token
  ```
- 报错形态、截断字符 `'è¿?,` 及错误类型完全一致。

---

## 5. 验证原始输出

### (1) locales.json 解析验证
```powershell
PS E:\agent-project\northing> node -e "JSON.parse(require('fs').readFileSync('src/shared/i18n/contract/locales.json','utf8')); console.log('locales.json OK')"
locales.json OK
```

### (2) generate 脚本校验与语法检查
```powershell
PS E:\agent-project\northing> node scripts/generate-i18n-contract.mjs --check
PS E:\agent-project\northing> node --check scripts/generate-i18n-contract.mjs
(exit code 0)
```

### (3) contract test 脚本语法检查
```powershell
PS E:\agent-project\northing> node --check scripts/i18n-contract.test.mjs
(exit code 0)
```

### (4) audit 脚本语法与 Mojibake 一致性检查
```powershell
PS E:\agent-project\northing> node --check scripts/i18n-audit.mjs
E:\agent-project\northing\scripts\i18n-audit.mjs:481
  'è¿?,
  ^^^^^

SyntaxError: Invalid or unexpected token
    at checkSyntax (node:internal/main/check_syntax:72:5)

Node.js v24.19.0
```

### (5) check-core-boundaries 边界检查
```powershell
PS E:\agent-project\northing> node scripts/check-core-boundaries.mjs
Core boundary check passed.
```

### (6) Cargo Workspace 编译检查（MSVC Wrapper）
```powershell
PS E:\agent-project\northing> & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 52.05s
```

### (7) 契约面 mobile 归零复核
```powershell
PS E:\agent-project\northing> rg -n "mobile-web|mobileWeb|Mobile" src/shared/i18n scripts --glob "*.mjs" --glob "*.json"
(no output - 0 results)
```

注：全局检索 `rg -n "mobile-web|mobileWeb|Mobile|mobile" src/shared/i18n scripts` 仅命中 `scripts/check-repo-hygiene.mjs:97`（`\.mobileprovision` 文件名安全扩展名规则），与 mobile-web i18n 契约无关。

---

## 6. Git Diff 规模统计

```
 scripts/generate-i18n-contract.mjs    |  74 ------
 scripts/i18n-audit.mjs                | 219 ----------------------------------
 scripts/i18n-contract.test.mjs        |  89 --------------
 scripts/i18n-governance-baseline.json |   2 -
 scripts/i18n-hardcoded-baseline.json  |   4 -
 src/shared/i18n/contract/locales.json |  10 --
 6 files changed, 0 insertions(+), 398 deletions(-)
```
所有 6 个代码文件的 diff 均为纯删除（0 insertions），无任何存活 surface 逻辑扰动与未授权重构。

---

## 7. 遗留疑虑

无。C7 子批目标已全量达成。
