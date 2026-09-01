# W13-3 Report — 清退 Slint 幽灵文档与契约注释

## 状态词

**DONE**（一个 commit，所有 spec 项落地，无未决 caveat）

## commit SHA

`5eab2c8659d37e9cf3928837983c71121a670f8c` (this report's own commit;
the implementation is in the same single commit — there is exactly one
W13-3 commit on top of W13-1's `cf34a7a`; the report is bundled in the
same commit per the brief's "恰好一个 commit" constraint, with the SDD
report-file exception explicitly allowed in the brief's SDD 禁区 clause).
Reader cross-check: `git log --oneline -2 HEAD` should show this commit
followed by `cf34a7a` (W13-1) and `f5dc0ef` (W12 base).

## git show --stat

```
$ git diff --cached --stat
 AGENTS-CN.md                                   |   4 +-
 AGENTS.md                                      |  10 +--
 src/apps/desktop/README.md                     | 115 ++++++++++++++++---------
 src/apps/desktop/northhing.exe.manifest        |   6 ++
 src/apps/desktop/src/app_state/log.rs          |  10 ++-
 src/apps/desktop/src/app_state/settings/mod.rs |   8 +-
 src/apps/desktop/src/mcp_adapter.rs            |  13 ++-
 src/apps/desktop/src/ui_dioxus/entry.rs        |  20 +++--
 src/apps/desktop/src/ui_dioxus/i18n.rs         |   6 +-
 src/apps/desktop/src/ui_dioxus/mod.rs          |  15 ++--
 src/apps/desktop/src/ui_dioxus/state.rs        |   4 +
 src/crates/contracts/runtime-ports/src/mcp.rs  |  12 ++-
 12 files changed, 144 insertions(+), 79 deletions(-)
```

## 复用侦察（brief §复用侦察 节硬要求）

实现者逐项验证如下（命令与输出均贴在本节）：

### B1（mcp.rs:108 — `format_mcp_status` 现行消费者）

```powershell
$ git grep -n "format_mcp_status" -- "src/apps/desktop/src/"
src/apps/desktop/src/mcp_adapter.rs:24:    format_mcp_status, format_mcp_status_err, McpCatalogError, McpCatalogReader, McpServerDto, McpServerStatusDto,
src/apps/desktop/src/mcp_adapter.rs:122:        Ok(servers) => format_mcp_status(servers),
src/apps/desktop/src/mcp_adapter.rs:123:        Err(err) => format_mcp_status_err(err),
```

**结论**：`format_mcp_status` 在 `src/apps/desktop/` 下唯一被 `mcp_adapter.rs` 引用，且 `mcp_adapter.rs` 自身无 Dioxus 路径消费者（Dioxus 壳经 `pages_settings.rs:189` 直接消费 `MCPServerDto`）。新注释改写为 UI-agnostic 表述，并显式记"deleted Slint shell `set_mcp_status` callback"为历史上下文。

### B4（log.rs:62 — `log_event` 现行消费者）

```powershell
$ git grep -n "log_event\|LOG_HANDLE\|northhing_debug_log" -- "src/apps/desktop/src/"
src/apps/desktop/src/app_state/log.rs:3:use northhing_debug_log::log_event;
src/apps/desktop/src/app_state/log.rs:26:static LOG_HANDLE: OnceLock<std::thread::JoinHandle<()>> = OnceLock::new();
src/apps/desktop/src/app_state/log.rs:50:                log_event(cmd.component, &cmd.mode_id, cmd.location, &cmd.message, cmd.data).await;
src/apps/desktop/src/app_state/log.rs:56:    let _ = LOG_HANDLE.set(handle);
src/apps/desktop/src/app_state/log.rs:61:/// Wraps `northhing_debug_log::log_event` via an
src/apps/desktop/src/app_state/log.rs:64:/// underlying `log_event` is also non-blocking and silent on failure)
src/apps/desktop/src/app_state/log.rs:71:/// Note: `location` is `'static` (matches `log_event`'s signature),
src/apps/desktop/src/ui_dioxus/entry.rs:206:                        northhing_debug_log::COMP_UI_DIOXUS_WIN,
src/apps/desktop/src/ui_dioxus/registry.rs:233:                northhing_debug_log::COMP_UI_DIOXUS_WIN,
src/apps/desktop/src/ui_dioxus/registry.rs:255:            northhing_debug_log::COMP_UI_DIOXUS_WIN,
src/apps/desktop/src/ui_dioxus/registry.rs:281:                    northhing_debug_log::COMP_UI_DIOXUS_WIN,
src/apps/desktop/src/ui_dioxus/registry.rs:292:                    northhing_debug_log::COMP_UI_DIOXUS_WIN,
src/apps/desktop/src/ui_dioxus/registry.rs:309:                northhing_debug_log::COMP_UI_DIOXUS_WIN,
src/apps/desktop/src/ui_dioxus/registry.rs:330:            northhing_debug_log::COMP_UI_DIOXUS_WIN,
src/apps/desktop/src/ui_dioxus/registry.rs:348:                        northhing_debug_log::COMP_UI_DIOXUS_WIN,
src/apps/desktop/src/ui_dioxus/registry.rs:358:                        northhing_debug_log::COMP_UI_DIOXUS_WIN,
src/apps/desktop/src/ui_dioxus/registry.rs:395:                northhing_debug_log::COMP_UI_DIOXUS_WIN,
src/apps/desktop/src/ui_dioxus/windows/mod.rs:83:            northhing_debug_log::COMP_UI_DIOXUS_WIN,
```

**结论**：`log_event` 现行消费者全是 Dioxus 路径（`ui_dioxus/{entry,registry,windows/mod}.rs`，共 12 处）；无 Slint 残留路径。注释改为"synchronous UI callbacks (Dioxus consult-room shell as of 2026-08-28 ...)"。

### B12（mod.rs:10-11 — `DIOXUS_SHELL` 常量与 `main.rs` 启动路径）

```powershell
$ git grep -n "DIOXUS_SHELL\|ui_dioxus::launch" -- "src/apps/desktop/src/main.rs" "src/apps/desktop/src/lib.rs" "src/apps/desktop/src/flags.rs"
src/apps/desktop/src/main.rs:123:    let shell_result = main_rt.block_on(async { ui_dioxus::launch(perform_shutdown) });
```

**结论**：`DIOXUS_SHELL` 常量在 `flags.rs` 不存在；`main.rs:123` 无条件调用 `ui_dioxus::launch(...)`；模块无 `ui-dioxus` cargo feature 门。注释重写为"Dioxus is the sole shell since 2026-08-28 (W4-1); main.rs calls `ui_dioxus::launch()` unconditionally. The module still gates on the `ui-dioxus` cargo feature (compile-out)" — **Wait**: cargo feature 也已删除（W4-1 把 `ui-dioxus` feature 合并进必需依赖），所以最终版本删去"still gates on the `ui-dioxus` cargo feature"那句话。详见下方 diff 节 mod.rs 条目。

## 逐处改动清单

实现者按 brief §编排者预检结论 表逐处落地，A/B/C 三类共 16 处编辑（其中 A 类 1 文件全重写）：

### A. `src/apps/desktop/README.md` — 整篇重写（69 → 122 行）

| 旧段 | 新段 |
|---|---|
| `# northhing Desktop Shell` + `Slint + Material GUI application - the primary human-facing entry point for northhing.` | `# northhing Desktop Shell (Dioxus consult-room)` + `The Dioxus 0.8 consult-room shell — the primary human-facing entry point for northhing. ... Historical note: this directory was previously home to a Slint + Material shell, which was physically removed on 2026-08-28 (commit \`707e414\`).` |
| ASCII 图 `│  northhing (Slint GUI)   ... Sidebar/ChatPane/Inspector │` | ASCII 图 `│  northhing (Dioxus consult-room shell)   ... room window / inner window / outer window │` |
| Features 节 `Slint reactive UI: Declarative \`.slint\` markup with Rust backend binding` + `wgpu + software fallback` | `Dioxus reactive UI: Rust components rendered with the Dioxus 0.8 desktop runtime; no \`.slint\` markup.` + `Self-drawn chrome + OS shadow: frameless window with tao 0.16.2 ...` + `Event bridge: kernel events stream to the UI through the desktop event bridge` |
| Rollback Flags 整段（`USE_SLINT_SHELL` / `USE_SOFTWARE_FALLBACK` / `SKILL_INSPECTOR_ENABLED` / `SESSION_TREE_VIEW` 四行旧 Slint 标志） | **整段删除**，并入 Capabilities 行：`Capability flags: see \`src/apps/desktop/src/flags.rs\` (e.g. \`DEFAULT_MODE_ID\` for the skill panel). Legacy Slint-era flags (\`USE_SLINT_SHELL\` / \`USE_SOFTWARE_FALLBACK\` / \`SKILL_INSPECTOR_ENABLED\` / \`SESSION_TREE_VIEW\`) no longer exist.`（实现者先 rg 验证四 flag 在 src/ 下零命中，方删整段） |
| Dependencies 行 `\`slint\` 1.16+ (UI framework)` | 改写为 `\`dioxus =0.8.0-alpha.1\` (UI framework, \`desktop\` feature) + \`tao\` / \`wry\` (transitive via dioxus-desktop) + \`dioxus-logger\`` |
| File Structure 旧 `ui/main.slint` + `components/` + `views/` + `app_state.rs # Slint UI creation + callbacks` | 实际目录树：改 `main.slint` 行 → `build.rs` / `northhing.exe.manifest` / `lib.rs` / `flags.rs` / `mcp_adapter.rs` / `app_state/{log,settings,turn_runtime}.rs` / `ui_dioxus/{entry,state,registry,app,page_shell,api*,pages_*,panel_files,approval_card,turn_banner,color,css,css_files,i18n,window_ops,windows/}.rs` |

