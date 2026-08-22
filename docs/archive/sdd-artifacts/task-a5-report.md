# Task A5 Report — negation.rs（用户显式否定检测）

## 状态

**DONE**

## 基本信息

- 文件：`src/agentic/src/negation.rs`
- 行数：560（含测试）
- 分支：`feat/growth-a5`
- 基线：`7e96126`
- 只改了一个文件，未碰任何其他文件。

## §5 验证命令原始输出

### `cargo test -p northhing-agentic-growth`

```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 19.05s
     Running unittests src\lib.rs (target\debug\deps\northhing_agentic_growth-f6dc5dbd6f97d99a.exe)

running 28 tests
test error::tests::error_display_includes_context ... ok
test negation::tests::case_insensitive_english ... ok
test negation::tests::english_fact_is_wrong ... ok
test negation::tests::chinese_stop_remembering ... ok
test negation::tests::chinese_preference_replaced ... ok
test negation::tests::chinese_fact_is_wrong ... ok
test negation::tests::english_preference_replaced ... ok
test negation::tests::no_hit_empty_or_whitespace ... ok
test negation::tests::no_hit_false_friend_ji ... ok
test negation::tests::parse_out_of_range_dropped ... ok
test negation::tests::parse_zero_candidates_always_empty ... ok
test negation::tests::no_hit_vague_negative_chinese ... ok
test negation::tests::parse_duplicates_deduped ... ok
test negation::tests::parse_malformed_returns_empty ... ok
test negation::tests::parse_negative_float_string_dropped ... ok
test negation::tests::english_stop_remembering ... ok
test negation::tests::parse_simple_valid ... ok
test negation::tests::parse_with_json_fence ... ok
test negation::tests::parse_with_surrounding_prose ... ok
test negation::tests::no_hit_not_great ... ok
test negation::tests::priority_fact_is_wrong_over_preference ... ok
test negation::tests::prompt_candidates_numbered_without_fact_id ... ok
test negation::tests::prompt_contains_user_message_tags_and_original_text ... ok
test negation::tests::same_kind_earliest_phrase_wins ... ok
test negation::tests::target_hint_capped_at_60_chars ... ok
test negation::tests::target_hint_extracted ... ok
test negation::tests::prompt_empty_candidates_does_not_panic ... ok
test negation::tests::target_hint_none_when_nothing_after ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests northhing_agentic_growth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### `cargo check -p northhing-agentic-growth`

```
    Checking northhing-agentic-growth v0.2.10 (E:\agent-project\northing\.worktrees\growth-a5\src\agentic)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.31s
```

两者均通过，无 warning、无 error。

## 最终采用的三张短语表全文

### StopRemembering

```rust
const STOP_REMEMBERING_PHRASES: &[&str] = &[
    "别再记",
    "不要再记",
    "别记着",
    "忘掉",
    "忘了这条",
    "删掉这条",
    "不用记",
    "stop remembering",
    "forget that",
    "forget about",
    "don't remember",
];
```

### FactIsWrong

```rust
const FACT_IS_WRONG_PHRASES: &[&str] = &[
    "记错了",
    "你记错",
    "那条是错的",
    "这条不对",
    "搞错了",
    "that's wrong",
    "that is wrong",
    "you got it wrong",
    "incorrect memory",
];
```

### PreferenceReplaced

```rust
const PREFERENCE_REPLACED_PHRASES: &[&str] = &[
    "改用",
    "不再用",
    "改成用",
    "现在改用",
    "以后改用",
    "switched to",
    "now i use",
    "no longer use",
    "not anymore",
];
```

## Git

```
$ git log --oneline -1
07b986f feat(growth): add conservative explicit-negation detection

$ git status --short
（空 — 工作区干净）
```

## 偏离及原因

无偏离。所有 §2 规格、§3 测试、§4 硬约束全部满足。
