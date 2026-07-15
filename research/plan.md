# 调研计划：NortHing（纳森）项目理念与代码在开源 Agent 市场的竞争力

**Date**: 2026-06-25
**Topic**: 项目理念与代码在开源 Agent 市场的竞争力
**Route**: B（Focused Search）— 有清晰维度，无需广泛搜索
**Output**: `E:/agent-project/agent-app/research/`

---

## 项目背景（供子agent参考）

- **项目名称**：NortHing（纳森）
- **技术栈**：Rust（21个workspace crate），CLI + GUI（Slint）
- **核心理念**：
 - 黑盒式 vibecoding：编码功能对人类隐藏，用户不直接面对代码
 - 永不停歇的对话机器：超越 session 的上下文延续（驱力/Drive）
 - 多模型协作：Plan（MiniMax 3）+ Code（Kimi 2.7）+ Fast（MiniMax 2.7）
 - 自进化：技能系统、工具折叠、上下文压缩自适应
 - 无代码用户目标：用户没有代码基础，Orchestrator 角色
- **架构特点**：
 - Actor/Dispatcher 运行时（A2 轻量路径）
 - 子agent 嵌套 + 会话关系系统（Parent/Child/Sibling）
 - 背景 agent（fire-and-forget）
 - 工具折叠（ToolExposure::Collapsed）
 - 提示词缓存分层（L2/L3）
 - 执行引擎分步 tick（init_turn → tick → finalize_turn）
- **当前状态**：v0.1.0，Rust 代码库，21 crate workspace

---

## 维度分解（5个核心维度）

### 维度1：终端编码 Agent 直接竞品
**范围**：Claude Code, Aider, Cline, OpenCode, Goose, Kilo Code, Codex CLI
**角度**：
- 这些工具的架构模式（CLI vs IDE vs 云）
- 子agent 实现方式（Kimi Code 的 LaborMarket 模式 vs 你的 Actor 模式）
- 是否支持无代码用户
- 是否支持多模型切换
- 会话连续性机制
- 安装包大小和部署方式

### 维度2：编排框架竞品
**范围**：LangGraph, CrewAI, AutoGen, Dify, OpenAI Agents SDK, MetaGPT
**角度**：
- 多agent 协作模型（图编排 vs 角色扮演 vs 事件驱动）
- 状态持久化能力
- 是否支持嵌套 agent
- 是否支持后台任务
- 黑盒 vs 白盒执行
- 对无代码用户的友好度

### 维度3：Vibe Coding / 无代码竞品
**范围**：Lovable, Bolt.new, Replit, v0, Cursor（非工程师视角）
**角度**：
- 目标用户画像（无代码 vs 有代码基础）
- 代码可见性策略（完全黑盒 vs 可查看 vs 必须编辑）
- 会话/项目延续性
- 部署和交付方式
- 自进化能力（自适应、技能学习）

### 维度4：市场痛点与未满足需求
**范围**：开源 agent 的共性问题
**角度**：
- 短期记忆溢出和上下文丢失
- 工具调用幻觉和安全性
- 多模型协作的复杂度
- 无代码用户的门槛
- 会话连续性（session 中断后的恢复）
- 自进化/自适应能力缺失
- 安装包大小和部署门槛

### 维度5：多模型协作与无代码定位的差异化
**范围**：市场中是否存在类似定位的产品
**角度**：
- 是否有"Plan + Code + Fast"三模型分工的开源实现
- 是否有面向无代码用户的开源 agent 工具
- 是否有强调"黑盒执行"的产品
- 是否有"永不停歇/超越 session"的产品
- 是否有"驱力/自进化"理念的产品
- 市场空白：你的独特定位（纳森 = 拉康驱力 + 克苏鲁黑盒 + 无代码 vibecoding）

---

## 执行计划

### Phase 1：并行深度调研（5个子agent）
- 每个子agent负责一个维度
- 每个子agent ≥10次搜索
- 输出到 `research/northhing_dim{NN}.md`

### Phase 2：交叉验证（Orchestrator）
- 读取所有维度文件
- 分类置信度
- 识别— 突
- 输出 `research/northhing_cross_verification.md`

### Phase 3：洞察提取
- 跨维度分析
- 提取非显而易见的洞察
- 输出 `research/northhing_insight.md`

### Phase 4：报告整合
- 将洞察整合为竞争力分析报告
- 评估 NortHing 的独特定位、优势、劣势、机会、威胁
- 给出策略建议
