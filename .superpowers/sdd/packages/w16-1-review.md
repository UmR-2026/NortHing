# W16-1 审查：工作流闸脚本 + SSOT 策略文件

- 审查对象：commit `deae1b7`（3 文件：workflow-policy.json 新增 28 行、verify-task-gate.mjs 新增 701 行、rot-budget.json 改 4 行）
- 审查立场：独立验收，找茬优先
- 证据原则：diff + 实跑 + 文件原文字节比对；brief 自述不采信

---

## 一、SPEC 判据逐条核对

### A. `scripts/workflow-policy.json`（SSOT 雏形）

| 项 | brief 钉死 | 实现 | 判 |
|---|---|---|---|
| `version` | `1`（隐含正整数） | `1` | PASS |
| `judgeChecklist` 9 项顺序 | spec-by-acceptance-criteria → evidence-spot-check | 顺序一致 | PASS |
| `statusWords` | `[DONE, DONE_WITH_CONCERNS, NEEDS_CONTEXT, BLOCKED]` | 一致 | PASS |
| `reviewVerdicts` | `[APPROVE, APPROVE_WITH_CONCERNS, CANNOT_VERIFY, BLOCKED, FAIL]` | 一致 | PASS |
| `cannotVerifyPolicy` | 含 `blocking`/`auxiliary` 两键 | 含 | PASS |
| `metaRatchetPaths` | 4 条且顺序一致 | 含 4 条且顺序一致 | PASS |
| `briefRequiredSections` | 6 项 | 一致 | PASS |
| `reportRequiredSections` | 3 项 | 一致 | PASS |

字段顺序、key 拼写、引号风格与 brief §A 逐字吻合（`workflow-policy.json:1-28`）。

### B. `scripts/verify-task-gate.mjs`

#### B.1 `validate-policy`（brief §B.1）

- 必需字段存在且类型正确：`verify-task-gate.mjs:88-151`（version / judgeChecklist / statusWords / reviewVerdicts / cannotVerifyPolicy / metaRatchetPaths / briefRequiredSections / reportRequiredSections 八字段全检）
- `judgeChecklist` 为非空字符串数组：`verify-task-gate.mjs:92-98`（`Array.isArray` + length > 0 + every 非空字符串）
- `statusWords` / `reviewVerdicts` 强枚举精确比对：`verify-task-gate.mjs:100-118`（长度 + 逐位 `===`）
- `cannotVerifyPolicy` 含 `blocking`/`auxiliary` 两键：`verify-task-gate.mjs:120-128`
- `metaRatchetPaths` 为字符串数组：`verify-task-gate.mjs:130-135`
- 错误逐条列出：`errors: []` 累积后 `console.error('  - ${err}')` 逐条打

PASS。

#### B.2 `verify-attempt`（brief §B.2）

- `git rev-parse --verify` 校验两个 SHA：`verify-task-gate.mjs:175-204`（base 与 tip 各一次 try/catch）
- `git diff --name-only <base>..<tip>`：`verify-task-gate.mjs:230-244`
- allowlist 解析：`#` 注释 + 空行忽略 + 路径正斜杠归一化 + 去 `./` 前缀：`verify-task-gate.mjs:222-228`
- 双向比较：越界 = `actual \ allowlist`（错误），未兑现 = `allowlist \ actual`（警告，不失败）：`verify-task-gate.mjs:253-260`
- 越界非零退出，错误逐行列出：`verify-task-gate.mjs:262-266` + `main:655-668`

**关键路径独立验证**：
- 实跑 `git diff --name-only 05bbd40..0ea30b3` → 8 文件（含 `pages_archive.rs`）✓
- 实跑 `git rev-parse --verify 05bbd40^0 0ea30b3^0 19349cd^0` → 三 SHA 全有效 ✓

PASS。

#### B.3 `validate-brief`（brief §B.3）