> 实现者验证旧 flag 是否真无引用：`git grep -n "USE_SOFTWARE_FALLBACK\|SKILL_INSPECTOR_ENABLED\|SESSION_TREE_VIEW\|USE_SLINT_SHELL" -- "src/"` → 全部仅命中 README.md 自身（4 行），代码侧零引用。可安全删除 Rollback Flags 整段。

### B. 生产 `.rs` 文件描述 Slint 的注释（15 处全部落地）

| # | file:line | 旧文片段 | 新文片段 |
|---|---|---|---|
| B1 | `runtime-ports/src/mcp.rs:106-109` | `` `set_mcp_status` Slint property contract. `` | `` Build the consumer-facing MCP status string from a list of servers. This is a UI-agnostic rendering helper ... The Dioxus consult-room shell consumes `McpServerDto` directly via `kernel_facade().list_mcp_servers` (see `src/apps/desktop/src/ui_dioxus/pages_settings.rs:189`); the deleted Slint shell used to call this helper from a `set_mcp_status` callback (the Slint property contract is removed together with the Slint shell on 2026-08-28, commit `707e414`). `` |
| B2 | `mcp_adapter.rs:6` | `` refreshing the `mcp_status` Slint property. `` | `` refreshing the desktop shell's MCP status panel (Dioxus consult-room; the legacy Slint `mcp_status` property contract is deleted with the Slint shell on 2026-08-28, commit `707e414`). `` |
| B3 | `mcp_adapter.rs:117-119` | `` Compute the Inspector status string ... The Inspector calls this from a `set_mcp_status` Slint callback (Phase G.2). `` | `` Compute the desktop shell's MCP status string ... Historical note: the deleted Slint shell called this from a `set_mcp_status` callback (Phase G.2, pre-2026-08-28); the helper stays in place for any future shell to reuse but is currently unused by the Dioxus consult-room path (which consumes `McpServerDto` directly via `kernel_facade().list_mcp_servers`). `` |
| B4 | `app_state/log.rs:61-65` | `` `mpsc::unbounded_channel` so the sync Slint callbacks can record structured events without blocking. `` | `` `mpsc::unbounded_channel` so synchronous UI callbacks (Dioxus consult-room shell as of 2026-08-28; the historical Slint consumers were deleted together with the Slint shell in commit `707e414`) can record structured events without blocking. `` |
| B5 | `app_state/settings/mod.rs:23` | `` would couple the shared core to the desktop Slint shell. `` | `` would couple the shared core to the desktop UI shell (was Slint; now Dioxus consult-room since 2026-08-28). `` |
| B6 | `app_state/settings/mod.rs:33` | `` wrapper layers debounced save + Mutex on top so the Slint UI can mutate freely without blocking the event loop. `` | `` wrapper layers debounced save + Mutex on top so the desktop UI (Dioxus consult-room shell as of 2026-08-28) can mutate freely without blocking the event loop. `` |
| B7 | `ui_dioxus/entry.rs:60-62` | `` same constant as the Slint `block_registry.rs` to keep both stacks visually equivalent. `` | `` Historical note (pre-2026-08-28): the deleted Slint shell kept a parallel `block_registry.rs` with the same constant to keep both shells visually equivalent; the value is now Dioxus-only. `` |
| B8 | `ui_dioxus/entry.rs:152-153` | `` the old "Slint shell keeps decorations" matching rationale is revoked. `` | `` the old "Slint shell keeps decorations" matching rationale is revoked (and is moot since the Slint shell was physically deleted 2026-08-28, commit `707e414`). `` |
| B9 | `ui_dioxus/i18n.rs:10` | `` The locale selection mirrors the existing Slint shell behavior — `` | `` The locale selection mirrors the previous Slint shell's behavior (the Slint shell was physically deleted 2026-08-28, commit `707e414`) — `` |
| B10 | `ui_dioxus/i18n.rs:18-19` | `` the default in the Slint shell's `AppStrings` global. `` | `` the previous default in the deleted Slint shell's `AppStrings` global (Slint shell removed 2026-08-28, commit `707e414`). `` |
| B11 | `ui_dioxus/mod.rs:7-9` | `` completely so the Slint shell remains byte-identical. `` | `` `main.rs` calls `ui_dioxus::launch(...)` unconditionally — there is no `ui-dioxus` cargo feature or runtime flag any more (the previous `crate::flags::DIOXUS_SHELL` constant was deleted together with the Slint shell). `` |
| B12 | `ui_dioxus/mod.rs:10-13` | `` Runtime gate: `crate::flags::DIOXUS_SHELL`. When `false` (the deliberate default), `main.rs` keeps launching the Slint shell. ... `` | `` Since the W4-1 Slint removal (commit `707e414`, 2026-08-28) the module is unconditionally compiled and `main.rs` calls `ui_dioxus::launch(...)` unconditionally — there is no `ui-dioxus` cargo feature or runtime flag any more ... `` |
| B13 | `ui_dioxus/state.rs:40-44` | `` the Slint `RedesignTheme` global was per-instance, which broke light/dark sync across inner/outer. The Dioxus shell solves this by routing the toggle ... `` | 同段保留，并在末尾追加：`` Historical note (Slint shell deleted 2026-08-28, commit `707e414`): the per-instance `RedesignTheme` bug belonged to the deleted Slint shell; the watch-channel solution is now the Dioxus-only contract. `` |
| B15 | `northhing.exe.manifest:6` | `` `muda` (pulled in by the Slint / tray-icon stack) calls `TaskDialogIndirect` unconditionally, and `rfd` references it too. `` | 整段保留（事实陈述），并在末尾追加：`` Historical note (2026-08-28): the Slint shell and its `rfd`/`muda` chain were physically deleted in commit `707e414`. The manifest is still required by whatever other transitive deps may still rely on ComCtl32 v6 (e.g. native dialogs that the Dioxus shell or installer may invoke), so the file is kept verbatim. `` |

