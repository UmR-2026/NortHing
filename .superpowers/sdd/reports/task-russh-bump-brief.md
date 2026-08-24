# Task: russh 0.45 → 0.62.7 大版本迁移（RUSTSEC-2026-0089 修复）

## 1. 来源与验收标准

来源：cargo audit（2026-08-23 实跑，`.superpowers/sdd/reports/cargo-audit-2026-08-23.txt`）发现：

```
Crate:     russh
Version:   0.45.2
Title:     Missing strict kex in russh (CVE-2026-41406 / RUSTSEC-2026-0089)
Solution:  Upgrade to >=0.60.3
```

用户已于 2026-08-23 拍板立项升级。验收标准（逐条机械可核对）：

1. workspace Cargo.toml 中 russh 钉到 0.62.7（当前在根 `Cargo.toml` 第 214-216 行附近：`russh = "0.45"` / `russh-keys = "0.45"` / `russh-sftp = "2.1"`）。
2. `cargo check -p northhing-services-integrations` 通过。
3. `cargo check -p northhing` 通过（桌面编译门）。
4. MSVC 工具链实跑 `remote_ssh` 相关测试全绿（命令见 §6，输出原文进 report）。
5. 升级后 `cargo audit` 不再报告 russh 的 RUSTSEC-2026-0089。

## 2. 编排者预检结论（直接采信，勿重复侦察）

| 事实 | 证据 |
|---|---|
| russh 使用面集中在 `src/crates/services/services-integrations/src/remote_ssh/`（含 `remote_exec/` 子目录），约 10 个文件；workspace 其它 crate 不直接用 russh | rg 全仓 |
| 我方是**纯 client** 使用：`client::{Config, Handler, Handle, Msg, DisconnectReason, connect_stream}`、`Channel<Msg>`、`ChannelMsg::{Data, ExtendedData, Eof, Close, ExitStatus, ExitSignal}`、`Sig`、`Preferred`、`kex::*` 常量、`Channel::open_reverse`（manager_port_forward.rs） | rg 逐行 |
| `russh_keys` 使用：`key::{PublicKey, KeyPair}`、算法常量 `ED25519 / ECDSA_SHA2_NISTP256 / ECDSA_SHA2_NISTP521 / RSA_SHA2_256 / RSA_SHA2_512 / SSH_RSA`、`decode_secret_key`（mgr_lifecycle_handlers.rs:81） | rg 逐行 |
| `russh_sftp` 使用：`client::SftpSession`、`client::fs::{ReadDir, Metadata}`、`SftpSession::new(channel.into_stream())`（manager_sftp.rs:93） | rg 逐行 |
| client Handler 只实现了 `check_server_key(&PublicKey) -> Result<bool, Error>`（manager_handler.rs:129）；**没有**任何 `channel_open_*` Handler 实现 | rg |
| 0.62.0 的 `channel_open_*` Handler 签名 break（ChannelOpenHandle）是 server 侧 API，**不命中我们** | russh v0.62.0 release notes + 上一条 |
| **0.63.0 改了 `check_server_key` 签名**（`PublicKeyOrCertificate` enum），故目标钉死 0.62.7，不上 0.63.x | russh v0.63.0 release notes |
| russh 0.60 起密钥类型迁移到上游 `ssh-key` crate，`russh-keys` 被吸收；`russh::keys` 模块提供新类型 | russh release notes |
| russh-sftp 最新 2.4.0（2026-08-03），其 dev-dep 钉 `russh ^0.62.5` —— 与目标 0.62.7 同代，配套兼容 | crates.io API |
| russh 最新稳定 0.62.7（2026-08-17）；0.62.4/0.62.5/0.62.6 还附带 3+ 个额外安全修复 | crates.io / releases |
| 现有测试：`remote_ssh/manager_tests.rs`、`manager.rs`、`password_vault.rs` 内有 `#[cfg(test)]` | rg |
| **无真 SSH 服务器可做回归**：验证上限 = cargo check + 单测 + audit，行为等价只在 API 层面 | 项目现状 |

