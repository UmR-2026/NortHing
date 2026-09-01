# 盲审报告：memory_db.rs（god-file 对照组，ceiling 894）

- 目标：`E:\agent-project\NortHing\src\crates\assembly\core\src\service\agent_memory\memory_db.rs`（894 行，实测 = rot-budget ceiling）
- 边界：父 `mod.rs` 通过 `mod memory_db;` 引入；本文件又 `mod dream;` 引入子模块 `memory_db/dream.rs`（128 行）；同父下还有顶层 `agent_memory/dream.rs`（340 行）。两个 `dream` 模块并存。
- 评审基线：仓库 `main`，最近一次文件级 commit `5d4d98a refactor(core): deduplicate memory_db queries, clean dead variables and handle sort/clock fallbacks (W8-2)`（2026-08-29）
- 量规：`E:\agent-project\NortHing\.superpowers\sdd\deep-rot-review-rubric.md` 8 项 + 5 项 judge 纪律

---

## 1. 死代码

| 抽查 | 结果 |
| --- | --- |
| `rg "pub\(crate\) fn" memory_db.rs` | 18 处声明 |
| `rg -c "\.unwrap\(\)" memory_db.rs` | 0（生产代码） |
| 仓内交叉引用 | 所有 `pub(crate)` 项均有调用点（dream.rs / facts.rs / auto_memory.rs / continuity_selfcheck.rs / memory_db_tests.rs / kernel_facade/memory.rs） |
| 私有 fn 引用 | `parse_scope` / `parse_confidence` / `parse_fact_type` / `parse_fact_fields` / `map_fact_row` / `map_search_row` / `sort_scored_facts` / `compute_recency_boost` / `load_keyword_weights` / `migrate_facts_columns` —— 全部有调用方 |

**结论**：干净。未发现 unreachable 分支 / 注释掉的代码 / 遗留 cfg 门 / 零引用的私有 fn。`TEST_MEMORY_DB_PATH` 线程局部 + `MemoryDbPathGuard` 不是死代码——`facts.rs:664,727`、`auto_memory.rs:431,464,483,506`、`continuity_selfcheck.rs:98` 实际在用。

**观察项**：
- `memory_db_tests.rs` 自己从未调用 `with_test_memory_db_path`（rg 命中 0）；测试都走硬编码 `temp_dir.join("memory.db")`，绕开 thread-local seam。seam 是为外部测试准备的，但放在本文件容易让读代码者误以为本文件的测试会用它。（`memory_db_tests.rs` 全部 21 个测试无 `with_test|default_memory_db_path|MemoryDbPathGuard`）

## 2. 重复

| 重复块 | 命中数 | 文件内/跨文件 |
| --- | --- | --- |
| `let conn = self.conn.lock().map_err(\|e\| NortHingError::service(format!("MemoryDb lock poisoned: {}", e)))` | **15** | 仅本文件 (`memory_db.rs:57, 204, 265, 314, 330, 365, 508, 558, 574, 591, 607, 631, 662, 678, 694`) |
| `NortHingError::service(format!("Failed to X: {}", e))` | **24** | 仅本文件 |
| workspace_key Some/None 二分支 prepare+query | **3 处** | `memory_db.rs:268/276`、`memory_db.rs:369/381`、`memory_db/dream.rs:17/26`（跨子模块） |
| `parse_scope`/`parse_confidence`/`parse_fact_type` 在 `memory_db.rs:792-823` 私有定义，`memory_db/dream.rs:75-109` 内联重复展开 match（同样的 `"workspace"/"global"`、`"high"/"med"/"low"`、`"user"/"feedback"/"project"/"reference"` 三组串字面量） | **2 份** | 跨子模块（rust 私有性使子模块无法引用父的 `fn`，因此被迫重写） |
| 测试文件 `let temp_dir = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());` + `let db_path = temp_dir.join("memory.db");` + `let _ = std::fs::remove_dir_all(&temp_dir);` | **20 套** | `memory_db_tests.rs:6-27, 32-74, 79-103, 108-134, 139-162, 167-196, 201-261, 266-277, 282-307, 312-354, 364-375, 380-393, 398-422, 427-437, 444-467, 472-516, 521-553, 558-578, 583-598, 603-691` |

