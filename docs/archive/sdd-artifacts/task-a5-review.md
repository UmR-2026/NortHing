# Task A5 Review — negation.rs

> Reviewer: judge (双判决 spec + quality)
> Worktree: `E:\agent-project\northing\.worktrees\growth-a5`
> Base/Head: `7e96126` → `07b986f`
> 唯一被改文件: `src/agentic/src/negation.rs` (560 行, 单 commit)

---

## 1. 判决摘要

- **SPEC: PASS**
- **QUALITY: PASS**
- **APPROVED WITH NOTES**

实现者严格照 brief 把所有 24 项测试、§2 类型/签名/短语表/解析规则、§4 硬约束逐条落地，`cargo test -p northhing-agentic-growth` 28 passed（27 个 negation + 1 个 error::tests）、`cargo check` 无 warning、单文件改动、文件 560 行（< 800）、无 IO/网络/panic/`regex`/`Cargo.toml` 改动。无 Critical，无破坏 spec 的 finding。

Notes 内容：brief 自身把若干**过宽**短语塞进了三张表（`忘掉`、`不用记`、`forget that`、`forget about`、`don't remember`、`not anymore`、`改用`、`搞错了`），实现者如实还原，但这与 brief §1 的"显式指令性"原则张力明显 —— 这是 **plan-mandated** 风险，需用户裁定（详见 §4 短语表裁定表）。

---

## 2. Findings

### Critical
无。

### Important

| # | file:line | 描述 | 建议 |
|---|----------|------|------|
| I-1 | negation.rs:51 (`忘掉`) | brief §2.2 列出此短语，但 brief §1 同时要求"必须是显式指令性表达"。`忘掉` 是 2 字通用词，常见误伤面太大，详见 §4 短语表行。 | 用户决策：是收紧（要求"忘掉那条/这个" + 词边界）还是接受漏报？ |
| I-2 | negation.rs:54 (`不用记`) | 同上。"这个不用记笔记"/"今天不用记日报" — 用户在当前回合说"不用记"，与"删除已存记忆"语义不同。LLM 二审能拦下多数，但第一关就有噪音。 | 用户决策：是否要删或收紧。 |
| I-3 | negation.rs:57 (`forget that`) | 子串匹配无词边界。"Don't forget that we have a meeting at 3pm" 误伤。 | 用户决策：是否要求词边界（如 `\bforget that\b`），或换更长的形式（"forget that fact"）。 |
| I-4 | negation.rs:58 (`forget about`) | "Don't forget about the deadline" — 典型误伤。"Let's forget about it" 才是真正指令。 | 用户决策：同 I-3，或换 "forget about it" 形式。 |
| I-5 | negation.rs:59 (`don't remember`) | "I don't remember my password" / "I hope they don't remember me" — 用户在求助或在叙述情绪，并非删除记忆。 | 用户决策：收紧形式（"don't remember this/that"）或删除。 |
| I-6 | negation.rs:82 (`not anymore`) | "I'm not hungry anymore" / "I hope this doesn't happen anymore" — 完全与记忆无关。`not anymore` 在英语里出现密度极高。 | 用户决策：删除此条，或换 "no longer use ... anymore" 复合形式。 |
| I-7 | negation.rs:74 (`改用`) | brief §2.2 显式授权作"通用形式"，但"这次我们改用 TypeScript 重构吧" / "明天改用 pnpm" — 用户在做新提议，并非否定既有偏好。LLM 二审会拦，但首关就触发。 | 用户决策：是否要回退到只用具体形式（`改成用`/`现在改用`/`以后改用`），删除 `改用` 这一最宽形式。 |
| I-8 | negation.rs:70 (`搞错了`) | "代码搞错了"/"方向搞错了" — 与记忆无关。"你搞错了" 是中性误伤入口（用户抱怨任何错误都说"你搞错了"）。 | 用户决策：要求主语限制（"记搞错了"/"你搞错了"），或换更窄形式。 |

> 说明：以上 8 条均为 brief-mandated（brief §2.2 表格内逐字列出），实现者无可选择余地。按 reviewer 守则"plan-mandated finding = 用户决策"，全部移交用户裁定。判据统一是"该短语能否出现在用户并非要求删除/修改记忆的正常句子里"。

### Minor

