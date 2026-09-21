# W25-2 Brief — verdict 三档一期（D4 信号卫生，meta-ratchet 双 judge）

## 1. 来源与验收标准（逐字）

Plan `.superpowers/sdd/plan-2026-09-10-w23-w26-closure-and-decoupling.md` §1：
> "W25-2 verdict 三档一期（violations 计数口径，零余量警告转 advisory；workflow-policy.json rubric SSOT + selftest 同步）"

防腐建议 4（用户 2026-09-09 拍板 D4「红灯分级：一期现在，速率档缓办」，`anti-rot-recommendations-2026-09-09.md`）：
> "信号卫生：verdict 三档化——clean / at-limit（零余量但稳定，预期稳态）/ degrading（逼近速率超历史漂移率才亮灯）。恒红消失后，红才重新值钱。"

速率档（degrading 的历史漂移率判定）**缓办在案，本单不建**——一期只做计数口径 + 通道重分类。

存量触发项：unix_epoch_inline 69/69 顶格（W23 起每轮恒红噪音源），本单一期落地后应归入 at-limit。

### 验收标准（机械可核对）

- AC1: `scripts/workflow-policy.json` 的 `rotVerdictRubric` 改为恰三键（键名与文案逐字钉死）：
  - `"clean": "0 violations, 0 warnings, 0 advisories"`
  - `"at-limit": "0 violations; warnings/advisories present (bounded, stable)"`
  - `"degrading": ">=1 violation OR any unbounded"`
- AC2: `scripts/verify-rot-budget.mjs`：零余量条目（现 :863-885 块）从 `warnings` 改入新 `advisories` 通道；`verifyRotBudget` 返回对象顶层新增 `advisories` 数组（与 `warnings` 并列，测试断言习惯用顶层字段）；verdict 映射钉死：`clean` ⟺ violations=0 ∧ warnings=0 ∧ advisories=0 ∧ ¬unbounded；`at-limit` ⟺ violations=0 ∧ ¬unbounded ∧ (warnings+advisories)>0；`degrading` ⟺ violations≥1 ∨ unbounded。退出码不变（violations>0 → exit 1；warnings/advisories 不影响退出码）。
- AC3: `attestVerdictRubric`（checker :360-380 区）的期望键集与文案同步更新（含 "must be an object with exactly 3 keys" 报错文案里的旧键名）。
- AC4: `scripts/verify-rot-budget.test.mjs` 同步：rubric 缺失/键集/文案错配用例按新三键改写；verdict 类断言用例改写（clean/at-limit/degrading，含 F1.9 输出格式断言——AC7 改输出文案后旧正则必失败，必须同步）；新增 ≥1 例「仅 advisory（零余量）→ verdict at-limit 且 exit 0」。`scripts/fixtures/rot-budget/selftest-cases.test.mjs` 若引旧类名**或含零余量/warnings 通道断言**一并同步（自查命令：`rg 'rotting|healthy|stable|zero headroom|warnings'` 两测试文件；已知命中：selftest-cases.test.mjs :547 的 zero-headroom positive 例与 :892 的 lease 抑制例——前者断言必随通道迁移改 advisories，后者应加 `!res.advisories.some(...)` 防空洞）。
- AC5: 完工实跑 `node scripts/verify-rot-budget.mjs`：verdict = `at-limit`（unix_epoch_inline 69/69 等零余量项在 advisory 通道），exit 0；输出原文进 report。
- AC6: checker countLines ≤1050（现 998，余量 52）；若实现必然突破 → 停手 BLOCKED 上报（ceiling 上调需用户拍板）。
- AC7: 输出文案：passed/failed 行打印 verdict class + 各通道计数（violations/warnings/advisories）；英文无 emoji。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `e57c6387de88110789ae600d90a4853da05aa90a`（= origin/main + W25-1/W25-3 已落）。

| 事实 | 证据 |
|---|---|
| `attestVerdictRubric`（checker :360-380）：强制 policy.rubcic 恰 3 键 healthy/stable/rotting 且文案与 checker 内置期望逐字一致——本单两侧同步换词 | 源码 |
| verdict 计算现状 :888-889：`findings = violations+warnings`，class = healthy(0)/stable(1-2)/rotting(≥3 或 unbounded) | 源码 |
| warnings 三来源：:749/:752（dead god-file 注册宽限/缺 deadSince）、:759（悬空 lease）、:882（零余量）——**只有零余量块（:863-885）移入 advisories**，其余两条留 warnings（行号以现场为准，语义为纲） | 源码 |
| verdict 对象字段（:890-...）含 class/findings/unbounded；下游消费方：test.mjs F1.x 断言 + main 输出行（:913/:920）；rg 全仓无第三消费方 | 源码 + rg |
| `rotVerdictRubric` 仓内引用仅 workflow-policy.json + checker + test.mjs 三处；`rotting` 等旧类名的其它命中全是 docs/archive 与 docs/reviews 的历史存档（冻结记录，不改） | 编排者 rg 全仓（排 node_modules/.git/target/.superpowers） |
| 基线全绿：test.mjs 43/43、--selftest 53/53、plain + --base HEAD~1 exit 0（W25-1 修复轮后 m3 增量重审实跑确认，同快照证据复用有效） | W25-1 修复轮记录 |
| unix_epoch_inline 69/69 + 3 个 god_file 零余量（css.rs 790/790、selectors.rs 827/827、lsp/manager.rs 836/836）+ docs/design 1/1 = 现状零余量条目集共 **5 条**（完工后应全部出现在 advisory 计数里） | BASE 实跑输出 |
| checker 998/1050（W25-1 后实测） | W25-1 judge 实测 |
| metaRatchetPaths 含 verify-rot-budget.mjs 与 workflow-policy.json → 双 judge 车道 | workflow-policy.json:20-31 |
| 并行领地：W25-4 同波并行（desktop Cargo.toml/Cargo.lock/work.rs），与本单文件集不相交 | 编排者矩阵 |

