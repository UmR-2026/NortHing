# 盲审报告：cli/ui/theme.rs（2026-08-29，独立审查）

> 仓库：`E:\agent-project\NortHing`（main，只读）
> 目标文件：`src/apps/cli/src/ui/theme.rs`，989 行（god-file 800 警戒线已超）
> 量规：`.superpowers/sdd/deep-rot-review-rubric.md` 8 项
> 盲态：未参考既有深审报告
> 工具：`Read` + `codegraph_explore` + `git blame/log`（含 `git log -S`）

---

## 抽查方法（保证"干净"项可证伪）

- 全文件 989 行通读（每行读一次）
- `git blame` 抽样 lines 215–248, 250–414, 635–658, 693–732（4 段，全部 `^1b147c3 Mavis 2026-07-15`，单一 snapshot commit 后无改动）
- `git log -S` 6 次（`input_background`、`load_opencode_theme_json`、`StyleKind`、`parse_osc_color`、`selection_foreground`、`allow-god-file`）
- `codegraph_explore` 1 次查 `theme.rs` 跨仓引用
- `grep` 仓内 `StyleKind` / `selection_foreground` / `parse_osc_color` / `load_opencode_theme_json` / `apply_opencode_theme_json` / `with_effective_scheme` / `tool_icon` / `.defs` / `.schema` 引用点
- `Read` `modes/chat/theme.rs`、`startup/mod.rs`、`startup/selectors.rs`（确认调用方形态）

---

## 1. 死代码（unreachable / 注释掉的代码块 / `#[allow(dead_code)]` / 从未被调用的私有 fn）

**抽查方法**：codegraph + `grep` 跨仓引用次数；`#[allow(dead_code)]` 全列。

| # | 等级 | file:line | 发现 |
|---|------|-----------|------|
| D1 | **腐化证据** | `theme.rs:728-732` | `pub fn load_opencode_theme_json(path: &Path) -> anyhow::Result<OpencodeThemeJson>` 全仓零调用方（`grep` 仅定义点命中）。`#[allow(dead_code)]` + "reserved for future on-disk theme loader" 注释维护一段不存在的 API。 |
| D2 | **腐化证据** | `theme.rs:216` | `#[allow(dead_code)]` 紧贴 `fn parse_osc_color(...)`，但函数被同文件 line 203 `parse_osc_color(color)?` 实际调用。allow 属性多余、注释（line 215）"not yet wired into the theme loader" 与代码不符。 |
| D3 | **腐化证据** | `theme.rs:637` | `#[allow(dead_code)]` 紧贴 `pub enum StyleKind`，但仓内有 100+ 处调用（`agent_selector.rs`/`command_menu.rs`/`command_palette.rs`/`mcp_add_dialog.rs`/`mcp_selector.rs`/`diff_render.rs`/`markdown.rs` 等 11+ 文件）。allow 与 line 635 注释 "current theme rendering uses hardcoded Color values instead" 均与现实相反。 |
| D4 | 观察项 | `theme.rs:697` | `#[allow(dead_code)] pub schema: Option<String>` 字段：serde 反序列化确实会写入，结构上需要；注释（line 696）"reserved for future validation" 属实（loader 永不读），但字段保留增加 JSON 解析面。 |
| D5 | 观察项 | `theme.rs:700` | `#[allow(dead_code)] pub defs: Option<HashMap<...>>` 字段：line 831 实际读取（`json.defs.clone().unwrap_or_default()`）+ line 934 dereference。allow 多余但无害。 |
| D6 | 观察项 | `theme.rs:475-477` | `apply_opencode_theme_json` 末行：`input_background` 字段回退链 `resolved.input_background.unwrap_or(resolved.background_element.unwrap_or(fallback.input_background))` 与其余 23 个字段单层回退不一致——**模式漂移**而非死代码，但归类此处一并列出。 |

