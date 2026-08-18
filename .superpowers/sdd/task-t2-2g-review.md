# Task Review T2-2g — remote 栈子批 C5：relay 双 crate 整删（含 relay-i18n 契约摘除）

**Verdict**: SPEC PASS / QUALITY PASS — 1 Important + 2 Minor findings（fixer 必跑 1 项；其余记 ledger 等终审 triage）

---

## A. 双判决总览

| 判决 | 结论 | 证据摘要 |
|---|---|---|
| Spec 合规 | PASS | 5/5 Constraints 验证通过；out-of-scope i18n 与 surface 零伤亡 |
| 代码质量 | PASS | relay 删除手术干净；附 1 Important（编码 / EOL 副作用）+ 2 Minor（死函数 / 文档残留） |

---

## B. Spec 合规逐项验证（5/5 Constraints）

### Constraint 1 — SSH / `src/apps/server` 零改动 ✅
- 命令：`git diff HEAD -- src/apps/server/` → 空 diff
- 详细核对：`src/apps/server/src/ai_relay.rs` 仍包含 `create_relay_router` 与 `northhing-server --ai-relay` 注释；`src/apps/server/README.md` 仍含 3 条 `relay-server` 相对链接（行 5/7/8/10）—— 受 Constraint 1 保护，零改动符合规范
- 旁注：`README.md` 现在指向已删除的 `../relay-server/*`，链接已悬空；属于下批清理项，本任务零违规

### Constraint 2 — mobile-web 与其 i18n 表面零改动 ✅
- 命令：`git diff HEAD -- src/mobile-web dev.cjs build-installer.cjs pnpm-workspace.yaml` → 全空 diff
- `src/shared/i18n/contract/locales.json` mobile-web 块完整保留：`surfaceDefaults.mobile-web: en-US`、surfaceOrders `["zh-CN","zh-TW","en-US"]`、`surfaces.mobile-web.resourceRoot: src/mobile-web/src/i18n`（行 11/21–25/42–45）
- `scripts/i18n-audit.mjs` mobile-web 审计逻辑（`auditMobileWebBoundary`、`auditMobileWebMessageParity`、`auditMobileWebPlaceholderParity`、`mobile-web-source` CJK spec）行 609/672/728/750/1938/1959/2079 等多点完整保留
- `scripts/generate-i18n-contract.mjs` mobile-web 块完整保留（`mobile-web` 输出路径行 15、`generateMobileWebLocaleContract` 函数与 `mobile-web` locale 排序行 291–292）

### Constraint 3 — i18n 仅删 relay 面 ✅
逐处核对手术点：

| 文件 | 摘除点 | 验证 |
|---|---|---|
| `src/shared/i18n/contract/locales.json` | `surfaces["relay-static-homepage"]` 块 | 行 51–54 旧 → 现在仅剩 `core`；节点 `Object.keys(surfaces) = [web-ui,mobile-web,installer,core]` |
| `scripts/i18n-audit.mjs` | relay 路径常量 / 4 个函数 / 调用点 | 35 行 diff 删除，仅触 relay 关键字；存活 surface 的 `audit*` 全部完整保留（行 546–847 / 1500–2126） |
| `scripts/generate-i18n-contract.mjs` | output 条目 + `RELAY_HOMEPAGE_SHARED_TERM_KEYS` + `generateRelayHomepageSharedTerms` 函数 | 输出文件数 6 → 5；`--check` 绿 |
| `scripts/i18n-contract.test.mjs` | `expectedGeneratedJsonFiles` 置空数组；删 `auditRelayStaticHomepageResources` 断言；整合测试改名为 `core reuses shared product terms`；删 stale relay 集成测试 | 行 18–22 / 360–363 / 817–855 |
| `scripts/i18n-governance-baseline.json` | `sharedTermDuplicates.bySurface` 与 `l10nQualityCandidates.bySurface` 的 `relay-static-homepage: 0` 键 | JSON 结构合法；表面度量唯一变化是删两键 |
| `scripts/i18n-hardcoded-baseline.json` | `budgets[]` 的 `relay-static-homepage-source` 项 | JSON 结构合法；budgets 现在 3 项（web-ui / mobile-web / installer） |
| `scripts/check-repo-hygiene.mjs` | 注释 `relay static assets` + ignore 正则 `/src\/apps\/relay-server\/static\/assets\//` | 行 11 + 81 删除 |

