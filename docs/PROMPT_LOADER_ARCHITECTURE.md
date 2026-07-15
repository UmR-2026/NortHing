# northhing— Agent Prompt 加载架构 v3（分— + 检— + 后台 Memory Agent— > **Status**: 设计文档（待 review— > **Date**: 2026-06-17
> **Goal**: `input tokens: 73,882`— `~1-2K` 初始，由后台 Memory Agent 持续维护与检— ---

## 1. 核心思想（v3— **三大支柱**— 1. **分区加载**：启动只加载 `soul.md` + `agent.md` + `personality`
2. **检索即工具**：Skill / Agent / Memory 通过工具按需检— 3. **后台 Memory Agent**：升级现— `memory_keeper` 为持续后— agent— *并行**做记忆维护与预检— ```
┌─────────────────────────────────────────────────────────────— 前台 (— agent) ~1-2K tokens— soul.md + agent.md + personality + user_ctx + runtime— └─────────────────────────────────────────────────────────────— user message— 注入相关记忆— ┌─────────────────────────────────────────────────────────────— 后台 (Memory Agent) - 并行运行，不阻塞前台— 事件触发:— ├─ DialogTurnStarted— 预检索相关记忆，注入下一— ├─ UserMessageReceived— 语义分析，预测需要的 skill/agent— ├─ ToolCallCompleted— 提取事实，更新记忆库— └─ DialogTurnCompleted— LLM 提取长期记忆（现有逻辑— 持续任务:— ├─ 维护 skills.db / agents.db 索引— ├─ 清理过期/低置信度记忆— └─ 向主 agent 提供"记忆通道"（memory channel— └─────────────────────────────────────────────────────────────— ┌─────────────────────────────────────────────────────────────— 数据— (SQLite)— memory.db - 长期记忆（已有，增强— skills.db - 24— skill 的可检索索— agents.db - 8— agent 的可检索索— └─────────────────────────────────────────────────────────────— ```

---

## 2. v3 相比 v2 的关键升— | 维度 | v2（分— 检索） | v3— 后台 Memory Agent— |
|------|----------------|--------------------------|
| 检索触— |— agent 调用工具 | **后台预检— +— agent 按需** |
| 记忆写入 | turn 结束— debounce | **实时事件驱动 + turn 结束** |
| skill/agent 索引 | 静— DB | **后台 agent 持续维护** |
| 上下文相关— | 依赖 LLM 判断 | **后台语义预匹— * |
|— agent 负担 | 需要调检索工— | **减少：相关记忆自动注— * |
| 并行— | 串行 | **前台/后台并行** |

---

## 3. 后台 Memory Agent 设计

### 3.1 现有 `MemoryKeeperSubscriber` 的局— ```rust
// 当前：只— DialogTurnCompleted 后做 debounce LLM 提取
impl EventSubscriber for MemoryKeeperSubscriber {
 async fn on_event(&self, event: &AgenticEvent) -> northhingResult<()> {
 match event {
 AgenticEvent::DialogTurnCompleted { .. } => {
 self.handle_turn_completed(session_id, turn_id).await
 }
 _ => Ok(()), // 其他事件全忽— }
 }
}
```

**问题**— - 只监— 1 个事— - 串行执行（阻塞下一— turn— - 不做检索，只做写入
- 没有向主 agent 反馈

### 3.2— `MemoryAgent`（后— agent— ```rust
/// 后台 Memory Agent：持续维护记忆库 + 向主 agent 提供记忆通道
pub struct MemoryAgent {
 coordinator: Arc<ConversationCoordinator>,
 memory_service: Arc<MemoryService>,
 skill_index: Arc<SkillIndex>,
 agent_index: Arc<AgentIndex>,
 /// 向主 agent 注入记忆的通道
 memory_channel: Arc<MemoryChannel>,
 /// 后台任务句柄
 background_tasks: Vec<JoinHandle<()>>,
}

impl MemoryAgent {
 /// 启动后台 agent（在 init_agentic_system 中调用）
 pub fn start(self: Arc<Self>) {
 // 1. 订阅所有相关事— self.subscribe_to_events();
 // 2. 启动持续维护任务
 self.start_maintenance_loop();
 }
}
```

### 3.3 事件订阅（升级）

```rust
#[async_trait]
impl EventSubscriber for MemoryAgent {
 async fn on_event(&self, event: &AgenticEvent) -> northhingResult<()> {
 match event {
 // 新增：用户消息到达时，预检索相关记— AgenticEvent::DialogTurnStarted { session_id, user_message, .. } => {
 self.prefetch_relevant_memories(session_id, user_message).await— ;
 }
 
 // 新增：工具调用完成时，实时提取事— AgenticEvent::ToolCallCompleted { tool_name, result, .. } => {
 self.extract_facts_from_tool_result(tool_name, result).await— ;
 }
 
 // 现有：turn 完成后，LLM 深度提取
 AgenticEvent::DialogTurnCompleted { session_id, turn_id, .. } => {
 self.deep_extract_memories(session_id, turn_id).await— ;
 }
 
