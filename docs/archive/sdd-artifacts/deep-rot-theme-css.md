# Deep Rot Review — 代码层腐化深审（theme.rs + css.rs）

> **量规版本**: deep-rot-review-rubric.md · **执行口径**: 只读（grep/codegraph/git log），不改代码不 commit  
> **对照**: rot-probe-2026-08-28.md（结构层初判：theme=持平, css=持平）

---

## 文件 1：`src/apps/cli/src/ui/theme.rs`（989 行）

### 总判定：**稳定** — 与结构层初判一致

> 结构层描述"主题数据 + 19 函数"基本准确（实际 ≈ 20 个 pub 项）。代码逻辑核心通路健康（dark/light/mono/ansi16 构造 + JSON 覆盖 + style() 映射）。结构性腐化风险在数据漂移据点，不在代码逻辑。

---

#### 1. 死代码

| 等级 | 发现 | 证据 |
|------|------|------|
| 腐化证据 | `load_opencode_theme_json()` (L728): 函数体自引用是唯一调用点，tombstone 注释 (L726-727) 确认 reserved for future | `rg load_opencode_theme_json` → 仅 L726 + L728 |
| 观察项 | `OpencodeThemeJson.schema` (L698): `#[allow(dead_code)]` + tombstone (L696-697)，字段被 serde 反序列化但从未读取 | 全仓 grep `OpencodeThemeJsonJson` 引用 schema 字段 = 0 |
| 干净 | `StyleKind` (L638): rot-probe 列为第 5 死代码项，实则**活跃使用**（6 文件 70+ 次 `theme.style(StyleKind::X)`）。`#[allow(dead_code)]` (L637) 是 stale 注解 | `rg StyleKind::` → 6 文件命中 |
| 干净 | `OpencodeThemeJson.defs` (L700): rot-probe 列为死代码，实则**活跃使用**（L831 `json.defs.clone()`, L934 `defs.get(t)`） | `rg "\bdefs\b"` → L831, L934 |
| 干净 | `parse_osc_color()` (L217): rot-probe 列为死代码，实则**内联调用**（L203 `parse_osc_color(color)?`） | L203 直接调用 |

**结论**: rot-probe 所列 5 项死代码中，1 真死 + 4 属误报（stale `#[allow(dead_code)]`）。真正的死代码积累量为 1 个函数 + 1 个字段，规模可控。

#### 2. 重复

| 等级 | 发现 | 证据 |
|------|------|------|
| 腐化证据 | 5 个主题构造函数（dark/dark_ansi16/light/light_ansi16/monochrome）重复设置相同的 25 个字段。加新字段须同时 touch 5 构造器 + `with_effective_scheme` (L416) + `apply_opencode_theme_json` (L449) + `ResolvedTokens` (L804)。数据漂移高发区 | L251-414 四组全字段初始化 |
| 腐化证据 | `dark_ansi16`/`light_ansi16` 可完全由对应 truecolor 构造器 + `to_ansi16()` 推导，当前是全量复制粘贴 | L284-315, L350-381 与 L251-282, L317-348 几乎一一映射 |

**定量**: 5 × 25 字段 = 125 处字段赋值，其中 3 个构造器的 ANSI16 版本 ≈ 75 处均可由 `Self { ..dark().ansi16() }` 一个方法替代。

#### 3. 模式不一致

| 等级 | 发现 | 证据 |
|------|------|------|
| 观察项 | env 变量读取风格混用：`terminal_supports_truecolor` 用 `.unwrap_or_default()` (L97, L102, L107)，`resolve_effective_color_scheme` 用 `.ok().as_deref()` (L74) | L74 vs L97 |
| 腐化证据 | `unsafe` 块 (L164-194) 写裸 `libc::fcntl` 调用，无 **SAFETY** 注释——违反仓库 AGENTS.md 核心规则（DeepReview 要求 unsafe 必须有 SAFETY block） | L164 `unsafe {` ... L194 `}`，无 SAFETY 注释 |
| 观察项 | `detect_terminal_appearance` 中 3 处 `let _ =` 吞错误：L152-153 写 stdout、L193 恢复 fd flags | 无 ponytail 注释说明为何可忽略 |

#### 4. 注释腐化

| 等级 | 发现 | 证据 |
|------|------|------|
| 稳定 | 4 处 tombstone 注释（L215, L635-636, L696-697, L699, L726-727）明确标注 reserved/future，不构成腐化 | 注释格式统一，有 reason 前缀 |
| 稳定 | L2 `/// Theme and style definitions` — StyleKind 实际活跃使用，注释无过期 | grep 确认 StyleKind 在所有引用处有效 |