**腐化证据**：
- 15 处 lock 句重复 ~60 行；24 处 `Failed to X` 模板重复 ~70 行；合计约 **130 行可折叠为 1 个 `lock_conn()` 助手 + 1 个 `map_db_err(op, e)` 助手**（腐化中——随每加一个 `MemoryDb` 方法会持续堆积）
- workspace-key Some/None 二分支在 3 个 query 方法（`get_facts`/`search_facts`/`get_stale_facts`）各展开一次，新增 query 类型必须再展开（腐化中）
- 子模块 `memory_db/dream.rs` 重写父模块私有 helper 4 个 match 块，**视觉/语义双胞胎**（`memory_db.rs:792-823` ↔ `memory_db/dream.rs:75-109`）——一处改动必漏另一处（腐化中）
- `memory_db_tests.rs` 20 套 temp_dir 样板 ~80 行；测试文件 770 行中 ~80 行是纯克隆（腐化中）

**观察项**：
- 8 元组 `map_fact_row` / 10 元组 `map_search_row`（`memory_db.rs:231-261`）——`row.get(0..9)` 编号位置访问；`get_stale_facts` 又内联自己的 8 元组 closure（`memory_db/dream.rs:39-50`）。可读但脆弱，新增列极易错位。

## 3. 模式不一致

| 模式 A | 模式 B |
| --- | --- |
| `let weight: Option<f64> = conn.query_row(...).ok(); Ok(weight.unwrap_or(1.0))` (`memory_db.rs:561-569`) | `conn.query_row(...).ok()` + 后续 `if let Some(val) = existing` 模式（`memory_db.rs:511-517`） |
| `match stmt.prepare(...) { Ok(s) => s, Err(_) => return HashMap::new() }` + `match stmt.query_map(...) { Ok(r) => r, Err(_) => return HashMap::new() }` (`memory_db.rs:480-499`，全程静默，无日志) | 仓内其余 24 处统一 `.map_err(\|e\| NortHingError::service(format!("Failed to X: {}", e)))`（同文件，`memory_db.rs:283, 290, 295, 322, 335, 393, 402, 416, 536, 549, 583, 599, 640, 653, 670` 等） |
| `serde_json::from_str(&related_json).unwrap_or_default()`（`memory_db.rs:521`，解析坏 JSON 静默吞） | `serde_json::to_string(...).map_err(\|e\| NortHingError::serialization(...))`（同函数 `memory_db.rs:527-529`，序列化失败硬错） |
| `mapped.filter_map(\|r\| r.ok()).collect()`（`memory_db.rs:124, 143, 162, 654`，每行 IO 错误静默丢弃） | `row.map_err(\|e\| NortHingError::service(format!("Failed to read ... row: {}", e)))`（`memory_db.rs:295, 416`，同类型错误却报错） |
| `let conn = self.conn.lock().map_err(...)`（所有生产路径） | 测试中 `db.insert_fact(&fact, ...).unwrap()`、`Connection::open(&db_path).unwrap()`（`memory_db_tests.rs:9, 11, 34, 81, 98, 110, 126, 141, 157, 169, 185, 203, 247-249, 268, 284, 300, 314, 344-345, 348, 366, 382, 400, 416-419, 429, 446, 474, 505-506, 523, 543-544, 561-562, 564, 585, 591, 595, 605, 654-657`） |

