# Round 6 — dialog_turn.rs Split Review Packet

> **For**: External reviewer (Kimi / QClaw / 等)
> **Branch**: `impl/round6-dialog-turn-split` @ `aeef006` (HEAD)
> **Worktree**: `E:\agent-project\northing-impl-round6`
> **Author**: Mavis (M3 orchestrator) — coder sub-agent 1st attempt + Mavis take-over
> **Date**: 2026-06-28
> **Spec**: `docs/handoffs/2026-06-28-round6-dialog-turn-split-spec.md`
> **Handoff doc**: `docs/handoffs/2026-06-28-round6-dialog-turn-split-impl.md`
> **Review template**: 参考 Round 5 review report `docs/handoffs/2026-06-28-round5-chat-rs-review-report.md`

---

## TL;DR

把 `src/crates/assembly\core\src\agentic\coordination\dialog_turn.rs`（**3397 行 god object**, 86 方法, **54 public API**）拆成 **1 facade (`dialog_turn/mod.rs` 1652 行) + 6 sibling sub-domain 文件** (workspace/session/turn/compaction/restore/thread_goal)。`restore.rs` 是空 stub（per spec §7 E2 facade-only design, 12 个 `restore_*` 都是 `pub` 留 facade）。

**Verification 4 axis PASS**:
- cargo check: 0 errors
- cargo test: 899 pass, 0 fail
- custom subdomain-verifier: PASS (86/86 methods preserved, 54/54 pub API preserved)
- cargo fmt: clean on touched files

**⚠️ Reviewer attention**: 需要看 deviations D1-D8 (见 handoff doc § "Spec deviations")。最关键是:
- **D4**: turn.rs **1352 行超 §7 E1 cap 1000** (start_dialog_turn_internal 单方法 701 行)
- **D2**: spec 估算 24 public API 方法，实际 54 (2.25× 偏差)
- **D8**: Mavis take-over narrative — 4 类 systemic errors worker 漏报，cargo check "0 NEW errors" 是误报

---

## 改动范围

### 6 个 sub-domain sibling 文件

| 文件 | 行数 | 方法数 | 内容 |
|---|---|---|---|
| `dialog_turn/mod.rs` | 1652 | 55 | facade + 54 public API + impl ConversationCoordinator |
| `dialog_turn/workspace.rs` | 398 | 6 | workspace binding helpers |
| `dialog_turn/session.rs` | 253 | 4 | session CRUD helpers |
| `dialog_turn/turn.rs` | **1352** ⚠️ | 13 | start_dialog_turn_internal (701) + 12 helpers |
| `dialog_turn/compaction.rs` | 255 | 4 | compaction helpers |
| `dialog_turn/thread_goal.rs` | 211 | 4 | thread goal helpers |
| `dialog_turn/restore.rs` | 2 | 0 | empty stub (12 restore_* methods are pub, stay in facade) |
| **Total** | **4123** | **86** | (含 1652 行 facade) |

### Mavis take-over 修改

- `src/crates/assembly/core/src/agentic/coordination/coordinator.rs` — `WrappedUserInputPayload` 4 fields `pub(crate)` 提升
- `src/crates/services/services-integrations/src/mcp/protocol/transport_remote.rs` — 2 行 `info` → `&info` (rmcp 1.8.0 兼容)

---

## Commit 链 (7 个)

```
aeef006 docs(handoff): Round 6 dialog_turn.rs split final handoff (Mavis take-over) ← HEAD
e6397de fix(dialog_turn): Mavis take-over — split visibility + imports + struct fields + rmcp 1.8.0 compat
4ed95c9 docs(handoff): Round 6 dialog_turn.rs split impl handoff (review request)
ff87df9 chore(dialog_turn): cargo fmt fixups + remove orphan doc comments (Step 10)
ea83dd3 refactor(dialog_turn): sub-domain split — 31 helpers extracted to 5 siblings (Steps 3-9)
65e43dc refactor(dialog_turn): replace dialog_turn.rs with dialog_turn/mod.rs facade (Step 2)
14ee45f refactor(dialog_turn): create dialog_turn/ dir + 7 empty sibling skeleton (Step 1)
```

