# W15-1 仲裁输入包 — Markdown 渲染选型（2026-09-02）

> 仲裁员：独立裁决，只读。产出裁决书到 `.superpowers/sdd/w15-1-arbitration.md`。
> 本包 = 全部事实来源；编排者不提供倾向性结论，选项与取舍由你裁决。

## 0. 任务

裁决桌面 Dioxus consult-room 聊天消息的 Markdown 渲染方案。需裁决的子问题：

1. **解析 crate 选型**（见 §2 候选与事实）
2. **注入安全方案**：`dangerous_inner_html` + 消毒 vs 事件流直接映射 RSX（见 §3）
3. **渲染覆盖范围**：assistant（Entity）/ user（Witness）/ 流式 draft / archive 只读页，哪些上 Markdown
4. **CSS 落点**（css.rs 790/790 零余量硬约束下）
5. **拆单建议**：实现单怎么切、验收门是什么

硬约束（不可裁掉）：
- 模型输出不可信，**禁止未消毒 raw HTML 注入**（用户已拍板的验收重点）
- `css.rs` rot 闸 790/790，**不许加行**；新样式只能走独立文件（`pages_onboarding_css.rs` 活先例）
- 家规 YAGNI + 已装依赖优先；新增依赖准入已批（用户 2026-09-02 拍板：允许引入 markdown crate，选型闭环不上交）
- dioxus 钉死 `=0.8.0-alpha.1`（workspace 锁文件一致），选型必须兼容此 alpha 版

## 1. 侦察事实（explore 子代理磁盘取证，已逐条 file:line 核实）

### 1.1 渲染点（全链）

- 真实消息：`app.rs:74` `api::get_messages` → `session_mock.rs:102-161` `messages_to_entries` → `Vec<MockEntry>`（User→Witness，Assistant→Entity，System/Tool 丢弃）
- 渲染链：`app.rs:501` `render_entries` → :722-734 循环 → `render_entry`（:736-780）
- 文本叶子：assistant = `app.rs:747` `div.msg-agent "{body}"`；user = `app.rs:757` `div.body "{body}"`（纯文本插值，Dioxus 自动转义）
- 流式 draft：`app.rs:112-118` TextChunk 累积进 `assistant_draft: Signal<Option<String>>`；渲染点 `app.rs:502-509`，markup 与完成态相同但在 entries 列表外
- archive 只读页：`pages_archive.rs:49-72`（content 拍平 String）+ :652-661 `div.mem-text` 纯文本
- mock 与真实共用 MockEntry/render_entry（session_mock.rs:19-40）

### 1.2 现有 dangerous_inner_html 使用

仅 CSS 字符串与 SVG 图标注入（app.rs:330-331、各 pages_*、windows/* 共 12 处 style 注入 + theme_toggle_svg/brand_logo_svg），**无任何消息文本注入先例**。

### 1.3 CSS

- `css.rs` 790 行：TRUTH_CSS（include_str! 真值、字节锁定、:769 有 assert 守卫，禁改）+ OVERLAY_CSS（:52 转写层大字符串）+ 2 个 svg fn
- 独立文件活先例：`pages_onboarding_css.rs`（mod.rs:43 注册，导出 ONBOARDING_CSS 常量，页面内 style 注入）
- **孤儿注意**：`css_files.rs` 存在但未注册 mod、零消费（侦察意外发现，与本决策无关，记 follow-up）
- 消息正文 CSS 现状：`.msg-agent` 用衬线体 Fraunces/Noto Serif SC；**无 white-space: pre-wrap / word-break**，换行折叠、代码围栏按普通字符显示；`--font-mono` 变量存在但未用于正文

### 1.4 依赖现状

- desktop Cargo.toml :13-68 无 markdown|cmark|comrak|syntect|ammonia
- **workspace 已有 pulldown-cmark 0.11 + syntect 5**（根 Cargo.toml:130-132 → src/apps/cli/Cargo.toml:46,53-54，CLI 在用）——加进 desktop 只是新增 dep 边，crate 本体已在 lock
- MessageContentDto（contracts/kernel-api/src/session.rs:179-199）四变体：Text(String 纯文本) / Multimodal / ToolResult / Mixed{reasoning_content, text, tool_calls}

## 2. 候选 crate 事实（2026-09-02 联网核实）

| crate | 定位 | 与本仓关系 |
|---|---|---|
| `pulldown-cmark` 0.11 | Rust 生态事实标准，pull-parser 事件流，性能/内存最优 | **已在 workspace（CLI 在用）** |
| `comrak` | CommonMark + GFM 全内置（表格/任务列表开箱），AST 略重 | 新依赖 |
| `markdown-rs` | micromark 系，较新 | 新依赖 |
| dioxus 社区组件（dioxus-markdown 等） | 打包方案 | 需核实与 0.8.0-alpha.1 兼容性，新依赖 |

## 3. 注入安全两条路线（裁决核心）

**路线 A：cmark → HTML 字符串 → ammonia 消毒 → dangerous_inner_html**
- 业界标准模式；ammonia = Rust 消毒事实标准（新依赖）
- 代价：+1 新依赖（ammonia）；dioxus alpha 版 dangerous_inner_html 行为需实测；样式需对消毒白名单对齐（class 可能被剥）

**路线 B：cmark 事件流 → 直接映射 Dioxus RSX 元素（不落 HTML 字符串）**
- 零注入面：根本没有 HTML 字符串，消毒问题不存在
- 代价：要写 event→rsx 映射层（段落/标题/列表/代码块/内联码/链接/强调，约 100-200 行）；表格等 GFM 扩展按需
- pulldown-cmark 的事件流 API 天然适配此路线

**路线 C：社区 dioxus-markdown 组件**
- 需核实：0.8.0-alpha.1 兼容性、内部是否消毒、维护状态

## 4. 裁决书要求

输出 `.superpowers/sdd/w15-1-arbitration.md`，结构参照 `.superpowers/sdd/w14-1b-arbitration.md`：
- 主判（≤5 条）+ 每子问题裁定与理由
- 成本估算（S/M/L + 人天）
- 附带条件（验收门、不许碰清单、rot 闸注意点）
- 「我无法判定的项」诚实记录
- 签署（技术细则闭环，不上交用户——用户 2026-08-28 治理规矩 + 2026-09-02 依赖准入已批）
