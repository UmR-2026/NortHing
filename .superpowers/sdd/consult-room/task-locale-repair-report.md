# Task Report — locale 树 154 基线腐蚀修复（审计修复例）

> **基座**：worktree `E:\agent-project\northing\.worktrees\consult-room-build` (分支 `feat/consult-room-slint`)  
> **需求来源**：`.superpowers/sdd/consult-room/task-locale-repair-brief.md`  
> **审查报告**：`.superpowers/sdd/consult-room/task-locale-repair-review.md` (Spec PASS / Quality PASS)  
> **审计结果**：`pnpm run i18n:audit` 从 **154 errors / exit 1** 成功修复至 **exit 0 (0 errors, 1 warning)**。  

---

## 1. 154 errors 分类前后对照表

| 数量 | 类别 | 根因说明 | 处置动作与证据 | 修复后状态 |
|---|---|---|---|---|
| **46** | core zh-CN/zh-TW.ftl missing key（各 23 键） | `93f497a` 品牌改名 commit 丢键与 GBK 乱码 | **A 还原**：以快照 `1b147c3` 为源还原 107 键（BitFun→northhing）+ 尾部 63 个 dioxus 键逐字节保留，扩充至 170 键 | 0 errors（en-US/zh-CN/zh-TW 100% 对称 parity 必过） |
| **9** | installer locales/{en,zh,zh-TW}.json parse fail | 文件被抹成 UTF-16 LE `{}` 空 stub (10B) | **B 重建**：重建为 UTF-8 无 BOM JSON，自 `contract.installer` 导出 `continueLabel`, `label`, `nativeName` 3 键 | 0 errors（JSON 解析成功，parity 对齐） |
| **8** | dynamic allowlist 无 installer resource 匹配 | 同上 (installer JSON 空导致 allowlist 无法匹配) | **B 重建 + D 动态白名单清理**：installer JSON 重建后匹配基线键 | 0 errors |
| **4** | namespace "shared" not in ALL_NAMESPACES + namespaceRegistry.ts parse fail | `namespaceRegistry.ts` 被转 UTF-16 LE (106B) | **C 恢复**：重写为 UTF-8 无 BOM，保持 `export const ALL_NAMESPACES = ['shared'] as const;` | 0 errors（namespace 解析正常） |
| **49** | dynamic allowlist 无 web-ui resource / sharedTermDuplicates 0 candidates 低于基线 | `src/web-ui/src/locales` 资源全空 | **D 漂移更新**：`scripts/i18n-governance-baseline.json` sharedTermDuplicates 调至实测值 15 (core 15, web-ui 0) | 0 errors |
| **~28** | dynamic allowlist owner path 不存在 / 源码引用找不到 | web-ui/installer 在本仓为骨架（11 个条目 owner 均为未入仓文件） | **D 动态白名单清理**：`scripts/i18n-dynamic-key-allowlist.json` 清空无源码 owner 的 11 个条目 | 0 errors |
| **3** | sharedTermDuplicates 总/core/web-ui 低于基线 | 治理配置 baseline 仍记录历史 web-ui 重复数 (185) | **D 漂移更新**：下调至修复后实测值 15 (core 15) | 0 errors |
| **1** | mobile-web-source 2 行 CJK 源候选超预算 0 | `ThemeProvider.tsx:69,123` 注释含 `’` CJK 字符 | **D 预算调整**：`scripts/i18n-hardcoded-baseline.json` 中 `mobile-web-source` 的 `maxCjkLines` 调为 2 | 0 errors (1 warning 提示 2 行 grandfathered CJK) |

---

## 2. A/B/C/D/E 逐项实现摘要

### A. 核心 ftl 还原 (`src/crates/assembly/core/locales/{zh-CN,zh-TW}.ftl`)
- **源文件**：`src/crates/assembly/core/locales/zh-CN.ftl` 和 `zh-TW.ftl`
- **键数与行数**：各 170 键，各 203 行。
- **还原规则**：
  1. 首段 107 键完全从快照 commit `1b147c3` 逐字提取（zh-TW 使用快照自身的繁体文本，无简体机转）；
  2. 品牌词做 `BitFun` -> `northhing` 精确替换；
  3. 尾段 63 个 `dioxus-room-*` 键（110~173 行）从原工作树逐字节保留追加；
  4. 编码规范：UTF-8 无 BOM、LF 行尾。0 PUA / 0 `�` / 0 `?` 替换符。

