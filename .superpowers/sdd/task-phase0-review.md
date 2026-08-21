# Task PHASE-0 Review (independent verifier)

- range: `b7ede1c..9df49b2` (single commit)
- worktree: `E:\agent-project\.worktrees\northing-phase0` (branch `feat/phase0-0821`)
- diff package: `.superpowers/sdd/review-package-phase0.diff` (same 6 files, 43 +/- 7 stats)
- implementer report: `.superpowers/sdd/task-phase0-report.md`
- review verdict: **APPROVED** (spec clean, 0 Critical / 0 Important / 3 Minor)

## SPEC 双判决

### Spec 1 — i18n 路径修复与 CI 断言

- **`scripts/generate-i18n-contract.mjs`**：diff 严格落在 :15 与 :23 两行（`get-content` 验证 line 15 与 line 23 均为 `path: path.join(root, 'northing-installer', ...)`）。其余 467 行零改动。无任何「顺手清配额」式越界。
- **幽灵目录 `northhing-Installer/`**：
  - 磁盘：`Test-Path northhing-Installer` = False ✅
  - git 历史：`git log --all --oneline -- northhing-Installer` 无任何提交 ✅（说明从未被跟踪，纯运行时产物）
  - 重新跑生成器后 `git status --short` 仅显示 `task-phase0-brief.md` / `task-phase0-report.md` 两个未跟踪文件 ✅
  - 生成器无新增写入路径（4 个目标路径全部为 `northing-installer` / `src/crates/assembly/core` / `src/web-ui`，磁盘已存在目标全部命中 gitignored 路径，无任何变更）
- **CI 断言**：`ci.yml:88` 的 `test ! -d northhing-Installer && test -f northing-installer/src/i18n/generatedLocaleContract.ts` 在 generate 步骤之后（顺序正确），`shell: bash` 与步骤原设置一致。

### Spec 2 — `allow-god-file` 头注释删除

- `src/apps/cli/src/ui/theme.rs`：line 1 现为 `use once_cell::sync::Lazy;`，无残句 ✅
- `src/apps/desktop/src/app_state/callbacks_lifecycle.rs`：line 1 现为 `//! Lifecycle Slint callback wirings (R37a split from mod.rs)`，无残句 ✅
- 两个文件仍列于 `scripts/rot-budget.json` manifest（`theme.rs` ceiling 990、`callbacks_lifecycle.rs` ceiling 1010），当前实测 989 / 1009 行均未越线，`pnpm run check:rot` 通过 ✅

### Spec 3 — `review-prompt.md` 改名 + SKILL.md 同步

- `git mv` 完成：`git diff --diff-filter=R --name-status` 显示 `R100 .agents/skills/lightweight-agent-execution/{review-prompt.md => reviewer-prompt.md}`（100% 相似，0 行变化，历史保留）✅
- SKILL.md：line 68（Phase 3 prompt 引用）与 line 111（Phase 3 dispatch 注释）均已替换为 `reviewer-prompt.md` ✅
- 活动 `rg 'review-prompt\.md'` 在 SKILL.md / reviewer-prompt.md 自指之外仍有 **3 处历史档案命中**（详见 Minor-1）。

### Spec 4 — CI 插电

- `repo-hygiene` 作业（`ci.yml:153-165`）：**无 `continue-on-error`**，硬门 ✅
- `i18n-contract` 作业（`ci.yml:167-187`）：`continue-on-error: true` + 步骤上方注释 `Observation slot: 24 pre-existing failures belong to frozen T2-3 surface, convert to hard gate when unfrozen` ✅（English-only、无 emoji、T2-3 标注准确）
- 两作业 `runs-on: ubuntu-latest`、使用 `actions/setup-node@v4` + `node-version: "22"`，与既有 `core-boundaries` / `rot-budget` 作业形态一致，复用良好
- 顺序：generate（i18n 步骤）→ `Check compilation`（cargo check）→ `Run workspace Rust tests`；新两作业作为独立 job 并列，无 `needs:` 依赖（合适，独立可跑）

### Spec 5 — 边界约束

- `.opencode/model-capability-notes.md` **不在 diff** 中 ✅（`git diff --name-only` 仅含 6 文件：SKILL.md / reviewer-prompt.md / ci.yml / generate-i18n-contract.mjs / theme.rs / callbacks_lifecycle.rs）
- i18n audit 工程（`scripts/i18n-audit.mjs` / `scripts/i18n-contract.test.mjs`）未触碰 ✅
- 无产品运行时业务代码变更 ✅

## 独立验证（实跑输出）

