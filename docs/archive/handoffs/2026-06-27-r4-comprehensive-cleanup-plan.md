# Debt-R4 综合清理 Plan — 2026-06-27

> **Type**: refactor + fix (综合)
> **Trigger**: Round 2.5 v3 audit (`research/audit_redim_v3_01-04.md`) 发现 18 项未修复 + 3 项恶化
> **Status**: spec 阶段 + 即将 dispatch team plan 执行
> **Predecessor**: Round 4 (`9dbcb9c`) 完成 panic cleanup
> **Goal**: 在一次 cycle 内清掉 P0 + P1 全部 + P2 优先级最高的 3 项

---

## 0. 审计数据校正

| 审计声明 | 校正 |
|---|---|
| session_manager.rs "6532 行零变化" | ❌ 数字错。实际 **5948 行**（Round 3b 后）。但 audit agent 没说错的是 **行数没减** — Round 3b 是"名义拆分"（impl block 复制到 sibling files, session_manager.rs 本身未瘦）。从外部看仍是 God Object |
| target 970 MB → 27 GB | ✅ 真实回退。Round 2-4 期间持续 build + 未 split-debuginfo + 未 target-dir 重定向 |
| unwrap 65 → 495 | ✅ Round 2-4 未审计 unwrap 增量，miniapp 模块等新增代码引入大量 unwrap |
| 旧 Phase struct 未删 | ✅ Round 4 没动这部分 |
| dialog_turn.rs 3395 行 | ✅ 验证正确，未二次拆分 |
| review_platform 4866 行 | ✅ 验证正确，未拆分 |

**关键纠错**：session_manager.rs 已被 `5250199` commit **拆过**（生产文件存在 session_persistence.rs 等 3 个 sibling modules），但**行数没减**，导致 audit agent 误判"未拆分"。这是**名义拆分 vs 实质拆分**的差异。

---

## 1. 综合修复目标（按优先级）

### P0（必须本轮完成 — 3 项）

| # | 行动 | 目标文件 | 预期工作量 | 验收 |
|---|---|---|---|---|
| P0-1 | **实质拆分 session_manager.rs**（行数减半） | `agentic/session/session_manager.rs` (5948 → ~2500) | 4-5 小时 | session_manager.rs 行数 < 3000；4 个 sibling modules 各 < 1500 行；零功能变化；现有 935 测试全过 |
| P0-2 | **修 7 处信号量 unwrap** | `agentic/insights/service.rs:445,455,466,478,496,508,519` | 30-60 分钟 | 所有 .unwrap() → `.await— ` 或 log + fallback；7 处 panic 风险消除 |
| P0-3 | **target 27 GB 收敛** | `target/` + `.cargo/config.toml` | 1 小时 | cargo clean + 配置 split-debuginfo + 可选 target-dir 重定向；target 大小 < 5 GB |

### P1（本轮完成 — 5 项）

| # | 行动 | 目标 | 工作量 |
|---|---|---|---|
| P1-1 | **拆 review_platform/mod.rs** | 4866 → 5 模块（按 Round 6 spec） | 5-7 小时 |
| P1-2 | **删旧 Phase 路径** | SubagentPhase1Output/Phase2Output/SubagentExecutionScope + execute_hidden_subagent_phase1/2/3 函数 | 2-3 小时 |
| P1-3 | **拆 dialog_turn.rs 3395 → 二次拆分** | 3-4 模块 | 3-4 小时 |
| P1-4 | **删 computer_use_input.rs / browser_launcher.rs shim** | 5+3 处 dead_code | 30 分钟 |
| P1-5 | **修 installer 依赖版本— 突** | dirs 5/6、zip 0.6/4.6、reqwest 0.12/0.13.4 + exclude 拼写（northhing vs northing） | 4-8 小时 |

### P2（本轮选 2-3 项 — 4 项）

| # | 行动 | 目标 | 工作量 |
|---|---|---|---|
| P2-1 | **修边界泄露（97 处）** | CLI 通过 runtime-ports 访问 core，禁止直接穿透 | 3-5 天（不做） |
| P2-2 | **miniapp 模块 unwrap 收敛** | miniapp/storage.rs (57) + manager.rs (44) + builtin/mod.rs (28) → Result | 1-2 天 |
| P2-3 | **let _ = Result 丢弃清理（~526 处）** | Top 3 hotspot + 全局策略 | 2-3 天 |
| P2-4 | **配置 sccache** | `.cargo/config.toml` | 30 分钟 |

**本轮选 P2-2 + P2-4**（边界泄露和 let _ = 太大，留 Round 5+）。

---

## 2. Team Plan 调度方案（用 code-rotation-guard skill）

### 2.1 skill 触发理由

按 `code-rotation-guard` skill 的 5 个 trigger signals：

| Signal | 当前是否触发 |
|---|---|
| Last 5 commits touch ≥ 3 crates | ✅ Round 3a/3b/4 跨多 crate |
| team plan auto-paused | ❌ |
| Single commit split > 1000 lines | ✅ Round 3a (7215→618), Round 3b (5948→5948 + 3 siblings) |
| Release tag imminent | ❌ |
| User manual trigger | ✅ 用户明确说"做腐化审计" |

