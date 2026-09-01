# Task Brief — Audit I1: provider 编辑路径 keyring 读失败吞错 + 错误消息误导

## 1. 来源与验收标准（逐字 + 编排者实证修正）

来源：`.superpowers/sdd/reviews/project-audit-20260826/r1-desktop.md` Finding 1（Important）。

**编排者实证修正（2026-08-26 @ fb98a77，逐行 trace 后结论，直接采信）**：审计声称的"keyring 读失败 → `Some("")` 进 upsert → 抹掉用户 key"**不成立**——`validate_provider_input`（`settings/sync.rs:58-60`）拒空 key，保存必被拒绝，keyring 与 core memory 均不可能被抹。真实残余缺陷只有两条：

1. `PRODUCTION_KEYRING.get(&pid).ok()` 吞掉 keyring 错误，无日志（fail-open 观测盲区）。
2. 用户看到的拒绝消息是"API Key 不能为空"（误导——暗示用户输入有误，实为密钥库读取失败）。

审计原文 fix 方向（仍有效，即为本任务）："propagate the keyring error and refuse the save"。

验收标准（逐条可机械核对）：

1. 编辑路径（已有 provider + key 字段留空）keyring `get` 返回 `Err` 时：有 `tracing::warn!` 日志（英文、不含 secret）、UI 拒绝保存、消息不再归咎于用户输入。
2. 该错误臂有单元测试覆盖（审计原话："No test covers this error arm"）。
3. 其余路径行为不变：编辑+已存 key 正常继承；新建/已输入 key 走原逻辑；`resolve_effective_api_key` 及其 4 个现有测试不动。
4. `cargo check -p northhing` 与聚焦测试全绿，输出原文进 report。

## 2. 编排者预检结论（直接采信，勿重复侦察）

| 事实 | 锚点 |
|---|---|
| 吞错点：`resolve_effective_api_key(PRODUCTION_KEYRING.get(&pid).ok().as_deref(), &pkey)` | `src/apps/desktop/src/app_state/callbacks_settings/provider.rs:121-122` |
| 该分支进入条件：`!id.is_empty() && pkey.trim().is_empty()`（编辑+留空） | `provider.rs:121` |
| 拒空 gate：`api_key.trim().is_empty() → Err("API Key 不能为空")` | `src/apps/desktop/src/app_state/settings/sync.rs:58-60`；测试 `settings/tests.rs:281` |
| `resolve_effective_api_key(stored: Option<&str>, incoming: &str) -> String`（留空取 stored，否则取 incoming） | `sync.rs:5-11`；测试 `settings/tests.rs:321-340` |
| `KeyringBackend::get(&self, account: &str) -> Result<String>`（anyhow）；条目缺失与读取失败同为 `Err`，类型层面不可区分 | `settings/keyring.rs:95, 119-126, 187-193` |
| keyring store 失败已有正确范式可抄：`tracing::warn!` + `set_inline_error(ui_weak.clone(), "密钥存储失败，请重试")` + return | `provider.rs:134-139` |
| 另一 keyring 消费点 `push_resolved_keys_to_core` 已是 fail-safe（Err 跳过、非空才 upsert），本任务不动 | `sync.rs:29-44` |
| UI 文案现状 = 硬编码中文（v0.1.0，i18n frozen），新增中文 UI 消息合规；**日志必须英文无 emoji** | 根 AGENTS.md i18n/Logging 节 |
| `set_inline_error(ui_weak.clone(), msg)` 签名 | `provider.rs:128, 137, 163` |

## 3. 复用侦察（强制）

动手前用 codegraph_explore 或 rg 查：`resolve_effective_api_key` / `validate_provider_input` / `KeyringBackend::get` 的现有调用与测试；report 必须有「复用侦察」一节（查了哪些符号、复用了什么、若新写等价物逐条给理由）。无此节 = 未完成。

## 4. Spec（必须全部满足）

1. **`sync.rs` 新增纯函数**（与 `resolve_effective_api_key` 同文件同风格）：

