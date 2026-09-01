# Task Report — W14-1e: 修复测试污染开发机真实记忆库

- **状态**：`DONE`
- **Commit**：`eee155215ffeb3a2fae392ed7190b1439bba57d8` (`eee1552`)
- **BASE**：`8c00962`

---

## 1. Git 变更统计 (`git show --stat`)

```text
commit eee155215ffeb3a2fae392ed7190b1439bba57d8
Author: Mavis <mavis@northhing.local>
Date:   Tue Sep 1 21:46:03 2026 +0800

    fix(core/memory): isolate test memory db in query_aware_tests

 src/crates/assembly/core/src/service/agent_memory/auto_memory.rs | 7 ++++++-
 1 file changed, 6 insertions(+), 1 deletion(-)
```

---

## 2. 真实库 mtime/size 前后对比（核心验收证据）

- **解析到的真实数据库绝对路径**：`C:\Users\UmR\AppData\Roaming\northhing\memory\memory.db`

### 测试前
```powershell
FullName      : C:\Users\UmR\AppData\Roaming\northhing\memory\memory.db
LastWriteTime : 2026/8/29 17:55:58
Length        : 94208
```

### 测试后（跑完所有 `agent_memory` 与 `memory` 测试及全仓检查后）
```powershell
FullName      : C:\Users\UmR\AppData\Roaming\northhing\memory\memory.db
LastWriteTime : 2026/8/29 17:55:58
Length        : 94208
```

- **对比结论**：`LastWriteTime` 与 `Length` 严格一致（0 字节变化，0 毫秒漂移），真实 memory.db 未受任何读写污染。

---

## 3. 全仓核对逐条结论

| 序号 | 文件与行号 / 测试或函数 | 类型 | 守卫持有状态与结论 |
|---|---|---|---|
| 1 | `auto_memory.rs:575` (`build_query_aware_facts_reminder_returns_some_with_matching_fact`) | 测试代码 | **已补守卫**：函数体第一行注入 `let _db_guard = with_test_memory_db_path(unique_test_memory_db_path());`，解决核心缺陷 |
| 2 | `auto_memory.rs:563` (`build_query_aware_facts_reminder_returns_none_when_no_match`) | 测试代码 | **已补守卫**：新发现未受保护点，非空查询触发底层 `MemoryDb::open`，已补入守卫隔离 |
| 3 | `auto_memory.rs:551` (`build_query_aware_facts_reminder_returns_none_for_empty_query`) | 测试代码 | **已补守卫**：空查询快速返回，已补入守卫确保模块测试风格与密闭性一致 |
| 4 | `facts.rs:668`, `:704` (`migrate_facts_jsonl_once_idempotency_and_marker`) | 测试代码 | **已有守卫**：`:664` 已持有 `let _guard = with_test_memory_db_path(unique_test_memory_db_path());` |
| 5 | `facts.rs:731` (`migrate_facts_jsonl_once_missing_file_sets_marker`) | 测试代码 | **已有守卫**：`:727` 已持有 `let _guard = with_test_memory_db_path(unique_test_memory_db_path());` |
| 6 | `continuity_selfcheck.rs:187`, `:288` (`continuity_selfcheck_seed_restore_diff`) | 测试代码 | **已有守卫**：`:98` 已持有 `let memory_guard = with_test_memory_db_path(unique_test_memory_db_path());` 直至 `:318` drop |
| 7 | `kernel_facade/memory.rs:63`, `:102` (`list_facts`, `search_facts`) | 生产代码 | **生产代码无需改**：`KernelFacade` 供外部查询的契约实现，非测试代码 |
| 8 | `auto_memory.rs:246`, `:302` (`build_workspace_agent_memory_prompt`, `build_query_aware_facts_reminder`) | 生产代码 | **生产代码无需改**：运行时提示词构建与检索逻辑，非测试代码 |
| 9 | `dream.rs:38` (`run_dream_sweep`) | 生产代码 | **生产代码无需改**：定期记忆清洗运行时逻辑，非测试代码 |
| 10 | `turn_persist.rs:457` (`append_facts_entry`) | 生产代码 | **生产代码无需改**：会话轮次持久化运行时逻辑，非测试代码 |

---

## 4. 验证命令输出