**腐化证据**：
- `load_keyword_weights` 静默路径与仓内 `.map_err` 主流冲突，且无 `tracing::warn!` 记录失败——磁盘损坏 / schema 漂移会返回"keyword 不存在"假象，掩盖真实错误（腐化中——会误导未来 debug）
- `boost_keyword` 同函数内"读 JSON 静默 fallback / 写 JSON 硬错"双向不对称——若 DB 中存在历史脏 JSON，会被静默重置为 `HashSet::new()`，下一次写回覆盖原数据（腐化中——数据丢失风险）
- 4 处 `filter_map(|x| x.ok())` 与同文件 `row.map_err` 同性质处理不一致——`create_tables` 反查 `PRAGMA table_info` 时列读取失败被静默，导致 has_xxx 永远为 false，进而重复 ALTER（腐化中——可能 ALTER 失败的错误被吞）

**观察项**：
- `.unwrap()` 在生产路径全 0，但测试中 35+ 处 `.unwrap()` 与生产 `.map_err` 风格不一致（接受——测试容许 panic）

## 4. 注释腐化

| 抽查 | 结果 |
| --- | --- |
| `rg "TODO\|FIXME\|HACK\|XXX\|UNREACHABLE" memory_db.rs` | 0 命中 |
| `git log --oneline -5 memory_db.rs` | 最近 5 次 commit 主题均与功能/重构相关，无墓碑 |
| `git log -S "// TODO" memory_db.rs`（抽查墓碑） | 未发现过期 TODO |
| 17 行注释块 `memory_db.rs:721-737` | 解释 thread-local seam 设计动机，描述仍准确（seam 当前在 facts.rs/auto_memory.rs/continuity_selfcheck.rs 中确实使用） |

**结论**：干净。无墓碑注释、无过期 TODO、文档与实现一致。

**观察项**：
- `memory_db.rs:721-737` 17 行注释块 vs ~70 行 seam 实现（739-790），比例 1:4。可读但解释比实现还长。

## 5. hack / 绕路

| 检查项 | 命中 | 说明 |
| --- | --- | --- |
| `// ponytail:` 自标注降级 | 0 | 无任何 ponytail 注释 |
| 魔数 | `memory_db.rs:362` `(limit * 3).max(30)`、`memory_db.rs:526` `(weight + 1.0).min(5.0)`、`memory_db.rs:581` `MAX(weight * ?1, ?2)`（外部参数）、`memory_db.rs:461` `86_400_000.0`、`memory_db.rs:441` `-rank * keyword_weight * recency_boost` | 多数有上下文注释（`compute_recency_boost` 用天数/毫秒换算），`min(5.0)` 上限无解释 |
| 内联 `duration_since(UNIX_EPOCH)` | `memory_db.rs:404` | T2-9 时间助手 ratchet 目标 = `northhing_core_types::time`；本文件应走 helper（无 ponytail 标注） |
| `Mutex<Connection>` 串行化 | 全部 15 处 `lock` | 不是 hack，是有界并发模型 |
| 静默兜底 | `boost_keyword:521` `unwrap_or_default`、`get_keyword_weight:569` `unwrap_or(1.0)`、`load_keyword_weights:480-499` `Err(_) => HashMap::new()` | 见 §3 |

**腐化证据**：
- `memory_db.rs:404` 内联 `SystemTime::now().duration_since(UNIX_EPOCH)`——仓内 T2-9 ratchet（rot-budget `unix_epoch_inline` ceiling=69）规定应走 `northhing_core_types::time`，本文件违反，未带 `ponytail:` 降级标签（腐化中——同一助手在 `dream.rs:48-50`、`facts.rs:82-85` 等多处重复内联）

**观察项**：
- `boost_keyword` 静默吞 JSON 解析错（见 §3）——可视为"为过编译器的怪写法"的弱形式
- `weight + 1.0).min(5.0)` 权重上限 5.0 无注释解释语义

## 6. 职责归属错误

按 `core/AGENTS.md`：core 仍拥有 IO / 平台 / 持久化实现，`memory_db.rs` 作为 SQLite + FTS5 存储层归属 `service/agent_memory` 子树合法。