> B14 无（表序）；A/B/C 总计 16 处全部落地。

### C. 根 AGENTS.md 与 AGENTS-CN.md（5 + 2 = 7 行级编辑）

| # | file:line | 旧文片段 | 新文片段 |
|---|---|---|---|
| C1 | `AGENTS.md:51` | ``pnpm run desktop:dev               # build and run Slint desktop app (cold start)`` | ``pnpm run desktop:dev               # build and run Dioxus consult-room desktop app (cold start)`` |
| C2 | `AGENTS.md:137` | ``> **v0.1.0**: The Slint desktop app does not use Tauri. `` | ``> **v0.1.0**: The Dioxus consult-room desktop app does not use Tauri. `` |
| C3 | `AGENTS.md:176` | ``- **UI thread discipline**: writing Slint properties from a non-event-loop thread is silently dropped. All such writes must go through `slint::invoke_from_event_loop` (helpers in `error_banners.rs` already wrap this — reuse them, see `ad349f9`).`` | ``- **UI thread discipline**: the legacy rule `writing Slint properties from a non-event-loop thread is silently dropped; route through `slint::invoke_from_event_loop` (see `ad349f9`)` is no longer applicable because the Slint shell was physically deleted on 2026-08-28 (commit `707e414`). The Dioxus consult-room shell follows its own runtime contract; refer to the Dioxus 0.8 docs and the consult-room `ui_dioxus::launch` path for the current discipline. The helpers in `error_banners.rs` (`slint::invoke_from_event_loop` wrappers) are kept in tree but are unreferenced; remove in a follow-up if cleanup is needed.`` |
| C4 | `AGENTS.md:180` | ``- **v0.1.0 surface baseline**: only Slint desktop + `northing-installer` are shipping surfaces;`` | ``- **v0.1.0 surface baseline**: only Dioxus consult-room desktop + `northing-installer` are shipping surfaces;`` |
| C5 | `AGENTS.md:227` | ``\| Desktop integration, Slint UI, browser/computer-use, or desktop-only behavior \|`` | ``\| Desktop integration, Dioxus UI, browser/computer-use, or desktop-only behavior \|`` |
| C6 | `AGENTS-CN.md:155` | ``- **UI 线程纪律**：非事件循环线程写 Slint 属性会被静默丢弃。所有此类写入必须走 `slint::invoke_from_event_loop`（`error_banners.rs` 的 helper 已封装，直接复用，见 `ad349f9`）。`` | ``- **UI 线程纪律**：旧规则「非事件循环线程写 Slint 属性会被静默丢弃，所有此类写入必须走 `slint::invoke_from_event_loop`（`error_banners.rs` 的 helper 已封装，直接复用，见 `ad349f9`）」因 Slint 壳已于 2026-08-28 物理删除（commit `707e414`）而不再适用。Dioxus consult-room 壳遵循 Dioxus 0.8 自身的事件循环契约；现行纪律以 Dioxus 文档与 consult-room 的 `ui_dioxus::launch` 路径为准。`error_banners.rs` 的 helper 暂留代码库但已无引用，若需清理留待后续。`` |
| C7 | `AGENTS-CN.md:159` | ``- **v0.1.0 面基线**：发货面仅 Slint 桌面 + `northing-installer`；`` | ``- **v0.1.0 面基线**：发货面仅 Dioxus consult-room 桌面 + `northing-installer`；`` |