`auditSurfaceResourceRoots` 的 switch：现仅剩 web-ui / installer / core / mobile-web 四 surface 分支（行 595–613）；relay 分支彻底消失。
`auditHardcodedSourceBudgets` 的 specs：现仅剩 web-ui-source / mobile-web-source / installer-source 三项（行 2072–2088）。

### Constraint 4 — 仅动任务书清单内文件 ✅
- `git diff HEAD --name-only` 输出 42 个文件路径（与 `task-t2-2g-diff.md` "git diff --stat" 段的 42 文件计数一致）
- 已查 `memory/northhing.md` 与 `.opencode/model-capability-notes.md` 处于"working tree modified" 状态，但**与本任务无关**——均属 pre-existing 改动，diff stat 未列入本任务 42 文件清单；不在本任务责任范围
- `frontend-redesign-*`、`memory/`、`.opencode/`、`.superpowers/sdd/其它 task-*`、`.handoffs/`、`archive/`、`.worktrees/`、`docs/`、`frontend-redesign-*` 等保护区均空 diff
- brief §4 关于"不动 memory/、.opencode/、.superpowers/sdd/ 其它 task-*"硬规则 —— 本任务严格执行

### Constraint 5 — 验证门槛 ✅
- `cargo check --workspace` (MSVC stable-msvc) → 终结 "Finished `dev` profile... in 46.53s"，仅 warnings（19 + 1 + 5 条 pre-existing），零 errors
- `cargo check -p northhing` → 终结 "Finished `dev` profile... in 57.68s"，仅 warnings
- `node scripts/check-core-boundaries.mjs` → "Core boundary check passed."
- `node scripts/core-boundaries/self-test.mjs` → exit 0
- `node scripts/generate-i18n-contract.mjs` → "Wrote 5 generated i18n contract file(s)"（6 → 5，符预期）
- `node scripts/generate-i18n-contract.mjs --check` → exit 0
- `pnpm run i18n:audit` 残留失败 → 全源自 `src/web-ui/src/locales` ENOENT（v0.1.0 snapshot 缺席）；orchestrator 已验证编辑前后同源失败、非本任务所致；本任务执行残留扫描 `rg -ln "relay-core|relay_core|relay-server|relay_server|relay-static-homepage" src scripts Cargo.toml package.json .github` → 唯一命中 `src/apps/server/README.md`（受保护的独立 frozen surface）

---

## C. 代码质量 findings

### F1 — Important：`scripts/i18n-audit.mjs` 编码/EOL 副作用（out-of-scope change）

**位置**：
- `scripts/i18n-audit.mjs:250` `const zhTwSameTextTerminologySignals = ['了解'];`
- `scripts/i18n-audit.mjs:252–258` `const zhTwSameTextScriptSignals = new Set([...])` —— 60 个 Han 字符信号列表

**事实链**：
- HEAD 文件这两段为 mojibake 形式（Latin-1 误解的 UTF-8 字节，如 `'äºè§£'` 即 UTF-8 `了解` 的 Latin-1 表现），文件所有非空行带 `\r\r\n`（双 CR）行尾
- 当前文件这两段已规范化为正确 UTF-8（`'了解'`、`'这'`、`'个'` 等），全文件行尾规范化为 `\r\n` 单 CR
- `git diff -w` 视角下：273 行真正内容变更；其中 ~70 行是这处编码/EOL 副作用，非 brief 明示的"relay 删除"

**行为影响**：
- `getZhTwSameTextSignal` 中 `zhTwSameTextScriptSignals.has(character)` 此前永远 false（真实 zh-TW 字符串是 UTF-8，不会撞 mojibake bytes）；现在能正确匹配真实 zh-TW Han 字符，从 dead-code 转为有效审计信号
- 此改动与 surface 删除逻辑无关、与 relay 删除无关，属工具链串入的副产物

**为何 Important 而非 Minor**：
- 严格违反 brief Constraint 4："不顺手重构" 与 "只动任务书清单内文件内涵"
- 虽然 i18n engineering frozen（影响实证为 0），但代码 diff 与 review 期望偏离，将干扰 git blame 与 blame 责任划分
- 不修复会让 diff 长期掺杂一条非任务解释的语义变动

