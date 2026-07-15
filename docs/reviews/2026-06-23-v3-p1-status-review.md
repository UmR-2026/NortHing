# v3-P1 Prompt Loader — Status Review (2026-06-23)

> **Author:** Auto-review (用户出门期间生成)
> **Purpose:** 让用户回来后基于这份文档 review，决定 v3-P1 是否还需要做，还是可以标记为"已完成 + 转向其他任务"
> **Related spec:** `docs/superpowers/specs/2026-06-17-v3-prompt-loader-design-v2.md`

---

## TL;DR

**v3-P1 (按 v2 spec 的 6 changes) 已经 100% 完成**。剩下两个 candidate task **不是简单的 dead code 清理**，需要重新评估：

| 项 | 现状 | 是否值得做 |
|----|------|-----------|
| Change F（合并 first_entry reminders） | **已取消**（v3 完成记录） | ❌ 内容差异太大 |
| Tool manifest 拆分 (10-15K tokens) | 基础设施已就位 (`ToolListingSections`, `collapsed_tools`) | ⚠️ 可行但需评估 |
| ~~Mode prompts 是 orphan~~ | **修正：实际被嵌入使用** | ❌ 见 §3 |
| gstack 删 legacy | 已合并到 bundle | ⚠️ 需调研 |

---

## 1. v3-P1 各 Change 完成状态

| Change | 描述 | 节省 tokens | 状态 | Commit |
|--------|------|-------------|------|--------|
| **A** | `auto_memory` → skill | ~4,000-4,500 | ✅ v3.2 | `aea1386` |
| **B** | 删 agent listing 的 `default_tools` | ~1,500-2,000 | ✅ v3.0 | `c4785ca` |
| **C** | 删 collapsed tool listing reminder | ~80-120 | ✅ v3.0 | `9672348` |
| **D** | Project Layout opt-in | ~500-2,000 | ✅ v3.3 | `8735ca8` |
| **E** | 合并 gstack skills | ~500-1,000 | ✅ v3.1 | `bbe09f8` |
| **F** | 合并 first_entry reminders | ~2,000-3,000 (估计) | ❌ **取消** | (见下文) |
| **C.1-C.3** | PartitionedLoader 4-layer cache + 集成 | (性能，不是 token) | ✅ | `320cc92`, `a6c625c`, `380b92b` |
| **Total shipped** | - | **~6,500-9,500** | ✅ | - |

**F 取消理由**（已在 PROJECT_STATE.md 记录）：

> 实测 plan_mode/multitask_mode/debug_mode 三个 first-entry reminder 各自 3,978/6,580/6,002 chars，**内容几乎完全不同**（plan workflow vs multitask delegation vs debug logging），与 spec v2 假设"60-80% 共享"不符。

**结论**：v3-P1 spec 的 6 changes 全部处理完，**v3-P1 任务标记为已完成** ✅。

---

## 2. 候选 Next Tasks（用户回来后可选）

### 候选 1: Tool Manifest 拆分（最大 token 节省）

**目标**：将 24 个 tool 的完整 manifest 拆分为 "core 5（始终列出）+ advanced 19（按需 GetToolSpec）"。

**节省估计**：~10,000-15,000 tokens/turn（v2 spec §6 提到）

**现状**：
- ✅ `ToolListingSections` 已存在（`prompt_builder_impl.rs:99`）
- ✅ `collapsed_tools` 机制已存在（`tool-contracts/framework.rs:208`）
- ✅ `GetToolSpecTool` 已实现

**需要做的事**：
- 把当前的 24 个 tool 分类为 core/advanced
- 修改 `resolve_tool_manifest` / `build_prompt_context_for_workspace` 来使用 split
- 测试 core 5 包含所有常用工具，advanced 19 通过 GetToolSpec 按需加载

**风险**：中（如果 core 5 选错，模型会找不到常用工具）

**估计时间**：2-3 天

### 候选 2: ~~Mode prompt 精简~~ 误判（见 §3）

