# Review Report — P3a 死代码删除：ensure_assistant_bootstrap 死簇（−245/+17）

- **Reviewer**: judge (MiniMax-M3)
- **BASE**: `ff55a9b` ; HEAD diff = staged only
- **Scope**: 13 files, +17/−245（`git diff --staged --stat` 实测）
- **Repo**: `E:\agent-project\NortHing`
- **审查模式**: 删除类改动逐符号零调用方复核 + 修正事件恢复函数逐字节校验
- **复核纪律**: 每个符号亲自全仓 grep（不限同模块 / 不限 test / 不限 archive），恢复函数 PowerShell `-ceq` 字节对比

---

## 一、双判决

### 1. Spec 合规（删除清单逐项）

#### 1.1 `dialog_turn/coordinator_bootstrap.rs` 整文件删除（137 行）

**PASS**。文件内唯一方法 `ensure_assistant_bootstrap` 全仓 grep：

```
git grep -n "ensure_assistant_bootstrap"  # working tree
```

命中：
- `docs/archive/handoffs/2026-06-28-round6-dialog-turn-split-spec.md` (3 处) — 历史归档，frozen
- `docs/archive/handoffs/2026-07-02-r21-dialog-turn-mod-split-spec.md` (2 处) — 历史归档，frozen
- `docs/handoffs/2026-08-22-final-review-fixes.md:11,73` — 本批 P3a 笔记本身（提到此符号）
- `docs/handoffs/2026-08-22-final-review-fixes.md:71` — P3a 段落标题
- `dialog_turn/mod.rs:24` — 注释 `(2026-08-23, P3a: coordinator_bootstrap / ensure_assistant_bootstrap removed — snapshot-era dead code with zero callers since 2026-07-12.)`

无活代码调用方。归档/历史文档不应也未做改动。**确认零调用方。**

#### 1.2 `dialog_turn/turn.rs` 三孤儿助手：`assistant_bootstrap_kickoff_query` / `is_chinese_locale` / `assistant_bootstrap_system_reminder`

**PASS**。

- `assistant_bootstrap_kickoff_query`：working tree 全仓 grep → 仅 `docs/archive/handoffs/2026-06-28-round6-dialog-turn-split-impl.md` + `2026-06-28-round7-turn-internal-split-spec.md`，frozen 历史归档。
- `assistant_bootstrap_system_reminder`：同上，仅 frozen 归档。
- `is_chinese_locale`：

```
git grep -n "is_chinese_locale"  # working tree
```

命中：
- `docs/handoffs/2026-08-22-final-review-fixes.md:75` — P3a 删除范围自述
- `.superpowers/sdd/reviews/2026-08-23-p3a-deadbootstrap/brief.md:16` — 本审查 brief 自述
- frozen 归档 handoffs（同上）

**特别复核**：方法名 `is_chinese_locale` 易与同名但不同语义的 `language.is_chinese()`（`AppConfig`/`Language` 类型方法，在 `.agents/reference/skills/03-skill-policy.rs` 等多处使用）混淆。但 `language.is_chinese()` 是另类型同名方法，与被删 `ConversationCoordinator::is_chinese_locale` 静态方法无任何关联（参数、所属类型、可见性都不同）。`init_agents_md.rs` 中 `is_chinese` 仅是参数名（`fn init_agents_md_user_query(is_chinese: bool)`），与被删方法无关。**确认零调用方。**

#### 1.3 `coordinator.rs`：三 `AssistantBootstrap*` 枚举 + `ASSISTANT_BOOTSTRAP_AGENT_TYPE` 常量 + 死 import

**PASS**。

```
git grep -n "AssistantBootstrap"  # working tree
```

→ **0 命中**（无活代码、无文档、无测试）。变体名 `BootstrapNotRequired` / `SessionHasExistingTurns` / `SessionNotIdle` / `ModelUnavailable` 同为零命中。

```
git grep -n "ASSISTANT_BOOTSTRAP_AGENT_TYPE"  # working tree
```

→ 唯一命中是 `docs/handoffs/2026-08-22-final-review-fixes.md:75`（P3a 笔记自述删除范围），无代码。

