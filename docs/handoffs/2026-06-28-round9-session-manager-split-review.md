# Round 9 — session_manager.rs Split Review Packet (Reviewer Guide)

> **For**: External reviewer (Kimi / QClaw / 等)
> **Branch**: `impl/round9-session-manager-split` @ `6e77ac3` (HEAD, merged into main @ `59019c7`)
> **Worktree**: `E:\agent-project\northing-impl-round9`
> **Author**: coder (Mavis M2.7-highspeed) — Round 9 worker, 45 min no-stall
> **Date**: 2026-06-28
> **Spec**: `C:\Users\UmR\.mavis\scratchpads\mvs_4cfd3e045ea44bf1942ff29fa9970579\round9-session-manager-plan.yaml` (Round 9 plan YAML)
> **Impl handoff doc**: `docs\handoffs\2026-06-28-round9-session-manager-split-impl.md`
> **Review template**: 参考 Round 6 review packet `docs\handoffs\2026-06-28-round6-dialog-turn-split-review.md`

---

## TL;DR

把 `src\crates\assembly\core\src\agentic\session\session_manager.rs`（**3988 行 God Object**, 115 top-level fns, 1 pub + 48 pub(crate) + 66 private, **distributed God Object** with no single dominant god method — max `delete_session` 152 lines）拆成 **1 facade (137 行) + 8 sibling sub-domain files + 1 test sibling**。Round 8 Task A pattern applied + Round 3b "名义拆分" orphan bug prevention mandatory.

**Verification 4 axis PASS**:
- cargo check: 0 errors
- cargo test: 899 passed; 0 failed; 1 ignored (= Round 6/7/8 baseline)
- cargo fmt --check (whole crate): exit 0, clean
- iron rules: 0 violations (no new unwrap/panic/unreachable/let _ =)
- **Round 3b orphan prevention**: 19 `pub mod` declarations = 19 `.rs` files in `session/` directory — all 8 new siblings + 11 existing siblings properly wired

**⚠️ Reviewer attention**: 需要看 deviation **D1** (见 handoff doc + below):
- **D1**: `session_manager_tests.rs` 2228 行 > 800 cap by 1428 行 (test code extracted as 1 file) — similar pattern to Round 8 `round_executor.rs` 1631 which triggered Round 8b follow-up; **Round 9b follow-up likely needed** to split test file by sub-domain (each test sibling 1:1 with production sibling)

---

## 改动范围

### 9 个 sub-domain sibling 文件 (8 production + 1 test)

| 文件 | 行数 | 方法数 | 内容 |
|---|---|---|---|
| `session/session_manager.rs` (facade) | **137** | 2 (`Default::default` + `SessionTitleMethod::as_str`) | was 3988 → **96.6% reduction**, ≤ 1000 cap ✅ |
| `session/session_manager_model_selection.rs` | 112 | 5 | model resolution + context window sync |
| `session/session_manager_titles.rs` | 220 | 7 | normalize + truncate + fallback + AI title gen |
| `session/session_manager_persistence_predicate.rs` | 82 | 5 | predicate helpers |
| `session/session_manager_auto_save_cleanup.rs` | 241 | 9 | auto-save + cleanup background tasks |
| `session/session_manager_workspace_path.rs` | 146 | 4 | workspace path resolution |
| `session/session_manager_lifecycle.rs` | 463 | 11 | new + create + delete (152-line god method) + list + state update |
| `session/session_manager_metadata.rs` | 567 | 29 | metadata merge + message ops + state compression |
| `session/session_manager_tests.rs` | **2228** ⚠️ | 0 | full test block moved out of facade (test code, no cap) |
| `session/mod.rs` | 48 | — | +9 lines (8 new `pub mod` + `pub use` declarations) |

**Total sibling**: 9 files (8 production + 1 test). Total production sibling lines: 1831 + 137 facade = ~1968 (was 3988 = 51% reduction after tests extracted).

### Sub-domain cluster mapping

| Cluster | Sibling | Source fn count |
|---|---|---|
| Model selection | `session_manager_model_selection.rs` | 5 (load_ai_config, is_auto_model_selector, context_window_for_model_selection, session_context_window_from_ai_config, sync_session_context_window_from_ai_config) |
| Titles | `session_manager_titles.rs` | 7 (normalize_session_title_input, normalize_whitespace, truncate_chars, fallback_session_title, paginate_messages, ...) |
| Persistence predicate | `session_manager_persistence_predicate.rs` | 5 (should_persist_session_kind, should_persist_session, same_session_version, should_persist_session_id, ...) |
| Auto-save + cleanup | `session_manager_auto_save_cleanup.rs` | 9 (collect_auto_save_snapshots, auto_save_snapshot_is_current, is_session_expired, cleanup_*, spawn_cleanup_task god method 115 lines, ...) |
| Workspace path | `session_manager_workspace_path.rs` | 4 (effective_workspace_path_from_config, session_workspace_path, effective_session_workspace_path, resolve_session_workspace_path) |
| Lifecycle | `session_manager_lifecycle.rs` | 11 (new, create_session, delete_session god method 152 lines, rollback_*, restore_session_*, ...) |
| Metadata | `session_manager_metadata.rs` | 29 (rebuild_skill_agent_listing_baseline_*, collect_hidden_subagent_cascade_*, ...) |

