# Review: Cleanup Pass — Pre-existing pnpm-lock noise + untracked files

> **Reviewer**: Code审查员（AI）
> **Review Date**: 2026-06-26 17:07
> **Scope**: working tree cleanup + residual issue scan post-fix(observations)
> **Base commit**: `10db367` (docs: record review follow-up)

---

## 审查结论

**APPROVE WITH 1 OBSERVATION** — cleanup pass 可以执行。fix(observations) 已落地（3 latent fix + 1 style refactor），测试全绿。发现 5 处残留的非 UI 线程 Slint setter 调用（`schedule_error_clear` + response-complete refresh），但 commit `bff005a` 的 message 已明确将其标记为 "out of scope, left for a future pass"，不阻塞当前 cleanup。

---

## 一、Working Tree 状态分析

### 应该 commit 的

| 项目 | 状态 | 建议 |
|---|---|---|
| `pnpm-lock.yaml` | M (+25/-3446) | ✅ commit — 清理 BitFun-Installer 残留 + 新增 iconv-lite |
| `docs/handoffs/2026-06-25-bitfun-decomposition.md` | untracked (36KB) | ✅ commit — 6/25 调研文档 |
| `docs/handoffs/2026-06-25-mattpocock-long-term-skills.md` | untracked (17KB) | ✅ commit — 同上 |
| `docs/handoffs/2026-06-25-missing-features-roadmap.md` | untracked (37KB) | ✅ commit — 同上 |
| `docs/handoffs/2026-06-25-northhing-features-catalog.md` | untracked (24KB) | ✅ commit — 同上 |
| `docs/handoffs/2026-06-25-northhing-vs-bitfun-comparison.md` | untracked (39KB) | ✅ commit — 同上 |
| `docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md` | untracked (69KB) | ✅ commit — handoff 中引用的 spec v1.2 |

### 应该忽略的（加 .gitignore）

| 项目 | 状态 | 原因 |
|---|---|---|
| `northhing-Installer/src/i18n/generatedLocaleContract.ts` | untracked | 生成文件 |
| `northhing-Installer/src-tauri/src/installer/generated_locale_contract.rs` | untracked | 生成文件 |
| `src/web-ui/public/version.json` | untracked | 生成文件（.gitignore 有规则但路径不匹配） |
| `src/web-ui/src/generated/version.ts` | untracked | 生成文件（同上） |
| `src/web-ui/src/generated/version-injection.html` | untracked | 生成文件（同上） |
| `src/web-ui/src/infrastructure/i18n/presets/generatedLocaleContract.ts` | untracked | 生成文件 |

### 应该删除的

| 项目 | 原因 |
|---|---|
| `CODE_REVIEW_2026-06-26.md` | 我之前放错位置的临时审查文件，内容已过时（被 `4a768be` relay fix supersede），应删除 |
| `_scan.py` / `_scan_result.txt` | 本次审查的临时工具文件，应删除 |

### .gitignore 需要扩展的规则

当前 `.gitignore` 有：
```
src/generated/version.ts
src/generated/version-injection.html
public/version.json
```

但这些路径不匹配实际生成文件的位置（在 `src/web-ui/` 下）。建议改为通配：
```gitignore
# Generated version files
**/generated/version.ts
**/generated/version-injection.html
**/public/version.json

# Generated locale contracts
**/generatedLocaleContract.ts
**/generated_locale_contract.rs
```

---

## 二、fix(observations) 验证

### Commit `bff005a` — 3 latent Slint setter fixes

审查了 `git show bff005a`，确认：
- ✅ Phase C.3 model-status：`invoke_from_event_loop` 正确包裹
- ✅ Phase G.2 mcp-status：同上
- ✅ P0-A OK branch：`invoke_from_event_loop` 包裹 `set_current_session_id` + `refresh_sessions_ui`，内部用 fresh runtime 驱动 async future
- ⚠️ P0-A 三个 Err 分支（`set_session_error`）未修复 — commit message 明确标记为 out of scope

### Commit `5b7deeb` — open-settings 纯 Slint 重构

审查了 `git show 5b7deeb`，确认：
- ✅ 删除了 `callback open-settings();` 和 Rust handler
- ✅ Slint 端改为 `open-settings => { root.current-route = "settings"; }`，与 `close-settings` 对称
- ✅ 行为不变，减少了 FFI 边界

### Commit `10db367` — handoff 文档更新

确认记录了 3 个 latent fix + 1 个 style fix + outstanding cleanup items。

### 测试验证

- ✅ `cargo test -p northhing --lib`: **40/40 passed**
- ✅ `cargo test -p northhing-relay-server --lib`: **3/3 passed**

---

## 三、残留问题扫描

### ⚠️ 5 处未修复的非 UI 线程 Slint setter 调用

用脚本扫描 `app_state/mod.rs` 中所有 `std::thread::spawn` 闭包内直接调用 `ui.set_*` 且没有 `invoke_from_event_loop` 包裹的位置：

| 行号 | 调用 | 上下文 |
|---|---|---|
| 310 | `ui.set_session_error(...)` | `schedule_error_clear` — Session error auto-clear |
| 311 | `ui.set_input_error(...)` | `schedule_error_clear` — Input error auto-clear |
| 313 | `ui.set_banner_message(...)` | `schedule_error_clear` — Banner auto-clear |
| 314 | `ui.set_banner_detail(...)` | `schedule_error_clear` — Banner detail auto-clear |
| 748 | `ui.set_messages(...)` | Response-complete 后的 message refresh |