- 必含节取自 policy `briefRequiredSections`，缺节非零：`verify-task-gate.mjs:285-308`
- 豁免短语扫描：`verify-task-gate.mjs:312-349`（三短语 `不算失败` / `不算越界` / `无需验证` + 授权 `用户拍板`/`拍板`）
- 预判审查措辞扫描：`verify-task-gate.mjs:351-358`（5 模式全覆盖：`do not flag` / `(?:不要|不需) flag` / `at most Minor` / `至多 Minor`）
- **归一化规则**：剔除围栏代码块 + 行内反引号：`verify-task-gate.mjs:47-66`（先 inFence toggle 处理 ``` 包围段，再 ` `[^`]*` ` 替换行内反引号）
- `续单` 含 BASE 行 + `允许文件集` 节：`verify-task-gate.mjs:360-367`

**钉死节判定规则**（brief §B.4 末段）：
- 条件 ①：`^##\s+/i` + `.toLowerCase().includes(sLower)` —— `verify-task-gate.mjs:36-38`
- 条件 ②：`^(?:-\s+)?${escapeRegex(sectionName)}(?:[:：\s]|$)/i` —— `verify-task-gate.mjs:31, 40-42`

**独立边界验证**（自写 8 用例全过）：
- `任务标识ABC`（无分隔符）→ 正确 FAIL（节判定排除）✓
- `任务标识：W16-X`（全角冒号）→ PASS ✓
- `续单` 无 BASE → FAIL ✓
- `续单` 在 fence 内 → PASS（不被抓）✓
- `不算失败（用户拍板 noted）` 同句授权 → PASS ✓
- `不算失败 no auth` → FAIL ✓
- `do not flag` 行内反引号 → PASS ✓
- `do not flag` 在 prose → FAIL ✓

PASS（带 2 个 Minor 见下）。

#### B.4 `--selftest`（brief §B.4）

11 fixture 完整覆盖：
- 负向 a-g：`verify-task-gate.mjs:390-509`（W15-1l replay / 错 SHA / 缺节 brief / 未标注豁免 / 预判措辞 / 坏 policy / 枚举不一致）
- 正向 1-4：`verify-task-gate.mjs:511-571`（8 文件完整名单 / +1 未兑现 / 本 brief / 本 policy）
- 临时文件全部走 `os.tmpdir()`：`verify-task-gate.mjs:377`（`fs.mkdtempSync(path.join(os.tmpdir(), 'task-gate-selftest-'))`）
- W15-1l 7 文件名单全路径写入 fixture a：`verify-task-gate.mjs:392-400`（与 brief §B.4 钉死文件集逐字一致）

实跑 `node scripts/verify-task-gate.mjs --selftest` → 11/11 PASS, exit 0 ✓

PASS。

### C. `scripts/rot-budget.json`

仅 `dir_entries:scripts` 块变更（`diff --stat` 与 git diff 双验）：
- ceiling: 42 → 48
- note: 改写为 brief §C 钉死原文 `一次性额度：用户 2026-09-05 拍板 +6 供 Phase -1~2 整改用，到期 2026-10-15 未用部分回落需重确认；新增脚本仍须退役旧脚本（机械化待 Phase 0）`

其它 7 个条目（unwrap_production / expect_production / let_underscore / unix_epoch_inline / allow_dead_code / dir_entries:docs/design / dir_entries:.superpowers/sdd）+ 6 个 god-file 条目一字未动（实读 `rot-budget.json:1-77` 与 diff 一致）。

**拍板核验**：`D-synthesis-plan-2026-09-05.md:220` §9.2 原文「批准 42→48（+6），到期 2026-10-15。... 首个额度消耗 = Phase -1 的 SSOT 策略文件与闸脚本」——与本单内容精确匹配。

实跑 `node scripts/verify-rot-budget.mjs` → `dir_entries:scripts=44/48`（新增 2 文件）✓

PASS。

---

## 二、QUALITY 独立判断

### 1. 无 owner 抽象

- `judgeChecklist` / `statusWords` / `reviewVerdicts` / `cannotVerifyPolicy` / `metaRatchetPaths` 等字段均有 spec 来源（brief §A 逐字列出）
- 验证器仅实现 spec 列出的语义（验证/扫描/归一化），未引入投机字段
- 无 interface-with-one-implementation、无 factory-for-one、无 config-for-never-change

PASS。

### 2. 预算闸严守

`rot-budget.json` diff 仅 `dir_entries:scripts` 一块（ceiling 42→48 + note 改写），其它 ceiling 零变更。GC#2「任何 ceiling 不得上调」严格守住。

