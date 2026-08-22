# Task Brief T2-2e — remote 栈子批 C3：services-integrations remote_connect 整删

Roadmap: `docs/architecture/backend-roadmap.md` T2-2（remote 栈整删 TH-4）。批次划分见 `.superpowers/sdd/task-t2-2c-recon.md` §建议删除批次划分。前置：C1（core 摘除，fa88342）✅、C2（agentic remote_file_delivery 链，02c6520）✅。

## Goal

删除 `services-integrations` 的 `remote_connect` 模块全链（4,081 行生产 + 测试 + feature + orphan deps + boundary 规则锚点），SSH（`remote_ssh`/`remote-ssh` feature）与共享依赖零损伤。

## 已核实事实（编排者 2026-08-19 亲验，勿重复侦察，但须在报告中复核确认）

- 模块外消费方 = **仅本 crate tests**（`tests/common/mod.rs:4` re-export 块，~60 符号）。core 的 `service/remote_ssh/*` 引用的是 `services_integrations::remote_ssh`（不同模块，保留）。`services_integrations::remote_connect` 全仓外部引用 = 0。
- `remote-connect` feature 在 core/product-capabilities/根 Cargo.toml 零引用（C1 已清）。
- services-integrations AGENTS.md / services AGENTS.md / surfaces.md 中 `remote_connect|remote-connect` 零提及（若你发现有，同 commit 同步）。
- 测试文件**非整删**（recon 说"tests 7 文件"已过时，实测 8 文件全是混合内容）：
  - `tests/pairing_qr_relay.rs`（58 行）：整文件 `#![cfg(feature = "remote-connect")]` 门控 → **整文件删**。
  - `tests/common/mod.rs`（577 行）：删 `:4` 起的 `pub use northhing_services_integrations::remote_connect::{...}` 整块 re-export + 文件内其余 remote 专属 helper；保留 SSH/其它共享 helper。
  - 其余 7 个 tests/*.rs（command_runtime / dialog_cancel_contracts / file_transfer / model_catalog_tracker_poll / remote_ssh_contracts / session_wire_and_responses / submission_images）：**逐个测试函数判定**——凡 `use` 或调用 remote_connect 符号（经 common re-export）的测试函数删除；纯 SSH/非 remote 测试保留。`remote_ssh_contracts.rs` 零 `common::` 引用，大概率整体保留，仍需逐行核。
  - 判定尺：删完后 `cargo test -p northhing-services-integrations --features product-full` 编译+全绿，且 `rg "remote_connect|RemoteConnect" tests/` 归零。

## Files（删除/修改清单）

1. 整删目录 `src/crates/services/services-integrations/src/remote_connect/`（14 文件，4,081 行）。
2. `src/crates/services/services-integrations/src/lib.rs:31` 删 `pub mod remote_connect;`。
3. `src/crates/services/services-integrations/Cargo.toml`：
   - 删 `remote-connect = [...]` feature 块（:100-121）与 product-full 内 `"remote-connect"` 引用（:157）。
   - orphan optional deps 清理，owner 表依据 `scripts/core-boundaries/rules/feature-rules.mjs:46-88`：
     - **可删**（唯一 owner = remote-connect）：`hostname` `image` `mac_address` `qrcode` `rustls` `rustls-native-certs` `schannel` `tokio-tungstenite` `urlencoding` `x25519-dalek`。
     - **保留**（共享 owner）：`aes-gcm`（mcp/remote-ssh-concrete）、`anyhow`、`base64`（mcp/miniapp-runtime/remote-ssh-concrete）、`chrono`（git/remote-ssh-concrete）、`futures`（mcp）、`rand`、`sha2`（remote-ssh）、`tokio-util`（remote-ssh）、`uuid`（miniapp-runtime/remote-ssh-concrete）。
     - `northhing-runtime-ports`（:53 owner 仅 remote-connect）：先查 crate 内其它模块是否还有 `use northhing_runtime_ports`（如 remote_ssh），有则改为非 optional 或保留在剩余 feature，无则删。**以实测为准，别照单全收。**
   - 每个 dep 删除前 `rg` 本 crate src 确认零残留 import。
4. `tests/common/mod.rs` + 上列 tests 文件按上述判定尺处理。
5. `Cargo.lock` 同步（`cargo check` 会自动；确认 lock 中 orphan 包消失，如未消失说明还有拉取方，报告之）。
6. Boundary 规则同步（本批内同 commit，家规 2）：
   - `scripts/core-boundaries/rules/feature-rules.mjs:46-88`：从各 ownerFeatures 数组移除 `'remote-connect'`；orphan dep 整条目删除；`:153` 附近 known-features 列表移除 `'remote-connect'`。
   - `scripts/core-boundaries/rules/crate-rules.mjs:188,208,216` 附近：services-integrations allowed-deps 中删除已删 orphan dep 条目。
   - `scripts/core-boundaries/self-test.mjs:1691` 起 remote_connect fixture 块整段删除。
   - **保留**：`self-test.mjs:546,575` 的 `coreFullyMigratedDeps`（qrcode/x25519-dalek 等断言 core 不依赖它们——方向相反，仍然有效）；`:2179` SSH contracts 锚点。
   - 改完必跑 `node scripts/check-core-boundaries.mjs` + `node scripts/core-boundaries/self-test.mjs`（若后者可独立运行）确认绿。

## Constraints（逐字自计划 Global Constraints）

1. SSH 语义零改动：`remote_ssh/` 模块、`remote-ssh`/`remote-ssh-concrete` feature、`remote_connection_id`、`lookup_remote_connection*`、`RemoteWorkspaceEntry` 等一行不动。
2. contracts 层零改动（C4 才修剪）。
3. 不顺手重构；tests 只删 remote 专属函数，不"顺手清理"其它测试。
4. 不动 `memory/`、`.opencode/`、`.superpowers/sdd/` 其它 task-* 文件、前端文件；工作区有其它并行 session 改动，不 commit 不还原。
5. 不 commit、不 push。

## Verification（MSVC rustup wrapper，原始输出贴报告）

```powershell
$cargo = { param($a) & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo @a }
# 1. 主门
& $cargo check --workspace
& $cargo check -p northhing
# 2. SSH feature 自足性（删 dep 后必验）
& $cargo check -p northhing-services-integrations --features remote-ssh,remote-ssh-concrete
& $cargo check -p northhing-services-integrations   # default features
# 3. boundary
node scripts/check-core-boundaries.mjs
# 4. 测试
& $cargo test -p northhing-services-integrations --features product-full
# 5. 归零（须 0 命中）
rg -n "remote_connect|RemoteConnect" src --glob "*.rs"
rg -n "remote-connect" src scripts --glob "*.toml" --glob "*.mjs"
```

（第 5 组中 `self-test.mjs:546,575` 的 qrcode/x25519 字符串与 remote_ssh 语义命中除外，报告里逐条解释残留。）

## Report

写 `.superpowers/sdd/task-t2-2e-report.md`：status（DONE/DONE_WITH_CONCERNS/BLOCKED）、逐文件操作清单、每个 orphan dep 的处置与依据、测试函数删/留判定表、验证原始输出、遗留疑虑。假汇报 = 停用。
