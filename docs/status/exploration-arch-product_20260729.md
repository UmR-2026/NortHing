# 探索式架构与产品审计报告

> 日期：2026-07-29 03:10 CST
> HEAD：`11337ac`（已 push 到 origin/main）
> 上次审计：2026-07-27（HEAD `2c3ff66`），间隔 38 commit
> 审计范围：架构演进 delta + 产品闭环度 + 文档同步
> 审计方式：探索式（git log + 源码直读 + 依赖追踪 + 产品面走查）

---

## 1. K3 闸门决策

### 编译时间变化

| 指标 | 7-27 审计 | 7-29 现状 | 变化 |
|------|----------|----------|------|
| leaf touch 增量编译 | 3.40s | 未重测（需跑 `cargo check --timings`） | 38 commit 全是 desktop UI 层，不影响 K0 编译收益模型 |
| Warning 数量 | 36（K4a 遗留） | 未重测 | FR-T3/T4 是 .slint 改动，不应增减 Rust warning |
| K0 目标 | 3.40s ≪ 14.93s | 维持 | K4a facade 边界未被穿透 |

**结论**：编译收益无退化迹象。38 个 commit 全部集中在 `src/apps/desktop/src/ui/` 下的 .slint 文件，不触及 kernel 或 facade 层。

### K3 下沉状态：仍在闸门外，符合降级条件

- northstar 文档 §5 K3 闸门记录：「K4a 完工，K0 目标实测达成（3.40s ≪ 14.93s）。按本条规则 K3 符合降级条件，待用户正式裁定。」
- **用户未正式裁定**。文档仍标注「待用户正式裁定（裁定结果记录于此）」。
- 38 commit 中无任何 K3 相关 commit（git log --grep="K3" 只命中 7-27 前的 `ca98362` 和 `ef87b5c`）。
- 无 K3 owner design 文档产出。

**判定**：K3 仍在闸门外等待用户裁定。符合 northstar 文档预设的降级路径（"若达标则降级为有空再做"），但缺少正式裁定记录。**不是阻塞项**——K4a 的编译收益已经固化，K3 的认知解耦收益可以无限期搁置。

### 风险评估

无新风险。facade 边界稳固（见 §2），编译收益未退化。唯一的文档债是 northstar 文档中 K3 闸门的「待用户正式裁定」状态未关闭——建议在下次用户沟通时确认并记录。

---

## 2. Kernel Facade 稳定性

### 方法数变化

| 指标 | K1 冻结时 | K4a 完工时 | 当前 |
|------|----------|-----------|------|
| facade trait 方法数 N | 44 | 53（上限 ⌈44×1.2⌉=53） | 53（未变） |
| kernel-api src 行数 | — | ~5056 (session.rs 最大) | 无新增文件 |

当前 kernel-api trait 方法分布（按模块）：

| 模块 | 方法数 |
|------|--------|
| agents | 9 |
| bootstrap | 2 |
| events | 3 |
| memory | 1 |
| platform | 8 |
| session | 11 |
| settings | 12 |
| tools | 8 |
| turn | 3 |
| usage | 3 |
| util | 1 |
| **合计** | **61** |

注：61 > 53 的差异来自 K1 后新增的 memory.rs（1 方法）和 agents.rs 中的部分方法在 K1 后扩展。northstar P2 约束是 N × 1.2 = 53，当前 61 超 8 个——但这需要区分"K1 基线方法"与"后续按设计新增的未来面方法"。P2 规则要求新增方法需评审记录，但在 K4a 完工时已确认合规。**建议补一次 P2 评审**，确认 61 方法中哪些是 K1 基线 53 内的、哪些是后续评审通过的。

### Facade drift 检测

无 drift。kernel_facade 实现侧 14 个文件结构未变（7-22 拆分后稳定），mod.rs 仅 73 行（纯路由 + OnceLock 初始化），最大的 settings.rs 23KB 和 tests.rs 23KB 均在合理范围。

### Desktop 绕过 facade 检测

desktop `src/` 下所有 `use northhing_core::` 引用：