## 3. 复用侦察（强制）

- 动手前先确认：russh 0.62.7 的 `russh::keys` 是否覆盖我们全部 `russh_keys` 用法（PublicKey / 私钥类型 / decode_secret_key / 算法常量）。若覆盖 → **删掉 `russh-keys` 依赖**；若不覆盖 → 保留并升到与 russh 0.62 兼容的版本，report 里给理由。
- report 必须有「复用侦察」一节：查了哪些符号、复用了什么（`russh::keys` vs 保留 `russh-keys`）、判断依据。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. 根 `Cargo.toml`：`russh = "0.62.7"`、`russh-sftp = "2.4.0"`；`russh-keys` 按 §3 结论删或升。`Cargo.lock` 一并更新进 commit。
2. `services-integrations/src/remote_ssh/` 全部编译错误修完。迁移手法：**compiler-driven**——先 bump 版本跑一次 `cargo check`，按错误清单逐条适配。
3. 除 API 适配外**不改任何行为**：不重排 kex/算法优先级顺序、不改超时/窗口/重连逻辑、不动 SSHHandler 语义。
4. **兼容性缺口必须上报而非默默丢弃**：若新版 russh 移除了我们现役清单中的算法（如 `DH_G14_SHA1` / `DH_G1_SHA1` / `SSH_RSA` 等 legacy 项）或常量改名后有语义差异，列在 report「兼容缺口」节并整体状态给 DONE_WITH_CONCERNS。
5. `russh-keys` 去留是授权判断点：implementer 按 §3 证据自决，report 写结论 + 一行理由。

## 5. Global Constraints（逐字遵守）

- GNU 工具链只能 check 不能 link 可执行产物（aws-lc-sys `nanosleep64` undefined）；跑测试必须用 `rustup run stable-x86_64-pc-windows-msvc cargo test ...`。
- 重链/跑测试前确认无残留 northhing/northhing-cli 进程占着 exe（os error 5 事故）。
- **禁 `git add -A` / `git add .`**；只 add 本任务改动文件（根 Cargo.toml、Cargo.lock、remote_ssh 下改动文件）。
- 测试代码里禁新增 `.unwrap()`（rot 棘轮 grep 计数不分 test 代码）；测试里用 `unwrap_or_default()`。存量测试的 unwrap 不在本任务清理范围。
- 禁碰以下在途文件（其它 session 持有）：`.opencode/model-capability-notes.md`、`.superpowers/sdd/progress.md`、`memory/northhing.md`、`src/crates/contracts/kernel-api/src/memory.rs`、`src/crates/contracts/kernel-api/src/turn.rs`。
- 单个 conventional commit 落 main，message 形如 `chore(deps): bump russh 0.45 -> 0.62.7 (RUSTSEC-2026-0089)`。

## 6. 验证（命令 + 输出原文全部进 report）

```powershell
# 1. crate 编译门
cargo check -p northhing-services-integrations 2>&1

# 2. 桌面编译门
cargo check -p northhing 2>&1

# 3. MSVC 实跑 remote_ssh 测试（真跑，不是 check）
rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-services-integrations remote_ssh 2>&1

# 4. audit 复验（输出中 russh 相关段落贴原文；若 audit 整体输出无 russh 条目，贴 "russh" grep 为空的结果）
cargo audit 2>&1 | Select-String -Pattern "russh" -Context 3,6
```

## 7. 报告

- 路径：`.superpowers/sdd/reports/task-russh-bump-report.md`
- 必含节：改动清单 / 复用侦察（§3）/ 兼容缺口（无则写"无"）/ 遇到的编译错误及修复层（机制层/设计层，一行一个，见文末 Rust 约定）/ §6 四条命令的输出原文。
- 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。

## 8. 派发元信息

- BASE commit：`d95e96e`（docs(handoff): 2026-08-23 E2E keyring round）
- 禁区文件：见 §5。
- commit 规则：单 commit，只含本任务文件，禁 `git add -A`。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
