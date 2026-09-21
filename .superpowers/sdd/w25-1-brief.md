# W25-1 Brief — 授权记录机器可读（A-3：scripts 退役配额接入 authorization 通道，meta-ratchet 双 judge）

## 1. 来源与验收标准（逐字）

Plan `.superpowers/sdd/plan-2026-09-10-w23-w26-closure-and-decoupling.md` §1：
> "W25-1 授权记录机器可读（rot-budget.json schema + checker 复用 isAuthorizationLive；A-3）"

A-3〔Important〕（`E:\agent-project\.opencode\external-review\2026-09-09\independent-review-2026-09-09.md`，逐字）：
> "`--base` 模式当前对历史基线必然假红：退役配额与用户授权互相打架，机制在劝退自己的使用者"
> 最小修法（逐字）："配额检查读取 `dir_entries:scripts` 的授权语义（要么在 entry 加 `authorization` 并复用 `isAuthorizationLive`，要么把配额改成 `min(baseCount+usedQuota, ceiling)`）；顺带把 858 行写死的「expires 2026-10-15」文案改为读数据"

用户拍板（2026-09-09 防腐包，台账在案）："D2 特批机器可读（汇率缓办）"——本单即 D2 的机器可读通道落地（汇率不建）。

用户既有授权事实（数据源，逐字自 rot-budget.json:35 现状 note）："一次性额度：用户 2026-09-05 拍板 +6 供 Phase -1~2 整改用，到期 2026-10-15 未用部分回落需重确认；新增脚本仍须退役旧脚本（机械化待 Phase 0）"。

### 验收标准（机械可核对）

- AC1: `scripts/rot-budget.json` 的 `dir_entries:scripts` entry 增机器可读 `authorization` 字段：`{ "delta": 6, "expires": "2026-10-15", "reference": "user sign-off 2026-09-05" }`（字段名三件套钉死）；ceiling 48 不动；note 改为短指针（删重复语义，留一句说明指向 authorization 字段）。
- AC2: `scripts/verify-rot-budget.mjs` 退役配额检查（现 :845-861）改为：允许增量 = 存活授权 delta（复用既有 `isAuthorizationLive`，:406）；`tip > base + liveDelta` 才 violation；violation 文案中的到期日/额度全部读数据（消灭 :858 写死文案）；无授权或授权过期 → liveDelta=0 → 行为与现状一致（净增即红）。
- AC3: `scripts/verify-rot-budget.test.mjs` 增四例（夹具构造见 S3 钉死）：(a) 存活授权覆盖 delta 内增长 → 无配额 violation **且断言配额检查真实执行过**（如 `counts['dir_entries:scripts']` 已填充）；(b) 超 delta 增长 → violation（文案含具体数字）；(c) 过期授权 → violation（含 expired 语义）；(d) 无 authorization 字段 → violation 且文案不含 expires 片段。全套件绿。
- AC4: A-3 原假红场景复跑：`node scripts/verify-rot-budget.mjs --base df5c1ce` 输出的 violation 列表中**不再出现** `dir_entries:scripts` 的 `net increase prohibited`（整条消失；若该老基线触发其它历史 violation，属范围外，report 如实记录即可）。
- AC5: §6 验证命令全绿，输出原文进 report。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `77a508f8c9bafae564a7cd1dd11ca02c3a87fa72`（= origin/main，W24 + CI 修复全绿后）。