| # | file:line | 描述 | 建议 |
|---|----------|------|------|
| M-1 | negation.rs:164 (`{:?}` 格式化 `kind_desc`) | `format!("...{:?}...", kind_desc)` 中 `kind_desc` 是 `&'static str`，`{:?}` 会输出带字面引号的 `"stop-remembering"`。LLM 仍可理解，但 system prompt 文本会出现 `\"stop-remembering\"` 这种带转义引号的丑陋形式。 | 改为 `{}`（Display 形式）即可，零风险。 |
| M-2 | negation.rs:118-121 (同 kind 重复 find) | 用 `lower.find(&prev.matched_phrase)` 重搜上一轮的匹配位置以比较早晚。功能正确但每次循环 O(n)。可改成在 `best` 里同时缓存 `pos: usize` 字段，复杂度 O(n*m) → O(n+m)。 | 可选性能优化，本任务数据量小不必改。 |
| M-3 | negation.rs:175-178 (`<user_message>` 拼接) | 若 `user_text` 自带 `</user_message>` 字面量，可闭合提示词标签。LLM 第二关仍能识别（JSON 解析不依赖标签），但轻微污染提示词。 | 可加 escape，但 brief 未要求；标 Minor 观察。 |

---

## 3. Constraints 核对（10 条）

| # | 约束 | 结果 | 证据 |
|---|------|------|------|
| 1 | 只改 `negation.rs` | ✅ | `git diff --name-only 7e96126..07b986f` → 单文件 `src/agentic/src/negation.rs`；diff stat 1 file changed, 560 insertions(+), 1 deletion(-) |
| 2 | 依赖只用已有 `serde_json` / `tracing`；未改 `Cargo.toml`；无 `regex` | ✅ | 仅 `use serde_json::Value;`（line 17）；rg 未匹配到 `regex`/`std::fs`/`std::net`/`tokio`/`reqwest`/`hyper`；`Cargo.toml` 不在 diff 内 |
| 3 | 无 IO 无网络，只造/解析字符串 | ✅ | 全文件无 IO 操作；函数返回 `Option<NegationSignal>` / `(String, String)` / `Vec<usize>`，无网络/文件调用 |
| 4 | 非测试代码无 panic / 无 `unwrap`/`expect` / 无字节切片 `&s[..n]` / 按 char 处理 | ✅ | 14 处 `unwrap` 全在 `#[cfg(test)] mod tests`（line 294-559）；生产代码无 `panic!`/`expect`/`unwrap!`/`unimplemented!`/`todo!`/`unreachable!`；切片均为 `&reply[s..=e]`（line 209，端点 ASCII `{`/`}`）和 `&full_text[after_start..]`（line 268，端点为 ASCII 短语边界）；`extract_target_hint` 用 `chars().take(60)` |
| 5 | `parse_confirmation` 失败 → 空 Vec，不 panic；越界/负数/小数/字符串项/重复项处理 | ✅ | 失败路径全 `return Vec::new()`；line 228 `match Value::Number(n) if n.is_i64() || n.is_u64() => { if let Some(idx) = n.as_u64() {...} }` —— 负数 `as_u64()` 返 None、小数既不是 i64/u64、字符串不匹配 Number；line 232 `idx < candidate_count` + `seen.insert(idx)` 去重 |
| 6 | kind 优先级 FactIsWrong > StopRemembering > PreferenceReplaced；同 kind 取最早短语 | ✅ | `KIND_PRIORITY` 常量（line 89-93）按此顺序遍历；同 kind 内 line 118-121 用 `pos < prev_pos` 取最早位置；test `priority_fact_is_wrong_over_preference`（line 354）验证 |
| 7 | `target_hint` 截断 60 字符（`chars().count()` 可验证），空则 None | ✅ | line 278 `trimmed.chars().take(60).collect()`；line 273-275 / 283-284 返回 None 当空；test `target_hint_capped_at_60_chars`（line 393）`assert_eq!(hint.chars().count(), 60)` |
| 8 | `<user_message>`/`</user_message>` 包裹；候选 `[0]`/`[1]` 序号；不含 fact_id | ✅ | line 175-184 `format!("<user_message>{}</user_message>\n<candidates>\n", user_text)`；for 循环用 `enumerate()` 输出 `[i] {fact_text}`；迭代时丢弃 `_` fact_id（line 180 `(_, fact_text)`）；test `prompt_candidates_numbered_without_fact_id`（line 462）显式断言 `!user_content.contains("id-001")`/`"id-002"` |
| 9 | 注释/文档 English-only 无 emoji；测试函数名英文；prompt 文本英文 | ✅ | 所有 doc comments 英文（line 1-16, 19-28, 30-38, 99-105 等）；无 emoji；27 个 test 函数名全英文；system prompt 全英文 |
| 10 | 未跑 `cargo fmt`；文件 < 800 行；24 条测试存在；未越界实现 | ✅ | 文件 560 行；27 个 test fn 覆盖 brief §3 的 24 项（部分合并：`no_hit_vague_negative_chinese` 一条覆盖 brief 项 8/9/10 + `算了`；`no_hit_not_great` 一条覆盖 brief 项 11 + `didn't like`/`hmm`；其余一一对应）；diff 不涉及 extract/score/competition/ports |

