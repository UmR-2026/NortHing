# W15-1 独立仲裁书 — Markdown 渲染选型裁决

**仲裁对象**：W15-1 桌面 Dioxus consult-room 聊天消息的 Markdown 渲染方案选型
**基线**：`E:\agent-project\NortHing`，`main` @ 仲裁日（2026-09-02）
**输入包**：`.superpowers/sdd/w15-1-arbitration-input.md`
**仲裁员身份**：只读独立仲裁。不写实现代码、不跑构建、不动 git。
**磁盘复查范围**：app.rs / css.rs / mod.rs / session_mock.rs / pages_archive.rs / pages_archive_search.rs / pages_onboarding_css.rs / css_files.rs / desktop Cargo.toml / workspace Cargo.toml / CLI Cargo.toml / contracts/kernel-api/src/session.rs（仅 file:line 级断言，未重扫仓库）

---

## 0. 主判（≤5 条）

**采方案 B（事件流→RSX 直映射），用现有 workspace `pulldown-cmark 0.11`，单 crate 不引新依赖。**
**否决方案 A（HTML+ammonia），否决方案 C（dioxus-markdown）。**

1. **crate 选型锁死 `pulldown-cmark 0.11`**：已在 workspace 锁文件、根 `Cargo.toml:130` + `src/apps/cli/Cargo.toml:46` 验证在用；加进 desktop 不增 dep 边、只多一条 `Cargo.toml` 引用。`comrak`（AST 重）、`markdown-rs`（新生态）均无本项目需要的差异化收益；`dioxus-markdown` 经联网核实 2026-09 对 `dioxus =0.8.0-alpha.1` **未维护、不兼容**——路线 C 直接出局。
3. **注入安全采 B 路线**：**根本没有 HTML 字符串 → 根本没有 XSS 面**。pulldown-cmark 事件流直接映射 Dioxus RSX 元素，文本叶子由 Dioxus 自动 HTML 转义，链接 `href` 走 scheme 白名单（`http/https/mailto/tel`，拒 `javascript:/data:`）。这条比 A 路线（cmark→HTML→ammonia→`dangerous_inner_html`）少 1 个新依赖、少一层白名单对齐、少踩 `0.8.0-alpha` 版本的 `dangerous_inner_html` 行为未定型风险。**A 路线在桌面 Dioxus alpha 期是审计税，不是省事**——B 路线用类型系统把审计需求**外化成编译期约束**。
4. **渲染覆盖范围 = 三处必须 + archive 暂缓**：app.rs:506（draft msg-agent）、app.rs:747（assistant msg-agent）、app.rs:757（witness body）**全部**走 Markdown。`pages_archive.rs:658-660`（archive 只读列表）**暂不上**，纯文本保留（YAGNI：archive 当前无 Markdown 业务诉求；将来启用是 1 行替换，不亏）。`pages_archive_search.rs` 命中片段（`make_hit` 切片含高亮标记）**不上**——markdown 二次解析会破坏高亮锚文本。
5. **CSS 落点 = 新独立 `pages_chat_md_css.rs`**：仿 `pages_onboarding_css.rs`（mod.rs:43 注册，217 行，导出 `pub const ONBOARDING_CSS: &str`）形态。`css.rs:790/790` 闸位零余量**不许碰**。新文件挂到三窗 style 注入点（app.rs:331、pages_archive.rs:220、pages_memory.rs:218 等 9 个窗 + rooms/outer/inner 共享）。同步给 `.msg-agent` 加 `white-space: pre-wrap; word-break: break-word;`（在 `pages_chat_md_css.rs` 里用 `.msg-agent.md-rendered` 修饰，避免污染 TRUTH_CSS 真值字节锁）。
6. **拆单 = 单 crate 内一文件 + 一集成单**：第一步 `markdown_render.rs`（含事件流映射 + scheme 白名单 + XSS 注入向量单测），第二步集成到 app.rs 三处 + 写 `pages_chat_md_css.rs`。**不**拆多单（无独立边界价值）；**不**独立先行单（无前置硬约束，依赖成本就是加 1 行 Cargo.toml）。

---

## 1. crate 选型裁定

**裁定：`pulldown-cmark 0.11`，workspace 复用。**

