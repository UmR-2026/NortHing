---
name: code-review
description: "Use when reviewing code changes before merge, when a subagent completes a task, or when a const flag change needs quality gating. Enforces five-axis review with severity labels. Trigger this between tasks in subagent-driven-development and before finishing-a-development-branch."
---

# Code Review (northhing v3 适配— > 合并— obra/superpowers requesting/receiving-code-review + addyosmani/agent-skills code-review-and-quality

## When to trigger

- subagent 完成一个任务后（subagent-driven-development— review 阶段— - 合并分支前（finishing-a-development-branch 的前置检查）
- 引入新的 const flag 后（质量关卡— - 手动请求：当你不确定某个改动是否安全

## Review 流程

### Step 1: 前置检— ```bash
cd E:/agent-project/northhing-v3
cargo fmt --check # 格式检— cargo clippy --workspace -- -D warnings # lint 检— cargo test --workspace --lib # 全量测试
```

任何一项失— 不进入审查，退回修复— ### Step 2: 收集变更上下— ```bash
git log --oneline main..HEAD # 列出所— commit
git diff main...HEAD --stat # 变更文件统计
```

### Step 3: 五轴审查

对每个变更文件，按五个维度逐项检查：

---

## Axis 1: 正确— (Correctness)

**问题**：代码是否做了它应该做的事？

检查项— - [ ] const flag— if/else 两个分支都覆盖了
- [ ] flag 默认值是 `false`（safe default，旧路径生效— - [ ] 重构后的行为与重构前**语义等价**（不只是"看起来一— - [ ] 错误处理路径完整（不是只处理 happy path— - [ ] 没有 unwrap() 在可能为 None 的地— - [ ] panic 路径— `catch_unwind` 保护（如果是桌面端代码）

**Rust 特定**— - [ ] 所有权转移是否符合预期（没有意外的 move— - [ ] 生命周期标注是否正确（或省略后编译器推断正确— - [ ] trait 实现是否符合 Send/Sync 约束（如果涉及多线程— ---

## Axis 2: 可读— (Readability)

**问题**：其他开发者（— agent）能快速理解吗— 检查项— - [ ] const flag 命名清晰：`USE_X_FEATURE` 而非 `FLAG1` / `NEW`
- [ ] 函数职责单一，名称准— - [ ] 注释解释 **为什— *，不解释 **是什— *（代码本身说了是什么）
- [ ] 没有死代码（旧路径被 flag 关闭后，确认仍被其他 flag 配置使用— ---

## Axis 3: 架构 (Architecture)

**问题**：改动放在了正确的位置吗— 检查项— - [ ] const flag 放在正确— crate/module 层级
- [ ] 没有— crate 的循环依— - [ ] 新代码遵— workspace 现有的模块边— - [ ] 如果引入了新— Skill pointer 模式，与 v3.2 (auto_memory) 的模式一— **引用 codebase-design skill 的词— *— - [ ] 接口面（interface）够窄？— trait 方法— 3 个为— - [ ] 实现面（implementation）足够深？— 复杂逻辑隐藏— impl 内部
- [ ] seam 在正确的位置？— const flag— seam，不是业务逻辑

---

## Axis 4: 安全— (Safety)

**问题**：改动引入了不安全的行为吗？

检查项— - [ ] 没有 `unsafe` 块（如果有，需要安全论证）
- [ ] slice 索引有边界检查（教训：P1-10 to_tauri_color panic— - [ ] panic reporter— catch_unwind（教训：P1-11 panic_report re-panic— - [ ] 如果涉及桌面— Tauri 代码，前— 后端边界验证完整

---

## Axis 5: 性能 (Performance)

**问题**：改动是否影响运行时性能— 检查项— - [ ] const flag 是编译期常量（零运行时开销，编译器— const propagation— - [ ] prompt loading 的新路径不比旧路径慢
- [ ] 没有在热路径上引入不必要的分配（String/Vec— - [ ] 测试通过时间没有显著增加

---

## Step 4: 严重性分— 对每个发现，分配严重性标签：

| 标签 | 含义 | 行动 |
|------|------|------|
| 🔴 **Critical** | 会导致编译失败、运行时 panic、或行为错误 | **阻塞合并**，必须立即修— |
| 🟡 **Important** | 潜在— bug 风险、架构问题、或测试缺失 | **应该修复**，可以创— follow-up task |
| 🔵 **Nit** | 风格偏好、命名建— | **可选修— *，不阻塞 |
|— **FYI** | 信息性发现，不需要行— | 记录在案 |

---

## Step 5: 输出格式

```markdown
## Code Review Report

**Reviewer**: <agent name>
**Scope**: commits <sha1>..<sha2>, files changed: N
**Verdict**:— APPROVED / ⚠️ APPROVED WITH NOTES /— CHANGES REQUESTED

### Critical Issues
1. [文件:行号] 描述 + 建议修复

### Important Issues
1. [文件:行号] 描述 + 建议修复

### Nits
1. [文件:行号] 描述

### FYI
1. 观察/建议
```

---

## 审查者行为规— ### 应该做的

- **基于代码证据判断**，不凭感— - **验证而非信任**：每个断言都检查实际代— - **区分品味和技术错— *：品味问题用 Nit，技术错误用 Critical/Important
- 如果审查者不理解 const flag 模式，先— `northhing-v3-workflow` skill

### 不应该做— -— "You're absolutely right!"— 不表演赞— -— 建议没有代码依据— 改进"
-— 要求添加 "just in case" 的防御性代— -— 因为"其他项目这么— 就要求照— -— 审查 const flag 模式本身是否合理（这— ADR 已经决定的，除非有新的技术证据）

---

## 与其— skill 的关— - **subagent-driven-development**: review 阶段使用— skill 的五轴框— - **verification-before-completion**: 审查前先— verification
- **finishing-a-development-branch**: 合并前强— code review
- **systematic-debugging**: 如果审查发现 bug，用 debugging skill 修复
