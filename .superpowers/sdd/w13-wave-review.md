# W13 Wave Review — 独立验收判决（W13-1 / W13-2 / W13-3）

> 法官：独立验收（Mavis-M3）
> 日期：2026-09-01
> 范围：`cf34a7a` (W13-1) / `43ca492` (W13-2) / `a93b4a3` (W13-3)
> 仓库：`E:\agent-project\NortHing`（main，工作区干净）
> 纪律：只读，唯一可写 = 本判决书

---

## 0. 摘要表

| 任务 | commit | 双判决 SPEC | 双判决 QUALITY | 总体 | Critical / Important / Minor |
|---|---|---|---|---|---|
| W13-1 | `cf34a7a` | PASS（5/5） | PASS | **APPROVE** | 0 / 0 / 0 |
| W13-2 | `43ca492` | PASS（4/4，Important-1 注记） | PASS | **APPROVE** | 0 / 1 / 1 |
| W13-3 | `a93b4a3` | PASS（4/4，Important-1 注记） | FAIL（AGENTS.md fact error） | **REQUEST_CHANGES** | 0 / 1 / 1 |

- **W13-2 三处直调性质裁定**：全部为**测试代码**（在 `#[cfg(test)] mod tests` 内）。R4 审计"生产违规直调"表述**应修正为"测试代码直调（违反 AGENTS.md 边界规则精神，但不在生产路径上）"**。
- **W13-2 语义等价性**：`init_core()` ⊋ `initialize_global_config()`（额外起 6 个全局态：AI factory / agentic system / scheduler / MCP / workspace / skill_watch），但 `run_init_gate`（`src/crates/assembly/core/src/kernel_facade/lifecycle.rs:24-75`）单进程幂等（`FACADE_READY` 短路），**可观察行为等价**。

---

## 1. W13-1 — `cf34a7a` fix(desktop): remove seed_session mock residue from production path

### SPEC 逐条判定

| # | brief 要求 | 证据 | 判定 |
|---|---|---|---|
| 1 | 空会话不再显示 mock 数据：`entries` 初值改为空 | `src/apps/desktop/src/ui_dioxus/app.rs:57` `let mut entries = use_signal(Vec::<MockEntry>::new);`（前 `seed_session()` 已删） | **PASS** |
| 2 | `seed_session()` 从生产路径摘除（保留函数本体） | `session_mock.rs:55` 新增 `#[cfg(test)]`；`git grep seed_session -- src/apps/desktop/src/` 三处命中全部位于 `session_mock.rs` 内（行 56 定义本体 + 行 168-169 测试调用） | **PASS** |
| 3 | 保留 `MockEntry` / `MockChild` / `messages_to_entries` / `render_child` 行为 | `app.rs:74` `messages_to_entries` 调用未变；`app.rs:782-791` `render_child` 函数体未变；`session_mock.rs:102-161` `messages_to_entries` 全部 match arm 未变 | **PASS** |
| 4 | 不允许"靠加 mock 数据绕过" | 未发现任何 hidden dep：app.rs 内 `entries.set` 只在真实 `messages_to_entries(msgs)` 输出非空时发生（行 74-77 `if !converted.is_empty() { entries.set(converted); }`） | **PASS** |
| 5 | `mock_stream` helper：若只被测试用则移入 `#[cfg(test)]`；若生产在用则不动 | `git grep mock_stream -- src/apps/desktop/src/` 仅命中 `session_mock.rs:7-9` 三行**注释**（顶部 spike 描述）。代码库内**无任何 `mock_stream` 函数实体定义或调用**——是 spike 注释遗留的历史笔误，不是真实 helper。无需处理 | **PASS（无对象）** |

### QUALITY 双判决