### 命令 1: `cargo test -p northhing-core --features product-full agent_memory`
```text
running 69 tests
test service::agent_memory::distiller::tests::parse_bad_json_returns_empty ... ok
test service::agent_memory::distiller::tests::parse_empty_array_returns_empty ... ok
test service::agent_memory::dream::tests::parse_index_out_of_bounds_skipped ... ok
test service::agent_memory::dream::tests::parse_fence_tolerant ... ok
test service::agent_memory::dream::tests::parse_reason_truncated ... ok
test service::agent_memory::dream::tests::parse_unknown_action_skipped ... ok
test service::agent_memory::dream::tests::parse_valid_json_array_maps_fields ... ok
test service::agent_memory::distiller::tests::parse_text_over_300_chars_truncated ... ok
test service::agent_memory::distiller::tests::parse_four_items_truncates_to_three ... ok
test service::agent_memory::distiller::tests::parse_unknown_fact_type_skipped_valid_kept ... ok
test service::agent_memory::distiller::tests::parse_json_fence_wrap ... ok
test service::agent_memory::facts::tests::distill_facts_no_keyword_returns_empty ... ok
test service::agent_memory::dream::tests::parse_bad_json_returns_empty ... ok
test service::agent_memory::distiller::tests::parse_valid_json_array_maps_fields ... ok
test service::agent_memory::facts::tests::distill_facts_truncates_long_sentence ... ok
test service::agent_memory::facts::tests::distill_facts_multiple_sentences_with_keyword ... ok
test service::agent_memory::facts::tests::distill_facts_with_cjk_period ... ok
test service::agent_memory::facts::tests::select_facts_budget_exactly_full ... ok
test service::agent_memory::facts::tests::distill_facts_with_keyword_always ... ok
test service::agent_memory::facts::tests::distill_facts_with_keyword_remember ... ok
test service::agent_memory::facts::tests::distill_facts_with_keyword_chinese ... ok
test service::agent_memory::facts::tests::select_facts_budget_zero_excludes_all ... ok
test service::agent_memory::facts::tests::read_facts_missing_file_returns_empty_not_error ... ok
test service::agent_memory::facts::tests::select_facts_empty_input_returns_empty ... ok
test service::agent_memory::facts::tests::select_facts_respects_confidence_high_first ... ok
test service::agent_memory::facts::tests::select_facts_respects_newer_first_within_same_scope_and_confidence ... ok
test service::agent_memory::facts::tests::select_facts_respects_scope_global_first ... ok
test service::agent_memory::facts::tests::select_facts_short_text_exact_budget ... ok
test service::agent_memory::facts::tests::select_facts_truncates_within_budget ... ok
test service::agent_memory::facts::tests::select_facts_zero_budget_returns_empty ... ok
test service::agent_memory::facts::tests::serde_confidence_serializes_to_lowercase ... ok
test service::agent_memory::facts::tests::serde_fact_missing_fact_type_defaults_to_feedback ... ok
test service::agent_memory::facts::tests::serde_fact_round_trip ... ok
test service::agent_memory::facts::tests::serde_scope_serializes_to_lowercase ... ok
test service::agent_memory::facts::tests::token_estimation_ceiling_division ... ok
test service::agent_memory::facts::tests::read_facts_nonexistent_file_returns_empty ... ok
test service::agent_memory::memory_db::tests::recency_boost_skips_on_clock_anomaly ... ok
test service::agent_memory::auto_memory::query_aware_tests::build_query_aware_facts_reminder_returns_none_for_empty_query ... ok
test service::agent_memory::memory_db::tests::segment_for_fts_bigram ... ok
test service::agent_memory::memory_db::tests::sort_scored_facts_nan_sinks_to_bottom ... ok
test service::agent_memory::facts::tests::read_facts_skips_damaged_lines ... ok
test service::agent_memory::memory_db::tests::boost_keyword_increases_weight ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_without_facts_excludes_remembered_facts_section ... ok
test service::agent_memory::facts::tests::migrate_facts_jsonl_once_missing_file_sets_marker ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_degrades_when_facts_file_unreadable ... ok
test service::agent_memory::memory_db::tests::fact_type_round_trip ... ok
test service::agent_memory::memory_db::tests::fts_search_chinese_bigram ... ok
test service::agent_memory::auto_memory::query_aware_tests::build_query_aware_facts_reminder_returns_none_when_no_match ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_select_facts_budget_limit ... ok
test service::agent_memory::memory_db::tests::open_creates_tables ... ok
test service::agent_memory::memory_db::tests::decay_weights_respects_floor ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_facts_includes_remembered_facts_section ... ok
test service::agent_memory::facts::tests::migrate_facts_jsonl_once_idempotency_and_marker ... ok
test service::agent_memory::memory_db::tests::empty_query_returns_empty ... ok
test service::agent_memory::memory_db::tests::fts_search_matches_keyword ... ok
test service::agent_memory::memory_db::tests::delete_fact_removes_from_fts ... ok
test service::agent_memory::memory_db::tests::judge_mom_kv_round_trip ... ok
test service::agent_memory::memory_db::tests::migration_idempotent_on_reopen ... ok
test service::agent_memory::memory_db::tests::fts_search_respects_workspace_scope ... ok
test service::agent_memory::auto_memory::query_aware_tests::build_query_aware_facts_reminder_returns_some_with_matching_fact ... ok
test service::agent_memory::memory_db::tests::fts_search_two_char_cjk ... ok
test service::agent_memory::memory_db::tests::fact_reviews_round_trip ... ok
test service::agent_memory::memory_db::tests::insert_duplicate_id_ignored ... ok
test service::agent_memory::memory_db::tests::insert_and_get_fact_round_trip ... ok
test service::agent_memory::memory_db::tests::status_filter_hides_superseded ... ok
test service::agent_memory::memory_db::tests::keyword_weight_affects_scored_fact ... ok
test service::agent_memory::memory_db::tests::ranking_fuses_three_factors ... ok
test service::agent_memory::memory_db::tests::get_stale_facts_filters_and_orders ... ok
test service::agent_memory::memory_db::tests::boost_keyword_respects_cap ... ok

test result: ok. 69 passed; 0 failed; 0 ignored; 0 measured; 1003 filtered out; finished in 0.24s
```

### 命令 2: `cargo test -p northhing-core --features product-full memory`
```text
test result: ok. 76 passed; 0 failed; 0 ignored; 0 measured; 996 filtered out; finished in 0.25s
```

### 命令 3: `node scripts/verify-rot-budget.mjs`
```text
Rot budget verification passed (5 grep rules [unwrap_production=477/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=386/400], 6 god-file rules checked across 1365 files).
```

### 补充验证门禁
- `cargo check --workspace`: Finished `dev` profile in 1m 41s (0 errors)
- `cargo check -p northhing`: Finished `dev` profile in 28.02s (0 errors)

---

## 5. 偏离清单与编译错误层级

- **偏离清单**：无偏离。严格按 brief 与 spec 执行，只修改 `auto_memory.rs` 测试块。
- **编译错误修复层级**：零编译错误，首轮即通过所有编译与类型检查。
