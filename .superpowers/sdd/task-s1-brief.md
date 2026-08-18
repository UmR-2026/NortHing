# Task S-1：拆分两个贴顶文件（纯搬移，行为等价）

## 1. 目的

两个文件已达 **799 行**，硬上限 800，导致后续任何改动都无法落地（上一单 R-2 只能靠"净增 0 行"硬挤）。本单**只搬代码、不改逻辑**，给后续任务腾出空间。

⚠️ 本单的验收核心是**行为等价**：任何逻辑改动、签名语义改变、条件调整都是缺陷。除文件位置与可见性修饰符外，代码内容应当逐字节保留。

## 2. 现状（已实测，不必重新怀疑）

| 文件 | 总行 | 生产 | 内联测试 |
|---|---|---|---|
| `src/crates/assembly/core/src/agentic/growth_adapter.rs` | 799 | 246（`:1-246`） | 552（`:247-799`） |
| `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs` | 799 | 730（`:1-730`） | 68（`:731-799`） |

两者构成不同，故**拆法不同**（见 §3、§4）。

先例（本仓已做过同类拆分）：`memory_db/dream.rs` 子模块就是为「`memory_db.rs` 841 行警戒线」而拆的。惯例已存在，照做即可。

## 3. 拆分 A：`growth_adapter.rs`（测试搬出即可，生产代码不动）

生产代码只有 246 行，空间充足；瓶颈全在 552 行内联测试。故：

- **production 全部留在原文件** `agentic/growth_adapter.rs`，内容不动。
- 把 `#[cfg(test)] mod tests { ... }`（`:247-799`）整块搬到新文件 **`agentic/growth_adapter/tests.rs`**，原处改为一行声明：
  ```rust
  #[cfg(test)]
  mod tests;
  ```
- Rust 2018 允许 `growth_adapter.rs` 与目录 `growth_adapter/` 并存，无需 `#[path]` 属性。
- 测试内的 `use super::*;` 语义在子模块中仍指向父模块（原文件），careful：搬过去后 `super` 依然是 `growth_adapter`，故原有 `use super::...` **不需要改**。若确有解析不到的符号，优先加显式 `use`，不要改生产代码的可见性。

⚠️ 若本仓对「单文件模块 + 同名目录」有更强的既有惯例（例如统一用 `xxx_tests.rs` + `#[path]`），照既有惯例做，并在报告里说明你依据哪些既有文件判断的。参考既有沿袭：`agents/prompt_builder/tests.rs`、`agents/registry/tests.rs`、`session/compression/fallback/tests.rs`。

**必须保持**：模块路径 `northhing_core::agentic::growth_adapter` 不变（`src/agentic/AGENTS.md:9` 有引用）。

## 4. 拆分 B：`turn_persist.rs`（真正的模块切分）

730 行生产代码需要切走一块。切法已定，**不要自己另选边界**：

把「facts 蒸馏钩子 + 其门禁」整块搬到同级新文件 **`agentic/coordination/dialog_turn/turn_persist_facts.rs`**：

搬走的内容（连续四块 + 测试）：
1. `struct SessionSignals`（`turn_persist.rs:23`）
2. `async fn resolve_distill_signals`（`:363`）
3. `fn should_distill_facts`（`:393`）
4. `async fn append_facts_entry`（`:487`）与 `async fn load_last_assistant_text`（`:658`）
5. `#[cfg(test)] mod tests`（`:731-799`，11 条门禁测试，全部与上述内容相关）

留在 `turn_persist.rs` 的内容：
- `persist_completed_dialog_turn` / `persist_cancelled_dialog_turn` / `persist_failed_dialog_turn`
- `finalize_persisted_turn_in_workspace_if_needed`（编排入口，继续调用搬走的钩子）
- `append_episode_log_entry`（**不要搬**，它与 facts 无关；这也保持了「门禁只管 facts 不管 episode」这条既有裁定在结构上可见）

### 4.1 实现要点（Rust 细节，已为你查清，照做别推导）