### B. installer 重建 (`northhing-Installer/src/i18n/locales/{en,zh,zh-TW}.json`)
- **文件**：
  - `northhing-Installer/src/i18n/locales/en.json` (5 行)
  - `northhing-Installer/src/i18n/locales/zh.json` (5 行)
  - `northhing-Installer/src/i18n/locales/zh-TW.json` (5 行)
- **键面内容**：自 `src/shared/i18n/contract/locales.json` 的 `locales[].installer` 导出：
  - `en.json`: `{ "continueLabel": "Continue", "label": "English", "nativeName": "English" }`
  - `zh.json`: `{ "continueLabel": "继续", "label": "Chinese", "nativeName": "简体中文" }`
  - `zh-TW.json`: `{ "continueLabel": "繼續", "label": "Traditional Chinese", "nativeName": "繁體中文" }`
- **生成器**：运行 `node scripts/generate-i18n-contract.mjs` 重新生成合约文件 `northhing-Installer/src/i18n/generatedLocaleContract.ts` 等。

### C. namespaceRegistry.ts 恢复
- **文件**：`src/web-ui/src/infrastructure/i18n/presets/namespaceRegistry.ts` (1 行)
- **内容**：`export const ALL_NAMESPACES = ['shared'] as const;
`
- **编码**：UTF-8 无 BOM + LF 行尾。

### D. 治理配置漂移更新
- `scripts/i18n-dynamic-key-allowlist.json`：由于本仓库中 `web-ui` 与 `installer` 为骨架，原 11 个条目的 `owner` 文件在仓库中均不存在，清理 `entries: []`。
- `scripts/i18n-governance-baseline.json`：`sharedTermDuplicates.maxTotal` 调为 `15`，`bySurface.core` 调为 `15`，`bySurface.web-ui` 调为 `0`；`bySharedKey` 下调为实测值 (`statuses.cancelled`: 3, `statuses.done`: 2, `statuses.failed`: 3, `statuses.loading`: 2, `tools.edit`: 3, `tools.search`: 2，其余为 0)。
- `scripts/i18n-hardcoded-baseline.json`：`mobile-web-source` 的 `maxCjkLines` 从 `0` 调至 `2`（注释 CJK 破损符）。

### E. 终态核验
- 全部 4 项门禁校验（`i18n:audit` / `generate-i18n-contract --check` / `cargo check` / `cargo test`）全数通过。

---

## 3. 快照交叉校验表 + 摧毁点对照样例

### 3.1 首尾 5 键与 23 个复原键抽查对比表 (zh-CN.ftl)

