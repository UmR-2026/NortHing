# Round 11 Review Guide — `remote_connect.rs` split

> **Status**: Main HEAD `df81bb9` 等待 QClaw review (user 决定明早起来自己 review)
> **Target**: 验证 R11 拆分结构 + D-deviation 处理 + iron rules

---

## §1 当前状态

| 项 | 值 |
|---|---|
| merge commit | `df81bb9` |
| worker commit | `3b50768` (refactor) + `5ccd835` (fmt fix) |
| original `remote_connect.rs` | **DELETED** (replaced by `remote_connect/` subdir) |
| mod.rs (facade) | 81 ✅ |
| remote_request_builders.rs | 638 ✅ |
| **remote_session_tracker.rs** | **1278 ❌** |
| **remote_command_handlers.rs** | **1309 ❌** |
| remote_file_io.rs | 176 ✅ |
| remote_workspace_resolver.rs | 102 ✅ |
| device/encryption/pairing/qr_generator/relay_client | preserved |
| **Total fns** | 59 (vs main HEAD 59 baseline = 0 dropped) |
| Cargo test | **899/0/1** = baseline ✅ |
| Cargo fmt | clean ✅ |
| Iron rules | 0 unwrap/panic in production ✅ |

---

## §2 Spec vs 实际偏差

| File | Spec 目标 | 实际 | 偏差 |
|---|---|---|---|
| mod.rs | ≤200 | 81 | -119 ✅ |
| remote_request_builders.rs | 400-500 | 638 | +138 ⚠️ (still ≤800 OK) |
| **remote_session_tracker.rs** | 600-700 | **1278** | **+578 ❌** |
| **remote_command_handlers.rs** | 700-800 | **1309** | **+509 ❌** |
| remote_file_io.rs | 400-500 | 176 | -224 ✅ (worker 拆得比预期小) |
| remote_workspace_resolver.rs | 300-400 | 102 | -198 ✅ |

**worker 拆分不平衡**: session_tracker + command_handlers 集中了过多 fns/structs（worker 可能没拆 RemoteSessionTracker struct 单独），其他 3 个新 sibling 反而比预期小。

---

## §3 D-deviation 重点 review

### D1: `remote_command_handlers.rs` 1309 行 (+509, 64% over)

**Spec §4 D-deviation 接受**: "command_handlers ~700-800"

**实际超出**: 1309 - 800 = **+509 行** (远超 spec 估算 +509, 是 spec tolerance 的 5×)

**内容**:
- handle_remote_* 6 fns (Pub async)
- 11 struct/enum/trait (RemoteCancelDecision/Request/RuntimeHost, RemoteDialogQueuePriority/Policy/SubmissionRequest/ResolvedSubmission/SubmitOutcome/SchedulerOutcomeFact/RuntimeHost, RemoteTerminalPrewarmRequest)
- remote_dialog_* 2 fns (submit_remote_dialog, remote_dialog_submit_outcome_from_scheduler)
- 多个 test fns

**R11b 必做拆 3 sub**:
| New file | Target | Scope |
|---|---|---|
| `remote_dialog_handlers.rs` | ~500 | handle_remote_dialog_* + RemoteDialog* types |
| `remote_session_handlers.rs` | ~400 | handle_remote_session_command 等 |
| `remote_cancel_handlers.rs` | ~400 | cancel_remote_task + RemoteCancel* types |

### D2: `remote_session_tracker.rs` 1278 行 (+478, 60% over)

**Spec §4 D-deviation 接受**: "session_tracker ~600-700"

**实际超出**: 1278 - 800 = **+478 行**

**内容**:
- remote_session_* 6 fns (Pub)
- RemoteSessionTracker struct + impl
- RemoteConnectSubmissionSource enum (放在 mod.rs 中, 但 session_tracker 仍依赖)
- state mutations + response DTOs

**R11b 必做拆 2 sub**:
| New file | Target | Scope |
|---|---|---|
| `remote_session_state.rs` | ~700 | RemoteSessionTracker struct + impl + state mutations |
| `remote_session_response_builders.rs` | ~500 | remote_session_info/list/created/deleted/model_updated response DTOs |

### D3: 其他轻微偏差
- remote_request_builders 638 (spec 400-500, +138 偏多但 OK)
- remote_file_io 176 (spec 400-500, -224 偏少, worker 拆得保守)
- remote_workspace_resolver 102 (spec 300-400, -198 偏少)

---

## §4 Iron rules 检查清单

| 规则 | 验证方法 | 期望 |
|---|---|---|
| 禁止 unwrap() in production | `git grep -n 'unwrap()' src/crates/services/services-integrations/src/remote_connect/*.rs` | 0 |
| 禁止 panic!/unreachable! | `git grep -n 'panic!\|unreachable!' ...` | 0 |
| 禁止 let _ = Result 静默吞错 | `git grep -n 'let _ = ' ...` | 0 in production |
| move not copy | cross-check 59 main HEAD fns vs worktree | 0 dropped ✅ |
| 文件 ≤ 800 行 (QClaw tolerance 800±10) | wc -l | D1+D2 over |
| mod.rs 11 pub mod 声明 | `cat mod.rs` | YES ✅ |
| Test fns 保留 attribute | grep `#[test]` / `#[tokio::test]` | all preserved |
| Public API 不变 | `git grep -l 'use northhing_services_integrations::remote_connect::'` count 不变 | YES |

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

# 3. fn completeness (cross-check)
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

