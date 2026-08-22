# Task A5 Brief — negation.rs（用户显式否定检测）

> 需求唯一来源。本文件之外的信息不得作为需求依据。
> 工作目录（唯一）：`E:\agent-project\northing\.worktrees\growth-a5`，分支 `feat/growth-a5`，基线 `7e96126`
> 报告：`E:\agent-project\northing\.superpowers\sdd\task-a5-report.md`（在 worktree 之外，不进 commit）

## 0. 你只能改一个文件

- `src/agentic/src/negation.rs`

**其它任何文件一行都不能改**（`lib.rs` 已预声明该模块）。有 4 个并行任务在同一 crate 的其它文件上作业，越界就会撞车。

## 1. 背景与设计裁定

整个成长系统里**只有一条硬作废通道**：用户**显式**否定一条记忆（"别再记着我用 npm 了"、"那条是错的"）。管家（judge-mom）永远无权作废，只能改权重。

因此本模块是权限最敏感的一环，设计原则是 **宁漏不误（precision over recall）**：

- 漏检 → 用户再说一次，成本低；
- 误检 → 无辜记忆被作废，用户信任崩塌，成本极高。

所以第一道关是**保守的显式短语匹配**（本模块），命中后还要经 LLM 二次确认（本模块只负责构造 prompt 与解析回复，不发请求）。模糊的负面情绪（"这个不太好"、"我不太喜欢"）**必须不命中**。

## 2. 规格