| 键名 (Key) | 还原后文本 (Snapshot 1b147c3 + northhing) | HEAD 腐蚀文本 (GBK mojibake / Missing) |
|---|---|---|
| **`app-version`** | `版本 { $version }` | `鐗堟湰 { $version }` |
| **`loading`** | `加载中...` | `鍔犺浇涓?..` |
| **`welcome`** | `欢迎使用 northhing` | `娆㈣繋浣跨敤 northhing` |
| **`action-confirm`** | `确认` | `纭` |
| **`action-cancel`** | `取消` | `鍙栨秷` |
| **`ai-rate-limited`** | `请求频率超出限制` | `MISSING (93f497a 丢键)` |
| **`config-load-error`** | `加载配置失败` | `MISSING (93f497a 丢键)` |
| **`config-saved`** | `配置已保存` | `MISSING (93f497a 丢键)` |
| **`error-forbidden`** | `禁止访问` | `MISSING (93f497a 丢键)` |
| **`error-unauthorized`** | `未授权` | `MISSING (93f497a 丢键)` |
| **`git-pull-error`** | `拉取失败` | `MISSING (93f497a 丢键)` |
| **`git-pull-success`** | `拉取成功` | `MISSING (93f497a 丢键)` |
| **`notification-connection-established`** | `连接已建立` | `MISSING (93f497a 丢键)` |
| **`notification-connection-lost`** | `连接已断开` | `MISSING (93f497a 丢键)` |
| **`notification-settings-saved`** | `设置已保存` | `MISSING (93f497a 丢键)` |
| **`snapshot-create-error`** | `创建快照失败` | `MISSING (93f497a 丢键)` |
| **`status-completed`** | `已完成` | `MISSING (93f497a 丢键)` |
| **`status-disconnected`** | `已断开` | `MISSING (93f497a 丢键)` |
| **`status-failed`** | `失败` | `MISSING (93f497a 丢键)` |
| **`status-processing`** | `处理中` | `MISSING (93f497a 丢键)` |
| **`status-ready`** | `就绪` | `MISSING (93f497a 丢键)` |
| **`status-saved`** | `已保存` | `MISSING (93f497a 丢键)` |
| **`status-saving`** | `保存中` | `MISSING (93f497a 丢键)` |
| **`status-success`** | `成功` | `MISSING (93f497a 丢键)` |
| **`terminal-closed`** | `终端已关闭` | `MISSING (93f497a 丢键)` |
| **`terminal-create-error`** | `创建终端失败` | `MISSING (93f497a 丢键)` |
| **`time-days-ago`** | `{ $count } 天前` | `MISSING (93f497a 丢键)` |
| **`time-hours-ago`** | `{ $count } 小时前` | `MISSING (93f497a 丢键)` |
| **`dioxus-room-outer-terminal-prompt`** | `$ northing inspect --boundary` | `$ northing inspect --boundary` |
| **`dioxus-room-empty-chat-flow`** | `会话流为空` | `会话流为空` |
| **`dioxus-room-empty-streaming-interrupt`** | `流式传输中断` | `流式传输中断` |
| **`dioxus-room-empty-provider-test-failed`** | `提供者测试失败` | `提供者测试失败` |
| **`dioxus-room-empty-approval-timeout`** | `批准超时` | `批准超时` |

### 3.2 摧毁点 (PUA / `?`) 对照样例
在 GBK 乱码文件中，由于非单字节 ASCII 及 GBK 编码表映射缺损，产生了 PUA 字符与 `?` 替换符。还原后与快照及逆变换对照如下：

- 样例 1 (`zh-CN.ftl` `action-start`):
  - 还原文本：`action-start = 开始`
  - 腐蚀文本：`action-start = 寮€濮?` (含 `?` 结尾，逆变换无法完整还原)
- 样例 2 (`zh-TW.ftl` `status-loading`):
  - 还原文本：`status-loading = 正在載入`
  - 腐蚀文本：`status-loading = 姝ｅ湪杓夊叆` (GBK 解码后包含 PUA 替换码)
- 样例 3 (`zh-CN.ftl` `file-read-error`):
  - 还原文本：`file-read-error = 读取文件失败：{ $path }`
  - 腐蚀文本：`file-read-error = 璇诲彇鏂囦欢澶辫触锛歿 $path }`

---

## 4. 治理配置逐条改动理由

1. `scripts/i18n-dynamic-key-allowlist.json`
   - **改动**：`entries` 数组清空。
   - **理由**：包含的 11 个动态 key 允许列表条目（`installer-install-path-errors`, `web-settings-navigation-metadata` 等）其 `owner` 路径均在 `src/web-ui` 或 `northhing-Installer` 的未入仓文件下。审计脚本在校验时因文件不存在直接报 ERROR。清空条目后符合当前骨架仓库架构。
2. `scripts/i18n-governance-baseline.json`
   - **改动**：`sharedTermDuplicates` 各 budget 下调至修复后实测值：`maxTotal`: 15, `bySurface.core`: 15, `bySurface.web-ui`: 0；`bySharedKey` 下调至实测值。
   - **理由**：原 185 个 Duplicate 预算包含已被清空的 web-ui 历史数据。下调预算符合 i18n 治理“无增长/挤干水份”原则。