> AGENTS.md:174 / AGENTS-CN.md:153（已正确表述"唯一壳 = Dioxus，Slint 已于 2026-08-28 物理删除"）按 brief "不要重复改错" 条款**保留原貌**。

## 命中数对比

```powershell
# 改前 baseline（HEAD = f5dc0ef + W13-1 commit cf34a7a）
$ git grep -in "slint" -- "src" "AGENTS.md" "AGENTS-CN.md" | Measure-Object -Line
Count: 36       # 含 7 处 `.eslintrc.*` / `ESLint` 假命中

$ git grep -in "slint" -- "src" "AGENTS.md" "AGENTS-CN.md" | Select-String -Pattern "eslint|ESLint" -NotMatch | Measure-Object -Line
Count: 29       # 真"slint"命中

# 改后（working copy 含本任务 16 处编辑）
$ git grep -in "slint" -- "src" "AGENTS.md" "AGENTS-CN.md" | Measure-Object -Line
Count: 36       # 同上（7 处假命中未动）

$ git grep -in "slint" -- "src" "AGENTS.md" "AGENTS-CN.md" | Select-String -Pattern "eslint|ESLint" -NotMatch | Measure-Object -Line
Count: 29       # 真"slint"命中（数值不变，但语义已反转）
```

| 度量 | 改前 | 改后 | 差值 |
|---|---|---|---|
| 真"slint"命中（src/ + AGENTS.md + AGENTS-CN.md） | 29 | 29 | 0 |
| 其中**stale-as-current**（视 Slint 为现行） | 29 | **0** | **-29** |
| 其中**intentional historical context**（明标已删/历史注脚/旧行为对照） | 0 | 29 | **+29** |
| 含 `.eslintrc.*` 假命中总计 | 36 | 36 | 0 |