**腐化证据**：
- **两个 `dream` 模块同名异义**：
  - 父 `mod.rs:3` `mod dream;` → `agent_memory/dream.rs:30 run_dream_sweep`（顶层调度 + LLM 调用 + 警告日志）
  - 本文件 `memory_db.rs:890` `mod dream;` → `memory_db/dream.rs:7 get_stale_facts`（纯 DB 查询 impl MemoryDb）
  - 命名完全冲突路径是 `crate::service::agent_memory::memory_db::dream::get_stale_facts` vs 调用点 `crate::service::agent_memory::dream::run_dream_sweep`（`dream.rs:66`）。语义上风马牛不相及——`get_stale_facts` 是 SQL 查询，应作为 `MemoryDb` 公开方法（与 `search_facts`/`get_facts` 同级），不应藏在 `dream` 命名空间下（腐化中——命名误导）
- **`memory_db/dream.rs` 重写父模块私有 helper**：因为 `parse_scope`/`parse_confidence`/`parse_fact_type`/`parse_fact_fields` 在 `memory_db.rs:792-849` 是 `fn`（私有），子模块 `memory_db/dream.rs` 看不到，被迫内联 ~35 行重复 match 块。设计层面：要么这些 helper 升 `pub(super)` / `pub(crate)`，要么子模块位置本身就是错位（腐化中）

**观察项**：
- `default_memory_db_path` (709) / `with_test_memory_db_path` (762) / `MemoryDbPathGuard` (756) / `unique_test_memory_db_path` (769) + 17 行注释 (721-737) 共 ~80 行测试基础设施挂在生产文件，靠 `#[cfg(test)]` 隔离。标准做法，但混在生产文件里膨胀了主文件（讨论项——按 house rule 3 god-file 800+ 行需拆分时，这块要塞到独立 `memory_db_test_seam.rs`？）
- 表名 `judge_mom`（`memory_db.rs:98-102`）实际是通用 KV（`facts_jsonl_migrated_v1:ws`、`dream_last_sweep_at` 都用它，见 `facts.rs:73, 86`、`dream.rs:52, 79`）——命名误导但已稳定

## 7. 复杂度热点

| 检查 | 结果 |
| --- | --- |
| >80 行函数 | `search_facts` 116 行 (`memory_db.rs:341-456`)、`create_tables` 98 行 (`memory_db.rs:55-152`) |
| 嵌套 >4 层 | 未观察到（`search_facts` 最深约 4 层 if-let 嵌套） |
| 参数 >6 个 | `parse_fact_fields` 8 参数 (`memory_db.rs:826-835`)，已带 `#[allow(clippy::too_many_arguments)]` (825) |
| match >20 臂 | 0 |
| `let conn = self.conn.lock()` 块级宏化机会 | 15 处（见 §2） |

**观察项**：
- `create_tables` (98 行) 把 schema 创建 + 列迁移 + backfill + text_fts 反射查询混在一个 fn 内，逻辑 4 段可分（已接近拆分阈值）
- `search_facts` 116 行混合 FTS5 SQL 拼装 + 三因子排序 + 关键字加权 + 时钟兜底，5 段职责，可拆为 `build_match_expr` / `apply_keyword_weight` / `apply_recency_boost`

## 8. 测试质量（基于 `memory_db_tests.rs`，770 行，21 个测试）