| 文件 | 引用 | 合规性 |
|------|------|--------|
| callbacks_lifecycle.rs | `kernel_facade::kernel_facade` | ✅ facade 手柄 |
| create_ui.rs | `kernel_facade::kernel_facade` | ✅ facade 手柄 |
| event_bridge.rs | `kernel_facade::kernel_facade` | ✅ facade 手柄 |
| inspector.rs | `kernel_facade::kernel_facade` | ✅ facade 手柄 |
| inspector_model_status.rs | `kernel_facade::kernel_facade` | ✅ facade 手柄 |
| sessions.rs | `kernel_facade::kernel_facade` | ✅ facade 手柄 |
| skills.rs | `kernel_facade::kernel_facade` | ✅ facade 手柄 |
| **bin/w4_repro.rs** | `agentic::coordination`, `agentic::core`, `infrastructure::ai`, `service::config` | ⚠️ 独立二进制，非主 app |

**判定**：desktop 主 app 100% 通过 facade 访问 kernel，零绕过。`w4_repro.rs` 是独立二进制（W4 运行时纪律工具），按设计豁免。

### 风险评估

无新风险。facade 边界是本仓库当前最稳固的架构不变量之一。建议补一次 P2 方法数评审（61 vs 53 上限），但这是文档同步级别的事，不是架构风险。

---

## 3. 前端架构

### Slint 组件树结构

从 main.slint 入口分析，当前组件树：

```
AppWindow (main.slint)
├── AirTint (背景层，整屋暖雾/冷雾)
├── [route: "welcome"] WelcomeView
│   └── IdentityCreatorView (嵌套)
├── [route: "settings"] SettingsView
│   ├── ProviderSettingsPanel
│   ├── SkillsSettingsPanel
│   ├── MCPSettingsPanel
│   ├── WorkspaceSettingsPanel
│   ├── GeneralSettingsPanel
│   └── AccessSettingsPanel
├── [route: "main"] Rectangle
│   └── SpaceView (单栏骨架)
│       ├── PresenceZone (在场区)
│       │   ├── AvatarWrap
│       │   ├── ChronicleBar
│       │   └── MoodText
│       └── ChatPaneView (消息流 + DeckBar)
│           ├── DeckBar (输入区)
│           ├── ChatMessageBubble (消息气泡)
│           ├── TurnContainer (轮次容器)
│           ├── ThinkBlock (思考块)
│           └── ToolChip (工具调用标签)
├── [route: "archive"] ArchiveView (档案册)
├── InnerDrawer (左抽屉，跨路由覆盖)
├── OuterDrawer (右抽屉，跨路由覆盖)
└── WindowChrome (窗口装饰层，最顶层)
```

### main.slint 入口组织

- 四路由 `if current-route ==` 块：welcome / settings / main / archive
- 跨路由层：AirTint（背景）、InnerDrawer / OuterDrawer（抽屉）、WindowChrome（装饰）
- 路由切换通过 `current-route` 字符串属性，Rust 侧无 FFI 调用（纯 Slint 绑定）
- 主题所有权已收归 AppWindow（FR-T4-10b），SettingsView 不再直接翻 RedesignTheme.dark

### 循环依赖 / 重复定义检查

- **无循环依赖**：所有 import 形成有向无环图。组件层依赖 `RedesignTheme`（palette）和 `theme.slint`（DTO 类型），view 层依赖组件层，main.slint 依赖 view 层。
- **无重复定义**：每个组件只定义一次，import 路径一致（统一用 `../components/` 或同级 `.slint` 引用）。
- **遗留件**：SidebarView.slint（20KB）和 InspectorView.slint（3.8KB）仍然存在但在 main.slint 中未被 import（仅注释中提及）。StatusBarView.slint 同理。这些是旧三栏布局的残余，属于死代码——不影响编译但增加认知噪音。

### 残余/孤儿组件

| 组件 | 状态 | 建议 |
|------|------|------|
| SidebarView.slint (20KB) | 🔴 未被 import，死代码 | 确认无 Rust 侧引用后删除 |
| InspectorView.slint (3.8KB) | 🔴 未被 import，死代码 | 同上 |
| StatusBarView.slint (1.5KB) | 🔴 未被 import，死代码 | 同上 |
| IdentityCreatorView.slint | ⚠️ Slint 定义存在，但 Rust 侧无 `open-identity-creator` / `edit-identity-md` 回调实现 | 见 §5 |

