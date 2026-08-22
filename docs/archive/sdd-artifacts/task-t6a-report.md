# Task T6a Report - 话题权重上升通道接线

> 状态：**DONE**

## 1. 状态

**DONE** - 实现完成，6 条验证全过，已提交 `27c9738`。

本任务经历两阶段：预检发现 brief 自身矛盾 -> 标 BLOCKED 上报 -> 用户裁定解法 A 加强版 -> 实现并通过全部验证。矛盾分析作为「§10 预检发现的 brief 缺陷」保留，是后续修订 brief 的依据。

## 2. 两条已授权行为变更的落地说明

### 2.1 衰减底线 0.1 -> 1.0

**落地位置**：`src/crates/assembly/core/src/agentic/growth_adapter.rs` 顶部常量

```rust
pub(crate) const TOPIC_DECAY_FACTOR: f64 = 0.99;
pub(crate) const TOPIC_DECAY_FLOOR: f64 = 1.0;
```

`turn_persist.rs` 原先的 `db.decay_all_weights(0.99, 0.1)` 已删除，改由 `boost_turn_topics` 内部调用 `db.decay_all_weights(TOPIC_DECAY_FACTOR, TOPIC_DECAY_FLOOR)`，即 `decay_all_weights(0.99, 1.0)`。底线 1.0 与「从未提及」的隐含权重一致，保证「提过又冷却」的话题不会排得比「从未提及」还低。

### 2.2 boost 与 decay 成对、每回合一次、搬到早退之前

**落地位置**：`src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs`，`append_facts_entry` 函数内

改动前（`985bbb9`，行号相对当时文件）：
```rust
// :486
if let Ok(db) = &db {
    growth_adapter::finish_distill_turn(db, &mut growth_state, !candidates.is_empty(), now_ms);
}

if candidates.is_empty() {
    return;
}
// ...（迁移块内 :563）
let _ = db.decay_all_weights(0.99, 0.1);
```

改动后（`27c9738`）：
```rust
// finish_distill_turn 之后、is_empty 早退之前
if let Ok(db) = &db {
    growth_adapter::finish_distill_turn(db, &mut growth_state, !candidates.is_empty(), now_ms);
}

// Topic weight boosting: boost the topics mentioned in this turn's
// user input, then apply the paired per-turn decay. This must happen
// on every completed turn (before the candidates-empty early return)
// so boost and decay stay in lockstep; running boost only on turns
// that produced facts would let weights monotonically inflate. The
// function is warn-only and reuses the existing `now_ms`.
if let Ok(db) = &db {
    growth_adapter::boost_turn_topics(db, user_input, now_ms);
}

if candidates.is_empty() {
    return;
}
// ...（迁移块内，decay 行已删除，留注释指明迁移去向）
// Per-turn topic decay moved to growth_adapter::boost_turn_topics
// (paired with boost, before the candidates-empty early return).
```

boost 与 decay 现在成对发生在每个完成的回合（包括 `candidates.is_empty()` 早退的回合），顺序是先 boost 再 decay，由 `boost_turn_topics` 内部保证。

## 3. 改动文件清单（仅 2 个，符合 brief §3）

| 文件 | 改动 |
| --- | --- |
| `src/crates/assembly/core/src/agentic/growth_adapter.rs` | +2 常量、+1 函数 `boost_turn_topics`、+9 inline 测试、+1 EPS 常量 |
| `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs` | 删 1 行 `decay_all_weights(0.99, 0.1)`、+8 行 boost 调用与注释、+2 行迁移去向注释 |

未改任何其它文件。特别未改：`memory_db.rs`、`facts.rs`、`dream.rs`、`distiller.rs`、`judge_memory.rs`、crate 侧、`Cargo.toml`、任何 SQL/schema。

## 4. `boost_turn_topics` 实现要点

```rust
pub(crate) fn boost_turn_topics(db: &MemoryDb, user_input: &str, now_ms: u64) {
    let topics = extract_topics(user_input);

    for (idx, topic) in topics.iter().enumerate() {
        let related: Vec<String> = topics
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != idx)
            .map(|(_, t)| t.clone())
            .collect();
        if let Err(err) = db.boost_keyword(topic, &related, now_ms) {
            tracing::warn!("Failed to boost topic weight for '{}': {}", topic, err);
        }
    }

    if let Err(err) = db.decay_all_weights(TOPIC_DECAY_FACTOR, TOPIC_DECAY_FLOOR) {
        tracing::warn!("Failed to decay topic weights: {}", err);
    }
}
```