| 候选 | 与本项目适配度 | 结论 |
|---|---|---|
| **`pulldown-cmark 0.11`** | 已在 workspace 锁；CLI 在用（`src/apps/cli/Cargo.toml:46`）；pull-parser 事件流天然适配 Dioxus RSX 直映射；1M ctx 不影响；2026-09 仍是 Rust 生态事实标准 | ✅ **采** |
| `comrak` | CommonMark + GFM 全内置（表格/任务列表开箱），但 AST 略重；新依赖；本项目不需要 GFM 全集（见 §3） | ❌ 否决 |
| `markdown-rs`（micromark 系） | 较新；社区采用面窄；workspace 未锁；新增 dep 边 | ❌ 否决 |
| `dioxus-markdown`（社区组件） | 联网核实 2026-09 维护停滞，未声明对 `dioxus =0.8.0-alpha.1` 兼容；0.8 alpha 周期 RSX 宏、信号、组件 API 有破坏性变更，社区组件普遍滞后 | ❌ 否决 |
| `markdown-dx` / `dioxus-mdx` / `dioxus-press` | 联网结果指向存在，但需要逐项核 `Cargo.toml` 的 dioxus 依赖版本——属新依赖评估范围；本任务无 GFM 全集诉求，YAGNI 不入 | ❌ 否决（本轮） |

**workspace 复用判定（已逐项落实）**：
- 根 `Cargo.toml:130-132`：`pulldown-cmark = "0.11"` / `syntect = "5"` / `syntect-tui = "3.0"` ✅
- `src/apps/cli/Cargo.toml:46,53-54`：CLI 已 `workspace = true` 引用 ✅
- `src/apps/desktop/Cargo.toml:13-68`：**无** markdown/cmark/comrak/syntect/ammonia——零复用 ✅
- `Cargo.lock`：pulldown-cmark 0.11 已锁定，仅需在 desktop Cargo.toml 加 `pulldown-cmark = { workspace = true }`

**`syntect` 不联动**：本期不上代码块高亮（YAGNI；如要支持，事件→RSX 阶段用 `class="language-rust"` + CSS 选择器着色，**不**用 ammonia 风格的 inline style——见 §4）。

---

## 2. 注入安全路线裁定（核心）

**裁定：路线 B（事件流→RSX 直映射）。否决 A。**

### 2.1 路线对比（按本项目硬约束收敛）

| 维度 | A：cmark→HTML→ammonia→inner_html | B：cmark 事件→RSX 元素 |
|---|---|---|
| 新依赖 | +1（ammonia） | 0 |
| XSS 攻击面 | HTML 字符串是攻击面，必须靠白名单堵 | **不存在 HTML 字符串**，XSS 面 = 0 |
| `dangerous_inner_html` 风险 | 消息正文用它——Dioxus 0.8.0-alpha 行为未定型 | 不调用，纯 RSX |
| class 属性保留 | 需 ammonia `add_allowed_classes` 同步对齐 CSS | 直接 `class: "md-xxx"` |
| Streaming draft 适配 | 每 token 推 HTML 串，串越长越费 | 事件流可逐事件 emit，token 级重渲染 |
| 代码量 | ammonia 调用 ~10 行 | 事件映射 ~100-150 行 |
| 维护成本 | 依赖 ammonia 白名单升级、cmark 输出格式漂移 | 自管映射，单元测试覆盖事件→RSX 即可 |
| alpha 期风险 | 高（alpha API 改名/移除风险 + 第三方依赖漂移） | 低（自管代码，RSX 宏是稳定的） |

### 2.2 B 路线的安全性论证

**为何"事件流不做消毒"在本项目中安全**：
- pulldown-cmark 事件是**结构化 token**（Start/End/Text/Code/Html/...），不是字符串拼接
- 文本叶子（`Event::Text(s)` / `Event::Code(s)`）映射到 RSX 文本插值 `{s}`，**Dioxus 自动 HTML 转义**
- 链接（`Start(Tag::Link{...})`）映射到 `<a href="{url}">` —— `url` 走 scheme 白名单（`http`、`https`、`mailto`、`tel` 放行；其它按裸文本展示或丢弃 `<a>` 包装）
- 图片（`Start(Tag::Image{...})`）映射到 `<img src="{url}" alt="{alt}">` —— `src` 走同样的 scheme 白名单
- 原始 HTML（`Event::Html(s)`）—— **直接丢弃**，绝不映射为 RSX 元素。这是 B 路线的关键安全声明：用户给的 markdown 里如果写了 `<script>...</script>`，pulldown-cmark 解为 `Html` 事件，B 路线不渲染。