## 3. 复用侦察（强制）

- advisories 通道复用既有 warnings 的收集/打印模式（数组 + 循环 push），不建第二套输出框架。
- attestVerdictRubric 的期望-对差模式原样复用（只换期望内容）。
- report 必须有「复用侦察」一节。无此节 = 未完成。

## 4. Spec（必须全部满足）

- S1: policy rubric 三键三文案按 AC1 逐字落地。
- S2: checker 按 AC2 落地：advisories 数组（main 函数内与 warnings 并列声明）；零余量块 push 改 advisories；verdict 映射按钉死真值表；verdict 对象携带 `class` + `violations`/`warnings`/`advisories`/`unbounded` 计数字段（`findings` 字段去留由实现者定，但 test.mjs 断言必须同步一致，且 rg 确认无第三消费方——在 report 里给 rg 证据）。
- S3: attestVerdictRubric 期望键集 = {clean, at-limit, degrading}，期望文案 = AC1 三串逐字；报错文案同步。
- S4: test.mjs + selftest-cases.test.mjs 同步（AC4）；新用例命名带 W25-2 标记。
- S5: 禁区：① 配额检查逻辑、authorization 字段、W25-1 落地的四配额用例（test.mjs 约 :1082-1219 区）——逻辑与语义一行都不动（本单只在 test.mjs 里改 verdict/rubric 类用例）；② check-github-config.mjs / gate-registry.json；③ ci.yml；④ verify-task-gate.mjs；⑤ desktop 侧文件（W25-4 并行领地：src/apps/desktop/Cargo.toml、Cargo.lock、ui_dioxus/windows/work.rs）。
- S6: 行数纪律 ≤1050（AC6）。
- S7: 不建速率档/历史漂移统计（D4 缓办项，越出 = SPEC FAIL）。

## 5. Global Constraints（逐字遵守）

- ceiling/配额/authorization 数值零改动。
- 禁整树 git 操作；只点名 add/commit。
- 测试必须真实执行，report 贴输出原文。
- 路径卫生（D-2）：report 输出原文的本机绝对路径段一律 `<HOME>`/`<RUSTUP>` 占位；示例也不许写字面路径形态。
- 输出/日志英文，无 emoji。

## 6. 验证（命令 + 输出原文进 report）

BASE 全 sha = `e57c6387de88110789ae600d90a4853da05aa90a`（下文 `<BASE>`）。

1. `node scripts/verify-rot-budget.test.mjs` → exit 0（含新用例）
2. `node scripts/verify-rot-budget.mjs` → exit 0，输出含 `verdict: at-limit`（AC5）
3. `node scripts/verify-rot-budget.mjs --base <BASE>` → exit 0
4. `node scripts/verify-rot-budget.mjs --selftest` → exit 0（53 例在新文案下全绿；若红了说明 AC4 漏同步，回去补）
5. `node scripts/check-repo-hygiene.mjs` → exit 0
6. `node scripts/verify-task-gate.mjs validate-policy` → exit 0（policy 改动后自检）
7. 红态探针（本单 commit 后执行）：临时把 policy rubric 的 `at-limit` 键改名 → `node scripts/verify-rot-budget.mjs` exit 1（attestation fail-closed）→ `git checkout -- scripts/workflow-policy.json` 还原。输出原文进 report。
8. `node scripts/verify-task-gate.mjs verify-attempt --base <BASE> --tip <本单最后 commit sha> --allowlist .superpowers/sdd/w25-2-allowlist.txt` → exit 0（先自建 allowlist 含其自身；**窗口内若出现 W25-4 文件（Cargo.toml/Cargo.lock/work.rs），停手报编排者复跑，不得私扩 allowlist**）

## 7. 报告

路径 `.superpowers/sdd/w25-2-report.md`。章节：改动摘要 / 复用侦察 / 验证（命令+输出原文）/ 疑虑 / 状态（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）。

## 8. 派发元信息

- BASE: `e57c6387de88110789ae600d90a4853da05aa90a`
- 允许文件集（diff 越出即 judge Critical）：
  - `scripts/verify-rot-budget.mjs`
  - `scripts/verify-rot-budget.test.mjs`
  - `scripts/fixtures/rot-budget/selftest-cases.test.mjs`（仅在有旧类名或零余量/warnings 通道语义断言需同步时才动，并在 report 说明）
  - `scripts/workflow-policy.json`
  - `.superpowers/sdd/w25-2-brief.md` / `w25-2-report.md` / `w25-2-allowlist.txt`
- 禁区：§4-S5 所列 + 其它未列出文件。
- commit 规则：点名 git add；message 前缀 `feat(rot-budget): W25-2`，body 注「用户 2026-09-09 拍板 D4 一期签名」；允许多 commit。
- 并行声明：W25-4 同波并行（desktop 侧），文件集不相交。cargo 本单不需要运行。