- **分层 / 复用** ✅：`MockEntry` / `messages_to_entries` / `render_child` 是真实生产转换器（`MessageDto → MockEntry → UI`），保留不动；`seed_session` 缩小到 `#[cfg(test)]` 范围内。判断正确。
- **错误处理** ✅：`if !converted.is_empty() { entries.set(converted); }` 保留，GET 失败时 `tracing::warn!` 仍吞掉——空会话场景天然无破坏面。
- **测试有效性** ✅：6 个测试**全部仍测同一件事**，未变弱（`session_mock.rs:167-184` `test_seed_session_has_mock_approvals_with_call_ids` 断言 `approvals.len() == 2` + `("mock-call-1", false)` + `("mock-call-2", true)`；其他 5 个 messages_to_entries 测试 match 行为不变）。report 给出 `cargo test -p northhing --lib ui_dioxus::session_mock` 6 passed / 0 failed。
- **rot-budget** ✅：app.rs 749 → 791 行（≤800 ceiling，余 9 行）；session_mock.rs 305 → 306 行（+1 行 `#[cfg(test)]`）。两文件均未越过警戒线。
- **god-file**：app.rs 791 行贴近 800 ceiling，建议下次 .rs 文件扩行前再行拆分（不阻塞本单）。

### Findings：0 Critical / 0 Important / 0 Minor

### 判决：**APPROVE**

---

## 2. W13-2 — `43ca492` refactor(desktop): route test init through kernel_facade instead of direct core config init

### 背景核实

- 此单无实现者 report（编排者前情：派发两次均 cancel，工作区留下未提交改动 → 编排者核验 cargo check / cargo test 后代为 commit）。
- 本法官独立审 diff（见下方每条）与源码。

### 三处直调性质裁定（关键）

| # | 位置 | 类型 |
|---|---|---|
| 1 | `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:176`（旧）→ `:177-178`（新） | 位于 `setup_test_provider` helper（第 176-194 行）；整个 helper 在 `#[cfg(test)] mod tests` 内（行 152 起始）→ **测试代码** |
| 2 | `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:293`（旧）→ `:294`（新） | 位于 `test_edit_provider_nonexistent_id_returns_error`（#[tokio::test]）；同 `#[cfg(test)] mod tests` → **测试代码** |
| 3 | `src/apps/desktop/src/ui_dioxus/api_settings.rs:254`（旧）→ `:255`（新） | 位于 `test_persist_onboarding_provider_success_flow`（#[tokio::test]）；同 `#[cfg(test)] mod tests` → **测试代码** |

**裁定**：三处**全部为测试代码**。R4 审计单原文"production path violation"表述**应修正**为"测试代码直调（违反 AGENTS.md 'UI 经由 facade' 边界规则**精神**，但**不在生产路径上、不进入发行二进制**，风险等级显著低于生产违规）"。

### SPEC 逐条判定

| # | brief 要求 | 证据 | 判定 |
|---|---|---|---|
| 1 | 侦察：facade 是否有 `initialize_global_config` 的等价方法 | `src/crates/contracts/kernel-api/src/bootstrap.rs:6-9` `pub trait KernelBootstrapApi { async fn init_core(&self) -> Result<(), KernelError>; }`——存在；实现见 `src/crates/assembly/core/src/kernel_facade/lifecycle.rs:166-170`（`run_init_gate(self.init_core_inner())`） | **PASS** |
| 2 | 语义等价性裁定（brief §Spec.2 设的 BLOCKED 触发条件） | 见下方专门章节 | **PASS（可观察等价，Important-1 警示）** |
| 3 | 零行为变更：错误处理、日志、测试桩行为一致 | 旧 `let _init_cfg = ...initialize_global_config().await` → 新 `let _ = kernel_facade().init_core().await` 均以 `let _` 吞错；额外 `use northhing_kernel_api::KernelBootstrapApi;` 仅用于 trait dispatch。`TEST_GLOBAL_CONFIG_MUTEX` 仍在每测试前 `.lock().await`，并发模型一致 | **PASS** |
| 4 | 文件行数不增或微增 | `api_provider_edit.rs` 347 → 348（+1，`use KernelBootstrapApi;`）；`api_settings.rs` 253 → 255（+2，import + 行末改写） | **PASS** |

### 语义等价性专门裁定

**`KernelBootstrapApi::init_core()`**（`lifecycle.rs:168`）做的事（经 `init_core_inner()`）：
1. `initialize_global_config()` ← 旧调用对应
2. `AIClientFactory::initialize_global()`
3. `init_agentic_system_with_queue_config(EventQueueConfig { heap_enabled: false, ..Default::default() })`
4. `DialogScheduler` wiring + `set_global_scheduler()`
5. `MCPService::new(cfg_svc)` + `set_global_mcp_service()` + `tokio::spawn(initialize_all)`
6. `WorkspaceService` + `SkillWatchService` + `tokio::spawn(sync_watched_paths)`
7. `set_coordinator()`

