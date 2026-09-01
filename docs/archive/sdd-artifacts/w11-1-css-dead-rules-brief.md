# Task Brief — W11-1: css.rs 死规则群清理 + 闸口游戏回滚

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/desktop/src/ui_dioxus/` css 相关文件。
病灶：`blind-review-css-2026-08-29.md`（逐项带 file:line；**行号可能漂移，执行前 rg 核实存在性**——深审 file:line 断言先抽查的纪律）。

## Spec（验收标准）

### 1. 死规则/死声明删除（每个删除点必须先 rg 证明零引用，证据进 report）

- `.depth-bar / .depth-seg / .depth-note` 死规则块（盲审报 css.rs:504-513，10 条规则）
- `padding-right:160px` 被 136px 覆盖的死声明（报 :198）
- membrane-node 级联累积死声明（报 :130,156,259,306-311，12 处）
- 死函数 `inject_stylesheet_html`（报 :753，rg+codegraph 双确认零调用——你再独立确认一遍）
- 盲审发现的其它死项若你核实为真，一并清（report 逐条列）

### 2. 闸口游戏回滚（ readability 恢复）

- css.rs:86 被硬塞一行的三规则（close-btn/degraded-banner/close-btn:hover）恢复为正常一行一条；W9-6 合并的 fold-btn/tag-x/diff-add/diff-del 同样恢复。
- 死规则删除腾出的行数预算内完成；收口 css.rs 行数应明显下降。

### 3. 注释矛盾修复（3 组）

- 头注释"待提取 .css" vs 实际已 include_str（:9-14）
- doc 记录已清空的 `#room-scrim`（:51-54）
- `--gem-mid=123px` 注释 vs 声明 85px（:244）——以声明值为准改注释

### 4. manifest

- css.rs ceiling 830 → 同 commit **下调到实测值**。

### 5. 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`：0 error
2. `+stable-msvc test -p northhing --lib`：全绿
3. `node scripts/verify-rot-budget.mjs`：绿
4. 视觉风险说明：被删规则均要求零引用证据；若某规则有引用但疑似无效，**不删**，记观察项。

## Global Constraints

1. 分层边界：只动 `src/apps/desktop` + manifest。
2. 日志英文无 emoji；零新增日志。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止整树 git 操作；开工先 `git status`。
4. rot-budget：ceiling 只降不升。
5. commit：恰好一个；不含 `.superpowers/`。
6. 行为变化仅限：死规则删除 + 注释修正 + 行格式恢复；样式语义零变化。
7. 遇编译错误先加载对应 rust skill。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因。
- 返回消息含：状态 / commit SHA / git show --stat / 删除点逐条零引用证据 / 验证输出尾部 / css.rs 新行数 / 偏离清单。
