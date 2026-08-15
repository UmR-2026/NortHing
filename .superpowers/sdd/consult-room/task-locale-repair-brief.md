# Task brief — locale 树 154 基线腐蚀修复（用户授权，同审计修复例）

> 基座：worktree `E:\agent-project\northing\.worktrees\consult-room-build`，分支 `feat/consult-room-slint`，base commit = 当前 HEAD。
> 需求唯一来源 = 本文件。授权：用户 2026-08-15 裁定「locale 154 腐蚀修复」，同 i18n-audit 脚本修复例（授权例外 + 字节级证据 + 全披露）。
> 编排者预检已完成全部根因定位，§2 事实可直接信赖。

## 1. 目标

`pnpm run i18n:audit` 从 **154 errors / exit 1** 修到 **exit 0**。三层根因（ledger 在案）：
① 核心 zh-CN/zh-TW .ftl GBK 型腐蚀 + 丢键；② installer JSON 被抹成 UTF-16 空 stub + 生成的 installer 合约 ts 腐蚀；
③ 治理配置（baseline/allowlist JSON）与仓库实际形态漂移（web-ui/mobile-web/installer 在本仓为骨架/部分状态）。

## 2. 预检事实（编排者字节级实证，可直接用）

### 2.1 154 errors 分类全表
| 数量 | 类别 | 根因 | 本任务处置 |
|---|---|---|---|
| 46 | core zh-CN/zh-TW.ftl missing key（各 23） | 93f497a 品牌改名 commit 丢 23 键 | A 修复 |
| 9 | installer locales/{en,zh,zh-TW}.json parse fail | 被抹成 UTF-16 LE `{}` stub（10B） | B 重建 |
| 8 | dynamic allowlist 无 installer resource 匹配 | 同上 | B 重建后消 |
| 3+1 | namespace "shared" not in ALL_NAMESPACES + 无法 parse namespaceRegistry.ts | 该文件被转 UTF-16 LE（106B，内容 `export const ALL_NAMESPACES = ['shared'] as const;`） | C 恢复 UTF-8 |
| 29+17+3 | dynamic allowlist 无 web-ui resource / sharedTermDuplicates 0 candidates 低于基线 | `src/web-ui/src/locales/{en-US,zh-CN,zh-TW}/` 全空（无文件） | D 漂移更新 |
| ~28 | dynamic allowlist owner path 不存在 / 在 owner 文件找不到（settingsConfig.ts、shortcuts.ts、installPathErrors.ts、modelProviders.ts 等） | web-ui/installer 在本仓为骨架（web-ui 全树仅 2 个 ts；installer 仅 5 文件） | D 漂移更新 |
| 3 | sharedTermDuplicates 总/core/web-ui 低于基线 | 同上 | D 漂移更新 |
| 1 | mobile-web-source 2 条 CJK 源候选超预算 0（ThemeProvider.tsx:69,123） | src/mobile-web（58 文件，untracked 骨架）内联中文 | E 处置 |

### 2.2 腐蚀点与净本（全部已验证）
- **腐蚀 commit = `93f497a`**（chore: remove all BitFun brand references）：该 commit 里 zh-CN.ftl 已是 mojibake + BOM + 84 键（= 快照 107 键 − 丢 23）。
- **净本 = 快照 commit `1b147c3`**（2026-07-12）：zh-CN.ftl / zh-TW.ftl **完全干净**（无 BOM、无 mojibake、各 107 键）；**当前缺失的 23 键在快照里全部干净存在**（已逐键核实）。
- 当前工作树 zh-CN.ftl（147 键 = 84 腐蚀旧键 + 63 干净 dioxus 键尾段）/ zh-TW.ftl 同构（147 键）。dioxus 键段（尾部 63 键）干净，**逐字节保留不动**。
- PUA/`?` 摧毁点：zh-CN 36 PUA + 34 `?`；zh-TW 59 PUA + 33 `?`——**全部可从快照机械还原，零语义猜测**。
- 共享契约 `src/shared/i18n/contract/locales.json` **干净**（3518B，installer 段 简体中文/繁體中文/繼續 完好）——**只读**，若发现腐蚀立即停手报告。
- 生成器 `scripts/generate-i18n-contract.mjs` 从契约生成 5 件合约（web-ui ts / mobile-web ts / installer ts / core rs / installer rs）。core rs 干净；**installer ts 腐蚀**（nativeName/continueLabel mojibake）→ 重跑生成器修复（禁手改生成物）。
- 主仓 `E:\agent-project\northing` 的 zh-CN.ftl 同为腐蚀版，**不是净本**；任何分支均无 installer/web-ui 的 git 历史。
- en-US.ftl 有 BOM 但审计零投诉——**不动 en-US.ftl**。

### 2.3 审计脚本机制（修改治理配置前必读）
- `scripts/i18n-audit.mjs` 读两个**配置**（非脚本逻辑）：`scripts/i18n-governance-baseline.json`（1501B，budgets.sharedTermDuplicates 等，注明 "do not raise without review"——本任务用户授权视同 review，逐项披露即合规）与 `scripts/i18n-dynamic-key-allowlist.json`（version + entries[{id, surface, owner, description, ...}]）。
- installer JSON 审计：`auditInstallerKeyParity`（en.json 键集为基线，zh/zh-TW 对齐）+ placeholder parity + dynamic allowlist installer surface 引用。
- **禁改** `scripts/i18n-audit.mjs` 与 `scripts/generate-i18n-contract.mjs` 的逻辑（红线）；只许改上述两个配置 JSON + 重跑生成器。

