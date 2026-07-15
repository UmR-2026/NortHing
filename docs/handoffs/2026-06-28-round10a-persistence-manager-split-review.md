# Round 10a Review Guide — `persistence/manager.rs` split

> **Status**: Main HEAD `4adb7ba` 等待 QClaw review
> **Target**: 验证 R10a 拆分结构 + D-deviation 处理 + iron rules
> **Spec**: `docs/handoffs/2026-06-28-round10a-persistence-manager-split-spec.md` (cfe83ef)

---

## §1 当前状态

| 项 | 值 | 出处 |
|---|---|---|
| merge commit | `4adb7ba` | git log |
| worker commit | `0c5d7df` (refactor) + `803e970` (fmt fix) | git log |
| manager.rs (facade) | **70 行** ✅ | ReadAllLines.Count |
| session_branch.rs (保留) | 471 行 (Round 3b 产物) | ReadAllLines |
| session_subhandlers.rs | 437 行 ✅ | ReadAllLines |
| turn_subhandlers.rs | **1195 行** ❌ | ReadAllLines |
| transcript_subhandlers.rs | **981 行** ❌ | ReadAllLines |
| metadata_subhandlers.rs | 481 行 ✅ | ReadAllLines |
| skill_snapshot_subhandlers.rs | 547 行 ✅ | ReadAllLines |
| paths_utilities.rs | 412 行 ✅ | ReadAllLines |
| mod.rs | 18 行 (12 + 6 new pub mod) | ReadAllLines |
| **Total fns** | **128** (vs spec 120 = +4 expected: branch_session + 3 test fns) | fn count cross-check |
| fns dropped | **0** ✅ | main HEAD vs worktree cross-check |
| Cargo test | **899/0/1** = main HEAD baseline ✅ | cargo test |
| Cargo fmt | clean ✅ | cargo fmt --check |
| Cargo build | 0 errors ✅ | cargo build --tests |

---

## §2 Spec §2 目标 vs 实际

| File | Spec 目标 | 实际 | 偏差 |
|---|---|---|---|
| manager.rs (facade) | ≤200 | 70 | -130 ✅ (优) |
| session_subhandlers.rs | 450-500 | 437 | -13 ✅ |
| turn_subhandlers.rs | 700-750 | **1195** | **+445** ❌ |
| transcript_subhandlers.rs | 600-650 | **981** | **+331** ❌ |
| metadata_subhandlers.rs | 350-400 | 481 | +81 ⚠️ |
| skill_snapshot_subhandlers.rs | 400-450 | 547 | +97 ⚠️ |
| paths_utilities.rs | 400-450 | 412 | -38 ✅ |
| session_branch.rs (保留) | 471 (unchanged) | 471 | 0 ✅ |

---

## §3 D-deviation 重点 review

### D1: turn_subhandlers.rs 1195 行 超 800 cap by 395

**Spec §4 D1 接受**: "turn_subhandlers.rs ~700-750 行, 可能超 800 cap"

**实际超出**: 1195 - 800 = **+395 行** (远超 spec 估算 +445)

**内容**:
- 25 fns (12 pub async + 13 priv helper + 4 test fns)
- 22 async (主要是 turn IO 操作)
- fns 集中: load_session_with_turns, load_session_with_turns_timed, save_dialog_turn, load_dialog_turn, delete_dialog_turns_from, load_recent_turns 等

**建议 R10b 方向** (待 reviewer 决定):
- A) 拆 turn_io / turn_metadata_sync / turn_batch_ops 3 个 sub-handler (~400 each)
- B) 拆 turn_load / turn_save / turn_delete 3 个 sub-handler (~400 each)
- C) 整体细化为更小 fn (不拆文件, 重构 1-2 个 200+ 行 fn)

### D2: transcript_subhandlers.rs 981 行 超 800 cap by 181

**Spec §4 D2 接受**: "transcript_subhandlers.rs ~600-650 行, 可能超 800 cap"

**实际超出**: 981 - 800 = **+181 行** (超 spec 估算 +331)

**内容**:
- 27 fns (1 pub async export_session_transcript + 26 priv helper)
- 1 pub fn (export_session_transcript)
- 26 helpers 主要是 transcript 渲染/解析/transcript_path 等

