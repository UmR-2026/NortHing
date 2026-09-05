# W16-1 Brief Review — 派发前独立审查

- 审查对象：`.superpowers/sdd/w16-1-brief.md`（124 行，未跟踪工作树文件）
- 审查者：reviewer-53（brief review 环节；与后续代码 judge 不同角色，只找茬不放行）
- 日期：2026-09-05；BASE：`19349cd`（实测 = NortHing main HEAD；工作树除 brief/plan 两个未跟踪文件外干净）
- 方法：五判据逐条 + 两条自由检查；所有结论出自 brief/计划/拍板原文与 git/脚本实测（基线三命令亲跑、fixture SHA 与 diff 亲查），不采信 brief 自述

## 判定总表

| # | 判据 | 判定 |
|---|---|---|
| 1 | 可证伪性 | **FAIL** |
| 2 | 预设豁免 | PASS |
| 3 | allowlist 完整性 | PASS |
| 4 | 预判审查结论 | PASS |
| 5 | 判断点授权 | **FAIL** |
| 6 | 验证命令可执行性 | PASS（逐条实测） |
| 7 | brief 内部矛盾 | **FAIL** |

## 逐条判定与证据

### 1. 可证伪性 — FAIL

绝大多数要求精确可判定：policy JSON 逐字给出（brief:26-55）、退出码语义明确、selftest fixture 枚举、rot note 逐字（brief:92）、44/48 读数算术核验通过（scripts/ 实测 42 个顶层文件 + 2 新文件；checker 实测输出 `dir_entries:scripts=42/42` 证实顶层文件计数语义）。两处破例：

- 正向 fixture「本 brief 自身经 validate-brief → 绿」（brief:85）在字面规则下**不可满足**：brief:74 规定命中五个预判措辞短语即非零且「（无论上下文）」，而 grep 全文实证这五个短语的唯一出现处就是 brief:74 规格行自身。不存在任何能满足该验收标准的字面实现——比"模糊"更糟的过约束矛盾（详见 C-1）。
- B.1「statusWords/reviewVerdicts 与上述枚举一致」（brief:62）与 B.2「名单有而实际无 = 未兑现（warning 不失败）」（brief:68）无对应 fixture：selftest a–f（brief:78-84）不含「枚举不一致 policy → 红」与「allowlist 超集 → 绿+warning」两条分支，跳过实现这两条规则 selftest 仍全绿——存在"看起来做完了"糊弄面（详见 M-2）。

### 2. 预设豁免 — PASS

- brief 全文无未标注的豁免措辞。唯一规则放宽（`dir_entries:scripts` ceiling 42→48）携带用户拍板 + 日期 + 到期 + commit body 引拍板（brief:92、114），与 D-synthesis §9 拍板 #2 逐字吻合（该文件 L220 已核对：批准 42→48、到期 2026-10-15、首个额度消耗 = Phase -1 的 SSOT 策略文件与闸脚本——与本单内容精确一致）。
- brief:73 的豁免短语清单是闸脚本的扫描规格（功能需求），不是 brief 自身的豁免声明；brief:68「warning 不失败」是机器可判定的工具行为规格。二者均非放弃验证。
- 家规 7（ceiling 上调需用户拍板记录进 commit message）已满足：brief:114 commit body 明确引用拍板。

### 3. allowlist 完整性 — PASS

- 3 个允许文件（brief:14-16）覆盖全部功能要求的落点；selftest fixture 为临时文件（不入 diff）；禁区（brief:18）划定清晰且本任务无需触碰其中任何文件。
- fixture a 所需 7 文件名单（brief:88）与 `w15-1l-brief.md` L83-89 原名单及真实 git diff 双向吻合（见判据 6）。
- 擦边一处：report 文件（brief:110 要求写 `.superpowers/sdd/reports/w16-1-report.md`）不在 allowlist。仓库惯例支持现状（W15-1l 实现提交 `0ea30b3` 仅含 8 个代码文件，report/brief/ledger 由后续 docs 提交 `0844150` 收口），但 brief 未写明「report 不入本 commit」——按 brief:12「diff 越出 = judge Critical」字面，误 commit report 会触发假 Critical。记 M-3，不影响 PASS。

### 4. 预判审查结论 — PASS

- 无约束审查者的措辞。brief:12「diff 越出 = judge Critical」是特定机械条件的严重度定义（本仓 brief 惯例，w15-1l-brief.md L82 同款），不构成对 finding 的压制。
- brief:74 的短语清单是闸脚本扫描功能需求，按 brief review 判据 4 的定义明确豁免。

### 5. 判断点授权 — FAIL