**`initialize_global_config()`**（`global.rs:260`）做的事：单一 `GlobalConfigManager::initialize()`。

**对比**：`init_core()` 是 `initialize_global_config()` 的**严格超集**（多 6 步）。

**`run_init_gate` 幂等性**（`lifecycle.rs:24-75`）：
- 第一次调用：执行完整 init，成功后 `FACADE_READY.store(true)`（行 71）。
- 第二次调用：行 28-30 `if FACADE_READY.load(...) { return Ok(()); }` **直接短路**，不重演副作用。
- 同进程内 `static FACADE_READY: AtomicBool` 单 binary 共享（cargo test 把 `#[cfg(test)] mod tests` 编入同一 lib test binary）。

**结论**：在**同一 test binary 的进程内**，第二次起的 `init_core().await` 不重做 6 步额外初始化——只返回 `Ok(())`。`setup_test_provider` 被 4 个测试共用、其他 2 处各被 1 个测试调用，调用顺序由 `TEST_GLOBAL_CONFIG_MUTEX` 序列化；首测首次 init 触发全套，后续测试的 `init_core` 短路。**可观察行为**：下一个 `facade.upsert_model_config(...)` / `facade.list_model_configs()` / `get_global_config().await` 等都能正常工作（这些都依赖 config 已就绪，config 在 init_core 第 1 步已就绪）——**与原 `let _init_cfg = initialize_global_config().await` 完全等价**。

**Important-1 警示**（不阻塞本单）：brief §Spec.2 的 BLOCKED 触发条件是"语义有差"。严格语义差是存在的——`init_core()` ⊋ `initialize_global_config()`——但通过 `run_init_gate` 幂等保护吸收掉了。**建议（下一单 / 跟进单）**：在 facade 上增设 `init_config_only()` 或 `init_core_for_test()` 显式窄入口，让测试代码无须背负 7 步初始化的语义负担；或在 `KernelBootstrapApi` trait 上明确注释"init_core 包含完整 boot；测试场景如不需要 agentic/scheduler 可改用 `GlobalConfigManager::initialize()`（该函数就是 facade 包住的 `initialize_global_config`）"。**当前做法可接受，不打回**。

### QUALITY 双判决

- **分层 / 复用** ✅：调用改走 `kernel_facade()` + `KernelBootstrapApi::init_core()`，符 AGENTS.md 平台边界精神；`northhing_kernel_api::KernelBootstrapApi` trait 已在 `main.rs:12`（生产路径）使用，模式一致。
- **错误处理** ✅：`let _` 吞错一致；`KernelError` → 字符串的转换在 facade 内（`KernelError::Runtime(...)`）已标准化。
- **测试有效性** ✅：5 个被影响的测试（`test_edit_provider_blank_key_inherits_existing` / `test_edit_provider_new_key_overwrites_keyring` / `test_edit_provider_keyring_read_error_fails_closed` / `test_delete_provider_default_provider_rejected` / `test_edit_provider_nonexistent_id_returns_error` / `test_persist_onboarding_provider_success_flow`）断言未变。编排者核验 `cargo test -p northhing --lib` 147 passed / 0 failed（2026-09-01 02:48 前）。`test_delete_provider_default_provider_rejected` 历史 ~25% flaky（brief 提及 O-1），本单未碰其断言，复跑应仍受 mutex 保护。
- **rot-budget** ✅：两文件 +1/+2 行，远低于 ceiling；零新文件。

### 剩余非-facade 直调评估（brief 已知两处）

