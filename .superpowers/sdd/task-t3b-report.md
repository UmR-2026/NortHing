# Task T3b Report: 自我认知注入改走 store + 稠密路径门禁（growth-core-0804）

- 分支：`growth-core-0804`（基线 39fadea，即 T3a 的 3 个 commit 之后）
- 状态：**DONE** — 实现 + 测试 + 全部验证通过；未 commit

## 改动清单

### 生产代码

1. `src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt.rs`
   - 删除 `use crate::agentic::identity::{identity_exists, load_identity}`；新增 `growth_adapter::{init_self_cognition_store, load_self_cognition}`、`service::agent_memory::{default_memory_db_path, MemoryDb}`、`northhing_agentic_growth::ports::SelfNote`。
   - 新增常量 `SELF_COGNITION_BUDGET_CHARS = 2000`（按 char 计数，非字节，因内容为中文）。
   - `build_workspace_persona_with_identity`（L33-56）改为三级优先级（brief T3b 3.1）：
     1. store 读成功且 ≥1 条非空笔记 → 渲染块；
     2. 否则回退 identity.md（与 T3b 前逐字节一致）；
     3. 两者皆无 → persona 原样返回。
     store 打开即触发一次性迁移（幂等，warn-only），首个 prompt 构建即可见迁移笔记。
   - 新增模块级 helper（L287-410）：
     - `build_self_cognition_block_from_store() -> Option<String>`：读全局 memory DB，warn-only（DB 打开失败 → None → 回退），迁移失败不影响读取。
     - `render_self_cognition_block(&[SelfNote]) -> Option<String>`：`# Self-cognition\n\n{bodies}\n\n`；旧→新排序、空行分隔、空白笔记跳过、预算 2000 char（不含标题）；溢出时保首条（最旧/基础）+ 从最新往回填充可容纳者，中间丢弃，无截断标记。
     - `select_notes_within_budget(&[&SelfNote], usize) -> Vec<&SelfNote>`：纯函数选择逻辑（含 `\n\n` 分隔符计数）。
     - `join_persona_and_block(persona, block)`：与 T3b 前相同空白规则（persona 非空才插 `\n\n`）。
     - `load_identity_for_prompt() -> Option<String>`：`exists()` + `read_to_string().ok()`，语义与旧 `identity_exists()`+`load_identity()` 逐字节一致，但经 `resolve_identity_path()` 解析以便测试注入。
   - 文件尾新增 `#[cfg(test)] #[path = "system_prompt_tests.rs"] mod tests;`。

2. `src/crates/assembly/core/src/service/agent_memory/mod.rs`
   - 生产 re-export 增加 `resolve_identity_path`（供 prompt_builder 使用）；`#[cfg(test)]` 增加 `with_test_identity_path, IdentityPathGuard`（供测试隔离 identity.md）。

3. `src/crates/assembly/core/src/service/agent_memory/dream.rs`
   - 仅测试模块新增 D9 负向测试（见下）；生产代码未动。

### 测试（新增/重写）

4. `src/crates/assembly/core/src/agentic/agents/prompt_builder/system_prompt_tests.rs`（18 例，重写先前遗留的半成品文件）
   - **预算/选择**：`budget_keeps_all_when_under_budget`、`budget_overflow_keeps_first_fills_newest_drops_middle`（保首条+最新、丢中间）、`budget_single_note_within_budget`、`budget_counts_chars_not_bytes`（中文按字符计）、`render_block_respects_total_budget`、`blank_notes_skipped`、`multiple_notes_render_oldest_first_blank_separated`。
   - **join 空白规则**：`join_persona_and_block_empty_persona_no_leading_newline`、`join_persona_and_block_nonempty_persona_gets_separator`。
   - **优先级/回退**：`store_yields_nothing_falls_back_to_identity_md`（空 store → None）、`store_open_failure_falls_back_to_identity_md_and_prompt_builds`（坏 DB 路径 → 回退且 persona 仍构建）、`neither_source_returns_persona_unchanged`、`persona_interaction_preserved_end_to_end`（store 路径完整走 PromptBuilder::build）。
   - **§6.1 行为等价证据**：`empty_store_with_identity_present_output_identical_to_today`（空 store + identity.md 存在 → 输出与 T3b 前逐字节一致）、`single_migrated_note_reproduces_current_output`（迁移后单笔记渲染与旧格式一致）、`equivalence_case_a_no_trailing_newline` / `equivalence_case_b_trailing_newline` / `equivalence_case_c_utf8_bom`（identity.md 内容边界情形逐字节一致）。

5. `src/crates/assembly/core/src/service/agent_memory/dream.rs` tests
   - `dream_payload_never_contains_self_cognition_sentinel`（D9 负向，brief 3.3）：在隔离 DB 的 `self_cognition` 表种入哨兵 `"T3B_DENSE_PATH_SENTINEL_我是自我认知标记"`，按 `run_dream_sweep` 同路径 `build_dream_messages`（只读 facts + judge_mom）构造 payload，断言哨兵缺席 + fact 文本在场（防空洞断言）。证明稠密路径（dream）读不到自我认知表。

## 验证结果

```
cargo check -p northhing-core --features product-full     # Finished, 19 warnings（= 基线，未增）
cargo test -p northhing-core --features product-full      # 1225 passed, 0 failed, 1 ignored
cargo test -p northhing-core --features product-full system_prompt   # 21 passed（含新增 18）
cargo test -p northhing-core --features product-full self_cognition  # 19 passed（无回归）
cargo test -p northhing-core --features product-full dream           # 7 passed（含新增 D9 哨兵测试）
cargo test -p northhing-agentic-growth                     # 139 passed（= 基线，identity 仓未动）
node scripts/check-core-boundaries.mjs                    # Core boundary check passed.
git diff --check                                          # 干净（CRLF 警告为仓库既有噪声）
```

## 约束合规

- 未 commit；仅触碰本任务 4 个文件（system_prompt.rs、system_prompt_tests.rs 新增、dream.rs 测试、mod.rs re-export）。
- 未运行 `cargo fmt`（brief 禁令）；未动 `scripts/core-boundaries/**`。
- 非测试代码无 unwrap/expect/panic；store 打开失败 warn-only，绝不阻断 prompt 构建。
- 中文注释沿用仓库既有风格；测试名/输出无 emoji；新增英文错误文案。
- `northhing-agentic-growth` 未改动（139 基线保持）；`MemoryDb`/store 接口未变。
- 既有 19 warning 基线保持（含 `PathBuf` 等 unused-import 由 crate 级 allow 抑制，与基线一致）。

## 语义变化披露

1. persona 下自我认知段的内容来源从 identity.md 切到 store（优先）；store 迁移仅一次性导入 identity.md，此后 identity.md 后续编辑不再被重新导入（T3a 语义）。
2. 回退路径输出与 T3b 前逐字节一致（§6.1 测试证明）；store 命中时输出与迁移前的 identity.md 渲染逐字节一致（同一标题/空白格式）。
3. 首次 prompt 构建会打开全局 memory DB（可能触发建库/迁移）——与 T3a 设计一致，warn-only，失败不影响 prompt。
4. D9 门禁：dream 路径按既有 `build_dream_messages`（只读 facts + judge_mom）不变，新增测试锁定"读不到自我认知表"，无生产改动。

## 备注

- 工作树中 `system_prompt.rs`、`mod.rs` 的改动部分继承自先前遗留尝试（与本次实现重叠），已全部复核为本任务语义；`system_prompt_tests.rs` 为本次完全重写。
