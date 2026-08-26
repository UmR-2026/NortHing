# Task P3a — F6 Onboarding 三步校验流 + 完成副作用接线

来源：`.superpowers/sdd/consult-room/prescription-v3-20260825.md` §F6。依赖 P0a/P1b 均已完成。
文件：`ui_dioxus/pages_onboarding.rs` 就地改（**不建 onboarding_state.rs**）；api.rs 加薄包装。

## 现状 bug（已核实）

- L462-480：Step2「测试」按钮是假的——onclick 直接设成功文案 + `tested_connection=true`，零内核调用。
- L536-542：底部完成按钮只校验色板，然后直接 `ritual_completed.set(true)`；无 provider 测试门、无 workspace 存在性校验、无任何持久化副作用。

## 改动

### ① 步骤状态机（pages_onboarding.rs）

```rust
#[derive(Clone, Copy, PartialEq)]
enum Step { One, Two, Three }
let mut current_step = use_signal(|| Step::One);
```

底栏按钮改为分步驱动（现有三张卡保持常显可滚动填写，DOM 结构不动；按钮文案随步切换：「下一步 · 身份」→「下一步 · 管道」→「完成仪式」，中文硬编码与本文件既有风格一致）：

| 当前步 | 点击行为 |
|---|---|
| One | 校验：`selected_palette().is_some()` **且** `!agent_input.read().trim().is_empty()` → 过则 `current_step.set(Two)`；不过则 `room_state_hint.set(Some(原因))`（沿用 L538 的提示模式） |
| Two | `spawn` 调 `api::test_provider_config(form)`（见②）：Ok(success=true) → `tested_connection.set(true)` + 成功文案 + `current_step.set(Three)`；success=false 或 Err → `test_status_text` 显示错误首行、**停留本步**。等待期间按钮禁用防重复点击（新增局部 `testing: Signal<bool>` 即可） |
| Three | 校验 `std::path::Path::new(workspace_dir_input.read().trim()).exists()` → 过则执行③副作用再 `ritual_completed.set(true)`；不过则 hint 提示目录不存在 |

表单 DTO 组装（Step::Two，字段名逐字对齐 kernel-api/src/settings.rs:119-131）：

```rust
ProviderFormDto {
    provider_id: "onboarding".into(),          // 表单态临时 id，处方明文用 form 态测试
    base_url: Some(provider_url_input.read().trim().to_string()),
    api_key: Some(provider_key_input.read().clone()),
    model: Some(provider_model_input.read().trim().to_string()),
    provider_type: None,                        // 缺省回落 provider_id 兼容路径
}
```

### ② api.rs 薄包装（与既有函数同风格）

```rust
pub async fn test_provider_config(
    form: northhing_kernel_api::settings::ProviderFormDto,
) -> Result<ProviderTestResultDto, KernelError> {
    kernel_facade().test_provider_config(form).await
}

pub async fn store_provider_api_key(provider_id: &str, plaintext: &str) -> anyhow::Result<String> {
    super::super::app_state::settings::store_api_key(
        &*super::super::app_state::settings::PRODUCTION_KEYRING, provider_id, plaintext)
}
```
（use 路径以文件现状整理；`store_api_key`/`PRODUCTION_KEYRING` 已由 `app_state/settings/mod.rs` `pub use keyring::*` 导出，同 crate 直调合规。）

### ③ 完成副作用（Step::Three 通过后，spawn 内顺序执行）

1. `api::store_provider_api_key("onboarding", &provider_key_input.read())`
   —— **fail-closed**：Err 则 hint 报错并中止（绝不落盘明文，C3 家规）。key 为空串时该函数本身是 no-op Ok，无需特判。
2. `crate::app_state::settings::update_app_settings(|s| { s.onboarding_completed = true; s.add_workspace(PathBuf::from(ws)); Ok(()) })`
   —— Err 则 hint 报错并中止。
3. `kernel_facade().create_session(SessionConfigDto { workspace_path: Some(ws), agent_type: "agentic".into(), model_name: "default".into(), name: Some(display_agent_name.clone()) })`
   —— **best-effort**：Err 仅 `tracing::warn!`，不阻断完成状态（room 发消息时 `ensure_room_session` 会兜底重建，与 P0b 语义一致）。
   全部走完后 `ritual_completed.set(true)`；已完成后按钮进入既有 L525-531 完成态分支，无重复提交路径。

### ④ 可测性接缝（页内纯函数）

把三步门控抽成页内纯函数并各配 1 条单测（不需要 Dioxus runtime）：
`fn step_gate(step: Step, palette_ok: bool, agent_ok: bool, ws_exists: bool) -> Result<Step, &'static str>`
——One→Two 要求 palette+agent；Three 要求 ws_exists；Two 的网络测试不在纯函数内（返回 Ok(Three) 由调用方在实际测试成功后调用）。RSX 渲染不要求测试。

## 禁区

- 不动 CSS 文件与任何视觉结构（卡片布局、抽屉、seg-bar 全保留）。
- 不动其它 pages / app.rs / registry。
- 不做 i18n 键变更：新按钮文案硬编码中文（与 L522/L534/L538 既有硬编码同风格）；复用既有 keys 处继续复用。
- Browse 按钮（L508-512）无 onclick 保持原样（文件选择器超范围）。
- 不动 kernel-api contracts 与 core。
- 完成副作用严格按③的三项；**不**调用 upsert_model_config / set_default_provider（处方明文范围外；"provider 未注册进 core"属已知产品限制，编排者记台账，不由你处理）。

## 复用侦察（必填进 report）

- api.rs 是否已有 test_provider_config / store_api_key 包装（应无）。
- 仓内是否已有 ProviderFormDto 组装先例（提示：Slint 侧 settings 回调可能有，仅参考不可跨 UI 层引用）。
- update_app_settings 在 ui_dioxus 的既有调用先例（pages_settings.rs workspace 路径用过，确认其 import 方式并保持一致）。

## 验证（report 必贴命令+尾部输出）

```
cargo check -p northhing
cargo check -p northhing --tests
& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib ui_dioxus
```
（第三条跑页面模块既有+新加单测；若 filter 命中 0 条贴输出说明。）

## Report

写 `.superpowers/sdd/reports/task-p3a-onboarding-report.md`：改动清单（file:line）、复用侦察结论、验证输出尾部、偏离及理由。
