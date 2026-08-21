# Task PHASE-1A Fix Report — 修复轮（设计真值移回 + 归档引用改写）

## 1. 修复元信息

- **Worktree**: `E:\agent-project\.worktrees\northing-p1a`
- **Branch**: `feat/phase1a-sweep-0821`
- **BASE**: `9970c6a`
- **Phase-1A Commit**: `c8868fe`
- **Fix Commit**: `e768ba3`
- **Commit Message**: `fix(cleanup): restore frontend-redesign to docs/design and update self-cognition references to archive (PHASE-1A-fix)`

---

## 2. 编排者裁定落地详情

| 问题项 | 裁定方案 | 实施动作 | 结果 |
|---|---|---|---|
| **I-1 / I-3** | **路径 B**：`frontend-redesign` 移回原位 | `git mv docs/archive/design/2026-07-22-frontend-redesign docs/design/` | 原型与 token 单一事实源、8 个 `.slint` 设计真值注释、`oklch-to-srgb.py` 脚本执行路径完全愈合 |
| **I-2** | **路径 A**：`self-cognition` 保持归档，更新 6 个活跃引用文件 | 批量修改 6 处活跃文档中路径指向 `docs/archive/design/2026-07-23-self-cognition/...` | 引用断裂完全修复，目录归档规范保持 |
| **禁区** | `.opencode/model-capability-notes.md` 保护 | 确认未包含 self-cognition 引用，零触碰 | growth 禁区 100% 遵守 |

### 2.1 改写的 6 个活跃引用文件列表
1. `docs/status/tech-debt-ledger.md:180`
2. `docs/status/audit-p2-debt_20260727.md:28`
3. `docs/status/exploration-arch-product_20260729.md:197`
4. `exploration-governance-debt_20260724.md:201`
5. `exploration-self-cognition_20260724.md:14, 466, 467, 468`
6. `exploration-frontend-product_20260724.md:485, 486, 487`

---

## 3. 引用复核（`rg` 实测证据）

### 3.1 `rg 'docs/design/2026-07-22-frontend-redesign'`
所有命中均指向实际存在的 `docs/design/2026-07-22-frontend-redesign/`，路径有效：
- `src/apps/desktop/src/ui/views/SkillsSettingsPanel.slint`
- `src/apps/desktop/src/ui/views/ProviderSettingsPanel.slint`
- `src/apps/desktop/src/ui/views/MCPSettingsPanel.slint`
- `src/apps/desktop/src/ui/views/GeneralSettingsPanel.slint`
- `src/apps/desktop/src/ui/views/ArchiveView.slint`
- `src/apps/desktop/src/ui/views/AccessSettingsPanel.slint`
- `src/apps/desktop/src/ui/redesign_palette.slint`
- `src/apps/desktop/src/ui/components/SegmentedControl.slint`
- `src/apps/desktop/src/ui/components/DeckBar.slint`
- `docs/superpowers/plans/2026-07-28-fr-t4-layout-migration.md`
- `docs/superpowers/plans/2026-07-27-fr-t3a-token-low-complexity.md`
- `docs/superpowers/plans/2026-07-29-fr-t5-settings-drawers.md`
- `docs/plans/2026-07-22-frontend-redesign-plan.md`
- `exploration-frontend-product_20260724.md`
- `docs/design/2026-07-22-frontend-redesign/oklch-to-srgb.py`
- `docs/design/2026-07-22-frontend-redesign/HANDOFF-2026-07-30.md`

### 3.2 `rg 'docs/design/2026-07-23-self-cognition'`
活跃工作区中 **0 命中**（仅历史归档 `docs/archive/handoffs/` 中有历史记录）。

### 3.3 `rg 'docs/archive/design/2026-07-23-self-cognition'`
命中 6 个活跃文档：
- `docs/status/tech-debt-ledger.md:180`
- `docs/status/audit-p2-debt_20260727.md:28`
- `docs/status/exploration-arch-product_20260729.md:197`
- `exploration-governance-debt_20260724.md:201`
- `exploration-self-cognition_20260724.md:14, 466, 467, 468`
- `exploration-frontend-product_20260724.md:485, 486, 487`

---

## 4. 验证命令与输出

### 4.1 `node scripts/check-repo-hygiene.mjs`
```powershell
node scripts/check-repo-hygiene.mjs
```
```text
Repository hygiene check passed (188 content files scanned, 3347 filenames checked).
```

### 4.2 `pnpm run check:rot`
```powershell
pnpm run check:rot
```
```text
✔ compliant fixture exits 0 and reports success (103.821ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (93.4703ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (100.4762ms)
✔ registered god-file exceeding ceiling fails (6.0248ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (6.7373ms)
✔ actual workspace rot budget passes with current manifest (342.3415ms)
ℹ tests 6
ℹ suites 0
ℹ pass 6
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 658.9626
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1363 files).
```

### 4.3 `node scripts/check-core-boundaries.mjs`
```powershell
node scripts/check-core-boundaries.mjs
```
```text
Core boundary check passed.
```

### 4.4 `cargo check --workspace`
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.22s
```

### 4.5 `cargo check -p northhing`
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.79s
```

---

## 5. 偏离与声明

- 零未授权偏离。严格按编排者裁定（I-1/I-3 走路径 B，I-2 走路径 A）执行完毕。
- 零行为逻辑变更。
