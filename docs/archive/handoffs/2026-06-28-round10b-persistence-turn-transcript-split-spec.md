# Round 10b Spec — persistence turn + transcript 二次拆

> **目标**: 把 `turn_subhandlers.rs` 1195 + `transcript_subhandlers.rs` 981 拆成 5 个 sibling 文件 ≤ 800 each
> **Trigger**: QClaw R10a COND APPROVE 7.5/10 + R10b REQUIRED (D1 49% over cap, D2 23% over cap)
> **Spec source**: QClaw review report §8 R10b Specification (Draft)

---

## §1 当前状态

| File | Lines | Cap | Over |
|---|---|---|---|
| turn_subhandlers.rs | 1195 | 800 | +395 ❌ D1 |
| transcript_subhandlers.rs | 981 | 800 | +181 ❌ D2 |
| session_subhandlers.rs | 437 | 800 | OK ✅ |
| metadata_subhandlers.rs | 481 | 800 | OK ✅ |
| skill_snapshot_subhandlers.rs | 547 | 800 | OK ✅ |
| paths_utilities.rs | 412 | 800 | OK ✅ |
| manager.rs (facade) | 70 | 200 | OK ✅ |

Test baseline: 899/0/1 (R10a 已 maintain)

## §2 拆分方案 (QClaw Option A: sub-domain split)

### turn_subhandlers.rs 1195 → 3 sub-handlers

| New file | Target | Scope |
|---|---|---|
| `turn_io.rs` | ~400 | 单 turn 读写：save_dialog_turn, load_dialog_turn, delete_dialog_turn, delete_dialog_turns_from + helpers |
| `turn_batch.rs` | ~400 | 批量 turn：load_session_with_turns, load_session_with_turns_timed, load_session_tail_turns, load_recent_turns, load_session_turns, list_indexed_turn_paths, read_turn_paths |
| `turn_metadata_sync.rs` | ~300 | 元数据同步：read_metadata_tail_turns, build_session_metadata_for_save 等 |

### transcript_subhandlers.rs 981 → 2 sub-handlers

| New file | Target | Scope |
|---|---|---|
| `transcript_export.rs` | ~400 | export_session_transcript (209 行 god method 拆解) + render/format helpers |
| `transcript_fingerprint.rs` | ~500 | transcript_path / preview / fingerprint / parse 等渲染解析 |

## §3 最终文件结构

```
agentic/persistence/
├── mod.rs                       18 (5→8 pub mod + 3 = 11 lines)
├── manager.rs                   70 (facade, unchanged)
├── session_branch.rs            471 (Round 3b, unchanged)
├── session_subhandlers.rs       437 (R10a, unchanged)
├── turn_io.rs                   NEW ~400
├── turn_batch.rs                NEW ~400
├── turn_metadata_sync.rs        NEW ~300
├── transcript_export.rs         NEW ~400
├── transcript_fingerprint.rs    NEW ~500
├── metadata_subhandlers.rs      481 (R10a, unchanged)
├── skill_snapshot_subhandlers.rs 547 (R10a, unchanged)
└── paths_utilities.rs           412 (R10a, unchanged)
```

Total: 12 files (was 9 in R10a), all ≤ 800 cap

## §4 关键约束

- **每个新 sibling ≤ 800 行** (QClaw tolerance 810)
- **0 fns dropped**: 128 → 128 (R10a 总数不变)
- **Public API 不变**: PersistenceManager 所有现有 pub method 名/签名不变
- **multi-impl pattern 保持**: 6 个 sibling + 5 个新 sibling 各自 impl PersistenceManager
- **mod.rs 加 5 行 pub mod**
- **cargo test 899/0/1 maintained**
- **cargo fmt clean**
- **0 unwrap/panic/unreachable in production**

## §5 拆分细节 (fn-to-file mapping)

### turn_io.rs (~400)
- save_dialog_turn (pub async)
- load_dialog_turn (pub async)
- delete_dialog_turns_from (pub async)
- delete_turns_after (pub async)
- delete_turns_from (pub async)
- turns_dir, turn_path (helpers)
- ensure_turns_dir (helper)
- list_indexed_turn_paths (helper)

### turn_batch.rs (~400)
- load_session_with_turns (pub async)
- load_session_with_turns_timed (pub async)
- load_session_with_tail_turns (pub async)
- load_session_with_tail_turns_timed (pub async)
- load_session_turns (pub async)
- load_session_tail_turns (pub async)
- load_recent_turns (pub async)
- read_turn_paths (helper)
- turn_status_label (helper)
- + test fns (load_session_tail_turns_returns_latest_turns_in_chronological_order 等 4 个)