- 这些函数都是 `impl ConversationCoordinator` 的固有方法。本仓**已有多文件为同一类型分别 `impl` 的既有做法**（`coordinator_session.rs` / `sub_handle_in.rs` / `sub_handle_out.rs` / `turn_cancel.rs` 等），故在新文件里再写一个 `impl ConversationCoordinator { ... }` 块即完全合法且合乎既有风格。
- ⚠️ **可见性必坑**：固有 impl 里的**私有**方法只在其定义模块及其后代可见。搬到新的同级模块后，`turn_persist.rs` 会**看不见**它们。故被跨文件调用的方法需要提升可见性，用**最小够用**的 `pub(super)`（不要直接上 `pub(crate)`，除非 `pub(super)` 编译不过）。仅内部使用、不跨文件调用的保持私有。
- `struct SessionSignals` 同理，按需给最小可见性。
- 新文件需要在 `dialog_turn/mod.rs`（58 行）里登记 `mod turn_persist_facts;`，可见性与相邻 `turn_persist` 的登记方式保持一致。

## 5. 硬约束

1. **行为等价**：不改任何逻辑、条件、常量、日志文本、签名语义（可见性修饰符除外）。不做"顺手优化"，不重命名，不合并重复代码，不调整既有注释文字。
2. **每个产出文件 < 800 行**，且每个都要留出余量（报告里给出各文件实测行数）。行数用 `(Get-Content -LiteralPath <file> -Encoding UTF8).Count` 判定，**不要**用 `Measure-Object -Line`。
3. **测试总数与断言逐条不变**：growth_adapter 的 **27 条**、turn_persist 的 **12 条**必须原样通过（不要新增、不要删除、不要改断言）。
4. **禁止 `cargo fmt`**（有污染前科）。搬动时手工维持原缩进；因脱离 `impl` 块内层缩进而必须整体减少缩进的，视为可接受的必要改动（会在 diff 里体现为整块缩进变化，请在报告里明确说明哪些块变了缩进）。
5. 非测试代码不新增 `unwrap`/`expect`/`panic!`；日志/注释 English-only 无 emoji（既有中文测试字面量原样保留）。
6. **不动**其它任何文件的逻辑，尤其不要碰 `facts.rs` / `memory_db.rs` / `dream.rs` / `scheduler.rs` / `state.rs`。

## 6. 验证（全部执行，**完整原始 stdout+stderr** 贴进报告，不要摘录节选）

前置：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`

1. `cargo check -p northhing-core --features product-full` —— warning **基线 19，不得新增**（拆分若产生 `unused_imports` 等新 warning，必须清理干净再交）
2. `cargo test -p northhing-core --features product-full growth_adapter` —— **27 条**全绿
3. `cargo test -p northhing-core --features product-full turn_persist` —— **12 条**全绿
4. `cargo test -p northhing-core --features product-full memory_db` —— 21 条无回归
5. `cargo test -p northhing-agentic-growth` —— 131 条无回归
6. `node scripts/check-core-boundaries.mjs` —— **exit 0**（若新文件触发布局规则，按规则要求调整**文件位置/命名**去顺应规则；**不要改** `scripts/core-boundaries/` 里的规则脚本——那需要编排者批准，遇到就报 BLOCKED）
7. 全部涉及文件的实测行数

## 7. 报告

写到 `E:\agent-project\northing\.superpowers\sdd\task-s1-report.md`：
- 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
- 每个产出文件的实测行数（拆前/拆后对照表）
- 拆分 A 你选了哪种既有惯例、依据哪些既有文件
- 提升了可见性的符号清单（每个：原可见性 → 新可见性 → 为什么最小够用）
- 缩进整体变化的代码块清单
- **凡是不得不做的非纯搬移改动，逐条列出并说明为什么不可避免**（这是编排者最关心的一节）
- §6 七条的完整原始输出
- 改动/新增文件清单

## 8. 工作目录与提交

- `E:\agent-project\northing\.worktrees\growth-core-0804`（分支 `feat/growth-core-0804`，当前 HEAD `d1d6d92`）
- 建议**两个 commit**（拆分 A 一个、拆分 B 一个），便于逐块回溯；message 用 `refactor(growth): ` 前缀。
- 提交前 `git status --short` 确认无意外文件；**不要**提交 `.superpowers/` 下任何文件。

## 9. 纪律

- brief 是需求唯一来源。发现 brief 与代码矛盾（例如给出的行号对不上、指定的切分边界会破坏编译） → **停下报 BLOCKED**，不要自行改边界。
- 不要自派子代理。
- 不要预判审查者。