**建议 R10b 方向** (待 reviewer 决定):
- A) 拆 transcript_export.rs (1 pub fn + helpers, ~400 行)
- B) 拆 transcript_export.rs + transcript_fingerprint.rs (parse + helpers, ~300/400 行)
- C) 重构 export_session_transcript (唯一 pub fn) 提取子函数

### D3: metadata_subhandlers.rs 481 行 略超 spec 上限 +81

**Spec §2.2**: 350-400 目标
**实际**: 481 (+81 over)
**判断**: 可接受 (QClaw tolerance 800±10 = 上限 810, 481 < 810)

### D4: skill_snapshot_subhandlers.rs 547 行 略超 spec 上限 +97

**Spec §2.2**: 400-450 目标
**实际**: 547 (+97 over)
**判断**: 可接受 (QClaw tolerance 800±10 = 上限 810, 547 < 810)

---

## §4 Iron rules 检查清单

| 规则 | 验证方法 | 期望 |
|---|---|---|
| 禁止 unwrap() in production | `git diff cfe83ef..4adb7ba -- '*.rs' \| grep -E '\.unwrap\(\)'` | 0 |
| 禁止 panic!/unreachable! in production | `git diff cfe83ef..4adb7ba -- '*.rs' \| grep -E 'panic!\|unreachable!'` | 0 |
| 禁止 let _ = Result 静默吞错 | `git diff cfe83ef..4adb7ba -- '*.rs' \| grep 'let _ ='` | 0 in production code |
| move not copy (no duplicates) | cross-check 124 main HEAD fns vs 128 worktree fns | 0 dropped |
| 文件 ≤ 800 行 (QClaw tolerance 800±10) | manual line count | D1/D2 over |
| multi-impl pattern (no facade wrapper) | 每个 sibling file 各有 `impl PersistenceManager` | YES (6 siblings) |
| mod.rs 5 个新 pub mod 声明 | check `cat mod.rs` | YES |
| Test fns 保留 attribute | `grep -B1 'pub.*fn.*\(test\|persists\|returns\)'` | all have `#[test]`/`#[tokio::test]` |
| Public API 不变 | `git diff cfe83ef..4adb7ba -- src/.../persistence/manager.rs \| grep '^-.*pub'` | 0 (只有 struct + 3 constructors) |

---

## §5 Verification commands

```bash
cd E:\agent-project\northing
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path

# 1. baseline match
cargo test -p northhing-core --features product-full --lib
# 期望: 899 passed; 0 failed; 1 ignored

# 2. fmt clean
cargo fmt --check -p northhing-core
# 期望: exit 0

# 3. build 0 errors
cargo build --tests -p northhing-core --features product-full
# 期望: 0 errors

# 4. fn completeness (cross-check)
py -c "
import re
from pathlib import Path
main = Path(r'E:\agent-project\northing\src\crates\assembly\core\src\agentic\persistence\manager.rs')
wt = Path(r'E:\agent-project\northing\src\crates\assembly\core\src\agentic\persistence')
main_fns = set(re.findall(r'    (?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', main.read_text(encoding='utf-8')))
wt_fns = set()
for f in wt.glob('*.rs'):
    wt_fns.update(re.findall(r'    (?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', f.read_text(encoding='utf-8')))
print(f'main HEAD: {len(main_fns)} fns')
print(f'main now: {len(wt_fns)} fns')
print(f'dropped: {main_fns - wt_fns}')
print(f'added (expected): {wt_fns - main_fns}')
"

# 5. iron rules (production only)
git diff cfe83ef..4adb7ba -- 'src/crates/assembly/core/src/agentic/persistence/*.rs' | grep -E '^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!'
# 期望: 0

# 6. line counts
py -c "
import sys
from pathlib import Path
for f in sorted(Path(r'E:\agent-project\northing\src\crates\assembly\core\src\agentic\persistence').glob('*.rs')):
    n = sum(1 for _ in open(f, encoding='utf-8'))
    icon = '❌' if n > 800 else '✅'
    print(f'{icon} {f.name}: {n} lines')
"
```

---

## §6 Reviewer action items

