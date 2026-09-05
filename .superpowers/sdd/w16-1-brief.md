# W16-1 Brief：工作流闸脚本 + SSOT 策略文件

- 任务标识：W16-1
- 波次计划：`.superpowers/sdd/plan-2026-09-05-w16-trusted-core.md`（Global Constraints 逐字适用，本 brief 末附原文）
- 来源：`E:\agent-project\.opencode\external-review\2026-09-05\D-synthesis-plan-2026-09-05.md` §3 Phase -1（-1.1/-1.2/-1.3/-1.7（仅额度半，"新增必须退役旧脚本"机械化顺延 Phase 0，编排者负责波末台账登记该顺延项））+ §9 拍板记录
- BASE：`19349cd`（main HEAD；编排者已预跑基线：`verify-rot-budget.test.mjs` 12/12、`verify-rot-budget.mjs` 绿（scripts 42/42）、`check-repo-hygiene` 绿）

## 背景（一句话）

外部审查证实：W15-1l 的 allowlist 被事后扩围放行，因为"brief 允许文件集 vs 实际 diff"从未机械比较过。本单建立第一道 commit-bound 机械闸 + 策略单源文件。这是 Phase -1 最小可信核的第一块。

## 允许文件集（diff 越出 = judge Critical）

1. `scripts/workflow-policy.json`（新增）
2. `scripts/verify-task-gate.mjs`（新增；自测内联 `--selftest`，**不另建测试文件**）
3. `scripts/rot-budget.json`（仅 `dir_entries:scripts` 42→48 + note 改写，见下）

禁区：其它一切文件。尤其：`verify-rot-budget.mjs` / `ci.yml` / 任何 `.rs`。

## 功能要求

### A. `scripts/workflow-policy.json`（SSOT 雏形）

字段（逐字，值按描述填）：

```json
{
  "version": 1,
  "judgeChecklist": [
    "spec-by-acceptance-criteria",
    "quality-independent",
    "reuse-verification",
    "no-ownerless-abstraction",
    "budget-gate",
    "conditional-early-exit-test",
    "god-file-health-note-if-touched",
    "pure-move-equivalence",
    "evidence-spot-check"
  ],
  "statusWords": ["DONE", "DONE_WITH_CONCERNS", "NEEDS_CONTEXT", "BLOCKED"],
  "reviewVerdicts": ["APPROVE", "APPROVE_WITH_CONCERNS", "CANNOT_VERIFY", "BLOCKED", "FAIL"],
  "cannotVerifyPolicy": {
    "blocking": "verdict-blocking evidence (acceptance criteria assertions)",
    "auxiliary": "max 2 items, no trust-boundary touch, requires owner + deadline, verdict capped at APPROVE_WITH_CONCERNS"
  },
  "metaRatchetPaths": [
    "scripts/verify-task-gate.mjs",
    "scripts/verify-rot-budget.mjs",
    "scripts/workflow-policy.json",
    ".github/workflows/"
  ],
  "briefRequiredSections": ["任务标识", "BASE", "允许文件集", "禁区", "验证", "报告"],
  "reportRequiredSections": ["改动摘要", "验证", "状态"]
}
```

### B. `scripts/verify-task-gate.mjs`（纯 Node 标准库，零依赖）

三个子命令 + 自测：

1. `validate-policy [--policy <path>]`（默认 `scripts/workflow-policy.json`）
   - 校验：必需字段存在且类型正确；`judgeChecklist` 为非空字符串数组；`statusWords`/`reviewVerdicts` 与上述枚举一致；`metaRatchetPaths` 为字符串数组；`cannotVerifyPolicy` 含 `blocking`/`auxiliary` 两键。
   - 任一不满足：逐条列出错误，非零退出。

2. `verify-attempt --base <sha> --tip <sha> --allowlist <path>`
   - `git rev-parse --verify` 校验两个 SHA；`git diff --name-only <base>..<tip>` 取实际改动文件集。
   - allowlist 文件格式：每行一个 repo 相对路径（正斜杠），`#` 开头与空行忽略。
   - 精确比较双向：实际有而名单无 = 越界（列出）；名单有而实际无 = 未兑现（列出，warning 不失败）。
   - 越界 → 非零退出，输出逐行列出越界文件。

