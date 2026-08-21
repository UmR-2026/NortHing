# Task PHASE-1A Review — Independent Verdict

**Worktree**: `E:\agent-project\.worktrees\northing-p1a` (branch `feat/phase1a-sweep-0821`)
**Range**: `9970c6a..c8868fe` (single commit, 211 files, -5677 lines)
**Verdict**: **REJECTED**

---

## TL;DR

实现者干净地完成了 9 脚本删除、package-lock.json 删除、.handoffs 迁移、nightly.yml 修正与全仓 `northhing-Installer` 大写 I 清扫、禁区清单 100% 遵守、cargo + 三个 repo 脚本验证全绿。

但 SPEC 4 引用完整性约束被违反：`docs/design/2026-07-22-frontend-redesign/` 与 `docs/design/2026-07-23-self-cognition/` 两个目录被归档，但报告 §3.3 误判它们"无外部活引用"，实际有 22 个活文档/代码引用文件路径 broken。brief 第 16 行明确要求"有活引用的留下并在 report 说明"，我的任务约束第 9 行明确要求"引用必须同步改或该目录不该动"——实施者两条都没做。

**Fixer 需要决定**：A) 在 22 个活引用文件中批量 sed 替换 `docs/design/2026-07-2{2-frontend-redesign,3-self-cognition}` → `docs/archive/design/...`；B) `git restore` 把这两个目录移回原位。

---

## 双判决

### SPEC 判决

| Spec | 要求 | 落地 | 判决 |
|---|---|---|---|
| **1** | 删 9 个一次性脚本（+ `replace_theme.py` 已不存在核销） | ✅ `git diff --name-only --diff-filter=D` 确认恰好 9 个脚本 + 1 个 `package-lock.json`；核销项有 `Get-ChildItem` 空集证据 | **PASS** |
| **2** | 删根 `package-lock.json`（先 rg 验证） | ✅ `git rm` 已做；`git grep` 全仓零活引用（3 个 `builtin_skills/gstack-*/SKILL.md` 是 generic npm-fallback 教学文档，非活引用；`docs/status/full-review-2026-08-16.md:90` 是历史叙述） | **PASS** |
| **3** | `.handoffs/` 已跟踪文件迁移 | ✅ `git mv .handoffs/review-commit-997e14e_20260717.md docs/handoffs/`；`.handoffs/` git ls-files 空 | **PASS** |
| **4** | docs/design 5 目录归档 + 引用完整性 | ⚠️ **5 目录已归档**，但**引用完整性约束违反**：2 个目录有 22 个活引用文件未同步改路径，也未按 brief 要求"留下不动" | **FAIL** |
| **5** | nightly.yml 路径修正 + 全仓 `northhing-Installer` 大写 I 清扫 | ✅ `.github/workflows/nightly.yml:201` 修正；git grep `northhing-Installer` 仅命中 i18n 冻结区 3 处（豁免合规） | **PASS** |
| **6** | M3 文档命名修正 | ✅ 白名单 5 文件 + `northing-installer/AGENTS-CN.md` 全改（brief 第 17 行"全文件 rg 清残余"覆盖） | **PASS** |
| **7** | 顺手禁区清单 | ✅ `split_manager.py`/`copy_reference.cjs` 留存；`handoff-g2-t9-2026-08-07.md` 零出现在 diff；`locales.json`/`allowlist.json` 零改动；worktree/target 零改动 | **PASS** |

### QUALITY 判决（三必查）

1. **git mv 历史保留**：抽查 3 个归档目录（`first-entry-design.md`、`oklch-to-srgb.py`、`docs/handoffs/review-commit-997e14e_20260717.md`），`git log --follow` 全部回溯到原始 commit ✓
2. **禁区 100% 遵守**：`split_manager.py` 与 `copy_reference.cjs` git ls-files 确认仍在；i18n 冻结文件 diff 零命中；growth 未跟踪 handoff 零出现在 diff ✓
3. **行为零变化**：`cargo check --workspace` 与 `cargo check -p northhing` 0 error（仅 pre-existing warnings）；`check-core-boundaries` / `check-repo-hygiene` / `check:rot` 全 PASS ✓

**QUALITY**：**PASS**（独立命令实跑复现报告 §5 输出）

---

## 独立验证命令实跑

### 1. 删除清单精确匹配

```text
$ git diff --name-only --diff-filter=D 9970c6a..c8868fe
package-lock.json
scripts/analyze_r16_structure.py
scripts/cleanup_r16.py
scripts/find_callers.py
scripts/legacy-prefix.py
scripts/make_helpers_pub_super.py
scripts/r17_split.py
scripts/rename-to-northhing.py
scripts/rename-to-northing.py
scripts/split_exec_engine.py
```
→ 9 脚本 + package-lock.json，与 brief 列举精确一致。

