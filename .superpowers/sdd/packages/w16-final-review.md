# W16 波次终审（final review）— 最小可信核 Phase -1

- 范围：NortHing `19349cd..353a20f`（deae1b7 闸脚本+SSOT / 559cd6f W16-1 收口 / e9833a6 家规 8 / cedc231 theme.rs / 353a20f W16-2/3/4 收口）；W16-2 代码在 agent-project 仓 `c827e02`+`63a9289`（跨仓核验）
- 终审者：reviewer-53；日期 2026-09-06
- 方法：不采信台账/报告自述；全部结论出自 diff、源码、git 历史与本人独立复跑
- 结论：**NEEDS_FIX** — Critical 0 / Important 3 / Minor 3（四单交付物本身全部合格；阻塞点在可信核自身的三处收口，见 §五）

## 0. 终审独立复跑清单（HEAD = 353a20f，本机亲跑）

| 命令 | 结果 |
|---|---|
| `node scripts/verify-task-gate.mjs --selftest` | 11/11 PASS，exit 0 |
| `node scripts/verify-rot-budget.mjs` | passed：scripts=44/48, allow_dead_code=104/109, let_underscore=370/388, sdd=57/400 |
| `pnpm run check:repo-hygiene` | passed（10 content files scanned），exit 0 |
| `cargo check -p northhing-cli`（stable-msvc） | exit 0，仅 1 条基线 warning（question/mod.rs:15 unused imports，与 brief 声明基线一致）——独立复核 W16-4 的 MSVC 编译声明，关闭其 review 的「MSVC 复现存疑」项 |
| `git grep BackgroundPanel|BackgroundElement|load_opencode_theme_json -- src/apps/cli/` | 零命中 |
| gh CLI 查 CI（repo UmR-2026/NortHing） | 见 I-3：本波 5 commit 未推送零 CI；main 最近 60 次 ci.yml 运行 0 成功 |

## 一、跨任务接缝走查

### 1.1 policy 字段名三方一致性 — 字段名一致；结论词枚举值分裂（→ I-1）

`workflow-policy.json` 的 `cannotVerifyPolicy` / `metaRatchetPaths` / `statusWords` / `reviewVerdicts` 四字段名，与 AGENTS.md 家规 8.3/8.4（L105/L106）、AGENTS-CN.md（L101/L102）引用、`verify-task-gate.mjs` `validatePolicy`（L100-135）读法**同名同型，三方一致**。

但结论词的**枚举值**不一致：文档 8.3 状态机写 `PASS / FAIL / CANNOT_VERIFY / BLOCKED`，policy `reviewVerdicts` 与脚本 `EXPECTED_REVIEW_VERDICTS`（verify-task-gate.mjs:14）为 `APPROVE / APPROVE_WITH_CONCERNS / CANNOT_VERIFY / BLOCKED / FAIL`。`PASS` 不在 policy 枚举，`APPROVE` 不在文档状态机。四份 review 实际用词已然分裂：W16-1 / W16-2（两轮）用 APPROVE 系，W16-3 / W16-4 用 PASS。SSOT 在自己的核心词表上不是单源。

### 1.2 metaRatchetPaths 完备性 — 存在看守者盲区（→ I-2）

现行清单 4 条（workflow-policy.json:20-25）：verify-task-gate.mjs / verify-rot-budget.mjs / workflow-policy.json / `.github/workflows/`。以下同为 gate 但**不在清单**：

- `scripts/check-repo-hygiene.mjs`（token / 私钥 / 本地路径扫描，独立 gate，CI 有对应 job）
- `scripts/check-core-boundaries.mjs` + 实现体 `scripts/core-boundaries/`（分层边界 gate——顶层薄壳不在清单等于实现体也不在）
- `scripts/check-github-config.mjs`
- `package.json`（gate 接线层：删掉一行 script 定义即可静默解除任何 gate，且不触发 ratchet）

削弱以上任一文件只需走普通审查车道，与 meta-ratchet「改看守者升最高车道」的设计目的正面冲突。跨仓的 `assemble-review-pkgs.ps1` 天然不在本仓 policy 覆盖面内（他仓文件，登记为已知边界即可；补偿控制：manifest 内嵌 scriptSha256，篡改可检测——本轮已实测 manifest 与脚本哈希一致）。