Parent: `2398ad8` (main HEAD after Round 5 merge)

---

## 4-axis verification (Mavis run)

### Axis 1: cargo check

```bash
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cargo check -p northhing-core --features product-full --lib --message-format=short
```
**Result**: 0 errors. Finished in 2m 05s.

### Axis 2: cargo test

```bash
cargo test -p northhing-core --features product-full --lib
```
**Result**: `899 passed; 0 failed; 1 ignored`. Finished in 2.15s.

### Axis 3: custom subdomain-verifier

```bash
py C:\Users\UmR\.qclaw\workspace\.rot\subdomain-verifier-dialog-turn.py
```
**Result**: PASS. (86/86 methods preserved, 54/54 pub API preserved, risk high→low)

### Axis 4: cargo fmt

```bash
cargo fmt --check -p northhing-core
```
**Result**: 0 diffs on touched files (dialog_turn/, coordinator.rs, transport_remote.rs). Pre-existing fmt noise in `apps/cli/src/modes/chat/*.rs` and `service/review_platform/providers/gitlab.rs` discarded — separate cleanup by review→fix cycle.

---

## Reviewer 必读 (关注点)

### 1. Spec deviations D1-D8 (见 handoff doc)

最关键:
- **D2**: spec 估算 24 public API 方法，实际 54 → facade 1652 行 (vs spec ~500 行). 可接受吗？
- **D4**: turn.rs 1352 行超 §7 E1 cap 1000 by 352 行 (35% over). 可接受吗？还是必须先拆 `start_dialog_turn_internal` 701 行？
- **D5**: spec 说"1 + 7 sibling = 8 files", 实际 1 + 6 sibling = 7 files (spec §2.1 列了 6 sibling 名). 是 spec 错还是 impl 错？
- **D8**: Mavis take-over — worker 的 cargo check "0 NEW errors" 是 cargo check stop at first error 的误报. Mavis 修复的 4 类 errors 是真问题吗？

### 2. Method distribution 验证

- 86 方法应该全在 `mod.rs` (55 facade) + 5 sibling (4+4+13+4+4=27 helpers) + 1 empty stub. 86 = 55 + 27 ✅
- 54 public API 应该全在 `mod.rs` (spec §7 E2 facade-only design). 验证方法: `grep -rn "pub.* fn" dialog_turn/ --include='*.rs' | grep -v 'mod.rs'`

### 3. Imports 检查

- `dialog_turn/mod.rs`: `use super::{scheduler::..., turn_outcome::..., coordinator::*, ports::{...}}` — siblings within coordination/
- `dialog_turn/*.rs` (siblings): `use super::super::{coordinator::*, ports::*, scheduler::*, turn_outcome::TurnOutcome}` — parent of dialog_turn (coordination module)
- 不能搞混: mod.rs 用 `super` (siblings), sibling files 用 `super::super` (parent)

### 4. 跨模块可见性

- `WrappedUserInputPayload` struct fields 是 `pub(crate)` (Mavis take-over)
- 16 个 sibling file methods 是 `pub(super)` (Mavis take-over via promote-sibling-visibility.py)
- struct `pub(crate)` + fields default private = E0616; Mavis 修了 (per spec §3 没列这个 struct)

### 5. iron rules

- 无 unwrap()/panic!/unreachable!/let _ = Result 新增
- Mover not copy (dialog_turn.rs 删除，31 方法物理移动)
- ⚠️ turn.rs 1352 > 1000 §7 E1 cap
- 字段/方法 visibility 提升到正确级别 (sibling methods pub(super), struct fields pub(crate))

### 6. 计划外修复: rmcp 1.8.0 兼容

