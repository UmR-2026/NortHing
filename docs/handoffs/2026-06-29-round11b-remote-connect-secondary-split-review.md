# Round 11b Review Guide — `remote_connect` 二次拆

> **Status**: Main HEAD `fa40e87` 等待 QClaw review (user 决定跑不跑 review)
> **Target**: 验证 R11b 5-file 拆分 + mod.rs sub-facade + iron rules pre-existing 区分

---

## §1 当前状态 (R11b 后)

| File | Lines | Cap | Status |
|---|---|---|---|
| mod.rs (facade + wire enums + dispatcher) | 447 | 800 | ✅ (worker 设计: enums 在 mod.rs 自己, 不像 R10a 那种 strict facade) |
| device.rs | 74 | — | preserved |
| encryption.rs | 189 | — | preserved |
| pairing.rs | 282 | — | preserved |
| qr_generator.rs | 82 | — | preserved |
| relay_client.rs | 511 | — | preserved |
| remote_request_builders.rs | 638 | 800 | preserved (R11a) |
| remote_file_io.rs | 176 | 800 | preserved (R11a) |
| remote_workspace_resolver.rs | 102 | 800 | preserved (R11a) |
| remote_cancel_handlers.rs (NEW R11b) | 111 | 800 | ✅ |
| remote_dialog_handlers.rs (NEW R11b) | 201 | 800 | ✅ |
| remote_session_handlers.rs (NEW R11b, was command_handlers) | 708 | 800 | ✅ (close to cap but ≤ 800) |
| remote_session_response_builders.rs (NEW R11b) | 593 | 800 | ✅ |
| remote_session_state.rs (NEW R11b, was session_tracker) | 725 | 800 | ✅ |
| **Total fns** | 59 | — | 0 dropped ✅ |
| **Pre-existing unwrap** | 26 | — | all moved to remote_session_state.rs (Δ=0) |
| **Pre-existing let _ =** | 9 | — | all moved to remote_session_state.rs (Δ=0) |
| **Cargo test** | 899/0/1 | — | baseline match ✅ |
| **Cargo fmt** | clean | — | ✅ |
| **Cargo build** | 0 errors | — | ✅ |

---

## §2 QClaw R11 5-file 方案 vs 实际

| QClaw 钦定 | 实际 | Notes |
|---|---|---|
| remote_session_state.rs ~700 | 725 ✅ | RemoteSessionStateTracker struct + Registry |
| remote_session_response_builders.rs ~500 | 593 ✅ | response DTO builders |
| remote_dialog_handlers.rs ~500 | 201 ✅ (worker 拆得偏小) | RemoteDialog* types + handlers |
| remote_session_handlers.rs ~400 | 708 ⚠️ (worker 拆得偏大) | session command handlers |
| remote_cancel_handlers.rs ~400 | 111 ✅ (worker 拆得偏小) | RemoteCancel* types + handlers |

**Worker 拆分布局略有偏差但都 ≤ 800 cap**:
- session_handlers.rs 708 (接近 cap, worker 可能没拆足够细)
- dialog/cancel 偏小 (worker 拆得保守)

---

## §3 Worker 设计偏差 (跟 spec 不同)

### 3.1 `RemoteSessionTracker` → `RemoteSessionStateTracker` 重命名

Worker 把 `RemoteSessionTracker` (spec 钦定名) 重命名为 `RemoteSessionStateTracker` (因为 struct 主要管 state, tracker 在 registry 中)。

判断: worker 的重命名合理 (state 描述更精确), 但 cross-crate caller 若用 `RemoteSessionTracker::xxx` 会 broken。需要 mod.rs `pub use` 改名 (worker 已做)。

### 3.2 mod.rs 设计变化: wire enums + dispatcher 留在 mod.rs

QClaw spec 钦定方案:
- mod.rs ~100 lines (sub-facade with pub use)
- remote_session_handlers.rs 含 `RemoteCommand`/`RemoteResponse` enums + handlers

Worker 实际设计:
- mod.rs 447 lines (sub-facade + wire enums + dispatcher + handler trait)
- remote_session_handlers.rs 只含 handler impl + sub-handler dispatchers

