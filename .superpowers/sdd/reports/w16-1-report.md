# W16-1 实施报告：工作流闸脚本 + SSOT 策略文件

## 改动摘要

1. **新增 `scripts/workflow-policy.json`**：落实工作流 SSOT 策略定义，包含版本号、judgeChecklist（9项）、statusWords（4项）、reviewVerdicts（5项）、cannotVerifyPolicy、metaRatchetPaths 以及 brief/report 必含节定义。
2. **新增 `scripts/verify-task-gate.mjs`**：零依赖纯 Node.js 标准库实现工作流机械验证脚本，提供 `validate-policy`、`verify-attempt`、`validate-brief` 三个子命令，内联 `--selftest` 自测套件（7 负向 fixture + 4 正向 fixture，临时文件严格走 `os.tmpdir()`）。
3. **更新 `scripts/rot-budget.json`**：将 `dir_entries:scripts` 上限从 42 调整为 48，note 注明用户 2026-09-05 拍板一次性额度（到期 2026-10-15）。其他条目一字未动。

Commit: `deae1b7` (`feat(scripts): add workflow policy SSOT + task gate verifier (W16-1)`)。

---

## 三子命令设计决策

### 1. `validate-policy`
作为 SSOT 策略文件的防御性校验器，严格比对必需字段的存在性与类型正确性。对 `statusWords` 与 `reviewVerdicts` 实施强枚举精确比对（长度与各项字符串全等），确保下游工具与子代理状态词无漂移；同时校验 `cannotVerifyPolicy` 包含 `blocking` 与 `auxiliary` 键、`metaRatchetPaths` 为字符串数组，遇任何不符立即列出具体错误并以非零码退出，践行 fail-loud 原则。

### 2. `verify-attempt`
负责 commit-bound 范围的机械核验，首先通过 `git rev-parse --verify` 确保 base 与 tip SHA 存在，杜绝虚假提交点；使用 `git diff --name-only <base>..<tip>` 获取真实改动文件集，与清洗后的 allowlist（支持 `#` 注释、空行忽略、路径正斜杠归一化）进行双向精确比对。名单外改动列为越界并阻断（非零退出），名单内未改动列为 warning 提示但不阻断，直接复现并防范 W15-1l 历史事故。

### 3. `validate-brief`
基于 policy 定义的 `briefRequiredSections` 执行任务简报合规性检验。遵循钉死规则实现标题（`## ` 包含匹配）与元数据行（`- <节名>` / `<节名>` 开头匹配）的双重节判定；在短语扫描阶段先剥离围栏代码块（```）与行内反引号（`），防止规范引用引发假阳性；对豁免短语实施同级行或同句（按标点划分）的授权关键词（`用户拍板` / `拍板`）关联校验；对审查预判短语在正文中一律拦截；遇 `续单` 强制校验独立 BASE 行与允许文件集。

---

## 验证命令与输出原文

### 命令 1：`node scripts/verify-task-gate.mjs --selftest`
- Exit Code: 0
- 输出：
```text
[PASS] negative fixture a: replay W15-1l real incident (detected out-of-bounds pages_archive.rs)
[PASS] negative fixture b: invalid git revision rejected
[PASS] negative fixture c: missing required section in brief rejected
[PASS] negative fixture d: unapproved exemption phrase rejected
[PASS] negative fixture e: prejudging reviewer phrase in prose rejected
[PASS] negative fixture f: bad policy missing required field rejected
[PASS] negative fixture g: policy enum mismatch rejected
[PASS] positive fixture 1: complete 8-file allowlist passes
[PASS] positive fixture 2: allowlist with unfulfilled file passes with warning
[PASS] positive fixture 3: w16-1-brief.md passes validate-brief
[PASS] positive fixture 4: default workflow-policy.json passes validate-policy
Selftest passed: 11 fixtures passed (7 negative, 4 positive).
```

### 命令 2：`node scripts/verify-task-gate.mjs validate-policy`
- Exit Code: 0
- 输出：
```text
Policy validation passed: E:\agent-project\northing\scripts\workflow-policy.json
```

### 命令 3：`node scripts/verify-task-gate.mjs validate-brief .superpowers/sdd/w16-1-brief.md`
- Exit Code: 0
- 输出：
```text
Brief validation passed: .superpowers/sdd/w16-1-brief.md
```