# 4. line counts
py -c "
import sys
from pathlib import Path
for f in sorted(Path(r'E:\agent-project\northing\src\crates\services\services-integrations\src\remote_connect').glob('*.rs')):
    n = sum(1 for _ in open(f, encoding='utf-8'))
    icon = '❌' if n > 800 else '✅'
    print(f'{icon} {f.name}: {n} lines')
"

# 5. iron rules
git grep -n 'unwrap()\|panic!\|unreachable!' src/crates/services/services-integrations/src/remote_connect/*.rs
# 期望: 0

# 6. cross-crate caller count (preserved external API)
git grep -l 'use northhing_services_integrations::remote_connect::' | wc -l
# 与 baseline 对比, 必须相等
```

---

## §6 Reviewer action items

### 必答 (verdict 决定项)
1. **D1 remote_command_handlers 1309 行**: R11b 拆 3 sub 方案是否可接受 (dialog/session/cancel)
2. **D2 remote_session_tracker 1278 行**: R11b 拆 2 sub 方案是否可接受 (state/response_builders)
3. **拆分结构 11 file 总览**: 5 prefix cluster 划分是否合理 (request_builders / session_tracker / command_handlers / file_io / workspace_resolver + 5 preserved)
4. **D3 不平衡**: request_builders 638 + file_io 176 + resolver 102, vs session_tracker 1278 + command_handlers 1309 — 是否要求 worker 重新平衡

### 次要 (observation, 不阻塞)
- mod.rs 81 行 (spec 50-100 OK)
- RemoteConnectSubmissionSource 放 mod.rs (cross-sibling shared type)
- preserved siblings 未被 touch

### 不需要 review (Mavis 已 verify)
- 0 fns dropped
- cargo test 899/0/1
- cargo fmt clean
- iron rules 通过

---

## §7 Reviewer 工作流建议

按 R5/6/7/8/9/10 precedent:
1. Read spec `docs/handoffs/2026-06-29-round11-remote-connect-split-spec.md` (cccf9e5)
2. Read impl handoff `docs/handoffs/2026-06-29-round11-remote-connect-split-impl.md` (df81bb9)
3. Run §5 verification commands
4. 检查 §3 D-deviation (D1+D2 是历史最严重 cap 偏离)
5. 检查 §4 iron rules
6. 给 verdict (APPROVE / REJECT + 评分 + observations)

预期 verdict:
- **COND APPROVE 7-8/10** with R11b REQUIRED (类比 R10a 7.5/10)
- 0-6/10 = REJECT (if iron rules violated, unlikely)

---

## §8 Round 11 vs 历史 D-deviation 对比

| Round | File | Lines | Over cap | Severity |
|---|---|---|---|---|
| R5 | chat_run.rs | 1200 | +20% | minor |
| R6 | turn.rs | 1352 | +35% | medium |
| R8 | round_executor.rs | 1631 | +104% | high |
| R9b | lifecycle_tests.rs | 957 | +20% | minor (test tolerance) |
| R9b | metadata_tests.rs | 1027 | +28% | minor (test tolerance) |
| R10a | turn_subhandlers.rs | 1195 | +49% | medium |
| R10a | transcript_subhandlers.rs | 981 | +23% | minor |
| **R11** | **remote_command_handlers.rs** | **1309** | **+64%** | **HIGH (项目最高非 test cap 偏离)** |
| **R11** | **remote_session_tracker.rs** | **1278** | **+60%** | **HIGH** |

R11 的 2 个 deviation 排在 R8 之后，**第 2 高** (R8 round_executor 是 +104%)。R11b 必做。

---

## §9 R11b 拆分布局（QClaw 钦定方向候选）

按 fn prefix + struct owner 拆分:

```
remote_connect/
├── mod.rs                       (preserved, ~100 lines)
├── device.rs                    (preserved)
├── encryption.rs                (preserved)
├── pairing.rs                   (preserved)
├── qr_generator.rs              (preserved)
├── relay_client.rs              (preserved)
├── remote_request_builders.rs   (preserved, 638)
├── remote_file_io.rs            (preserved, 176)
├── remote_workspace_resolver.rs (preserved, 102)
├── remote_session_state.rs             NEW ~700 (from session_tracker)
├── remote_session_response_builders.rs NEW ~500 (from session_tracker)
├── remote_dialog_handlers.rs           NEW ~500 (from command_handlers)
├── remote_session_handlers.rs          NEW ~400 (from command_handlers)
└── remote_cancel_handlers.rs           NEW ~400 (from command_handlers)
```

Total 13 files (was 11), all ≤ 800 cap ✅.

R11b 约束:
- 0 fns dropped (59 → 59)
- public API 不变 (mod.rs pub use 重导出覆盖所有)
- cargo test 899/0/1 maintained
- cargo fmt clean

---

## §10 Mavis take-over 痕迹

| Action | 原因 | commit |
|---|---|---|
| cargo fmt fix | worker `3b50768` 有 6 file fmt 不 clean | `5ccd835` |
| merge to main | 完成 take-over 闭环 | `df81bb9` |

Worker 自跑 ~50 min 出 commit (跟 R10a 46 min 接近, model M2.7-highspeed 节奏稳)。Mavis take-over 用了 ~5 min 做 fmt + merge。

## §11 User 上下文 (Mavis 内部记录)

- 项目: 个人 side project, 给 agent 做"住所"
- 不是工作, 个人行为驱动
- 用户跨夜工作是常态
- Mavis 按 personal pace 派 plan