→ **触发 code-rotation-guard Phase 1**。

### 2.2 Team Plan 结构（4 阶段）

**Plan A: Debt-R4 Comprehensive Cleanup**

```
Plan:
 name: "debt-r4-comprehensive-cleanup-2026-06-27"
 max_concurrency: 3
 max_cycles: 3

Tasks:
 Phase 1 (audit) — 4 并行 audit agents, depends_on: []
 Phase 2 (spec) — 2 writing-plans agents, depends_on: [Phase 1]
 Phase 3 (impl) — 4 coder agents, depends_on: [Phase 2]
 Phase 4 (review) — 1 Mavis summary + verifier, depends_on: [Phase 3]
```

### 2.3 Phase 1 — 4 并行 audit sub-agent（20 min each）

| Agent | 任务 | 输出 |
|---|---|---|
| **visibility-auditor** (general) | 扫描所有 Rust 文件的 `pub`/`pub(crate)`/`pub(super)` 一致性；找跨 module 同名 `impl Type { fn foo() }` 重复（特别 session_manager.rs 的 sibling impl） | `research/audit-r4-visibility.md` |
| **duplicate-scanner** (general) | 扫描 session/ 目录所有 `pub struct X` / `pub enum X` / `impl X` 重复定义；跨文件同函数体比对 | `research/audit-r4-duplicates.md` |
| **arch-boundary-checker** (general) | 验证 AGENTS.md 边界规则；CLI → core 穿透点（97 处）；installer 依赖版本— 突 | `research/audit-r4-boundary.md` |
| **deadcode-hunter** (general) | cargo build warnings 收集 + `#[allow(dead_code)]` 站点审计 + orphan method 扫描 + unwrap 分布 | `research/audit-r4-deadcode.md` |

**Audit agent 关键 prompt 校正**（避免 audit data drift bug）：
- 必须 `wc -l <file>` 真实测量当前行数
- 不能用"上次审核 baseline"作为当前值
- 每个 finding 必须有 `file:line` 精确引用，不能写"约 X 处"
- 必须运行 `git log -1 --format='%H %s'` 确认最新 commit SHA

### 2.4 Phase 2 — writing-plans 派 2 spec agents

| Agent | 输入 | 输出 |
|---|---|---|
| **spec-session-split** (general, 30 min) | Phase 1 visibility + duplicates 结果 + Round 3b handoff | `docs/handoffs/2026-06-27-r4-session-real-split-spec.md` — 实质拆分 spec（行数减半，不是 nominal） |
| **spec-review-platform** (general, 30 min) | Round 6 spec（已写）+ Phase 1 boundary 结果 | `docs/handoffs/2026-06-27-r4-review-platform-impl-spec.md` — Round 6 实现 spec |

### 2.5 Phase 3 — coder 派 4 implement agents

| Agent | 任务 | 预计耗时 | Timeout |
|---|---|---|---|
| **impl-session-split** (coder, 4-5h) | 按 spec-session-split 实质拆分 session_manager.rs | 4-5h | 2700000 (45 min) |
| **impl-review-platform** (coder, 5-7h) | 按 spec-review-platform 拆 review_platform/mod.rs | 5-7h | 2700000 |
| **impl-semaphore-unwrap** (coder, 30-60 min) | 7 处 insights/service.rs unwrap → Result 传播 | 1h | 1200000 |
| **impl-target-cleanup** (coder, 30 min) | cargo clean + split-debuginfo + target-dir 配置 | 1h | 600000 |

**4 个 agent 真正并行**（max_concurrency: 4），但 `impl-session-split` 和 `impl-review-platform` 是大型改动，建议 sequential（max_concurrency: 2）以避免 workspace — 突。

### 2.6 Phase 4 — Mavis review

- Read 4 个 impl agent 的 deliverable
- cargo check + cargo test 全 workspace
- cargo fmt --check
- 写 `docs/handoffs/2026-06-27-r4-final-review.md`
- 更新 MEMORY.md

---

## 3. 风险与缓解

| 风险 | 影响 | 缓解 |
|---|---|---|
| team plan auto-pause (2 cycles 0 pass) | Mavis take over | 历史经验 Round 3a/3b 都触发，Mavis 5min 接管 |
| Coder 30 min timeout 偏紧 | spec 不完整就停 | Round 4 验证用 45 min timeout |
| sub-agent context drift (拿 stale baseline) | audit 错位 | Prompt 强制"必须真实测量 wc -l + git log 验证 commit" |
| session_manager.rs 实质拆分行数不达标 | P0-1 失败 | spec 强制验收 "session_manager.rs 行数 < 3000 + 现有 935 测试全过" |
| target 27 GB 收敛但 cargo clean 丢失增量编译缓存 | 后续 build 变慢 | 配置 split-debuginfo（不删 debug info）+ target-dir 重定向（不影响主缓存） |
| impl-session-split 和 impl-review-platform workspace — 突 | git 工作树污染 | sequential 调度 + 每个 impl 用独立 worktree |