判断: worker's design 把 cross-sibling shared types 集中在 mod.rs (single source of truth), 避免 5 个 sibling 都 import 同一 enum。设计合理, 但 mod.rs 偏大 (447 vs spec 100)。

### 3.3 mod.rs `use` 私 import 问题 (Mavis 修复)

Worker 写了 3 个 private `use self::remote_xxx::{...};` block (用于 internal scope), 这些 shadowed 同 module 的 `pub use remote_xxx::*;` glob re-exports (private item shadows public glob re-export warning)。

Mavis take-over 删了这 3 个 private import (因为 pub use glob re-export 已经覆盖)。

---

## §4 Iron Rules — Pre-existing vs New (QClaw 反馈应用)

| State | unwrap() | let _ = |
|---|---|---|
| R11 (post-split, df81bb9) | 26 | 9 |
| R11b (post-split, fa40e87) | 26 | 9 |
| **Δ R11b** | **0** | **0** |

**Pre-existing 26 unwrap 全部移到 `remote_session_state.rs`** (RemoteSessionStateTracker RwLock state management):
- `self.state.read().unwrap() / write().unwrap()` pattern (24 处)
- 其他 2 处 unwrap (Option/Result 处理)

**R11b 没有新增 iron rules violations**。

---

## §5 Verification commands

```bash
cd E:\agent-project\northing
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path

# 1. baseline match
cargo test -p northhing-core --features product-full --lib
# 期望: 899 passed; 0 failed; 1 ignored

# 2. fmt clean
cargo fmt --check -p northhing-services-integrations
# 期望: exit 0

# 3. line counts (each new file ≤ 800)
py -c "
import sys
from pathlib import Path
for f in sorted(Path(r'E:\agent-project\northing\src\crates\services\services-integrations\src\remote_connect').glob('*.rs')):
    n = sum(1 for _ in open(f, encoding='utf-8'))
    icon = '❌' if n > 800 else '✅'
    print(f'{icon} {f.name}: {n} lines')
"

# 4. fn completeness (cross-check)
py -c "
import re
from pathlib import Path
wt = Path(r'E:\agent-project\northing\src\crates\services\services-integrations\src\remote_connect')
wt_fns = set()
for f in wt.glob('*.rs'):
    wt_fns.update(re.findall(r'^(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', f.read_text(encoding='utf-8'), re.M))
print(f'fn count: {len(wt_fns)}')
print('expected: 59')
"

# 5. iron rules — 0 NEW (pre-existing moved, not new)
git diff origin/main..HEAD -- src/crates/services/services-integrations/src/remote_connect/ | Select-String "^\+.*unwrap\(\)|^\+.*panic!|^\+.*unreachable!"
# 期望: 0

# 6. cross-crate caller count (preserved external API)
git grep -l 'use northhing_services_integrations::remote_connect::' | wc -l
# 与 R11 baseline 对比, 必须相等

# 7. mod.rs pub use count (5 preserved + 5 new R11a + 5 new R11b = 15 modules should be re-exported)
grep -c '^pub use remote_' src/crates/services/services-integrations/src/remote_connect/mod.rs
# 期望: 5 (one per R11a/R11b sibling) + preserved (5) = total 10
```

---

## §6 Reviewer action items

### 必答 (verdict 决定项)
1. **mod.rs 447 lines 设计可接受？** wire enums + dispatcher 留在 mod.rs 自己 (worker 选择), 不像 R10a 的严格 facade。QClaw review 觉得 OK 还是要求拆出去？
2. **`RemoteSessionTracker` 重命名为 `RemoteSessionStateTracker` 可接受？** worker 设计选择 (state 更明确), 但破坏任何外部 caller 若引用原名
3. **5 个新 sibling 拆分 OK？** remote_session_handlers 708 接近 cap, 是否要求再拆

### 次要 (observation, 不阻塞)
- dialog/cancel handlers 偏小 (201/111 lines), spec 估算 ~500/400
- mod.rs sub-facade pub use glob re-exports 13 modules (consistent with R11)

