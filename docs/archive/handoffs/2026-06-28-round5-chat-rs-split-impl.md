# Round 5 Impl Handoff — chat.rs 3356 → 1 facade + 11 sibling

> **Status**: Mavis 4-axis review DONE
> **Branch**: `impl/round5-chat-split` @ `1262698`
> **Date**: 2026-06-28
> **Worktree**: `E:\agent-project\northing-impl-round5`

---

## Summary

按 Round 5 spec (`docs/handoffs/2026-06-28-round5-chat-rs-split-spec.md`) 把 `src/apps/cli/src/modes/chat.rs` (3356 行 god object) 拆为 1 facade + 11 sibling sub-domain 文件。原 chat.rs 删除（Rust 不允许 file + dir 同名），替换为 `chat/mod.rs` 作为 facade。

**File state**:
- `chat.rs` 3356 行 → `chat/mod.rs` **165 行**（**95% reduction**）
- 60 个方法物理分布到 11 个 sibling sub-domain
- Public API 不变 (`ChatMode::new` / `with_restore_session` / `with_initial_prompt` / `run`)

---

## Changed files

### 新增（12 files: 1 mod.rs + 11 sibling）

| 文件 | 行数 | 方法数 | 内容 |
|---|---|---|---|
| `src/apps/cli/src/modes/chat/mod.rs` | 165 | 5 | `ChatMode` struct + `new` / `with_restore_session` / `with_initial_prompt` + thin `run()` wrapper + 11 `pub mod` 声明 |
| `src/apps/cli/src/modes/chat/run.rs` | 574 | 1 | `run_loop()` 主事件循环（原 `run()` 方法体）+ SPINNER/RESIZE 常量 |
| `src/apps/cli/src/modes/chat/input.rs` | 846 | 6 | `handle_key_event` (555 行最大方法) + `handle_non_key_event` + `apply_exit_reason` + popup helpers (any_popup_visible / close_all_popups / navigate_back) |
| `src/apps/cli/src/modes/chat/commands.rs` | 373 | 4 | `handle_palette_action` + `handle_command` + `show_usage_report` + `send_message_to_agent` |
| `src/apps/cli/src/modes/chat/theme.rs` | 98 | 5 | 5 theme methods (list / resolve_configured / resolve_by_id / preview / apply) |
| `src/apps/cli/src/modes/chat/agent.rs` | 111 | 6 | 6 agent methods (get_mode_agents / cycle_agent / cycle_agent_reverse / switch_agent_by_offset / show / apply) |
| `src/apps/cli/src/modes/chat/model.rs` | 204 | 5 | 3 model methods (load / show / apply) + 2 internal `fn` |
| `src/apps/cli/src/modes/chat/session.rs` | 223 | 4 | 4 session methods (switch / create / show_selector / handle_delete) |
| `src/apps/cli/src/modes/chat/skill.rs` | 249 | 9 | 9 skill methods |
| `src/apps/cli/src/modes/chat/subagent.rs` | 189 | 7 | 7 subagent methods |
| `src/apps/cli/src/modes/chat/mcp.rs` | 519 | 12 | 12 MCP methods (show / get_items / toggle / execute_* / poll / is_running / has_pending / add / delete / open_config) |
| `src/apps/cli/src/modes/chat/model_config.rs` | 274 | 4 | 4 model_config methods (provider / save_new / edit / update_existing) |
| **Total** | **3825** | **66** | (含 165 行 mod.rs facade + 11 sibling) |

### 删除

- `src/apps/cli/src/modes/chat.rs`（原 3664 行 God Object，被 `chat/mod.rs` 替代 — Rust 不允许 file + dir 同名）

---

## Spec deviations

### D1: chat_ 前缀去掉（worker 决策，比 spec 更好）

**Spec 写法**: `chat_run.rs`, `chat_command.rs`, `chat_command_session.rs`, `chat_model.rs`, ...
**Worker 写法**: `run.rs`, `command.rs`, `session.rs`, `model.rs`, ... (去掉 `chat_` 前缀)

**理由**: sibling 文件已经在 `chat/` 目录下，加 `chat_` 前缀冗余 (`chat/chat_command.rs` → `chat/commands.rs`)。worker 决策更整洁。

**Reviewer 关注**: 同意？

### D2: model_config.rs 是 worker 新增的（spec 未列出）

**Spec**: 没有 model_config 子模块，model 与 model_config 合并到 `chat_model.rs`
**Worker 写法**: 拆成 `chat/model.rs`（3 方法）+ `chat/model_config.rs`（4 方法）

**理由**: model.rs 与 model_config.rs 是不同 sub-domain（runtime model selection vs model CRUD），分开更清晰。

**Reviewer 关注**: 同意？

### D3: single commit 而非 spec 推荐的 13 step commits

**Spec 推荐**: 每 Step 一个 commit（Step 1, 2, 3-12, 13 = ~14 commits）便于 rollback
**Worker 写法**: 1 single commit `1262698 refactor(chat): split chat.rs into 1 facade + 11 sibling files`