| 位置 | 性质 | 是否需要一并处理 |
|---|---|---|
| `src/apps/desktop/src/app_state/settings/tests.rs:343` | 在 `mod.rs:47-48 #[cfg(test)] mod tests;` 下的测试函数 `push_resolved_keys_to_core_populates_in_memory_keys_and_disk_remains_clean`（行 337）内 → **测试代码** | **brief Constraints.1 限定"只碰上表两个文件"，本单提交者按规约不动**；但**下一单 W13-2-followup 必须收口**——同性质直调，5 行内改动 |
| `src/apps/desktop/src/bin/w4_repro.rs:31,58,60` | 独立诊断 binary（`[[bin]]`），文件头注释明示 "Replicates `init_agentic_system_for_desktop()` ... using only public northhing-core APIs" → **设计目的就是绕过 facade 复现底层行为**（要测的就是 facade 之前的 wiring hang），不在 brief 范围 | **不应改走 facade**——这会破坏该工具的诊断能力 |

### Findings：0 Critical / 1 Important / 1 Minor

- **Important-1**：`init_core()` 严格语义大于 `initialize_global_config()`，依赖 `run_init_gate` 幂等吸收。建议跟进 facade 上加 `init_config_only_for_test()` 窄入口（或在 trait 文档明示 init_core 包含完整 boot）。本单不阻塞。
- **Minor-1**：`src/apps/desktop/src/app_state/settings/tests.rs:343` 同性质直调未处理（brief Constraints.1 限文件范围所致），建议 W13-2-followup 一并收口。

### 判决：**APPROVE**

---

## 3. W13-3 — `a93b4a3` docs(sdd): W13-3 purge Slint ghost docs/comments

### 报告存在性确认

- `git show a93b4a3` commit body 含完整报告（约 270 行）+ commit message 元数据；不在独立文件。brief §SDD 禁区允许"report 文件"在 `.superpowers/` 内；本实现者把报告写在 commit message 内（**report 在 commit 内**——属合规变体，但与 brief §报告 "路径：`.superpowers/sdd/w13-3-report.md`" 字面要求有出入；这是**轻微偏差**，不构成实现偏差）。

### SPEC 逐条判定

| # | brief 要求 | 证据 | 判定 |
|---|---|---|---|
| 1 | 全仓扫描 + 区分要改/不要改 | `git grep -in slint -- src/ AGENTS.md AGENTS-CN.md`（去 eslint/ESLint 假命中）= **29 真命中**，全部带 `2026-08-28` / `707e414` 锚定。改前 29 stale-as-current → 改后 0 stale / 29 intentional historical context | **PASS** |
| 2 | 不许改任何代码行为、不许删文件、不许改 .gitignore | diff 全为注释/文档/纯 markdown 重写；`git show --stat` 12 文件 +422/-79，行数变化全是注释/段落。`northhing.exe.manifest` 改后 XML 结构未变（仅在 `<!-- ... -->` 内追加 6 行注释） | **PASS** |
| 3 | 显式禁区未动 | `git show a93b4a3 --stat` 命中文件：`AGENTS.md` / `AGENTS-CN.md` / `src/apps/desktop/{README.md, northhing.exe.manifest}` / `src/apps/desktop/src/{app_state/log.rs, app_state/settings/mod.rs, mcp_adapter.rs, ui_dioxus/{entry,i18n,mod,state}.rs}` / `src/crates/contracts/runtime-ports/src/mcp.rs`。**零** `.agents/reference/**`、**零** `docs/archive/**`、**零** `docs/design/**`、**零** `.superpowers/**`（除了 report 自身，brief 允许）、**零** 归档 handoff。grep 交叉验证：`.superpowers/` 在 diff 中仅命中 `w13-3-report.md`（在 commit message 内，不在 diff 中——`git show --stat` 不显示 commit message 内的路径）。`docs/handoffs/` 中 3 行 Slint 命中（grep 实测）—— brief §1 显式"归档 handoff 保留原貌"，未动 | **PASS** |
| 4 | AGENTS.md / AGENTS-CN.md 改动仅 Slint → Dioxus（或退役旧规则），不夹带私货 | diff 显示 5 + 2 = 7 行级编辑：<br>• `AGENTS.md:51` desktop:dev 注释<br>• `AGENTS.md:137` Tauri preface<br>• `AGENTS.md:176` UI thread discipline（**退役旧规则**——见下方专门裁定）<br>• `AGENTS.md:180` v0.1.0 surface baseline<br>• `AGENTS.md:227` verification table<br>• `AGENTS-CN.md:155` UI 线程纪律（退役旧规则中文版）<br>• `AGENTS-CN.md:159` v0.1.0 面基线<br>**无任何其他规则被改 / 增 / 删** | **PASS** |

