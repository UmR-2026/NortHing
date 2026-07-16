# northing project — AGENTS.md

**Generated**: 2026-07-16 02:55 (auto from mavis memory migration to BitFun)
**Source**: mavis `agents/mavis/memory/` 4 northing .md files (consolidated)
**Tag**: v0.1.0 at facc9c3 on main
**Status**: clean working tree, only `.loop-worktrees/` untracked

> BitFun Agent: 本文件是 northing 项目的根级指令文件。每次任务开始时, 优先读这份 AGENTS.md + `.loop-worktrees/` 当前内容。

---

## 0. Project 现状 (2026-07-15 v0.1.0)

**14 commits on main, tag v0.1.0**:
- B3-T6 cargo fmt (47 文件)
- B2 handoff + v0.1.0 roadmap
- model_config_form 拆分 (1058→4 子模块)
- chat/render 拆分 (983→7 子文件)
- question 拆分 (803→3 子模块)
- QClaw review 8.2/10 SHIP

**Toolchain**: rustup `stable-x86_64-pc-windows-gnu` + MSYS2 gcc 16.1.0-5 (broken) + LLVM clang 22.1.7

**Blocker**: C5 `cargo test --workspace` runtime 0xC0000139 STATUS_ENTRYPOINT_NOT_FOUND (mingw 整体错位)

---

## 1. 重要 lessons (跨 session, 必读)

### 1.1 Mavis producer-boundary (2026-07-15 push-back)
**Mavis 写 spec + dispatch, subagent 写 .rs. 不写代码。** 这次 session 大量手写 code 违反了这个守则. 后续 M3 take-over 才 5-15 min/file 合理。

### 1.2 Subagent dispatch 失败时 Mavis 不要自己硬上
上下文会爆. Mavis 应该派 subagent 处理 MSVC + tests, 不要自己 diagnostics. 这次连续 5+ 个 subagent call aborted 后我自己干, 浪费大量上下文。

### 1.3 Token Plan 上限
30 turn 后开始消耗 token plan 限速. 下次开 session 写新 plan 之前先 batch 起来。

### 1.4 OpenCode task tool 限制
1 message 最多 2 个 `task` calls, 3+ abort. 之前派了 2 个, 都 aborted. 实际有效 = 1 by 1 sequential。

### 1.5 MSVC 切换细节
- `rustup override set <toolchain>` 只对当前目录生效
- `cargo clean` 不清 `target-shared/`, 跨项目共享. cross-project 用 `target-shared` 别名
- `gcc` 在 MSYS2 路径里直接跑 OK, 但被 `cc` crate 调用挂 — 神秘 (PATH? DLL? argv encoding?)

---

## 2. C5 Blocker 修法 (2 选 1, user pick)

### 修法 A: Fix gcc
用 MSYS2 重装/更新 gcc, 然后 GNU toolchain 跑 `cargo test --workspace`:
```bash
C:\msys64\usr\bin\bash.exe -c "pacman -Syu mingw-w64-x86_64-gcc --noconfirm"
```

### 修法 B: Make onig/QuickJS optional
砍掉 `readability-js` 依赖, 把 rquickjs-sys 变 optional feature (CLI 核心不依赖这两个)。

### MSVC env setup (verified work)
```cmd
cmd /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=amd64'
```

---

## 3. Next session 第一步

1. 选一个修法 (fix gcc 或 make optional)
2. 派 1 个 subagent 做, Mavis 留 context review
3. 修通后 commit + re-tag v0.1.0
4. 跑 `cargo test --workspace`, capture pass count
5. QClaw review (已经在 `docs/superpowers/specs/2026-07-15-qclaw-review-spec.md`, dispatch QClaw agent)
6. 修 QClaw findings, 最终 tag v0.1.0 human-usable-final
7. 写 v0.1.0 release notes (`docs/releases/2026-07-15-v0.1.0-release.md`)
8. 推 GitHub (per user "0.1.0 人类可以使用后再上传")

---

## 4. 关键文件 (handoff 给新 session)

- `HANDOFF.md` §0 — 反映 v0.1.0 状态 (但 cron+worktree 等未更新)
- `README.md` — v0.1.0 human-usable 公开版
- `docs/plans/2026-07-15-v0.1.0-roadmap.md` — critical path plan
- `docs/superpowers/specs/2026-07-15-qclaw-review-spec.md` — review spec
- `docs/superpowers/reviews/2026-07-15-qclaw-review-report.md` — QClaw report
- `docs/handoffs/2026-07-15-b2-v2-source-recovery-c21-complete.md` — source recovery context

---

## 5. 关键 stash / untracked

- `stash@{0}` — MSVC 切换失败的 .cargo/config.toml + .cargo/cl-wrapper.bat 配置. **不需要 apply, 只是 record**
- `.loop-worktrees/b2-god-split-v2-20260715/` — b2 v2 worktree (user 之后 `git worktree remove`)

---