| 事实 | 证据 |
|---|---|
| `isAuthorizationLive(authorization, todayUtc)` 已存在（verify-rot-budget.mjs:406-411）：验 `authorization.expires` 形如 YYYY-MM-DD 且 ≥ 今日 | 源码 |
| ceiling 上调的授权通道已落地：:662-681（ceiling raise 无存活 authorization → violation，expired 有专属文案）——本单把同一语义延伸到退役配额检查 | 源码 |
| 退役配额检查现状（:845-861）：`tip > base` 即 violation，文案写死 "expires 2026-10-15"（:858） | 源码 + A-3 |
| A-3 假红实证：`--base df5c1ce` → "dir_entries:scripts: current 45 exceeds base count 44 → net increase prohibited"（45/48，额度 48 从未触顶但净增被禁） | A-3 原文 + 编排者复核 rot-budget.json:32-36 |
| BASE 基线全绿：`verify-rot-budget.mjs`（plain / `--base HEAD~1`）exit 0；`verify-rot-budget.test.mjs` 套件 exit 0；hygiene / boundary / github-config exit 0 | 编排者 BASE 实跑 |
| checker 行数余量：999/1050（countLines 口径，台账）——本单预计 +15~25 行；**硬约束 ≤1050**（god_file ceiling，W18-5a 用户拍板） | rot-budget.json:86-90 |
| test.mjs 的夹具模式：自建 temp dir + 合成 manifest 调 `verifyRotBudget({...})`（参考 F1.x 既有用例 :443-620） | test.mjs 源码 |
| 配额检查的 base 数据来源：:630-650 `git ls-tree <base> scripts/` 数 base 文件数 | 源码 |
| metaRatchetPaths 含 `scripts/verify-rot-budget.mjs` 与 `scripts/rot-budget.json`——本单双 judge 车道（机械触发） | workflow-policy.json:20-31 |

## 3. 复用侦察（强制）

- `isAuthorizationLive` 必须复用（计划逐字要求），禁止写第二个日期存活判断。
- authorization 三字段命名与 ceiling-raise 通道的 `authorization.expires` 保持同形（:667-669 既有消费）。
- test.mjs 新用例复用 F1.x 的 temp-dir/manifest 夹具模式，不建第二套 fixture 基建。
- report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

- S1: rot-budget.json `dir_entries:scripts` entry 增 `authorization` 对象（AC1 三字段）。`delta` 必须为 number、`expires` 必须为 YYYY-MM-DD 字符串、`reference` 字符串。ceiling=48 与其它 entry 零改动。
- S2: checker 配额检查改造（AC2 语义）。实现约束：从 `manifest['dir_entries:scripts']` 取 authorization；`isAuthorizationLive(entry?.authorization, todayUtc)` 为 true 时 `liveDelta = authorization.delta`（number 否则 0）；violation 条件 `tipScriptsCount > baseScriptsCount + liveDelta`；violation 文案形如 `dir_entries:scripts: current <tip> exceeds base count <base> + authorized delta <liveDelta> — net increase beyond live authorization (expires <expires>) prohibited`；无/过期授权时文案保持「net increase prohibited」语义但到期日读数据（授权对象存在但过期时写 `expired <expires>`；不存在时不写到期间）。
- S3: test.mjs 四新例（AC3），用例命名带 W25-1 标记。**夹具构造钉死（brief 复审 Important 修复）**：F1.x 的纯 temp-dir 夹具不进 git，配额分支（`if (base && baseScriptsCount !== null)`）在其中不可达、会造成用例 (a) 假绿——本单夹具 = F1.x temp-dir 模式的最小 git 扩展：temp dir 内 `git init -q` + 写 scripts/ 文件与 rot-budget.json + commit 得 base sha，再调 `verifyRotBudget({ projectRoot: tmpDir, base: <sha>, silent: true })`。声明：此为既有夹具模式的最小扩展，不属第二套 fixture 基建。用例 (a) 必须同时断言「无配额 violation」与「配额检查被执行」（counts 填充），堵死假绿通道。
- S4: A-3 复跑证据（AC4）：report 贴 `--base df5c1ce` 输出原文并标注 scripts 配额 violation 已消失（若输出仍 exit 1，逐条列出残余 violation 并标注「范围外（老基线历史项）」）。
- S5: 禁区：① 不调任何 ceiling/不删任何 metric；② 不动 verdict 计算/attestVerdictRubric/warnings 通道（W25-2 领地——指 verdict class/findings 的计算与输出；**violation 文案按 AC2 修改不属本禁区**）；③ 不动 workflow-policy.json（W25-2 领地）；④ 不动 check-github-config.mjs / gate-registry.json（W25-3 并行领地）；⑤ 不动 ci.yml。
- S6: 行数纪律：完工后 `verify-rot-budget.mjs` 的 checker countLines ≤1050（验证命令 7 自动覆盖）；若实现发现必然突破 → 停手 BLOCKED（ceiling 上调需用户拍板，brief 外决策）。

