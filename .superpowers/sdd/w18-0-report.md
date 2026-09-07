# W18-0 Report — task-gate rename 漏检修复

## 改动摘要

1. **diff 枚举与 NUL token 解析**：
   - 将 `scripts/verify-task-gate.mjs` 中 `verifyAttempt()` 的 diff 命令从 `git diff --name-only <base>..<tip>` 改为 `git diff --name-status -z <base>..<tip>`。
   - 解析 `\0` 分隔的 token 流：当状态 token 首字符为 `R` 或 `C`（如 `R100`）时，依次取出旧路径与新路径（步进 3 个 token）；其余状态取出单个文件路径（步进 2 个 token）。
   - 对所有提取出的路径统一按项目既有规则归一化（`\` → `/`，去除前导 `./`）。

2. **rename/copy 双端校验与定位标注**：
   - rename/copy 的旧路径与新路径均加入 `actualFiles` 与 `actualSet`。
   - 越界检查针对两端：若未包含在 allowlist 中即记为 out-of-bounds 错误；对于 rename 旧路径越界，错误信息附加 `(rename source)` 标注。
   - `unfulfilled` 语义保持不变（allowlist 中声明但 diff 中未出现的条目报 warning）。

3. **测试固件与统计更新**：
   - `--selftest` 新增 negative fixture h（基于真实历史 `df5c1ce..a32b4f7`，allowlist 缺 rename 源路径，断言非零退出且输出包含被重命名的旧路径）与 positive fixture 5（同 base/tip，allowlist 包含全部 4 个路径，断言 exit 0）。
   - 汇总统计动态计算 negative 与 positive 数量，当前输出为 `13 fixtures passed (8 negative, 5 positive)`。

## 验证

### 1. 自测试套件 (--selftest)

命令：
```bash
node scripts/verify-task-gate.mjs --selftest
```

输出：
```text
[PASS] negative fixture a: replay W15-1l real incident (detected out-of-bounds pages_archive.rs)
[PASS] negative fixture b: invalid git revision rejected
[PASS] negative fixture c: missing required section in brief rejected
[PASS] negative fixture d: unapproved exemption phrase rejected
[PASS] negative fixture e: prejudging reviewer phrase in prose rejected
[PASS] negative fixture f: bad policy missing required field rejected
[PASS] negative fixture g: policy enum mismatch rejected
[PASS] negative fixture h: rename source path omitted from allowlist rejected
[PASS] positive fixture 1: complete 8-file allowlist passes
[PASS] positive fixture 2: allowlist with unfulfilled file passes with warning
[PASS] positive fixture 3: w16-1-brief.md passes validate-brief
[PASS] positive fixture 4: default workflow-policy.json passes validate-policy
[PASS] positive fixture 5: rename with both source and destination in allowlist passes
Selftest passed: 13 fixtures passed (8 negative, 5 positive).
```
Exit code: `0`

### 2. 漏洞复现对照 (df5c1ce..a32b4f7)

测试 allowlist（缺少 rename 旧路径 `docs/handoffs/2026-09-05-six-tasks-green-rot-audit.md`）：
```text
.superpowers/sdd/plan-2026-09-07-w18-phase0-checker-hardening.md
docs/handoffs/2026-09-07-workflow-improvement-verdicts.md
docs/archive/handoffs/2026-09-05-six-tasks-green-rot-audit.md
```

#### 修复前（基线实证）
命令：
```bash
node scripts/verify-task-gate.mjs verify-attempt --base df5c1ce --tip a32b4f7 --allowlist <allowlist-file>
```
输出：
```text
Attempt verification passed: all modified files are within allowlist.
```
Exit code: `0`

#### 修复后
命令：
```bash
node scripts/verify-task-gate.mjs verify-attempt --base df5c1ce --tip a32b4f7 --allowlist <allowlist-file>
```
输出：
```text
Attempt verification failed:
  - Out-of-bounds file modification: docs/handoffs/2026-09-05-six-tasks-green-rot-audit.md (rename source)
```
Exit code: `1`

补全旧路径后的正向对照：
```text
Attempt verification passed: all modified files are within allowlist.
```
Exit code: `0`

### 3. 仓库卫生检查 (check:repo-hygiene)

命令：
```bash
pnpm run check:repo-hygiene
```

输出：
```text
> northhing@0.2.10 check:repo-hygiene <WORKSPACE>
> node scripts/check-repo-hygiene.mjs

Repository hygiene check passed (3 content files scanned, 3843 filenames checked).
```
Exit code: `0`

### 4. 策略验证回归 (validate-policy)

命令：
```bash
node scripts/verify-task-gate.mjs validate-policy
```

输出：
```text
Policy validation passed: <WORKSPACE>/scripts/workflow-policy.json
```
Exit code: `0`

## 状态

DONE
