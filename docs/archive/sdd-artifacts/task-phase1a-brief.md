# Task PHASE-1A Brief — 一次性清扫批（删除 + 命名 + 归档）

## 来源与验收标准

来源：GLM-5.3 咨询方案 Phase 1（编排者按现状修正版）+ R-16/R-17 + T2-8 + PHASE-0 挂账 M2/M3。

**验收**：Spec 1-6 落地 + 验证输出进 report。

## 编排者预检结论（2026-08-21 实测，直接采信）

| 项 | 实证 |
|---|---|
| 一次性脚本 | 删这 10 个（全仓零活引用，仅历史文档提及）：`analyze_r16_structure.py` / `cleanup_r16.py` / `find_callers.py` / `legacy-prefix.py` / `make_helpers_pub_super.py` / `r17_split.py` / `split_exec_engine.py` / `rename-to-northhing.py` / `rename-to-northing.py`（+`replace_theme.py` 已不存在，核销）。**排除**：`split_manager.py`（出处不明，保守留）、`copy_reference.cjs`（**活的**：write_handoff.cjs:76,137 引用） |
| 锁文件 | 根 `package-lock.json` 删（pnpm 是唯一工具链，pnpm-lock.yaml 留）；删前 rg 确认 CI/脚本无 package-lock 引用 |
| .handoffs/ | 已跟踪仅 `review-commit-997e14e_20260717.md` → 移到 docs/handoffs/；**未跟踪的 `handoff-g2-t9-2026-08-07.md` 是 growth 并行 session 的，不许碰**（移动后目录若还剩它则目录保留） |
| docs/design 归档 | 5 个七月过程稿目录入 `docs/archive/design/`：`2026-07-22-agent-centric` / `2026-07-22-frontend-redesign` / `2026-07-23-self-cognition` / `2026-07-25-judge-mom` / `2026-07-31-orchestrator-system-prompt`。**保留**：`2026-08-05-memory-architecture-research`（近期研究）与 `2026-07-25-k4a-desktop-facade.md`（活引用——T2-9-B2 刚同步过它）。每个待移动目录先 rg 验证无活引用（排除 docs/archive、.superpowers、docs/handoffs），有活引用的留下并在 report 说明 |
| M2 真雷 | `.github/workflows/nightly.yml:201` upload-artifact 路径 `northhing-Installer/` → `northing-installer/`（顺手全文件 rg 一次 `northhing-Installer` 清残余） |
| M3 命名 | 文档群旧名引用修正：`docs/architecture/i18n.md`、`docs/development/i18n.md`、`northing-installer/AGENTS.md`、`AGENTS-CN.md`、`northing-installer/README.md` 中的 `northhing-Installer` 类错误路径。**禁区**：`src/shared/i18n/contract/locales.json` 与 `scripts/i18n-dynamic-key-allowlist.json` 属 i18n 冻结工程，不许动（改它们要触发再生成链） |
| worktree 清理 | **本任务不做**——两个候选分支各有 39/15 个未合并 commit，需用户拍板 |
| target 清理 | **本任务不做**——67.6GB 超红线，但 growth 并行 session 可能在编译，cargo clean 会撞车；归用户闲时执行 |

## 复用侦察（强制）

每个删除对象动手前 `rg --fixed-strings <name>` 全仓复核一次（含 .github/workflows、package.json、scripts 互引）。report 写「复用侦察」节。

## Spec（必须全部满足）

1. 删 10 个一次性脚本（git rm）。
2. 删根 package-lock.json（先 rg 验证无引用）。
3. .handoffs 已跟踪文件 git mv 到 docs/handoffs/。
4. docs/design 五目录归档（git mv 到 docs/archive/design/，逐目录先验证无活引用）。
5. nightly.yml 路径修正 + 全仓 `northhing-Installer` 残余清扫（冻结区除外）。
6. M3 文档命名修正（仅限上述白名单文件，逐处核对语义——是路径引用就改路径，是历史叙述则不动）。
7. 顺手禁区清单重申：split_manager.py / copy_reference.cjs / growth 未跟踪文件 / i18n 冻结数据文件 / worktree / target。

## Global Constraints（逐字遵守）

- 日志/注释 English-only、无 emoji。
- 纯删除/移动：零行为变化；任何"顺手改逻辑"的冲动压住。
- 发现预检与现状不符（某脚本其实有活引用等）→ 该项改核销写证据，不强行删。

## 验证（命令 + 输出都要进 report）

1. `cargo check --workspace`（MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`——确认删除无编译影响）
2. `node scripts/check-core-boundaries.mjs` + `node scripts/check-repo-hygiene.mjs` + `pnpm run check:rot`
3. `git diff --stat` + `rg --fixed-strings <每个删除名>` 残留零命中（逐一贴）

## 报告

`.superpowers/sdd/task-phase1a-report.md`：Spec 逐条、复用侦察节、核销项证据、验证输出、偏离声明。最后消息以状态词开头。

## 派发元信息

- BASE `9970c6a`；worktree `E:\agent-project\.worktrees\northing-p1a`（分支 `feat/phase1a-sweep-0821`）
- commit message 后缀 `(PHASE-1A)`；只 stage 你改的文件。