 _ => {}
 }
 Ok(())
 }
}
```

### 3.4 预检索（关键新能力）

```rust
impl MemoryAgent {
 /// 用户消息到达时，后台预检索相关记忆和 skill
 async fn prefetch_relevant_memories(&self, session_id: &str, user_message: &str) -> northhingResult<()> {
 // 并行执行三个检— let (memories, skills, agents) = tokio::join!(
 self.search_memories(user_message),
 self.search_skills(user_message),
 self.search_agents(user_message),
 );
 
 // 通过 memory channel 注入到主 agent 的下一— let injection = MemoryInjection {
 relevant_memories: memories,
 suggested_skills: skills,
 suggested_agents: agents,
 timestamp: Utc::now(),
 };
 
 self.memory_channel.inject(session_id, injection).await;
 Ok(())
 }
}
```

### 3.5 Memory Channel（主 agent— 后台 agent 通信— ```rust
///— agent 和后— Memory Agent 的通信通道
pub struct MemoryChannel {
 /// session_id— 待注入的记忆
 pending: Arc<Mutex<HashMap<String, Vec<MemoryInjection>>>>,
}

impl MemoryChannel {
 /// 后台 agent 调用：注入预检索结— pub async fn inject(&self, session_id: &str, injection: MemoryInjection) {
 let mut pending = self.pending.lock().await;
 pending.entry(session_id.to_string())
 .or_insert_with(Vec::new)
 .push(injection);
 }
 
 ///— agent 在构— prompt 时调用：取出并清空待注入的记— pub async fn drain(&self, session_id: &str) -> Vec<MemoryInjection> {
 let mut pending = self.pending.lock().await;
 pending.remove(session_id).unwrap_or_default()
 }
}
```

### 3.6— agent 集成

```rust
//— execution_engine.rs 构建 prompt— async fn build_prompt_with_memory_injection(&self, session_id: &str) -> String {
 let mut prompt = self.build_initial_prompt().await; // soul + agent + personality
 
 // 从后— Memory Agent 取出预检索结— let injections = self.memory_channel.drain(session_id).await;
 
 for injection in injections {
 if !injection.relevant_memories.is_empty() {
 prompt.push_str(&self.format_memories(&injection.relevant_memories));
 }
 if !injection.suggested_skills.is_empty() {
 prompt.push_str(&self.format_skill_hints(&injection.suggested_skills));
 }
 }
 
 prompt
}
```

---

## 4. 时序图：一次完整对— ```
用户输入: "帮我写一— PDF 报告"— ┌─────────────────────────────────────────────────────────────— 事件: DialogTurnStarted— ├─ [后台] MemoryAgent.prefetch_relevant_memories()— ├─ search_memories("pdf report")— [3 条相关记忆]— ├─ search_skills("pdf report")— [pdf skill]— └─ inject to memory_channel— └─ [前台] execution_engine 开始构— prompt— ├─ build_initial_prompt() ~1.9K tokens— └─ memory_channel.drain()— 取出预检索结— └─────────────────────────────────────────────────────────────— ┌─────────────────────────────────────────────────────────────— agent prompt (总计 ~3-4K tokens):— soul.md + agent.md + personality (~1.5K)— + 注入— 3 条相关记— (~500)— + 注入— pdf skill hint (~200)— + user_context (~500)— + runtime_context (~200)— └─────────────────────────────────────────────────────────────— ┌─────────────────────────────────────────────────────────────— LLM 推理— 如果需— pdf skill 完整文档:— └─ 调用 get_skill_detail("pdf")— +1.5K tokens— 执行工具 (— Write)— └─ 事件: ToolCallCompleted— └─ [后台] MemoryAgent.extract_facts_from_tool_result— └─────────────────────────────────────────────────────────────— ┌─────────────────────────────────────────────────────────────— 事件: DialogTurnCompleted— └─ [后台] MemoryAgent.deep_extract_memories— └─ LLM 深度提取长期记忆，写— memory.db— └─────────────────────────────────────────────────────────────— ```

**关键**：后— Memory Agent 和前— LLM 推理**并行**，不阻塞— ---

## 5. Token 对比

| 场景 | 当前 | v3 |
|------|------|----|
| 初始 prompt | 73K | **1.9K** |
| + 后台预注— | - | +0.7K |
| + 按需 skill 详情 | - | +1.5K（可选）|
| **总计** | **73K** | **~3-4K**— *减少 95%**）|

---

## 6. 文件结构

```
src/crates/assembly/core/src/
├── agentic/— ├── prompts/ # 新增：分区加— ├── mod.rs— ├── soul.md # 核心灵魂— ├── agent.md # 指导原则 + 检索脚本说— ├── personality/— ├── default.md— └── formal.md— ├── loader/— ├── mod.rs— ├── partitioned_loader.rs— └── memory_channel.rs # 主↔后台通信— └── search_tools/— ├── mod.rs— ├── search_skill.rs # skill 检索工— ├── search_agent.rs # agent 检索工— └── search_memory.rs # 记忆检索工— └── memory_agent/ # 新增：后— Memory Agent— ├── mod.rs— ├── agent.rs # MemoryAgent 主体— ├── prefetcher.rs # 预检索逻辑— ├── fact_extractor.rs # 实时事实提取— └── maintenance.rs # 持续维护任务— ├── service/— ├── memory_keeper/ # 保留（向后兼容）— └── subscriber.rs # 逐步迁移— memory_agent— └── data/— ├── skill_index.rs # skills.db 管理— └── agent_index.rs # agents.db 管理
```

---

## 7. 实施路线（v3— ### Phase 1: 基础设施— -2 天）

**目标**: 建立数据— + 分区加载 + 检索工— 1. 创建 `skills.db` / `agents.db`（从现有 builtin_skills 导入— 2. 实现 `PartitionedLoader`（soul + agent + personality— 3. 实现 `search_skill` / `get_skill_detail` / `search_agent` 工具
4. 实现 `MemoryChannel`（主↔后台通信基础— **验证**: Token 降到 ~5K

### Phase 2: 后台 Memory Agent— -3 天）

**目标**: 升级 memory_keeper 为持续后— agent

1. 实现 `MemoryAgent`（订阅多事件— 2. 实现 `prefetch_relevant_memories`（用户消息到达时预检索）
3. 实现 `extract_facts_from_tool_result`（实时事实提取）
4. 集成 `MemoryChannel`— `execution_engine`

**验证**: Token 降到 ~3-4K，主 agent 收到预注入记— ### Phase 3: 持续维护 + 优化— -3 天）

**目标**: 后台 agent 持续优化索引

1. 实现 `maintenance` 循环（清理过期记忆、更新索引）
2. 添加 embedding 向量检索（Phase 2 的语义检索）
3. 检索结果缓— 4. 置信度衰减（旧记忆权重降低）

**验证**: 检索质量提升，长期使用体验改善

---

## 8. 关键决策— ### 8.1 MemoryAgent— EventSubscriber 还是独立 tokio task— **建议**— *两者都— *
- 作为 `EventSubscriber` 接收事件（现有架构兼容）
- 内部 spawn 独立 tokio task 做耗时操作（不阻塞事件循环— ```rust
impl MemoryAgent {
 async fn on_event(&self, event: &AgenticEvent) -> northhingResult<()> {
 let this = self.clone();
 // 耗时操作放到独立 task
 tokio::spawn(async move {
 match event {
 DialogTurnStarted { .. } => this.prefetch().await,
 ToolCallCompleted { .. } => this.extract_facts().await,
 _ => {}
 }
 });
 Ok(())
 }
}
```

### 8.2 预检索用什么算法？

**Phase 1**: 关键词匹配（TF-IDF / BM25— **Phase 2**: embedding 向量（用 small model 本地推理— ### 8.3— agent 如何知道有预注入的记忆？

两种方式— - **A. 自动注入**：`execution_engine` 在构— prompt 时自— `drain` memory channel
- **B. 工具查询**：主 agent 调用 `get_pending_memories()` 工具

**建议**— *A（自动）**，减少主 agent 负担

### 8.4 现有 memory_keeper 怎么处理— **迁移策略**— - Phase 1: 保留 memory_keeper（向后兼容）
- Phase 2: MemoryAgent 接管 memory_keeper 的职— - Phase 3: 移除 memory_keeper

---

## 9. 风险评估

| 风险 | 等级 | 缓解 |
|------|------|------|
| 后台 agent 拖慢系统 |— | 限制并发数，— spawn 不阻— |
| 预检索结果不— |— | Phase 1 关键词，Phase 2 向量 |
| Memory channel 竞— |— |— tokio::sync::Mutex |
|— agent 行为变化 |— | 自动注入是增量的，不删原有信— |
| 数据库迁— |— | 自动— builtin_skills 导入 |

---

## 10. 待用户确— 1. **架构方向对吗— * 后台 Memory Agent 并行预检— +— agent 分区加载
2. **MemoryAgent— EventSubscriber + spawn task 模式— *
3. **— agent 自动注入记忆（方— A）？**
4. **— Phase 1 开始？**（先建数据库和分区加载，再升级后— agent— 5. **现有 memory_keeper 保留多久— *（Phase 2 后移除）

---

## 附录：核心数据流

```
 ┌─────────────────— User Input— └────────┬────────— ┌────────▼────────— EventRouter— DialogTurnStarted— └───┬────────┬────— ┌────────────▼──— ┌──▼───────────────— [后台] Memory— [前台] Execution— Agent— Engine— prefetch:— build prompt:— - memories— - soul.md— - skills— - agent.md— - agents— - personality— inject ───────— ◄── drain— └───────┬───────— memory_channel— └──────┬───────────— ┌───────────────— ┌───────────────— memory.db— LLM 推理— skills.db— + 工具调用— agents.db— └───────┬───────— └───────────────— ┌─────────────────— ToolCallCompleted— DialogTurnDone— └────────┬────────— ┌────────▼────────— [后台] Memory— Agent— - extract facts— - deep extract— - update index— └─────────────────— ```

简洁、并行、可扩展—