**修复方向**：
- 方案 a：**revert** 编码/EOL 副作用，恢复到与 HEAD 字节级一致；或
- 方案 b：**保留**新编码，但作为独立 commit 提交，commit message 明示"驱动工具规范化副作用，与 relay 删除无关"
- 推荐方案 a（最稳，符合"minimal diff"与"不顺手重构"）；方案 b 次优

**fix 派发必带覆盖**：
- `node scripts/i18n-audit.mjs --help` 或类似 dry-run 仅加载模块路径，确认 mojibake 改回后不影响 `getZhTwSameTextSignal` 调用行为
- `git diff -w scripts/i18n-audit.mjs | wc -l` 行数对比作为最小化证据

### F2 — Minor：`collectConfirmedUnusedKeys` 现在是空函数（死代码残留）

**位置**：`scripts/i18n-audit.mjs:1411–1412`
```js
function collectConfirmedUnusedKeys() {
}
```

**事实链**：
- 旧实现：`readRelayHomepageMessages()` + `collectRelayHomepageDataKeys()` 提取 baseline 未被 `data-i18n` 消费的 key，写入 `governanceReport.confirmedUnusedKeys`
- relay surface 删后，两数据源都为 `undefined`-safe no-op
- 调用点行 1502 `collectConfirmedUnusedKeys();` 仍存在
- `auditGovernanceCategoryBudget('confirmedUnusedKeys', { maxTotal: 0 })` 对 0 entries vs 0 maxTotal 通过（行 5–7 baseline）

**影响**：功能等价 no-op；仅是代码风格残留（空函数 + dead call）
- `pnpm run i18n:audit` 现状本就因 web-ui 缺席失败；此空函数本身不引入新失败
- i18n-contract.test.mjs 中对应 stale-relay test 已删除，无并发失败

**Minor 而非 Important**：
- 不破坏任何约束
- i18n engineering frozen 范围内可容忍
- 留待终审 triage 阶段清理（一次成型不需要分多批）

**修复方向（Minor 队列，统一交付，终审前 fixer 一并处理）**：
- 删除函数体与调用点
- 或保留函数体但加注释说明"residual after relay-static-homepage removal; confirmedUnusedKeys baseline remains 0"

### F3 — Minor：`src/apps/server/README.md` 残留 3 条 `relay-server` 链接（Frozen 表面文档悬空）

**位置**：`src/apps/server/README.md:5,7,8,10`

**事实**：
- 行 5 "If you are looking for **Remote Connect self-hosted relay deployment**, use:"
- 行 7 `- [Relay Server README](../relay-server/README.md)`
- 行 8 `- [deploy.sh](../relay-server/deploy.sh)`
- 行 10 `` `src/apps/server` and `src/apps/relay-server` are different components...``

**为何保留**：
- Constraint 1 明确：`src/apps/server` 是独立 frozen surface，本批零改动
- README 中 3 个相对路径链接现在指向已删除目录，渲染为坏链

**Minor 而非 Important**：
- README 是 server 表面自述，独立 frozen 受保护
- 链接悬空不破坏构建/测试，仅 reader friction
- 应在 server surface unfrozen 或专门 cleanup 批处理（不属于本任务范围）

**修复方向**：等 server 表面 unfrozen 或独立文档清理批；同时应反馈给 frozen server 的 owner 提议同步更新。

---

## D. Cannot-verify-from-diff 项逐条核实

