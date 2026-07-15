# R44-R49 Plan Review Guide (Mavis-authored, 2026-07-07)

Reviewer: marvis (Mavis-authored draft) + user (APPROVE 8.5/10)
Status: APPROVED + 2 P2 fixed (commit `e7b41f69` on `fix/r40-r50-rework`)
Trigger: user 指令"你直接规划到49然后我去做review"

## Patch applied (2026-07-07 13:06)

**P2-1: `fmt=pass` → `fmt=na`**
- 39 task self-report lines updated across 6 yaml
- 原因: northing 项目无 rustfmt (`cargo fmt --check` not applicable)
- Apply: `replaceAll fmt=pass → fmt=na` 6 yaml

**P2-2: 动态 base commit**
- 39 task prompts updated: hardcoded `4841f3bd` → `current default branch HEAD at dispatch`
- Worker PREFLIGHT 必 `git rev-parse origin/main` 取当前 main HEAD
- sequential rounds: R44 from `4841f3bd`, R45+ from previous round's squash-merge commit
- Apply: `replaceAll ", from main 4841f3bd)" → ", from current default branch HEAD at dispatch — verify `git rev-parse origin/main` during PREFLIGHT; sequential rounds R45+ use previous round's squash-merge commit)"` 6 yaml

**Verify**: `fmt=na=39, fmt=pass=0, dynamic_base=39, hardcoded_4841f3bd=0` ✅

---

## 总览 (R40-R49 全清单)

| Round | Task count | Model split | Files | Status |
|---|---|---|---|---|
| R40 | 6/6 accept | 6 step-3.7-flash (piloted but M2.7 actually) | R40a-f (1811-1638 行) | ✅ done |
| R41 | 7/7 accept | 6 step-3.7-flash + 1 M2.7 | R41a-g (1630-1415 行) | ✅ done |
| R42 | 5/5 accept | 1 M2.7 + 4 step-3.7-flash | R42a/c-f (1373-1257 行) | ✅ done |
| **R43** | 6 task | 2 M2.7 + 4 step-3.7-flash | R43a/b/c/e/f/g (1255-1168 行), R43d SKIP | 🟢 running (plan_6d824461) |
| **R44** | 7 task | 3 M2.7 + 4 step-3.7-flash | R44a-g (1144-1005 行) | ⏳ ready |
| **R45** | 6 task | 1 M2.7 + 5 step-3.7-flash | R45a-f (1001-950 行) | ⏳ ready |
| **R46** | 7 task | 1 M2.7 + 6 step-3.7-flash | R46a-g (921-876 行) | ⏳ ready |
| **R47** | 7 task | 1 M2.7 + 6 step-3.7-flash | R47a-g (871-824 行) | ⏳ ready |
| **R48** | 7 task | 1 M2.7 + 6 step-3.7-flash | R48a-g (819-778 行) | ⏳ ready |
| **R49** | 5 task | 5 step-3.7-flash | R49a-e (766-720 行) | ⏳ ready |

**Total R44-R49: 39 task, 7 M2.7 + 32 step-3.7-flash**

---

## Model assignment rationale

### M2.7 (7 task — hard cases, prior partial follow-ups, cross-crate, prompt family):
- **R44d** acp/mod.rs 1082 (R22 partial follow-up, cross-crate)
- **R44e** session/mod.rs 1051 (R23 partial follow-up, most cross-crate module)
- **R44f** memory/embeddings.rs 1015 (R26 partial follow-up, pluggable backend)
- **R45d** human_input_impl.rs 960 (cross-crate: cli + desktop + tools)
- **R46e** acp/executor.rs 890 (cross-crate, state machine)
- **R47e** prompt/render.rs 835 (prompt family — R43g/R47e/R48b 共享 template engine)
- **R48b** prompt/builder.rs 818 (prompt family)

### step-3.7-flash (32 task — standalone files, no prior partial, isolated):
R44a/b/c/g, R45a/b/c/e/f, R46a/b/c/d/f/g, R47a/b/c/d/f/g, R48a/c/d/e/f/g, R49a-e (all)

### step-3.7-flash reasoning (R42 4/4 success, R41b 4/4 success):
- R40-R41 早期失败: prompt 没显式带 R40-R41 lessons
- R42 起: 每个 task prompt 显式 iron rules + sub-domain suggestion + cross-ref grep + PREFLIGHT commands + commit format, 4/4 成功
- step-3.7-flash 省 M2.7 额度 (user 偏好)

---

## 6 个 plan yaml 文件

| Round | YAML path | Lines (大致) |
|---|---|---|
| R44 | `docs/superpowers/plans/round44-7-way-parallel-mid-zone-p2-2026-07-07.yaml` | ~200 |
| R45 | `docs/superpowers/plans/round45-6-way-parallel-mid-zone-p3-2026-07-07.yaml` | ~180 |
| R46 | `docs/superpowers/plans/round46-7-way-parallel-low-zone-p1-2026-07-07.yaml` | ~190 |
| R47 | `docs/superpowers/plans/round47-7-way-parallel-low-zone-p2-2026-07-07.yaml` | ~190 |
| R48 | `docs/superpowers/plans/round48-7-way-parallel-low-zone-p3-2026-07-07.yaml` | ~190 |
| R49 | `docs/superpowers/plans/round49-5-way-parallel-tail-2026-07-07.yaml` | ~150 |