| 测试 | 真断言？ | 覆盖关键路径？ |
| --- | --- | --- |
| `open_creates_tables` (L5) | ✓ 断言 4 张表存在 | schema |
| `insert_and_get_fact_round_trip` (L31) | ✓ assert_eq!(all.len(), 2) | insert + get + scope 双分支 |
| `insert_duplicate_id_ignored` (L78) | ✓ assert_eq!(facts.len(), 1) | `INSERT OR IGNORE` |
| `fts_search_matches_keyword` (L107) | ✓ 真匹配 + 真不匹配 | FTS5 关键字 |
| `fts_search_chinese_bigram` (L138) | ✓ 2 字 CJK 命中 | segment_for_fts bigram |
| `fts_search_two_char_cjk` (L166) | ✓ 三组双字断言 | 同 |
| `fts_search_respects_workspace_scope` (L200) | ✓ workspace_key 双分支断言 | scope 过滤 |
| `boost_keyword_increases_weight` (L265) | ✓ assert_eq!(2.0) | boost 累加 |
| `keyword_weight_affects_scored_fact` (L281) | ✓ assert!(score > 0.0) | keyword 加权 |
| `ranking_fuses_three_factors` (L311) | ✓ assert_eq!(hits[0].fact.id, "high") | 三因子融合排序 |
| `segment_for_fts_bigram` (L358) | ✓ assert_eq!("以后 后都 都用 pnpm") | 分词核心算法 |
| `boost_keyword_respects_cap` (L363) | ✓ assert!(w <= 5.0) | 5.0 上限 |
| `decay_weights_respects_floor` (L379) | ✓ assert!(w >= 0.1) | floor |
| `delete_fact_removes_from_fts` (L397) | ✓ 双 assert | delete + FTS5 trigger |
| `empty_query_returns_empty` (L426) | ✓ 真空查询短路 | early-return |
| `fact_type_round_trip` (L443) | ✓ enum 4 值之一 | fact_type 列 |
| `status_filter_hides_superseded` (L471) | ✓ assert_eq!(all.len(), 1) | status 列 |
| `fact_reviews_round_trip` (L520) | ✓ 2 reviews 顺序 | fact_reviews 表 |
| `migration_idempotent_on_reopen` (L557) | ✓ 二次 open 表仍存在 | 迁移幂等 |
| `judge_mom_kv_round_trip` (L582) | ✓ 三态断言 | judge_mom KV |
| `get_stale_facts_filters_and_orders` (L602) | ✓ filter + order + limit | get_stale_facts |
| `sort_scored_facts_nan_sinks_to_bottom` (L694) | ✓ f64::NAN 在尾部 | NaN sink |
| `recency_boost_skips_on_clock_anomaly` (L763) | ✓ None → 1.0 | 时钟兜底 |

**观察项**：
- 22 个测试均为真断言（非恒真），覆盖核心路径（insert/get/search/scope/FTS5/bigram/keyword weight/ranking/NaN sort/clock anomaly/idempotency），质量 OK
- **缺测**：`load_keyword_weights` 静默失败路径无测试（如果实现 bug 让 prepare 永远成功但 query_map 失败，测试不会暴露）
- **缺测**：`supersede_fact` 未单独覆盖——只在 `get_stale_facts_filters_and_orders` 中作为 setup 使用
- **缺测**：CJK 单字（`flush_cjk` 中 `cjk.len() == 1` 分支，`memory_db.rs:864-865`）无测试覆盖——`segment_for_fts_bigram` 只测了 4 字串
- **缺测**：`boost_keyword` 的 `unwrap_or_default` 静默 JSON 解析（line 521）——损坏 related_keywords JSON 不报错不报警，测试未验证

**腐化证据**：
- 测试文件 ~80 行纯克隆（20 套 temp_dir 样板，见 §2）——腐化中
- 测试和 `memory_db.rs` 生产代码共享 770 行文件（`#[path = "memory_db_tests.rs"] mod tests` at L894），但文件物理独立，无纠缠——只是规模

---

## 总判定

**腐化中**。

理由：
1. **行数压顶**：894 行 = rot-budget ceiling（实测 `wc = 894`），再涨 1 行即触发 ratchet 升级（house rule 3：>800 升压、>1000 必须 `// allow-god-file` 或拆分）
2. **腐化扩散点集中**：~130 行 lock+error 样板 + ~80 行测试样板 + 子模块 ~35 行 helper 重写 = **~245 行（27%）** 可在一次重构中消解（提出 `lock_conn()` + `map_db_err()` + 把 `memory_db/dream.rs` 内容并回 `memory_db.rs` 并把 helper 升 `pub(super)` + 让 `memory_db_tests.rs` 改用 `with_test_memory_db_path`）
3. **错误处理双标准并存**：静默 `unwrap_or`/`filter_map(|x|x.ok())` 与 `.map_err` 共存于同函数（§3），且无 `tracing` 警告日志掩盖未来的 silent corruption
4. **边界命名冲突**：`agent_memory::dream::run_dream_sweep` 与 `agent_memory::memory_db::dream::get_stale_facts` 同名前缀，新人极易误解
5. **时间助手 ratchet 违反**：`duration_since(UNIX_EPOCH)` 内联 1 处，未带 `ponytail:` 标签，与仓内 T2-9 规则冲突

