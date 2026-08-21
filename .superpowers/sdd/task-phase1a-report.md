# Task PHASE-1A Report — 一次性清扫批（删除 + 命名 + 归档）

## 1. 任务元信息

- **Worktree**: `E:\agent-project\.worktrees\northing-p1a`
- **Branch**: `feat/phase1a-sweep-0821`
- **BASE**: `9970c6a`
- **HEAD**: `c8868fe`
- **Commit**: `c8868fe refactor(cleanup): sweep one-off scripts, package-lock, design archives and fix installer doc paths (PHASE-1A)`

---

## 2. Spec 逐条达成

| Spec | 要求 | 落地情况 | 证据/说明 |
|---|---|---|---|
| **Spec 1** | 删 10 个一次性脚本 | ✅ 完成（9 删除 + 1 核销） | `git rm` 9 个脚本；`replace_theme.py` 预检已不存在核销；`split_manager.py` 与 `copy_reference.cjs` 严格保留 |
| **Spec 2** | 删根 package-lock.json | ✅ 完成 | `git rm package-lock.json`；rg 复核全仓 CI/脚本零引用；`pnpm-lock.yaml` 保持 |
| **Spec 3** | .handoffs 已跟踪文件迁移 | ✅ 完成 | `git mv .handoffs/review-commit-997e14e_20260717.md docs/handoffs/`；未跟踪文件不碰 |
| **Spec 4** | docs/design 五目录归档 | ✅ 完成 | `git mv` 5 个七月过程稿目录入 `docs/archive/design/`；保留 `2026-08-05-memory-architecture-research` 与 `2026-07-25-k4a-desktop-facade.md` |
| **Spec 5** | nightly.yml 路径修正 + 残余清扫 | ✅ 完成 | `.github/workflows/nightly.yml:201` 路径 `northhing-Installer/` → `northing-installer/` |
| **Spec 6** | M3 文档命名修正（白名单） | ✅ 完成 | 修正白名单文档中 `northhing-Installer` 类路径引用；冻结工程（`locales.json` / `allowlist.json`）零触碰 |
| **Spec 7** | 顺手禁区清单重申 | ✅ 遵守 | `split_manager.py` 留存 / `copy_reference.cjs` 留存 / growth 文件未碰 / i18n 冻结数据未改 / worktree/target 未做 |

---

## 3. 复用侦察（Reuse Reconnaissance）

动手前对全仓进行 `rg --fixed-strings` 复核：

### 3.1 一次性脚本侦察

| 目标文件 | `rg` 命中情况 | 判定 |
|---|---|---|
| `scripts/analyze_r16_structure.py` | 仅 `docs/archive/handoffs/` 历史文档提及 1 处 | 无活引用，已删 |
| `scripts/cleanup_r16.py` | 0 命中 | 无活引用，已删 |
| `scripts/find_callers.py` | 0 命中 | 无活引用，已删 |
| `scripts/legacy-prefix.py` | 仅历史 handoff/review 提及 2 处 | 无活引用，已删 |
| `scripts/make_helpers_pub_super.py` | 0 命中 | 无活引用，已删 |
| `scripts/r17_split.py` | 0 命中 | 无活引用，已删 |
| `scripts/split_exec_engine.py` | 仅历史 handoff 提及 2 处 | 无活引用，已删 |
| `scripts/rename-to-northhing.py` | 历史 review / handoff / full-review 提及 | 无活引用，已删 |
| `scripts/rename-to-northing.py` | 历史 review / handoff / full-review 提及 | 无活引用，已删 |
| `scripts/replace_theme.py` | 仅 `docs/status/full-review-2026-08-16.md` 提及 | 文件已不存在，核销 |
| **排除** `scripts/split_manager.py` | 历史 handoff 提及 | 保守留存（未删） |
| **排除** `scripts/copy_reference.cjs` | `scripts/write_handoff.cjs:76,137` 活跃引用 | 活引用，严格留存（未删） |

### 3.2 `package-lock.json` 侦察

`rg --fixed-strings "package-lock.json"`：
- 仅在 builtin_skills 文档中提及作为通用 npm fallback 说明。
- CI workflows、`package.json`、`scripts/` 中零引用。
- `package-lock.json` 安全删除。

### 3.3 `docs/design` 5 目录活引用侦察