---

## Iron rules (R40-R42 lessons, all 6 yaml 共用)

1. **0 `_lost_methods.rs` placeholder**, **0 `part1.rs`/`part2.rs` 机械命名** — 必须 sub-domain 命名
2. **mod.rs ≤ 600 行**, **sibling ≤ 800 行** (R40c ports.rs 612 略超, R41d scheduler.rs 1315 严重超, R42f persistence_compact.rs 817 略超, 留 R50 cleanup)
3. Fields `pub(super)` for cross-sibling struct field access, methods default private / `pub(super)`
4. Wildcard re-export `pub use super::*;` in mod.rs
5. Write .rs with Edit tool (auto UTF-8), **禁 PowerShell Out-File/Set-Content**
6. **MUST run `cargo test -p <crate> --no-run --features product-full --lib <module>`** (R41b 教训: test module 12 compile errors from missing imports, PREFLIGHT 不跑 cargo test --no-run 会漏)
7. core.autocrlf=false set locally on worktree
8. 0 NEW unwrap/panic/unreachable, 0 CRLF/BOM, 0 Cargo.lock drift, 0 cross-crate regression
9. Long-line cap ≤ 120 chars, ≤ 5 NEW per file (R18+ relaxed)
10. 1 atomic commit per task, **禁 WIP commits**, **禁 `git commit --amend`** (R39c 0-byte 教训)

---

## Review checklist (user review focus)

- [ ] **Target path / 行数** 是否准确? (R44-R49 行数从 R40-R50 plan 继承, 实际可能 drift, worker PREFLIGHT 必 wc-l 验)
- [ ] **Model 分配** 是否合理? (R44 3 M2.7, R45 1 M2.7, R46 1 M2.7, R47 1 M2.7, R48 1 M2.7, R49 0 M2.7)
- [ ] **Sub-domain suggestion** 是否合理? (prompt worker 应基于实际代码 structure 调整, suggestion 仅 hint)
- [ ] **max_concurrency** = 7 (R44/R46/R47/R48) / 6 (R45) / 5 (R49), 经验值 7 worker 同时 spawn 会触发 rate limit (R41 第一次试 7-way 失败, 4-way 重派 OK, 6-way R40 OK, 5-way R49 OK, 但 R42 5-way 也 OK)
- [ ] **R44-R49 cron 频率**: 建议 20 min (task 数量大, 15min 会有过多 wake)
- [ ] **R46g protocol_persistence.rs 路径不确定** (PREFLIGHT 必查), 已加 fallback package 提示
- [ ] **Mavis 3-axis verify post-merge** (R21+ flow):
    1. `cargo check --workspace --message-format=short | rg '^error\['` = 0
    2. `cargo check -p <each dependent crate>` (R19 跨 crate 教训)
    3. `cargo test -p <target> --lib` + `cargo test -p <core> --lib` 不退化

---

## 派单顺序建议

**user 偏好**: R43 在跑, R43 cycle 1 完成 → R44 起, **sequential 1 round at a time** (避免 50+ worktree 同时占盘)

每 round 流程:
1. `mavis team plan run <yaml>` (or use my Mavis run command)
2. cron auto-monitor (15-20min)
3. cycle 1 完成 → Mavis 审 decision (auto_accept: false 强制审)
4. accept → R+1 起
5. cycle 2 (retry) → accept → R+1 起
6. R44-R49 全 accept → R50 cleanup + retrospective

---

## R50 cleanup (提前记下, review 不阻塞 R49)

1. **R40c ports.rs facade 612** → 再 split (port_set/port_route/port_format) < 600
2. **R41d scheduler.rs facade 1315** → 紧急 split, 严重超 cap (R50 must do)
3. **R41c tool_pipeline.rs 1628 horizontal** → 转 subdir split 风格统一
4. **R42f persistence_compact.rs 817** → 评估是否再 split
5. **R40c/R41c horizontal vs R40-R42 subdir 风格不一致** → 统一 subdir
6. **2 个 em-dash mojibake** R40e runtime_cancel_host.rs:1, runtime_dialog_host.rs:1 → review-fix 阶段清理
7. **R40-R49 12 round 全 review-fix-cleanup 循环** → persist
8. **R50 retrospective handoff** → `docs/handoffs/2026-07-06-r40-r50-retrospective.md`
9. **QClaw + Kimi batch review** (R40-R50 combined)

---

## Files

- 6 plan yaml: `docs/superpowers/plans/round4[4-9]-*-2026-07-07.yaml`
- R40-R42 plan: `docs/superpowers/plans/round40-r50-rework-with-step37flash-2026-07-06.md` (顶层 R40-R50 design doc)
- R43 plan: `docs/superpowers/plans/round43-6-way-parallel-2026-07-07.yaml` (已派, running)
- R43 plan state: `C:\Users\UmR\.mavis\plans\plan_6d824461\` (mavis plan engine state dir)

---

## 状态总结

- ✅ R40-R42 全部 accept (18 task, $16.95 + R42 $0 verifier cycle)
- 🟢 R43 running (plan_6d824461, 6 task)
- ⏳ R44-R49 plans ready, 6 yaml files written, **awaiting user review**

User review 通过后 → 派 R44 → accept → R45 → ... → R49 → R50 cleanup + retrospective。