#### 5. Hack/绕路

| 等级 | 发现 | 证据 |
|------|------|------|
| 观察项 | L131 `.unwrap_or(Appearance::Dark)` — `detect_terminal_appearance` 返回 `Option`，unwrap 其 None 为 Dark 是合理降级，但无注释说明为何不传播 `?` | 降级语义清晰，缺注释 |
| 腐化证据 | L164 unsafe 块无 SAFETY 注释（同第 3 项） | 见上 |

#### 6. 职责归属

| 等级 | 发现 | 证据 |
|------|------|------|
| 观察项 | `tool_icon()` (L661-689) — 工具名称→Unicode 符号的映射表，属 CLI UI 展示逻辑，非主题/色彩职责 | 函数体不含任何 Theme 引用 |
| 观察项 | `Theme` 结构体已从 854L (1b147c3) 增长到 989L，字段（diff hunk header/line number, command text, inline icon）逐批加入，无拆分迹象 | rot-probe 行数走势 |

#### 7. 复杂度热点

| 等级 | 发现 | 证据 |
|------|------|------|
| 干净 | 最大函数 `rgb_to_ansi16` 38L/12 臂 match（L595-633），在色彩转换范畴内可接受 | 无 >80L 函数，无 >4 层嵌套 |
| 干净 | `with_effective_scheme` 31L（L416-447）——25 字段逐一 to_ansi16，无结构复杂度 |  |

#### 8. 测试质量

| 等级 | 发现 | 证据 |
|------|------|------|
| 观察项 | 2 个测试：`builtin_themes_resolve_for_dark_and_light`（全主题遍历 smoke），`eight_digit_hex_colors_are_supported`（8 位 hex alpha 边界）——覆盖了解析主流路径 | L951-989 |
| 观察项 | 缺 unit test：`to_ansi16`/`rgb_to_ansi16` 无边界颜色验证；`resolve_effective_color_scheme` 无输入矩阵测试；`terminal_supports_truecolor` 零覆盖 | 无对应 #[test] |

---

## 文件 2：`src/apps/desktop/src/ui_dioxus/css.rs`（830 行）

### 总判定：**稳定** — 与结构层初判一致，重复问题略高于初判预期

> 结构层描述"纯 CSS 字符串嵌入，无逻辑膨胀"准确。TRUTH_CSS byte-lock 机制有效（L809 guard test）。OVERLAY_CSS 692 行覆盖层字符串为活的样式演进记录（R3'→W2.7），每次用户判决附 CSS 变更，可追溯。

---

#### 1. 死代码 / 孤儿 CSS

| 等级 | 发现 | 证据 |
|------|------|------|
| 稳定 | `#room-scrim` 选择器在 TRUTH_CSS 中为孤儿——app.rs 元素已删除，但 byte-lock 阻止移除 | L134-136 注释确认退役；TRUTH_CSS byte-lock → 无害残留 |
| 干净 | 所有 `body[data-window="archive"]/space/settings/inner/outer"]` 选择器均有对应渲染文件 | pages_archive.rs:135, pages_space.rs:186, pages_settings.rs:216, windows.rs:225/472/677 |
| 干净 | `.w2-head`, `.fold-caret`, `.w2-group-seam`, `.w2-stat`, `.w2-token`, `.room-controls`, `.rc-btn`, `.membrane-node`, `.agent-avatar` 均在 windows.rs / pages_*.rs 中渲染 | grep 逐类确认 |
| 干净 | `theme_toggle_svg` (L770) 和 `brand_logo_svg` (L795) 分别在 5/4 个文件中使用 | app.rs:413/453, pages_archive.rs:168/298 等 |

#### 2. 重复

| 等级 | 发现 | 证据 |
|------|------|------|
| 腐化证据 | **三窗口 chrome 三重奏**：archive (L452-556, ~105 行)、space (L560-711, ~152 行)、settings (L727-746, ~20 行) 的 chrome 结构（flex row, title, actions, fold/theme/close 按钮）完全相同，仅前缀名不同 | archive-chrome/space-chrome/settings-chrome 逐块结构一致 |
| 腐化证据 | 三窗口 CSS 自定义属性声明重复（L453-465 archive、L561-573 space、L728-729 settings 均设置 `--mind-base:#C8714C` + 5 个派生变量） | 3 处 near-identical `--mind-*` 块 |
| 腐化证据 | archive + space 的 `.mod.is-folded`、`.w2-pin`、`.w2-scroll`、`.side-title` 模式几乎逐行相同 | L485-493 (archive) vs L594-602 (space) 逐字段对比 |

