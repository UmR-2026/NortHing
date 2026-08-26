# Review — Task P22 (Room Workspace Session Resolution & Double-Create TOCTOU)

- **Diff**: 8622667..1f3a15a (1 commit: `1f3a15a fix(consult-room): P22 …`)
- **Files touched**: `src/apps/desktop/src/ui_dioxus/api.rs` (+157/-10, 195→329 lines), `src/apps/desktop/src/ui_dioxus/i18n.rs` (-1)
- **Verdict basis**: brief `task-p22-room-workspace-brief.md` + implementer report + diff; auxiliary verification by `git diff -- scripts/rot-budget.json` (empty), full-repo grep for `ONBOARDING_BTN_COMPLETE` and `list_sessions_all_workspaces`.

---

## Spec Compliance

- ✅ **Spec compliant**

Judgment-table match for `pick_room_session` (`api.rs:77-94`):

| Brief case | Implementation branch | Verdict |
|---|---|---|
| `preferred=Some(ws)` + matching group with sessions | `find` returns group → `sessions.first()` | ✅ |
| `preferred=Some(ws)` + no matching group OR matching group has empty `sessions` | `find` returns `None` *or* `and_then(\|g\| g.sessions.first())` returns `None` | ✅ |
| `preferred=None` + first non-empty group exists | `find(\|g\| !g.sessions.is_empty())` then `sessions.first()` | ✅ |
| All empty / no groups at all | both `find` and `first()` resolve to `None` | ✅ |

`ensure_room_session` five-step rewrite (`api.rs:96-137`) matches brief step-by-step:

1. Lock-then-cache-check early return (lines 96-100). ✅
2. Preferred workspace resolution with `current_workspace → workspaces.first()` fallback and `Err(e) → tracing::warn! + None` (lines 105-114). ✅
3. Thin wrapper over `list_sessions_all_workspaces` (line 117, thin wrapper itself at lines 58-61). ✅
4. Cache hit on `pick_room_session` returns session id (lines 118-119). ✅
5. Miss → `create_session` with `preferred_workspace` and the documented `agent_type="agentic"` / `model_name="default"` / `name="诊室"` (lines 120-128). ✅

- **Err not cached**: ✅ Both `list_sessions_all_workspaces().await?` (line 117) and `kernel_facade().create_session(config).await?` (line 127) propagate via `?` **before** `*guard = Some(session_id.clone())` on line 130. Confirmed.
- **Ponytail ceiling comment**: ✅ `api.rs:96` (`ponytail: process-lifetime room session cache, restart required to switch room after session deletion/archival; upgrade path = invalidate on delete_session event`). Matches brief verbatim intent.
- **Existing uninitialized tests still pass**: ✅ `test_ensure_room_session_fails_cleanly_when_uninitialized` assertion unchanged (`assert!(res.is_err())`). `test_api_functions_fail_cleanly_before_init` augmented with `let _ = list_sessions_all_workspaces().await;` (line 225) — still wraps each call in `let _ =`, so any `Err` is silently discarded. Both pass in MSVC test output (lines 82-83 of report).
- **Four new unit tests assert real behavior**: ✅ All four present (`api.rs:268-365`):
  - `test_pick_room_session_preferred_hit` — constructs `/ws/a` (s1) + `/ws/b` (s2), picks `s2` for `preferred=Some("/ws/b")`. Real assertion (`Some("s2")`).
  - `test_pick_room_session_preferred_miss_returns_none` — two sub-assertions: non-existent preferred ws → `None`; existing preferred ws but empty sessions → `None`. Real behavior.
  - `test_pick_room_session_no_preferred_picks_first_non_empty` — empty-then-non-empty groups, picks second. Real assertion.
  - `test_pick_room_session_empty_groups_returns_none` — empty `Vec` and single-empty-group cases. Real assertion.
- **i18n const deleted, no residual Rust references**: ✅ Verified by full-repo grep for `ONBOARDING_BTN_COMPLETE` — all 7 hits are in `.superpowers/sdd/*`, `docs/handoffs/*`, `docs/status/tech-debt-ledger.md` (markdown only). Zero Rust references remain. Compiler warning count `36 → 35` (lines 38/45 of report) corroborates the deletion. No contract/audit failure reported.

---

## Strengths