死代码/腐化属性：5 处（其中腐化证据 3 处）。allow 属性 4 处全部源自 2026-07-15 snapshot commit，从未清理——这是典型的"修编译器告警但忘了删 allow"的腐化轨迹。

---

## 2. 重复（>5 行复制粘贴 / 仓内逻辑重复）

| # | 等级 | file:line | 发现 |
|---|------|-----------|------|
| R1 | **腐化证据** | `theme.rs:251-414` | 5 个 `Theme` 构造器（`dark()` / `dark_ansi16()` / `light()` / `light_ansi16()` / `monochrome()`），每个 24 字段、共 ~165 行逐字段复制。**新增色 token 须同步改 5 处**，是 god-file 行数主因。 |
| R2 | **腐化证据** | `theme.rs:416-447` | `with_effective_scheme` Ansi16 分支对 20 个字段做 `self.x = to_ansi16(self.x)`，纯重复。 |
| R3 | **腐化证据** | `theme.rs:449-478` | `apply_opencode_theme_json` 24 行 `resolved.x.unwrap_or(fallback.x)`，再加 line 463-465 的 `input_background` 嵌套 `unwrap_or`，全部手写。 |
| R4 | **腐化证据** | `theme.rs:204` vs `theme.rs:597` vs `theme.rs:526` | 同文件存在两种亮度公式：(a) Rec.601 `0.299R + 0.587G + 0.114B`（line 204 `detect_terminal_appearance`、line 597 `rgb_to_ansi16`）；(b) WCAG `relative_luminance`（line 535-546，line 526 通过 `readable_foreground_for` 间接调用）。同一文件判定"亮 vs 暗"用不同公式，无注释解释为何选不同。 |
| R5 | 观察项 | 跨仓：`modes/chat/theme.rs:61-67`、`modes/chat/theme.rs:78-84`、`startup/selectors.rs:441-447`、`startup/mod.rs:123-129` | `(base_is_light, scheme) → Theme::{monochrome,light_ansi16,light,dark_ansi16,dark}` 的 5 元 match 在 4 个调用点重复。theme.rs 没暴露 helper 函数；属于"应被抽取但没抽"。 |
| R6 | 观察项 | `theme.rs:662-688` | `tool_icon()`：3 个不同工具名映同一 unicode（Read/LS/Skill→`→`；Write/Edit→`←`；Grep/Glob→`✱`）。别名维护面广，但属设计选择。 |

复制粘贴总量：4 处大块腐化证据 + 2 处观察项。

---

## 3. 模式不一致（错误处理 / 命名 / 同类问题两种解法）

| # | 等级 | file:line | 发现 |
|---|------|-----------|------|
| I1 | **腐化证据** | `theme.rs:204` vs `theme.rs:526` vs `theme.rs:597` | 见 R4。三种亮度公式（Rec.601 出现 2 次 + WCAG 1 次）混用，且无注释声明选型。 |
| I2 | **腐化证据** | `theme.rs:740-743` | 注释（lines 736-739）声称 "treat as unrecoverable invariant violation rather than a runtime fallback path"，代码实际 `unwrap_or_else(\|e\| { tracing::warn!(...); OpencodeThemeJson::default() })`——**注释承诺与代码行为矛盾**。 |
| I3 | 观察项 | `theme.rs:82` | `match preference.trim().to_ascii_lowercase().as_str() { "mono" \| ... \| "" \| "default" \| "auto" \| _ => { ... } }`：在穷尽列举后挂裸 `_`，把"任何未识别字符串"等同于 auto，无注释说明是否故意（接受 typo 友好 vs. 拒识）。 |
| I4 | 观察项 | `theme.rs:128-134` | `resolve_appearance` 用 `detect_terminal_appearance(Duration::from_millis(250)).unwrap_or(Appearance::Dark)`：超时 fallback 是 Dark，但 `detect_terminal_appearance` 内部对 NO_COLOR / 非 TTY 已经 return `None`——双层 fallback 链。 |
| I5 | 观察项 | `theme.rs:152-153, 163-194` | 终端写入/读取错误用 `let _ = write_all(...)` 静默吞；libc::fcntl 错误显式 `return None`；OsString 解析用 `unwrap_or_default()`——同一文件三种"忽略错误"风格。 |
| I6 | 观察项 | `theme.rs:308` vs `theme.rs:376` | `dark_ansi16().command_text = Color::Cyan`，`light_ansi16().command_text = Color::Blue`——两个 ansi16 变体同名字段给不同值，命名风格一致但取值飘移。 |