### 2. 禁区留存

```text
$ git ls-files scripts/split_manager.py scripts/copy_reference.cjs
scripts/copy_reference.cjs
scripts/split_manager.py
```
→ 两个文件均在 git 跟踪。

### 3. growth handoff 零触碰

```text
$ git diff --name-only 9970c6a..c8868fe | rg "handoff-g2-t9"
(空)
$ git ls-files | rg -i "handoff-g2-t9"
(空)
```
→ diff 中零出现，git 跟踪中零出现。

### 4. i18n 冻结文件零触碰

```text
$ git diff --name-only 9970c6a..c8868fe | rg "locales\.json|i18n-dynamic-key-allowlist"
(空)
```
→ 零 diff。

### 5. 大写 I `northhing-Installer` 残余清查

```text
$ git grep -n --fixed-strings "northhing-Installer"
scripts/i18n-dynamic-key-allowlist.json:7
scripts/i18n-dynamic-key-allowlist.json:26
src/shared/i18n/contract/locales.json:37
```
→ 3 处命中，**全部在 i18n 冻结区**（brief 第 18 行豁免）。工作流、Cargo.toml、CI workflows 零残留。

### 6. docs/design 5 目录归档结果

```text
$ git ls-files docs/design/
docs/design/2026-07-25-k4a-desktop-facade.md  ← 必须保留
docs/design/2026-08-05-memory-architecture-research/codex-anthropic-memory-research.md  ← 必须保留
```

```text
$ git ls-files docs/archive/design/ | rg "^docs/archive/design/2026-07-(22|23|25|31)"
docs/archive/design/2026-07-22-agent-centric/
docs/archive/design/2026-07-22-frontend-redesign/
docs/archive/design/2026-07-23-self-cognition/
docs/archive/design/2026-07-25-judge-mom/
docs/archive/design/2026-07-31-orchestrator-system-prompt/
```
→ 5 目录精确归档 + 2 项精确保留。

### 7. git log --follow 历史保留

```text
$ git log --oneline --follow -- docs/archive/design/2026-07-22-frontend-redesign/oklch-to-srgb.py
c8868fe refactor(cleanup): sweep one-off scripts, ...
9946da9 feat(frontend-redesign): FR-T1 tokens translation ...
```
→ rename 被检测。

### 8. cargo check 实跑

```text
$ rustup run stable-msvc cargo check --workspace
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.27s
(18 warnings, 0 errors — pre-existing, 与本任务无关)
```

```text
$ rustup run stable-msvc cargo check -p northhing
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.77s
(0 errors)
```

### 9. repo 脚本实跑

```text
$ node scripts/check-core-boundaries.mjs
Core boundary check passed.

$ node scripts/check-repo-hygiene.mjs
Repository hygiene check passed (2 content files scanned, 3346 filenames checked).

$ pnpm run check:rot
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1363 files).
```

---

## 发现（按严重度）

### Important

#### **I-1** — SPEC 4 引用完整性：`docs/design/2026-07-22-frontend-redesign/` 被错误归档

**事实**：16 个活文档/代码文件引用 `docs/design/2026-07-22-frontend-redesign/...`，归档后路径全部 broken。

活引用文件清单（`git grep -l --fixed-strings "docs/design/2026-07-22-frontend-redesign"` 排除 archive/handoffs/.superpowers）：

```text
.ohmyagent/skills/northhing-slint-desktop/SKILL.md          ← 活 skill 文档
.opencode/model-capability-notes.md                          ← 活编排者台账（每 session 回填）
docs/plans/2026-07-22-frontend-redesign-plan.md              ← 活 plan
docs/superpowers/plans/2026-07-27-fr-t3a-token-low-complexity.md  ← 活 plan（含 python 命令引用）
docs/superpowers/plans/2026-07-28-fr-t4-layout-migration.md  ← 活 plan
docs/superpowers/plans/2026-07-29-fr-t5-settings-drawers.md  ← 活 plan
exploration-frontend-product_20260724.md                     ← 探索文档
src/apps/desktop/src/ui/components/DeckBar.slint             ← 代码注释（设计真值）
src/apps/desktop/src/ui/components/SegmentedControl.slint    ← 代码注释
src/apps/desktop/src/ui/redesign_palette.slint               ← 代码注释（含 OKLCH 生成器路径）
src/apps/desktop/src/ui/views/AccessSettingsPanel.slint     ← 代码注释
src/apps/desktop/src/ui/views/ArchiveView.slint             ← 代码注释
src/apps/desktop/src/ui/views/GeneralSettingsPanel.slint    ← 代码注释
src/apps/desktop/src/ui/views/MCPSettingsPanel.slint        ← 代码注释
src/apps/desktop/src/ui/views/ProviderSettingsPanel.slint   ← 代码注释
src/apps/desktop/src/ui/views/SkillsSettingsPanel.slint     ← 代码注释
```