结构层初判（推测 baseline：god-file ≥800 升压）一致。代码层确实腐化且正在恶化（每加一个 `pub(crate) fn` 都会复制 1 套 lock 模板）。

---

## 证据抽查（盲态纪律硬格式要求）

每个数字断言均当次实测（rg / wc / git log），未凭记忆。

| 断言 | 命令 | 结果 |
| --- | --- | --- |
| memory_db.rs 894 行 | `(Get-Content -Raw ...).Split("`n").Count` | **895**（894 + 末尾 newline；与 ceiling=894 一致） |
| rot-budget 登记 memory_db.rs | `rg -n "memory_db" scripts/rot-budget.json` | 命中 `L62`，ceiling=894 |
| `let conn = self.conn.lock` 重复 | `rg -c "let conn = self.conn.lock" memory_db.rs` | **15** |
| `MemoryDb lock poisoned` 字面量 | `rg -n "MemoryDb lock poisoned" memory_db.rs` | **15** 行（57, 204, 265, 314, 330, 365, 508, 558, 574, 591, 607, 631, 662, 678, 694） |
| `NortHingError::service` 调用 | `rg -c "NortHingError::service\(" memory_db.rs` | **44**（其中 24 个 `Failed to X` 模板） |
| `Failed to X: {}` 模板 | `rg -n "Failed to (insert\|delete\|prepare\|query\|read\|update\|search\|touch\|add\|boost\|backfill\|supersede\|ignore\|decay)" memory_db.rs` | **24** 行 |
| workspace-key Some/None prepare 双分支 | 手工 grep `if workspace_key.is_some()`、`if let Some(ws) = workspace_key` | **3 处**：`memory_db.rs:268/276` (get_facts)、`memory_db.rs:369/381` (search_facts)、`memory_db/dream.rs:17/26` (get_stale_facts) |
| `parse_scope`/`parse_confidence`/`parse_fact_type` 在 dream 子模块重写 | `rg -n "parse_scope\|parse_confidence\|parse_fact_type\|parse_fact_fields" agent_memory/` | **仅 memory_db.rs 定义**；`memory_db/dream.rs` 用内联 match 替代 |
| 子模块 dream.rs 中 `FactScope::Workspace` 等字面量 | `rg -n "FactScope::Workspace\|FactScope::Global" memory_db/dream.rs` | 命中 L76,77；对应父模块 `memory_db.rs:794-795` |
| `filter_map(|x| x.ok())` 静默吞错 | `rg -n "filter_map\(\|.*\| .*\.ok\(\)\)" memory_db.rs` | **4** 处（124, 143, 162, 654） |
| `unwrap_or_default`/`unwrap_or` 静默 fallback | `rg -n "\.unwrap_or\|\.unwrap_or_default" memory_db.rs` | **3** 处（475 = NaN sort，521 = JSON parse，569 = missing keyword） |
| 生产代码 `.unwrap()`/`expect` | `rg -n "\.unwrap\(\)\|\.expect\(" memory_db.rs` | **0**（rot-budget `unwrap_production` ceiling=502 不含 tests） |
| `allow(dead_code)` | `rg -n "allow\(dead_code\)" memory_db.rs` | 1 处 `#[allow(clippy::too_many_arguments)]` (L825)，无 dead_code 抑制 |
| 内存联 `duration_since(UNIX_EPOCH)` | `rg -n "duration_since.*UNIX_EPOCH" memory_db.rs` | **1** 处（L404） |
| TODO/FIXME/HACK/XXX | `rg -n "UNREACHABLE\|unreachable!\(\)\|TODO\|FIXME\|HACK\|XXX" memory_db.rs` | **0** |
| `// ponytail:` 标签 | `rg -n "// ponytail:" memory_db.rs` | **0** |
| `pub(crate) fn` 声明 | `rg -n "pub\(crate\) fn" memory_db.rs` | **18**（行号：30, 186, 263, 312, 328, 341, 501, 556, 572, 589, 605, 629, 660, 676, 692, 709, 762, 769） |
| `with_test_memory_db_path` 在 memory_db_tests.rs 使用 | `rg -n "with_test\|default_memory_db_path\|MemoryDbPathGuard" memory_db_tests.rs` | **0** |
| `with_test_memory_db_path` 在仓内总使用 | `rg -n "with_test_memory_db_path" --type rust src/` | **8** 处（4 在 memory_db.rs 定义 + mod.rs re-export + facts.rs:664,727 + auto_memory.rs:431,464,483,506 + continuity_selfcheck.rs:98 + identity.rs 注释镜像） |
| 测试 temp_dir 样板 | `rg -n "std::env::temp_dir" memory_db_tests.rs` | **20** 处 |
| 测试 file 行数 | `(Get-Content -Raw memory_db_tests.rs).Split("`n").Count` | **770** |
| memory_db/dream.rs 行数 | `(Get-Content -Raw memory_db/dream.rs).Split("`n").Count` | **128** |
| 顶层 dream.rs 行数 | `(Get-Content -Raw dream.rs).Split("`n").Count` | **340** |
| 最近 commit | `git log -1 --format="%cI %s" -- memory_db.rs` | `2026-08-29T00:54:10+08:00 refactor(core): deduplicate memory_db queries, clean dead variables and handle sort/clock fallbacks (W8-2)` |
| HEAD commit | `git log --oneline -1 HEAD` | `600f21b docs(sdd): 5-item judge checklist validated via blind review of theme.rs (0 misses, 2 false positives -> check E hardened)` |