### 1.3 theme.rs 死变体删除后的引用面 — 无恙

- `git grep` 全 cli crate：`BackgroundPanel` / `BackgroundElement` / `load_opencode_theme_json` 零残留（W16-4 review 的全仓 rg 复核与本审一致）。
- `Theme::background_panel` / `background_element` 字段保留（theme.rs:24-25），live 调用 6 处：permission.rs:222/281/337、chat/render/messages.rs:267、question/render.rs:62/267——消费者直接用字段，不走枚举，删除变体无涟漪。
- `Theme::style` match 无 `_` 臂，删后 17 变体 = 17 臂，编译器强制穷尽；本审 cargo check exit 0 佐证。
- 净减数据点真实：989 → 979（`rg -c "^"` 本审复核），net −10 = 新增安全语义（SAFETY 4 行 + fcntl 错误处理 3 行）由死代码收割（8 行死函数 + 2 变体 + 2 match 臂 + 2 误标 allow + 陈旧注释）买单。「闸逼出带修复的净减」首个数据点成立。
- 附带验证：W16-4 的变体删除是 brief 停止条件（dead_code warning → NEEDS_CONTEXT）触发后编排者「选项 A」裁决的留痕执行，report/台账链条完整，非失控扩围。

### 1.4 rot-budget 口径 — 一致

- `scripts/` 实测 44 个顶层文件（子目录不计入），checker 读数 44/48，计划预期「额度消耗后 44/48」三方吻合，余量 4 供 Phase -1~2。
- rot-budget.json 全 range diff 仅 `dir_entries:scripts` 一块（42→48 + note 改写），其余 ceiling 零变更（GC2）。
- note 文件本体为干净 UTF-8、无替换符（本审 node 读 JSON 复核；终端曾显示的乱码是控制台编码，非文件损坏）。

## 二、Global Constraints 逐条（计划 GC 1-7）

| # | 约束 | 证据 | 判 |
|---|---|---|---|
| 1 | 纯 Node 标准库 + pwsh 7 兼容 | verify-task-gate.mjs 仅 node: 内置；assemble ps1 仅 pwsh 内置（两轮 review 实跑 + 本审计行抽查） | PASS |
| 2 | rot-budget 仅 scripts 42→48 + note 拍板日期/到期 + commit 引拍板 | diff 唯一变更块；note 含 2026-09-05 拍板 + 到期 2026-10-15；deae1b7 body 与 brief 钉死句逐字一致（对 D-synthesis §9.2 为忠实转述而非逐字引原文——句式系 brief 层拍板，两轮 judge 均核过，记注不计 finding） | PASS |
| 3 | 日志/脚本输出 English-only | 闸脚本输出全英文；theme.rs warn 英文；assemble 控制台消息 round-2 后纯英文（R1-M2 已修，本审 ps1:42-46 抽查证实） | PASS |
| 4 | 验证输出原文进 report（命令 + exit code） | 四份 report 均有原文块（含 W16-1 的 hygiene exit 1 原文、W16-2 §3.1-3.4 四段含 LASTEXITCODE、W16-3 三段、W16-4 三命令） | PASS |
| 5 | 逐文件点名 add + 前缀 + (W16-N) | 5+2 个 commit 文件集与 allowlist / 钉死 message 逐一吻合（本审逐一 `git show --name-only` 核对；字面 `git add` 命令不可由 diff 核，结果等价） | PASS |
| 6 | W16-4 专项：净行 ≤0 + SAFETY + cfg(unix) 声明 | 989→979（净 −10）；SAFETY 在位（theme.rs:163-166）；cfg(unix) 声明 + 本地 cross-check 尝试（openssl-sys 阻塞，brief 预告失败不阻塞）+ CI ubuntu 兜底标注 | PASS（**兜底网有效性未闭环 → I-3**） |
| 7 | report 结尾状态词 | 四份均 DONE（在 statusWords 枚举内） | PASS |

## 三、台账-vs-git 校准 — SHA 全对账；一处 Minor 计数偏差