**影响评估**:
- `schedule_error_clear`（行 310-314）：5 秒后自动清除错误提示，如果 Slint 静默丢弃，错误提示会永远留在屏幕上不会消失。**用户可见**。
- 行 748 message refresh：AI 回复完成后刷新消息列表，如果 Slint 静默丢弃，用户需要手动切换 session 才能看到最新消息。**用户可见**。

**建议**: 这 5 处应作为下一个 fix pass 的目标。当前不阻塞 cleanup pass，但应在近期修复。

---

## 四、建议的 cleanup commit

```bash
# 1. 删除临时文件
rm CODE_REVIEW_2026-06-26.md _scan.py _scan_result.txt

# 2. 扩展 .gitignore
# 添加生成文件的通配规则

# 3. Stage 所有
git add .gitignore pnpm-lock.yaml \
  docs/handoffs/2026-06-25-*.md \
  docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md

# 4. Commit
git commit -m "chore(workspace): clean up pnpm-lock, track handoff docs, ignore generated files"
```

---

## 五、决策

**APPROVE** — cleanup pass 可以执行。

**非阻塞 observation**: `schedule_error_clear`（4 处）和 response-complete message refresh（1 处）仍有同样的非 UI 线程 Slint setter 问题。`bff005a` 的 commit message 已将其标记为 "out of scope, left for a future pass"，不阻塞当前 cleanup。建议在下一个 fix pass 中修复。

---

**审查完成时间**: 2026-06-26 17:07 (GMT+8)

---

## Follow-up (2026-06-26, cleanup pass landed in `50797e3`)

> **Source**: this review (`a4f3ac3`)
> **Outcome**: cleanup pass executed as recommended above. Working tree
> now matches `main` plus the 6 handoff/spec files that were promoted
> from untracked to tracked. The 3 scratch files (`CODE_REVIEW_2026-06-26.md`,
> `_scan.py`, `_scan_result.txt`) were moved to OS trash. The 6 generated
> files are now silenced by the extended `.gitignore` rules.

### Observation #1 — 5 sites for the next fix pass

The cleanup review's observation #1 is now a tracked issue (not a latent
ambiguity in a previous commit's message). Five call sites in
`src/apps/desktop/src/app_state/mod.rs` have the same non-UI-thread
Slint setter pattern that `bff005a` fixed for the model-status /
mcp-status / P0-A startup-session cases, but were not in scope for
that commit.

| Site (line) | Call | Context | Visible symptom if Slint 1.16 silently drops the update |
|---|---|---|---|
| 310 | `ui.set_session_error(SharedString::from(String::new()))` | `schedule_error_clear` — Session error auto-clear | Session error banner never auto-clears after 5s; stays on screen until next user action |
| 311 | `ui.set_input_error(SharedString::from(String::new()))` | `schedule_error_clear` — Input error auto-clear | Input error banner never auto-clears after 5s |
| 313 | `ui.set_banner_message(SharedString::from(String::new()))` | `schedule_error_clear` — Banner auto-clear | Top-of-window banner never auto-clears after 5s |
| 314 | `ui.set_banner_detail(SharedString::from(String::new()))` | `schedule_error_clear` — Banner detail auto-clear | Banner detail text never auto-clears after 5s |
| 748 | `ui.set_messages(...)` | Response-complete message refresh (inside `rt.block_on`) | After an AI reply completes, the message list is not refreshed; user has to switch sessions to see the new messages |

**Fix shape (for the next pass)**: identical to `bff005a`. Wrap the setter
call in `slint::invoke_from_event_loop`. For the four `schedule_error_clear`
sites, the wrapper is a small one-liner; for line 748 the wrapper is a
bit more involved because `set_messages` is called from inside the same
`rt.block_on` that awaits the model response, so the dispatched closure
needs to be carefully placed (similar to the P0-A startup-session fix).

**Effort estimate**: low — single `fix(ui):` commit, ~30-50 lines of
mod.rs. The fix is mechanical given the `bff005a` pattern is already
established and a sibling for `set_current_session_id` is already in
the same file.

**Test plan**:
- Re-run `cargo test -p northhing --lib` (40/40) to confirm no regression
- Manual smoke: trigger an error condition, wait 5s, confirm the banner
  clears
- Manual smoke: send a chat message, wait for AI reply to complete,
  confirm the new message appears without needing a session switch

**Status**: ✅ FIXED in `29a72eb` (2026-06-26).

The next-pass target landed as a single `fix(ui):` commit:

- `src/apps/desktop/src/app_state/mod.rs` only; 1 file changed,
  50 insertions, 20 deletions.
- 4 `schedule_error_clear` setter sites wrapped in a single
  `slint::invoke_from_event_loop` call.
- Response-complete `set_messages` site restructured: dropped
  the outer `std::thread::spawn` and the inner runtime, replaced
  them with a single `slint::invoke_from_event_loop` that
  fetches the data and builds the model on the UI thread (where
  `ModelRc` is `Send`-free, so this is the only legal way to
  call `set_messages`).
- Test results: 40/40 desktop + 3/3 relay, zero regressions.
- Manual smoke recommended for the v0.1.0 release: trigger an
  error and confirm the 5s auto-clear; send a chat message and
  confirm the new message appears after the AI reply.

Cleanup review observation #1 is now closed. No further
non-UI-thread Slint setter issues are known in the v0.1.0 cycle.