| 项 | 验证手段 | 结论 |
|---|---|---|
| **F1** 编码/EOL 副作用 | `node` 直接读 OLD/NEW 字节级差异；diff 视图交叉对照 | 已确认（见上） |
| **F2** 空函数 + dead call | 直接读新文件行 1411–1412 / 1502；trace 到原 relay 数据源全部下线 | 已确认 |
| Cargo.lock 同步正确（无 orphan 依赖） | `rg -n "northhing-relay-core\|northhing-relay-server" Cargo.lock src crates` → 0 命中；`git diff HEAD -- Cargo.lock` 限定为两 package block 删除 | 已确认（仅删两 package 条目，无其他 stale ref） |
| `crate-layout.mjs` 同步 | `git diff HEAD --` 行 26–31 → `relay-core` 条目删除 | 已确认 |
| `Cargo.toml:154` 注释微调 | `git diff HEAD -- Cargo.toml` 行 149–153 → `installer/relay-server crates` → `installer crate` | 已确认（与 brief §3 :154 注释同步说明一致） |
| `locales.json` 删块后 JSON 合法 + 其它 surface 完整 | `node` 读取 `Object.keys(surfaces)` = `[web-ui,mobile-web,installer,core]`，3 个 locale 块完整 | 已确认 |
| `i18n-audit.mjs` relay 删除手术干净，存活 audit 逻辑零扰动 | 全函数列表 diff，行数 35 ± relay 关键字；存活 `auditMobileWeb*` / `auditInstaller*` / `auditCore*` / `auditKeyParity` / `auditGeneratedContract` 等完整保留 | 已确认 |
| `generate-i18n-contract.mjs` 删除后对存活 surface 输出不变 | `--check` exit 0 + 文件数 6 → 5（仅少 relay 那一文件） | 已确认 |
| `i18n-contract.test.mjs` relay 集成测试块清理 | diff 中删 `i18n audit fails stale relay static shared-term references` 块，重命名整合测试 | 已确认 |
| 两个 baseline JSON 的 relay 键 + JSON 合法 | `node -e` 读两个 JSON OK；仅各减 1 或 2 个键 | 已确认 |
| `check-repo-hygiene.mjs` relay ignore 精准 | diff 行 11 + 81 删除 | 已确认 |
| `surfaces.md` 精确摘除 relay 行，未误删 mobile-web / server | diff 行 19 与 51 删；server 与 mobile-web 完整保留 | 已确认 |
| `AGENTS.md` / `AGENTS-CN.md` relay 精确摘除 | diff 删除 `desktop, CLI, server, relay, mobile web` 段中 `, relay` 与 "frozen-experimental" `mobile-web / server / relay` 段中 `/ relay`；mobile-web / server / CLI 其它段保留 | 已确认 |
| 残留扫描仅 `src/apps/server/README.md` | `rg -ln` 实跑 → 唯一命中即 server README；其它历史 archive / docs / handoffs 均不在 brief 范围 | 已确认 |
| 其它 frozen/private 表面零改动 | `git diff HEAD -- src/apps/server src/mobile-web dev.cjs build-installer.cjs pnpm-workspace.yaml memory .opencode .superpowers/sdd docs` 均空 / 非任务清单 | 已确认 |
| `pnpm run i18n:audit` 与 `node scripts/i18n-contract.test.mjs` 失败非 relay 相关 | 未实跑（frozen i18n engineering），依赖 orchestrator 核实 + audit 错误码中无 `relay-static-homepage` 残留路径（已在 `i18n-audit.mjs` 文本层面核对） | 间接确认 |
| `Cargo.lock` line ending 是否会污染 lockfile | 实读文件首 50 字节 = `0a` (LF) 单字节行尾；`git diff --check` 报 7 文件 LF→CRLF 待规范化（autocrlf 行为，非内容错误），不会影响 `cargo check` 解析 | 已确认 |

---

## E. 派发复盘建议

- **fixer 必跑**：F1（编码/EOL 副作用处理）
- **fixer 报告必带**：
  - `git diff -w -- scripts/i18n-audit.mjs | wc -l` 对比（确保 < 200 行的真内容差异）
  - `node scripts/i18n-audit.mjs --report-json /tmp/i18n-audit.json 2>&1 | head -3` 或等价 module-load dry-run，证明修改后无解析错误
- **fixer 派发正文要点**：
  - F1 二选一（revert vs 独立 commit），由编排者裁定前先问
  - F2/F3 由编排者拍板是本 fixer 顺手清理还是放进终审 triage

---

## F. 裁定依据

- 与 brief Files / Constraints 段逐字比对，**无任何强制重做的 spec violation**
- Constraint 1–5 全清，relay 整删与 i18n 摘除手术精确，文档同步完整
- F1 / F2 是工具链副作用或 residue，未越本任务 spec 红线
- 编排者已核实的环境事实（web-ui ENOENT 失败与 relay 无关、`expectedGeneratedJsonFiles` 置空精确 = 原列表仅 relay 一项）—— 与本任务 diff 验证一致，可入账

---

## G. ledger 推进

通过条件：
- Constraint 1–5 全 ✅
- Verification 门槛（cargo / boundary / i18n contract）全 ✅
- F1 作为 Important 由 fixer 跑，expectedGeneratedJsonFiles 等价 + 编码不动状态保持
- F2 / F3 推入终审 triage 阶段，与本任务无关地分别清理

待 fixer 处理 F1 后，本任务可入 ledger。