### 2.1 类型

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegationKind {
    /// "stop remembering X" / "forget X"
    StopRemembering,
    /// "that is wrong" / "you remembered it wrong"
    FactIsWrong,
    /// "I use Y now, not X" — replacement of a stated preference
    PreferenceReplaced,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NegationSignal {
    pub kind: NegationKind,
    /// The exact phrase that triggered detection (for audit trails).
    pub matched_phrase: String,
    /// Text after the trigger phrase, if any, as a weak hint for candidate lookup.
    pub target_hint: Option<String>,
}
```

### 2.2 短语表（`const` 数组，小写）

三张表，每 kind 一张。**必须是显式指令性表达**，不要放任何单纯表达偏好或不满的词。

- `StopRemembering`：`别再记` `不要再记` `别记着` `忘掉` `忘了这条` `删掉这条` `不用记` `stop remembering` `forget that` `forget about` `don't remember`
- `FactIsWrong`：`记错了` `你记错` `那条是错的` `这条不对` `搞错了` `that's wrong` `that is wrong` `you got it wrong` `incorrect memory`
- `PreferenceReplaced`：`改成用` `现在改用` `以后改用` `不用 x 了改用`（用通用形式：`改用` + `不再用`）、`switched to` `now i use` `no longer use` `not anymore`

**不得命中的反例（必须写成测试）**：`不太好` `不喜欢` `有点问题` `不太合适` `算了` `didn't like` `not great` `hmm`。

### 2.3 函数

```rust
/// Conservative first-pass detector. Returns None unless an explicit negation
/// phrase is present. Precision over recall by design: a miss costs one extra
/// user message, a false positive destroys a legitimate memory.
pub fn detect_explicit_negation(user_text: &str) -> Option<NegationSignal>;

/// Builds the (system_prompt, user_content) pair for the LLM confirmation step.
/// The caller sends it; this module performs no IO.
pub fn build_confirmation_prompt(
    signal: &NegationSignal,
    user_text: &str,
    candidates: &[(String, String)], // (fact_id, fact_text)
) -> (String, String);

/// Parses the confirmation reply. Expected shape: {"retire":[0,2]} with
/// zero-based indices into the candidate slice.
/// Robustness: tolerates surrounding prose and ```json fences; out-of-range and
/// duplicate indices are dropped; anything unparseable yields an empty Vec.
pub fn parse_confirmation(reply: &str, candidate_count: usize) -> Vec<usize>;
```

匹配规则（`detect_explicit_negation`）：

1. 输入转小写副本用于匹配（原文用于提取 `target_hint`）。
2. 依次检查三张表；**按 kind 优先级** `FactIsWrong` > `StopRemembering` > `PreferenceReplaced`（"那条记错了，改用 pnpm" 应判为 `FactIsWrong`）。
3. 同一 kind 内命中多个短语时，取在文本中**位置最靠前**的那个作为 `matched_phrase`。
4. `target_hint`：取匹配短语**之后**的内容，去首尾空白与标点，截断到 60 个**字符**（按 `char`，不按字节）；为空则 `None`。
5. 中文短语匹配用 `contains`；英文短语同样 `contains`（不要求词边界——短语本身足够长，误伤风险低）。

`build_confirmation_prompt` 要求：

- system prompt 明确：判定用户是否**明确要求**忘掉/否定列出的某条记忆；不确定时返回空数组；只输出 JSON。
- user content 必须把用户原话包在 `<user_message>` 与 `</user_message>` 之间，候选包在 `<candidates>` 里，每条一行形如 `[0] fact text`（用**序号**而非 fact_id 暴露给 LLM，避免 id 泄漏与幻觉 id）。
- 候选为空时仍返回合法 prompt（不 panic）。

`parse_confirmation` 要求：

- 支持 ```json 围栏与前后散文：定位第一个 `{` 与最后一个 `}` 之间的子串再解析（用 `serde_json`，它已在依赖表里）。
- 只接受 `retire` 字段为数字数组；非数组、负数、非整数、越界（`>= candidate_count`）一律丢弃该项。
- 去重后保持首次出现顺序。
- 解析失败 → 空 Vec（**不得**返回 Err、不得 panic）。

## 3. 测试（每条都要有）

**命中**：

1. 中文三 kind 各至少一条命中，断言 `kind` 与 `matched_phrase`
2. 英文三 kind 各至少一条命中
3. 优先级：`"那条记错了，以后改用 pnpm"` → `FactIsWrong`
4. 同 kind 多命中取最靠前短语
5. `target_hint` 提取：`"别再记我用 npm 这件事"` → hint 含 `npm`；`"忘掉"`（后面没内容）→ `None`
6. `target_hint` 超长按**字符**截断（构造 80+ 字符中文，断言 `chars().count() == 60`）
7. 大小写不敏感：`"Forget That"` 命中

**不命中（宁漏不误的证明，至少 6 条）**：

8. `"这个不太好"` → `None`
9. `"我不太喜欢这个方案"` → `None`
10. `"有点问题"` → `None`
11. `"not great"` → `None`
12. 空串 / 纯空白 → `None`
13. `"我记得你说过要用 pnpm"`（含"记"字但不是否定指令）→ `None`

**prompt 构造**：

14. 输出的 user content 同时包含 `<user_message>` 与 `</user_message>`，且包含用户原话
15. 候选以 `[0]` `[1]` 形式编号出现，且**不包含** fact_id 字符串
16. 空候选列表 → 不 panic，返回非空 system prompt

**解析**：

17. `{"retire":[0,2]}`，candidate_count=3 → `vec![0,2]`
18. ```json 围栏包裹 → 正常解析
19. 前后有散文（`"Sure, here: {\"retire\":[1]} done"`）→ `vec![1]`
20. 越界 `{"retire":[0,9]}`，count=2 → `vec![0]`
21. 重复 `{"retire":[1,1]}` → `vec![1]`
22. 负数 / 小数 / 字符串项 → 被丢弃
23. 坏 JSON / 空串 / `{}` / `{"retire":"all"}` → 空 Vec
24. `candidate_count == 0` → 任何输入都返回空 Vec

## 4. 硬约束

- 只改第 0 节那一个文件。
- 依赖只能用已在 `Cargo.toml` 里的：`serde_json`（解析用）、`tracing`（如需 warn）。**不得改 `Cargo.toml`**，不得引 `regex`。
- 无 IO、无网络：本模块只造 prompt 字符串与解析字符串，**绝不发请求**。
- **不得 panic**：非测试代码禁止 `unwrap()` / `expect()` / 字节切片 `&s[..n]`（中文会 panic，必须按 `char` 处理）。
- 注释与文档 **English-only、无 emoji**。测试里的中文字符串字面量是允许的（被测数据），但注释和测试函数名必须英文。
- prompt 文本本身用英文（system prompt 面向 LLM，仓库惯例英文）。
- **禁止运行 `cargo fmt`**（本仓两次污染前科）。手工对齐：4 空格缩进。
- 文件 < 800 行（预计 350-500 行含测试）。
- 不要实现别的模块（extract / score / competition / ports 归其它任务）。
- 不要调用或实现任何"执行作废"的动作（写库归宿主适配层）。本模块只**检测与解析**。

## 5. 验证（必须实际执行并把命令与原始输出贴进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-agentic-growth
cargo check -p northhing-agentic-growth
```

不要跑 `cargo check --workspace`（被上游 embed-resource 阻断，与本任务无关）。

## 6. 交付

1. 在本 worktree 内提交一个 commit：`feat(growth): add conservative explicit-negation detection`
   提交前 `git status --short` 确认只有那一个文件。
2. 报告写到 `E:\agent-project\northing\.superpowers\sdd\task-a5-report.md`，包含：
   - 状态：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
   - 文件行数
   - §5 命令原始输出（含测试名与通过数）
   - 你最终采用的三张短语表全文（便于审查者判断是否有过宽条目）
   - `git log --oneline -1`、`git status --short`
   - 与本 brief 的任何偏离及原因