- `2026-07-22-agent-centric`: 仅同批归档目录 `2026-07-22-frontend-redesign` 与 `docs/archive/handoffs/` 提及，无外部代码/脚本活引用。
- `2026-07-22-frontend-redesign`: 仅 `.slint` 文件的设计真值注释与历史 handoffs/plans 提及，无构建/编译逻辑依赖。
- `2026-07-23-self-cognition`: 仅技术债台账历史解决记录与历史 handoffs 提及，无外部活引用。
- `2026-07-25-judge-mom`: 仅历史 handoff 提及 1 处，无外部活引用。
- `2026-07-31-orchestrator-system-prompt`: 0 外部引用。
- **保留项**：`2026-08-05-memory-architecture-research` 与 `2026-07-25-k4a-desktop-facade.md`（`docs/architecture/agent-kernel-northstar.md` 活跃引用）完好保留在 `docs/design/`。

---

## 4. 核销项证据（Cancelled Items & Evidence）

- **`scripts/replace_theme.py`**：
  - 预检结论：已不存在。
  - 实测证据：`rg --fixed-strings "replace_theme.py"` 仅在 `docs/status/full-review-2026-08-16.md` 出现，`Get-ChildItem -Path scripts/ -Filter "*replace_theme*"` 返回空。
  - 处置：核销并记录证据。

---

## 5. 验证输出