| 台账行 | git 实况 | 判 |
|---|---|---|
| W16-1：19349cd..deae1b7，AWC 0C/0I/3M | deae1b7 = 3 文件（policy/gate/rot-budget），review 文件 AWC 0/0/3 | ✓ |
| W16-3：559cd6f..e9833a6，PASS 0C/0I/1M | e9833a6 = AGENTS.md + AGENTS-CN.md，review PASS 0/0/1 | ✓ |
| W16-4：e9833a6..cedc231，PASS 0C/0I/2M | cedc231 = theme.rs 单文件，review PASS 0/0/2（NEEDS_CONTEXT→选项 A 留痕） | ✓ |
| W16-2：agent-project c827e02 + 63a9289，r1 AWC 0C/1I/4M + r2 APPROVE 0C/0I/2M | 跨仓 git show 核对：c827e02 单文件 A +143、63a9289 单文件 M +31/−7；review 文件两轮结论一致 | ✓ |
| 台账 W16-2 行「Minor：5 个留待 final triage」 | review 实况：两轮共记 6 条，round-2 已闭环 4 条，开放仅 M-5/M-6 | **多计 3（→ M-1）** |

## 四、流程实验有效性（Phase -1 试点）— 三项机制证据全部为真

1. **brief review 首跑抓 1C/1I：真**。`w16-1-brief-review.md`（随 559cd6f 入库）为真实详尽工件：C-1（brief:74 五短语 vs brief:85「本 brief 自身→绿」在字面规则下不可满足，grep 实证短语唯一出现处即规格行自身）+ I-1（「节」判定粒度未授权）+ 5M，判 NEEDS_REVISION 并给出最小修单。落地物证闭环：实现含 `normalizeMarkdownLines` + 双条件 `hasSection`；selftest 补齐「枚举不一致 policy→红」「allowlist 超集→绿+warning」两条 fixture（即 brief review M-2 的要求）。机制有效。
2. **verify-attempt 实战：真**。W16-3 report 贴实跑原文（559cd6f..e9833a6 + 临时 allowlist，exit 0），judge 独立复跑一致；W16-1/W16-4 按计划过渡条款（「基线期先用人工对照」）用 `git show --name-only` 等价核对；W16-2 跨仓不适用。闸的首战成立。建议：W17 起 judge 一律机械跑闸（闸已就位，过渡条款不再需要）。
3. **theme.rs 净减数据点：真**。见 §1.3。同单还附带验证了第二条机制：brief 的停止条件（出现 dead_code warning → 停手报 NEEDS_CONTEXT）按设计触发，未发生「糊住编译器」式扩围——「预算逼出修复」与「停止线防糊弄」两个机制在同一单得到实证。

## 五、Findings

### Critical：0

（四单交付物双审 + 本审独立复跑全绿；无正确性 / 安全 / 数据丢失问题。）

### Important：3

**I-1 结论词表分裂：文档状态机 PASS vs policy 枚举 APPROVE 系**
证据：AGENTS.md:105 / AGENTS-CN.md:101（8.3「PASS / FAIL / CANNOT_VERIFY / BLOCKED」）vs workflow-policy.json:15 + verify-task-gate.mjs:14（APPROVE / APPROVE_WITH_CONCERNS / CANNOT_VERIFY / BLOCKED / FAIL）。两者都在本 range 内落地；四份 review 用词已分裂（PASS×2 / APPROVE 系×3）。SSOT 在核心词表上不是单源。
**来源是计划自身**（plan:38「reviewVerdicts（含 APPROVE_WITH_CONCERNS）」vs plan:56「审查结论状态机：PASS / FAIL / CANNOT_VERIFY / BLOCKED」）→ 按规矩**交用户拍板哪个为准**。
建议：policy 枚举胜出（gate 机械强制之 + APPROVE_WITH_CONCERNS 本就必须在枚举内，PASS 无机械落点）。修法 = AGENTS.md / AGENTS-CN.md 8.3 各一词改动（文档不在 metaRatchetPaths，普通车道）；若用户选 PASS，则改 policy + 脚本 EXPECTED_REVIEW_VERDICTS，走 meta-ratchet 车道。