## 6. 不需要重新做的事

- model_config_form / chat/render / question 拆分 — 已 commit + QClaw approved
- B3-T6 cargo fmt — 已 commit
- v0.1.0 tag — 已打
- QClaw review — 已 done (8.2/10 → SHIP)

---

# B1 batch 2 Plan review guide + cleanup proposal (2026-07-14)

**Reviewer: marvis**
**Source**: Gemini Antigravity 跑完 B1 batch 2 plan

## 12 verify 维度 (Mavis spot-check)

1. **代码正确性**: 5 phase 实际 line count 跟 walkthrough 描述 (3 处不精确, 详见 `walkthrough-fix-suggestion-2026-07-14.md`)
2. **测试覆盖**: cargo test 34/34 pass (R75 batch 1 baseline 一致)
3. **clippy clean**: refactored files 0 warning
4. **line count 守则**: facade ≤50 (chat_render 27, theme 21, selectors 34 OK), biggest sub ≤350 (chat_render 345 OK 但接近 cap)
5. **visibility 迁移**: selectors 改 `pub(crate)` 解决 E0624 错误
6. **clippy fix 完整性**: 4 个 warning fix
7. **worktree 隔离**: theme + selectors 各在独立 branch, 没污染 main
8. **commit 格式**: 1+4+1 commit 跟 R75 batch 1 pattern 一致
9. **commit message 准确性**: 跟 walkthrough 描述基本对 (1 处 sub count 不精确)
10. **main working tree 干净**: 0 modified + 0 deleted
11. **Loop-engineering 集成**: AGENTS.md + .gitignore 1 commit
12. **不 commit handoff self**: Gemini 1+4+1 commit 都是 production code

## 9 verify steps (跟 R75 batch 1 模式)

1. `git log main -10` verify 1+4+1 commit 顺序对
2. `git log loop/B1-surface-theme-20260714 -5` verify theme worktree commit + merge
3. `git log loop/B1-surface-selectors-20260714 -5` verify selectors worktree commit
4. `cargo test -p northhing-cli` verify 34/34 pass
5. `git status --short main` verify 0 modified + 0 deleted
6. `git worktree list` verify 3 worktree + 1 旧 impl-b0-smoke
7. chat_render line count verify (mod.rs 27 + 7 sub, biggest message.rs 345)
8. theme line count verify (mod.rs 21 + 4 sub)
9. selectors line count verify (mod.rs 34 + 1 sub)

---

# Walkthrough 3 处不精确 fix 建议 (2026-07-14)

**Source**: `C:\Users\UmR\.gemini\antigravity\brain\dd71699c-1519-4487-8082-b4ebfabad176\walkthrough.md`

## 不精确 #1: chat_render sub count

**Walkthrough 描述**: "5 sub-modules"
**实际**: 7 sub (mod.rs 27 + header.rs 49 + message.rs 345 + message_helpers.rs 91 + messages.rs 268 + root.rs 139 + shortcuts.rs 145 + status.rs 81)
**Diff**: 7 sub 不是 5 sub. walkthrough 漏 2 sub (header + message_helpers), biggest sub 实际 345 lines 没标.
**Fix 建议**: walkthrough 改 "5 sub-modules" → "7 sub-modules", 加 "biggest sub: message.rs 345 lines"

## 不精确 #2: theme god split sub count

**Walkthrough 描述**: "8 sub-modules"
**实际**: 4 sub (mod.rs 21 + types.rs 83 + presets.rs 178 + detection.rs 180)
**Diff**: 4 sub 不是 8 sub
**Fix 建议**: walkthrough 改 "8 sub-modules" → "4 sub-modules"

## 不精确 #3: selectors sub count

**Walkthrough 描述**: (类似问题)
**实际**: 1 sub (mod.rs 34)
**Fix 建议**: 同步修正

---

# Archive Reference

历史详细 lesson 在 archive:
- `archive/northing-god-object-split-2026-07.md` (28 KB) — B2 重构详细
- `archive/northing-split-execution-2026-07.md` (15 KB) — B2 拆分执行详细

---

# Cross-reference (Mavis 在 BitFun 端)

- 跨项目守则: `C:\Users\UmR\AppData\Roaming\bitfun\data\rules\MEMORY.md`
- 流程 lesson: `C:\Users\UmR\AppData\Roaming\bitfun\data\rules\workflow-revised.md`
- 模型选型: `C:\Users\UmR\AppData\Roaming\bitfun\data\rules\model-evaluation.md`
- Windows gotchas: `C:\Users\UmR\AppData\Roaming\bitfun\data\rules\windows-powershell-gotchas.md`
- codegraph 用法: `C:\Users\UmR\AppData\Roaming\bitfun\data\rules\codegraph-workflow.md`
- 知识图谱: `C:\Users\UmR\AppData\Roaming\bitfun\.graph\`
- evolve 框架: `C:\Users\UmR\AppData\Roaming\bitfun\evolve\`
