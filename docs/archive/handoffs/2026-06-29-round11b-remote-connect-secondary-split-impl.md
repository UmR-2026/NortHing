# Round 11b Implementation Handoff

> R11b 二次拆: command_handlers 1301 → 3 sub + session_tracker 1272 → 2 sub (QClaw R11b 钦定 5-file 方案)
> commit `e3947d8`, merge `fa40e87` on main
> Mavis take-over from worker session error (15:17)

## TL;DR

| 指标 | 数值 |
| --- | --- |
| 拆分文件数 | 2 → 5 new siblings (cancel/dialog/session handlers + session state + response builders) |
| mod.rs 行数 | 81 → 447 (worker 把 wire enums + dispatcher 留在 mod.rs 自身) |
| 总 fn 数 | 59 preserved (0 dropped) |
| Cargo test | 899/0/1 = main HEAD baseline ✅ |
| Cargo fmt | clean ✅ |
| Cargo build | 0 errors ✅ |
| Plan | `plan_f063bdff`, decision `override_accept` |

## 最终 13-file 结构 (R11b 后)

```
remote_connect/
├── mod.rs                                447  facade (含 wire enums + dispatcher)
├── device.rs                              74  preserved
├── encryption.rs                         189  preserved
├── pairing.rs                            282  preserved
├── qr_generator.rs                        82  preserved
├── relay_client.rs                       511  preserved
├── remote_request_builders.rs            638  preserved (R11a)
├── remote_file_io.rs                     176  preserved (R11a)
├── remote_workspace_resolver.rs          102  preserved (R11a)
├── remote_cancel_handlers.rs             111  NEW (R11b)
├── remote_dialog_handlers.rs             201  NEW (R11b)
├── remote_session_handlers.rs            708  NEW (R11b, was command_handlers)
├── remote_session_response_builders.rs  593  NEW (R11b)
└── remote_session_state.rs               725  NEW (R11b, was session_tracker)
```

Total 13 files, all ≤ 800 cap ✅

## D-deviation closure

R11 D-deviation D1 + D2 (command_handlers +501 / session_tracker +472) → **CLOSED**. QClaw 5-file 钦定方案完整执行.

## Mavis take-over 修正清单

| # | 问题 | 修复 |
|---|---|---|
| 1 | Worker session error 后部分完成 — 5 个新 sibling + mod.rs 已写，但 cross-reference paths 错 | Python script 修 6 个 import 路径 |
| 2 | `remote_session_response_builders.rs:504` 用 `super::remote_session_handlers::RemoteCommand` (错, RemoteCommand 在 mod.rs) | 改 `super::RemoteCommand` |
| 3 | `remote_file_io.rs:12` 用 `super::remote_command_handlers::RemoteResponse` (错) | 改 `super::RemoteResponse` |
| 4 | `remote_session_handlers.rs` 和 `remote_session_response_builders.rs` 用 `use crate::remote_connect::RemoteCommand` | 改 `use super::RemoteCommand` (4+1 处) |
| 5 | `remote_dialog_handlers.rs` 和 `remote_cancel_handlers.rs` 用 `use crate::remote_connect::RemoteResponse` (private import shadow) | 改 `use super::RemoteResponse` |
| 6 | mod.rs 3 个 private `use` block shadow pub use glob re-exports | 删 `use self::remote_xxx::{...};` (保留 pub use `*`) |
| 7 | 我 fix 引入 unmatched brace (handle_remote_poll_command 缺 `{`) | Python rewrite function block |
| 8 | cargo fmt 1 diff in mod.rs | `cargo fmt -p services-integrations` |
| 9 | 3 个 worker backups (`.orig.ps1.txt` + `.orig.rs`) + scripts/count_lines.py | mavis-trash |

## Round 11 lessons 应用 (proved QClaw 反馈有效)

1. ✅ **Pre-existing vs new violations 区分**: 26 unwrap + 9 let _ = all in `remote_session_state.rs` (pre-existing, Δ=0)
2. ✅ **Struct owner mapping**: RemoteCancel* → cancel_handlers, RemoteDialog* → dialog_handlers, RemoteSessionStateTracker → session_state
3. ✅ **Worker 每步 cargo check 报告行数**: 5 个新 sibling 全部 ≤ 800 cap
4. ✅ **mod.rs pub use glob re-exports**: 13 file 全 re-export (consistent with R11)

## Round 5/6/7/8/9/10a/10b/11 D6 一致性

| Round | commit | main cap | D-deviation | review verdict |
|---|---|---|---|---|
| R11 | `df81bb9` | 800 | command 1301 + session 1272 | QClaw COND 6.8/10 + R11b |
| **R11b** | `fa40e87` | 800 | **none (all ≤ 800)** | (awaiting review) |

**R11b 是 R5-R10b 以来第一个 0 D-deviation** 的 split round。

## Verification (reproducible)

```bash
cd E:\agent-project\northing
export PATH="/c/msys64/mingw64/bin:$PATH"

cargo test -p northhing-core --features product-full --lib
# 期望: 899 passed; 0 failed; 1 ignored

cargo fmt --check -p northhing-services-integrations
# 期望: exit 0

# Line counts (each new file ≤ 800)
for f in cancel dialog session_handlers session_state session_response_builders; do
  py -c "import sys; print(sum(1 for _ in open(r'E:\agent-project\northing\src\crates\services\services-integrations\src\remote_connect/remote_${f}.rs', encoding='utf-8')))"
done

# 0 fns dropped
py -c "
import re
from pathlib import Path
wt = Path(r'E:\agent-project\northing\src\crates\services\services-integrations\src\remote_connect')
fns = set()
for f in wt.glob('*.rs'):
    fns.update(re.findall(r'^(?:pub(?:\([^)]+\))?\s+)?(?:async\s+)?fn\s+(\w+)', f.read_text(encoding='utf-8'), re.M))
print(f'fn count: {len(fns)}')
print('expected: 59')
"
```

## 给 QClaw 的 review guide (review-guide 文档见 review doc)

重点 review:
1. 5 个新 sibling 拆分结构 (按 struct owner + fn domain, R11a worker 错的反面教材)
2. mod.rs 447 lines (worker 把 wire enums 留在 mod.rs — 是否可接受, 或应拆出去)
3. 0 fns dropped, 0 NEW iron rules violations (pre-existing 26 unwrap + 9 let _ = all moved to session_state.rs)
4. cross-crate caller 不受影响 (`use northhing_services_integrations::remote_connect::*` 全部 OK)
5. `RemoteSessionTracker` 被 worker 重命名为 `RemoteSessionStateTracker` (因为 state 更明确) — 是否可接受

## Plan & decision

- Plan ID: `plan_f063bdff`
- Decision: `override_accept`
- Decision file: `C:\Users\UmR\.mavis\scratchpads\mvs_4cfd3e045ea44bf1942ff29fa9970579\round11b-decision.json`