- 话题来源：`extract_topics(user_input)`（crate 侧纯函数，一行未改）
- `related` = 同回合其它话题（写入现有 `related_keywords` 字段）
- 顺序：先 boost 再 decay
- topics 为空时仍执行 decay（不早退）
- warn-only：所有失败 `tracing::warn!` 后吞掉，无 `?` 传播、无 `unwrap`/`expect`/`panic`

## 5. 权重语义（用户裁定的权威口径，已写入函数文档注释）

区间 `[1.0, 5.0]`，`boost_keyword` / `decay_all_weights` / `search_facts` 一行未改。语义澄清为：

- **从未提及** -> 隐含 1.0（`get_keyword_weight` 对无行返回 1.0，`search_facts` fold 初值 1.0）
- **首次提及** -> 1.0（`boost_keyword` INSERT 分支把权重**置为** 1.0，非基线 +1.0；经 floor=1.0 钳制后不升温）
- **第二次及以后提及** -> 严格 > 1.0（UPDATE 分支 `weight + 1.0`，上限 5.0），每回合 ×0.99 缓慢回落但不跌破 1.0

这个语义可接受且更正确：单次提及不构成「热话题」，重复提及才是热度信号。文档注释已说明首次提及为何不升温，避免后来者误以为是 bug。

## 6. 六条验证命令与原始输出

### 命令 1：`cargo check -p northhing-core --features product-full`

```
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo check -p northhing-core --features product-full
```

```
   Compiling northhing-core v(...) (...)

warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:236:36
    |
236 |         let mut stmt = if let Some(ws) = workspace_key {
    |                                    ^^ help: ...
warning: unused variable: `last_mentioned_at`
   --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:291:80
warning: unused variable: `at_ms`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db.rs:743:85
warning: unused variable: `ws`
  --> src\crates\assembly\core\src\service\agent_memory\memory_db\dream.rs:17:36
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 23.46s
```

**warning 数 = 19**（= 基线，无新增）✓

### 命令 2：`cargo test -p northhing-core --features product-full growth_adapter`

```
test agentic::growth_adapter::tests::boost_turn_topics_co_occurrence_records_related_row_count ... ok
test agentic::growth_adapter::tests::boost_turn_topics_cjk_input_produces_a_row ... ok
test agentic::growth_adapter::tests::boost_turn_topics_empty_and_stopword_input_still_decays ... ok
test agentic::growth_adapter::tests::boost_turn_topics_first_mention_equals_baseline_by_design ... ok
test agentic::growth_adapter::tests::boost_turn_topics_floor_never_broken_by_long_cooling ... ok
test agentic::growth_adapter::tests::boost_turn_topics_never_mentioned_returns_baseline ... ok
test agentic::growth_adapter::tests::boost_turn_topics_repeated_mentions_increase_monotonically ... ok
test agentic::growth_adapter::tests::boost_turn_topics_respects_five_cap ... ok
test agentic::growth_adapter::tests::boost_turn_topics_second_mention_raises_above_baseline ... ok
test agentic::growth_adapter::tests::boost_turn_topics_warn_only_no_panic_on_healthy_db ... ok
test agentic::growth_adapter::tests::finish_distill_turn_continues_counting_while_paused ... ok
test agentic::growth_adapter::tests::finish_distill_turn_does_not_rewrite_legacy_keys ... ok
test agentic::growth_adapter::tests::finish_distill_turn_triggers_pause_at_threshold_and_persists ... ok
test agentic::growth_adapter::tests::finish_distill_turn_uses_migrated_legacy_counts ... ok
test agentic::growth_adapter::tests::finish_distill_turn_with_facts_increments_hits_and_no_pause ... ok
test agentic::growth_adapter::tests::begin_distill_turn_returns_false_when_paused ... ok
test agentic::growth_adapter::tests::begin_distill_turn_returns_true_on_unpaused_db ... ok
test agentic::growth_adapter::tests::blob_takes_precedence_over_legacy_keys ... ok
test agentic::growth_adapter::tests::dirty_legacy_keys_do_not_panic ... ok
test agentic::growth_adapter::tests::fresh_db_loads_default_state ... ok
test agentic::growth_adapter::tests::legacy_keys_are_migrated_into_state_fields ... ok
test agentic::growth_adapter::tests::legacy_keys_are_preserved_after_migration_and_save ... ok
test agentic::growth_adapter::tests::migration_is_idempotent_load_save_load ... ok
test agentic::growth_adapter::tests::modified_state_round_trips_through_save_and_load ... ok
test agentic::growth_adapter::tests::system_clock_returns_reasonable_timestamp ... ok

test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 1135 filtered out; finished in 0.28s
```