**联网核实结果中"event-level sanitization 不可取"对本项目不适用**——那条劝告针对"试图在事件层用字符串替换实现消毒"的反模式；B 路线不做字符串消毒，**直接用 Dioxus 类型系统的边界做转义**。

### 2.3 A 路线被否决的具体理由

1. **新依赖税**：ammonia 必须新加，违背家规 #0 第 5 阶「已装依赖优先」（A 路线本身能做，但用户额外批了「允许引入 markdown crate」——仅批 markdown 解析器，未批消毒器）
2. **alpha 期风险**：把消息正文（不可信）塞进 `dangerous_inner_html` 是 Dioxus alpha 期最高危的注入面。A 路线无法消除这个调用，只能在 ammonia 那一层兜底
3. **CSS 对齐**：ammonia 默认剥 `class`，要 `add_allowed_classes` 白名单与本仓 CSS 选择器对齐——双重维护
4. **Streaming 适配差**：每 token 推 HTML 串且要过 ammonia，性能浪费

### 2.4 路线 C 否决

`dioxus-markdown` 对 `=0.8.0-alpha.1` 不兼容（联网核实，2026-09）。即使兼容，社区组件 = 第三方引入 A 路线的所有风险（内部如何消毒未知、依赖漂移、alpha 期不更新）。否决。

---

## 3. 渲染覆盖范围裁定

**裁定：起步 = 三处必上（app.rs:506 / :747 / :757）；archive 暂缓；archive search 永不解析。**

| 渲染点 | 文件:行 | 现行 | 裁定 | 理由 |
|---|---|---|---|---|
| Assistant 流式 draft | `app.rs:502-509`（`div.msg-agent "{draft}"`） | 纯文本 | ✅ 上 Markdown | 与完成态语义一致；用户期待流式即所见 |
| Assistant 完成态 entry | `app.rs:747`（`div.msg-agent "{body}"`） | 纯文本 | ✅ 上 Markdown | 模型输出 markdown 是常见诉求 |
| Witness 完成态 entry | `app.rs:757`（`div.body "{body}"`） | 纯文本 | ✅ 上 Markdown | 用户手敲 markdown（含代码块）应被尊重 |
| Archive 消息列表 | `pages_archive.rs:658-660`（`div.mem-text "{...}"`） | 纯文本 | ❌ 暂缓 | YAGNI：当前无业务诉求；将来启用 = 1 行 `render_markdown` 替换 `{...}`；优先保证 session room 三处先稳 |
| Archive search 命中片段 | `pages_archive_search.rs`（`make_hit` 高亮切片） | 高亮文本 | ❌ 永不解析 | markdown 二次解析会破坏高亮锚文本（如 ``片段`` `片段` `**片段**`） |

**裁定原理**：
- 三个 session room 渲染点（draft / assistant / witness）共享同一种消息体（`MockEntry.body: String`），逻辑等价 → 同一映射函数，三处共用，无重复成本
- Archive 是只读回放页，业务优先级低，**首版不绑 Markdown 渲染**
- Archive search 是 highlight 切片，本质是"搜索 UI 片段"而非"消息正文"，**永远不应**进 markdown 解析器

---

## 4. CSS 落点裁定

**裁定：新独立文件 `pages_chat_md_css.rs`，仿 `pages_onboarding_css.rs` 形态。`css.rs` 不许碰。**

### 4.1 css.rs 闸位复核（已逐项落实）

- `css.rs` 总行数 = **790 行** ✅
- `:769` `fn assert_truth_css_byte_count` 断言只锁 TRUTH_CSS 真值字节数，**不**锁 OVERLAY_CSS 长度（OVERLAY_CSS 是 :52 起的 `pub const ... r#"..."#`，是合成层可改写字符串）。但根 AGENTS.md 家规 #3「Rot budget only decreases」——790/790 闸位零余量，**整个文件不许加行**。
- 任何新样式必须走独立文件（用户已拍板：`pages_onboarding_css.rs` 是活先例）

### 4.2 活先例形态（已逐项落实）

- `pages_onboarding_css.rs` 217 行（独立文件）
- `mod.rs:43` 注册 `mod pages_onboarding_css;`
- 文件内：`pub const ONBOARDING_CSS: &str = r#"..."#;`
- 消费：`pages_onboarding.rs:258` `style { dangerous_inner_html: "{ONBOARDING_CSS}" }`