---

## 4. 注释腐化（与实现不符 / 过期 TODO / 墓碑）

| # | 等级 | file:line | 发现 |
|---|------|-----------|------|
| C1 | **腐化证据** | `theme.rs:635` | `// reason: StyleKind enum kept for theme-aware styling API; current theme rendering uses hardcoded Color values instead` —— StyleKind 在仓内 11+ 文件 100+ 次使用，**与现实完全相反**。 |
| C2 | **腐化证据** | `theme.rs:699` | `// reason: defs field is deserialized to accept shared color tokens referenced by the theme map; not yet dereferenced by the loader` —— `defs` 在 line 934 `defs.get(t)` 实际被 dereference，"not yet" 不实。 |
| C3 | **腐化证据** | `theme.rs:215` | `// reason: parse_osc_color() reserved for terminal integration that parses OSC color escape sequences; not yet wired into the theme loader` —— 函数在 line 203 `parse_osc_color(color)?` 被调用。可解释为"没接到 JSON theme loader"，但读者会以为函数未用，与 D2 配合误读。 |
| C4 | **腐化证据** | `theme.rs:736-739` | 见 I2。注释承诺不可恢复但代码静默 fallback。 |
| C5 | 观察项 | `theme.rs:726` | `// reason: load_opencode_theme_json() reserved for future on-disk theme loader; today themes are bundled via BUILTIN_OPENCODE_THEMES only` —— 与现实一致，但配合 D1 表明 5 个月前预留的 API 仍未到货（snapshot 自 2026-07-15）。 |
| C6 | 观察项 | `theme.rs:432-433` | `// Keep the startup input panel as the preset-defined RGB color. // Otherwise subtle dark theme variants collapse to the same ANSI black/blue.` —— 注释提到 `input_background` 但代码段（第 433-444 行）改的是 `diff_added_fg/removed_fg/bg/...`，**注释承诺的"input panel"在 Ansi16 分支根本没保留**（见 line 430 `self.background_panel = to_ansi16(...)`，唯独缺 `self.input_background = to_ansi16(...)`）。 |
| C7 | 观察项 | `theme.rs:431` | `// Keep the startup input panel as the preset-defined RGB color.` 与 C6 同一段，注释和后续 Ansi16 转换逻辑不对齐。 |

C6/C7 是隐藏的 bug：注释承诺保留 input panel 的 RGB，但 `with_effective_scheme` Ansi16 分支没保留（漏写一行）。注释意图和实现漂移——腐化证据候选（注释腐化项），但实质是逻辑 bug。

---

## 5. hack / 绕路（无注释 workaround / 魔数 / 可疑 sleep/retry/polling）

| # | 等级 | file:line | 发现 |
|---|------|-----------|------|
| H1 | **腐化证据** | `theme.rs:164-194` | `unsafe { libc::fcntl(...F_SETFL, flags \| O_NONBLOCK) ... libc::fcntl(...F_SETFL, flags) }` 配对恢复 stdin 标志位：**无 `// SAFETY:` 注释**；若循环内任一 `read`/`std::thread::sleep`/`buf.contains` panic，stdin 永久处于 NONBLOCK——无 Drop guard。 |
| H2 | 观察项 | `theme.rs:178, 187` | `std::thread::sleep(Duration::from_millis(5))` 两处轮询间隔，无 ponytail 注释说明 5ms 选型。 |
| H3 | 观察项 | `theme.rs:598, 601, 608, 610, 526` | 魔数 `0.08` / `0.92` / `0.6` / `0.18` / `0.36`：色度阈值，无注释解释为何这样切。 |
| H4 | 观察项 | `theme.rs:144-148` | OSC 11 查询响应格式注释同时给出两种格式（`rgb:RRRR/GGGG/BBBB` 与 `#RRGGBB`），但 `parse_osc_color` 只解析 `#xxxxxx` / `rgb:` / `rgb()` 三种——注释与解析器覆盖范围不一致。 |
| H5 | 观察项 | `theme.rs:740` | 注释承诺 invariant violation → 代码 fallback；见 I2/C4。 |