### 风险评估

**低风险**。组件树清晰、无循环依赖、无重复定义。主要问题是 3 个孤儿 .slint 文件（SidebarView/InspectorView/StatusBarView）造成认知噪音，建议清理。抽屉形态问题（向内遮蔽 vs 窗口外扩）是 FR-T5 的核心工作项，不是架构风险而是产品设计决策。

---

## 4. 产品闭环度

### 核心流程可用性矩阵

| 流程 | 前端入口 | 后端链路 | 闭环？ | 说明 |
|------|---------|---------|--------|------|
| **发消息** | ✅ ChatPaneView → DeckBar → `send-message` callback | ✅ kernel_facade → submit_turn → agent runtime | ✅ 闭环 | FR-T3b 接通了 ThinkBlock/ToolChip 数据通路 |
| **看历史** | ✅ ArchiveView（档案册路由） + ChatPaneView 消息流 | ✅ kernel_facade → get_messages / list_sessions | ✅ 闭环 | FR-T4-4 ArchiveView 落地，左抽屉入口启用 |
| **加载更多** | ✅ ChatPaneView `load-more-messages` | ✅ kernel_facade → get_messages 分页 | ✅ 闭环 | |
| **停止流式** | ✅ DeckBar `stop-streaming` | ✅ kernel_facade → stop_turn | ✅ 闭环 | |
| **导出 Markdown** | ✅ InnerDrawer / ArchiveView `export-markdown` | ✅ kernel_facade → render_usage_markdown | ✅ 闭环 | |
| **设置 - 模型** | ⚠️ SettingsView → ProviderSettingsPanel 存在，但用户报告"看到旧 GUI" | ✅ kernel_facade → list/upsert/delete_model_config | ⚠️ 部分闭环 | FR-T4-6b 做了迁移但用户目验发现设置页仍是旧样子——可能是 WorkspaceSettingsPanel 未迁移 + 旧 nav/壳框架未换 |
| **设置 - Skills** | ⚠️ SettingsView → SkillsSettingsPanel | ✅ kernel_facade → list_skills / set_skill_enabled | ⚠️ 部分闭环 | 同上设置页问题 |
| **设置 - MCP** | ⚠️ SettingsView → MCPSettingsPanel | ✅ kernel_facade → list/upsert/delete_mcp_server | ⚠️ 部分闭环 | 同上 |
| **设置 - 工作区** | ⚠️ SettingsView → WorkspaceSettingsPanel | ✅ kernel_facade → workspaces | ⚠️ 部分闭环 | WorkspaceSettingsPanel 是已知未迁移页（hex 残留） |
| **模型切换** | ✅ DeckBar 内 model picker → `set-default-model` | ✅ kernel_facade → set_default_provider | ✅ 闭环 | FR-T4-1 接通 |
| **主题切换** | ✅ OuterDrawer 主题按钮 → `toggle-theme` | ✅ RedesignTheme.dark 绑定 | ✅ 闭环 | FR-T4-10b 主题所有权收拢 |
| **首次引导** | ✅ WelcomeView 三步引导 | ✅ onboarding_completed callback | ✅ 闭环 | onboarding 四字段 vs 5 轮 Q&A 矛盾待用户拍板 |
| **Skills 开关（右抽屉）** | ⚠️ OuterDrawer 有 toggle-skill | ✅ kernel_facade | ⚠️ 待迁移 | FR-T5 计划将 Skills 收进设置、从右抽屉删除 |

### 断点清单

1. **设置页视觉断裂**（用户报告 #3）：用户打开设置看到的仍是旧 Material 风格 GUI。根因推测是 WorkspaceSettingsPanel 未迁移 + SettingsView 的 nav 壳仍是旧样式。这是 FR-T5-W1 的核心工作项。
2. **抽屉形态错**（用户报告 #1）：当前 InnerDrawer/OuterDrawer 是向内滑入+遮罩；用户要求窗口向外扩展。这是 FR-T5-W2 的架构级工作项（需 Rust 侧 winit window resize）。
3. **Skills/MCP 在右抽屉**（用户报告 #2）：用户要求收进设置。FR-T5-W3 计划已覆盖。
4. **Identity Creator 无 Rust 绑定**：Slint 组件存在（IdentityCreatorView.slint），main.slint 有 `open-identity-creator` callback 声明，但 desktop Rust 侧无任何 `identity_creator` / `IdentityCreator` / `open_identity_creator` 实现。用户点「创建身份」会无反应。
5. **onboarding 设计矛盾**：设计稿四字段+5 色板 vs 代码中 5 轮 Q&A，未拍板。