- **短语扫描的文本归一化语义未授权**（反引号内的短语算不算「命中」？）：brief:106 的调整授权只覆盖「节标题匹配规则」，不覆盖短语扫描。实现者为让 brief:85 fixture 变绿只能自行发明豁免规则——闸的检测语义由实现者定义，正是本波要消灭的「判据被被审查者定义」病理（D-synthesis §0 病理层 1）。→ C-1
- **「节」判定粒度未授权到位**：policy 要求 6 节中，任务标识（brief:3，元信息列表行）、BASE（brief:6，列表行）、禁区（brief:18，段落行）都不是 `##` 标题；brief:106 授权的「大小写不敏感的节标题包含匹配」按字面（标题级）无法命中这三者，「本 brief 自身 → 绿」再次依赖实现者自行放宽匹配。→ I-1
- 次要未定点（不单独阻塞）：selftest 临时文件位置未指定（建议 `os.tmpdir()`，仓库先例 `verify-rot-budget.test.mjs:14`）；「同句」的句界定义未给（fixture 自构，影响有限）。

### 6. 验证命令可执行性 — PASS（BASE `19349cd` 逐条亲测）

- 三条现存命令实测全绿，与 brief:6 基线声明逐项吻合：`node scripts/verify-rot-budget.mjs` EXIT=0（`dir_entries:scripts=42/42`、`.superpowers/sdd=53/400`）；`node scripts/verify-rot-budget.test.mjs` 12/12 pass EXIT=0；`node scripts/check-repo-hygiene.mjs` passed EXIT=0。
- 三条新脚本命令（brief:98-100）参数形态自洽；`validate-brief` 目标路径存在。
- **fixture 地真核验**（自由检查项）：`git rev-parse --verify` 证实 `05bbd40` / `0ea30b3` / `19349cd` 三 SHA 均存在；`git diff --name-only 05bbd40..0ea30b3` 实测恰 8 个文件 = brief:88 的 7 文件（与 w15-1l-brief.md 原名单逐字一致）+ `src/apps/desktop/src/ui_dioxus/pages_archive.rs`。fixture a/b 描述与 git 历史完全吻合，replay 可行。
- 44/48 读数核验：scripts/ 目录实测 42 个文件（另有 3 个子目录不计入——checker 顶层文件计数语义已由 42/42 输出证实）+ 2 个新文件 = 44 ✓；`verify-rot-budget.test.mjs` 全部 fixture 使用临时目录自带 manifest、无硬编码 42，额度上调不会引爆旧测试 ✓。
- `package.json` 实测 `check:repo-hygiene` = `node scripts/check-repo-hygiene.mjs`，brief 直跑形式与计划 W16-1 卡（plan:41）的 pnpm 形式等价 ✓。
- `check-repo-hygiene.mjs` 对改动文本文件的扫描（本地绝对路径 / token / 私钥 / 敏感文件名）不构成风险，前提是新脚本内不写字面本机用户目录类绝对路径（selftest 临时路径走 `os.tmpdir()` 即安全，与 M-4 同一修法）。
- `.superpowers/sdd` 预算 53/400，本 brief/report/review 文件的加入无触顶风险。

### 7. brief 内部矛盾 — FAIL

- **矛盾 1（= C-1）**：brief:74 规则（五短语命中即非零、无论上下文）vs brief:85/100/106（本 brief 自身必须过 validate-brief 且绿）。grep 全文实证五短语唯一出现处为 brief:74 自身（均在反引号内）。
- **矛盾 2（= I-1）**：brief:106 的授权调整（节标题包含匹配）vs brief 实际结构（任务标识/BASE/禁区非标题形态）——按字面执行授权则正向 fixture 不可满足。
- **矛盾 3（= M-1）**：brief:4/117 声称「Global Constraints 逐字复制自计划」，实际为删节版：GC#2（brief:120）删去计划 GC#2（plan:22）的括注「（note 记拍板日期 + 到期 2026-10-15 + commit message 引拍板原文）」；GC#5（brief:123）删去计划的 message 格式规则；计划 GC#6（W16-4 专项，plan:26）未注明省略。实质要求已被 §C（brief:92 note 逐字含拍板日期+到期）与派发元信息（brief:114）完整覆盖，无行为损失——但「逐字复制」标签不实，在以约束保真为主题的波次里应改准确。

## Findings

