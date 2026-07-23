# Memory 检索层设计

## 概述

northing 的 agent 记忆检索从"全量 top-N 注入"升级为"query-aware 关键词加权检索"。语义理解在写入时由 judge 完成，检索时纯 FTS5，零 LLM 成本。

## 现状问题

- facts.jsonl 按 recency/confidence 排序，top-N 全量注入（1000 token）
- 不相关：用户聊代码时注入无关偏好
- 不精准：exact-text dedup 无法识别语义重复
- 不进化：重复提及不增加权重

## 架构

### 写入时（judge 异步处理）

1. judge 收到记忆候选
2. 提取关键词
3. 查询 keyword_weights 表，检查是否有关联关键词已存在
4. 如有关联：boost 相关关键词权重（不合并 fact）
5. 如无关联：新 fact 入库 + 关键词以基础权重入表
6. judge 写入 related_keywords（语义关联词组）

### 检索时（主 agent 每轮触发）

1. 用户消息到达 → 提取查询关键词
2. FTS5 MATCH 查询（关键词匹配）
3. 乘以 keyword_weight（预计算权重）
4. 乘以 recency_boost（时间衰减）
5. top-K within token budget → 注入 prompt

### 检索公式

score = BM25(query, fact_text) × keyword_weight × recency_boost

- BM25：FTS5 内置关键词匹配度
- keyword_weight：judge 写入时更新的语义权重
- recency_boost：1.0 + 0.1 × (1 / days_since_last_mention)

## 存储

config_dir()/northhing/memory/memory.db（SQLite）

### 表结构

facts:
- id TEXT PRIMARY KEY
- text TEXT
- scope TEXT (workspace/global)
- confidence TEXT (high/med/low)
- created_at INTEGER
- last_mentioned_at INTEGER

facts_fts (FTS5 虚拟表):
- text 分词索引

keyword_weights:
- keyword TEXT PRIMARY KEY
- weight REAL DEFAULT 1.0
- mention_count INTEGER DEFAULT 1
- last_boosted_at INTEGER
- related_keywords TEXT (JSON array)

judge_mom (隔离表，core agent 不可查):
- key TEXT PRIMARY KEY
- value TEXT
- updated_at INTEGER

## 权重进化规则

| 事件 | 权重变化 |
|---|---|
| 用户再次提及同义内容 | weight += 0.5, mention_count++ |
| judge 识别语义关联 | related_keywords 互相关联，共享 boost |
| 注入后 agent 实际使用 | weight += 0.2（正反馈） |
| 注入后完全没被引用 | weight -= 0.1（衰减，不低于 0.1） |
| 时间衰减 | 每 30 天 weight *= 0.9 |
| 用户说"别记这个" | weight = 0（标记忽略） |

## 分期

| Phase | 内容 |
|---|---|
| P0 | SQLite FTS5 + keyword_weights + query-aware 检索 + 反馈循环 |
| P1 | flat vector search（embedding + cosine，记忆量增长后加） |
| P2 | HNSW（hnsw_rs，记忆破万后替换 flat search） |

## 隔离

- core agent 的 Rust 模块只暴露 facts_* 和 keyword_weights 的查询接口
- judge_mom 表由 judge 专用模块访问
- prompt builder 永远不查 judge_mom
- 结构层保证：不同 Rust module 持有不同 DB 连接/handle

## 与多 agent 架构的关系

- 主 agent：每轮触发检索（纯 FTS，零 LLM 成本）
- memory-judge：写入时做语义加权（LLM 驱动，异步）
- memory-writer：执行 judge 指派的写入操作
- dream：judge 定期全量扫描，合并关联/降权过时/删除无用
