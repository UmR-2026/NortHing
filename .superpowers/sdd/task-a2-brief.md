# Task A2 Brief — topics/extract.rs（话题抽取纯函数）

> 需求唯一来源。本文件之外的信息不得作为需求依据。
> 工作目录（唯一）：`E:\agent-project\northing\.worktrees\growth-a2`，分支 `feat/growth-a2`，基线 `7e96126`
> 报告：`E:\agent-project\northing\.superpowers\sdd\task-a2-report.md`（在 worktree 之外，不进 commit）

## 0. 你只能改一个文件

- `src/agentic/src/topics/extract.rs`

**其它任何文件一行都不能改**（`topics/mod.rs` 已预声明该模块）。有 4 个并行任务在同一 crate 的其它文件上作业，越界就会撞车。

## 1. 背景

agent 记忆系统的重要度分两层：**话题权重（主）+ 条目分数（次）**。本任务实现最底层的一步：从一条记忆文本里抽出话题标签。

约束现实：本 crate **不允许引入任何新依赖**（不能用分词库、不能用 regex crate 以外的东西——实际连 regex 都不在依赖表里，所以只能用标准库字符处理）。因此话题抽取必须是**朴素、确定性、可测试**的规则，不追求语义准确，只追求：同样输入永远同样输出，且不产生垃圾话题。

## 2. 规格

### 2.1 常量（全部 `pub const`，集中在文件顶部）

```rust
pub const MAX_TOPICS: usize = 3;              // 一条记忆最多挂 3 个话题
pub const MIN_ASCII_TOKEN_CHARS: usize = 3;   // ASCII 词最短长度
pub const MIN_CJK_RUN_CHARS: usize = 2;       // CJK 连续段最短长度
pub const MAX_TOPIC_CHARS: usize = 24;        // 单个话题最长长度（超长截断）
```

### 2.2 主函数

```rust
/// Extracts up to MAX_TOPICS topic labels from memory text.
///
/// Deterministic and dependency-free: the same input always yields the same
/// output in the same order.
pub fn extract_topics(text: &str) -> Vec<String>
```

规则（**按序实现**）：

1. **切分**：把文本切成 token。分隔符 = 任何 ASCII 空白与 ASCII 标点（`is_ascii_punctuation`），以及全角标点 `，。！？；：、（）【】「」《》""''—…`。
2. **ASCII token**：仅由 ASCII 字母/数字/`-`/`_`/`.`/`+`/`/` 组成的段（例如 `pnpm`、`node-18`、`src/agentic`、`C++`）。
   - 转小写
   - 长度 < `MIN_ASCII_TOKEN_CHARS` 丢弃（这样 `is`、`a`、`的` 之类不会进来；但注意 `c++`、`npm` 长度 3 保留）
   - 命中停用词表丢弃
   - 纯数字（全是 ASCII 数字）丢弃
3. **CJK 段**：连续的 CJK 字符（判定：`char` 落在 `\u{4E00}..=\u{9FFF}` 或 `\u{3400}..=\u{4DBF}`）视为一个段。
   - 长度 < `MIN_CJK_RUN_CHARS` 丢弃（单字不成话题）
   - 命中中文停用词表丢弃
   - **不做分词**：整段作为一个话题（这是刻意的朴素策略，必须写进函数文档注释说明理由）
4. **截断**：任何话题超过 `MAX_TOPIC_CHARS` 个**字符**（不是字节）→ 取前 `MAX_TOPIC_CHARS` 个字符。必须按 `char` 截断，禁止用字节切片（会切坏 UTF-8）。
5. **去重**：保持首次出现顺序去重（大小写已在第 2 步归一）。
6. **截顶**：最多返回 `MAX_TOPICS` 个，取前 N 个（首次出现顺序）。

### 2.3 停用词表

两张 `const` 数组（ASCII 与 CJK 各一张），元素排序无要求但**必须小写**。