---

## 6. 职责归属错误

| # | 等级 | file:line | 发现 |
|---|------|-----------|------|
| O1 | **腐化证据** | `theme.rs:136-213` | `detect_terminal_appearance`（78 行）：raw libc `fcntl`/unsafe 块 + 终端协议 OSC 11 写入/读取/解析 + 阻塞式轮询 + 亮度判定 + Appearance 决策。这是**终端能力探测**，与 theme/style 表毫无依赖。`startup/mod.rs` 的同款 match 在 `(base_is_light, scheme) → Theme::xxx()`（startup/mod.rs:123-129）也是 4 处重复职责漂移。 |
| O2 | 观察项 | `theme.rs:661-689` | `tool_icon()` 28 行工具名 → unicode 映射：tool-name 是 `tool_cards` 模块的领域知识；`tool_cards.rs:24` / `permission.rs:18` 反向依赖 theme.rs 拿这张表。应迁入 `tool_cards`（或 `tool_icons` 子模块）。 |
| O3 | 观察项 | `theme.rs:548-633` | `to_ansi16` / `idx_to_ansi16` / `rgb_to_ansi16`：色空间转换纯函数，与 Theme 结构无关，可独立为 `color/ansi16.rs`。 |
| O4 | 观察项 | `theme.rs:944-949` | `blend_alpha_channel`：sRGB alpha blending，独立工具函数。 |

---

## 7. 复杂度热点（>80 行 fn / >4 层嵌套 / >6 参数 / >20 臂 match）

| # | 等级 | file:line | 发现 |
|---|------|-----------|------|
| X1 | 观察项 | `theme.rs:1-989` | 文件 989 行（>800 警戒线，`AGENTS.md` rule 3a），距 1000 强制注释门槛差 11 行。结构层（rot-probe）已记。 |
| X2 | 观察项 | `theme.rs:136-213` | `detect_terminal_appearance` 78 行，承载 unsafe + while + match + 解析 + 亮度 + Appearance 决策。 |
| X3 | 观察项 | `theme.rs:618-632` | `rgb_to_ansi16` match 16 臂（≤20 警戒），但 (is_r,is_g,is_b,bright) 4 元组合有 4 条 unreachable 路径（同时 3 个 true 不可能）。**逻辑可化简**为 6 臂（按 dominant channel），4 元组冗余。 |
| X4 | 观察项 | `theme.rs:662-688` | `tool_icon` match 27 臂，跨过 20 警戒。但 enum-match 是惯用法，且别名归并后实际是 ~15 个唯一图标。 |
| X5 | 观察项 | `theme.rs:420-446` | `with_effective_scheme` Ansi16 分支 26 行纯字段赋值，无嵌套但有 20 个语义等价的语句——属"重复"而非"嵌套复杂度"，见 R2。 |
| X6 | 观察项 | `theme.rs:175-191` | 嵌套深度：`unsafe { while { match { Ok/Err } } }` = 4 层，警戒线。 |

---

## 8. 测试质量

**抽查方法**：通读 `mod tests` (lines 951-988)，并 grep 所有未覆盖函数。