- **[Critical] C-1 预判措辞扫描规则与「本 brief 自身 → 绿」fixture 自相矛盾，selftest 按字面不可实现** — brief:74 vs brief:85/100/106。grep 实证：`do not flag` / `不要 flag` / `不需 flag` / `at most Minor` / `至多 Minor` 五短语全文唯一出现处即 brief:74 规格行自身；「无论上下文」封死一切上下文豁免。实现者只剩两条路：私定豁免语义（如忽略行内代码——闸的检测规则由被审查方定义，本波核心病理复发）或 fixture 永红（任务卡死）。**修复（最小）**：B.3 增加一句归一化规则——「扫描判定忽略围栏代码块与行内代码（反引号内）的文本；该豁免面为已知取舍，Phase 0 可加严」。brief:73/74 全部短语均在反引号内 → 结构性自洽通过；真实 prose 豁免不受影响（对照 w15-1l-brief.md:73「401/空列表等数据层状态不算失败」无反引号，仍会被抓）。备选方案（改动更大，不推荐本轮）：短语表移入 workflow-policy.json 字段、brief 正文不再出现字面量——需同步修订 §A 钉死 JSON 与 validate-policy 规格。
- **[Important] I-1 「节」判定粒度未定义，brief:106 的授权不覆盖本 brief 实际结构** — brief:72（必含节）+ brief:106（授权仅「大小写不敏感的节标题包含匹配」）vs brief:3/6/18（任务标识/BASE 为元信息列表行、禁区为段落行，均非 `##` 标题）。按标题级字面执行，「本 brief 自身 → 绿」不可满足，实现者被迫自行放宽——第二次制造「实现者定义闸语义」。**修复（最小）**：在 B.3 或 brief:106 钉死「节判定 = 存在以 `## <term>` 开头的标题行，或以 `<term>` / `<term>：` 开头的任意行（含 `- ` 列表项）；匹配大小写不敏感」。或更彻底（推荐）：把本 brief 六节全部改为 `##` 标题形态，一处结构改动换取匹配规则最简，顺带为后续 brief 立模板标杆。
- **[Minor] M-1 「逐字复制自计划」标签不实**（判据 7 矛盾 3）— brief:4/117。修复：改为「摘编自计划（W16-4 专项省略）」或真正逐字复制。
- **[Minor] M-2 负向 fixture 缺两条分支**：枚举不一致的 policy → 红（覆盖 B.1 brief:62 的枚举一致性要求）；allowlist 超集 → 绿 + warning 不失败（覆盖 B.2 brief:68 的未兑现分支）。D-synthesis §6「机器校验不到的禁令不算禁令」原则下建议补齐；judge 代码级复核可兜底，不阻塞派发。
- **[Minor] M-3 report 提交归属未写明** — brief:110 要求写 report，但 brief:12 允许文件集不含它。修复：派发元信息加一句「report/brief 不入本 commit，由编排者 docs commit 收口」（惯例实证：`0ea30b3` 仅 8 个代码文件 vs `0844150` docs 收口），防止误 commit 触发假 Critical。
- **[Minor] M-4 selftest 临时文件位置未指定** — 修复：钉 `os.tmpdir()`（先例 verify-rot-budget.test.mjs:14），顺带规避 check-repo-hygiene 的本地绝对路径扫描误伤与 scripts/ 目录计数污染。
- **[Minor] M-5 来源行声称覆盖 -1.7，但 -1.7 的机器判据（`dir_entries:scripts` 规则带例外租约字段 /「新增必须退役旧脚本」机械化，D-synthesis §3 L98 + 拍板 #2 L220）不在本单功能要求内**，仅以 note「机械化待 Phase 0」（brief:92）口头顺延，且 Phase 0 表内亦无显式承接任务——无 owner 的顺延承诺正是 F-B-7「findings 蒸发」形态。与计划一致（plan:12 W16-1 卡同样只有额度半），非 brief 独有偏差。修复：来源行改为「-1.7（仅额度半，机械化待 Phase 0）」+ 在波次台账/handoff 登记顺延项，或把最小租约字段拉进本单。

## Cannot verify from diff

- 无。本环节审查对象是 brief 文本；其全部事实性声明（BASE 位置、基线预跑结果、fixture SHA 与文件集、拍板记录、44/48 算术）均已实测核对成立。

## 总体结论：NEEDS_REVISION

brief 骨架质量高（fixture 回放真实事故、拍板链完整、验证命令全部可执行、允许文件集与需求严格对齐），但存在一个会让 selftest 按字面不可实现的 Critical 矛盾和一个迫使实现者自定义闸语义的 Important 缺口——对一个「建可信闸」的任务，这两点必须在派发前修掉。

最小修改清单（按序，1+2 为阻塞项）：

1. [C-1] B.3 增加短语扫描归一化规则（推荐：忽略行内代码与围栏代码块内文本 + 取舍注记），消除 brief:74 vs brief:85/100/106 矛盾。
2. [I-1] 钉死「节」判定粒度并同步 brief:106 授权语（推荐：本 brief 六节改 `##` 标题，或行级「`<term>` 开头即算」规则）。
3. [M-1~M-5] 建议同轮顺手修（不单独阻塞）：GC 标签改准确；补 2 条负向 fixture；写明 report 不入本 commit；临时文件钉 os.tmpdir()；-1.7 半覆盖显式化 + 台账登记顺延。

修完 1、2 即可派发。