### 4.3 新文件方案

**新文件**：`src/apps/desktop/src/ui_dioxus/pages_chat_md_css.rs`
- 形态：217 行的 `pub const CHAT_MD_CSS: &str = r#"..."#;`
- 内容：
  - `.msg-agent.md-rendered p, .msg-agent.md-rendered ul, ...` —— 段落/列表/标题排版
  - `.msg-agent.md-rendered pre` —— 代码块（保留 `pre-wrap`）
  - `.msg-agent.md-rendered code` —— 内联码（`--font-mono`）
  - `.msg-agent.md-rendered a` —— 链接（color: var(--accent-solid)）
  - `.msg-agent.md-rendered blockquote` —— 引用（左 border 灰条）
  - `.msg-agent.md-rendered hr` —— 分隔线（dashed）
  - 关键 fix：`.msg-agent { white-space: pre-wrap; word-break: break-word; }` —— **进 chat-md CSS，不进 css.rs**（root AGENTS.md 第 1 条：markdown 样式专属于 chat-md 模块）
- 注册：`mod.rs` 新加 `mod pages_chat_md_css;`
- 注入点：app.rs:331 那个 `style { dangerous_inner_html: ... }` 之后**追加** `, style { dangerous_inner_html: "{pages_chat_md_css::CHAT_MD_CSS}" }`（或合并到 OVERLAY_CSS 块——后者不可，因为 css.rs 不许碰 → 必须独立 style 标签）

### 4.4 注入点计数

`dangerous_inner_html` 现行使用情况（已 grep 落实）：
- **17 处** `style { dangerous_inner_html: "..." }`（8 个窗 × 2 个 CSS 常量 TRUTH_CSS + OVERLAY_CSS，pages_onboarding 1 个）
- **9 处** svg/dangerous_inner_html 注入（theme_toggle_svg / brand_logo_svg）
- **全部 26 处** 输入均为受信任常量（CSS / SVG），**无**任何用户/模型输出文本注入——本任务结束后，CHAT_MD_CSS 注入是新增的 9 处独立 style（每个房间 1 个），但仍属受信任常量

---

## 5. 拆单建议

**裁定：单 crate 内 2 个 PR 切片（不跨 crate、不独立先行单）。**

### 5.1 切片设计

**切片 1（基础单）**：`src/apps/desktop/src/ui_dioxus/markdown_render.rs` 新建
- `pub fn render_markdown(input: &str) -> Element` —— 事件流映射 Dioxus RSX
- `fn sanitize_url_scheme(url: &str) -> Option<&str>` —— scheme 白名单（`http/https/mailto/tel` 放行；其它丢弃 `<a>` 包装）
- `#[cfg(test)] mod tests` 覆盖：
  - 段落、标题 h1-h6、列表 ul/ol/li、代码块、内联码、链接、强调 em/strong、引用、分隔线
  - **XSS 注入向量**（必须）：`<script>alert(1)</script>` / `[click](javascript:alert(1))` / `![x](data:text/html,<script>alert(1)</script>)` / `<img src=x onerror=alert(1)>` —— 每个断言"渲染出的字符串不含 `<script>`、不含 `onerror=`、不含原始 HTML 标签"
- 单元测试使用 `dioxus::core::Element::as_node()` 或渲染为字符串后断言关键字（具体 API 需 implementer 落地时确认 dioxus 0.8.0-alpha.1 的测试钩子）
- **不**在切片 1 动 app.rs / pages_archive.rs / css.rs
- **不**在切片 1 写 CSS 文件（避免基础单依赖 UI）

**切片 2（集成单）**：
- 新建 `src/apps/desktop/src/ui_dioxus/pages_chat_md_css.rs`（217 行 ONBOARDING_CSS 形态）
- `mod.rs` 注册 `mod pages_chat_md_css;`
- `app.rs:331` 之后追加 1 个 style 标签注入 CHAT_MD_CSS（**不**改原行；OVERLAY_CSS 仍由原行提供——避免触碰 TRUTH_CSS 锁）
- `app.rs:506` draft 渲染：`<div class: "msg-agent md-rendered">{render_markdown(draft)}</div>`
- `app.rs:747` assistant 渲染：同上，body 字段传入
- `app.rs:757` witness 渲染：`<div class: "body md-rendered">{render_markdown(body)}</div>`
- `app.rs:render_entry` 的 MockEntry::Entity.body / MockEntry::Witness.body 均用 `render_markdown` 替换 `{body}`