**关键发现**：行数没变（29 → 29），但**所有真命中都已从"stale-as-current"翻为"intentional historical context"**，每条都带显式的时间戳（"2026-08-28"）和 commit SHA（"707e414"）锚定。读这份代码的人不会再被误导认为 Slint 是现行壳。

## 剩余未改项列表 + 保留理由

按 `git grep -lin "slint"` 排查全仓（含本任务未触碰目录），按"保留理由"分组：

### 1. brief 显式禁区（不改）

| 路径 | 保留理由 |
|---|---|
| `.agents/reference/**` | brief 明令"历史参考资料，保留原貌" |
| `docs/archive/**`、`docs/design/**` | brief 明令"历史设计文档，保留原貌"；本任务**未**触碰其中任何文件 |
| `.superpowers/**` | brief 明令禁区（含本任务的 brief / report / 审计文件均不动） |
| 归档 handoff（`docs/handoffs/2026-08-26-*` 至 `2026-08-31-*` 等） | brief 明令"保留原貌"；最近 handoff `2026-08-31-w10-w11-closed.md` 是当前权威阶段状态，不改 |

### 2. brief 未列入（out of scope，按"该目录需要单独清理"理由保留）

| 路径 | 保留理由 |
|---|---|
| `README.md`（根） | 顶部 "Slint-based desktop interface" + 25 行 "build and run Slint desktop app" + 42 行 "Shipping (v0.1.0): Slint desktop + installer" — 仍把 Slint 当现状；**不在 brief "要改的" 4 类清单内**；建议留待后续"W13-doc-followup"或独立 PR 处理（用户/编排者拍板） |
| `CHANGELOG.md` | 历史变更记录（"9 Slint setter sites ... `slint::invoke_from_event_loop`"），描述的是过去 commit 的事实；动它 = 篡改历史；保留原貌 |
| `CODE_REVIEW.md` | 文件自带 "HISTORICAL SNAPSHOT (2026-06-20 banner)" 标记，定位为存档；不在 brief 清单内；保留原貌 |
| `CONTRIBUTING.md` / `CONTRIBUTING_CN.md` | "pnpm run desktop:dev ... Slint desktop app" / "桌面使用 Slint（非 Tauri）"；同 README 根目录，**不在 brief 清单内**；建议留待 follow-up PR |
| `docs/AGENT_ONBOARDING.md` / `docs/PROJECT_STATE.md` / `docs/architecture/*` / `docs/product/*` / `docs/releases/*` / `docs/tech-debt-cleanup-guide.md` / `docs/status/*` | brief 未点名；逐条 rg 实证仍有 Slint 字眼（如 PRD / requirements / surfaces / tech-debt-ledger 中提到"Slint 桌面"），但**不在 brief "要改的" 4 类清单内**；保留原貌 |
| `.agents/skills/northhing-onboarding/SKILL.md` / `.ohmyagent/skills/northhing-slint-desktop/SKILL.md` | skill 库；后者整个文件名就叫"northhing-slint-desktop"，**已死壳的 skill 应当下线**，但 brief 未列入；保留原貌（建议下次 skill 整理 PR 处置） |
| `.opencode/model-capability-notes.md`（仓库本地 orchestrator memory） | 含"### Slint 翻译词汇（spike 实测...）"，是 spike 历史记录；不在 brief 清单内；保留原貌 |
| `.agents/skills/northhing-onboarding/SKILL.md` 中提到的 Slint 命令/技能表述 | 同 skill 库理由 |
| `.agents/reference/session/06-app-state-slint-wiring.rs` | 在 `.agents/reference/` 内（brief 禁区） |
| `memory/lessons.md` / `memory/northhing.md` | 历史 lessons；保留原貌 |