- ASCII（至少包含，可再补，不要超过 60 个）：`the` `and` `for` `with` `that` `this` `you` `your` `not` `but` `use` `used` `using` `have` `has` `will` `would` `should` `can` `could` `are` `was` `were` `been` `all` `any` `from` `into` `about` `just` `like` `want` `need` `make` `made` `get` `got` `let` `now` `then` `than` `too` `very` `really` `also` `please` `thanks`
- CJK（至少包含，可再补，不要超过 60 个）：`的` 类单字不会进来（长度 <2 已被过滤），所以这张表放**双字虚词**：`这个` `那个` `一个` `我们` `你们` `他们` `什么` `怎么` `因为` `所以` `但是` `如果` `可以` `应该` `已经` `还是` `或者` `以及` `然后` `现在` `以后` `之前` `之后` `一下` `一些` `很多` `没有` `不是` `就是` `这样` `那样` `这里` `那里`

停用词判定：**整词相等**（不是包含）。

### 2.4 辅助函数（可选但推荐，便于测试）

```rust
pub fn is_cjk_char(c: char) -> bool;
pub fn truncate_chars(s: &str, max_chars: usize) -> String;
```

## 3. 测试（每条都要有，用 `assert_eq!` 断言完整 Vec，不要只断言长度）

1. 纯 ASCII：`"I prefer pnpm for dependency install"` → 期望包含 `pnpm`、`prefer`、`dependency` 一类结果（写死你实现后的实际期望值，但必须证明 `for`/`i` 被过滤）
2. 纯中文：`"用户偏好使用中文回复"` → 整段 CJK 被切成若干段（按标点/ASCII 边界），断言实际输出
3. 中英混排：`"以后依赖安装都用 pnpm，不要用 npm"` → 断言同时含 CJK 段与 `pnpm`、`npm`
4. 停用词过滤：ASCII 与 CJK 各一条
5. 单字/短词过滤：`"a b c 的 了"` → 空 Vec
6. 纯数字过滤：`"2026 18"` → 空 Vec
7. 超长话题按字符截断：构造一个 30+ 字符的 CJK 段，断言结果长度为 `MAX_TOPIC_CHARS` 个**字符**（用 `chars().count()` 断言），且是合法 UTF-8（能正常比较字符串即证明）
8. 去重：`"pnpm pnpm PNPM"` → 只有一个 `pnpm`
9. 截顶：构造 5 个合法话题的输入，断言只返回 3 个且是前 3 个
10. 空输入 / 纯标点 / 纯空白 → 空 Vec
11. **确定性**：同一输入调用两次结果相等
12. 大小写归一：`"PNPM"` → `pnpm`

## 4. 硬约束

- 只改第 0 节那一个文件。
- 零依赖：只用标准库。不得改 `Cargo.toml`，不得 `use` 任何外部 crate（含 `regex`、`serde`）。允许 `use std::...`。
- 无 IO、无时钟、无随机：这是纯函数模块。
- **按字符处理，不按字节**：任何 `&s[..n]` 形式的字节切片都是 bug（中文会 panic）。
- 注释与文档 **English-only、无 emoji**。测试里的中文**字符串字面量是允许的**（它们是被测数据），但注释和测试函数名必须英文。
- **禁止运行 `cargo fmt`**（本仓两次污染前科）。手工对齐：4 空格缩进。
- 文件 < 800 行（预计 200-350 行含测试）。
- 不要实现别的模块（score / competition / ports / negation 归其它任务）。

## 5. 验证（必须实际执行并把命令与原始输出贴进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo check -p northhing-agentic-growth
```

不要跑 `cargo check --workspace`（被上游 embed-resource 阻断，与本任务无关）。

## 6. 交付

1. 在本 worktree 内提交一个 commit：`feat(growth): add dependency-free topic extraction`
   提交前 `git status --short` 确认只有那一个文件。
2. 报告写到 `E:\agent-project\northing\.superpowers\sdd\task-a2-report.md`，包含：
   - 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
   - 文件行数
   - §5 命令原始输出（含测试名与通过数）
   - 你为 §3 各条测试写死的**实际期望值**（便于审查者复核规则是否自洽）
   - `git log --oneline -1`、`git status --short`
   - 与本 brief 的任何偏离及原因