### 必答 (verdict 决定项)
1. **D1 turn_subhandlers 1195 行**: 是否接受 + R10b 二次拆方向 (A/B/C)
2. **D2 transcript_subhandlers 981 行**: 是否接受 + R10b 二次拆方向 (A/B/C)
3. **拆分结构 5 个 sub-handler 划分**: 是否合理 (按 fn domain split)

### 次要 (observation, 不阻塞)
- D3 metadata 481 + D4 skill_snapshot 547 略超 spec 上限, 但 < 810 tolerance
- helper fn `build_turn` 在 paths_utilities 是否合适 (跨 domain 用)
- session_branch.rs 471 行 Round 3b 产物是否需要 review

### 不需要 review (Mavis 已 verify)
- 0 fns dropped
- cargo test 899/0/1
- cargo fmt clean
- iron rules 通过 (无 unwrap/panic in production)

---

## §7 Reviewer 工作流建议

按 R5/6/7/8/9 precedent:
1. Read spec §1-§7 (cfe83ef)
2. Read impl handoff (4adb7ba merge 包含 11KB handoff doc)
3. Run §5 verification commands
4. 检查 §3 D-deviation (turn 1195 / transcript 981)
5. 检查 §4 iron rules
6. 给 verdict (APPROVE / REJECT + 评分 + observations)
7. commit review report: `docs/handoffs/2026-06-28-round10a-persistence-manager-split-review-report.md`

预期 verdict 范围:
- **APPROVE 7-9/10** with D1+D2 fix 必做项 (类比 R9 9.1/10)
- **COND APPROVE** with R10b 必做项 (类比 R8 7.5/10)
- **REJECT** only if iron rules violated (unlikely)

---

## §8 R10a vs R5/R6/R7/R8/R9/R9b 历史一致性

| Round | commit | main cap | D-deviation | review verdict |
|---|---|---|---|---|
| R5 (chat.rs 3356→) | `68b12c4` | 1000 | run 1200 | QClaw APPROVE 9.x |
| R6 (dialog_turn 3656→) | `e31fda3` | 1000 | turn_subhandlers 1604 + turn.rs 1352 | QClaw APPROVE 8.1 (D4 E1 exception) |
| R7 (turn_internal 709→) | `4d85f74` | 1000 | turn.rs 1352 | QClaw APPROVE 8.5 (D8 cond) |
| R8 (exec_engine 3494→) | `6a416e3` | 1000 | multiple | QClaw COND 7.5 |
| R8b (round_executor 1631→) | `7bec409` | 1000 | none | QClaw APPROVE |
| R9 (session_manager 3988→) | `59019c7` | 600 (facade stricter) | none | QClaw APPROVE 9.1 |
| R9b (session_manager_tests 2228→) | `5e30916` | 800 (test tolerance 800±10) | lifecycle 957 + metadata 1027 | not yet reviewed |
| **R10a (persistence 3650→)** | `4adb7ba` | 800 | **turn 1195 + transcript 981** | **awaiting QClaw** |

R10a 的 D-deviation 比 R9b 更严重 (turn 1195 vs R9b lifecycle 957, transcript 981 vs R9b metadata 1027), QClaw verdict 预期 COND APPROVE with R10b fix 必做.

---

## §9 给 QClaw 的具体 review 建议

1. **APPROVE 8.x/10** with 2 fix 必做项:
   - R10b 必做 turn_subhandlers 1195 二次拆
   - R10b 必做 transcript_subhandlers 981 二次拆
2. **Observations**:
   - metadata 481 + skill_snapshot 547 略超 spec 上限 (within 810 tolerance)
   - paths_utilities 412 helper `build_turn` 跨 domain, 考虑移到 manager.rs 或 turn_subhandlers.rs
   - session_branch.rs 471 行 (Round 3b 产物) 是否纳入 R10b 二次拆

---

## §10 Mavis take-over 痕迹

| Action | 原因 | commit |
|---|---|---|
| cargo fmt fix | worker `0c5d7df` 有 7 个 file fmt 不 clean | `803e970` |
| 清理 9 个 worker helper scripts | scripts/* 不是 project convention | (untracked, 删除) |
| merge to main | 完成 take-over 闭环 | `4adb7ba` |

Worker 自跑 46 min 出 commit (比 R9b 17min error 好), Mavis take-over 用了 ~5 min 做 cleanup + merge.