## 5. Global Constraints（逐字遵守）

- ceiling/配额数值零改动；authorization 字段内容 = 既有用户拍板的数据化（非新授权）。
- 禁整树 git 操作；只点名 add/commit。
- 测试必须真实执行，report 贴输出原文。
- 路径卫生（D-2）：进 report 的输出原文若含本机绝对路径段（盘符起头的 Windows 用户目录路径、Unix 用户目录路径等），一律改写为 `<HOME>`/`<RUSTUP>` 占位（hygiene 闸扫 committed report；示例也不许写字面路径形态）。
- 输出/日志英文，无 emoji。

## 6. 验证（命令 + 输出原文进 report）

1. `node scripts/verify-rot-budget.test.mjs` → exit 0（含三新例）
2. `node scripts/verify-rot-budget.mjs` → exit 0（配额检查仅在 --base 模式执行；plain 模式本就不评估它）
3. `node scripts/verify-rot-budget.mjs --base HEAD~1` → exit 0
4. `node scripts/verify-rot-budget.mjs --base df5c1ce` → 输出原文进 report；断言：violation 列表无 `dir_entries:scripts` 净增条目（AC4）
5. `node scripts/check-repo-hygiene.mjs` → exit 0
6. 红态探针（在本单 commit 之后执行）：临时把 rot-budget.json 的 `authorization.expires` 改成 `2020-01-01` → `node scripts/verify-rot-budget.mjs --base HEAD~1`…（注：本单 diff 未增删 scripts 文件，净增为零，过期授权下也绿——所以红态实证用 test.mjs 用例 (c) 承担，这里只验证 checker 不崩）→ `git checkout -- scripts/rot-budget.json` 还原
7. `node scripts/verify-rot-budget.mjs`（完工态，god_file 行数检查含 checker 自身 ≤1050）→ exit 0（输出含行数摘要，贴原文）
8. `node scripts/verify-task-gate.mjs verify-attempt --base 77a508f8c9bafae564a7cd1dd11ca02c3a87fa72 --tip <本单最后一个 commit sha> --allowlist .superpowers/sdd/w25-1-allowlist.txt` → exit 0（先自建 allowlist 含其自身；**若窗口内出现 W25-3 文件（check-github-config.mjs 等），停手报编排者复跑，不得私扩 allowlist**）

## 7. 报告

路径 `.superpowers/sdd/w25-1-report.md`。章节：改动摘要 / 复用侦察 / 验证（命令+输出原文）/ 疑虑 / 状态（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）。

## 8. 派发元信息

- BASE: `77a508f8c9bafae564a7cd1dd11ca02c3a87fa72`
- 允许文件集（diff 越出即 judge Critical）：
  - `scripts/rot-budget.json`
  - `scripts/verify-rot-budget.mjs`
  - `scripts/verify-rot-budget.test.mjs`
  - `.superpowers/sdd/w25-1-brief.md` / `w25-1-report.md` / `w25-1-allowlist.txt`
- 禁区：§4-S5 所列 + 其它未列出文件。
- commit 规则：点名 git add；message 前缀 `feat(rot-budget): W25-1`，body 注「用户 2026-09-09 拍板 D2/A-3 签名；authorization 数据 = 2026-09-05 拍板存量」；允许多 commit。
- 并行声明：W25-3 同波并行（check-github-config.mjs），文件集不相交；W25-2 在本单完成后串行启动（同触 checker）。cargo 本单不需要运行。