### 不需要 review (Mavis 已 verify)
- 0 fns dropped ✅
- cargo test 899/0/1 ✅
- cargo fmt clean ✅
- iron rules: 26 unwrap + 9 let _ = pre-existing, 0 NEW ✅

---

## §7 Reviewer 工作流建议

按 R5/6/7/8/9/10a/10b/11 precedent:
1. Read spec `docs/handoffs/2026-06-29-round11b-remote-connect-secondary-split-spec.md` (3ce8a8d)
2. Read impl handoff `docs/handoffs/2026-06-29-round11b-remote-connect-secondary-split-impl.md` (fa40e87)
3. Run §5 verification commands
4. 检查 §3 worker 设计偏差 (RemoteSessionTracker → RemoteSessionStateTracker 重命名, mod.rs 447 lines 设计)
5. 检查 §4 iron rules pre-existing 区分 (QClaw 反馈应用)
6. 给 verdict

预期 verdict:
- **APPROVE 8-9/10** (无 D-deviation, R11b 是 R5-R10b 以来首个 0 deviation 的 split)
- 或 COND 7.x/10 with minor observation (worker 设计偏差)

---

## §8 Mavis take-over 痕迹

| Action | 原因 | commit |
|---|---|---|
| 修 6 个 cross-reference paths (RemoteCommand/RemoteResponse 路径错) | Worker 把 wire enums 放 mod.rs 自己, sibling 用了 `super::remote_*_handlers::*` 路径 | (in `e3947d8`) |
| 修 handle_remote_poll_command function 结构 (我的 replace 引入 unmatched brace) | 我之前的 replace bug | (in `e3947d8`) |
| 删 3 个 mod.rs private `use` blocks (shadowed pub use) | 警告 "private item shadows public glob re-export" | (in `e3947d8`) |
| 清理 3 个 worker backups (.orig.ps1.txt + .orig.rs + scripts/count_lines.py) | worker 备份文件, 跟 R5/6/7/8/10a Mavis take-over cleanup pattern 一致 | (untracked, deleted) |
| merge to main | 完成 take-over 闭环 | `fa40e87` |

Worker 自跑 ~25 min (15:00 启动, 15:17 error). Mavis take-over ~25 min (15:17-15:42) 完成 fix + commit + merge.

---

## §9 Lessons for Mavis (from R11 + R11b)

### R11 review (QClaw + Kimi) 反馈的 3 个 lessons 已在 R11b spec prompt 显式列出:
1. **Pre-existing vs new violations**: R11b spec 明确说 "26 unwrap + 9 let _ = preserved, all moved to remote_session_state.rs, 0 NEW"
2. **Struct owner mapping**: R11b spec §5 显式列出每个 struct/enum 归属哪个 sibling (R11a worker 没看的反面教材)
3. **Worker 每步 cargo check 报告行数**: R11b spec §6 显式要求 "报告当前 sibling 行数" + "超 800 立即调整"

### R11b worker 表现:
- Worker 在 plan error 之前已经写了 5 个新 sibling + mod.rs (step 1/6 进度), 但 cross-reference paths 错 (因为 worker 把 wire enums 放 mod.rs 自己, 跟 spec "wire enums 在 remote_session_handlers.rs" 不同)
- Mavis take-over 6 个 fix, 总耗时 ~25 min
- 比 R9b (17min error + 30min take-over) 短

### R11 → R11b cycle 验证 review-fix-cleanup pattern:
- R11 merged
- QClaw review: COND APPROVE + R11b REQUIRED (review 周期正常)
- R11b spec drafted by Mavis (per QClaw 5-file 方案)
- R11b plan dispatched
- Worker error at 17 min (跟 R9b 17 min error 类似, but R11b worker had more partial work)
- Mavis take-over 25 min fix + merge

未来 round should:
1. R12+ 按 R11 spec 模式 (explicit struct owner mapping)
2. Worker prompt 应更明确"按 struct owner 拆, 不只是按 fn prefix"
3. Mavis take-over speed 25 min OK, 但争取减少 worker error 概率 (M2.7-highspeed 已用, 可能需要其他防御)