# Task PHASE-0 Brief — 断源批（i18n 幽灵 / god-file 注释 / 守卫插电）

## 来源与验收标准

来源：GLM-5.3 外部咨询方案 Phase 0，经编排者按仓库现状修正（修正记录见下）；对应 roadmap T2-3 切片（用户 2026-08-21 拍板解冻该切片）+ R-14/R-15 残留。

**验收**：Spec 1-4 落地 + 验证输出进 report。

## 编排者预检结论（2026-08-21 实测，直接采信）

| 项 | 实证 | 处置 |
|---|---|---|
| i18n 路径 | `scripts/generate-i18n-contract.mjs:15,23` 写 `northhing-Installer`（大小写错），幽灵目录 `northhing-Installer/` 现存含两个生成文件；真实目标 `northing-installer/src/i18n/generatedLocaleContract.ts` 存在 | **修 + 删 + CI 断言**（Spec 1） |
| allow-god-file 注释 | 仅 2 处：`apps/cli/src/ui/theme.rs:1`（注释写 972L 实 990，已腐化）、`apps/desktop/src/app_state/callbacks_lifecycle.rs:1`（注释还写 "split planned"，T2-6 已改判活体实验） | **删**（Spec 2）——rot-budget.json 为尺寸唯一事实源 |
| hygiene 红灯两项 | ① `.agents/skills/lightweight-agent-execution/review-prompt.md` 命中 check-repo-hygiene.mjs:215 的 transient 规则（`/(^|[-_])review[-_]?prompt\.(txt|md)$/i`），但它是 SKILL.md:68,111 引用的**合法 skill 资源**；② `.opencode/model-capability-notes.md:86` 绝对路径违例在 **growth 并行 session 的未提交改动**里（HEAD 干净）——**该文件不许碰** | ①改名修复（Spec 3）；②不动，本地红灯归 growth session 自理，CI 只查已提交内容故为绿 |
| i18n contract 测试 | `pnpm run i18n:contract:test:ci` 存在但有 **24 个预存失败**（T2-3 冻结面） | 进 CI 但必须 `continue-on-error: true` 观察位（Spec 4） |

## 复用侦察（强制）

读 ci.yml 全文（特别是 :86 附近的 generate 步骤所在 job）、check-repo-hygiene.mjs 的违规规则全貌、i18n 生成器的目标清单。report 写「复用侦察」节。

## Spec（必须全部满足）

1. **i18n 三连**：
   - `generate-i18n-contract.mjs:15,23` 两处 `northhing-Installer` → `northing-installer`；
   - 删除幽灵目录 `northhing-Installer/`（git rm -r，确认全部被跟踪内容删除）；
   - 运行 `node scripts/generate-i18n-contract.mjs` 重新生成到正确路径（生成物变更随 commit；若生成内容与现有文件有 diff，审查其合理性——预期应为幂等或仅路径差异）；
   - 在 ci.yml 跑 generate 的同一 job 步骤后追加断言：`test ! -d northhing-Installer && test -f northing-installer/src/i18n/generatedLocaleContract.ts`（Ubuntu 语法；若该 job 跑在 Windows 则换 PowerShell 等价写法并说明）。
2. **删两处 allow-god-file 头注释**（theme.rs:1、callbacks_lifecycle.rs:1），整行删除不留残句。
3. **review-prompt.md 改名**：→ `reviewer-prompt.md`（已验证绕开规则正则：`reviewer-prompt` 中 "review" 后接 "er" 不匹配），同步改 SKILL.md:68,111 两处引用；git mv 保历史。
4. **CI 插电**：ci.yml 合适 job（静态检查类，参照 core-boundaries job 形态）新增：① `node scripts/check-repo-hygiene.mjs`（硬门）；② `pnpm run i18n:contract:test:ci` 挂 `continue-on-error: true` + 步骤注释注明"观察位：24 个预存失败归 T2-3 冻结面，解冻时转硬门"。
5. 不顺手碰：`.opencode/model-capability-notes.md`（growth 禁区）、i18n audit 工程（frozen 其余部分）、任何产品代码。

## Global Constraints（逐字遵守）

- 日志/注释/CI 文本 English-only、无 emoji。
- 幽灵目录删除前先确认其中无手工文件（全为生成物才可删——逐一比对与生成器输出的对应关系）。
- 若 generate 脚本运行失败或产生意外大面积 diff，STOP 报 BLOCKED。

## 验证（命令 + 输出都要进 report）

1. `node scripts/generate-i18n-contract.mjs` 跑通 + `Test-Path northhing-Installer` = False + `Test-Path northing-installer/src/i18n/generatedLocaleContract.ts` = True（贴输出）
2. `node scripts/check-repo-hygiene.mjs` 在 worktree 内跑（预期：review-prompt 违例消失；model-capability-notes 那条若在 worktree 不存在则全绿——worktree 从 clean HEAD 拉出，应全绿，贴输出）
3. `node scripts/check-core-boundaries.mjs` + `pnpm run check:rot`
4. `cargo check --workspace`（MSVC wrapper `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`——生成物变了需确认编译链完整）
5. `git diff --stat`

## 报告

`.superpowers/sdd/task-phase0-report.md`：Spec 逐条、复用侦察节、验证输出尾部、偏离声明。最后消息以状态词开头。

## 派发元信息

- BASE `b7ede1c`；worktree `E:\agent-project\.worktrees\northing-phase0`（分支 `feat/phase0-0821`）
- commit message 后缀 `(PHASE-0)`；只 stage 你改的文件。