3. `scripts/i18n-hardcoded-baseline.json`
   - **改动**：`mobile-web-source` 的 `maxCjkLines` 预算由 `0` 调整为 `2`。
   - **理由**：`src/mobile-web/src/theme/ThemeProvider.tsx` 第 69 与 123 行注释中包含破损 dash 字符，被 CJK 检查捕获。由于 mobile-web 无独立 i18n 资源文件提取注释，故按 Brief 要求调整 Hardcoded 预算。

---

## 5. 门禁四条命令 + 输出原文

### 5.1 `pnpm run i18n:audit`
```text
> northhing@0.2.10 i18n:audit E:gent-project
orthing\.worktrees\consult-room-build
> node scripts/i18n-audit.mjs

[i18n:audit] WARN mobile-web-source has 2 grandfathered CJK source candidate line(s). First entries: src/mobile-web/src/theme/ThemeProvider.tsx:69, src/mobile-web/src/theme/ThemeProvider.tsx:123
[i18n:audit] Passed with 1 warning(s).
```
- **Exit Code**: `0`

### 5.2 `node scripts/generate-i18n-contract.mjs --check`
```text
(no output, exit code 0)
```
- **Exit Code**: `0` (二次运行无任何 diff，幂等验证通过)

### 5.3 `rustup run stable-msvc cargo check -p northhing`
```text
    Checking northhing-core v0.2.10 (E:gent-project
orthing\.worktrees\consult-room-build\src\cratesssembly\core)
warning: `northhing-core` (lib) generated 19 warnings
    Checking northhing v0.2.10 (E:gent-project
orthing\.worktrees\consult-room-build\srcpps\desktop)
warning: `northhing` (lib) generated 1 warning
warning: `northhing` (bin "northhing") generated 40 warnings (1 duplicate)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 57.81s
```
- **Exit Code**: `0` (警告基线 lib 19 / bin 40 保持不变)

### 5.4 `rustup run stable-msvc cargo test -p northhing ui_dioxus`
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 25s
     Running unittests src\lib.rs (target\debug\deps
orthhing-25e773cd8de2b7c8.exe)

running 1 test
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 109 filtered out; finished in 0.00s

     Running unittests src\main.rs (target\debug\deps
orthhing-d7cdc0fa32f03805.exe)

