# Task P1b Brief — F5 Settings 持久化

> 需求唯一来源：`.superpowers/sdd/consult-room/prescription-v3-20260825.md` §F5。
> Base commit: `a62a74b`（P1a 已落）。

## 范围：`src/apps/desktop/src/ui_dioxus/pages_settings.rs`（就地改，不新建文件）

真实签名（已核实）：
- `crate::app_state::settings::io::load_app_settings() -> Result<AppSettings>`（io.rs:23）
- `crate::app_state::settings::io::update_app_settings<T>(f: impl FnOnce(&mut AppSettings) -> Result<T>) -> Result<T>`（io.rs:54，事务闭包）
- api_key 走 `crate::app_state::settings::keyring::store_api_key`（C3 模式，不落盘）

### 步骤

1. **读 AppSettings 结构**：先 `src/apps/desktop/src/app_state/settings/types.rs` 摸清与本页面对应的字段（providers、default model、MCP servers、display 类设置）。只接线**有对应字段**的 toggle；无对应字段的纯展示 mock（如"生物态呼吸 8s 周期"开关若无字段）保持 use_signal 并在行尾加 `// TODO(data): no AppSettings field yet`。

2. **页面加载**：`use_future` 启动时 `load_app_settings()` → 填充各 toggle Signal 初值。加载失败 warn + 保持默认。

3. **toggle 接线**：每个可持久化 toggle 的 onclick：
   ```rust
   onclick: move |_| {
       // 本地 Signal 先翻（乐观 UI）
       some_signal.toggle();
       let v = some_signal();
       spawn(async move {
           let _ = update_app_settings(|s| { s.<对应字段> = v; Ok(()) }).await;
       });
   }
   ```
   - **不做 debounce**（settings IO 频率低，ponytail）。
   - 失败仅 `tracing::warn!`，不回滚 UI（settings 写入失败是低频软错误）。

4. **model_name/provider id 映射**：页面引擎列表（Claude 3.7 Sonnet 等）是 mock 文案。若 `AppSettings`/`GlobalConfig` 里有默认模型字段，接通；没有则 TODO 注释保留 mock。

## 禁区

- 不建 `SettingsState` struct / `settings_store.rs` / event bus
- 不动 `io.rs` / `keyring.rs` 本体（只调用）
- 不动其他页面 / api.rs
- api_key 不落 GlobalConfig 磁盘（Scheme C 骨干不变量）
- 本页暂无 api_key 输入框——不加（F6 onboarding 才涉及）

## 验证（必跑并贴输出）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cd E:\agent-project\northing
cargo check -p northhing --features ui-dioxus
cargo test -p northhing --features ui-dioxus --lib ui_dioxus
cargo test -p northhing --lib settings
```

报告：`.superpowers/sdd/reports/task-p1b-settings-report.md`（status + files + 验证输出原文 + 偏离声明 + **接通了哪些字段 / 哪些保留 mock 的清单**）。
