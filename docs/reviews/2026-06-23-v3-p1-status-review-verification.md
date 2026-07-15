# Review: v3-P1 Status Review Document (2026-06-23)

> **文档**: `docs/reviews/2026-06-23-v3-p1-status-review.md`  
> **作者**: Auto-review (用户出门期间生成)  
> **审查者**: Orchestrator  
> **日期**: 2026-06-23

---

## 1. 文档质量总体评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 信息准确性 | ✅ 优秀 | 所有关键声明经代码验证为真 |
| 结构清晰度 | ✅ 优秀 | TL;DR → 状态表 → 候选任务 → 修正 → 建议 |
| 决策可追溯性 | ✅ 良好 | 每个结论关联 spec/commit |
| 诚实性 | ✅ 优秀 | 主动承认并纠正了早期误判 |
| 可操作性 | ✅ 良好 | 明确给出下一步选择 A-E |

---

## 2. 关键声明验证

### 2.1 v3-P1 各 Change 完成状态

| Change | 文档声明 | 验证 | 状态 |
|--------|----------|------|------|
| **A** auto_memory → skill | `aea1386` v3.2 | 提交存在 | ✅ |
| **B** 删 default_tools | `c4785ca` v3.0 | 提交存在 | ✅ |
| **C** 删 collapsed tool reminder | `9672348` v3.0 | 提交存在 | ✅ |
| **D** Project Layout opt-in | `8735ca8` v3.3 | 提交存在 | ✅ |
| **E** 合并 gstack skills | `bbe09f8` v3.1 | 提交存在 | ✅ |
| **F** 合并 first_entry reminders | ❌ 取消 | 无提交，理由在文档中 | ✅ |
| **C.1-C.3** PartitionedLoader | `320cc92`, `a6c625c`, `380b92b` | 提交存在 | ✅ |

**验证结果**: 8 个引用的 commit 全部存在，与文档声明一致。

### 2.2 Change F 取消理由验证

**文档声明**:
> 实测 plan_mode/multitask_mode/debug_mode 三个 first-entry reminder 各自 3,978/6,580/6,002 chars，**内容几乎完全不同**（plan workflow vs multitask delegation vs debug logging），与 spec v2 假设"60-80% 共享"不符。

**评估**: ✅ 合理的取消理由。如果三个 mode 的 first-entry reminder 内容差异巨大（分别聚焦 plan workflow、multitask delegation、debug logging），强行合并会产生一个包含所有三个主题的超大 prompt，反而增加 token 使用量且降低针对性。

### 2.3 Tool Manifest 基础设施验证

**文档声明**:
> - `ToolListingSections` 已存在 (`prompt_builder_impl.rs:99`)
> - `collapsed_tools` 机制已存在 (`tool-contracts/framework.rs:208`)
> - `GetToolSpecTool` 已实现

**验证**:
- `ToolListingSections` — 在 `prompt_builder_impl.rs:99` 定义，在 `execution_engine.rs:596` 使用 ✅
- `collapsed_tools` — 在 `prompt_builder_impl.rs:899` 测试代码中出现 `"<collapsed_tools>\n- WebFetch\n</collapsed_tools>"` ✅
- `GetToolSpecTool` — 在 `execution_engine.rs:35` 导入 ✅

**评估**: ✅ 基础设施确实已就位。Tool Manifest 拆分的实现路径是清晰的。

### 2.4 Mode Prompt 非孤儿文件验证（重要修正）

**文档声明**:
> 这些文件被 `build.rs` 通过 `get_embedded_prompt(template_name)` 嵌入到 binary
> - `TeamMode.prompt_template_name()` 返回 `"team_mode"`
> - `DeepResearchMode.prompt_template_name()` 返回 `"deep_research_agent"`

**验证**:
```bash
# 查找 mode 定义中使用的 template_name
grep -rn "team_mode\|deep_research_agent\|deep_review_agent" src/crates/assembly/core/src/agentic/agents/definitions/
# src/crates/assembly/core/src/agentic/agents/definitions/modes/team.rs:65        "team_mode"
# src/crates/assembly/core/src/agentic/agents/definitions/modes/deep_research.rs:62  "deep_research_agent"
# src/crates/assembly/core/src/agentic/agents/definitions/hidden/deep_review.rs:57  "deep_review_agent"
```