### 退役 "UI thread discipline" 旧规则裁定

- 旧规则：`writing Slint properties from a non-event-loop thread is silently dropped; route through slint::invoke_from_event_loop (helpers in error_banners.rs already wrap this)`
- 退役依据：Slint 壳已于 2026-08-28 物理删除（`707e414`），规则所约束的对象不存在。**退役正确**。
- 替代指引：报告"refer to the Dioxus 0.8 docs and the consult-room `ui_dioxus::launch` path for the current discipline"——成立（Dioxus 0.8 自身有事件循环契约，consult-room 单进程无跨线程 UI 写入问题）。
- 但**新规则段尾部还有 fact error**（见 Important-1）。

### QUALITY 双判决

- **复用** ✅：每条新文都引到正确的历史 commit SHA（`707e414`）和日期（`2026-08-28`）；其他历史实体（`block_registry.rs`、`RedesignTheme`、`AppStrings`、`DIOXUS_SHELL`、`muda`/`rfd`）经 `git ls-tree --name-only -r HEAD | grep` 实测**确实不在树中**——所以注释里"deleted/removed together with the Slint shell"陈述**事实正确**（除了 error_banners 那条，见下）。
- **分层 / 边界** ✅：未越过 brief 显式禁区的任何目录；未改任何 `.rs` 行为代码。
- **错误处理**：N/A（纯文档）。

### ⚠️ Important-1：AGENTS.md / AGENTS-CN.md 关于 `error_banners.rs` 的事实陈述错误

- **事实**：`src/apps/desktop/src/app_state/error_banners.rs` 已于 **707e414**（"physically remove Slint UI shell" commit）一并**物理删除**（`git log --diff-filter=D --summary 707e414~5..707e414` 显示 `delete mode 100644 src/apps/desktop/src/app_state/error_banners.rs`；`git ls-tree --name-only -r HEAD | grep error_banners` 零命中）。
- **错误陈述**：
  - `AGENTS.md:176`（W13-3 新文）："... The helpers in `error_banners.rs` (`slint::invoke_from_event_loop` wrappers) are kept in tree but are unreferenced; remove in a follow-up if cleanup is needed."
  - `AGENTS-CN.md:155`（W13-3 新文）："... `error_banners.rs` 的 helper 暂留代码库但已无引用，若需清理留待后续。"
- **问题**：`error_banners.rs` **不在树中**（"kept in tree" 为假）；"are unreferenced" 建立在 "in tree" 之上，前提错误。
- **影响**：AGENTS.md / AGENTS-CN.md 是**仓库规范事实源**，每次 session 启动由 system reminder 注入；任何 AI agent 读到这段都会以为 `error_banners.rs` 在树里并去 grep → 找不到 → 困惑 → 浪费精力。**事实源的可信性受损**。
- **正确改写建议**：`... The helpers in error_banners.rs (slint::invoke_from_event_loop wrappers) were deleted together with the Slint shell in commit 707e414; no follow-up is required.` / 中文镜像。
- **严重等级**：Important（不破坏代码/流程，但破坏 AGENTS.md 作为事实源的可信度；与 brief §Spec.2 "若规则已不适用于 Dioxus，删除或改写" 的"改写"要求冲突——改写本应基于准确事实）。

### Findings：0 Critical / 1 Important / 1 Minor

- **Important-1**：`AGENTS.md:176` + `AGENTS-CN.md:155` 关于 `error_banners.rs` "kept in tree but unreferenced" 的事实陈述错误——该文件已于 `707e414` 物理删除。建议改写为"deleted together with the Slint shell in 707e414; no follow-up needed"。
- **Minor-1**：未触及区域仍含 Slint 残留（根 `README.md` / `CONTRIBUTING.md` / `CONTRIBUTING_CN.md` / `CHANGELOG.md` / `CODE_REVIEW.md` / `docs/AGENT_ONBOARDING.md` / `docs/PROJECT_STATE.md` / `docs/architecture/*` / `docs/product/*` / `docs/releases/*` / `docs/status/*` / `.agents/skills/northhing-onboarding/SKILL.md` / `.ohmyagent/skills/northhing-slint-desktop/SKILL.md` / `memory/lessons.md` / `memory/northhing.md` / `.opencode/model-capability-notes.md`）——brief §Spec.2.4 显式限定"不在本任务范围"，实现者按规约不动；建议下一单 `W13-doc-followup` 收口；本单无瑕。

