# Round 11 Implementation Handoff

> `services-integrations/remote_connect.rs` 3446 → `remote_connect/` subdir + mod.rs sub-facade + 5 new siblings
> commit `3b50768` (refactor) + `5ccd835` (fmt fix), merge `df81bb9` on main
> Mavis take-over from worker done at 01:07

## TL;DR

| 指标 | 数值 |
| --- | --- |
| 拆分行数 | 3446 → 11 files (5 preserved + 5 new + 1 mod.rs) |
| 总 fn 数 | 59 preserved (0 dropped) |
| Cargo test | 899/0/1 = main HEAD baseline ✅ |
| Cargo fmt | clean ✅ |
| Plan | `plan_f323dc77`, decision `override_accept` |

## 拆分布局 (sub-domain split per fn prefix)

| File | Lines | Status | 用途 |
|---|---|---|---|
| `mod.rs` | 81 | ✅ ≤200 (facade) | sub-facade + `pub mod` 11 + `pub use` 12 重导出 |
| `device.rs` | 74 | preserved | DeviceIdentity |
| `encryption.rs` | 189 | preserved | encrypt/decrypt + KeyPair |
| `pairing.rs` | 282 | preserved | PairingProtocol + state |
| `qr_generator.rs` | 82 | preserved | QR code gen |
| `relay_client.rs` | 511 | preserved | RelayClient + websocket |
| `remote_request_builders.rs` | 638 | ✅ ≤800 | build_remote_* 7 fns + RemoteImageContext + Adapter |
| `remote_session_tracker.rs` | **1278** | ❌ **+478 over** | remote_session_* 6 fns + RemoteSessionTracker struct |
| `remote_command_handlers.rs` | **1309** | ❌ **+509 over** | handle_remote_* 6 fns + 11 types + remote_dialog_* 2 fns |
| `remote_file_io.rs` | 176 | ✅ ≤800 | read_remote_* 3 + remote_file_* 4 + path utilities |
| `remote_workspace_resolver.rs` | 102 | ✅ ≤800 | resolve_remote_* 5 + path utilities |

## D-deviation 状态（严重）

| Item | Lines | Over | 比 R10a 更严重？ |
|---|---|---|---|
| `remote_command_handlers.rs` | 1309 | +509 (64%) | ✅ 比 R10a turn 1195 (+49%) 更严重 |
| `remote_session_tracker.rs` | 1278 | +478 (60%) | ✅ 比 R10a transcript 981 (+23%) 更严重 |

跟 R10a/R9b precedent：accept + merge + R11b 必做二次拆。

**R11b 拆分方案**（待 QClaw review 给方向）：
- `remote_command_handlers.rs` 1309 → 拆 3 sub:
  - `remote_dialog_handlers.rs` (handle_remote_dialog_*)
  - `remote_session_handlers.rs` (handle_remote_session_command 等)
  - `remote_cancel_handlers.rs` (cancel_remote_task + RemoteCancelTaskRequest + RemoteCancelRuntimeHost)
- `remote_session_tracker.rs` 1278 → 拆 2 sub:
  - `remote_session_state.rs` (RemoteSessionTracker struct + impl + state mutations)
  - `remote_session_response_builders.rs` (remote_session_info/list/created/deleted/model_updated response DTOs)

## Mavis take-over 修正清单

| # | 问题 | 修复 |
|---|---|---|
| 1 | cargo fmt 6 文件不 clean (mod.rs + 5 new siblings) | `cargo fmt -p services-integrations` + commit `5ccd835` |
| 2 | (无 worker scripts 需要清理 — worker self-contained) | — |
| 3 | merge to main as `df81bb9` | — |

## Round 5/6/7/8/9/10a/10b D6 一致性

| Round | commit | main cap | D-deviation | review verdict |
|---|---|---|---|---|
| R5 (chat.rs) | `68b12c4` | 1000 | run 1200 | QClaw APPROVE |
| R6 (dialog_turn) | `e31fda3` | 1000 | turn 1604 | QClaw 8.1 (D4 E1 exception) |
| R7 (turn_internal) | `4d85f74` | 1000 | turn.rs 1352 | QClaw 8.5 (D8 cond) |
| R8 (exec_engine) | `6a416e3` | 1000 | multiple | QClaw COND 7.5 |
| R8b (round_executor) | `7bec409` | 1000 | none | QClaw APPROVE |
| R9 (session_manager) | `59019c7` | 600 | none | QClaw 9.1 |
| R9b (tests) | `5e30916` | 800 | lifecycle 957 + metadata 1010 | (skipped review) |
| R10a (persistence) | `4adb7ba` | 800 | turn 1195 + transcript 981 | QClaw 7.5 COND |
| R10b (persistence 二次拆) | `2882a74` | 800 | none | (awaiting review) |
| **R11 (remote_connect)** | `df81bb9` | 800 | **command 1309 + session 1278** | **(awaiting review)** |

**R11 是项目历史最严重的 cap deviation** (1309 / 1278 都 > 1000，远超 R10a 的 1195/981)。

## 验证命令 (reproducible)

```bash
cd E:\agent-project\northing
export PATH="/c/msys64/mingw64/bin:$PATH"  # PowerShell: $env:Path = "C:\msys64\mingw64\bin;" + $env:Path

cargo test -p northhing-core --features product-full --lib
# 期望: 899 passed; 0 failed; 1 ignored

cargo fmt --check -p northhing-services-integrations
# 期望: exit 0

# Line counts
py -c "
import sys
from pathlib import Path
for f in sorted(Path(r'E:\agent-project\northing\src\crates\services\services-integrations\src\remote_connect').glob('*.rs')):
    n = sum(1 for _ in open(f, encoding='utf-8'))
    icon = '❌' if n > 800 else '✅'
    print(f'{icon} {f.name}: {n} lines')
"
```

## 给 QClaw 的 review guide (review-guide 文档见 review doc)

重点 review:
1. 11-file 拆分结构是否合理 (5 prefix clusters)
2. D-deviation command 1309 + session 1278 → R11b 必做 二次拆方案
3. mod.rs sub-facade pub use 重导出是否完整 (preserve external API)
4. cross-crate caller 不受影响 (`use northhing_services_integrations::remote_connect::*` 全部 OK)

## Plan & decision

- Plan ID: `plan_f323dc77`
- Decision: `override_accept`
- Decision file: `C:\Users\UmR\.mavis\scratchpads\mvs_4cfd3e045ea44bf1942ff29fa9970579\round11-decision.json`