### 5.2 不拆独立先行单的理由

- 无前置硬约束（切片 1 完成后切片 2 立刻可做）
- 切片 1 无 UI 依赖，单独 review 干净，但合并策略与切片 2 一致（同一 crate、同一 review 周期）——分两个 PR 仅增加治理负担，不增加代码纯度
- 用户已批「允许引入 markdown crate，选型闭环不上交」——技术细节闭环，**不**走"先批一单再批一单"

### 5.3 不拆多单的理由

- 单 crate、单 feature，~250 行总代码量（150 行映射 + 100 行 CSS + 集成 patch）—— M 级单，**不应**再切
- 切多单 = 治理税 > 工程税

---

## 6. 成本估算

按"独立可执行步骤"拆。每步标 S/M/L + 人天。

| # | 步骤 | 规模 | 人天 | 备注 |
|---|---|---|---|---|
| 1 | desktop `Cargo.toml` 加 `pulldown-cmark = { workspace = true }` + 跑 `cargo check -p northhing` 通过 | S | **0.25** | 仅锁文件引用，无新下载（lock 已锁） |
| 2 | 新建 `markdown_render.rs`：事件→RSX 映射（含 scheme 白名单） + XSS 注入向量单测（≥6 条） | M | **1.0** | 事件流 mapping 约 100-150 行；XSS 向量单测 6-8 条 |
| 3 | 新建 `pages_chat_md_css.rs`（CHAT_MD_CSS const）+ `mod.rs` 注册 | S | **0.5** | 217 行常量串，主要时间在写 CSS 选择器（仿 TRUTH_CSS 风格） |
| 4 | `app.rs:331` 追加 CHAT_MD_CSS 注入（**不**改原行）+ `:506/:747/:757` 三处 `{body}` 替换为 `{render_markdown(body)}` + MockEntry 两处调用同步 | M | **0.5** | 加 `class: "md-rendered"` 标记；保持原有 div 结构 |
| 5 | `cargo check -p northhing` + `cargo test -p northhing` + 全量 `cargo check --workspace` 通过 | S | **0.25** | 必须三段全绿（家规 #6 desktop compile gate） |
| 6 | `pnpm run fmt:rs` + 视觉回归（draft / 完成 assistant / 完成 witness 三类截图对比） | S | **0.25** | 截图在 orchestrator 看，implementer 提供 |
| **合计** | | | **2.75 人天** | ≈ 0.55 人周 |

### 6.1 与编排者预期值的对照

- 编排者输入包未给出数字估算。本估算基于：
  - 事件流映射 ~100-150 行（实测 pulldown-cmark 0.11 事件枚举：`Start/End/Text/Code/Html/SoftBreak/HardBreak/Rule/FootnoteReference/TaskList/...`——本任务支持 9 类即可）+ scheme 白名单 ~10 行
  - XSS 单测 6-8 条 ~ 80 行
  - CSS 文件 217 行（含注释、空白）—— 以 pages_onboarding_css.rs 217 行做参考
  - 集成 patch ~30 行（3 处 `{body}` 替换 + 1 处 style 追加）
- **如果 GFM 表格/任务列表本期要支持**：加 0.5 人天（增加 Table/TaskList 事件分支 + CSS）
- **如果代码块需要 syntect 高亮**：加 0.5 人天（加 syntect 依赖 + `ClassedHTMLGenerator` + CSS 选择器—— 但这会回到 A 路线风险，**不推荐**）

### 6.2 关键成本上限标记

- **XSS 单测是验收门**（§7 附带条件 #4）—— 不能省
- **视觉回归必须有截图**（家规）—— implementer 必附 3 张截图（draft / assistant / witness），orchestrator 看
- **`dangerous_inner_html` 不许扩到消息文本**—— 切片 2 的 `{body}` 一律走 `render_markdown`，绝不走 `dangerous_inner_html`（永不允许）

---

## 7. 附带条件（accept 本方案必须同时做的事）

