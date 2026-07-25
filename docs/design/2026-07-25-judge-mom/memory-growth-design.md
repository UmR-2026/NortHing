# judge-mom + dream 设计稿（Memory 成长闭环 C4→C5）

> 2026-07-25，编排者起草，用户四项拍板已并入。状态：**设计评审中**。
> 前置：Memory P0 已闭环（FTS5 存储 / 双写 / DB-backed 注入 / touch+decay 反馈 / per-turn query-aware 注入 / per-workspace 迁移）。
> 依据：`.opencode/memory/facts/decisions.md` "Memory 架构" 节 + 本 session 用户拍板。

## 1. 目标与哲学

- core agent 成长 + subagent 调度是中心，其它外部解耦。
- judge 从"验收裁判"升级为**记忆编排者（judge-mom）**：评估 + 路由 + 时机自学习。
- 写入全异步，用户无感；原始审计轨迹只增不改（对齐编排者记忆纪律）。

## 2. 用户拍板（2026-07-25，本 session）

| # | 决策点 | 拍板 |
|---|---|---|
| D1 | 第一刀 | **LLM 蒸馏替代关键词触发**（judge-mom 复用同一蒸馏通道） |
| D2 | 后台模型选型 | **新增专用配置项**（config 加字段，默认回落主模型） |
| D3 | 存储隔离 | **同 DB 独立表 + 访问层固化**（core agent 工具面不暴露 judge 表） |
| D4 | dream 清理动作 | **标 superseded 不删**，检索层过滤，可回溯 |

## 3. 总体架构

```
用户消息 ──┬── 实时轨：显式记忆意图（关键词/指令）→ 立即蒸馏 → 写 facts（High）
           │
           └── 后台轨（每 turn 结束，异步）：
                turn_persist::append_facts_entry
                  → LLM 蒸馏器（distiller）→ 候选 facts（JSON）
                  → dedup（精确文本 + FTS 相似）→ 双写 JSONL + MemoryDb
                  
judge-mom（后台编排者，tracer 2）：
  - 消费蒸馏产出 + judge 验收结果，评估 fact 质量（保留/降权/路由到 dream）
  - judge_mom KV 表存调度状态（上次扫描游标、时机统计）

dream（定期清理，tracer 3）：
  - 扫描 facts：过时/被推翻/长期未 touch（weight 衰减到地板 + recency 陈旧）
  - LLM 判定 → 标 superseded（不删），检索层过滤
```

### 双轨触发
- **实时轨**：用户显式（"记住/以后/prefer/always/never..."）。现关键词路径保留为实时轨的廉价前置——命中即走当 turn 蒸馏，不等后台。
- **后台轨**：每 turn 结束异步蒸馏（tracer 1）；N 轮批量 / session 结束批量留给 tracer 2 的 judge-mom 调度（时机自学习）。

### 存储隔离（D3）
- 同一个 `memory.db`，judge 侧专用表：`judge_mom`（KV，已存在）、新增 `fact_reviews`（judge 评估记录）。
- 固化方式：`memory_db.rs` 中 judge 表的方法单独 `pub(crate)` 在 `judge_memory` 子模块，core agent 的 prompt/工具面（`auto_memory.rs`、`build_*_prompt`）物理不可见该模块；boundary 规则入 `node scripts/check-core-boundaries.mjs`（tracer 2 时加）。

## 4. 数据模型变更

```sql
-- facts 表加列（ALTER TABLE，仿 text_fts 迁移模式）
ALTER TABLE facts ADD COLUMN status TEXT NOT NULL DEFAULT 'active';   -- active | superseded
ALTER TABLE facts ADD COLUMN superseded_by TEXT;                      -- 新 fact id 或 judge 备注
ALTER TABLE facts ADD COLUMN fact_type TEXT NOT NULL DEFAULT 'feedback'; -- user|feedback|project|reference（对齐 auto_memory prompt 四类型）

CREATE TABLE IF NOT EXISTS fact_reviews (
    id TEXT PRIMARY KEY,
    fact_id TEXT NOT NULL,
    reviewer TEXT NOT NULL,          -- 'distiller' | 'judge-mom' | 'dream'
    action TEXT NOT NULL,            -- 'keep' | 'downweight' | 'supersede' | 'boost'
    reason TEXT,
    created_at INTEGER NOT NULL
);
```

检索层过滤：`get_facts` / `search_facts` 的 SQL 加 `AND f.status = 'active'`（tracer 1 只需加列不过滤也可，过滤随 dream 上线；建议 tracer 1 就把过滤加上，反正默认全 active）。

## 5. Tracer 1：LLM 蒸馏通道（详细）

### 5.1 现状
- `distill_facts_from_user_message`（`facts.rs:224`）：14 个中英关键词 + 按句切分 + 截 300 字符，confidence 恒 Med、scope 恒 Workspace。
- 调用点：`turn_persist.rs:435` `append_facts_entry`（已是 turn 结束后的异步路径，失败只 warn）——**这就是蒸馏通道的家，不动生命周期**。