**I-2 metaRatchetPaths 看守者盲区**
证据：workflow-policy.json:20-25；清单外 gate 见 §1.2（check-repo-hygiene.mjs / check-core-boundaries.mjs + core-boundaries/ / check-github-config.mjs / package.json 接线）。削弱这些文件不升审查车道，与本波「可信核」目的冲突。
修法（任一）：
a) metaRatchetPaths 增补上述路径——改 workflow-policy.json 本身即触发 meta-ratchet（双 judge + 用户拍板），这正是该机制的设计用法；
b) 用户显式拍板维持最小清单，并在台账登记 deferral 与触发条件。
跨仓 assemble-review-pkgs.ps1 不可覆盖（他仓文件）：登记为已知边界，补偿控制（manifest scriptSha256）已在且本轮实测有效。

**I-3 W16-4 的 cfg(unix) 兜底网是空的：CI 在 main 上 60 连败，且本波 5 commit 未推送、零 CI**
事实链（gh 实查，repo UmR-2026/NortHing）：
1. 本波 5 个 commit 全部未推送（origin/main = 19349cd），从未有任何 CI 跑过本波代码；
2. main 最后一次 CI 执行（421a15f，早于本波 BASE）中 ubuntu-latest 与 macos-15 的「Check compilation」步骤双双失败（repo hygiene job 同红；windows / tests / boundaries / rot 绿）；
3. ci.yml 最近 60 次运行 **0 成功**（40 failure + 4 cancelled，追溯至 2026-07-28）。
W16-4 brief 钉死的「unix 语义由 CI ubuntu 兜底」在兜底网从未被证明能绿的前提下只是一纸声明；本地 cross-check 已尝试且被 openssl-sys 阻塞（brief 预告失败不阻塞，合规）。
处置：本波推送时 CI 必跑 cedc231（.rs 触发 push）；**在 ubuntu 转绿之前，W16-4 的 GC6 闭环保持 OPEN**。pre-existing 的 ubuntu/macos 编译失败需要单独诊断任务（非本波 diff 造成——421a15f 早于 BASE 且本波 Rust 改动仅 theme.rs——但本波验证故事依赖它，必须先行或伴随）。本审已独立复跑 `cargo check -p northhing-cli` exit 0（windows 侧无风险）；unix 侧为唯一开放面。

### Minor：3

**M-1 台账 W16-2 Minor 计数偏差** — 台账 W16-2 行称「Minor：5 个留待 final triage」；review 文件实况为 6 条记录 / 4 条已闭环 / 2 条开放（多计 3）。修法：台账计数改 2，或注记口径（两轮累计 vs 开放）。
**M-2 首个 APPROVE_WITH_CONCERNS 未带显式 owner + deadline** — w16-1-review.md:238 仅模糊路由「编排者波末登记或随 Phase 0 收口」；家规 8.5 要求 owner + 截止。本次终审 triage 追认闭环（owner = 编排者，截止 = 本文件日期 2026-09-06）；后续 AWC 判决必须显式两字段。
**M-3 agent-project 工作树残留未清** — `.opencode/tools/shot-window.ps1` +7 行（2026-09-04 TEMP 钉扎修复）整个波次未提交未还原，违反编排者自己的取消/失败卫生规则（W16-2 两轮 review 均目击并注记「他任务遗留」）。修法：提交（修复内容合理：Add-Type 的 csc 在服务上下文 TEMP 下失败的修法成立）或显式归属后续任务；不得继续带残留派新单。

## 六、Minor triage 处置表（全量 12 条：W16-1×3 / W16-2×6 / W16-3×1 / W16-4×2）