3. `validate-brief <path>`
   - 必含节（取自 policy `briefRequiredSections`）：缺节非零。
   - 预设豁免扫描：命中 `不算失败` / `不算越界` / `无需验证` 且**同行或同句无** `用户拍板` / `拍板` 标注 → 非零。
   - 预判审查措辞扫描：命中 `do not flag` / `不要 flag` / `不需 flag` / `at most Minor` / `至多 Minor` → 非零。**归一化规则（钉死，不得自行放宽）**：扫描前先剔除围栏代码块（``` 包围段）与行内代码（反引号包围段）内的文本再判定；该豁免面为已知取舍（规格文档可在代码标记内引用这些短语），Phase 0 可加严。
   - 含 `续单` 字样 → 该 brief 必须同时含独立的 `BASE` 行与 `允许文件集` 节（续单 = 新 attempt）。

4. `--selftest`（内联，不另建文件；临时文件一律走 `os.tmpdir()`，先例 `verify-rot-budget.test.mjs:14`）
   - 负向 fixture（必须全部非零退出）：
     a. **replay W15-1l 真实事故**：`verify-attempt --base 05bbd40 --tip 0ea30b3` + 7 文件 allowlist（即当年 brief 原名单，见下）→ 必须红，且越界输出含 `pages_archive.rs`；
     b. 错误/不存在的 SHA → 红；
     c. 缺节 brief（临时文件）→ 红；
     d. 含未标注豁免短语的 brief → 红；
     e. 含预判审查措辞的 brief（prose 行内、非代码标记包裹）→ 红；
     f. 坏 policy（缺字段，临时文件）→ 红；
     g. 枚举不一致 policy（`statusWords` 缺一项，临时文件）→ 红。
   - 正向 fixture（必须全绿）：a 的 8 文件完整名单（含 `pages_archive.rs`）→ 绿；8 文件名单 + 1 个未兑现文件（allowlist 超集）→ 绿且 warning 列出未兑现项；本 brief 自身经 `validate-brief` → 绿；当前 policy.json 经 `validate-policy` → 绿。
   - **节判定规则（钉死）**：对 policy `briefRequiredSections` 的每个节名，文件满足其一即视为存在——①存在以 `## ` 开头且包含该节名的标题行；②存在以 `- <节名>` 或 `<节名>` 开头（节名后接 `：` / `:` / 空格 / 行尾）的任意行。匹配大小写不敏感。该规则对本 brief 与 w15-1l 历史 brief 均成立，实现者不得再调整。
   - selftest 全部输出原文进 report。

   W15-1l 原 7 文件名单（fixture a 用）：`src/apps/desktop/src/ui_dioxus/api.rs`、`api_fs.rs`、`api_memory.rs`、`api_settings.rs`、`api_provider_edit.rs`、`app.rs`、`approval_card.rs`（同目录前缀省略写法禁止，fixture 里写全路径）。

### C. `scripts/rot-budget.json`

- `dir_entries:scripts`：ceiling 42→48；note 改写为：`一次性额度：用户 2026-09-05 拍板 +6 供 Phase -1~2 整改用，到期 2026-10-15 未用部分回落需重确认；新增脚本仍须退役旧脚本（机械化待 Phase 0）`。
- 其它条目一字不动。

## 验证（命令 + 输出原文进 report）

```text
node scripts/verify-task-gate.mjs --selftest
node scripts/verify-task-gate.mjs validate-policy
node scripts/verify-task-gate.mjs validate-brief .superpowers/sdd/w16-1-brief.md
node scripts/verify-rot-budget.test.mjs
node scripts/verify-rot-budget.mjs
node scripts/check-repo-hygiene.mjs
```

（注：本 brief 自身须通过 validate-brief，节判定按上述钉死规则。）

## 报告

写到 `.superpowers/sdd/reports/w16-1-report.md`：改动摘要 / 三子命令设计决策（各一段）/ 验证命令+输出原文 / selftest 逐条结果 / rot 读数（应 44/48）/ 遗留问题 / 结尾状态词（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）。

## 派发元信息

- commit 规则：逐文件点名 `git add`；message：`feat(scripts): add workflow policy SSOT + task gate verifier (W16-1)`，body 注明：`dir_entries:scripts 42→48 经用户 2026-09-05 拍板（一次性额度，到期 2026-10-15）`。
- **report / brief / 本 review 文件均不入本 commit**（允许文件集只有 3 个代码/数据文件），由编排者 docs commit 收口（先例：`0ea30b3` 仅代码文件，`0844150` docs 收口）。
- skill 前置阅读：`E:\agent-project\.opencode\skills\anti-rot-system\SKILL.md`——遵循其中与本任务相关的约定（fail-loud、ratchet 只紧不松），不因此扩展任务范围。

## Global Constraints（摘编自计划，W16-4 专项省略）

1. 纯 Node 标准库，零新依赖；PowerShell 脚本兼容 pwsh 7。
2. `scripts/rot-budget.json`：除 `dir_entries:scripts` 按拍板 42→48 外，任何 ceiling 不得上调。
3. 日志与脚本输出 English-only。
4. 所有验证命令必须在 report 贴原文输出（命令 + exit code）。
5. commit 规则：逐文件点名 `git add`，禁 `git add -A`。
6. report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