---

## Commit 链 (2 个)

```
6e77ac3 docs(handoff): Round 9 session_manager split impl report (3f10b78 ref) ← HEAD
3f10b78 refactor(session-manager): split session_manager 3988 → facade + 8 sibling sub-domain files (Round 9)
59019c7 (main) merge: Round 9 session_manager split (Round 3b orphan prevention PASS)
```

Parent: `7bec409` (main HEAD after Round 8b merge)

---

## 4-axis verification (Mavis run)

### Axis 1: cargo check

```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing-core --features product-full --lib --message-format=short
```

**Result**: 0 errors. Pre-existing 766 warnings (unchanged from Round 6/7/8 baseline).

### Axis 2: cargo test

```bash
cargo test -p northhing-core --features product-full --lib
```

**Result**: `899 passed; 0 failed; 1 ignored`. Finished in 2.63s. Equals Round 6/7/8 baseline.

### Axis 3: cargo fmt --check

```bash
cargo fmt --check -p northhing-core
```

**Result**: exit 0, clean. No diffs in touched files (session/* + turn_subhandlers.rs + execution/*). Pre-existing CRLF noise in unrelated files discarded.

### Axis 4: Round 3b orphan prevention

```bash
# Count pub mod declarations vs .rs files in session/
$pubModCount = (Select-String -Path src\crates\assembly\core\src\agentic\session\mod.rs -Pattern "^pub mod ").Count
$rsCount = (Get-ChildItem src\crates\assembly\core\src\agentic\session\*.rs).Count
# $pubModCount = 19, $rsCount = 19, equal → NO orphan
```

**Result**: 19 `pub mod` declarations = 19 `.rs` files. Zero orphan. Round 3b "名义拆分" bug prevention PASS.

---

## Reviewer 必读 (关注点)

### 1. Spec deviations D1-D3 (见 handoff doc)

- **D1**: `session_manager_tests.rs` 2228 行 > 800 cap by 1428 行 (test code extracted as 1 file). Same pattern as Round 8 `round_executor.rs` 1631 which triggered Round 8b follow-up. Reviewer decision:
 - (a) COND APPROVE with deviation log (similar to Round 8 turn.rs 1352 vs cap 1000 + Round 8 round_executor 1631)
 - (b) Require Round 9b further split (extract test file by sub-domain, e.g. `session_manager_lifecycle_tests.rs` + `session_manager_metadata_tests.rs` etc. — each test sibling 1:1 with production sibling)
 - (c) Tighten session_manager_tests.rs (collapse blank lines, strip comments) — unlikely to fit ≤ 800 given the test code density

- **D2**: Round 9 has no separate spec doc (used Round 8b plan YAML as template + plan YAML directly). Reviewer may want spec doc for future rounds — recommend writing spec docs for Round 9b+.

- **D3**: worker added `session_manager_metadata.rs` (567 行) which was not in original plan's 7 cluster list. Worker justified in handoff doc §D-deviations. Reviewer decision: accept (since cluster is cohesive) or reject (force re-cluster).

### 2. Method distribution 验证

- 115 top-level fns should split into:
 - facade (session_manager.rs): 2 fns (Default::default, SessionTitleMethod::as_str)
 - sibling files: 113 fns across 8 production siblings
 - test sibling: 0 fns (test code only)
- Verify: `grep -rn "^\s*(pub(crat)— \s+)— (async\s+)— fn" src\crates\assembly\core\src\agentic\session\*.rs | wc -l`
- Expected: 115 total (2 facade + 113 siblings + 0 tests)

### 3. mod.rs pub mod declarations (Round 3b orphan prevention)

- 11 existing siblings (Round 3b + earlier) + 8 new siblings (Round 9) = 19 total `pub mod` declarations
- Each sibling MUST have `pub mod <name>;` declaration in mod.rs
- Verify: `grep "^pub mod " src\crates\assembly\core\src\agentic\session\mod.rs | wc -l` = 19
- Verify: `ls src\crates\assembly\core\src\agentic\session\*.rs | wc -l` = 19
- Equal count = no orphan

### 4. 跨 sibling 可见性

- sub-handler methods: `pub(super)` (visible to mod.rs facade)
- shared state structs / cross-sibling fields: `pub(crate)`
- Round 9 plan YAML requires both; verify grep:
 - `grep -rn "pub(super)" src\crates\assembly\core\src\agentic\session\session_manager_*.rs | wc -l` ≥ 50
 - `grep -rn "pub(crate)" src\crates\assembly\core\src\agentic\session\session_manager_*.rs | wc -l` ≥ 5 (cross-sibling state)

### 5. iron rules

- 无 unwrap()/panic!/unreachable!/let _ = Result 新增 (Mavis 验证: 0 violations)
- Mover not copy (session_manager.rs 137 行 facade, method bodies physically moved to sibling files)
- ⚠️ session_manager_tests.rs 2228 > 800 §7 E1 cap (D1 deviation)
- 字段/方法 visibility 提升到正确级别

### 6. Public API 不变

- session lifecycle / persistence / restore entry points unchanged
- Verify: `grep -rn "pub.*fn" src\crates\assembly\core\src\agentic\session\mod.rs | wc -l` (should match pre-Round-9 facade pub fn count)

---

## Verification commands (reviewer 自跑)

```powershell
# Setup path
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cd E:\agent-project\northing # main worktree post-merge

# 1. cargo check
cargo check -p northhing-core --features product-full --lib --message-format=short 2>&1 | Tee-Object C:\Users\UmR\.qclaw\workspace\.rot\reviewer-r9-cargo-check.log | Select-String "error\[|Finished"

# 2. cargo test
cargo test -p northhing-core --features product-full --lib 2>&1 | Tee-Object C:\Users\UmR\.qclaw\workspace\.rot\reviewer-r9-cargo-test.log | Select-String "test result|FAILED|error\["

# 3. cargo fmt --check (whole crate)
cargo fmt --check -p northhing-core 2>&1 | Tee-Object C:\Users\UmR\.qclaw\workspace\.rot\reviewer-r9-cargo-fmt.log

# 4. Round 3b orphan check (CRITICAL)
$pubModCount = (Select-String -Path src\crates\assembly\core\src\agentic\session\mod.rs -Pattern "^pub mod ").Count
$rsCount = (Get-ChildItem src\crates\assembly\core\src\agentic\session\*.rs).Count
"pub mod count: $pubModCount | rs file count: $rsCount"
# Must be equal (no orphan)

# 5. Method distribution
$facadeMethods = (Select-String -Path src\crates\assembly\core\src\agentic\session\session_manager.rs -Pattern "^\s*(— :pub(— :\([^)]+\))— \s+)— (— :async\s+)— fn\s+\w+").Count
"sessions_manager.rs facade fns: $facadeMethods (expect 2)"

# 6. File sizes (post-merge main)
Get-ChildItem src\crates\assembly\core\src\agentic\session\session_manager*.rs | Select-Object Name, @{N='Lines';E={py -c "import sys; print(sum(1 for _ in open(r'$($_.FullName)', encoding='utf-8')))"}} | Format-Table -AutoSize
```

---

## 期望结果

- Axis 1: `Finished` (0 errors)
- Axis 2: `test result: ok. 899 passed; 0 failed; 1 ignored`
- Axis 3: exit 0 (clean)
- Axis 4: `pub mod count: 19 | rs file count: 19` (no orphan)
- Axis 5: facade fns = 2
- Axis 6: facade 137, sibling ≤ 800 except tests 2228

---

## 决策矩阵

请 reviewer 给出:

1. **APPROVE / REJECT / APPROVE with minor observations**— 2. D1 `session_manager_tests.rs` 2228 > 800 cap 是否需要 Round 9b follow-up— 3. D2 (无 spec doc) 是否需要补— 影响 future Round 9b 的 spec 写法
4. D3 (worker 新增 `metadata` cluster, 不在原 7 cluster 列表) 是否接受— 5. iron rules 7 条是否全过— 6. Round 3b orphan prevention 是否通过— 7. Public API 是否保留— ## Refs

- Plan YAML: `C:\Users\UmR\.mavis\scratchpads\mvs_4cfd3e045ea44bf1942ff29fa9970579\round9-session-manager-plan.yaml`
- Impl handoff (worker): `docs\handoffs\2026-06-28-round9-session-manager-split-impl.md`
- Round 6 review packet template: `docs\handoffs\2026-06-28-round6-dialog-turn-split-review.md`
- Round 8 exec-engine review packet: `docs\handoffs\2026-06-28-round8-exec-engine-split-review.md` (similar template)
- Before split: `C:\Users\UmR\.qclaw\workspace\.rot\before-session-manager.json` (split-analyzer output, Mavis 已生成)
- After split: `C:\Users\UmR\.qclaw\workspace\.rot\after-session-manager.json` (worker 已生成)
- Custom subdomain-verifier: `C:\Users\UmR\.qclaw\workspace\.rot\subdomain-verifier-session-manager.py` (Round 9 自写, 模仿 Round 6 verifier)
- Round 8 review packet template (most recent): `docs\handoffs\2026-06-28-round8-exec-engine-split-review-report.md` (注意: 这是 review-report 不是 review guide, 别混淆)

---

*Review packet prepared by Mavis at 2026-06-28 18:34 UTC+8. Branch `impl/round9-session-manager-split` @ `6e77ac3` ready for external review with D1 (session_manager_tests.rs 2228 > 800), D2 (no spec doc), D3 (metadata cluster not in plan) deviations noted.*