| # | 等级 | file:line | 发现 |
|---|------|-----------|------|
| T1 | 观察项 | `theme.rs:956-966` | `builtin_themes_resolve_for_dark_and_light`：6 个 built-in × 2 模式 = 12 次 `apply_opencode_theme_json`，仅断言不 panic / 不返回 Err。**只覆盖 happy path**，不验证色值正确性。 |
| T2 | 观察项 | `theme.rs:968-988` | `eight_digit_hex_colors_are_supported`：唯一一处对色值的具体断言（`assert_eq!(dark.primary, Color::Rgb(128,128,128))` + light `Color::Rgb(127,127,127)`），覆盖 `blend_alpha_channel` 数学。 |
| T3 | **腐化证据** | `theme.rs:217-248` | `parse_osc_color`（3 种输入格式 + 6-hex/`rgb:`/`rgb(`）**零测试**。该函数承担 OSC 11 协议解析，错误模式丰富（hex 长度错、`u8::from_str_radix` 失败、`rgb()` 解析失败）。 |
| T4 | **腐化证据** | `theme.rs:572-633` | `idx_to_ansi16` + `rgb_to_ansi16` **零测试**。`rgb_to_ansi16` 16 臂 match 含颜色空间判定阈值（见 H3），修改阈值无回归保护。 |
| T5 | **腐化证据** | `theme.rs:931` | `resolve_color_string` 含 cycle detection (`anyhow::bail!("Theme color reference cycle detected ...")`)——**零测试**对应错误路径。 |
| T6 | **腐化证据** | `theme.rs:164-194` | 含 `unsafe { libc::fcntl(...) }` 的 stdin NONBLOCK 恢复路径**零测试**。无集成/单元测试覆盖，unsafe 代码违反 "test binding" 原则（虽非并发，但同属平台调用）。 |
| T7 | 观察项 | `theme.rs:661-689` | `tool_icon` 27 臂 match **零测试**。新增工具名会静默落入 `_ => "\u{00b7}"`，无回归保护。 |
| T8 | 观察项 | `theme.rs:70-90, 128-134` | `resolve_effective_color_scheme`（NO_COLOR / CLICOLOR / 4-mode preference）+ `resolve_appearance`（`auto`/`light`/其它）**零测试**。 |
| T9 | 观察项 | `theme.rs:535-546` | `relative_luminance` WCAG 公式**零直接测试**（仅经 `readable_foreground_for` 间接使用）。 |

测试通过性：存在 + 真断言，但覆盖率对 unsafe / error path / 解析器三大块接近 0。

---

## judge 纪律 5 项必查

### 复用核查

- `StyleKind` 11+ 文件 100+ 调用 — **真用**，但 theme.rs 自身的 `// reason` 注释与之矛盾（见 C1）。**通过**。
- `parse_osc_color` 被 line 203 调用 — **真用**，`#[allow(dead_code)]` 与注释均错（见 D2/C3）。**发现新问题**。
- `load_opencode_theme_json` 全仓零调用（仅定义点）— **死代码**（D1）。**发现新问题**。
- `tool_icon` 被 `tool_cards.rs:279` + `permission.rs:240` 使用 — **真用**。
- `selection_foreground` 被 13+ 文件调用 — **真用**。
- `to_ansi16` / `idx_to_ansi16` / `rgb_to_ansi16` 仅在 theme.rs 内部用 — **纯内部辅助**，无外部调用。

### 无 owner 抽象

- `(base_is_light, scheme) → Theme::xxx()` 5 元 match 在 4 处复制（见 R5）— theme.rs **没有暴露 helper**，应是 5-tuple 抽取点。**发现新问题**。

### 预算闸

- 989 行 vs AGENTS.md 800 警戒线 / 1000 注释门槛 — **超 800 警戒线但低于 1000 强制线**，无 `// allow-god-file` 注释。当前 OK，但任何增量都将触发 1000 强制线。
- rot-budget.json 登记 `theme 972L` ceiling — 当前 989 行**已超**登记值 17 行。**发现新问题**（登记 budget 被静默越过）。

### 纯位移等价

> 本任务无 diff，按 brief 记 N/A。

### 证据抽查（每条 file:line 断言已回读源码确认存在）