| 来源 | # | 内容（证据位） | 处置 | 说明 |
|---|---|---|---|---|
| W16-1 | M1 | 归一化规则扩用到豁免短语扫描（gate:310-358；brief:74 仅对预判措辞钉死归一化） | accept-and-close | 行为方向与 brief-review C-1 修法一致（反引号内规范引用不应触发），selftest fixture 依赖该语义；Phase 0 短语表数据化入 policy 时把归一化规则一并写明 |
| W16-1 | M2 | 豁免授权检查扩到「同段」（gate:319-340；brief 钉「同行或同句」） | accept-and-close | 闸的对抗面是实现者 brief 措辞，编排者非对抗方；记录在案，Phase 0 扫描器数据化时收敛 |
| W16-1 | M3 | report hygiene 解释不完整（report:130-133 自指绝对路径） | accept-and-close | HEAD 实跑 hygiene 绿（本审复核），docs commit 已脱敏收口，moot |
| W16-2 R1 | M-1 | report 缺改回后正向重跑原文 | accept-and-close（已闭环） | round-2 §R3 以 report §3.4 输出 + judge 独立复跑数值吻合关闭 |
| W16-2 R1 | M-2 | 控制台错误消息内插中文标题（ps1:42/45/80/83） | 修一记一（已修） | 63a9289 改为 path 定位，judge 实跑复核纯英文；本审 ps1 抽查证实 |
| W16-2 R1 | M-3 | 空标题清洗正则硬编码 1/6（ps1:109） | 修一记一（已修） | 已通用化（ps1:133），judge 5 样本实跑不误删 |
| W16-2 R1 | M-4 | exit 1 在 EAP Stop 下非实际退出路径 | 修一记一（已修） | 统一 `$host.SetShouldExit(1)` + throw，实测 exit 1；本审 ps1 抽查证实 |
| W16-2 R2 | M-5 | 预检与写盘之间 TOCTOU 窗口（ps1:102-123 → 127-153） | accept-and-close | 人工单次运行工具、无并发写者、毫秒级窗口；升级路径已记录（临时文件 + Move-Item），触发条件 = 纳入自动化调度 |
| W16-2 R2 | M-6 | 清洗正则未命中时无告警（ps1:133） | defer-with-owner | owner = 编排者；下次触碰 assemble-review-pkgs.ps1 时加一行 Write-Warning（与首轮 I-1 放行条件同车道） |
| W16-3 | M1 | report 表格引文丢反引号 ×6（排版） | accept-and-close | 已收口任务的 report 为冻结工件不回改；后续 report 模板注意 |
| W16-4 | M1 | warn 缺 fd / errno 上下文（theme.rs:197） | defer-with-owner | owner = 下次 theme.rs 触碰者（979/989 观察队列必再触）；review 已给出现成代码 |
| W16-4 | M2 | SAFETY 注释高估 stdin.lock() 保障（theme.rs:165） | defer-with-owner | 同上，一行改写；operationally true，非紧急 |

计数：修一记一 3 / accept-and-close 6 / defer-with-owner 3。
口径说明：编排者按台账口径记 11 条（3/5/1/2）；review 文件实况 12 条（3/6/1/2），其中 4 条已在 W16-2 fixer 轮闭环。差 1 = 台账 W16-2 行多计（M-1）。

## 七、Cannot verify from diff

1. **CI ubuntu / macOS 对 cedc231 的编译结果** — commit 未推送，CI 从未运行（已升格为 I-3 并附处置；ubuntu 转绿前 W16-4 GC6 闭环保持 OPEN）。
2. **跨仓外部材料内容**（USERPROFILE 下 AGENTS.md 等） — 仅哈希 / 结构 / status 可验（两轮 review 已验 + 本审 manifest 抽查一致），逐字内容比对不在可达面。
3. **台账中 implementer / judge 模型身份陈述** — 流程元数据，工件不可证伪，不影响代码质量判定。

## 八、范围外改动

- NortHing range 内：**无**（5 个 commit 文件集逐一与 allowlist / 收口 docs 对账吻合）。
- 跨仓观察：agent-project 的 shot-window.ps1 未提交残留（见 M-3，非本波 commit，卫生项）。

## 九、结论与收口条件

**NEEDS_FIX** — Critical 0 / Important 3 / Minor 3。

四单交付物本身全部合格（双审 + 本审独立复跑全绿；接缝三查中字段名、theme.rs 引用面、rot 口径三项干净，词表一项分裂）。阻塞点不在任何单的代码，而在可信核自身的三处收口：

1. I-1 词表单源化 — 用户拍板（policy 枚举 vs PASS），拍板后分钟级修复；
2. I-2 ratchet 覆盖面 — 用户拍板（增补 vs 显式维持 + 登记），拍板后一行 JSON（走 meta-ratchet 车道）；
3. I-3 CI 兜底网 — 推送本波 + 一个 ubuntu/macos 编译失败诊断任务；ubuntu 绿之前 W16-4 GC6 闭环 OPEN。

三项闭环后，本波即可视为完整落地。**不要重开四单**——它们的交付物与审查链条完好。