## 3. 实施清单

### A. 核心 ftl 还原（zh-CN.ftl / zh-TW.ftl）
1. 以快照 `1b147c3` 为唯一源还原 107 键（zh-TW 用**自己的**快照繁体文本，禁止从简体机转）；
   快照文本做品牌改名映射（BitFun→northhing，与 93f497a 语义对齐）。
2. 尾部 63 个 dioxus-room-* 键从当前文件**逐字节拷贝**续上。
3. 终态：UTF-8 **无 BOM**、LF 行尾（快照约定）、170 键 = en-US 键集对称（审计 parity 必过）。
4. 交叉校验（报告附表）：还原后每键值 vs 当前腐蚀文件的 GBK 逆变换结果逐键比对，
   摧毁点（PUA/`?`）处列「快照原文 ← 腐蚀残迹」对照；抽查表含首尾各 5 键 + 全部 23 个复原键。

### B. installer 重建
1. `node scripts/generate-i18n-contract.mjs` 重跑（修 installer generatedLocaleContract.ts；
   报告附 git status/diff 证明其余 4 件生成物幂等零变化——core rs 是 tracked，其余 untracked 即比对内容）。
2. 重建 `northhing-Installer/src/i18n/locales/{en,zh,zh-TW}.json` 为 **UTF-8 无 BOM** 合法 JSON：
   键面以审计期望为准（先读 `auditInstallerKeyParity`/`readInstallerJsonKeys` 与 dynamic allowlist 的 installer 条目），
   内容自共享契约 installer 段 + SHARED_TERMS 导出；en 为基线，zh/zh-TW 键 parity + placeholder parity 对齐。
   **键面无法从审计/契约推出的，停手在报告列缺口**，不得杜撰键名。

### C. namespaceRegistry.ts 恢复
- 重写为 UTF-8 无 BOM，内容逐字节保持 `export const ALL_NAMESPACES = ['shared'] as const;`（语义不变，仅编码归位）。

### D. 治理配置漂移更新（每条目披露理由）
- `scripts/i18n-governance-baseline.json`：sharedTermDuplicates 各 budget 调成**修复后实测值**（core 实测可能回升；
  web-ui 资源缺席 → 0）。下调即合规；任何上调必须单列理由。
- `scripts/i18n-dynamic-key-allowlist.json`：owner path 在仓库不存在的 entries（web-ui/installer 骨架所致），
  按审计脚本支持的 schema 处置（disabled/移除/标注——先读脚本消费方式再定），每条条目 id + 理由入报告。
- mobile-web CJK 2 行（ThemeProvider.tsx:69/123）：若 mobile-web 有 locale 资源基建则抽取出源；
  无则在 baseline 作有注解的预算调整。处置方式写入报告。

### E. 终态核验（门禁，全过才算完）
1. `pnpm run i18n:audit` → **exit 0**（若有不可消余项，报告逐条列「为何不可消」并由编排者决定降级）。
2. `node scripts/generate-i18n-contract.mjs` 重跑幂等（二次运行零 diff）。
3. `rustup run stable-msvc cargo check -p northhing` exit 0（warnings 基线 lib 19 / bin 40 不增）。
   注：`rustup` 全路径 `C:\Users\UmR\.cargo\bin\rustup.exe`（PATH 无 .cargo\bin）。
4. `rustup run stable-msvc cargo test -p northhing ui_dioxus` 全过（ftl 是 dioxus i18n 加载输入）。
5. 字节证据：zh ftl 0 PUA / 0 替换符 / 无 BOM / LF；报告附 `git diff --stat` 与分类前后对照表。

## 4. 红线（授权例外的边界）

- 路径白名单（其余一律只读）：
  `src/crates/assembly/core/locales/{zh-CN,zh-TW}.ftl`、
  `northhing-Installer/src/i18n/**`、`src/web-ui/src/infrastructure/i18n/presets/namespaceRegistry.ts`、
  `scripts/i18n-governance-baseline.json`、`scripts/i18n-dynamic-key-allowlist.json`、
  `src/mobile-web/src/theme/ThemeProvider.tsx`（仅 E 处置选择抽取时）。
- **禁改** `scripts/i18n-audit.mjs`、`scripts/generate-i18n-contract.mjs`、`src/shared/i18n/contract/locales.json`。
- 编码纪律：一律 write/edit 工具（或 python 脚本显式 `encoding='utf-8', newline='\n'`），
  **禁 PowerShell 重定向/Add-Content 写源文件**（BOM 前科）。
- 零语义猜测：一切还原值可溯源（快照/契约）；推不出的停手列缺口。
- 禁任何破坏性 git 命令（reset --hard / clean / checkout -- 等）；**本任务不 commit**（编排者收口）。
- 恰好一份报告，无多余文件改动；审计配置改动不得削弱断言逻辑（只调配置值/条目状态）。

## 5. 报告

写 `.superpowers/sdd/consult-room/task-locale-repair-report.md`：
1. 154 分类前后对照表（每类处置+证据）；2. A/B/C/D/E 逐项实现摘要（文件:行号/键数）；
3. 快照交叉校验表 + 摧毁点对照样例；4. 治理配置逐条改动理由；
5. 门禁四条命令 + 输出原文；6. 偏离/缺口/风险披露。