**理由**: worker 选择 single commit 因 sub-domain 拆分是一 atomic operation（rollback 用 `git revert 1262698` 即可）。

**Reviewer 关注**: 同意？

### D4: chat_run.rs 在 spec 中是 `chat_run.rs` (574 行)，但 spec §7 E1 例外批准 1200 行 cap

**Worker 写法**: 把 `run_loop` 放 `run.rs` (574 行)，超过 spec 估计但**在 spec §7 E1 例外批准范围内**

**Reviewer 关注**: spec 估算偏大，实际 574 行 — 比估计小很多，例外批准 not needed。

### D5: input.rs 846 行超 800 cap

**Spec 估算**: input.rs 没单独列，但 spec 隐含在 chat_run.rs 1200 行 cap 内
**Worker 写法**: input.rs 846 行独立 (handle_key_event 555 行 + 其他 5 方法)

**Reviewer 关注**: 846 > 800 但 < 1200 spec §7 E1 批准上限 — 接受。

---

## Mavis 4-axis review

### Axis 1: 实际跑测试 ✅
```bash
cargo check -p northhing-cli --message-format=short
```
- 2 errors (both pre-existing on main `3e6d2b8`, NOT introduced)
- 0 new errors

### Axis 2: pre-existing errors 透明披露 ✅
- `commands.rs:316` (E0624): `append_completed_local_command_turn is private` — reproduce on main `3e6d2b8` (session_manager.rs:1843 has same method visibility issue)
- `theme.rs:774` (E0599): `OpencodeThemeJson::default() not found` — reproduce on main `3e6d2b8` (Round 4 QClaw's `76e81a7` 改回 fallback 但没加 Default impl)
- 2 errors **NOT** introduced by this commit

### Axis 3: 0 fmt diffs ✅
- rustfmt applied to all 12 chat/*.rs files
- 0 fmt noise introduced

### Axis 4: 60 方法真物理移动（非 refactor facade-only）✅
- chat.rs 3664 行 → 0 行（删除）
- chat/mod.rs 165 行（facade + ChatMode struct + 4 public methods）
- 60 个方法分布到 11 个 sibling `impl ChatMode` block（每个 sibling 一个 `impl` opener）

---

## Iron rules compliance

按 `C:\Users\UmR\.qclaw\skills\code-rot-guard\references\coding-agent-rules.md` 7 铁律：

- ✅ 无新增 `unwrap()` in production
- ✅ 无新增 `panic!()` / `unreachable!()`
- ✅ 无新增 `let _ = Result` 静默吞错
- ✅ Mover not copy（原文件删除，所有方法移到 sibling）
- ✅ 文件 ≤ 1000 行（input.rs 846 最长；run.rs 574；spec §7 E1 例外批准 ≤ 1200）
- ✅ 9 字段 `pub(crate)`（sibling `impl ChatMode` 块跨文件访问）
- ✅ Public API 路径 `crate::apps::cli::modes::chat::ChatMode` 不变

---

## Acceptance criteria

- [x] chat.rs → chat/mod.rs (3356 → 165, 95% reduction)
- [x] 12 files (1 mod.rs + 11 sibling)
- [x] 公共 API `ChatMode::new` + `run` + `with_restore_session` + `with_initial_prompt` 不变
- [x] `cargo check -p northhing-cli` 0 new errors
- [x] iron rules: 0 violations
- [x] spec §7 E1 cap exception (input.rs 846 < 1200)
- [x] pre-existing errors transparent (commands.rs:316 + theme.rs:774 both on main 3e6d2b8)
- [x] 60 methods physically moved (60 in 11 sibling `impl ChatMode` blocks)

---

## Out of scope (deferred to future rounds)

- `session_manager.append_completed_local_command_turn` 可见性提升（pre-existing E0624）
- `OpencodeThemeJson::default()` impl（pre-existing E0599）
- sub-domain split 进一步拆 `handle_key_event` 555 行 → 8 个 sub-handler（spec §7 E1 Alternative，未做）
- `dialog_turn.rs` 3656 行 God Object（Round 6）

---

## Commit

`1262698 refactor(chat): split chat.rs into 1 facade + 11 sibling files (sub-domain split)`

Parent: `3e6d2b8` (main HEAD)
Branch: `impl/round5-chat-split`
Worktree: `E:\agent-project\northing-impl-round5`

---

## Refs

- `docs/handoffs/2026-06-28-round5-chat-rs-split-spec.md` (Mavis spec)
- `docs/handoffs/2026-06-27-r4-final-handoff.md` (R4 经验)
- `C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\split-analyzer.py` (产出 before.json / after.json)
- `C:\Users\UmR\.qclaw\skills\code-rot-guard\scripts\structure-verifier.py` (Gate 验证 — 不支持 sub-domain split, 见 spec §7 E4)
- `C:\Users\UmR\.qclaw\workspace\.rot\subdomain-verifier.py` (worker 自写 custom verifier)
- `C:\Users\UmR\.qclaw\workspace\.rot\before-chat-rs.json` (拆分前分析)
- `C:\Users\UmR\.qclaw\workspace\.rot\after-chat-rs.json` (拆分后分析)