被删常量值 `"Claw"` 仍是其他代码大量使用的合法 agent_type（`registry/catalog.rs`、`agents/definitions/modes/claw.rs`、`agents.rs` 等十几处），与本常量无依赖耦合。**确认零调用方。**

#### 1.4 6 文件空挂 import 清理

**PASS**。

涉及 `compaction.rs` / `session.rs` / `thread_goal.rs` / `workspace.rs` / `so_handlers.rs` / `coordinator.rs` 删除的 `use crate::service::bootstrap::{ensure_workspace_persona_files_for_prompt, is_workspace_bootstrap_pending};` 行。

HEAD 状态复核：
- HEAD 中 `is_workspace_bootstrap_pending` 调用方唯一一处：`coordinator_bootstrap.rs:39`（被删文件）。
- HEAD 中 6 文件的 `use ... is_workspace_bootstrap_pending;` 均为空挂（无 `.await?`/`.await`/`(` 调用），编译应是 dead-import warning，已顺手清理。
- HEAD 中 6 文件的 `use ... ensure_workspace_persona_files_for_prompt;` 同样空挂（同名文件 6 个空 use + `coordinator_bootstrap.rs:38` 一处真调用）。真调用方随 `coordinator_bootstrap.rs` 删除，故 6 空 use 也可一并清理。

`ensure_workspace_persona_files_for_prompt` 实际仅在 `bootstrap_impl.rs:218`（`build_workspace_persona_prompt` 内部同模块调用）+ `bootstrap_impl.rs:431`（测试）两处使用，均不依赖 re-export 路径。**确认安全清理。**

#### 1.5 `service/bootstrap/bootstrap_impl.rs`：删 `is_workspace_bootstrap_pending` + `reset_workspace_persona_files_to_default` + 两处 re-export

**PASS**。

- `is_workspace_bootstrap_pending` working tree 零代码命中（仅 frozen 归档 doc）。
- `reset_workspace_persona_files_to_default` working tree 零代码命中（仅本批 P3a handoff 自述）。
- `bootstrap/mod.rs` 删除 `pub use bootstrap_impl::reset_workspace_persona_files_to_default;` 与 `is_workspace_bootstrap_pending` re-export — 无外部依赖，安全。
- `service/mod.rs` 删除 `pub use bootstrap::reset_workspace_persona_files_to_default;` — 同上。

**特别复核**：`ensure_workspace_persona_files_for_prompt` 是否需从 `bootstrap/mod.rs` re-export 保留？

- 该函数被 `bootstrap_impl.rs:189`（内部同模块调用）+ `bootstrap_impl.rs:402`（测试）使用，均不依赖 re-export（`tests` mod 通过 `super::` 路径）。
- 6 个 `use` 语句已被清理。
- 无活代码通过 `crate::service::bootstrap::*` re-export 路径消费该函数。

**因此 re-export 删除合法，无外部消费者遗漏。** 确认。

#### 1.6 `service/mod.rs` −1 行（re-export 级联）

**PASS**。仅 1 行 `pub use bootstrap::reset_workspace_persona_files_to_default;` 删除，与 1.5 末尾级联。零外部消费者（working tree grep 零命中）。**确认。**

#### 1.7 handoff P3a 记录 + 遗留段翻牌

**PASS**。

`docs/handoffs/2026-08-22-final-review-fixes.md` 改动 13 行：
- 新增 P3a 段落（裁决依据、删除范围、保留边界教训、验证证据）
- 翻牌遗留段 P3a → 已删（strikethrough）

内容与 brief「裁决背景」一致；保留边界段正确记录修正事件教训（"死代码判定必须包含'同模块内部调用'维度"）。

---

### 2. 修正事件 — `ensure_workspace_persona_files_for_prompt` 逐字节校验

**逐字节等价 = PASS**。

