# Task Brief T2-2i — remote 栈子批 C7：mobile-web i18n 契约面摘除

Roadmap: `docs/architecture/backend-roadmap.md` T2-2。批次：`.superpowers/sdd/task-t2-2c-recon.md` §C7（relay 面已在 C5 随 f6a011b 摘除，本批只剩 mobile-web）。前置：C6（646f93d）已删 `src/mobile-web/` 整目录——契约面不摘就成悬空注册。

## Goal

摘除 i18n 契约面的 mobile-web surface 注册与全部 mobile 专属审计/生成/测试逻辑。frozen i18n 工程——只删 mobile 面，desktop/core/installer/server 存活 surface 逻辑零扰动。

## 已核实事实（编排者 2026-08-19 亲验）

- `src/shared/i18n/contract/locales.json`：:11（surfaceDefaults mobile-web）、:21-25（surfaceLocales/orders 内 mobile-web 列表）、:42-45（surfaces 内 mobile-web 块，resourceRoot 指向已删的 src/mobile-web/src/i18n）。
- `scripts/generate-i18n-contract.mjs`：:15 生成目标 `src/mobile-web/.../generatedLocaleContract.ts`（目标目录已不存在，必须删条目）；:291-292 surface 读取；:306-331 mobile fallback chain 生成器（`MOBILE_LOCALES` / `mobileLocaleAliasesByPriority` / `getMobileFallbackChain` 相关）。
- `scripts/i18n-audit.mjs`：38 处 mobile 命中——:28-29 路径常量、`extractMobilePlaceholders`（:229）、`readMobileMessagesByLocale`（:552-596）、:671 surface 分支、`auditMobileWebBoundary`（:732）及后续 mobile 审计函数与调用点。
- `scripts/i18n-contract.test.mjs`：22 处——:15 生成文件列表项、:179-205 mobile messages/I18nProvider 断言、:337/:358/:362 audit 源断言、:630-700 附近 mobile 集成测试块。
- baseline：`scripts/i18n-governance-baseline.json`（2 处 mobile 键）、`scripts/i18n-hardcoded-baseline.json`（1 处）。dynamic-key/l10n-identical allowlist 无 mobile 键（C5 已确认过 relay；mobile 需复核）。
- ⚠️ **i18n-audit.mjs 有 pre-existing mojibake 语法损伤**（:503 附近 `'è¿?,` 字符串截断，node --check 必报 SyntaxError——T2-2g 已实证基线如此）。本批**不得修复也不得扩展**该损伤；编辑后 node --check 应报**同一位置同一错误**（行号随删除前移）。

## Files

1. `src/shared/i18n/contract/locales.json`：删 mobile-web 的 3 处（surfaceDefaults / surfaceLocale 列表 / surfaces 块），JSON 合法。
2. `scripts/generate-i18n-contract.mjs`：删 :15 生成目标条目 + mobile fallback chain 生成器全部 mobile 专属代码；若删除后生成器输出变化（生成文件数 5→4 之类），跑 `node scripts/generate-i18n-contract.mjs` 实际重生并纳入改动；若 `generated_locale_contract.rs`（core）内容变化，必须纳入并跑 cargo check。
3. `scripts/i18n-audit.mjs`：删 mobile 常量、mobile 读取/抽取/审计函数、surface 分支的 mobile 臂、顶层调用。**逐个符号查调用图**：被存活 surface 共享路径引用的 mobile helper 不得删（保留或最小泛化，报告说明取舍）。
4. `scripts/i18n-contract.test.mjs`：删上列 mobile 断言与集成测试块。
5. 两个 baseline JSON：删 mobile-web 键。
6. `rg -n "mobile" scripts/*.mjs scripts/*.json src/shared/i18n` 复核无遗漏（文档/注释中的历史提及除外，报告逐条列）。

## Constraints

1. 只删 mobile 面；desktop/core/installer/server 存活 surface 逻辑零改动。
2. i18n-audit.mjs 的 pre-existing mojibake 损伤字节原样保留；非 mobile 内容字节不动（判据：`-w` diff 全为 mobile 相关 hunk，numstat 尽量接近纯删除）。
3. 不顺手重构；不动 memory/、.opencode/、.superpowers/sdd/ 其它 task-*、前端文件；不 commit 不 push。

## Verification（原始输出贴报告）

```powershell
node -e "JSON.parse(require('fs').readFileSync('src/shared/i18n/contract/locales.json','utf8')); console.log('locales.json OK')"
node scripts/generate-i18n-contract.mjs --check   # 或直接生成后 git diff 核对
node --check scripts/generate-i18n-contract.mjs
node --check scripts/i18n-contract.test.mjs
node --check scripts/i18n-audit.mjs   # 期望：与基线同一 SyntaxError（行号前移），截图/贴文本证明错误形态一致
node scripts/check-core-boundaries.mjs
# 若 generated_locale_contract.rs 有变：
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
# 归零（残留逐条解释）：
rg -n "mobile-web|mobileWeb|Mobile" src/shared/i18n scripts --glob "*.mjs" --glob "*.json"
```

## Report

写 `.superpowers/sdd/task-t2-2i-report.md`：status、逐文件摘除点清单、共享 helper 取舍说明、生成器输出变化核对、mojibake 一致性证明、验证原始输出、遗留疑虑。假汇报 = 停用。
