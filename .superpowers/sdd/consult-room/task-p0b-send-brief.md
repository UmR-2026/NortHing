# Task P0b Brief — F2 消息发送/stop/streaming 接线

> 需求唯一来源：`.superpowers/sdd/consult-room/prescription-v3-20260825.md` §F2。
> Base commit: `8703901`（P0a 已落，api.rs 可用）。
> 前置已完成：`ui_dioxus/api.rs`（submit_turn/stop_turn/event_channel 等）。

## 范围：`src/apps/desktop/src/ui_dioxus/app.rs`（就地改）

### 1. input-box 换真输入元素

当前 `app.rs` 的 `div { class: "input-box", ... }`（占位符）→ Dioxus `input` 元素：

```rust
let mut user_input = use_signal(String::new);

input {
    class: "input-box",
    r#type: "text",
    value: "{user_input}",
    placeholder: "{locale.t(keys::DECK_PLACEHOLDER)}",
    oninput: move |e| user_input.set(e.value()),
    onkeydown: move |e| {
        if e.key() == dioxus::events::Key::Enter { send_action(); }
    },
}
```

- IME 注意：中文输入法合成中 Enter 不应触发发送。Dioxus 0.8 的 KeyboardEvent 若暴露 `is_composing()` 则加守卫；不暴露则注释说明已知限制（`// ponytail: IME composing Enter 守卫缺失，WebView2 若误触发再加`）。
- 移除占位符里的 `span { class: "cursor" }`（真 input 有原生光标）。

### 2. send/stop 合一按钮（保持真值语义）

状态：`streaming: Signal<bool>`（已有）+ 新增 `active_turn_id: Signal<Option<TurnId>>`。

- **非 streaming**：`user_input` 非空 trim 后 → 取/建 session → `api::submit_turn(&session_id, text)`：
  - Ok(turn_id) → `active_turn_id.set(Some(turn_id))`、`streaming.set(true)`、`user_input.set("")`、追加 Witness 气泡（见 §3）
  - Err → 不清输入，用 `KernelEventDto` 之外的本地 Signal 显示错误（就地文字，不接 banner 系统——本轮最小集）
- **streaming**：`api::stop_turn(&turn_id)` → `streaming.set(false)`、`active_turn_id.set(None)`

**session_id 来源（本轮 lazy，F6 正规化）**：
```rust
// 首个 send 时：
let sid = match api::list_sessions().await {
    Ok(list) if !list.is_empty() => list[0].id.clone(),
    _ => api 侧加 ensure_room_session() —— create_session(SessionConfigDto {
            workspace_path: None, agent_type: "agentic".into(),
            model_name: <读 GlobalConfig 默认模型；若 facade 无此 API 则 "default">,
            name: Some("诊室".into()) }) 返回 SessionId
};
```
`SessionConfigDto` 字段已核实（contracts session.rs:15-21）：`workspace_path: Option<String>, agent_type: String, model_name: String, name: Option<String>`。
`ensure_room_session()` 加在 `api.rs`（不在 app.rs 堆逻辑）。create 失败 → Err 上抛给 send handler 显示。

### 3. streaming 渲染（消费 api::event_channel()）

`app.rs` room 根组件挂载时 `use_future` 消费事件：

```rust
let mut assistant_draft: Signal<Option<String>> = use_signal(|| None);

use_future(move || {
    let mut rx = api::event_channel();
    let sid = session_id_signal; // 见 §2
    async move {
        while let Some(dto) = rx.recv().await {
            match dto {
                KernelEventDto::TextChunk { session_id, text } if Some(&session_id) == sid().as_ref() => {
                    let mut d = assistant_draft.write();
                    let cur = d.get_or_insert_with(String::new);
                    cur.push_str(&text);
                }
                KernelEventDto::TurnState { turn_id, state, .. } if 终态 => {
                    // Completed: assistant_draft 内容落成 MockEntry::Entity { who, body, children: vec![] } 追加进 entries；清 draft；streaming.set(false)
                    // Failed/Cancelled: 同落成但 body 前缀错误标记（Failed 带 error）；streaming.set(false)
                }
                _ => {}
            }
        }
    }
});
```

- **复用现有渲染**：用户消息落成 `MockEntry::Witness { who: "见证者", body }`；assistant 落成 `MockEntry::Entity { who: "它", body, children: vec![] }`。**不新增 entry 类型、不改 render_entry**。
- `TurnStateKind` 终态：`Completed | Failed | Cancelled`（contracts turn.rs:59-64）。
- `session_mock::seed_session()` 保留为启动 fallback（F1 批次再换 get_messages）；本轮 entries 初值不动。

## 禁区

- 不动 `session_mock.rs` / `entry.rs` / 其他页面文件
- 不建事件类型 / enum / bus
- 不接 approval 卡（P0c 的事）
- 不动 i18n 词表（DECK_PLACEHOLDER 已有键，直接复用；新增文案若必须，走既有 keys 不新增 ftl）
- `api.rs` 只允许加 `ensure_room_session()`，其余不动

## 验证（必跑并贴输出）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cd E:\agent-project\northing
cargo check -p northhing --features ui-dioxus
cargo test -p northhing --features ui-dioxus --lib ui_dioxus
```

报告：`.superpowers/sdd/reports/task-p0b-send-report.md`（status + files + 验证输出原文 + 偏离声明）。