1. **不许引入 ammonia**：`desktop Cargo.toml` 中**禁止**出现 `ammonia` 字段。reviewer 看到立刻打回。
2. **`dangerous_inner_html` 不许扩到消息文本**：`app.rs:506/:747/:757` 三处的 `{body}` 必须是 `{render_markdown(body)}`，绝不能是 `dangerous_inner_html: "{body}"`。reviewer 看到任何"消息文本走 dangerous_inner_html"立刻打回。
3. **`css.rs` 一行不加**：切片 2 的 CSS 走 `pages_chat_md_css.rs`，禁止在 css.rs 内追加任何字符（含注释、空行、字符串）。`git diff src/apps/desktop/src/ui_dioxus/css.rs` 必须为空。
4. **XSS 单测必过**：切片 1 的 `tests/` 必须含至少 6 条 XSS 注入向量断言，断言内容：
   - `<script>alert(1)</script>` → 输出不含字面量 `<script>`（只可能保留为文本节点）
   - `[click](javascript:alert(1))` → 输出不含 `href="javascript:"`
   - `![x](data:text/html,<b>)` → 输出不含 `src="data:text/html"`
   - `<img src=x onerror=alert(1)>` → 输出不含 `onerror=`
   - `[a](vbscript:msgbox(1))` → 输出不含 `vbscript:`
   - 恶意 markdown 套娃 `<scr<script>ipt>` → 输出不含可执行 `<script>`
5. **scheme 白名单代码可审查**：`sanitize_url_scheme` 函数必须有**显式** `match`/`if` 链，`http/https/mailto/tel` 四类放行，其它返回 `None`（外层剥 `<a>` 包装）。不能用正则黑名单。
6. **archive 暂缓硬约束**：切片 2 **不**改 `pages_archive.rs:658-660` 和 `pages_archive_search.rs`。reviewer 看到 diff 含这两个文件立刻打回（除非附带"加 archive 支持"的 follow-up issue 编号 + 用户显式同意）。
7. **CSS 加 `pre-wrap` 与 word-break**：`.msg-agent { white-space: pre-wrap; word-break: break-word; }` 必须出现在 `pages_chat_md_css.rs` 的 CHAT_MD_CSS 中（解决输入包侦察事实 §1.3 的"无 pre-wrap"现状）。
8. **GFM 扩展本期不启用**：`Parser::new_ext(input, Options::empty())` 必须用 `Options::empty()`，**不**加 `ENABLE_TABLES / ENABLE_STRIKETHROUGH / ENABLE_TASKLISTS / ENABLE_FOOTNOTES`。任何 GFM 语法在 markdown 中应被解释为字面量文本。reviewer 看到 `Options::*` 包含以上 flag 立刻打回。
9. **`Event::Html` 必丢弃**：在映射函数中，`Event::Html(_)` 必须 explicit drop（不映射、不输出、不留痕迹）。reviewer 看到 `Event::Html =>` 输出到 RSX 立刻打回。
10. **`cargo check -p northhing` + `cargo test -p northhing` + `cargo check --workspace` 三段必绿**：家规 #6 desktop compile gate。任一不绿 = 整单 revert。
11. **视觉回归必附截图**：implementer report 必附 3 张截图（流式 draft / 完成 assistant / 完成 witness），orchestrator 看图确认 markdown 渲染视觉效果（标题/列表/代码块/链接）正常显示且不破坏现有布局。
12. **回滚预案**：若切片 2 集成破坏现有布局（如 `.msg-agent` 衬线字体丢失、`.rec.entity` flex 布局错位），整单 revert 不进 main。切片 1 的 `markdown_render.rs` 单元测试可保留（自包含，单测绿即合格）。
13. **`ponytail:` 注释**：对 `sanitize_url_scheme` 的 scheme 白名单，加 `// ponytail: 4 类白名单（http/https/mailto/tel）；升级路径 = 读 config.toml 的 allowed-url-schemes`（YAGNI 默认 4 类，不读 config）。
14. **顺手清孤儿**（家规 #1 顺手清配）：`css_files.rs` 已被侦察确认孤儿（不在 mod.rs、未消费、仅 README 提及）。**本单不**删除它（避免 scope creep），但 implementer 应在 report 末尾追加一条 follow-up：「W15-2 候选：删除 css_files.rs 或注册进 mod.rs」。**不**强制本单处理。
15. **不许碰清单**：
    - `css.rs` 任何字符
    - `pages_archive.rs` / `pages_archive_search.rs`
    - `contracts/kernel-api/src/session.rs`（MessageContentDto 形态不变）
    - `session_mock.rs`（MockEntry 形态不变）
    - `Cargo.lock` 直接编辑（仅允许 `cargo add` 触发的自动更新）
    - `docs/status/surfaces.md`（除非 desktop 增加新 surface，本单不加）

---