PASS。

### 3. 闸自身质量

**失败路径全部非零退出**（逐路径读源码核对）：
- `verify-attempt`：
  - 缺参 → success=false → exit 1（`main:662-668`）
  - 错 base SHA → catch → success=false → exit 1（`verify-task-gate.mjs:180-187`）
  - 错 tip SHA → catch → success=false → exit 1（`verify-task-gate.mjs:196-203`）
  - allowlist 文件不存在 → success=false → exit 1（`verify-task-gate.mjs:210-217`）
  - 空 allowlist（仅注释）→ 全部越界 → success=false → exit 1
  - `git diff` 失败 → catch → success=false → exit 1（`verify-task-gate.mjs:237-244`）
  - 越界文件 → errors 累积 → exit 1
  - 未兑现 → 仅 warning，不影响 success（符合 brief §B.2「warning 不失败」）
- `validate-brief` / `validate-policy`：errors.length > 0 即 exit 1
- `--selftest`：全部 fixture 通过才 exit 0，否则 1

**正则/字符串误判面评估**：
- 节判定行级规则：条件 ① 要求 `## ` + 包含节名（中文也走 `toLowerCase().includes`，Chinese 不变）→ 对 `## 验证标准` 等子标题会误命中节 `验证`，但 brief §B.4 钉死规则「实现者不得再调整」，当前 brief/review 模板均无此干扰项，不影响本波使用。
- 豁免短语段落级授权（详见 Minor 1）

**条件早退检测**（selftest 是否「测了等于没测」）：
- 负向 a：不只测 status != 0，还断言输出含 `pages_archive.rs` → 真测语义，非恒真（`verify-task-gate.mjs:417`）
- 负向 c：断言输出含 `报告` → 真测错误信息，非恒真（`verify-task-gate.mjs:450`）
- 负向 d：断言输出含 `不算失败` → 真测具体短语（`verify-task-gate.mjs:465`）
- 负向 e：断言输出含 `do not flag` → 真测具体模式（`verify-task-gate.mjs:480`）
- 正向 2：断言输出含 `extra_unfulfilled.rs` → 真测 warning 真触发（`verify-task-gate.mjs:552`）

无「测了等于没测」用例。

### 4. 报告与命令一致性核对

- `verify-rot-budget.test.mjs`：12/12 实跑通过，与报告一致 ✓
- `verify-rot-budget.mjs`：scripts 44/48 + 其它读数全匹配报告 ✓
- `check-repo-hygiene.mjs`：实跑 exit 1 ✓（与报告一致）

PASS（带 2 个 Minor 见下）。

---

## 三、Critical / Important 发现

**无。**

所有 SPEC 必含节、子命令语义、selftest 覆盖、rot-budget 闸均严格对齐。失败路径全部非零，selftest 条件断言无恒真。

---

## 四、Minor 发现

### Minor 1：归一化规则超出 brief 字面范围（实现者自行放宽）

**位置**：`verify-task-gate.mjs:310-358`

**问题**：brief §B.3 行 74 仅对 PREJUDGING_PATTERNS 显式钉死归一化规则「扫描前先剔除围栏代码块与行内代码再判定」；行 73 的 EXEMPTION_PHRASES 段只写「同行或同句无 `用户拍板`/`拍板` 标注」，**未规定归一化**。但实现者在 `normalizeMarkdownLines` 后的 `normalizedLines` 上跑两类扫描，等于把归一化也加给了 EXEMPTION_PHRASES。

**判断**：非执行缺陷。brief-review §C-1 推演结论是「brief:73/74 全部短语均在反引号内 → 结构性自洽通过；真实 prose 豁免不受影响」，支持这一放宽。但严格按 brief §B.4 末段「实现者不得再调整」字面看，归一化扩大应用属「自行放宽」。本波无实际影响，记录备查。

### Minor 2：豁免短语授权检查超出「同行或同句」字面范围

**位置**：`verify-task-gate.mjs:319-340`

**问题**：brief §B.3 行 73 钉死「命中豁免短语且**同行或同句无** `用户拍板`/`拍板` 标注 → 非零」。实现三层授权检查：①同行 ②同句（按 `。！？!?；;` 切句）③同段（contiguous non-blank lines 合并后切句）。第 ③ 段是 brief 未规定的放宽。

