# 代码层腐化深审报告：memory_db.rs + lsp/manager.rs

> **审查口径**：只读审查（不改代码不 commit），8 项量规逐项枚举。结构层前情见 `rot-probe-2026-08-28.md`。

---

## 文件 1：`memory_db.rs`（918 行）

### 量规 1：死代码

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 1-腐化 | `search_facts` 行 542: `let bm25_pos = -rank;` 计算后丢弃，`ScoredFact.bm25` 存 raw `rank`（负值），`bm25_pos` 正数变量从未使用。死计算 + 命名暗示意图差异（`bm25_pos` 应该是存进 struct 的值） | l:542, l:547-553 | 腐化证据 |
| 1-观察 | `get_facts` 行 291: `last_mentioned_at` 从 9 字段行中解构，但 `Fact` 结构体重建（l:330-342）从未赋值该字段。死提取——行尾 Lint 不会报（变量已绑定），但 `last_mentioned_at` 数据静默丢弃 | l:291, l:330-342 | 观察项 |

### 量规 2：重复

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 2-腐化 | `get_facts` stmt 构造 Some/None 分支复制 17 行（l:236-252），query_map 闭包复制 34 行（l:254-287），两个分支只在 `params![ws]` vs `params![]` 一处不同 | l:236-287 | 腐化证据 |
| 2-腐化 | `search_facts` stmt 构造复制 27 行（l:404-430），query_map 闭包复制 36 行（l:434-469），同样仅在 `params` 参数数量不同 | l:404-469 | 腐化证据 |
| 2-腐化 | 字符串→枚举转换块在 `get_facts`（l:294-328）和 `search_facts`（l:481-515）完全一致，三块 match（scope/confidence/fact_type）合计 ~34 行复制 | l:294-328, l:481-515 | 腐化证据 |

### 量规 3：模式不一致

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 3-观察 | 错误处理风格一致（`.map_err(|e| NortHingError::service(...))?`），仅在 `load_keyword_weights`（l:562-582）使用 `.ok()` 吞错误（设计意图：缺表时返回空 map），其余全部 `?` propagate | l:562-582 | 观察项 |

### 量规 4：注释腐化

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 4-观察 | l:804-821 测试隔离注释准确、设计理由充分（thread-local vs mutex 选择有解释） | l:804-821 | 干净（抽查了） |
| 4-观察 | `segment_for_fts`（l:883-912）无 function-level doc comment——算法意图需从调用点推断。函数体注释无过期项 | l:883 | 观察项（缺注释，非腐化） |

### 量规 5：Hack/绕路

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 5-腐化 | `search_facts` l:556: `.unwrap_or(std::cmp::Ordering::Equal)` — `partial_cmp` 返回 `None` 仅当任一操作数为 NaN，而 score 由 f64 算术得出，NaN 来源不可控。用 Equal 吞 NaN 意味着 NaN score 的 fact 可能与正常 fact 排序等价，隐式丢失 | l:556 | 腐化证据 |
| 5-腐化 | l:475: `.unwrap_or(0)` 在 `SystemTime::duration_since(UNIX_EPOCH)` 上——系统时钟回拨导致 0 时间戳，后续 recency_boost 计算翻转到极端值 (`days = now/86M`)，搜索排序可能被时间戳污染 | l:472-475 | 腐化证据 |

### 量规 6：职责归属错误

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 6-观察 | `judge_mom` KV 方法 `get/set_judge_mom_value`（l:759-789）与 fact CRUD 在语义上独立，但共享同一 `Mutex<Connection>` 资源。属于第二功能域，当前靠同文件组织。边界不违反 core AGENTS.md（在 service/agent_memory/ 内），但 dream.rs 已出走，judge-mom 可进一步模块化 | l:759-789 | 观察项 |
| 6-观察 | `segment_for_fts` + `is_cjk`（l:875-912）是纯函数 FTS 分词器，与 SQLite IO 无耦合。属于文本处理工具，可从 DB 层抽离 | l:875-912 | 观察项 |

### 量规 7：复杂度热点

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 7-腐化 | `search_facts`: 183 行 (l:377-560)，4 层嵌套（if→let→for→match），10 字段元组，查询/映射/评分的三阶段流水线全塞一个函数 | l:377-560 | 腐化证据 |
| 7-腐化 | `get_facts`: 115 行 (l:231-346)，同型嵌套（if→let→for→3x match），9 字段行映射 | l:231-346 | 腐化证据 |
| 7-观察 | `create_tables` + `migrate_facts_columns`: 0-78 行 / 154-184 行——接近 80 行但 SQL DDL 是声明式的，可接受 | l:55-152, l:154-184 | 观察项 |

### 量规 8：测试质量

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 8-观察 | `memory_db_tests.rs` 覆盖关键路径：round-trip 插入/查询、FTS 搜索、keyword boost 影响排序、status 过滤 superseded。测试用 `unique_test_memory_db_path` 保证 hermetic ✅ | memory_db_tests.rs | 观察项 |
| 8-观察 | 线程隔离 seam（l:804-873）thread-local + Drop cleanup 模式正确，测试不共享 DB 文件 ✅ | l:804-873 | 观察项 |

### memory_db.rs 总判定

**腐化中**（与结构层初判 **一致**「更纠结」）

核心腐化驱动力：`search_facts`(183L) + `get_facts`(115L) 内的三重复二（Some/None 分支复制 + 字符串→枚举 match 块复制 + query_map 闭包复制），加上两条死变量线和两条 hack 回退。这些不会自行消退——每新增一个枚举值就要在三个地方同步修改，且复制块之间已出现过 scope/confidence/fact_type 三字段不同时更新的风险（migrate 时加 `fact_type` 列若忘更某处则枚举转换失败但无测试覆盖因为各路径需不同分支）。

