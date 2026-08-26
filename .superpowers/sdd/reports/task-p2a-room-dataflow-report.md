# Task P2a — Room 全页数据流报告

## 改动清单

- `src/apps/desktop/src/ui_dioxus/api.rs`:
  - 导入 `MessageDto`（L10）；
  - 新增 `get_messages(id: &SessionId) -> Result<Vec<MessageDto>, KernelError>` 薄包装（L63-L66）；
  - 在 `test_api_functions_fail_cleanly_before_init` 中增加 `get_messages` 覆盖（L157）。
- `src/apps/desktop/src/ui_dioxus/session_mock.rs`:
  - 导入 `MessageContentDto`, `MessageDto`, `MessageRoleDto`（L1）；
  - 新增纯函数 `pub fn messages_to_entries(msgs: Vec<MessageDto>) -> Vec<MockEntry>`（L96-L160），实现 User/Assistant/Mixed/Text/Multimodal 到 Witness/Entity/ToolLog 的映射，跳过 System 和 ToolResult；
  - 增加 5 个单元测试：User/Text 转 Witness、Assistant/Mixed 带 ToolLog 转换、Mixed 在 text 为空时回退 reasoning_content、System/Tool 被跳过、空 vec 返回空 vec（L181-L285）。
- `src/apps/desktop/src/ui_dioxus/app.rs`:
  - 新增启动 `use_future`（L120-L141）：首先调用 `api::ensure_room_session()` 设置 `session_id_signal`（同步使事件订阅过滤生效），成功后再调用 `api::get_messages()`，若返回消息非空则替换 `entries` 信号，空消息或异常时 `tracing::warn!` 并保留 `seed_session()`。
- `src/apps/desktop/src/ui_dioxus/pages_archive.rs`:
  - 在 `STRATA` 常量数组顶部添加 `// TODO(data): wire to session/archive query` 标记（L31）。
- `src/apps/desktop/src/ui_dioxus/pages_space.rs`:
  - 在 `DOORS` 常量数组顶部添加 `// TODO(data): wire to session/archive query` 标记（L46）。

## 复用侦察结论

1. **`api.rs` 中的 `get_messages` 包装**：
   - 检索方式：`grep "get_messages" src/apps/desktop/src/ui_dioxus`
   - 结果：无匹配项。确认此前未定义该内核 API 薄包装，在 `api.rs` 中补齐。
2. **`MessageDto` → `MockEntry` 转换器**：
   - 检索方式：`grep "MessageDto" src/apps/desktop` 及 `src/crates`
   - 结果：`src/apps/desktop/src/app_state/sessions.rs` 中存在 `message_to_item`（Slint 专属 DTO），`src/crates/assembly/core/src/kernel_facade/dto.rs` 中存在 core 侧与 `Message` 互相转换的方法。根据架构分层约束，core 侧逻辑不可跨层引用，Slint DTO 不可复用于 Dioxus `MockEntry`。`ui_dioxus` 内无既有转换器，故在 `session_mock.rs` 内新增纯函数 `messages_to_entries`。

## 验证结果

### 1. `cargo check -p northhing`
```
warning: `northhing` (bin "northhing") generated 35 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 4 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 51.21s
```

### 2. `cargo check -p northhing --tests`
```
warning: `northhing` (bin "northhing" test) generated 37 warnings (run `cargo fix --bin "northhing" -p northhing --tests` to apply 7 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 28.58s
```

## 偏离与 Caveat

- 无偏离，严格按 Brief 规范与已解决歧义执行。