特别严重：`.opencode/model-capability-notes.md:35` 引用 `slint-feasibility-consult-room.md`，归档后编排者回填台账时按注释找文件会 broken。

**违反约束**：
- brief 第 16 行："有活引用的留下并在 report 说明"
- 我的任务约束第 9 行："引用必须同步改或该目录不该动——逐个核实实现者的复用侦察声称"

**报告 §3.3 的误判**（"仅 `.slint` 文件的设计真值注释与历史 handoffs/plans 提及，无构建/编译逻辑依赖"）错把判断标准设为"编译依赖"，而 brief 标准是"活文档引用"。

---

#### **I-2** — SPEC 4 引用完整性：`docs/design/2026-07-23-self-cognition/` 被错误归档

**事实**：6 个活文档引用，归档后路径 broken。

活引用文件清单：

```text
docs/status/audit-p2-debt_20260727.md                       ← 活 ledger
docs/status/exploration-arch-product_20260729.md            ← 活状态文档
docs/status/tech-debt-ledger.md                             ← 核心 tech-debt ledger (AGENTS.md "Doc sync as hard rule" 引用)
exploration-frontend-product_20260724.md                    ← 探索文档
exploration-governance-debt_20260724.md                     ← 探索文档
exploration-self-cognition_20260724.md                      ← 探索文档
```

特别严重：`docs/status/tech-debt-ledger.md:180` 引用 `first-entry-design.md` 作为 P2-13 解决方案的引用源；这是核心活 ledger，归档后引用 broken。

**报告 §3.3 误判**为"仅技术债台账历史解决记录与历史 handoffs 提及"——但 ledger 是**活文档**，非"历史 handoff"。

---

#### **I-3** — 可执行命令路径 broken（I-1 子集但单独标）

`docs/superpowers/plans/2026-07-27-fr-t3a-token-low-complexity.md:65` 嵌入：

```text
Run: `python docs/design/2026-07-22-frontend-redesign/oklch-to-srgb.py`
```

归档后文件在 `docs/archive/design/2026-07-22-frontend-redesign/oklch-to-srgb.py`，命令路径 broken。

`src/apps/desktop/src/ui/redesign_palette.slint:6` 同样引用 `oklch-to-srgb.py` 作为生成器。

---

### Minor

#### **M-1** — brief 文字瑕疵

brief 第 13 行写"删这 10 个"但实际列举了 9 个脚本 + 1 个核销（`replace_theme.py`），Spec 1 也写"删 10 个一次性脚本"。实际操作 `git rm` 9 脚本 + 1 package-lock.json 与报告 §2 一致"9 删除 + 1 核销"。这是 brief 自身文字问题，**未影响实现**。

#### **M-2** — 报告 §3.3 复用侦察判断标准错误

brief 标准是"活文档引用"（排除 archive/handoffs/.superpowers），实施者实际用"编译依赖"判断。结果漏报 22 个活引用文件。修复 I-1/I-2 后此 finding 自动消化。

---

## 修复建议（给 fixer）

实施者需要为 I-1 / I-2 选一条合规路径：

**路径 A（sed 批量替换）**——推荐，影响范围可控：
- 在 22 个活引用文件中 `sed` 替换 `docs/design/2026-07-22-frontend-redesign` → `docs/archive/design/2026-07-22-frontend-redesign`
- 同样对 `docs/design/2026-07-23-self-cognition`
- 排除 docs/archive/design/ 自身（已在新路径）
- 但这会产生 22 个文件的进一步 diff；commit message 需说明

**路径 B（`git restore` 回滚两个目录）**——更保守：
- `git mv docs/archive/design/2026-07-22-frontend-redesign docs/design/`
- `git mv docs/archive/design/2026-07-23-self-cognition docs/design/`
- 在 docs/archive/design/ 删空子目录

**路径 A 的额外考量**：22 个文件 diff 会让 commit 不再"纯清理"，但合规性更稳。

**Fixer 决定权**：让 fixer 自行选择。两种都能消除 finding。

---

## 总结

- **SPEC**：6 PASS / 1 FAIL（SPEC 4 引用完整性）
- **QUALITY**：PASS（git history、禁区、行为零变化、验证全绿）
- **FINDINGS**：3 Important（实为 2 个目录违反 × 1 个子集）+ 2 Minor
- **决策**：**REJECTED** — 返回修复，fixer 处理 I-1/I-2（路径 A 或 B），M-1/M-2 自动消化