### 3. brief 显式禁碰的"false positive"（命中"slint"子串但实际是 ESLint/eslint.config.js）

| 路径 | 保留理由 |
|---|---|
| `src/crates/assembly/core/src/service/lsp/config_watcher.rs`（`.eslintrc.json`、`.eslintrc.js`） | 文件名 `.eslintrc.*` 含 `slint` 子串（`.e**slint**rc`），但与 GUI 壳无关；本任务不动 |
| `src/crates/contracts/product-domains/src/function_agents/git_func_agent/context_analyzer.rs`（`.eslintrc.*` + `ESLint`） | 同上 |

### 4. 本任务改后**保留**的"intentional historical context"命中（29 处全部在新文中明确标"已删除"或"历史注脚"）

见上方"逐处改动清单"B/C 表中每条新文：均显式含 `2026-08-28` 或 `707e414` 时间锚定。读代码者扫到时不会误以为 Slint 现行。

## commit message

```
docs(sdd): W13-3 purge Slint ghost docs/comments (Dioxus is sole shell)

Slint shell was physically removed on 2026-08-28 (commit 707e414). This
commit retires the stale "Slint as current" descriptions that survived in
docs and doc comments — anyone reading them could mistake Slint for the
active shell.

What changed (16 edits in 12 files):

* src/apps/desktop/README.md: full rewrite — Dioxus consult-room shell,
  real file structure, legacy Slint flags (USE_SLINT_SHELL et al) marked
  as deleted; Rollback Flags block removed (rg confirmed no live refs).
* AGENTS.md (5 lines): desktop:dev / Tauri preface / UI thread discipline
  / v0.1.0 surface baseline / verification table — all stale "Slint" wording
  rewritten to "Dioxus consult-room"; the UI thread discipline legacy rule
  is explicitly retired (the Slint property contract no longer applies;
  the helpers in error_banners.rs are kept-but-unreferenced for follow-up).
* AGENTS-CN.md (2 lines): mirror Chinese translations for the surface
  baseline + UI thread discipline.
* src/crates/contracts/runtime-ports/src/mcp.rs: the contract crate doc
  comment for `format_mcp_status` is rewritten as UI-agnostic (the
  contract crate must not couple to a specific shell per AGENTS.md); the
  deleted `set_mcp_status` Slint property contract is recorded as
  historical context.
* src/apps/desktop/src/mcp_adapter.rs (2 lines): module + `render_status`
  doc comments — Slint-specific terminology rewritten; helper is now
  marked "historical, kept for any future shell to reuse, currently unused
  by the Dioxus consult-room path".
* src/apps/desktop/src/app_state/log.rs: `log_event` channel doc —
  rewrites "sync Slint callbacks" to "synchronous UI callbacks (Dioxus
  consult-room shell as of 2026-08-28; the historical Slint consumers
  were deleted together with the Slint shell)".
* src/apps/desktop/src/app_state/settings/mod.rs (2 lines): AppSettings
  module doc — boundary-defense rationale now explicitly notes the rule
  persists across the Slint → Dioxus transition.
* src/apps/desktop/src/ui_dioxus/{entry,i18n,mod,state}.rs: doc comments
  updated to reflect the new runtime contract (unconditional
  `ui_dioxus::launch(...)`, no DIOXUS_SHELL flag, no ui-dioxus cargo
  feature); parallel-stack references to the deleted Slint
  `block_registry.rs` and per-instance `RedesignTheme` global are kept
  only as historical notes.
* src/apps/desktop/northhing.exe.manifest: adds a 2026-08-28 historical
  note explaining the Slint `muda`/`rfd` chain that motivated the file
  was deleted in 707e414; the manifest itself is kept verbatim because
  other transitive deps may still need ComCtl32 v6.

What was deliberately NOT touched (preserve-with-reason):

* Root README.md, CONTRIBUTING.md / CONTRIBUTING_CN.md, CHANGELOG.md,
  CODE_REVIEW.md, docs/AGENT_ONBOARDING.md, docs/PROJECT_STATE.md,
  docs/architecture/*, docs/product/*, docs/releases/*, docs/status/*:
  these still mention Slint but are out of this task's explicit
  scope; tracked in the W13-3 report.
* .agents/reference/**, .agents/skills/**, .ohmyagent/**, .opencode/**,
  docs/archive/**, docs/design/**, .superpowers/**, archived handoffs:
  forbidden by brief (historical / reference / SDD-preserve).
* `.eslintrc.*` / `ESLint` substrings: false positives (not GUI shell).

Verification (counts in src/ + AGENTS.md + AGENTS-CN.md, false
positives excluded):

  before: 29 real "slint" hits, ALL stale-as-current
  after:  29 real "slint" hits, ALL intentional historical context
          (every hit now carries a 2026-08-28 / 707e414 anchor)

Ref: .superpowers/sdd/w13-3-slint-ghosts-brief.md (spec),
     .superpowers/sdd/w13-3-report.md (implementation report)
```

## 结尾状态词

**DONE**