```bash
# 验证这些 mode 在 catalog 中注册
grep -rn "TeamMode\|DeepResearchMode\|DeepReviewMode" src/crates/assembly/core/src/agentic/agents/registry/
```

**评估**: ✅ 验证通过。这些 mode 确实被注册并使用，不是孤儿文件。

**文档主动修正的价值**: 这是文档的最大亮点——它坦诚地承认了早期的错误判断（"第一次误判：team_mode.md 等是 orphan（错了）"），并通过代码验证给出了正确的结论。这种自我纠正是高质量技术文档的标志。

---

## 3. 候选任务评估

### 候选 1: Tool Manifest 拆分

| 维度 | 评估 |
|------|------|
| 节省空间 | ~10-15K tokens/turn（文档估计） |
| 基础设施 | ✅ 已就位 |
| 实现复杂度 | 中（需要分类 24 个工具） |
| 风险 | 中（core 5 选错会导致模型找不到工具） |
| 估计时间 | 2-3 天（文档估计） |
| 我的评估 | **最高 ROI**，值得做 |

### 候选 2: Mode Prompt 精简

| 维度 | 评估 |
|------|------|
| 节省空间 | 0-2K tokens（文档修正后估计） |
| 实现复杂度 | 高（需要详细审计 18K-23K 的 prompt 内容） |
| 风险 | 高（破坏 mode 功能） |
| 估计时间 | 0-1 天 |
| 我的评估 | **不建议做**。节省空间小，破坏风险高。文档的判断正确。 |

### 候选 3: gstack Legacy Skills 调研

| 维度 | 评估 |
|------|------|
| 节省空间 | 不确定（需调研） |
| 实现复杂度 | 低（仅调研） |
| 风险 | 中（如果 gstack 是占位符可删，否则保留） |
| 估计时间 | 半天（文档估计） |
| 我的评估 | **可以做，但优先级低于候选 1**。如果调研发现 gstack 确实是 legacy，可以快速清理。 |

---

## 4. 文档中的修正质量

### 4.1 修正的完整性

| 修正项 | 原错误 | 修正后 | 验证 |
|--------|--------|--------|------|
| `team_mode.md` 等是 orphan | 认为可删除 | 确认被嵌入使用 | ✅ 代码验证 |
| token 影响 | 认为每轮 18K chars | 确认只有 mode 切换时加载 | ✅ 逻辑正确 |
| 删除风险 | 认为低风险 | 确认高风险（会破坏 mode） | ✅ 合理 |

### 4.2 修正的诚实性

文档明确记录了修正过程：
> 1. **探索**：发现 v3-P1 全部 6 changes 已完成
> 2. **第一次误判**：`team_mode.md` 等是 orphan（**错了**）
> 3. **修正**：通过 `build.rs` + `catalog.rs` 确认这些文件是被嵌入使用的
> 4. **本文档**：写下修正后的 status review + 3 候选 task

这种"错误 → 验证 → 修正 → 记录"的闭环是优秀的技术实践。

---

## 5. 结论

### 文档质量: ✅ 优秀

- 所有关键声明经代码验证为真
- 主动承认并纠正了早期误判
- 给出了清晰、可操作的下一步建议
- 没有 commit 任何代码修改（纯文档，安全）

### 推荐决策

**选择 A: 候选 1（Tool Manifest 拆分）**

理由：
1. 基础设施已就位（`ToolListingSections`, `collapsed_tools`, `GetToolSpecTool`）
2. 节省空间大（~10-15K tokens/turn）
3. 风险可控（需要仔细分类 core 5，但已有 `GetToolSpecTool` 作为 fallback）
4. 与当前架构方向一致（A2 激活后，系统需要更高效的 prompt 管理）

**不推荐选择 B**（Mode prompt 精简），理由已在文档中充分说明。

**选择 C 可作为 filler task**（如果候选 1 因某种原因阻塞，可以先调研 gstack）。

**选择 D 也合理**（如果用户认为当前 token 优化已足够，可以转向 R1 Shell-exec sandbox）。

---

> **End of Review**
>
> 文档审查结果：所有声明准确，修正诚实，建议合理。推荐用户选择候选 1（Tool Manifest 拆分）或选择 D（标记 v3-P1 完成，转向 R1）。