**定量**: 约 280 行 OVERLAY_CSS（~40%）属于三窗口 chrome 的 copy-paste 变体，改一处须同步三处。

#### 3. 模式不一致

| 等级 | 发现 | 证据 |
|------|------|------|
| 观察项 | Settings 区 (L731-746) 使用单行压缩书写（多条规则 `}` 后接下一个选择器），而 archive (L452-556) 和 space (L560-711) 使用展开多行书写 | L731 vs L466 — 格式风格分化 |
| 观察项 | TRUTH_CSS (`include_str!`) + OVERLAY_CSS（inline Rust string）两种注入方式共存于同一文件 | L26 vs L54 — 架构上并列但职责边界清晰 |

#### 4. 注释腐化

| 等级 | 发现 | 证据 |
|------|------|------|
| 腐化证据 | L1-14 开头注释说 "until the dedicated `.css` file is extracted, we fall back" — 该临时代码现已 4 个月+ 且已稳定，fallback 概念已过时 | 时间戳 T1 (2026-08-12) |
| 观察项 | L12-13 "conversion-annotations 规则（color-mix 48, keyframes 21，radial-gradient 22，shadow 4式）"— 具体计数可能已失真（真值 CSS 后续有变更） | 注释收口但无法从当前文件验证数字 |

#### 5. Hack/绕路

| 等级 | 发现 | 证据 |
|------|------|------|
| 干净 | L40 `strip_prefix('\u{FEFF}')` — BOM 去除有完整根因说明 (L28-38) 和 R4 追溯 | 合理 workaround，文档到位 |

#### 6. 职责归属

| 等级 | 发现 | 证据 |
|------|------|------|
| 观察项 | `theme_toggle_svg()` (L770) + `brand_logo_svg()` (L795) — SVG 图形标记内嵌于 CSS 文件，属图形资源而非样式职责 | 函数体为纯 static str 常量，不含样式逻辑 |

#### 7. 复杂度热点

| 等级 | 发现 | 证据 |
|------|------|------|
| 稳定 | OVERLAY_CSS 692 行字符串常量 (L54-746) — 无法函数化，但每段有清晰的 `/* ---- Rx.Wy */` 区块注释分隔 | 受 TRUTH_CSS byte-lock 约束，架构不变更即无法拆分 |
| 干净 | 无 >80L 函数 | `inject_stylesheet_html` 2L，`truth_css` 3L，`theme_toggle_svg` 8L |

#### 8. 测试质量

| 等级 | 发现 | 证据 |
|------|------|------|
| 观察项 | `assert_truth_css_byte_count` (L809-829) — byte-count guard 有效但不覆盖 OVERLAY_CSS 变化，任何 OVERLAY_CSS 修改无法自动回归检测 | 仅断言 TRUTH_CSS.len() == 22240 |
| 观察项 | 无 selector-match 覆盖率测试——无法确认 OVERLAY_CSS 中的选择器是否匹配实际渲染的 DOM 元素 | 缺集成/回归层覆盖 |

---

## 发现汇总

| 文件 | 腐化证据 | 观察项 | 总发现数 |
|------|---------|--------|---------|
| `theme.rs` | 4（数据漂移重复×2 / unsafe 无 SAFETY comment×1 / stale 注解误导×1） | 5 | 9 |
| `css.rs` | 3（三窗口 chrome 三重奏×1 / 注释过时×1 / 格式风格分裂×1） | 5 | 8 |

**总计：腐化证据 7 + 观察项 10 = 17 项发现**

---

## 与初判对比

| 文件 | 初判 | 终审 | 变化 |
|------|------|------|------|
| theme.rs | 持平 | **稳定** | 一致；补充了 stale `#[allow(dead_code)]` 误报修正 |
| css.rs | 持平 | **稳定** | 一致；确认重复比预判更系统化（3 窗口 chrome 三重奏），但受架构约束难以消除 |

> 两文件均未推翻结构层判定。结构层的"行数持稳 + 职责清晰"信号在代码层得到加强。theme.rs 的真正风险是数据漂移（加字段须 7 处同步），css.rs 的真正风险是 OVERLAY_CSS 窗口 chrome 的结构性重复——两者都是" Growth will worsen if unaddressed" 而非当前腐化。