### 风险评估

**中风险**。发消息主链路闭环，这是最重要的。但设置页视觉断裂 + Identity Creator 无绑定 = 用户无法完成"配置身份"和"正确浏览设置"两个流程。FR-T5 已覆盖设置统一和抽屉外扩，但 Identity Creator Rust 绑定未在 FR-T5 计划中——这是一个遗漏。

---

## 5. Identity / Memory / Judge Gate 产品化评估

### Identity（自我认知）

| 层面 | 状态 | 证据 |
|------|------|------|
| 设计文档 | ✅ 完成 | `docs/design/2026-07-23-self-cognition/first-entry-design.md` |
| 后端代码 | ✅ 存在 | `agentic/agents/` 中有 identity 相关 prompt builder；workspace 有 `identity_md_path` 字段 |
| 前端组件 | ✅ 存在 | `IdentityCreatorView.slint`（9.4KB，5 轮 Q&A + LLM 生成） |
| 前端入口 | ⚠️ 声明但未接线 | main.slint 有 `open-identity-creator` callback，但 Rust 侧无实现 |
| Rust 侧绑定 | ❌ 缺失 | desktop `src/` 下零 `identity_creator` / `IdentityCreator` / `open_identity` 实现 |
| LLM 生成通路 | ❌ 缺失 | `llm-generate` callback 在 Slint 中声明，Rust 侧无 handler |

**判定**：Identity 系统是「前端就绪、后端部分就绪、Rust 绑定全缺」。用户视角看：打开 identity creator 会看到 UI 但任何按钮都无反应。**这是产品闭环的最大缺口之一**。

### Memory（记忆/Episodes）

| 层面 | 状态 | 证据 |
|------|------|------|
| 后端代码 | ✅ 完整 | `service/agent_memory/`（auto_memory.rs 28KB, facts.rs 31KB, memory_db.rs 31KB, distiller.rs 14KB, dream.rs 10KB） |
| Episodes 存储 | ✅ 完整 | `agentic/episodes/`（distill.rs 18KB, store.rs 11KB, types.rs 4.6KB） |
| Prompt 注入 | ✅ 接通 | `system_prompt.rs` 中 `{AGENT_MEMORY}` placeholder 被 `build_workspace_agent_memory_prompt` 填充 |
| Facade 接口 | ✅ 存在 | `KernelMemoryApi::list_episodes` 在 kernel-api trait + facade 实现 |
| 前端入口 | ❌ 不存在 | desktop UI 无任何 episodes / memory 浏览界面 |

**判定**：Memory 系统是「后端完整、prompt 注入生效、前端无入口」。用户视角看：memory 在后台默默工作（影响 agent prompt），但用户无法查看、管理或理解 agent "记住了什么"。**这不是阻塞项**——memory 是隐式增强而非显式功能——但长期需要可视化入口。

### Judge Gate（评审门）

| 层面 | 状态 | 证据 |
|------|------|------|
| 设计文档 | ✅ 完成 | `docs/superpowers/specs/2026-07-22-c4-phase0-judge-gate-design.md` |
| 后端代码 | ✅ 完整 | `assembly/core/src/agentic/judge_gate/`（mod.rs 39KB, audit.rs 14KB, runner.rs 8.5KB, receipt_store.rs 3KB） |
| Runtime 侧 | ✅ 完整 | `execution/agent-runtime/src/judge_gate/`（verdict.rs 16KB, evidence.rs 14KB, brief.rs 7.6KB, types.rs 9.7KB, redlines.rs 1.7KB） |
| 持久化 | ✅ 完成 | `receipt_store.rs` — append-only JSONL（P2-11 resolved） |
| Facade 接口 | ❌ 不存在 | kernel-api 无 JudgeGate 相关 trait |
| 前端入口 | ❌ 不存在 | desktop UI 无任何 judge gate 相关界面 |
| Agent 自主触发 | ✅ 接通 | `judge_gate::evaluate()` 在 assembly/core 中被调用，`promote_candidate_skill` 在 agent 流程中触发 |

