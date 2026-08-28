# Implementation Report — W8-2: memory_db.rs 内部去重 + 死变量/回退 hack 处置

## 1. 状态

**DONE**

## 2. 改动清单

### §1 消三重复制
- 提取 `map_fact_row` 与 `map_search_row` 私有行映射 helper，消除 `get_facts` / `search_facts` 在 `Some(workspace_key)` 与 `None` 分支间的重复 query_map 闭包与 prepare 样板。
- 提取 `parse_scope`、`parse_confidence`、`parse_fact_type` 以及 `parse_fact_fields` 解析 helper，消除 `get_facts` 与 `search_facts` 之间逐字重复的三块枚举转换 match 与 Fact 字段构造。
- 语义严格与现状保持一致：未知枚举字符串均返回 `Err(NortHingError::service(format!("Unknown <field>: {}", ...)))`。

### §2 死变量处置
- `get_facts`: 移除了 SQL 查询及解构中未被 `Fact` 使用的 `last_mentioned_at` 字段（8 列）。
- `search_facts`: 移除了 `let bm25_pos = -rank;` 死计算，直接在 score 计算表达式中使用 `-rank * keyword_weight * recency_boost`。
- **观察项**：`ScoredFact.bm25` 保持存储 SQLite FTS5 原生 raw rank（负值 float），score 排序计算使用正向化 score，保持存储与计算语义不变。

### §3 回退 hack 处置
- `search_facts` 排序（NaN 处置）：提取 `sort_scored_facts` helper，针对 `partial_cmp` 在 NaN 时的 `None` 结果，通过 `match (a.score.is_nan(), b.score.is_nan())` 保证 NaN score 在降序排序中沉底（`Ordering::Greater`），并在 `memory_db_tests.rs` 中增加单测 `sort_scored_facts_nan_sinks_to_bottom`。
- `search_facts` 时钟回拨处置：提取 `compute_recency_boost` helper，在 `SystemTime::now().duration_since(UNIX_EPOCH)` 异常时记录 `tracing::warn!("System clock before UNIX_EPOCH ({}); skipping recency boost", e)` 并跳过 recency boost（返回 1.0），并在 `memory_db_tests.rs` 中增加单测 `recency_boost_skips_on_clock_anomaly`。

### §4 防线与行数
- `memory_db.rs` 行数由 918 行下降至 894 行（实测 countLines 894）。
- `scripts/rot-budget.json` 中 `god_file:src/crates/assembly/core/src/service/agent_memory/memory_db.rs` ceiling 由 918 下调至 894。

## 3. 验证输出

### 1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check --workspace`
```
warning: `northhing-core` (lib) generated 16 warnings (run `cargo fix --lib -p northhing-core` to apply 15 suggestions)
warning: `northhing` (bin "northhing") generated 44 warnings (run `cargo fix --bin "northhing" -p northhing` to apply 1 suggestion)
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.93s
```

### 2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core --features product-full memory_db`
```
running 23 tests
test service::agent_memory::memory_db::tests::recency_boost_skips_on_clock_anomaly ... ok
test service::agent_memory::memory_db::tests::sort_scored_facts_nan_sinks_to_bottom ... ok
test service::agent_memory::memory_db::tests::segment_for_fts_bigram ... ok
test service::agent_memory::memory_db::tests::boost_keyword_increases_weight ... ok
test service::agent_memory::memory_db::tests::insert_and_get_fact_round_trip ... ok
test service::agent_memory::memory_db::tests::delete_fact_removes_from_fts ... ok
test service::agent_memory::memory_db::tests::status_filter_hides_superseded ... ok
test service::agent_memory::memory_db::tests::empty_query_returns_empty ... ok
test service::agent_memory::memory_db::tests::fts_search_two_char_cjk ... ok
test service::agent_memory::memory_db::tests::open_creates_tables ... ok
test service::agent_memory::memory_db::tests::migration_idempotent_on_reopen ... ok
test service::agent_memory::memory_db::tests::insert_duplicate_id_ignored ... ok
test service::agent_memory::memory_db::tests::fts_search_chinese_bigram ... ok
test service::agent_memory::memory_db::tests::fact_type_round_trip ... ok
test service::agent_memory::memory_db::tests::fts_search_matches_keyword ... ok
test service::agent_memory::memory_db::tests::fact_reviews_round_trip ... ok
test service::agent_memory::memory_db::tests::judge_mom_kv_round_trip ... ok
test service::agent_memory::memory_db::tests::keyword_weight_affects_scored_fact ... ok
test service::agent_memory::memory_db::tests::decay_weights_respects_floor ... ok
test service::agent_memory::memory_db::tests::fts_search_respects_workspace_scope ... ok
test service::agent_memory::memory_db::tests::ranking_fuses_three_factors ... ok
test service::agent_memory::memory_db::tests::get_stale_facts_filters_and_orders ... ok
test service::agent_memory::memory_db::tests::boost_keyword_respects_cap ... ok

test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 1032 filtered out; finished in 0.20s
```

### 3. `node scripts/verify-rot-budget.mjs`
```
Rot budget verification passed (5 grep rules [unwrap_production=474/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=269/400], 7 god-file rules checked across 1348 files).
```

## 4. 偏离清单

零偏离。