**判断**：当前 brief 中无 paragraph-level exempt 案例触发差异，故无实际影响；但「同段授权」属规格外的判断点扩张，下一波若 brief 出现 paragraph 内豁免+远端拍板将造成 false negative。

### Minor 3：实施者 hygiene 报告说明不完整（自指）

**位置**：`reports/w16-1-report.md:130-133` vs 实跑 `check-repo-hygiene.mjs` 输出

**问题**：实跑 hygiene 输出两条违规：
1. `.superpowers/sdd/reports/w16-1-report.md:132 contains a local absolute path.` ← **报告自身**
2. `.superpowers/sdd/w16-1-brief-review.md:59 contains a local absolute path.`

报告 §遗留问题只解释第 ② 条（brief-review.md），未提第 ① 条。根因：报告 line 132 含字面本机用户目录绝对路径（此处已脱敏为占位描述）用以解释第 ② 条，本身又触发第 ① 条 hygiene 扫描。

**判断**：纯报告措辞完整性瑕疵，**不影响** committed 3 文件（实跑 hygiene 也未扫该 3 文件的绝对路径）。committed 文件本身 0 违规。`check:repo-hygiene` 失败属 WIP 阶段临时态，docs commit 收口（brief:117）时随 untracked 文件一并处理即可。

---

## 五、Cannot verify from diff

1. **实跑端到端 5 命令**：均亲跑（`--selftest` / `validate-policy` / `validate-brief` / `verify-rot-budget.test.mjs` / `verify-rot-budget.mjs`），结果与报告完全一致。`check-repo-hygiene.mjs` 也亲跑确认 exit 1。
2. **节判定规则对 w15-1l 历史 brief 成立**（brief §B.4 末段钉死）：未亲跑 `validate-brief .superpowers/sdd/w15-1l-brief.md`，因为 brief §B.4 钉死规则说明性即可，且 w15-1l-brief.md 实际是历史文档不在本单允许集。条件早退用例已用合成 brief 覆盖。
3. **报告里 rot 读数**：实跑 `verify-rot-budget.mjs` 5 grep + 3 dir + 6 god-file 读数全部逐项匹配报告。

无未核实缺口。

---

## 六、Global Constraints 逐项核验

| # | 约束 | 落地证据 | 判 |
|---|---|---|---|
| 1 | 纯 Node 标准库，零新依赖 | `verify-task-gate.mjs:3-7` 仅 import `fs`/`path`/`os`/`url`/`child_process` | PASS |
| 2 | rot-budget.json 仅 dir_entries:scripts 42→48 | diff 实测唯一变更块 | PASS |
| 3 | 日志与脚本输出 English-only | 实跑所有输出 PASS 行、错误消息、warning 文本均 English | PASS |
| 4 | 所有验证命令必须在 report 贴原文输出 | report §验证命令与输出原文 含 exit code + 完整文本 | PASS |
| 5 | commit 规则：逐文件点名 git add，禁 git add -A | `git show --name-only deae1b7` 仅 3 文件，无意外 | PASS（结果一致；具体 `git add` 命令不在 diff 内可核） |
| 6 | report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED | report line 136 = `DONE` | PASS |

---

## 七、判据总览

| 档位 | 数 |
|---|---|
| Critical | 0 |
| Important | 0 |
| Minor | 3（归一化规则超出 brief 字面 / 段落级授权超出「同行或同句」/ 报告 hygiene 解释不完整） |

---

## 八、结论

**APPROVE_WITH_CONCERNS**

- **SPEC PASS**：brief §A/B/C 三节逐字对齐；selftest 11/11；6 验证命令实跑与报告一致；rot-budget 改动匹配 §9.2 用户拍板。
- **QUALITY PASS**：零投机字段；失败路径全非零；selftest 条件断言非恒真；committed 3 文件零违规。
- **3 个 Minor** 均为规格外放宽/报告完整性瑕疵，本波无实际影响，记台账备查（实施者无需回修，可由编排者在波末一并登记或随 Phase 0 收口）。

审查者立场：交付可放行进入波级终审流程。