**25 passed, 0 failed**（含 9 个新增 `boost_turn_topics` 测试 + 16 个既有测试）✓

### 命令 3：`cargo test -p northhing-core --features product-full memory_db`

```
test service::agent_memory::memory_db::tests::boost_keyword_increases_weight ... ok
test service::agent_memory::memory_db::tests::boost_keyword_respects_cap ... ok
test service::agent_memory::memory_db::tests::decay_weights_respects_floor ... ok
test service::agent_memory::memory_db::tests::delete_fact_removes_from_fts ... ok
test service::agent_memory::memory_db::tests::get_stale_facts_filters_and_orders ... ok
test service::agent_memory::memory_db::tests::keyword_weight_affects_scored_fact ... ok
test service::agent_memory::memory_db::tests::open_creates_tables ... ok
test service::agent_memory::memory_db::tests::ranking_fuses_three_factors ... ok
test service::agent_memory::memory_db::tests::segment_for_fts_bigram ... ok
test service::agent_memory::memory_db::tests::status_filter_hides_superseded ... ok
...（共 21 个）

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 1139 filtered out; finished in 0.20s
```

**21 passed, 0 failed**（既有 `boost_keyword` / `decay_all_weights` / `get_keyword_weight` 测试全过，底线改动未引入回归）✓

### 命令 4：`cargo test -p northhing-core --features product-full auto_memory`

```
test service::agent_memory::auto_memory::tests::prompt_injection_degrades_when_facts_file_unreadable ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_facts_includes_remembered_facts_section ... ok
test service::agent_memory::auto_memory::tests::prompt_injection_with_select_facts_budget_limit ... ok
...（共 7 个）

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1153 filtered out; finished in 0.10s
```

**7 passed, 0 failed** ✓

### 命令 5：`cargo test -p northhing-agentic-growth`

```
test topics::extract::tests::pure_ascii_filters_stopwords_and_short_tokens ... ok
test topics::extract::tests::pure_cjk_keeps_contiguous_run_as_one_topic ... ok
test topics::extract::tests::mixed_cjk_ascii_contains_both_kinds ... ok
...（共 121 个）

test result: ok. 121 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests northhing_agentic_growth
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 measured; finished in 0.00s
```

**121 passed, 0 failed**（crate 侧一行未改，全过）✓

### 命令 6：`node scripts/check-core-boundaries.mjs`

```
Core boundary check passed.
```

✓

## 7. 测试 4 与测试 5 的数值证据

通过临时 `println!` 测试（运行后已删除，未进 commit）采集的精确值：

| 测试 | 场景 | 实测值 | 断言 | 结论 |
| --- | --- | --- | --- | --- |
| 测试 5 | `get_keyword_weight("never-mentioned")` | **1** | `== 1.0`（epsilon） | 从未提及 = 基线 1.0 ✓ |
| 测试 4 | boost 1 次后 500 次空输入 decay | **1** | `>= 1.0`（epsilon） | floor=1.0 钳制生效，永不跌破基线 ✓ |
| 测试 1b | 第 2 次提及 | **1.98** | `> 1.0` 且落在 `[1.95, 2.0]` | `(1.0+1.0)*0.99 = 1.98` ✓ |
| 测试 2 | 第 3 次提及 | **2.9502** | `> 第2次(1.98)` | `(1.98+1.0)*0.99 = 2.9502`，严格递增 ✓ |

测试 4 证明：首次提及后权重 = 1.0，500 次 decay 后仍 = 1.0（每次 `MAX(1.0*0.99, 1.0)` = 1.0），底线裁定落地正确。