### 5.1 `cargo check --workspace`
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
输出：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.46s
```
（无任何 error，全 workspace 编译通过）

### 5.2 `cargo check -p northhing`
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
输出：
```text
    Checking northhing v0.2.10 (E:\agent-project\.worktrees\northing-p1a\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 59s
```

### 5.3 `node scripts/check-core-boundaries.mjs`
```powershell
node scripts/check-core-boundaries.mjs
```
输出：
```text
Core boundary check passed.
```

### 5.4 `node scripts/check-repo-hygiene.mjs`
```powershell
node scripts/check-repo-hygiene.mjs
```
输出：
```text
Repository hygiene check passed (202 content files scanned, 3345 filenames checked).
```

### 5.5 `pnpm run check:rot`
```powershell
pnpm run check:rot
```
输出：
```text
✔ compliant fixture exits 0 and reports success (109.2562ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (100.0491ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (104.0884ms)
✔ registered god-file exceeding ceiling fails (6.3193ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (7.7486ms)
✔ actual workspace rot budget passes with current manifest (427.2035ms)
ℹ tests 6
ℹ suites 0
ℹ pass 6
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 761.8047
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1363 files).
```

### 5.6 `git diff --stat 9970c6a..c8868fe`
```text
 .github/workflows/nightly.yml                      |    2 +-
 AGENTS-CN.md                                       |    8 +-
 docs/architecture/i18n.md                          |    6 +-
 .../northing-design-philosophy.md                  |    0
 .../northing-design-spec-v1.md                     |    0
 .../northing-frontend-design-handoff.md            |    0
 .../northing-home-v1-final.html                    |    0
 .../2026-07-22-agent-centric/northing-home-v1.html |    0
 .../northing-lockup-color.svg                      |    0
 .../2026-07-22-agent-centric/northing-logo.svg     |    0
 .../northing-self-cognition-chronicle.html         |    0
 .../northing-ui-expanded.html                      |    0
 .../HANDOFF-2026-07-30.md                          |    2 +-
 .../audit-fr-t3-blockers_20260727.md               |    0
 .../consult-room/DELIVERY-NOTES.md                 |    0
 .../consult-room/PAGES-BRIEF.md                    |    0
 .../consult-room/PANELS-BRIEF.md                   |    0
 .../consult-room/TRIGGER-BRIEF.md                  |    0
 .../consult-room/consult-room-archive-v2-report.md |    0
 .../consult-room/consult-room-archive-v2.html      |    0
 .../consult-room/consult-room-main.html            |    0
 .../consult-room-onboarding-v2-report.md           |    0
 .../consult-room/consult-room-onboarding-v2.html   |    0
 .../consult-room-settings-v2-report.md             |    0
 .../consult-room/consult-room-settings-v2.html     |    0
 .../consult-room/consult-room-space-v2-report.md   |    2 +-
 .../consult-room/consult-room-space-v2.html        |    0
 .../consult-room/consult-room-v3.html              |    0
 ...consult-room-v4-trigger-gemini-31-pro-report.md |    0
 .../consult-room-v4-trigger-gemini-31-pro.html     |    0
 .../consult-room-v4-trigger-step-explore-report.md |    2 +-
 .../consult-room-v4-trigger-step-explore.html      |    0
 .../consult-room/consult-room-v4.html              |    0
 .../consult-room/gemini-31-pro-settings-report.md  |    0
 .../consult-room/gemini-31-pro-settings.html       |    0
 .../gemini-36-flash-onboarding-report.md           |    0
 .../consult-room/gemini-36-flash-onboarding.html   |    0
 .../consult-room/minimax-m3-archive-report.md      |    0
 .../consult-room/minimax-m3-archive.html           |    0
 .../consult-room/step-explore-space-report.md      |    0
 .../consult-room/step-explore-space.html           |    0
 .../exploration-frontend-audit_20260729.md         |    0
 .../fonts/Fraunces-Italic.woff2                    |  Bin
 .../fonts/Fraunces.woff2                           |  Bin
 .../fonts/JetBrainsMono.woff2                      |  Bin
 .../fonts/NotoSansSC-subset.woff2                  |  Bin
 .../fonts/NotoSansSC-var.woff2                     |  Bin
 .../northing-design-philosophy.md                  |    0
 .../northing-frontend-design-handoff.md            |    0
 .../northing-home-v1-final.html                    |    0
 .../northing-self-cognition-chronicle.html         |    0
 .../northing-ui-expanded.html                      |    0
 .../2026-07-22-frontend-redesign/oklch-to-srgb.py  |    0
 .../prototypes/3.1pro/agent-core-dashboard.html    |    0
 .../prototypes/3.1pro/agent-core-monolith.html     |    0
 .../prototypes/3.1pro/agent-deai-canvas.html       |    0
 .../prototypes/3.1pro/agent-deai-ticket.html       |    0
 .../3.1pro/agent-functional-workbench.html         |    0
 .../prototypes/3.1pro/agent-layout-telemetry.html  |    0
 .../prototypes/3.1pro/agent-layout-tiling.html     |    0
 .../prototypes/3.1pro/agent-membrane.html          |    0
 .../prototypes/3.1pro/agent-notebook.html          |    0
 .../prototypes/3.1pro/chat-refined.html            |    0
 .../prototypes/3.1pro/chat-style-doc.html          |    0
 .../prototypes/3.1pro/style-counseling-rust.html   |    0
 .../prototypes/3.1pro/style-evolution-aura.html    |    0
 .../prototypes/3.1pro/style-evolution-node.html    |    0
 .../prototypes/3.1pro/style-evolution-zen.html     |    0
 .../prototypes/3.1pro/style-philosophy-abyss.html  |    0
 .../3.1pro/style-philosophy-clinical.html          |    0
 .../prototypes/JUDGE-CRITERIA.md                   |    0
 .../prototypes/README.md                           |    0
 .../prototypes/_review/FINAL-REPORT.md             |    0
 .../prototypes/_review/agent-A-jobs-review.md      |    0
 .../prototypes/_review/agent-B-quant-scoring.md    |    0
 .../prototypes/_review/agent-V-verifier-report.md  |    0
 .../_review/design-philosophy-distilled.md         |    0
 .../prototypes/_review/mavis-demo/README.md        |    0
 .../prototypes/_review/mavis-demo/chat-v2.html     |    0
 .../prototypes/_review/mavis-demo/chat-v3.html     |    0
 .../prototypes/_review/mavis-demo/chat-v4.html     |    0
 .../prototypes/_review/mavis-demo/chat-v5.html     |    0
 .../prototypes/_review/mavis-demo/chat-v6.html     |    0
 .../prototypes/_review/mavis-demo/chat-v7.html     |    0
 .../prototypes/_review/mavis-demo/chat.html        |    0
 .../_review/mavis-demo/critique-r1-philosophy.md   |    0
 .../_review/mavis-demo/critique-r1-ux.md           |    0
 .../_review/mavis-demo/critique-r1-visual.md       |    0
 .../_review/mavis-demo/critique-r2-philosophy.md   |    0
 .../_review/mavis-demo/critique-r2-ux.md           |    0
 .../_review/mavis-demo/critique-r2-visual.md       |    0
 .../_review/mavis-demo/critique-r3-final.md        |    0
 .../_review/mavis-demo/critique-r4-final.md        |    0
 .../_review/mavis-demo/critique-r5-geometry.md     |    0
 .../_review/mavis-demo/critique-r6-dark.md         |    0
 .../prototypes/archive.html                        |    0
 .../prototypes/bakeoff-20260802/BRIEF.md           |    0
 .../bakeoff-20260802/gemini-36-flash-report.md     |    0
 .../bakeoff-20260802/gemini-36-flash.html          |    0
 .../prototypes/bakeoff-20260802/kimi-k3-report.md  |    0
 .../prototypes/bakeoff-20260802/kimi-k3.html       |    0
 .../prototypes/bakeoff-20260802/minimax-m3-report.md|    0
 .../prototypes/bakeoff-20260802/minimax-m3.html    |    0
 .../prototypes/bakeoff-20260802/qwen-report.md     |    0
 .../prototypes/bakeoff-20260802/qwen.html          |    0
 .../prototypes/chat-collapsed.html                 |    0
 .../prototypes/chat-expanded.html                  |    0
 .../prototypes/empty-state.html                    |    0
 .../prototypes/identity-creator.html               |    0
 .../prototypes/onboarding.html                     |    0
 .../prototypes/settings-access.html                |    0
 .../prototypes/settings-general.html               |    0
 .../prototypes/settings-mcp.html                   |    0
 .../prototypes/settings-models.html                |    0
 .../prototypes/settings-workspace-skills.html      |    0
 .../prototypes/shared/animations.css               |    0
 .../prototypes/shared/components.css               |    0
 .../prototypes/shared/layout.css                   |    0
 .../prototypes/shared/theme-switch.js              |    0
 .../prototypes/shared/tokens.css                   |    0
 .../prototypes/slint-safe-conventions.md           |    0
 .../prototypes/space-view.html                     |    0
 .../styles-bakeoff-20260802/STYLE-BRIEF.md         |    0
 .../gemini-36-flash-slate-report.md                |    0
 .../gemini-36-flash-slate.html                     |    0
 .../styles-bakeoff-20260802/kimi-k3-ink-report.md  |    0
 .../styles-bakeoff-20260802/kimi-k3-ink.html       |    0
 .../minimax-m3-lithic-report.md                    |    0
 .../styles-bakeoff-20260802/minimax-m3-lithic.html |    0
 .../styles-bakeoff-20260802/qwen-abyss-report.md   |    0
 .../styles-bakeoff-20260802/qwen-abyss.html        |    0
 .../styles2-bakeoff-20260802/STYLES2-BRIEF.md      |    0
 .../gemini-36-flash-cyber-report.md                |    0
 .../gemini-36-flash-cyber.html                     |    0
 .../kimi-k3-glitch-report.md                       |    0
 .../styles2-bakeoff-20260802/kimi-k3-glitch.html   |    0
 .../minimax-m3-collage-report.md                   |    0
 .../minimax-m3-collage.html                        |    0
 .../qwen-memphis-report.md                         |    0
 .../styles2-bakeoff-20260802/qwen-memphis.html     |    0
 .../styles3-bakeoff-20260802/STYLES3-BRIEF.md      |    0
 .../gemini-36-flash-darkroom-report.md             |    0
 .../gemini-36-flash-darkroom.html                  |    0
 .../kimi-k3-kintsugi-report.md                     |    0
 .../styles3-bakeoff-20260802/kimi-k3-kintsugi.html |    0
 .../minimax-m3-noir-report.md                      |    0
 .../styles3-bakeoff-20260802/minimax-m3-noir.html  |    0
 .../styles3-bakeoff-20260802/qwen-eink-report.md   |    0
 .../styles3-bakeoff-20260802/qwen-eink.html        |    0
 .../step-explore-blueprint-report.md               |    0
 .../step-explore-blueprint.html                    |    0
 .../styles4-bakeoff-20260802/FREE-BRIEF.md         |    0
 .../gemini-36-flash-deco-report.md                 |    0
 .../gemini-36-flash-deco.html                      |    0
 .../gemini-36-flash-moss-report.md                 |    0
 .../gemini-36-flash-moss.html                      |    0
 .../gemini-36-flash-weaving-report.md              |    0
 .../gemini-36-flash-weaving.html                   |    0
 .../kimi-k3-stainedglass-report.md                 |    0
 .../kimi-k3-stainedglass.html                      |    0
 .../kimi-k3-swiss-report.md                        |    0
 .../styles4-bakeoff-20260802/kimi-k3-swiss.html    |    0
 .../kimi-k3-woodblock-report.md                    |    0
 .../kimi-k3-woodblock.html                         |    0
 .../minimax-m3-ceramics-report.md                  |    0
 .../minimax-m3-ceramics.html                       |    0
 .../minimax-m3-pixel-report.md                     |    0
 .../styles4-bakeoff-20260802/minimax-m3-pixel.html |    0
 .../minimax-m3-zen-report.md                       |    0
 .../styles4-bakeoff-20260802/minimax-m3-zen.html   |    0
 .../styles4-bakeoff-20260802/qwen-destijl.html     |    0
 .../styles4-bakeoff-20260802/qwen-starchart.html   |    0
 .../styles4-bakeoff-20260802/qwen-ukiyo.html       |    0
 .../step-explore-bauhaus-report.md                 |    0
 .../step-explore-bauhaus.html                      |    0
 .../step-explore-inkwash-report.md                 |    0
 .../step-explore-inkwash.html                      |    0
 .../step-explore-nouveau-report.md                 |    0
 .../step-explore-nouveau.html                      |    0
 .../styles5-bakeoff-20260802/MULTI-BRIEF.md        |    0
 .../gemini-31-pro-prism-report.md                  |    0
 .../gemini-31-pro-prism.html                       |    0
 .../gemini-36-flash-stage-report.md                |    0
 .../gemini-36-flash-stage.html                     |    0
 .../prototypes/theme-system.html                   |    0
 .../redesign-v2-plan.md                            |    2 +-
 .../slint-feasibility-poc.md                       |    0
 .../slint-retarget-notes.md                        |    0
 .../2026-07-22-frontend-redesign/tokens-draft.css  |    0
 .../tokens-srgb-table.md                           |    0
 .../visual-iter-compass_20260802.md                |    0
 .../first-entry-design.md                          |    0
 .../memory-multi-agent-architecture.md             |    0
 .../memory-retrieval-design.md                     |    0
 .../2026-07-25-judge-mom/memory-growth-design.md   |    0
 .../orchestrator-system-prompt.md                  |    0
 docs/development/i18n.md                           |    2 +-
 .../handoffs}/review-commit-997e14e_20260717.md    |    0
 northing-installer/AGENTS-CN.md                    |   22 +-
 northing-installer/AGENTS.md                       |   22 +-
 northing-installer/README.md                       |    8 +-
 package-lock.json                                  | 3680 --------------------
 scripts/analyze_r16_structure.py                   |  228 --
 scripts/cleanup_r16.py                             |   41 -
 scripts/find_callers.py                            |   62 -
 scripts/legacy-prefix.py                           |   50 -
 scripts/make_helpers_pub_super.py                  |  138 -
 scripts/r17_split.py                               |  585 ----
 scripts/rename-to-northhing.py                     |  164 -
 scripts/rename-to-northing.py                      |  189 -
 scripts/split_exec_engine.py                       |  501 ---
 211 files changed, 39 insertions(+), 5677 deletions(-)
```

### 5.7 关键删除项全仓残留验证
- `analyze_r16_structure.py`: 0 活引用（仅历史 handoff 提及）
- `cleanup_r16.py`: 0 命中
- `find_callers.py`: 0 命中
- `legacy-prefix.py`: 0 活引用（仅历史 handoff/review 提及）
- `make_helpers_pub_super.py`: 0 命中
- `r17_split.py`: 0 命中
- `split_exec_engine.py`: 0 活引用（仅历史 handoff 提及）
- `rename-to-northhing.py`: 0 活引用（仅历史 review/handoff/full-review 提及）
- `rename-to-northing.py`: 0 活引用（仅历史 review/handoff/full-review 提及）
- `replace_theme.py`: 0 活引用（仅历史 full-review 提及）
- `package-lock.json`: 0 活引用

---

## 6. 偏离与声明

- 零未授权偏离。
- 零行为修改（纯清理、归档与文档路径修正）。
- 所有约束与禁区（`split_manager.py`、`copy_reference.cjs`、growth 未跟踪文件、i18n 冻结数据、worktree 与 target 边界）均 100% 遵守。