- `transport_remote.rs:515,549`: `info` → `&info` (2 行)
- Worker attribute 到 "pre-existing" 误判 (实际是 Cargo.lock drift rmcp 1.7.0→1.8.0)
- 修复是 forward-compat (deref coercion 对两个版本都工作)
- 接受吗— 或要求 revert + pin rmcp = "=1.7.0"— ## Verification commands (reviewer 自跑)

```powershell
# Setup path
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path
cd E:\agent-project\northing-impl-round6

# 1. cargo check
cargo check -p northhing-core --features product-full --lib --message-format=short 2>&1 | Tee-Object -FilePath C:\Users\UmR\.qclaw\workspace\.rot\reviewer-cargo-check.log | Select-String "error\[|Finished"

# 2. cargo test
cargo test -p northhing-core --features product-full --lib --message-format=short 2>&1 | Tee-Object -FilePath C:\Users\UmR\.qclaw\workspace\.rot\reviewer-cargo-test.log | Select-String "test result|FAILED|error\["

# 3. cargo fmt --check on touched files
cargo fmt --check -p northhing-core 2>&1 | Tee-Object -FilePath C:\Users\UmR\.qclaw\workspace\.rot\reviewer-cargo-fmt.log | Select-String "dialog_turn|coordinator|transport_remote"

# 4. custom subdomain-verifier
py C:\Users\UmR\.qclaw\workspace\.rot\subdomain-verifier-dialog-turn.py 2>&1 | Tee-Object -FilePath C:\Users\UmR\.qclaw\workspace\.rot\reviewer-subdomain-verifier.log

# 5. public API preservation check
$mod_methods = (Select-String -Path src\crates\assembly\core\src\agentic\coordination\dialog_turn\mod.rs -Pattern "^ pub.*fn ").Count
$sibling_methods = (Select-String -Path src\crates\assembly\core\src\agentic\coordination\dialog_turn\*.rs -Pattern "^ pub.*fn ").Count
"mod.rs pub methods: $mod_methods"
"sibling pub methods: $sibling_methods (expected: 0, all pub in facade)"

# 6. file sizes
Get-ChildItem src\crates\assembly\core\src\agentic\coordination\dialog_turn\*.rs | Select-Object Name, @{N='Lines';E={[System.IO.File]::ReadAllLines($_.FullName).Count}} | Format-Table -AutoSize
```

---

## 期望结果

- Axis 1: `Finished` (0 errors)
- Axis 2: `test result: ok. 899 passed; 0 failed; 1 ignored`
- Axis 3: `OVERALL: PASS (sub-domain split)`
- Axis 4: 无 dialog_turn/coordinator/transport_remote 行输出
- Axis 5: mod.rs ~54 pub methods, sibling 0 pub methods
- Axis 6: mod.rs ≤ 1652, turn.rs ≤ 1352, 其他 sibling ≤ 800

---

## 决策矩阵

请 reviewer 给出:

1. **APPROVE / REJECT / APPROVE with minor observations**— 2. 每个 deviation D1-D8 是否可接受— 3. turn.rs 1352 行超 cap 是否需要进一步拆— 4. transport_remote.rs 2 行修复是否接受— 或要求 rmcp pin— 5. iron rules 7 条是否全过— ## Refs

- Spec: `E:\agent-project\northing-impl-round6\docs\handoffs\2026-06-28-round6-dialog-turn-split-spec.md`
- Handoff (Mavis take-over): `E:\agent-project\northing-impl-round6\docs\handoffs\2026-06-28-round6-dialog-turn-split-impl.md`
- Round 5 review template: `E:\agent-project\northing-impl-round6\docs\handoffs\2026-06-28-round5-chat-rs-review-report.md`
- Before split: `C:\Users\UmR\.qclaw\workspace\.rot\before-dialog-turn.json`
- After split (Mavis): `C:\Users\UmR\.qclaw\workspace\.rot\after-dialog-turn-mavis.json`
- Custom verifier: `C:\Users\UmR\.qclaw\workspace\.rot\subdomain-verifier-dialog-turn.py`
- Promote visibility script: `C:\Users\UmR\.qclaw\workspace\.rot\promote-sibling-visibility.py`