# 后续任务规划文档

> **Created:** 2026-06-23
> **Branch:** `v3-restructure`
> **HEAD:** `5b2c137`
> **Previous:** A2 review fixes + A7-A8 streaming indicator complete

---

## 1. 当前项目状态快照

| 指标 | 数值 |
|------|------|
| 总提交数 | 153+ on v3-restructure |
| 回归测试 | 8/8 PASS |
| agent-dispatch 测试 | 24/24 PASS |
| desktop 测试 | 12/12 PASS |
| 编译警告 | 0 |
| Slint 回调 | 10/10 wired |
| 4 个 const flags | 全部 `false` |

**最近完成的工作：**
- A2 review 修复（P0-1 dialog_turn_id 缓存、P0-2 workspace 缓存、P1 FinishReason 映射）
- A7-A8 Streaming Indicator（UI-only 实现，`current_streaming_session` 状态跟踪）
- Review 指导文件自动化（`scripts/update-review-doc.sh` + `.cargo/config.toml` 别名）

---

## 2. 可选任务清单（按价值/紧急度排序）

### 🔴 高优先级 — 安全

| 编号 | 任务 | 估计时间 | 价值 | 说明 |
|------|------|----------|------|------|
| ~~**R1**~~ | ~~Shell-exec sandbox + confirm audit~~ | ~~2天~~ | ✅ **DONE 2026-06-23** | Spec: `docs/superpowers/specs/2026-06-23-r1-shell-exec-sandbox-design.md`. 18 commits, 1467 tests passing |

### 🟠 高优先级 — Token 削减（日常使用 ROI）

| 编号 | 任务 | 估计时间 | 价值 | 说明 |
|------|------|----------|------|------|
| ~~**v3-P1**~~ | ~~Prompt loader architecture~~ | ~~1-2天~~ | ✅ **DONE** | 6 changes A-F + PartitionedLoader. TaskTool collapse 额外节省 ~1K tokens/turn |

### 🟡 中优先级 — 代码质量

| 编号 | 任务 | 估计时间 | 价值 | 说明 |
|------|------|----------|------|------|
| **R2** | ChatView 拆分 | 2-3天 | 高 | 36 字段 → 4 子结构，减少重复 |
| **R3** | SessionStoragePathResolution enum | 1-1.5天 | 中 | 46 文件路径解析统一 |
| **R4** | tracing + 错误门面统一 | 1.5天 | 中 | 日志和错误处理标准化 |

### 🟢 低优先级/阻塞

| 编号 | 任务 | 状态 | 说明 |
|------|------|------|------|
| **K.2.4** | `create_ui` mock display test | 🚫 BLOCKED | slint 1.16.1 未暴露 backend-testing |
| **Full A1** | 替换 CoordinatorHiddenSubagentSkill | ⏸️ DEFERRED | 多日重构，需完整设计文档 |

---

## 3. 推荐顺序

```
选项 A: 安全优先
  R1 (2天) → v3-P1 (1-2天) → R2 (2-3天)

选项 B: 日常使用优先  
  v3-P1 (1-2天) → R1 (2天) → R2 (2-3天)

选项 C: 质量优先
  R2 (2-3天) + R3 (1-1.5天) 并行 → R4 (1.5天)
```

**建议：** 选 **选项 B**（v3-P1 → R1 → R2）
- v3-P1 的 token 节省直接影响每次 LLM 调用的成本和延迟
- R1 安全审计虽然重要，但当前系统尚无外部用户，风险可控
- 完成 v3-P1 后，R1 的审计范围可能因代码结构变化而减小

---

## 4. 每个任务的关键信息

### v3 Phase 1: Prompt Loader Architecture

**目标：** 将每轮 LLM 调用的 prompt 从 ~40-65K tokens 削减到 ~5K

**核心设计：**
- `skills.db`: 预编译 skill 模板，按需加载
- `agents.db`: 预编译 agent 定义，按需加载  
- `PartitionedLoader`: 分层加载策略（core + mode + session + turn）

**关键文件（预期）：**
- `src/crates/assembly/core/src/prompt/` — 新模块
- `src/crates/contracts/prompt/` — 契约定义
- `src/crates/services/services-core/src/prompt/` — 服务实现

**验收标准：**
- [ ] `cargo check --workspace` 0 errors
- [ ] 现有测试全部通过
- [ ] 手动验证：单轮 prompt 长度 < 10K tokens

---

### Remake R1: Shell-exec Sandbox

**目标：** 审计并加固所有 shell 命令执行路径

**关键检查点：**
- `std::process::Command` 的所有调用点
- `tokio::process::Command` 的所有调用点
- 用户输入是否经过 sanitization
- 是否需要添加 `--confirm` 交互

**验收标准：**
- [ ] 所有 shell 执行路径已审计
- [ ] 危险命令（rm, format, etc.）需要用户确认
- [ ] 添加 `#[cfg(test)]` mock 路径用于测试

---

### Remake R2: ChatView 拆分

**目标：** 将 `ChatView.slint` 的 36 个字段拆分为 4 个子结构

**设计：**
- `ChatViewHeader`: 标题、模式、工具栏
- `ChatViewMessageList`: 消息列表、滚动、分页
- `ChatViewInputBar`: 输入框、发送按钮、快捷操作
- `ChatViewStatusBar`: 状态、连接、错误提示

---

## 5. 下一步行动

请选择一个选项开始：

1. **开始 v3-P1**（Prompt Loader）— 最大日常使用价值
2. **开始 R1**（Shell Sandbox）— 最高安全价值
3. **开始 R2**（ChatView 拆分）— 代码质量提升
4. **自定义顺序** — 告诉我你的优先级

---

> **Review 流程提醒：** 每个任务完成后，自动运行 `cargo check-review` 更新 review 指导文件。