---

## 4. 短语表逐条裁定表

判据：**该短语能否出现在用户并非要求删除/修改记忆的正常句子里**。可构造出误伤例句 = NARROW 或 REMOVE。

### 4.1 StopRemembering

| 短语 | 行 | 来源 | 裁定 | 理由 / 误伤例句 |
|------|----|------|------|----------------|
| `别再记` | 48 | brief 强制 | **保留** | 4 字组合较具体，常规句式难误伤。 |
| `不要再记` | 49 | brief 强制 | **保留** | 同上。 |
| `别记着` | 50 | brief 强制 | **保留** | 同上。 |
| `忘掉` | 51 | brief 强制 | **NARROW**（用户裁定） | 2 字通用词。误伤面：①"我忘不掉那段回忆"（情感叙述）②"别让我忘掉密码"（求助/提醒）③"如果忘掉了什么请提醒我"（请求）④"忘掉过去吧"（宽泛情绪）。建议加词边界 + 要求对象（`忘掉这条`/`忘掉那个`），或删除。 |
| `忘了这条` | 52 | brief 强制 | **保留** | "这条" 收窄了范围，误伤率低；仅余"我忘了这条规则"（自述遗忘），LLM 二审能挡。 |
| `删掉这条` | 53 | brief 强制 | **保留** | 强指令，误伤极低（"删掉这行代码" 由 LLM 二审挡）。 |
| `不用记` | 54 | brief 强制 | **NARROW**（用户裁定） | 误伤面：①"这个不用记笔记"（本回合别写）②"今天不用记日报"（日程取消）③"这个字段不用记录"（字段设计）。与"删除已存记忆"语义不同。建议收紧或删除。 |
| `stop remembering` | 55 | brief 强制 | **保留** | 完整英文指令短语。 |
| `forget that` | 56 | brief 强制 | **NARROW**（用户裁定） | 无词边界。误伤：①"Don't forget that we have a meeting at 3pm" ②"I forget that I told you my name" ③"Make sure you forget that appointment reminder next time"（后者才是真指令）。建议加 `\b` 或换 `forget that fact/memory`。 |
| `forget about` | 57 | brief 强制 | **NARROW**（用户裁定） | 误伤：①"Don't forget about the meeting tomorrow" ②"Let's not forget about the deadline" ③"I always forget about lunch"（自嘲）。建议收紧。 |
| `don't remember` | 58 | brief 强制 | **NARROW**（用户裁定） | 误伤：①"I don't remember my password"（求助）②"I hope they don't remember this embarrassing moment"（情绪）③"They don't remember my birthday"（叙述）。建议收紧为 "don't remember this/that + memory-noun" 形式，或删除。 |

### 4.2 FactIsWrong

| 短语 | 行 | 来源 | 裁定 | 理由 / 误伤例句 |
|------|----|------|------|----------------|
| `记错了` | 62 | brief 强制 | **保留** | 明确指令。 |
| `你记错` | 63 | brief 强制 | **保留** | 同上，主语锁定"你"。 |
| `那条是错的` | 64 | brief 强制 | **保留** | 指向"那条"，足够具体。 |
| `这条不对` | 65 | brief 强制 | **保留** | 同上。 |
| `搞错了` | 66 | brief 强制 | **NARROW**（用户裁定） | 误伤：①"代码搞错了"（代码逻辑）②"日期搞错了"（事件错误）③"方向搞错了"（路径）。LLM 二审可挡，但首关噪音多。建议加主语限制（`你搞错了`/`记搞错了`）或换 `记忆搞错了`。 |
| `that's wrong` | 67 | brief 强制 | **保留** | 英文明确指令。 |
| `that is wrong` | 68 | brief 强制 | **保留** | 同上。 |
| `you got it wrong` | 69 | brief 强制 | **保留** | 同上。 |
| `incorrect memory` | 70 | brief 强制 | **保留** | 极具体短语，无误伤。 |