```powershell
# HEAD 版（ff55a9b）
git show ff55a9b:src/crates/assembly/core/src/service/bootstrap/bootstrap_impl.rs
# line 136..190 = 55 行 = function body + closing brace

# Working tree 版
src/crates/assembly/core/src/service/bootstrap/bootstrap_impl.rs
# line 131..185 = 55 行

# PowerShell -ceq 字节对比：
$headJoined = head[136..190] -join "`n"
$workJoined = work[131..185] -join "`n"
$headJoined -ceq $workJoined
# → BYTE-IDENTICAL
```

**测试函数同步验证**：

- `ensure_workspace_persona_files_for_prompt_preserves_completed_bootstrap`：HEAD line 417..447（31 行）= work line 388..418（31 行）→ `-ceq` BYTE-IDENTICAL。

**同模块内部调用确认**（实施方自述盲区根因的举一反三验证）：

```
git grep -n "ensure_workspace_persona_files_for_prompt" src/crates/assembly/core/src/service/bootstrap/
```

→ `bootstrap_impl.rs:131`（定义）、`:189`（内部调用）、`:269`（tests mod import）、`:388`、`:402`（test 函数与调用）。**同模块内部消费关系完整、未被误删。**

---

### 3. Constraints 复核

#### 3.1 「纯删除语义：活代码路径一行不许动」

**Minor 违反**（见 Minor #1）：`session.rs` 在删除空挂 `use` 后，`cargo fmt` 重新流式化了相邻 `match crate::service::agent_memory::build_query_aware_facts_reminder(...).await` 多行调用为单行，触发了活代码路径的格式变更（4 行 → 2 行）。语义等价（同一函数、同一参数、同一 match 分支、同一 `.await`），但严格读约束"一行不许动"，触线。

#### 3.2 「skip_tool_confirmation 豁免面」

**PASS**。

```
git grep -n "skip_tool_confirmation" src/   # 仅生产代码
```

剩余 `skip_tool_confirmation: true` / `.with_skip_tool_confirmation(true)` 共 4 处：

| # | 位置 | 注解状态 |
|---|---|---|
| 1 | `a1_path.rs:259` | `// Intentional exemption (A2 hidden-subagent path)` ✅ |
| 2 | `coordinator_compact.rs:100` | `// Intentional exemption (manual compaction)` ✅ |
| 3 | `subagent_orchestrator/so_lifecycle/lifecycle.rs:214` | `// Intentional exemption (hidden-subagent phase2)` ✅ |
| 4 | `subagent_orchestrator/so_handlers.rs:137` | **无注解** |

**核对**：被删 `coordinator_bootstrap.rs:220` 原为"第四处未注解豁免"（带 `.with_skip_tool_confirmation(true)` 但缺 probe-1 注释），本批删除后该处消失。✅

**⚠️ 残留观察（不归本批责任）**：`so_handlers.rs:137` 仍存在一处未注解的 `with_skip_tool_confirmation(true)`（pre-existing）。HEAD 中该行（line 138）同样无注解，本批未引入。归口："4 处 → 3 处已注解豁免 + 1 处 pre-existing 未注解"，但 brief 仅承诺"第四处确实随文件消失"（✅）与"无新增"（✅）。该残留是 pre-existing 债，建议挂账下次 housekeeping，不阻断本批。

#### 3.3 「rot-budget 只降不升」

**PASS**。`git diff HEAD -- scripts/rot-budget.json` 输出为空，本批未触动该文件。`coordinator_bootstrap.rs` 整文件删除使顶层 `dir_entries` 计数自然 −1（1365→1364 与 handoff 报告一致）。无 ceiling 调升。

#### 3.4 「日志英文-only / 分层边界」

**PASS**。删除范围未引入任何新日志字符串；分层影响面限于 `assembly/core` 内部（`agentic/coordination/*` + `service/bootstrap/*`），无跨层引用新增/删除。

#### 3.5 「远程兼容 / 骨干不变量」

**PASS**。删除面在 agentic 协调模块 + service 模块，均非 backbone 列项（desktop 包名 / GlobalConfig SSOT / UI 线程纪律 / shell guard / slug / installer / v0.1.0 面）。未触及任何列项；不存在配置 SSOT 二元化、shell guard 绕路、UI 线程直写等违规。

---

## 二、Findings

### Critical

**无**。