```rust
/// Edit-flow key resolution (P1-2 fail-closed): `stored` is the raw keyring
/// read result. Blank incoming key inherits the stored one; a keyring error
/// propagates so the caller refuses the save instead of swallowing it.
pub fn resolve_edit_api_key(stored: anyhow::Result<String>, incoming: &str) -> anyhow::Result<String>
```

   语义：incoming  trim 后为空 → 原样返回 `stored`（含 Err 传播）；非空 → `Ok(incoming.to_string())`。

2. **`provider.rs` 编辑分支改造**（:121-125）：`PRODUCTION_KEYRING.get(&pid)` 的结果进 `resolve_edit_api_key`，`Err(e)` 臂：
   - `tracing::warn!(target: "app_state", "keyring read failed for provider {pid}: {e}")`（不含 secret）；
   - `set_inline_error(ui_weak.clone(), "读取密钥库失败，请重试；如持续失败请重新输入 API Key".to_string())`；
   - `return`（拒绝保存）。
   `Ok(key)` 臂与 else 分支（新建/已输入）行为不变。分支进入条件 `!id.is_empty() && pkey.trim().is_empty()` 不变。`validate_provider_input` gate 保留（纵深防御第二道）。

3. **测试**（`settings/tests.rs`，仿 `resolve_effective_api_key` 四个现有测试风格）：
   - `Err` + 留空 → 返回 `Err`（错误臂覆盖，审计明确要求）；
   - `Ok(stored)` + 留空 → `Ok(stored)`；
   - `Err` + 非空 incoming → `Ok(incoming)`（"用户已输入新 key 时 keyring 故障不挡保存"语义）；
   - `Ok(_)` + 非空 incoming → `Ok(incoming)`。

4. **report 必须含「审计论断核实」一节**：用代码证据复述为什么"抹 key"不成立（validate gate），并说明本修复实际闭合的残余缺陷。

判断点（已授权，不许上报）：测试函数命名按现有文件惯例；其余不许自由发挥。

## 5. Global Constraints（逐字遵守）

- 禁止改动 `resolve_effective_api_key`、`validate_provider_input`、`push_resolved_keys_to_core` 及其现有测试。
- 禁止给 `KeyringBackend` trait 加类型化错误（anyhow 现状维持——不为本任务造错误分类机制）。
- 日志只许英文、无 emoji；UI 文案硬编码中文合规（i18n frozen）。
- 本任务不涉并发原语 —— 家规 4 不适用。
- provider.rs / sync.rs / tests.rs 均远低于 800 行，不许借机扩文件。
- Windows 环境：写非 ASCII 一律用 edit 工具，禁用 PowerShell Set-Content（GBK 双重编码事故史）。
- 免费池铁律：假汇报 = 停用；编排者将 diff 逐条核对；验证输出必须贴原文进 report。

## 6. 验证（命令 + 输出原文都要进 report）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cargo check -p northhing
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib settings
```

report 里每条命令贴完整输出尾部（含 test result 行）。report 无输出原文 = 假汇报嫌疑。

## 7. 报告

写入 `E:\agent-project\northing\.superpowers\sdd\audit-fix-i1-provider-keyring-report.md`：实现内容 / 复用侦察节 / 审计论断核实节 / 每个编译错误最终修在哪一层（机制层/设计层，一行一个）/ 测试与输出原文 / 文件清单 / 自审发现 / 疑虑。

最终回复只含（≤15 行）：Status（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit 短 SHA + subject、一行测试摘要、疑虑、report 路径。

## 8. 派发元信息

- BASE commit：`fb98a77`（派发前 HEAD）
- 禁区文件：`settings/keyring.rs`（trait 定义不动）、`callbacks_settings/` 下除 `provider.rs` 外的文件、`sync.rs` 中除新增函数外的现有代码
- commit 规则：conventional commits（如 `fix(desktop): ...`），不加 AI 署名/co-author
- 工作目录：`E:\agent-project\northing`，直接在 main 工作（本会话既定流程）

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
