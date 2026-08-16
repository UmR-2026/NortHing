# Round 11 Spec: services-integrations/remote_connect.rs 3446 → facade + 5 siblings

> **目标**: `src/crates/services/services-integrations/src/remote_connect.rs` 3446 行 → `remote_connect/` 子目录 + facade + 5 sibling files
> **Pattern**: Round 5 chat.rs sub-domain split (free functions + structs + enums + traits)
> **Draft**: Mavis 2026-06-29 00:15 (critical #1 of post-R10 god object list)

---

## §1 当前状态

| 项 | 值 | 出处 |
|---|---|---|
| 文件路径 | `src/crates/services/services-integrations/src/remote_connect.rs` | wc -l |
| 行数 | **3446** | ReadAllLines.Count |
| 总 fn 数 | **59** (pub + priv) | regex |
| 现有 partial split | **5 sibling files 已存在** | device/encryption/pairing/qr_generator/relay_client |
| 待拆 fn | **~50** (除现有 5 sibling 的 fns) | 59 total - 5 siblings |
| `pub use` 重导出数 | 12 (从 device/encryption/pairing/qr_generator/relay_client) | head -25 |
| lib.rs 声明 | `pub mod remote_connect;` (file-level) | lib.rs |

### 1.1 现有 sibling（保留）

| File | Lines | 用途 |
|---|---|---|
| `device.rs` | 74 | DeviceIdentity |
| `encryption.rs` | 189 | encrypt/decrypt + KeyPair |
| `pairing.rs` | 282 | PairingProtocol + state machine |
| `qr_generator.rs` | 82 | QR code generation |
| `relay_client.rs` | 511 | RelayClient + websocket |

### 1.2 fn 前缀聚类（59 fns，按 call site 拆分依据）

| Prefix | Count | 候选 sibling |
|---|---|---|
| `build_remote_*` | 7 | `remote_request_builders.rs` |
| `remote_session_*` | 6 | `remote_session_tracker.rs` |
| `handle_remote_*` | 6 | `remote_command_handlers.rs` |
| `resolve_remote_*` | 5 | `remote_workspace_resolver.rs` |
| `remote_file_*` | 4 | `remote_file_io.rs` |
| `read_remote_*` | 3 | `remote_file_io.rs` |
| `remote_dialog_*` | 2 | `remote_command_handlers.rs` |
| `remote_workspace_*` | 2 | `remote_workspace_resolver.rs` |
| `remote_assistant_*` | 2 | `remote_models.rs` |
| `normalize_*` | 2 | `remote_models.rs` |
| 其他 misc | ~20 | 分散到上述 5 sibling |

### 1.3 关键结构/枚举/Trait（~30 个类型）

按 `pub struct/enum/trait` 拆分到对应 sibling:
- `RemoteImageContext`, `RemoteImageContextAdapter` → request_builders
- `RemoteCancelDecision`, `RemoteCancelTaskRequest`, `RemoteCancelRuntimeHost` → command_handlers
- `RemoteDialogQueuePriority`, `RemoteDialogSubmissionPolicy` → command_handlers
- `RemoteDialogSubmissionRequest`, `RemoteTerminalPrewarmRequest`, `RemoteDialogResolvedSubmission` → command_handlers
- `RemoteDialogSubmitOutcome`, `RemoteDialogSchedulerOutcomeFact`, `RemoteDialogRuntimeHost` → command_handlers
- `RemoteSessionTracker` struct + impl → session_tracker
- 其他 (session_info/list/created/deleted/model_updated DTOs) → session_tracker

### 1.4 Round 5/6/7/8/9/10 经验

| 错误类 | Round hit | R11 防御 |
|---|---|---|
| Cargo.lock drift (rmcp 1.7→1.8) | R6 | Plan YAML preflight baseline cargo check |
| cargo check stop-at-first-error | R6 (32+ errors 被 2 E0308 掩盖) | worker 报"0 NEW errors"必须每个 crate 都跑过 |
| M3 model 慢 (39min 沉默) | R6 | Plan YAML 强制 `model: minimax/MiniMax-M2.7-highspeed` |
| Plan engine abort fail | R6 (50001) | 用 `mavis team plan cancel` 不依赖 `mavis session abort` |
| Sibling method private | R6 | bulk `pub(super)` via Python script (R10a pattern 不需要 — multi-impl) |
| Struct field private | R6 | 跨 sibling 共享 struct 字段 `pub(crate)` |
| Import 路径 super::super vs super | R6 (4 处) | 新 sibling 默认 `use super::*` (siblings) 或 `use super::super::*` (parent) |
| 漏 test attribute | R9b | worker split 必须保留 `#[test]`/`#[tokio::test]` attribute |
| mod.rs 漏 `pub mod` | R3b (orphan files) | 每个新 sibling 必须在 mod.rs 加 `pub mod` |
| 测 0 lines 漂移 (Measure-Object vs wc -l) | R6 audit | 用 `[System.IO.File]::ReadAllLines().Count` |
| R10a 1130 unused imports | R10a | 每个 sibling 用精确 use 块, 不复制 |
| Mavis take-over 后 verifier FAIL | R10a | merge to main 后立即 check plan status |

---

## §2 拆分方案（sub-domain split，按 fn prefix 簇）

### §2.1 目标文件结构

```
src/crates/services/services-integrations/src/
├── lib.rs                          (unchanged, `pub mod remote_connect;` 自动指向 mod.rs)
└── remote_connect/
    ├── mod.rs                      NEW (sub-facade with `pub use` re-exports)
    ├── device.rs                   74 (unchanged)
    ├── encryption.rs               189 (unchanged)
    ├── pairing.rs                  282 (unchanged)
    ├── qr_generator.rs             82 (unchanged)
    ├── relay_client.rs             511 (unchanged)
    ├── remote_request_builders.rs  NEW ~400-500 (build_remote_* 7 fns + 4 struct/enum)
    ├── remote_session_tracker.rs   NEW ~600-700 (remote_session_* 6 fns + RemoteSessionTracker struct + impl)
    ├── remote_command_handlers.rs  NEW ~700-800 (handle_remote_* 6 fns + 11 struct/enum/trait + remote_dialog_* 2 fns)
    ├── remote_file_io.rs           NEW ~400-500 (read_remote_* 3 + remote_file_* 4 + remote_file_display_name)
    └── remote_workspace_resolver.rs NEW ~300-400 (resolve_remote_* 5 + path utilities)
```

### §2.2 目标行数

| File | 目标 | spec cap | 备注 |
|---|---|---|---|
| `mod.rs` (facade) | ~50-100 | 200 | `pub mod` 11 + `pub use` 12 重导出 |
| `device.rs` | 74 | (preserved) | unchanged |
| `encryption.rs` | 189 | (preserved) | unchanged |
| `pairing.rs` | 282 | (preserved) | unchanged |
| `qr_generator.rs` | 82 | (preserved) | unchanged |
| `relay_client.rs` | 511 | (preserved) | unchanged |
| `remote_request_builders.rs` | ~400-500 | 800 | 7 build fns + 4 type defs |
| `remote_session_tracker.rs` | ~600-700 | 800 | 6 session fns + RemoteSessionTracker struct + impl |
| `remote_command_handlers.rs` | ~700-800 | 800 | 6 handle fns + 11 types + 2 dialog fns |
| `remote_file_io.rs` | ~400-500 | 800 | 3 read + 4 file fns + path utilities |
| `remote_workspace_resolver.rs` | ~300-400 | 800 | 5 resolve fns + path utilities |
| **TOTAL** | ~3500-3700 | — | +50-200 (import/use 重复, pub use) |

### §2.3 mod.rs (sub-facade) 设计

```rust
//! Remote-connect integration contracts (Round 11 split)
//!
//! This module owns remote-connect wire assembly, runtime-port request
//! construction, compatibility re-exports, and remote session tracker state.
//!
//! Round 11 split: 5 sibling files own domain-specific fns by prefix cluster:
//! - request_builders (build_remote_*)
//! - session_tracker (remote_session_*)
//! - command_handlers (handle_remote_* + remote_dialog_*)
//! - file_io (read_remote_* + remote_file_*)
//! - workspace_resolver (resolve_remote_*)
//!
//! Pre-existing partial split (Round 5/6 era): device/encryption/pairing/
//! qr_generator/relay_client — unchanged.

pub mod device;
pub mod encryption;
pub mod pairing;
pub mod qr_generator;
pub mod relay_client;

pub mod remote_request_builders;
pub mod remote_session_tracker;
pub mod remote_command_handlers;
pub mod remote_file_io;
pub mod remote_workspace_resolver;

// Re-export existing public types (preserves external API)
pub use device::DeviceIdentity;
pub use encryption::{decrypt_from_base64, encrypt_to_base64, KeyPair};
pub use pairing::{PairingChallenge, PairingProtocol, PairingResponse, PairingState, QrPayload};
pub use qr_generator::QrGenerator;
pub use relay_client::{
    ensure_rustls_crypto_provider, ConnectionState, RelayClient, RelayEvent, RelayMessage,
};

// Re-export new sibling public types (preserve external API)
pub use remote_request_builders::*;  // build_remote_*, RemoteImageContext, etc.
pub use remote_session_tracker::*;   // RemoteSessionTracker, session fns
pub use remote_command_handlers::*;  // handle_remote_*, dialog fns + types
pub use remote_file_io::*;           // read_remote_*, remote_file_*
pub use remote_workspace_resolver::*; // resolve_remote_*

// Keep RemoteConnectSubmissionSource enum at mod.rs (used in many fn signatures)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteConnectSubmissionSource {
    Relay,
    Bot,
}
// ... impl same as before
```

### §2.4 子 sibling 文件模板

每个新 sibling 文件结构（按 R5 chat sub-domain pattern）:

```rust
//! Remote-connect {domain} (Round 11 split)
//!
//! Owns {domain} fns/structs extracted from remote_connect.rs.
//! sub-domain split per fn prefix: {prefix_list}.

use super::RemoteConnectSubmissionSource;  // 共享类型 from mod.rs
// ... 各自需要的 import

// pub fn / pub struct / pub enum / pub trait / impl blocks
pub fn build_remote_session_create_request(...) -> ... { ... }
pub fn build_remote_submission_request(...) -> ... { ... }
// ... 其他 fns
```

**关键**: 没有 god struct（如 R10a PersistenceManager），所以 **不需要 multi-impl pattern**。每个 sibling 是 free functions + types，与 R5 chat sub-domain split 模式一致。

### §2.5 lib.rs 改动

无需改动：`pub mod remote_connect;` 已存在，rustc 自动从 file 转到 mod/ 子目录 + mod.rs。

---

## §3 验证策略

### §3.1 编译验证

```bash
# 1. baseline 重现 (preflight)
cd E:\agent-project\northing
git log -1 --oneline  # 确认 2882a74 (R10b merge)
cargo check -p northhing-services-integrations --features product-full  # 期望 0 errors

# 2. 改完后
cargo check -p northhing-services-integrations --features product-full  # 期望 0 errors
cargo build --tests -p northhing-services-integrations --features product-full  # 期望 0 errors

# 3. 全 workspace（防止 cross-crate regression）
cargo check --workspace  # 期望 0 errors
```

### §3.2 测试验证

```bash
cargo test -p northhing-services-integrations --features product-full --lib
# 期望: 899 passed; 0 failed; 1 ignored (与 main HEAD baseline 一致)
```

### §3.3 cross-crate public API check

```bash
# 确保 external 调用方仍能解析 RemoteConnectSubmissionSource, RemoteSessionTracker, etc.
git grep -l 'use northhing_services_integrations::remote_connect::' | wc -l
# baseline count + after count 必须相等 (no missing imports in callers)
```

---

## §4 D-deviation 风险

| Item | Plan 接受 | 实际预期 | 备注 |
|---|---|---|---|
| command_handlers 800 cap | 上限 810 | ~700-800 | 11 types + 6 handle + 2 dialog fns 估计偏高但 < 800 |
| session_tracker 800 cap | 上限 810 | ~600-700 | RemoteSessionTracker struct + impl + 6 fns |
| 其他 3 sibling | < 800 | OK | 比 cap 宽裕 |
| mod.rs sub-facade 200 cap | 上限 210 | ~50-100 | pub use 重导出 12 + 5 pub mod |

---

## §5 实施步骤

### Phase 1: 准备
1. 创建 worktree `northing-impl-round11`
2. 跑 baseline `cargo check -p services-integrations` 记录干净状态
3. 写 helper Python 脚本（fn 分类器 / import 迁移器）

### Phase 2: 拆文件 (atomic per sibling)
1. 拆 `remote_request_builders.rs` (~400) → cargo check
2. 拆 `remote_workspace_resolver.rs` (~400) → cargo check
3. 拆 `remote_file_io.rs` (~500) → cargo check
4. 拆 `remote_session_tracker.rs` (~700) → cargo check
5. 拆 `remote_command_handlers.rs` (~800) → cargo check + cargo test
6. 创建 `mod.rs` (sub-facade) → cargo check + cargo test
7. 删除原 `remote_connect.rs` (line 1-3446) → cargo check + cargo test

### Phase 3: 验证
1. cargo fmt --check clean
2. cargo test --lib 899/0/1 baseline match
3. cross-crate public API 不变 (use northhing_services_integrations::remote_connect::* 全部 OK)
4. diff stat: 1 deleted (3446) + 5 created (~2800) + 1 created mod.rs (~100)

### Phase 4: commit + merge
1. 1 atomic commit per R5/6/7/8/10a D6 precedent
2. 写 handoff doc
3. merge to main
4. (Mavis take-over self review-fix-cleanup if needed)

---

## §6 spec review check-list

QClaw 重点检查:
1. **5 个新 sibling 按 prefix 簇划分合理** — request_builders / session_tracker / command_handlers / file_io / workspace_resolver
2. **mod.rs sub-facade pub use 重导出完整** — 保留 external API
3. **D-deviation 1 项 (command_handlers ~800)** — 是否需要 R11b 二次拆
4. **0 fn dropped** — cross-check 59 fns preserved
5. **cross-crate 调用方不受影响** — services-integrations public API 不变

---

## §7 Errata

- §2.4 没有 multi-impl pattern（free functions 不是 god struct），按 R5 chat sub-domain split 模式
- §5 Phase 2 拆文件顺序按 fn 数从小到大
- §2.5 lib.rs 无需改动（rustc 自动处理 file → mod/ 转换）
- §4 D-deviation 跟 R9b/R10a 同 reviewer-tolerance