# Task Brief — W9-3: 降级即报错 UI 路径（原则 9：quota/key 耗尽可见）

仓库：E:\agent-project\NortHing（main）。起点：W9-2 完成（commit c80227b）。scope: S-M。

## 背景/动机

- 产品论题原则 9：降级即报错——后端错误分类有，但 UI 无"key 耗尽/quota 用完"的可见路径。
- 现状：`classify_ai_error_message` 在 `northhing-core-types/errors.rs` 已将 quota/billing/balance 等消息分类为 `ErrorCategory::ProviderQuota`/`ProviderBilling`；KernelEventDto::TurnState 有 `error: Option<String>` 字段。桌面 UI 当前把这些错误只作为普通字符串显示在聊天里（`[Error: {err_text}]`），用户看不出系统进入降级状态。
- 缺少：一个可见的、非弹出式的降级状态提示，让用户知道当前 provider 不可用/资源耗尽，且后续请求会失败。

## 现状（可直接采信）

- `northhing-core-types/src/errors.rs`:
  - `classify_ai_error_message(msg: &str) -> ErrorCategory` 可识别中英文 quota/billing 关键词（余额不足、账户已欠费、insufficient_quota、http 402 等）。
  - 该函数目前**未从 kernel-api 导出**（desktop 依赖 kernel-api，不直接依赖 core-types）。
- `kernel-api/src/lib.rs`：未 re-export classify_ai_error_message（或 ErrorCategory）。
- desktop `src/apps/desktop/src/ui_dioxus/app.rs`:
  - TurnStateKind::Failed 时 error 字符串只显示在聊天消息中（lines 181-198）。
  - `send_error: Signal<Option<String>>` 仅用于 submit 失败提示（lines 53, 305）。
  - 无全局降级状态信号。
- desktop `src/apps/desktop/src/ui_dioxus/css.rs`：有 `.send-error` class。

## Spec（验收标准）

### 1. 契约层：导出错误分类能力

在 `src/crates/contracts/kernel-api/src/lib.rs` 中 re-export：
- `pub use northhing_core_types::error::{ErrorCategory, classify_ai_error_message};`
（确认跨 crate 依赖链最长路径可行；不可行则改走适配层。）

### 2. 降级检测（desktop api.rs 或 app.rs）

在 TurnStateKind::Failed 的 event handler 里（app.rs 181 附近）：
- 用 `classify_ai_error_message` 检测 error 文本。
- 如果分类为 `ProviderQuota` 或 `ProviderBilling` → 设置降级状态。
- 在 TurnStateKind::Completed 时 → 清除降级状态。
- submit_turn Err 时 → 同样检测，设置降级状态。

### 3. 降级状态 Signal

在 room_app_root 中新增：
- `let mut degraded: Signal<Option<String>> = use_signal(|| None);`
- 值 = 降级原因短文本（中文），None = 正常。

### 4. 降级横幅 UI

在 room 的 station-head 下方、chat flow 上方插入：
- 条件渲染：`if let Some(reason) = degraded.read() { ... }`
- 样式：`.degraded-banner` 类，警告色（amber/orange），圆角，内边距。
- 文案：降级原因短句（来自 classify 结果）。
- **不可关闭**（integrated 降级状态，需服务恢复后自动消失）。
- 位置：可见但不挡主要内容。

### 5. 样式更新

在 `css.rs` 中加 `.degraded-banner` CSS（≤10 行）。

## Constraint（必读）

1. **分层边界**：error classification 逻辑在 contracts 层；降级检测在 desktop；UI 在 desktop。
2. **日志纪律**：英文无 emoji。
3. **rot-budget**：不上调任何 ceiling。
4. **不做的事**：不添加 provider 切换、不显示详细错误堆栈、不弹窗——降级横幅只负责"你知道有麻烦了"。
5. **commit 规则**：恰好一个 commit；不含 `.superpowers/`。

## 跨任务接口

- 依赖 `classify_ai_error_message` 从 kernel-api 导出（W9-3-1 需要先做）。
- 依赖 existing `TurnStateKind::Failed` → error 字段（已存在）。

## 跨任务接口

- 依赖 `classify_ai_error_message` 从 kernel-api 导出（W9-3-1 需要先做）。
- 依赖 existing `TurnStateKind::Failed` → error 字段（已存在）。

## 渲染后的预期行为

1. 正常 → hook sets `degraded` to None → 不渲染横幅。
2. Turn 失败 + quota 错误 → banner 显示颜色区分原因（"API 资源耗尽" 或 "账单/套餐异常"），占位但不挡内容。
3. 下次 Turn 成功 → banner 消失。
4.  quota 错误不应阻断后续交互输入（降级 ≠ 冻住 UI）。