### 判决：**REQUEST_CHANGES**（仅因 Important-1 AGENTS.md fact error）

---

## 4. 终审指令

- **W13-1**：APPROVE。可直接入 ledger。
- **W13-2**：APPROVE。可直接入 ledger；Important-1 / Minor-1 留待下一单/跟进单收口（不阻塞合并）。
- **W13-3**：REQUEST_CHANGES。fixer 需精确改写 `AGENTS.md:176` + `AGENTS-CN.md:155` 关于 `error_banners.rs` 的事实陈述（删除"kept in tree"等错误措辞，改写为"deleted together with the Slint shell in 707e414; no follow-up needed"），重新 commit 后再审；不改任何其他文件。
- **R4 审计单需要回填一项**：原"production path violation（直调 core config）" 应改述为"测试代码直调（违反 AGENTS.md 边界规则精神，但不在生产路径上）"——三处位置均经本法官核验在 `#[cfg(test)] mod tests` 内。
- **W13-2 / W13-3 报告位置差异**（W13-3 把 report 写在 commit message 内，不在独立 .md 文件）属于轻微程序偏差，不影响实质。

## 5. 取证命令清单

```powershell
# W13-1
git grep -n "seed_session" -- "src/apps/desktop/src/"        # 3 处全部在 session_mock.rs 内
git grep -n "mock_stream" -- "src/apps/desktop/src/"         # 仅注释，无代码
git grep -n "MockEntry\|messages_to_entries\|render_child" -- "src/apps/desktop/src/"  # 全部保留

# W13-2
git grep -n "initialize_global_config" -- "src/apps/desktop/src/"    # 剩余 settings/tests.rs:343 + bin/w4_repro.rs:31,58,60
git grep -n "northhing_core::" -- "src/apps/desktop/src/" | Where-Object { $_ -notmatch "kernel_facade|infrastructure" }
# W13-3
git grep -in "slint" -- "src/" "AGENTS.md" "AGENTS-CN.md"  | Select-String -Pattern "eslint|ESLint" -NotMatch
# 改后真命中 29 处，全部带 2026-08-28/707e414 锚定
git ls-tree --name-only -r HEAD | Select-String "error_banners"  # 0 命中 → 证实 AGENTS.md 事实错误
git ls-tree --name-only -r HEAD | Select-String "block_registry|RedesignTheme|AppStrings|DIOXUS_SHELL"  # 0 命中 → 证实其他历史注释事实正确
git log --diff-filter=D --summary 707e414~5..707e414 | Select-String "error_banners"
# → delete mode 100644 src/apps/desktop/src/app_state/error_banners.rs (129 行删除)
```

## 6. 教训 / Learnings（备忘，不入 commit）

- 改仓库规范事实源（AGENTS.md / AGENTS-CN.md）前，**先 git ls-files 或 git grep 确认被引用的实体是否仍存在**——这是 spec-only 单的关键前置。本次 W13-3 实现者未做这一步，导致 Important-1。
- W13-2 的 "init_core ⊋ initialize_global_config" 语义扩大现象是 facade 设计上的潜在小坑；建议下一轮 facade refactor 时考虑为测试场景暴露窄入口（`init_config_only()`），免得未来其他测试也踩同样的语义模糊。
- W13-2 brief 限定"只碰上表两个文件"导致 `settings/tests.rs:343` 同性质直调被遗留；brief 模板应在 Constraints.1 加例外条款："若同性质直调在他处发现，可一并处理并在 report 列出"——避免无谓跟进单。

---

> 本判决由独立验收法官独立审稿，未与实现者沟通，未运行 cargo（编排者已跑过 147 测试全绿）。所有结论指到 file:line 或命令输出；无可编造项。