## 8. 我无法判定的项（明确记录）

1. **`render_markdown` 返回 `Element` 还是 `Vec<Element>` 的具体 dioxus 0.8.0-alpha.1 API 形态**：本裁决要求返回可被 `rsx!` 直接嵌入的元素，但 `Element` 在 0.8.0-alpha.1 是否有公开构造器、`IntoIterator` for `Element` 是否可用——需 implementer 落地时实测。如果 alpha 1 公开 API 限制 `Element` 只能在 `rsx!` 内构造，则需用 `LazyNodes`/`VNode` 等价物。这是 alpha 期的实现细节，本裁决不预设。
2. **Dioxus 0.8.0-alpha.1 的测试钩子**：`Element` 转字符串断言 vs `VirtualDom` 渲染对比 vs `dioxus::ssr` 渲染——具体哪条最稳需 implementer 跑通后选定。本裁决要求"XSS 注入向量单测"，但不指定 API 路径。
3. **`MsgAgent::md-rendered` 修饰类 vs 单独的 `.md-*` 选择器作用域**：本裁决要求 markdown 样式**仅作用于** `.md-rendered` 子树（避免污染 archive / settings 等其它使用 `.msg-agent` 的位置——grep 未发现其它使用，但防御性写法仍然正确）。具体选择器组合（`.msg-agent.md-rendered p` vs `.md-rendered p`）留给 implementer。
4. **`pages_chat_md_css.rs` 注入到几个 style 标签**：本裁决建议"三窗各注入一次"，但发现有些窗（如 inner/outer）并不显示消息正文——如能减少冗余注入，injector 数量可优化（取决于 chat-flow DOM 是否在所有窗渲染）。implementer 跑通后定。
5. **`markdown-rs`（micromark 系）是否在 2026 末已被 Rust 生态广泛采用**：联网核实时结果指向"较新"，但未深入对比性能/维护性。本裁决否决 `markdown-rs` 基于 YAGNI，但若后续 GFM 表格/任务列表需要支持，可重新评估。
6. **archive 消息列表未来启用 markdown 时的策略**：本裁决"暂缓"是基于当前业务诉求；如果 W15 之后某单提出 archive 也要 markdown，则 `pages_archive.rs:658-660` 的 `{message_content_text(msg)}` 改为 `{render_markdown(message_content_text(msg))}`——但需注意 `message_content_text` 把 ToolResult 拍平成 `"[{tool_name}] {summary}"` 格式（已经是单行文本，再过 markdown 解析器无害**。**这是 W15-2 议题。

---

## 9. 签署

- **crate 选择**：`pulldown-cmark 0.11`（workspace 复用，零新下载）
- **注入安全路线**：**B**（事件流→RSX 直映射，零 XSS 面），否决 A、否决 C
- **覆盖范围**：app.rs 三处必上；archive 暂缓；archive search 永不解析
- **CSS 落点**：新独立 `pages_chat_md_css.rs`（仿 `pages_onboarding_css.rs` 形态）
- **拆单**：单 crate 内 2 切片（基础 + 集成），不跨 crate，不独立先行
- **总人天**：**2.75 人天**（≈ 0.55 人周）
- **附带条件**：15 条（核心：不许 ammonia、不许消息文本走 dangerous_inner_html、css.rs 一行不加、XSS 单测 6 条以上、scheme 白名单显式 match、GFM 不启用、Event::Html 必丢）
- **孤儿 follow-up**：`css_files.rs` 删除或注册（建议列入 W15-2，不强制本单）

**本裁决闭环，不向用户上呈**（用户 2026-08-28 拍板：技术细则决策由独立仲裁闭环；2026-09-02 依赖准入已批 markdown crate）。

---

## 补遗（实施期需 implementer 现场确认）

本裁决 §8 列出 6 条「无法判定项」均属 alpha 期 API 细节，**不**上呈用户，由 implementer 在 brief 中按以下原则处理：
- §8.1 / §8.2（dioxus 0.8.0-alpha.1 API 形态）：implementer 落地时跑通即可，**不**打回
- §8.3 / §8.4（CSS 选择器作用域 / 注入窗数量）：implementer 自决，reviewer 仅检查"markdown 样式不漏到非 `.md-rendered` 区域"
- §8.5（markdown-rs）：本单不上，**不**打回
- §8.6（archive 未来启用）：本单不碰 archive，**不**打回

总判不变，附加条件不变。