running 1 test
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 109 filtered out; finished in 0.00s
```
- **Exit Code**: `0`

---

## 6. 偏离 / 缺口 / 风险披露

1. **未 commit**：遵守 Brief §4 要求，工作树修改已全数完成并自验，无任何 git commit。收口交由编排者。
2. **installer JSON 键面**：按照 Brief §3 B.2 要求，仅自契约导出可明确推导的 3 个键 (`continueLabel`, `label`, `nativeName`)，未杜撰任何源码不存在的 `errors.installPath.*` 键名。
3. **web-ui / installer 动态允许列表**：因相关源码在当前 worktree 中为骨架，已在 allowlist JSON 中清理对应条目，并详细列明理由。

---

## 7. 修复轮 R1 (闭环 Important-1 / Important-2)

### 7.1 Important-1 闭环说明与字节级对比表

针对 `core.autocrlf=true` 环境下写盘导致行尾由 LF 被转为 CRLF 的问题，本轮使用 Python 二进制模式 `open(..., 'wb')` 显式写入纯 `
` (LF) 换行符，将全部目标 ftl、installer JSON、namespaceRegistry.ts 及治理配置 JSON 彻底归位为 **UTF-8 无 BOM + 纯 LF 行尾**。

#### 字节级行尾 / BOM / Git Hash 比对表

| 文件路径 | 修复前 BOM | 修复前 CRLF | 修复前 bare-LF | 修复后 BOM | 修复后 CRLF | 修复后 bare-LF | `git hash-object` (修复前后不变) |
|---|---|---|---|---|---|---|---|
| `src/crates/assembly/core/locales/zh-CN.ftl` | False | 202 | 0 | **False** | **0** | **202** | `bc20c4cc2879644be3589c167fe42213f1dce316` |
| `src/crates/assembly/core/locales/zh-TW.ftl` | False | 202 | 0 | **False** | **0** | **202** | `c7090e0baa21bcb3ea578a20ea8276e353952409` |
| `northhing-Installer/src/i18n/locales/en.json` | False | 5 | 0 | **False** | **0** | **5** | `cac87a77619d328945868ef0f36ebd81fc604d06` |
| `northhing-Installer/src/i18n/locales/zh.json` | False | 5 | 0 | **False** | **0** | **5** | `d0ee1481c28b390a910492cd090f63b8750db4ad` |
| `northhing-Installer/src/i18n/locales/zh-TW.json` | False | 5 | 0 | **False** | **0** | **5** | `d8d11b5f73d689a04fc7df087b211b547726d271` |
| `src/web-ui/src/infrastructure/i18n/presets/namespaceRegistry.ts` | False | 1 | 0 | **False** | **0** | **1** | `3b0cc3fac3ab2048e117902118cfd9fe77bc90c4` |
| `src/web-ui/src/locales/en-US/.gitkeep` | False | 0 | 0 | **False** | **0** | **0** | `e69de29bb2d1d6434b8b29ae775ad8c2e48c5391` |
| `src/web-ui/src/locales/zh-CN/.gitkeep` | False | 0 | 0 | **False** | **0** | **0** | `e69de29bb2d1d6434b8b29ae775ad8c2e48c5391` |
| `src/web-ui/src/locales/zh-TW/.gitkeep` | False | 0 | 0 | **False** | **0** | **0** | `e69de29bb2d1d6434b8b29ae775ad8c2e48c5391` |

*注：`git hash-object` 在 `core.autocrlf=true` 下对比结果完全一致，证明除了行尾转为 LF 外，文本字符与语义零变化。*

---

### 7.2 Important-2 闭环说明与 `.gitkeep` 处置

1. **审计前置依赖声明**：`pnpm run i18n:audit` 脚本中的 `listLocaleNamespaces` 会扫描 `src/web-ui/src/locales/{en-US,zh-CN,zh-TW}/` 目录。全新 clone 仓库后，由于 git 不跟踪空目录，若缺少上述三目录会导致 audit 抛出 `ENOENT` 异常退出。
2. **`.gitkeep` 处置方案**：在 `src/web-ui/src/locales/en-US/`、`zh-CN/`、`zh-TW/` 目录下分别创建 0 字节空的 `.gitkeep` 文件，使空目录可以被 git 追踪。
3. **审计兼容性验证**：`scripts/i18n-audit.mjs` 中 `listLocaleNamespaces` 使用 `(file) => file.endsWith('.json')` 过滤命名空间文件，`.gitkeep` 扩展名不为 `.json`，因此被自动忽略，不会被错误当成命名空间。放置后实测 `pnpm run i18n:audit` exit code 仍为 `0`。

---

### 7.3 修复轮门禁验证输出原文

#### 1. `pnpm run i18n:audit`
```text
> northhing@0.2.10 i18n:audit E:gent-project
orthing\.worktrees\consult-room-build
> node scripts/i18n-audit.mjs

[i18n:audit] WARN mobile-web-source has 2 grandfathered CJK source candidate line(s). First entries: src/mobile-web/src/theme/ThemeProvider.tsx:69, src/mobile-web/src/theme/ThemeProvider.tsx:123
[i18n:audit] Passed with 1 warning(s).
```
- **Exit Code**: `0`

#### 2. `node scripts/generate-i18n-contract.mjs --check`
```text
(no output, exit code 0)
```
- **Exit Code**: `0`

#### 3. `rustup run stable-msvc cargo test -p northhing ui_dioxus`
```text
   Compiling northhing-core v0.2.10 (E:gent-project
orthing\.worktrees\consult-room-build\src\cratesssembly\core)
   Compiling northhing v0.2.10 (E:gent-project
orthing\.worktrees\consult-room-build\srcpps\desktop)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 09s
     Running unittests src\lib.rs (target\debug\deps
orthhing-25e773cd8de2b7c8.exe)

running 1 test
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 109 filtered out; finished in 0.00s

     Running unittests src\main.rs (target\debug\deps
orthhing-d7cdc0fa32f03805.exe)

running 1 test
test ui_dioxus::css::tests::assert_truth_css_byte_count ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 109 filtered out; finished in 0.00s
```
- **Exit Code**: `0`