### turn_metadata_sync.rs (~300)
- read_metadata_tail_turns (helper async)
- build_session_metadata_for_save (helper)
- update_metadata_after_turn_save (helper)
- 其他 metadata 同步 helpers

### transcript_export.rs (~400)
- export_session_transcript (pub async, 209 行 god method → 拆为 prepare/build/format)
- export_session_transcript_handles_first_selected_turn_without_panicking (test)
- transcript_preview, transcript_text_lines, transcript_value_string, transcript_tool_input, transcript_tool_result (helpers)
- transcript_display_user_content, transcript_assistant_blocks, transcript_thinking_blocks, transcript_tool_blocks (helpers)
- transcript_round_blocks (helper)
- push_transcript_block, build_transcript_section (helpers)
- format_range, offset_range (helpers)

### transcript_fingerprint.rs (~500)
- transcript_path, transcript_meta_path (helpers)
- transcript_fingerprint (helper)
- parse_transcript_turn_selectors (helper)
- parse_transcript_turn_selector (helper)
- parse_transcript_turn_value (helper)
- transcript_normalize_slice_bound, transcript_normalize_index (helpers)
- transcript_select_turn_indices (helper)
- transcript_omitted_turns_label (helper)
- transcript_turn_selectors_support_head_and_tail_ranges (test)
- transcript_turn_selectors_deduplicate_and_sort_results (test)
- transcript_turn_selectors_reject_invalid_syntax (test)

## §6 multi-impl pattern (key difference from facade-wrapper)

延续 R10a 模式：每个新 sibling file 直接定义 `impl PersistenceManager { pub async fn ... }`。Rust 允许多 impl 块自动 link。Visibility 保持 `pub`（不降级）。

## §7 实施步骤 (R10a 经验: 按 fn 数从小到大, 降低风险)

1. turn_metadata_sync.rs (~300) — cargo check
2. transcript_export.rs (~400) — cargo check
3. turn_io.rs (~400) — cargo check
4. transcript_fingerprint.rs (~500) — cargo check
5. turn_batch.rs (~400) — cargo check + cargo test
6. cargo fmt + cargo test verification
7. Atomic single commit (R10a D6 precedent)

每步 cargo check 必须 0 errors。

## §8 Verification

```bash
cargo test -p northhing-core --features product-full --lib
# 期望: 899 passed; 0 failed; 1 ignored (与 R10a baseline 一致)

cargo fmt --check -p northhing-core
# 期望: clean

cargo build --tests -p northhing-core --features product-full
# 期望: 0 errors

# Line counts (each new file ≤ 800)
for f in turn_io turn_batch turn_metadata_sync transcript_export transcript_fingerprint; do
  py -c "import sys; print(sum(1 for _ in open(r'E:\agent-project\northing\src\crates\assembly\core\src\agentic\persistence/${f}.rs')))"
done

# 0 fns dropped
py -c "
import re
from pathlib import Path
# Get R10a fn count from git
out = subprocess.run(['git', 'show', '4adb7ba:src/crates/assembly/core/src/agentic/persistence/turn_subhandlers.rs'],
    capture_output=True, text=True)
r10a_turn_fns = set(re.findall(r'    (?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', out.stdout))
# Get new fn count
new_turn_fns = set()
for f in ['turn_io.rs', 'turn_batch.rs', 'turn_metadata_sync.rs']:
    content = Path(f'E:\agent-project\northing\src\crates\assembly\core\src\agentic\persistence/{f}').read_text(encoding='utf-8')
    new_turn_fns.update(re.findall(r'    (?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', content))
print(f'R10a turn fns: {len(r10a_turn_fns)}')
print(f'R10b turn fns: {len(new_turn_fns)}')
print(f'dropped: {r10a_turn_fns - new_turn_fns}')
"
```

## §9 D-deviation 风险

| Item | Plan 接受 | 实际预期 | 备注 |
|---|---|---|---|
| turn_batch.rs ~400 cap | 上限 410 | ~400 | load_session_with_tail_turns_timed 126 行 + batch fns 可能超 |
| transcript_fingerprint.rs ~500 cap | 上限 510 | ~500 | 14 fns + 3 tests, 估计偏高 |
| facade 200 cap | 上限 210 | 70 (unchanged) | OK |

如果 turn_batch / transcript_fingerprint 实际超 800，需 R10c 三次拆。

## §10 D-deviation closure

R10b 完成后:
- D1 turn_subhandlers 1195 → 3 files ≤ 800 → CLOSED
- D2 transcript_subhandlers 981 → 2 files ≤ 800 → CLOSED
- R10b verdict 预期 9.x/10 APPROVE（无 cap deviation）

## §11 Spec review check-list

QClaw review (e7d9927) R10b 草案为本 spec source. 本 spec 与 QClaw 草案一致. 无新方案.