| 命令 | 结果 |
|---|---|
| `node scripts/generate-i18n-contract.mjs` | `[i18n:generate] Wrote 4 generated i18n contract file(s).` ✅ |
| 二次重跑后 `git status --short` | 仅两个未跟踪 brief/report，**幂等** ✅ |
| `Test-Path northhing-Installer` | `False` ✅ |
| `Test-Path northing-installer/src/i18n/generatedLocaleContract.ts` | `True` ✅ |
| `node scripts/check-repo-hygiene.mjs` | `Repository hygiene check passed (2 content files scanned, 3352 filenames checked).` ✅ |
| `node scripts/check-core-boundaries.mjs` | `Core boundary check passed.` ✅ |
| `pnpm run check:rot` | `Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1363 files).` ✅ |
| `cargo check --workspace` (MSVC) | `Finished dev profile in 45.08s`（0 errors，仅 18 条预存 warnings，与改动无关）✅ |
| `pnpm run i18n:contract:test:ci` | 退出码 1，断言失败（确认 24 项预存失败之一：`web-ui resourceRoot does not exist`）— CI 中由 `continue-on-error: true` 承接 ✅ |

## QUALITY 三必查

1. **复用核查**：CI 新两作业完全复用既有模式（`actions/checkout@v4` + `setup-node@v4` + `node-version: "22"`），与 `core-boundaries` / `rot-budget` 形态对齐。生成器改动直接修改既有两个常量字符串，不引入新 helper。✅
2. **无 owner 抽象**：未新增任何 trait / interface / factory / wrapper；纯字符串修正 + 文件重命名 + CI 步骤插入。✅
3. **预算闸**：`scripts/rot-budget.json` 零变更；`theme.rs` (989) / `callbacks_lifecycle.rs` (1009) 仍在 ceiling 内（990 / 1010）。无 `let _ =` 等其他预算条目受影响。✅

## Findings

### Critical
（无）

### Important
（无）

### Minor

- **Minor-1**：活动 `rg 'review-prompt\.md'` 仍命中 **3 处历史档案**（活动 SKILL.md 已全同步）：
  - `docs/superpowers/specs/2026-06-23-lightweight-agent-execution-protocol.md:328`
  - `docs/archive/handoffs/2026-07-17-progress.md:223`
  - `.task/archive/prompt-cache-stats/HANDOVER.md:77`

  这些都是 `archive/` 或历史 spec doc 中的文字提及，不命中 `check-repo-hygiene.mjs:215` 的文件名正则（hygiene 检查仍通过），不阻断功能。严格按「全仓零命中」字面读是 spec 偏差，但活动 SKILL.md 与脚本均已同步，CI 卫生检查通过，影响仅限历史档案阅读体验。建议在终审 triage 阶段统一扫一遍并对非冻结文档做一次性订正。

- **Minor-2**：`.github/workflows/nightly.yml:201` 的 `upload-artifact` 路径仍为 `northhing-Installer/src-tauri/target/release/northhing-installer.exe`（首字母大写）。目录现已是 `northing-installer/`（小写），该路径在生产构建中很可能失败——属于本次 spec 范围外的相邻债务（spec 4 只指定 `ci.yml`），按「顺手清配额」原则可一并处理；不影响本次 PR 接受。

- **Minor-3**：`AGENTS-CN.md`、`docs/architecture/i18n.md`、`docs/development/i18n.md`、`docs/northhing-name.md`、`northing-installer/{AGENTS.md, AGENTS-CN.md, README.md}`、`src/shared/i18n/contract/locales.json`、`scripts/i18n-dynamic-key-allowlist.json` 仍引用旧名 `northhing-Installer`（含大小写错误的安装器目录）。其中：
  - `scripts/i18n-audit.mjs` / `scripts/i18n-contract.test.mjs` 明确属 spec 5 冻结的 i18n audit 工程（i18n 状态在 v0.1.0 锁定），不在本次范围；
  - 其余文档 / config 属相邻债务，可由终审 triage 决策是否开新切片。

  本次 PHASE-0 仅承诺生成器路径 + 幽灵目录 + god-file 注释 + CI 守卫，不应回炉扩范围。

## 偏离声明
- 实现者未在 report 中显式声明「历史档案中 review-prompt 文本残留」与「nightly.yml 路径漂移」——属于本次 spec 范围外的相邻债务。建议在终审 ledger 中以 Minor 标注 + 归属下一切片。

## 结论

**APPROVED** — 4 项 spec 全部满足，6 项硬约束全部满足，独立实跑 5 项验证全部通过；仅 3 项 Minor（均为 spec 范围外的相邻债务），不构成阻塞。PHASE-0 切片可进入终审 / 合入流程。