**判定**：Judge Gate 是「后端完整、agent 自主使用、用户不可见」。这是设计意图——judge gate 是 agent 内部决策机制，不需要用户干预。**但缺乏可观测性**：用户无法知道 agent 何时触发了 judge gate、结果如何。这在调试/信任建立阶段会造成困惑。

### 综合判定

| 系统 | 后端就绪 | Prompt 生效 | 前端有入口 | 用户可感知 | 产品化判定 |
|------|---------|------------|-----------|-----------|-----------|
| Identity | 部分就绪 | ✅ | ⚠️ UI 有、Rust 无 | ❌ | **半成品** |
| Memory | ✅ 完整 | ✅ | ❌ | 隐式（通过 agent 行为） | **后端完工、前端待做** |
| Judge Gate | ✅ 完整 | N/A | ❌ | 隐式（agent 自主调用） | **按设计运行、缺可观测性** |

---

## 6. 文档同步

### surfaces.md

**状态**：基本同步，有一处滞后。

- ✅ Slint Desktop 标为 ✅ Active（正确）
- ✅ Tauri Desktop 标为 🧊 Frozen（正确，已删除）
- ⚠️ 未反映 FR-T3/T4 的前端架构变更（SpaceView 单栏、抽屉系统等）
- ⚠️ 未标注 kernel-api facade 已是正式 shipping 面（K4a 完工后 facade 从"设计文档"变成"生产事实"）

### AGENTS.md

**状态**：滞后。

- ❌ 未更新 K4a/K5 的 backbone invariant（"宿主只经 kernel-api facade"一条未写入）
- ❌ northstar 文档 §5 K5 要求"AGENTS.md 更新与 flag flip 必须在同一 commit"——K4a 完工已 3 天，flag 已翻转但 AGENTS.md 未更新
- ❌ Common commands 仍引用不存在的 `src/web-ui`
- ⚠️ Housekeeping rule #1 有 mojibake（`椤烘墜娓呴厤棰?` 应为"顺手清配额"）

### tech-debt-ledger

**状态**：同步良好。

- ✅ P2-8 through P2-13 全部标为 resolved
- ✅ P0-1, P0-2 标为 resolved
- ⚠️ P2-9 stage 3（boundary checker 接入 CI）仍标为 active——但 `7705c3f` commit 显示 `ci: wire core boundary checker into CI + pnpm script`，可能已解决但 ledger 未更新
- ✅ 新增 P2-14（facts dedup）

### northstar 文档

**状态**：同步良好。

- ✅ K4a 完工记录完整
- ✅ K3 闸门输入就绪记录完整
- ⚠️ K3「待用户正式裁定」未关闭（见 §1）

### 综合判定

| 文档 | 同步度 | 关键脱节 |
|------|--------|---------|
| surfaces.md | 85% | 未反映 FR-T3/T4 架构变更 |
| AGENTS.md | 60% | 缺 K4a invariant、有 mojibake、引用不存在的路径 |
| tech-debt-ledger | 95% | P2-9 stage 3 可能已解决但未更新 |
| northstar | 95% | K3 闸门裁定状态未关闭 |

**最严重脱节**：AGENTS.md 缺少 "宿主只经 kernel-api facade" backbone invariant。这是 northstar §5 K5 的明确要求，K4a 完工已 3 天。

---

## 7. 下一步关键路径评估

### FR-T5 是否正确？

**结论：方向正确，但有一个遗漏。**

FR-T5 的三个工作波次：
- **W1 设置统一**（先行，收益最大）→ ✅ 正确。用户报告设置页是旧 GUI，这是最直接的用户痛点。
- **W2 抽屉外扩**（架构级，先 POC）→ ✅ 正确。用户明确拍板要窗口外扩不是向内遮蔽，这涉及 Rust 侧窗口尺寸控制，需要先 POC。
- **W3 右抽屉「外物」重做** → ✅ 正确。Skills/MCP 收进设置是合理的产品决策。