- **Minimum-scope surgery**: `+157/-10` for a fix that reworks selection, serializes TOCTOU, and removes an orphan. No new abstraction layer, no trait, no config struct. Cache is one `static Mutex<Option<String>>`. Selector is one pure function.
- **Pure-function testability**: `pick_room_session` is `&[WorkspaceSessionsDto] → Option<&SessionSummaryDto>` with no hidden state — four unit tests cover every branch of the judgment table without fixtures beyond two hand-built DTOs.
- **Brief-mandated ceiling annotation present and specific**: line 96 names both the failure mode (cache survives delete/archive until restart) and the upgrade path (invalidate on `delete_session` event). Defensible deferral.
- **Reuse investigation matches repo reality**: full-repo grep for `list_sessions_all_workspaces` confirms exactly the three expected non-doc call sites pre-task (`kernel-api/src/session.rs:249` trait, `core/src/kernel_facade/session.rs:71` impl, plus the new `api.rs` lines 59-60/115/225). Zero pre-existing picker existed to reuse.
- **Existing uninitialized tests left structurally intact**: only the additive `let _ = list_sessions_all_workspaces().await;` line in the smoke test. No assertions weakened, no skip introduced.

---

## Issues

#### Critical
*(none)*

#### Important
*(none)*

#### Minor

1. **Report does not document the CWD-group-last behavior change** (`reports/task-p22-room-workspace-report.md`, "偏离及理由" section). The brief's QUALITY checklist explicitly asks: *"CWD 组在 facade 里排末尾——`无 preferred 取第一非空组`时取到的是最近访问的工作区组而非 CWD，确认这与旧行为差异已被 report 说明"*. The report declares "无偏离" without acknowledging that zero-config users (settings load `Err` → `preferred=None`) with both a persisted workspace session and a CWD session will now resolve the most-recent-access session rather than the prior CWD-first behavior. The behavior matches the judgment table and is intentional per brief, but the implementer should have surfaced the migration story (upgrading user with prior CWD sessions gets a new session in their persisted workspace instead of resuming the CWD one — exactly the new-room-for-preferred-miss design choice).
2. **Held-across-await mutex not surfaced in report's concurrency evaluation**. The brief asks: *"持锁跨 await 的范围是否恰当（解析全程持锁 vs 双检）？持锁做网络调用（list/create）是否会卡住并发 send_action 的首个发送路径？评估实际后果并判定可接受性"*. The implementation holds `ROOM_SESSION_CACHE` from line 97 through line 131 — across `load_app_settings().await`, `list_sessions_all_workspaces().await`, and `create_session().await`. The brief itself approves this single-flight pattern, so the design is acceptable; the gap is that the implementer never documented the *acceptability reasoning* (consult-room's call site is one-shot: UI calls `ensure_room_session`, gets an id, then submits turns without re-locking). `send_action` never re-enters this mutex. The ponytail comment names the ceiling but does not justify the held-across-IO choice.
3. **`to_string_lossy()` on `PathBuf`** (lines 109-110). Same lossy conversion pattern exists in the codebase and is consistent with the brief's specified pseudo-code, so this is just an observation: any non-UTF-8 path in settings will produce U+FFFD replacement chars and never match the (assumed UTF-8) strings the facade emits. Not in scope to fix; flagged for awareness.

### Cannot verify from diff

- **Runtime ordering claim** (`kernel_facade/session.rs:71-106`) that groups are sorted most-recent-access first with CWD group as last-resort fallback. Brief asserts this; diff only consumes `list_sessions_all_workspaces()`. If facade ordering ever drifts from the assumption embedded in `pick_room_session` (preferred miss → first non-empty group is "best pick"), the pure function silently degrades to "first non-empty", which is still a defined behavior — so blast radius is contained, but worth a one-line cross-check at facade level if this gate ever regresses.
- **`load_app_settings` behavior in uninitialized test environment**. The new `ensure_room_session` will, on the `test_ensure_room_session_fails_cleanly_when_uninitialized` path, call `load_app_settings().await` before the existing `list_sessions_all_workspaces().await?` can return `Err`. The test passes per report (line 82), so either `load_app_settings` resolves to a real (possibly empty) `Ok` and the subsequent `list_sessions_all_workspaces().await?` returns `Err`, **or** `load_app_settings` itself returns `Err` (warned, `preferred=None`), and `list_sessions_all_workspaces().await?` returns `Err`. Either path satisfies `assert!(res.is_err())`. Cannot tell from diff which branch executes; both are spec-compliant.
- **Whether concurrent `send_action` calls during the first `ensure_room_session` are blocked**. The mutex is held across the entire I/O sequence, but `send_action` is documented in brief as not touching this mutex. Tracing the actual UI call graph at runtime is out of scope for diff review; the static evidence is that `send_action` does not appear in the diff's modified surface, and `submit_turn` (`api.rs:48-50`) does not acquire `ROOM_SESSION_CACHE`.

---

## Assessment

**Task quality:** Approved
**Reasoning:** Implementation matches the brief's judgment table, five-step rewrite, ceiling annotation, and test surface exactly. The two minor omissions are documentary gaps (CWD-group behavior change, held-across-await justification) — both concern the report rather than the code; the code itself implements the spec faithfully with no owner abstraction, no rot-budget touch, and a healthy file size (329 lines, far below the 800-line god-file ceiling).
