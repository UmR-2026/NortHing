# Task Brief T2-2f — remote 栈子批 C4：contracts 修剪（remote_connect-era 契约删除）

Roadmap: `docs/architecture/backend-roadmap.md` T2-2（remote 栈整删 TH-4，2026-08-17 拍板删除）。批次划分：`.superpowers/sdd/task-t2-2c-recon.md` §C4。前置：C1（fa88342）/ C2（02c6520）/ C3（0bc8d81）已并入 main，这些契约的生产者（core remote_connect 主机注册、services-integrations remote_connect 模块）已全部不存在。

## ⚠️ 契约层删除授权（wire 变更）

本批删除序列化词汇：`SurfaceKind::Remote`（"remote"）、`ThreadEnvironmentKind::RemoteConnect`（"remote_connect"）、`RuntimeServiceCapability::{RemoteConnection,RemoteWorkspace,RemoteProjection,RemoteCapabilities}`（4 个 snake_case 字符串）。计划层已拍板 remote 栈整删，本 brief 逐一枚举 = 显式授权。**仅限下列清单，其余 contracts 一字不动。**

## 已核实事实（编排者 2026-08-19 亲验）

- `runtime-ports/src/remote.rs`（143 行）全部符号的生产消费方 = **零**；仅 runtime-services registry 字段 + test_support fake + 两个测试文件引用。
- `with_optional_remote_*` 四个 builder 方法生产调用方 = **零**。
- `RuntimeServiceCapability::Remote*` 外部引用仅 `assembly/core/tests/product_assembly.rs:30-31` 与 `runtime-services/tests/runtime_services_contracts.rs`（测试断言 fake 全能力注册表含 remote）。
- `SurfaceKind::Remote` 外部引用仅 `core-types/tests/surface_contracts.rs`；`ThreadEnvironmentKind::RemoteConnect` 仅同文件 :64,73。
- `product_assembly.rs` 的 remote 断言靠 `FakeRuntimeServicesProvider::with_all_required()` 通过；真实 `CoreRuntimeServicesProvider` 已无 remote 注册（C1 摘除）。

## Files（精确清单）

### contracts/runtime-ports
1. **整删** `src/remote.rs`（143 行：RemoteWorkspaceKind / RemoteWorkspaceFacts / RemoteRecentWorkspaceFacts / RemoteAssistantWorkspaceFacts / RemoteWorkspaceUpdate / RemoteSessionMetadata / RemoteWorkspaceFileContent / RemoteWorkspaceFileChunk / RemoteWorkspaceFileInfo / RemoteFileChunkRange / RemoteWorkspaceRuntimeHost / RemoteWorkspacePort / RemoteInitialSyncRuntimeHost / RemoteWorkspaceFileRuntimeHost / RemoteProjectionPort / RemoteCapabilityPort）。
2. `src/lib.rs`：删 `pub mod remote;`（:16 附近）与 `pub use remote::*;`（:31 附近）+ 文件头 doc 注释 :9 的 "remote" 提及。
3. `src/session_workspace.rs:544`：删 `pub trait RemoteConnectionPort: RuntimeServicePort {}` 及其 doc 注释块。
4. `src/port_core.rs`：删 4 个变体 `RemoteConnection/RemoteWorkspace/RemoteProjection/RemoteCapabilities`（:58-61）与对应 match 臂（:77-80 字符串 "remote_connection"/"remote_workspace"/"remote_projection"/"remote_capabilities"）。
5. `src/runtime_facade_tests.rs`：删引用 remote.rs 符号的测试段（保留其余）。

### contracts/core-types
6. `src/surface.rs`：删 `SurfaceKind::Remote`（:16）与 `ThreadEnvironmentKind::RemoteConnect`（:27）。**`RemoteSsh` 变体与 `ThreadEnvironment.remote_connection_id` 字段保留（SSH 语义）。**
7. `tests/surface_contracts.rs`：删 RemoteConnect/Remote 相关断言段（:64,73 附近），保留其余。

### execution/runtime-services
8. `src/lib.rs`：删 4 个 registry 字段（:45-48）+ capability match 臂（:97-100）+ builder 字段（:132-135）+ 4 个 `with_optional_remote_*`（:193-209）+ build() 内 4 段（:225-235）+ import（:8 的 4 个 remote 符号）。
9. `src/test_support.rs`：删 remote port impl 与字段（:5 import、:46 impl、:156 构造等——以实际为准），FakeRuntimeServicesProvider 不再提供 remote 能力。
10. `tests/runtime_services_contracts.rs`：删 remote 相关断言。

### assembly/core
11. `tests/product_assembly.rs`：删 :30-31 两条 remote capability 断言（两处 test 函数都要查）。

## Constraints（逐字自计划 Global Constraints）

1. **清单外 contracts 零改动**：`DialogTriggerSource::{RemoteRelay,Bot}` 保留；`RemoteSsh` 变体、`remote_connection_id` 字段、`session_workspace.rs` 其余全部保留。
2. SSH 语义零改动（remote_ssh 模块、lookup_remote_connection*、RemoteWorkspaceEntry 等）。
3. 不顺手重构；不动 memory/、.opencode/、.superpowers/sdd/ 其它 task-*、前端；不 commit 不 push。
4. boundary 规则若有 remote.rs/port_core 变体锚点（查 self-test.mjs / required-rules.mjs），同 commit 同步并跑绿 `node scripts/check-core-boundaries.mjs`。

## Verification（MSVC rustup wrapper，原始输出贴报告）

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
node scripts/check-core-boundaries.mjs
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-runtime-ports
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core-types
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-runtime-services
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full --test product_assembly
# 归零（以下应只在注释/文档命中，逐条解释）：
rg -n "RemoteConnectionPort|RemoteWorkspacePort|RemoteProjectionPort|RemoteCapabilityPort|RemoteWorkspaceKind|RemoteInitialSyncRuntimeHost|RemoteWorkspaceFileRuntimeHost|RemoteWorkspaceRuntimeHost" src --glob "*.rs"
rg -n "SurfaceKind::Remote\b|ThreadEnvironmentKind::RemoteConnect|RuntimeServiceCapability::Remote" src --glob "*.rs"
```

## Report

写 `.superpowers/sdd/task-t2-2f-report.md`：status、逐文件操作清单、每处 wire 词汇删除的确认、验证原始输出、遗留疑虑。假汇报 = 停用。