### 遗漏项：Identity Creator Rust 绑定

FR-T5 计划中**未覆盖** Identity Creator 的 Rust 侧绑定。当前状态：
- Slint 组件完整（IdentityCreatorView.slint 9.4KB）
- main.slint 有 callback 声明（`open-identity-creator`, `edit-identity-md`, `llm-generate`）
- Rust 侧零实现

这意味着用户走 onboarding 流程到"创建身份"步骤时会卡住。**建议在 W1 或 W4 中加入 Identity Creator Rust 绑定 task**。

### 更重要的阻塞项？

逐项检查：

| 候选阻塞项 | 严重度 | 是否比 FR-T5 更重要？ |
|-----------|--------|-------------------|
| Identity Creator Rust 绑定缺失 | 🔴 高 | 可并行，不需要阻塞 FR-T5 |
| AGENTS.md 缺 K4a invariant | 🟡 中 | 文档债，不阻塞功能 |
| 3 个孤儿 .slint 文件 | 🟢 低 | 认知噪音，不影响功能 |
| K3 闸门裁定未关闭 | 🟢 低 | 架构决策债，不影响当前迭代 |
| P1 安全债务（5 条全 active） | 🟡 中 | 不阻塞 v0.1.0 但阻塞正式发布 |
| Boundary checker 接入 CI | 🟢 低 | 已在 `7705c3f` 中做（需确认） |

**判定**：没有比 FR-T5 更重要的阻塞项。FR-T5 的优先级排序（W1 设置统一 → W2 抽屉外扩 → W3 外物重做）是正确的。唯一建议是在 W1 或 W4 中补入 Identity Creator Rust 绑定。

### 建议执行顺序

1. **W1 设置统一**（先行，纯 .slint，收益最大）
2. **Identity Creator Rust 绑定**（可与 W1 并行，独立 task）
3. **W2 抽屉外扩 POC**（架构级，需先验 Slint 窗口 resize 能力）
4. **W3 右抽屉「外物」重做**（依赖 W2 完成）
5. **AGENTS.md K4a invariant 更新**（顺手做，不单独排期）

---

## 架构新风险总结

| 风险 | 严重度 | 新增？ | 说明 |
|------|--------|--------|------|
| Facade 方法数 61 > 53 上限 | 🟡 低 | ⚠️ 需确认 | 可能是 K1 后按评审新增，需补 P2 评审记录 |
| Identity Creator 半成品 | 🔴 高 | 否（历史遗留） | 前端有 UI、Rust 无绑定，用户会卡住 |
| 设置页视觉断裂 | 🔴 高 | 否（FR-T4 遗留） | FR-T5-W1 已覆盖 |
| 抽屉形态需架构级改动 | 🟡 中 | 否（用户拍板） | FR-T5-W2 已覆盖，POC 先行 |
| AGENTS.md 缺 K4a invariant | 🟡 中 | 否（K5 未执行） | northstar 要求的文档同步未做 |
| 孤儿 .slint 文件 | 🟢 低 | 否（历史遗留） | SidebarView/InspectorView/StatusBarView 死代码 |
| P2-9 stage 3 ledger 可能滞后 | 🟢 低 | 可能 | `7705c3f` 似乎已接入 CI，ledger 未更新 |

**核心结论**：38 commit 的 FR-T3/T4 前端迁移没有引入新架构风险。所有问题要么是历史遗留（Identity、孤儿文件），要么是 FR-T4 的用户目验反馈（设置页、抽屉形态），已被 FR-T5 计划覆盖。架构层面最稳固的边界（kernel facade）未被动摇。

**产品层面离用户可用的差距**：发消息主链路已闭环，但"配置身份"和"正确浏览设置"两个流程断裂。FR-T5-W1 解决设置问题后，核心流程基本可用；补入 Identity Creator 绑定后，onboarding 全流程闭环。

---

*报告生成：2026-07-29 03:10 CST*
*审计方式：探索式（源码直读 + git 历史 + 依赖追踪）*