### 4.3 PreferenceReplaced

| 短语 | 行 | 来源 | 裁定 | 理由 / 误伤例句 |
|------|----|------|------|----------------|
| `改用` | 74 | brief §2.2 显式授权（"用通用形式"） | **NARROW**（用户裁定） | brief 把它列为 `PreferenceReplaced` 的"通用形式"，但这是首关里最宽的中文短语。误伤：①"这次我们改用 TypeScript 重构吧"（新方案提议）②"明天改用 pnpm"（新指令）③"改用新方案试试"（探索）。LLM 二审能挡多数，但首关触发多。建议回退到只保留具体形式（`改成用`/`现在改用`/`以后改用`），删除裸 `改用`。 |
| `不再用` | 75 | brief 强制 | **保留** | 强指令；偶发误伤"我不想再用到这个函数"（代码上下文），LLM 二审可挡。 |
| `改成用` | 76 | brief 强制 | **保留** | 同上，更具体。 |
| `现在改用` | 77 | brief 强制 | **保留** | 时间状语收窄，明确"当下切换"。 |
| `以后改用` | 78 | brief 强制 | **保留** | 时间状语收窄，明确"今后切换"。 |
| `switched to` | 79 | brief 强制 | **保留** | 完整英文短语。 |
| `now i use` | 80 | brief 强制 | **保留** | 完整英文短语；极小误伤面（"for now I use" 但仍是指令）。 |
| `no longer use` | 81 | brief 强制 | **保留** | 完整英文短语。 |
| `not anymore` | 82 | brief 强制 | **NARROW**（用户裁定） | 误伤面巨大（英语日常高频短语）：①"I'm not hungry anymore" ②"I hope this doesn't happen anymore" ③"It doesn't matter anymore" ④"Is this still relevant anymore?"。建议删除此条，或要求紧邻 `use ... not anymore` 复合形式。 |

---

## 5. 无法从 diff 判定项

| 项 | 说明 |
|---|------|
| **二审 LLM 的实际过滤效果** | brief 设计了"短语表 → LLM 二次确认"两道关。本模块只负责第一关；第二关的 LLM 行为无法在此 diff 内评估。理论上 I-1~I-8 的误伤会被第二关拦下，但只有运行真实场景才能验证拦截率。 |
| **`extract_target_hint` 在 ASCII + CJK 混排的极端 case** | 当前实现假设 `byte_pos_in_lower == byte_pos_in_original`（ASCII 与 CJK 都不受 `to_lowercase` 影响，line 253-256 注释亦说明此点）。对目前所有短语成立。若未来加入土耳其语 İ/ı、希腊语 Σ/σ/ς 等大小写形态不同的字符，此假设会被破坏 —— **当前 spec 安全，仅作未来风险标注**。 |
| **`parse_confirmation` 在 reply 含两个 JSON 对象的解析** | 例：`"foo: {a:1} bar: {retire:[0]}"` 会取首 `{` 与末 `}` 截得 `"{a:1} bar: {retire:[0]}"`，serde_json 解析失败 → 空 Vec。当前实现是"宁可空错、不乱判"，符合 spec；brief 评估此项"非必要求修"，仅观察。 |
| **`build_confirmation_prompt` 在 `user_text` 含 `</user_message>` 字面量时的提示词完整性** | 当前实现为简单 `format!` 拼接，未做转义；LLM 提示词会"提前闭合"。属 Minor，未在 spec 列举。 |

---

## 6. 总体结论

- **SPEC: PASS** —— §2/§3/§4 全部满足，diff 干净。
- **QUALITY: PASS** —— 代码结构、注释、命名、测试覆盖、panic-freedom、依赖范围均达标。
- **APPROVED WITH NOTES** —— 8 条 Important 短语过宽问题全部由 brief 直接导致（plan-mandated），实现者未越权；建议把这 8 条（`忘掉`/`不用记`/`forget that`/`forget about`/`don't remember`/`not anymore`/`改用`/`搞错了`）统一交用户决定保留 / 收紧 / 删除。其余 Minor（`{:?}` 格式化、`<user_message>` 转义等）可选处理。