---

## 4. 不在本轮范围

- ❌ 边界泄露 97 处（3-5 天工作量）— Round 5+
- ❌ let _ = 526 处清理 — Round 5+
- ❌ dialog_turn.rs 二次拆分 — Round 5+ 跟 Round 5 chat.rs 拆分并行
- ❌ 旧 Phase struct 删除后 A1/A2 路径稳定性回归测试 — Round 6+

---

## 5. 验证清单

| 检查项 | 通过条件 |
|---|---|
| session_manager.rs 实质拆分 | 行数 < 3000；现有测试全过；4 个 sibling module 各 < 1500 行 |
| review_platform 拆 5 模块 | mod.rs < 200 行；4 子模块各 < 1000 行 |
| 7 处 unwrap 修 | insights/service.rs:445-519 零 panic；编译通过 |
| target 收敛 | < 5 GB（vs 27 GB 当前）；配置 .cargo/config.toml split-debuginfo |
| 旧 Phase 删除 | `SubagentPhase1Output` / `SubagentPhase2Output` / `execute_hidden_subagent_phase1/2/3` 在 workspace 0 引用 |
| dialog_turn 二次拆分 | < 5 模块；行数 < 5000（合并行） |
| dead_code shim 删除 | computer_use_input.rs / browser_launcher.rs 不再 untracked |
| installer 依赖 | dirs / zip / reqwest 版本统一；exclude 拼写修正 |
| 全 workspace cargo test | 935+ 测试通过（vs 当前 935 baseline） |
| cargo fmt --check | 全部干净 |
| cargo clippy | 无新增 warning |

---

## 6. 预计工作量

| 阶段 | 工作量 |
|---|---|
| Phase 1 audit | 20-30 min (4 sub-agent 并行) |
| Phase 2 spec | 30-45 min (2 sub-agent) |
| Phase 3 impl | 8-12 h (4 sub-agent, 关键路径是 session_manager.rs 实质拆分 4-5h) |
| Phase 4 review | 30-45 min (Mavis) |
| **总计** | **9-14 小时（~1.5-2 天）** |

比 Round 4 的 4h 多 2-3x 工作量，但覆盖 18 项 + 3 项恶化。

---

## 7. 与 code-rotation-guard skill 的对应关系

本 plan 是 `code-rotation-guard` skill 的一次具体应用：

| Skill phase | 本 plan 对应 |
|---|---|
| Phase 0: signal verification | Section 0 + Section 2.1（5 signals 触发） |
| Phase 1: 4 并行 audit agents | Section 2.3（visibility / duplicates / boundary / deadcode） |
| Phase 2: Mavis summary | Section 2.4（writing-plans 派 spec） |
| Phase 3: refactor spec | Section 2.5（4 个 impl coder） |
| Phase 4: refactor team plan + verification | Section 2.6（Mavis final review） |
| Loop closure | 本轮完成后，重新 invoke skill，对比 blocker count 变化 |

**学习反馈**：本 plan 暴露 audit_redim_v3 的 stale baseline bug，需要在 skill 的 audit prompt 模板里强制"必须 `wc -l` 真实测量"。下一轮 cycle 的 audit agent 会用改进的 prompt。

---

## 8. Errata — 不确定项

待 dispatch 前确认：

- **E1**: session_manager.rs 实质拆分策略 — (a) 按功能（lifecycle / persistence / restore / config / evidence / cleanup）；(b) 按 caller 关系（dialog_turn caller / coordinator caller / tools caller）。**倾向 (a) 与 Round 3b 产物对齐**，**待 review 决议**。
- **E2**: target 27 GB 收敛策略 — (a) cargo clean + 不配置 split-debuginfo（简单但每次 rebuild 全量）；(b) cargo clean + 配置 split-debuginfo（减少 80% 大小）；(c) target-dir 重定向到 D:/target（彻底隔离但改变项目结构）。**倾向 (b)**，**待 review 决议**。
- **E3**: 旧 Phase 删除时机 — 现在删（接受风险）vs 等 Round 5+（更多测试覆盖）。**倾向现在删**（audit 已 2 轮确认 A1 路径稳定），**待 review 决议**。
- **E4**: impl-session-split 和 impl-review-platform sequential 还是并行 — **倾向 sequential**（workspace — 突），**待 review 决议**。
- **E5**: 4 个 impl agent 用什么模型 — 我之前 Round 3a/3b 用 `MiniMax-M2.7-highspeed`（coder 30 min 不够，需要 45 min）。**沿用 highspeed**，**待 review 决议**。

---

## 9. 最终方案

```
dispatch 顺序:
 1. Phase 1: 4 audit agents (parallel, 20 min)
 2. Phase 2: 2 spec agents (parallel, 30 min, depends_on: Phase 1)
 3. Phase 3: 4 impl agents (sequential, 8-12h, depends_on: Phase 2)
 4. Phase 4: Mavis review + final commit (45 min, depends_on: Phase 3)
 5. Loop closure: re-run audit, diff blocker count (10 min)
```