**原假设（错误）**：`team_mode.md` (18,789 chars), `deep_research_agent.md` (23,143), `deep_review_agent.md` (23,566) 是 orphan 文件，可以删除。

**实际状态（修正）**：
- 这些文件被 `build.rs` 通过 `get_embedded_prompt(template_name)` 嵌入到 binary
- `TeamMode.prompt_template_name()` 返回 `"team_mode"`
- `DeepResearchMode.prompt_template_name()` 返回 `"deep_research_agent"`
- 所有 mode 都被注册到 `registry/catalog.rs` 中

**这些 mode 在什么情况下被加载？**
- 只有当用户主动切换到 Team/DeepResearch/DeepReview mode 时才注入 prompt
- 但 agent listing 中会列出这些 agent 的 short description（不带完整 prompt）

**Token 影响**：不是每轮 18K chars，而是：
- Agent listing 中每个 mode 的 short description（几百 chars）
- 完整 prompt 只有切换到该 mode 时才加载

**修正后的判断**：
- 这些文件**不是 dead code**，删除会破坏 Team/DeepResearch/DeepReview mode
- 如果想精简，应该看这些 prompt 内部是否有冗余（如 `team_mode.md` 18K chars 是否包含可以拆分的部分）

**节省估计**：0-2,000 tokens（取决于内容审计结果，需要详细 review）

**风险**：高（删错会破坏 mode 功能）

**估计时间**：0（不值得做） 或 1 天（详细 prompt 内容审计）

### 候选 3: gstack Legacy Skills 调研

**目标**：找出 `gstack-bundle` 内的 13 个 skill 哪些是 legacy/未使用。

**风险**：中（如果 gstack 是占位符，直接删；如果真的要用，得保留）

**估计时间**：半天调研

---

## 3. 重要修正（避免基于错误信息做决策）

**第一次 review 时我错误地把 `team_mode.md` 等判断为 orphan**。经过进一步 grep + `build.rs` 代码阅读，这些文件：

1. 被 `build.rs:280-287` 通过 `map.insert("template_name", "content")` 嵌入到 `EMBEDDED_PROMPTS`
2. 通过 `get_embedded_prompt(template_name)` 函数读取
3. 由 `Agent::build_prompt()` 在每次 `get_system_prompt()` 时调用
4. TeamMode/DeepResearchMode/DeepReviewMode 都注册在 `registry/catalog.rs` 中

**正确的 token 影响评估**：
- 完整 prompt（18K-23K chars）只在 mode 切换时加载，**不是每轮**
- Agent listing 中的 short description 才会每轮注入

如果想真正减少每轮 token，应该看 agent listing 的 short description 而不是完整 prompt。

---

## 4. 建议

**最高 ROI**：候选 1 (Tool Manifest 拆分，~10-15K tokens/turn)
**不建议**：候选 2 (Mode prompt 精简) — 节省空间小，破坏风险高
**最高不确定性**：候选 3 (gstack，需要先调研)

---

## 5. 已做的工作（本 session 在你出门期间）

1. **探索**：发现 v3-P1 全部 6 changes 已完成
2. **第一次误判**：`team_mode.md` 等是 orphan（**错了**）
3. **修正**：通过 `build.rs` + `catalog.rs` 确认这些文件是被嵌入使用的
4. **本文档**：写下修正后的 status review + 3 候选 task

**未做的工作**（等你回来 review 后再决定）：
- **没有动任何代码**
- **没有 commit 任何修改**

---

## 6. 下一步选择

用户 review 后：
- 选择 A：候选 1（tool manifest 拆分）— 走 brainstorming → spec → 等 review → 执行
- 选择 B：候选 2（mode prompt 内容审计）— 先 brainstorm 再决定
- 选择 C：候选 3（gstack 调研）— 先 brainstorm 再决定
- 选择 D：标 v3-P1 为 DONE，转向 R1 (Shell-exec sandbox)
- 选择 E：其他

---

**Last updated:** 2026-06-23
**Status:** Awaiting user review