---

## 文件 2：`lsp/manager.rs`（836 行）

### 量规 1：死代码

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 1-干净 | 全文件 17 个 pub 方法均有引用；`rollback_registration`、`get_process`、`register_plugin_internal` 在文件内自调用。无 `#[allow(dead_code)]` | 全局 | 干净（用 codegraph+grep 双确认零死引用） |

### 量规 2：重复

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 2-观察 | 15 个 LSP 协议方法（l:320-694）共享 `get_process → json! → send_request/send_notification` 骨架，差异仅 2-5 行 JSON 参数体。但这是 LSP 协议规范的 necesssary 映射——每个方法有独立签名（position vs range vs context 参数组合），无法合并 | l:320-694 | 观察项 |

### 量规 3：模式不一致

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 3-观察 | 错误处理一致性极好：全部用 `?` propagate + `error!`/`warn!` log。零 `unwrap`/`expect` 生产 panic 风险 | 全局 | 干净（抽查了） |

### 量规 4：注释腐化

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 4-干净 | l:64 `Note: workspace root path management has been moved to WorkspaceLspManager` — 准确反映架构变迁（不在 commit 中但注释仍然符合当前结构） | l:64 | 干净 |
| 4-观察 | l:158-163 `start_server` 注释 4 个参数描述有冗余（参数名自文档化），但不影响正确性 | l:158-163 | 观察项 |

### 量规 5：Hack/绕路

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 5-干净 | 无 `unsafe`、无魔数、无 sleep/retry/polling、无 workaround | 全局 | 干净 |

### 量规 6：职责归属错误

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 6-观察 | `stop_all_servers`（l:315-317）是 `shutdown` 的纯转发别名，无独立逻辑。不违规但多余——3 行占位符 | l:315-317 | 观察项 |
| 6-观察 | `Drop` impl（l:697-701）仅 debug 日志，不主动关闭 server processes——依赖 `LspServerProcess` 自身的 Drop 链路。若 Drops 逆序不确定（Arc→ HashMap→ Manager），server 可能不被干净关闭 | l:697-701 | 观察项 |

### 量规 7：复杂度热点

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 7-观察 | 「复杂度热点」不存在单函数超 80 行；最大函数 `start_server` 61 行，`uninstall_plugin` 43 行。JSON 参数构造是重复体而非复杂度 | l:164-228 | 干净（结构重复≠逻辑复杂） |
| 7-观察 | 无 match 超 20 臂；最大 match 在测试模块 `dummy_server_command` 的 cfg 分支（2 臂） | l:734-755 | 干净 |

### 量规 8：测试质量

| # | 发现 | file:line | 分级 |
|---|------|-----------|------|
| 8-观察 | 3 个测试覆盖关键 uninstall 场景：多语言 server 停止、无关 server 不受影响、文件删除失败回滚注册。属关键路径 ✅ | l:781-836 | 观察项 |
| 8-观察 | 测试使用真实 `cmd.exe /c exit 0`（l:734-755）dummy server——是好的 smoke 测试策略。但 `dummy_server_command` 依赖平台 `SystemRoot` 环境变量（l:737），不 hermetic | l:737 | 观察项 |

### lsp/manager.rs 总判定

**稳定**（与结构层初判 **一致**「更清晰」）

836 行中 ~400 行是 LSP 协议方法骨架（不可压缩的结构重复），余 ~400 行是 install/uninstall/lifecycle 逻辑——结构清晰、错误处理一致、无死代码、测试覆盖关键路径。三个观察项（别名函数、Drop 不主动关闭、dummy server 非 hermetic）均为边界且稳定，亟需升级为腐化的转化路径明确。

---

## 总览

| 文件 | 判定 | 腐化证据 | 观察项 | 合计 |
|------|------|---------|--------|------|
| memory_db.rs | 腐化中 | 6 | 2 | 8 |
| lsp/manager.rs | 稳定 | 0 | 3 | 3 |
| **合计** | | **6** | **5** | **11** |

### 与 rot-probe 初判对比

| 文件 | 结构层初判 | 代码层结论 | 一致/推翻 | 一句理由 |
|------|-----------|-----------|----------|---------|
| memory_db.rs | 更纠结 | 腐化中 | ✅ 一致 | 结构信号（三功能域混杂 + 增长轨迹）在代码层得到确认——但腐化的核心是三重复二而非"领域混杂"，dream scoop 已通过 `mod dream` 正确出走 |
| lsp/manager.rs | 更清晰 | 稳定 | ✅ 一致 | 代码层验证成立：零死代码、一致的错误处理、无 hack、合理的测试覆盖。唯一需关注的 Drop 行为在 LspServerProcess 层处理 |

### Top-2 优先行动

| 优先级 | 行动 | 预期释放 | 难度 |
|--------|------|---------|------|
| **P0** | 合并 `get_facts`/`search_facts` 中的 `last_mentioned_at` 死提取 (l:291) 和 `bm25_pos` 死计算 (l:542) — 两行修复，消除代码读者困惑 | 消除 2 处死变量 | 低 |
| **P1** | 提取 `get_facts` 和 `search_facts` 中的字符串→枚举转换块为 `MemoryDb` 私有方法（`fn parse_scope/confidence/fact_type`），消除三重复合 | 消约 70 行复制 | 低中 |
