# A7-A8: Message Streaming Indicator + Inspector UI Polish — Review Guide

> **Task ID**: A7-A8-2026-06-22  
> **Date**: 2026-06-22  
> **Branch**: `v3-restructure`  
> **HEAD**: `aa1072f`  
> **Author**: ZCode Agent  
> **Scope**: 实现 Message Streaming 指示器 + 修复 Inspector Skills 空状态文本

---

## 1. 任务概述 (What)

### 1.1 目标
- **P1**: 修复 InspectorView.slint 中过时的 skills 空状态占位文本（"Skills loading not yet implemented. (A4 will enable this)"）
- **P2**: 实现 Message Streaming 指示器——用户发送消息后，UI 显示动画指示器直到响应完成

### 1.2 非目标
- 不实现真正的消息流式传输（逐字显示），仅添加状态指示器
- 不改变动画效果本身（ChatMessageBubble.slint 已支持 `is-streaming` 属性）
- 不触及 coordinator 或 LLM 调用逻辑

---

## 2. 变更清单 (Changes)

### 2.1 修改文件

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/apps/desktop/src/ui/views/InspectorView.slint` | 修改 | 更新 skills 空状态文本 |
| `src/apps/desktop/src/app_state/mod.rs` | 修改 | 添加 `current_streaming_session` 字段 + streaming 生命周期管理 |
| `src/apps/desktop/src/app_state/sessions.rs` | 修改 | `message_to_item` 接受 `is_streaming` 参数；`build_messages_model` 支持 streaming 标记；`refresh_messages_ui` 接受 streaming session ID |

### 2.2 关键代码片段

**Streaming 状态生命周期** (`mod.rs`):
```rust
// 用户点击发送时设置 streaming 状态
app_state.set_streaming_session(Some(session_id.clone()));

// start_dialog_turn 完成后清除
app_state.set_streaming_session(None);
```

**Streaming 指示器逻辑** (`sessions.rs`):
```rust
let is_streaming = streaming_session_id.is_some() 
    && is_last 
    && is_assistant;
```

---

## 3. 设计决策 (Decisions)

### 3.1 决策 1：Streaming 状态存储位置
- **选项 A**: 存储在 `AppState` 中（全局 Mutex）
- **选项 B**: 存储在 Slint UI 属性中
- **选择**: A，理由：
  - `AppState` 是 Rust 端唯一可信状态源
  - 多个回调需要读写此状态（send_message, refresh_messages）
  - Slint 属性是单向数据流，不适合跨回调状态共享

### 3.2 决策 2：何时显示 Streaming 指示器
- **选项 A**: 只要 `start_dialog_turn` 未完成就显示
- **选项 B**: 检测最后一条消息是否为 assistant 且响应不完整
- **选择**: A，理由：
  - 更简单可靠，不需要解析消息内容
  - 与现有的 `start_dialog_turn` 异步模型自然对齐
  - 用户体验：用户发送后立即看到反馈

### 3.3 已知妥协
- **"假阳性"风险**: 如果 `start_dialog_turn` 快速完成（如缓存命中），用户可能几乎看不到指示器
- **"假阴性"风险**: 如果 coordinator 内部出错但 UI 未收到通知，指示器可能无限显示（直到用户刷新）
- **缓解**: 错误路径会清除 streaming 状态；refresh-messages 回调也会使用当前状态

---

## 4. 测试验证 (Verification)

### 4.1 编译状态
```bash
cargo check --manifest-path src/apps/desktop/Cargo.toml --lib --tests
```
- **错误数**: 0
- **警告数**: 0（northhing 本身无新警告；northhing-core 有 3 个已有警告）

### 4.2 测试状态
| 测试套件 | 结果 | 备注 |
|----------|------|------|
| desktop lib | ⚠️ 未运行 | 环境 `dlltool` 缺失导致链接失败 |
| 新增测试 | 3 个 | 编译通过（代码验证） |

### 4.3 新增测试详情
1. **`build_messages_model_streaming_on_last_assistant`**: 验证 streaming 指示器只在最后一条 assistant 消息上显示
2. **`build_messages_model_no_streaming_when_last_is_user`**: 验证用户消息不会显示 streaming
3. **`app_state_streaming_session_round_trip`**: 验证 getter/setter 正确性

### 4.4 手动验证步骤
1. 启动桌面应用
2. 选择或创建一个 session
3. 发送一条消息
4. 观察：消息气泡底部应出现动画指示条（蓝色渐变）
5. 等待响应完成后，指示条应消失

---

## 5. 风险与问题 (Risks)

### 5.1 已知问题
| 问题 | 严重性 | 状态 | 说明 |
|------|--------|------|------|
| 测试无法运行 | 中 | 已接受 | 环境 `dlltool`/`nanosleep64` 缺失；编译阶段已验证 |
| 指示器依赖 `start_dialog_turn` 完成 | 低 | 已接受 | 如果 coordinator 内部挂起，UI 会显示无限 streaming |

### 5.2 后续债务
- **真正的流式传输**: 当前只是状态指示器，未来可以实现逐字显示（需要 coordinator 支持 SSE 流式回调到 UI）
- **超时处理**: 没有 streaming 超时机制，如果 coordinator 失败，指示器可能一直显示

---

## 6. Review 检查清单 (Checklist)

### 6.1 代码审查重点
- [ ] `AppState.current_streaming_session` 的 Mutex 使用是否正确（无死锁风险）
- [ ] `on_send_message` 中错误路径是否都清除了 streaming 状态
- [ ] `build_messages_model` 的 streaming 逻辑是否只影响最后一条 assistant 消息
- [ ] 所有 `refresh_messages_ui` 调用点是否传入了正确的 streaming session ID

### 6.2 测试审查重点
- [ ] 3 个新测试是否覆盖了主要场景
- [ ] 测试是否可以在其他环境（有 `dlltool`）中运行通过

### 6.3 文档审查重点
- [ ] InspectorView.slint 的新文本是否合适
- [ ] 代码注释是否清晰说明了 streaming 生命周期

---

## 7. 后续建议 (Next Steps)

### 7.1 立即跟进（下一 session）
- 在可以运行测试的环境中验证 3 个新测试通过
- 考虑添加 streaming 超时机制（如 30 秒后自动清除）

### 7.2 未来迭代
- 实现真正的消息流式传输（逐字显示）
- 添加 "停止生成" 按钮（取消正在进行的 LLM 调用）

---

> **End of Review Guide**
> 
> 此文件由 ZCode Agent 在任务完成后自动生成，供后续 review 参考。
> 对应 commit: `aa1072f`