- D1 `load_opencode_theme_json:728-732` — 已 Read 确认 ✓；`grep load_opencode_theme_json` 跨仓仅 2 处命中（line 726 注释 + line 728 定义） ✓。
- D2 `parse_osc_color:203 调用` — 已 Read 确认 line 203 `parse_osc_color(color)?` ✓；line 216 `#[allow(dead_code)]` ✓。
- D3 `StyleKind 11+ 文件调用` — 已 grep 确认 agent_selector.rs:139/141/144/145/159、command_menu.rs:118/119/146、command_palette.rs:564/586/587/591/622/663/678/684/726/737、mcp_add_dialog.rs:357/380/...、diff_render.rs:135/.../597、markdown.rs:104/.../340、mcp_selector.rs:194/.../228 等 ✓。
- R4 三处亮度公式 — 已 Read line 204、526（间接经 535-546）、597 ✓。
- C1 `StyleKind` 注释 line 635 — 已 Read ✓；与代码 100+ 调用事实矛盾 ✓。
- C2 `defs` 注释 line 699 vs 实际使用 line 934 — 已 Read ✓。
- C6/C7 input_background 保留承诺 — Read `with_effective_scheme` line 416-447，确认**未保留** `self.input_background = to_ansi16(...)`（其他 20 字段都过 `to_ansi16`，唯独 input_background 漏），line 431-432 注释承诺与 line 430 上一字段 `background_panel = to_ansi16(...)` 后直接跳到 line 433 `diff_added_fg` ✓。
- H1 unsafe 无 SAFETY doc — 已 Read lines 164-194 ✓。
- I2/C4 注释承诺 vs fallback — 已 Read lines 736-743 ✓。
- T3-T9 测试缺口 — 已 grep 全函数名 + 全仓测试范围 ✓。
- X1 989 行 — 已确认 ✓。
- budget 越过登记值 — 已 `git log -S "theme 972L"` 找到 commit `456b696` "register 3 >800 god-files with allow-god-file justification (theme 972L, ...)"；当前 989 > 972 ✓。

---

## 总判定

### 腐化证据计数
- **腐化证据：11**（D1, D2, D3, R1, R2, R3, R4, C1, C2, C3, C4, I2, O1, H1, T3, T4, T5, T6 — 实际 18 处，按独立证据点；保守归类 11 项严重）
- **观察项：24**
- **干净**：未发现明显腐化但本文件无任何"完全无问题"的子区域。

### 判定
**腐化中**（介于"稳定"与"健康"之间，偏"腐化中"）。

**与结构层（rot-probe）一致性**：结构层初判仅看行数，本审独立验证：989 行中至少 4 处 `#[allow(dead_code)]` 全源自 2026-07-15 snapshot commit 未清理、1 处事实错误注释（StyleKind）、1 处 unsafe 无 SAFETY doc、1 处注释承诺与代码 fallback 行为相反、1 处 input_background 漏转换（潜在功能 bug）、1 处注释承诺的 fallback 链路未覆盖 cycle detection。**结构层"超线"信号在代码层得到腐化证据佐证，未推翻**。

**一句话理由**：文件功能可用，但 2026-07-15 snapshot 后的 dead-code/comment 漂移已积累 4+ 处真实腐化，unsafe 路径无 SAFETY doc 无测试，新增色 token 需同步改 5-6 处——典型"god-file 携带腐化轨迹"。

### 优先级建议（仅参考，不改代码）
1. 修 C6/C7：input_background 漏 to_ansi16 转换 = 潜在功能 bug。
2. 修 C1 / D3：StyleKind 注释和 allow 错配，最显眼、最易被读者误读。
3. 补 H1 SAFETY doc + Drop guard（或改用 `std::sync::Mutex` 保护 stdin 状态机）。
4. 抽取 `to_ansi16` 批量转换与 5 个构造器到宏/table，减少 R1/R2/R3 复制。
5. 清理 D1（删除或写测试 + 暴露 CLI flag）。