逐符号零调用方复核（每个符号亲自全仓 grep，含同模块内部、测试、re-export 链、字符串引用、archive 文档）均通过；恢复函数逐字节等价；修正事件教训举一反三验证到位。

### Important

**无**。

- 删除清单 1–7 全 PASS。
- 修正事件恢复函数 + 测试双 BYTE-IDENTICAL。
- skip_tool_confirmation 第 4 处豁免随文件消失，无新增豁免。
- rot-budget、骨干不变量、日志英文-only、分层边界均未触动。

### Minor

#### M1 — `session.rs` rustfmt 重排活代码路径（违反"活代码路径一行不许动"严格读）

- **位置**: `src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs:220-223`
- **证据**:

```diff
-                match crate::service::agent_memory::build_query_aware_facts_reminder(
-                    workspace.root_path(),
-                    &user_input,
-                )
-                .await
+                match crate::service::agent_memory::build_query_aware_facts_reminder(workspace.root_path(), &user_input)
+                    .await
                 {
```

- **性质**: rustfmt 副作用，紧跟 `use` 缩短后的流式重排。语义 100% 等价（同名函数、同参、同 match arm、同 `.await`、同 body），但严格读 brief constraint #1 "活着的代码路径一行不许动"，活代码路径 `session.rs` 的 match arm 被触动。
- **建议**: 若严格死守"纯删除、不动活代码格式"约束，应 `git checkout HEAD -- src/crates/.../session.rs` 撤掉该 reformat，并禁用本批 `cargo fmt`（或限制 fmt 范围到仅 5 个纯删除文件）。如接受本批处理，亦可在 handoff 增一行"session.rs 有 rustfmt 副作用重排，语义等价、无调用方变化"的明示，避免与终审 triage 的"活路径不重排"原则冲突。
- **阻断**: 否。行为不变、属 fmt 副作用、可在终审 triage 决策。

#### M2 — `so_handlers.rs:137` pre-existing 未注解 `with_skip_tool_confirmation(true)` 残留

- **位置**: `src/crates/assembly/core/src/agentic/coordination/subagent_orchestrator/so_handlers.rs:137`
- **证据**: HEAD line 138 / work line 137，pre-existing，无 `// Intentional exemption ...` 注释；其他 3 处均有 probe-1 注解。
- **性质**: 不在本批责任（pre-existing、未引入、未触动），brief 仅承诺"第四处随文件消失"（即被删 bootstrap 文件那一处）。但作为审查观察上报：注解完整度差一处，下次 housekeeping 可顺手补齐。
- **阻断**: 否。pre-existing 债，与本批删除目标正交。

---

## 三、Cannot verify from diff

无。

- 编译验证（`cargo check --workspace`、`cargo check -p northhing`、`cargo test -p northhing-core --features product-full --lib service::bootstrap` / `coordination`）：brief 明确"已声称验证（report 即证据，不重跑）"。本轮未实测重跑，信任 deliverer report。
- 测试运行同上。
- rot-budget 文件数值（1365→1364）来自 deliverer 复算报告，本轮未独立重跑 `node scripts/verify-rot-budget.mjs`。

如需独立复算，跑：
```bash
cargo check --workspace
cargo test -p northhing-core --features product-full --lib service::bootstrap
cargo test -p northhing-core --features product-full --lib coordination
cargo check -p northhing
cargo check -p northhing-cli
node scripts/verify-rot-budget.mjs
```

---

## 四、判决汇总

| 维度 | 结果 |
|---|---|
| Spec 合规（清单 1–7） | PASS（全部） |
| 修正事件恢复 | PASS（BYTE-IDENTICAL，函数 + 测试） |
| 零调用方判定 | PASS（每符号全仓 grep，含同模块内部 / test / archive） |
| skip_tool_confirmation 豁免面 | PASS（第 4 处消失，无新增；残留 1 处 pre-existing） |
| rot-budget | PASS（未触动） |
| 骨干不变量 | PASS（无触动） |
| 日志英文-only / 分层边界 | PASS |
| 活代码路径不可动（约束 #1 严格读） | **Minor 违反**（M1，session.rs rustfmt 重排，语义等价） |

**APPROVE**