测试 5 证明：从未提及的关键词返回 1.0，与首次提及经冷却后的 1.0 一致，基线单调性成立。

## 8. 测试 6 降级说明

`related_keywords` 列只在 `boost_keyword` 内部被读取（`memory_db.rs:596` 的 SELECT），无对外读取 API。brief §4 测试 6 已预见此情况。采用降级断言：输入 `"rust python javascript"`（3 个话题）后，断言每个话题都有可检索的权重行（均在基线 1.0），且 `never-mentioned` 关键词仍返回基线（证明无杂散行）。`related_keywords` 的内容正确性由 `boost_keyword` 既有测试间接覆盖。

## 9. git 状态

```
git log --oneline -1
27c9738 fix(growth): wire topic weight boosting so retrieval weights can rise

git status --short
（空--工作树干净）
```

- 分支：`feat/growth-core-0804`
- HEAD：`27c9738`（基线 `985bbb9` 之后一个 commit）
- 改动：2 files changed, 326 insertions(+), 1 deletion(-)

## 10. 预检发现的 brief 缺陷（保留作修订依据）

> 本节是 BLOCKED 阶段的分析，实现已按用户裁定完成。保留此节供后续修订 brief §2.1 / §4 测试 1 时参考。

### 10.1 矛盾

brief §2.1（衰减底线 1.0）与 brief §4 测试 1（断言 `> 1.0`）在 `boost_keyword` 实际行为下数学上不可同时满足：

- `boost_keyword` 首次提及执行 INSERT，权重置为 **1.0**（`memory_db.rs:629`，非基线 +1.0）
- decay SQL `MAX(weight * 0.99, floor)`，对 1.0 + floor=1.0：`MAX(0.99, 1.0)` = **1.0**
- 单次首次提及 + 配对 decay 后权重 = 1.0，不满足测试 1 的 `> 1.0`

brief §2.1 的「每次提及 +1.0」措辞对首次提及不成立（首次是「置为 1.0」）。根因是 brief 作者误以为 INSERT 是「基线 +1.0」。

### 10.2 用户裁定（解法 A 加强版）

- 权重区间仍 `[1.0, 5.0]`，底线仍 1.0，`boost_keyword`/`decay_all_weights`/`search_facts` 一行不改
- 语义澄清：首次提及 = 基线 1.0（设计如此），第二次起才升温
- 测试 1 拆为 1a（首次 = 1.0，epsilon 比较）+ 1b（第二次 > 1.0，落 `[1.95, 2.0]`）
- 测试 2 比较第 2 次与第 3 次（非第 1 次）
- 语义说明写入 `boost_turn_topics` 文档注释（英文）

## 11. 与 brief 的偏离

1. **测试 1 替换为 1a + 1b**（用户裁定授权）。brief §4 测试 1 原文断言 `> 1.0` 不可满足，按裁定改为：1a 断言首次提及 `== 1.0`（epsilon，消息含 "first mention equals the never-mentioned baseline by design"）；1b 断言第二次提及 `> 1.0` 且落 `[1.95, 2.0]`。
2. **测试 2 比较对象调整**（用户裁定授权）。从「3 次 vs 1 次」改为「3 次 vs 2 次」，避开首次提及=基线的陷阱。
3. **§2.1「每次提及 +1.0」措辞修正**（用户裁定授权）。函数文档注释采用用户裁定的权威口径：首次提及置为 1.0、第二次起 +1.0。

除以上 3 点（均经用户明确授权）外，无其它偏离。

## 12. 硬约束遵守情况

| 约束 | 遵守 |
| --- | --- |
| 只改 2 个文件 | ✓ growth_adapter.rs + turn_persist.rs |
| 未改 SQL/schema/表/列/依赖 | ✓ |
| warn-only，无 `?` 传播 | ✓ |
| 非测试代码禁 unwrap/expect/panic | ✓（测试内用 unwrap，允许） |
| English-only、无 emoji | ✓ |
| 禁止 cargo fmt | ✓（手工 4 空格对齐） |
| growth_adapter.rs < 800 行 | ✓（638 行） |
| 不改检索打分、不引双层打分 | ✓ |
| crate 侧一行不改 | ✓ |
| boost_keyword/decay_all_weights/get_keyword_weight/search_facts 实现不改 | ✓ |