### 5.2 设计
- 新增 `service/agent_memory/distiller.rs`：
  ```rust
  pub(crate) async fn distill_facts_with_llm(
      user_input: &str,
      last_assistant_text: Option<&str>,   // 上轮 assistant 回复片段（截 500 字符）
      session_id: &str,
      turn_id: &str,
  ) -> Vec<Fact>   // 失败/无配置/超时 → 回落 distill_facts_from_user_message
  ```
  输入 = user_input + 上轮 assistant 片段（用户拍板：上下文更准，如"对，就这样"类确认；assistant 片段截 500 字符控成本）。
- 蒸馏 prompt（system 固化）：输入用户消息，输出 JSON 数组，最多 3 条：
  ```json
  [{"text": "...", "fact_type": "user|feedback|project|reference",
    "confidence": "high|med|low", "scope": "workspace|global"}]
  ```
  约束写入 prompt：只记跨会话有价值的事（偏好/反馈/项目动机/外部资源指针）；不记代码模式、git 历史、临时任务状态（对齐 auto_memory prompt 的 "What NOT to save"）；text ≤300 字符；无可记 → 空数组。
- **成本控制**：
  - `user_input.chars().count() < 20` 跳过；
  - 关键词命中（实时轨）当 turn 必蒸馏；未命中也蒸馏但走后台（本 tracer 简化为统一每 turn 异步一次）；
  - 超时 15s + 单次 max_tokens 上限；JSON 解析失败 → 关键词回落。
- **dedup**：本 tracer 只做现有精确文本去重（`append_facts_dedup`），FTS 近似去重**不做**，留给 judge-mom（tracer 2）统一处理（用户拍板）。
- **写入**：走现有双写路径（MemoryDb INSERT OR IGNORE + JSONL append），`fact_type` 落新列。

### 5.3 配置（D2）
`GlobalConfig`（app.json，单一事实源）新增：
```json
"memory": {
  "distiller_enabled": true,
  "distiller_model": null        // null → 回落主会话模型；否则 "provider/model" 字符串，解析失败回落主模型（用户拍板）
}
```
- 读取：`get_global_config_service`；蒸馏客户端经 `get_global_ai_client_factory` 按配置模型构造。
- Desktop `AppSettings` 侧 UI 后续 P1 再做（本 tracer 手编 app.json 即可）。

### 5.4 改动文件预估
| 文件 | 改动 |
|---|---|
| `service/agent_memory/distiller.rs` | 新建，LLM 蒸馏 + JSON 解析 + 回落 |
| `service/agent_memory/facts.rs` | Fact 加 `fact_type` 字段（serde default 兼容旧 JSONL） |
| `service/agent_memory/memory_db.rs` | facts 表加 3 列（迁移模式仿 text_fts）；检索加 status 过滤；`fact_reviews` 表 |
| `service/agent_memory/mod.rs` | 导出 distiller |
| `agentic/coordination/dialog_turn/turn_persist.rs` | `append_facts_entry` 改走 `distill_facts_with_llm` |
| `service/config/types.rs` 等 | GlobalConfig 加 memory 节 |

### 5.5 测试
- distiller：mock/fake AI client 返回 JSON → 解析成 Fact（含 fact_type/confidence/scope 映射）；坏 JSON → 关键词回落；空数组 → 空。
- memory_db：旧库 ALTER 迁移幂等；status 过滤生效。
- dedup：FTS 近似命中 → touch 不新增。
- 验证基线：`cargo check --workspace` + `cargo test -p northhing-core --features product-full agent_memory` + `prompt_injection`。

## 6. Tracer 2/3 轮廓（本稿不定处方）

- **Tracer 2 judge-mom 骨架**：消费蒸馏产物 + judge 验收结果写 `fact_reviews`；judge_mom KV 存调度状态；boundary 规则固化隔离；时机自学习（记录"蒸馏命中率"：产出 facts / 蒸馏次数，据此调节后台轨频率）。
- **Tracer 3 dream**：触发 = session 结束 或 N 轮（先用 judge_mom KV 里的游标）；扫描候选 = weight 衰减到地板 且 last_mentioned_at 陈旧（如 >30d）；LLM 批量判定 → `status='superseded'` + `fact_reviews` 记录；JSONL 侧只增 superseded 标记行（审计轨迹只增）。

## 7. 雷区与纪律

- cargo 命令必带 `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`；core 测试 `--features product-full`；验收前 `cargo clean -p northhing-core`。
- 蒸馏失败**永远**不阻塞主流程（现有 append_facts_entry 已是 warn-only 语义，保持）。
- LLM 输出不可信：JSON 严格解析 + 字段白名单 + text 截断 300；蒸馏 prompt 里防注入（user_input 包在 `<user_message>` 标签内，指令只认 system）。
- 配置单一事实源 = core GlobalConfig，禁止第二份运行时可读配置（backbone invariant）。
- 远程 workspace：后台蒸馏同样跳过（本地 DB 语义，与 query-aware 注入一致）。

## 8. 开放问题（已拍板 2026-07-25）

1. ~~蒸馏输入是否带上一轮 assistant 回复~~ → **带**（assistant 片段截 500 字符）。
2. ~~FTS 近似去重阈值~~ → **本 tracer 不做**，精确文本去重即可，近似去重归 judge-mom（tracer 2）。
3. ~~`distiller_model` 引用形态~~ → **"provider/model" 字符串**，解析失败回落主模型。