### 命令 4：`node scripts/verify-rot-budget.test.mjs`
- Exit Code: 0
- 输出：
```text
✔ compliant fixture exits 0 and reports success (157.2079ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (141.8132ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (144.8778ms)
✔ registered god-file exceeding ceiling fails (12.8544ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (12.4578ms)
✔ dir-entry-count compliant fixture passes (148.4888ms)
✔ dir-entry-count exceeding ceiling fails and exits 1 (146.4455ms)
✔ dir-entry-count on non-existent directory fails and exits 1 (127.6648ms)
✔ tests.rs file is excluded from rot budget measurement (10.7775ms)
✔ *_tests directory files are excluded from rot budget measurement (11.1456ms)
✔ actual workspace rot budget passes with current manifest (491.8511ms)
✔ dead god-file registration warns but does not fail verification (146.9522ms)
ℹ tests 12
ℹ suites 0
ℹ pass 12
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 1567.7182
```

### 命令 5：`node scripts/verify-rot-budget.mjs`
- Exit Code: 0
- 输出：
```text
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=371/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=44/48, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=54/400], 6 god-file rules checked across 1368 files).
```

### 命令 6：`node scripts/check-repo-hygiene.mjs`
- Exit Code: 1
- 输出：
```text
Repository hygiene check failed:
- .superpowers/sdd/w16-1-brief-review.md:59 contains a local absolute path.
```

---

## selftest 逐条结果

| Fixture | 类型 | 预期行为 | 实际结果 | 状态 |
|---|---|---|---|---|
| negative fixture a | 负向 | replay W15-1l 真实事故（05bbd40..0ea30b3 + 7文件 allowlist），必须非零且列出 `pages_archive.rs` | 退出码 1，stderr/stdout 包含 `pages_archive.rs` | PASS |
| negative fixture b | 负向 | 不存在/错误的 SHA，必须非零退出 | 退出码 1，报错 `Invalid or non-existent git base revision` | PASS |
| negative fixture c | 负向 | 缺少必需节（如 `报告`）的 brief，必须非零退出 | 退出码 1，报错 `Missing required section(s): 报告` | PASS |
| negative fixture d | 负向 | 正文含未标注豁免短语 `不算失败`，必须非零退出 | 退出码 1，报错未授权豁免短语 | PASS |
| negative fixture e | 负向 | 正文含预判措辞 `do not flag`，必须非零退出 | 退出码 1，报错预判措辞 | PASS |
| negative fixture f | 负向 | policy 缺少 `metaRatchetPaths` 字段，必须非零退出 | 退出码 1，报错字段缺失 | PASS |
| negative fixture g | 负向 | policy 的 `statusWords` 缺失枚举项，必须非零退出 | 退出码 1，报错枚举不匹配 | PASS |
| positive fixture 1 | 正向 | W15-1l 完整 8 文件 allowlist，必须 0 退出 | 退出码 0，验证通过 | PASS |
| positive fixture 2 | 正向 | 8 文件 + 1 个未兑现文件，必须 0 退出并警告未兑现项 | 退出码 0，warning 输出列出 `extra_unfulfilled.rs` | PASS |
| positive fixture 3 | 正向 | 本 brief (`.superpowers/sdd/w16-1-brief.md`) 经 `validate-brief`，必须 0 退出 | 退出码 0，验证通过 | PASS |
| positive fixture 4 | 正向 | 当前 `scripts/workflow-policy.json` 经 `validate-policy`，必须 0 退出 | 退出码 0，验证通过 | PASS |

---

## Rot 读数

- `dir_entries:scripts` 实际读数：**44/48**（新增 2 个文件：`scripts/workflow-policy.json` 与 `scripts/verify-task-gate.mjs`，由原 42 个文件增至 44 个，未触及新上限 48）。
- 其他项：`unwrap_production=483/502`、`expect_production=940/1089`、`let_underscore=371/388`、`unix_epoch_inline=69/69`、`allow_dead_code=106/109`、`dir_entries:docs/design=1/1`、`dir_entries:.superpowers/sdd=54/400`、6 个 god-file 规则全部合规。

---

## 遗留问题与 Caveat

1. **`check-repo-hygiene.mjs` 在未跟踪文件上的拦截**：
   - 运行 `node scripts/check-repo-hygiene.mjs` 失败报错：`.superpowers/sdd/w16-1-brief-review.md:59 contains a local absolute path.`。
   - 原因：该文件是编排者派发任务前生成的简报审查文档（未跟踪文件），其第 59 行包含了本机用户目录绝对路径字样（此处已脱敏为占位描述），触发了 hygiene 脚本针对未跟踪文件的绝对路径扫描。
   - 现场纪律遵从：本单允许文件集严格限定为 3 个文件（`scripts/workflow-policy.json`、`scripts/verify-task-gate.mjs`、`scripts/rot-budget.json`），其他文件皆为禁区。本子代理未越权修改该审查文件，本提交包含的 3 个文件自身无任何 hygiene 违规，留待编排者统一进行 docs commit 收口。

---

DONE