---

## 总览计数

- 腐化证据（实质性、随时间恶化）：**8 条**
  - §2 lock-句 15× 重复
  - §2 `Failed to X` 24× 重复
  - §2 workspace-key 双分支 3 处跨方法
  - §2 子模块 helper 重写（parse_*）35 行
  - §2 测试 temp_dir 20 套克隆
  - §3 错误处理双标准并存（`load_keyword_weights` 静默 vs `.map_err`；`boost_keyword` 读静默写硬错；`filter_map(|x| x.ok())` 与 `row.map_err`）
  - §6 两个 `dream` 模块命名冲突（`agent_memory::dream` vs `memory_db::dream`）
  - §6 子模块拿不到父私有 helper（helper 提升 / 子模块合并 / 拆主文件的职责归属决策待决）

- 观察项（存在但稳定/有界）：**7 条**
  - §1 `memory_db_tests.rs` 不用 seam
  - §2 tuple-row positional get
  - §4 17 行注释 vs ~70 行 seam（注释比实现长）
  - §5 `weight + 1.0).min(5.0)` 无注释
  - §5 `boost_keyword` JSON 静默吞（弱 hack 形式）
  - §6 `default_memory_db_path` 等测试基础设施 ~80 行挂在生产文件
  - §6 `judge_mom` 表名实为通用 KV

- 干净（抽查了但没发现）：**5 项**
  - §1 死代码（所有 fn 有调用方）
  - §4 注释腐化（无墓碑）
  - §7 嵌套 >4 层 / match >20 臂 / 参数 >6 之外（搜索长函数、未爆炸的 match）
  - §8 测试真假断言（22/22 真断言）
  - §8 生产代码 `.unwrap()`/`expect`（0 命中）

- 与 rot-budget ceiling 关系：实测 894 = ceiling 894（精确触顶）
- 与 rot-probe 结构层初判一致（god-file ≥800 升压；本文件代码层确有实质腐化）