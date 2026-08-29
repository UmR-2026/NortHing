# 盲审报告：`src/apps/desktop/src/ui_dioxus/css.rs`（2026-08-29）

- 目标：`src/apps/desktop/src/ui_dioxus/css.rs`
- 实测规模：**829 行**（`git grep -c "" -- <path>` = 829；worktree 与 `HEAD` blob 一致，`git status --porcelain` 空）；blob 77148 字节
- 登记 ceiling：**830**（`scripts/rot-budget.json` `god_file:src/apps/desktop/src/ui_dioxus/css.rs`，note = "R-14 god-file; live observation cohort; registered with user sign-off 2026-08-26"）
- 余量：**1 行**
- 分支：main，只读，未改代码、未 commit、未跑 cargo
- 盲态：未读取任何既有审查报告（`deep-rot-*` / `blind-review-*` / `*-review.md`），故**无法与结构层初判比对**——本报告是独立判定

## 总判定：**腐化中**

一句话理由：CSS 以"轮次叠加"方式生长（R3'→R4→R5→R6→R7→R8→R9→W2.7→E1/E2/E3），旧轮被完全覆盖的声明与被删元素的墓碑注释原样留存，同一张卡片样式在文件内复制 7 份，而 830 行预算闸最近一次是靠**把三条规则并到同一行**（commit 57513b6 消息逐字承认）+ **把新规则外溢到 `css_files.rs`**（该文件头注释逐字承认）来满足的——度量已与它想度量的东西脱钩。

> 与结构层初判的一致性：**无法比对**（盲态纪律禁止读 `rot-probe-2026-08-28.md`）。

---

## 清单逐项

### 1. 死代码 —— 发现 3 类，**腐化证据**

| # | 证据 | 分级 |
|---|---|---|
| 1.1 | `pub fn inject_stylesheet_html()` (`css.rs:753-755`) 全仓零调用点。真实注入路径是 10 处 `style { dangerous_inner_html: "{css::truth_css()}" }`（`app.rs:330`、`windows/work.rs:133`、`windows/self_app.rs:142`、`windows/facility.rs:127`、`pages_settings.rs:237`、`pages_space.rs:187`、`pages_memory.rs:217`、`pages_archive.rs:376`）。其 doc（`css.rs:750-752`）仍宣称 `dioxus::desktop::document::Stylesheet` 是"the supported mechanism"，与实际实现路径不符。 | 腐化证据 |
| 1.2 | 死 CSS 块 `.depth-bar` / `.depth-seg`（7 条 nth-child）/ `.depth-note`：`css.rs:504-513`，共 10 条规则。全仓 `.rs` 零引用（仅出现在 `docs/design/.../consult-room-archive-v2.html` 等设计稿）。`pages_archive.rs` 侧栏实际只渲染 `.mod > .side-title/.w2-scroll/.row/.dot-radio`（`pages_archive.rs:407-414`），`depth` 唯一命中是 i18n key `ARCHIVE_SECTION_DEPTH_TITLE` 与 `.depth-marker`（`pages_archive.rs:410/441`）。 | 腐化证据 |
| 1.3 | 被后续规则**完全覆盖**的死声明（同选择器、同特异性、后定义胜出）：`#room .room-status { padding-right: 160px }` (`css.rs:198`) 被 `css.rs:293` 的 `136px` 覆盖；`#room .membrane-node` 系列在 `130,131,156,157,158,259,306,307,308,309,310,311` 共 12 处声明被 `252,253,318,319,320,321,322,323,324,325,326` 覆盖（opacity 改 3 轮、width 改 2 轮、`::before` width/box-shadow 各改 2 轮）。 | 腐化证据 |

### 2. 重复 —— 发现 8 组，**腐化证据（本文件最重的一项）**

文件内复制块（每组均 >5 行等价规则，逐字或近逐字）：

| 模式 | 出现行 | 次数 |
|---|---|---|
| `.mod` 卡片五件套（`border/border-top-color: var(--bevel)/radius/bg2/shadow`） | 227, 414, 485, 594, 623, 672, 736 | **7** |
| `.is-folded > :not(.side-title) { display: none !important; }` | 399, 416, 489, 598, 627, 736 | **6** |
| `.w2-pin { flex: 0 0 auto; margin: 0; padding …}` | 402, 490, 599, 628, 737 | **5** |
| `.side-title { font-family: var(--font-mono); font-size: 9px; …user-select: none; }`（逐字节相同） | 494, 603, 632, 738 | **4** |
| chrome `.close-btn:hover { background: var(--danger); color: #fff; }` | 476, 584, 723, 733 | **4** |
| 主题变量块 `--mind-glow/--mind-intense/--mind-line/--accent-solid/--frame` | 452-458, 560-566, 727 | **3** |
| `.dot-radio { width: 7px; height: 7px; border-radius: 50%; …}`（逐字节相同） | 501, 611, 740 | **3** |
| `.sq-toggle { width: 7px; height: 7px; …}` | 613, 741 | **2** |

archive / space / settings 三窗块（`css.rs:451-744`，294 行 = 全文件 35%）本质是同一套侧栏卡片样式换 `body[data-window="…"]` 前缀重打三遍。

跨文件重复：滚动条六规则块 `css.rs:165-170` ↔ `pages_onboarding_css.rs:196-201` 逐条同款，后者注释逐字写明"同 css.rs 同款；本窗自包含不注 TRUTH_CSS，需自带"（`pages_onboarding_css.rs:195`）。

### 3. 模式不一致 —— 发现 3 项，**腐化证据 1 + 观察项 2**

- 3.1 **排版规律断裂**：全文件基本"一行一规则"，但 `css.rs:730-744`（settings 块）每行塞 2-4 条规则，`css.rs:86` 一行塞 3 条规则（`.close-btn` / `.degraded-banner` / `.close-btn:hover`，中间用多空格粘连）。断裂点与预算闸重合（见 §预算闸）→ **腐化证据**。
- 3.2 **同一问题三种组织法并存**：全局 `OVERLAY_CSS`（`css.rs:55`）/ 独立文件 `css_files.rs:12 FILES_OVERLAY_CSS` / 页面自带 `pages_onboarding_css.rs`。新 CSS 该落哪里没有规则可循 → 观察项。
- 3.3 Rust 侧错误处理零混用：全文件仅 1 处 `unwrap_or`（`css.rs:41`，`strip_prefix` 的正确用法），无 `unwrap()/expect()/let _ =` → 该维度**干净**（抽查方法：`rg` 全文件仅 `css.rs:41` 命中）。

### 4. 注释腐化 —— 发现 3 组矛盾 + 6 处墓碑，**腐化证据**

- 4.1 **头注释与实现矛盾**：`css.rs:9-14` "Until the dedicated `.css` file is extracted, we fall back to the full `<style>` block from the truth HTML"——而 `css.rs:26-27` 早已是 `include_str!(".../consult-room-main.css")`，专用 `.css` 文件已存在（实测 22240 字节）。前提条件已消失，注释未撤。
- 4.2 **doc 记录已删规则**：`OVERLAY_CSS` doc 的选择器约定（`css.rs:51-54`）仍在讲 `#room-scrim` 压暗层"只作用于 room 主窗"，而规则已在 `css.rs:133-135` 被清空（R8 退役），且 `room-scrim` 全仓零引用。
- 4.3 **数值矛盾**：`css.rs:244-246`（R6.1）称"R5 布局上移后重测 `--gem-mid=123px`"并按 123 推导右结对称式；但唯一声明 `css.rs:129` 是 `--gem-mid: 85px`，且 `css.rs:124-127` 的推导写的是"物理 123px ÷ K≈1.44 → 逻辑 85px"。两段注释对"123 是不是 CSS 值"给出相反读法，其一必为陈旧。
- 4.4 **墓碑注释 6 处**（规则/元素已删，注释留坟）：`114-119`（F3 整段无任何规则）、`133-135`、`191-192`、`261-262`、`277`、`330-333`。
- 4.5 弱化项（观察项）：`css.rs:152-155` 记录的标定值 `.85/.45/.8` 已被 R8.1（`318-320`：`.9/.72/.95`）取代，原注释未标 superseded，自上而下读会读到失效数值。
- 4.6 `css.rs:768` `/// \`# ponytail: two branches …`（反引号未闭合、`#` 混在 doc 里）→ 观察项。
- 4.7 **干净项**：`css.rs:18-19` 声称真值取自 html 的 `lines 27..273` —— 实测 `consult-room-main.html` 中 `<style>` 在第 27 行、`</style>` 在第 273 行，注释**准确**。`css.rs:811` 声称文件带 UTF-8 BOM —— 实测首三字节 `239,187,191` = EF BB BF，**准确**。

### 5. hack / 绕路 —— 发现 3 项，**观察项**

- 5.1 `!important` 12 处（实测 `rg -o -F '!important'` = 12），集中在折叠态 `flex: 0 0 auto !important` / `display: none !important`——用于压过同文件更早轮次的 `flex: 1 1 auto`，是自造级联冲突的补丁，非外部约束。
- 5.2 魔数耦合 DOM 按钮数且无测试保护：`#room .room-status { padding-right: 160px }`(`198`，注释"五钮 148 + 余量") → `136px`(`293`，注释"四钮 4x28+3x4+10")。按钮增删会静默压线，无任何自动化守卫。
- 5.3 其余魔数（`--gem-mid: 85px`、`-4px` 抵消、`K≈1.44`）**均带成因注释**（`122-128`、`304-305`、`335-340`）→ 该子项抽查结论：**干净**。
- 5.4 无可疑 sleep / retry / polling（`rg` 零命中）→ 干净。

### 6. 职责归属 —— 发现 2 项，**观察项**

- 6.1 `css.rs` 持有 SVG 图标资产：`theme_toggle_svg`/`SUN_SVG`/`MOON_SVG`（`769-784`）、`brand_logo_svg`/`BRAND_SVG`（`794-798`）。图标 markup 不是 CSS；`ui_dioxus/` 下无 icons 模块，是就近堆放。注：该 dedup 本身是**正收益**（消除了 10x/4x 内联复制，`css.rs:765-766`/`792-793` 有记录），只是落错了文件。
- 6.2 archive/space/settings 三窗样式（`451-744`）住在通用 `css.rs`，而其 DOM 分别住在 `pages_archive.rs`/`pages_space.rs`/`pages_settings.rs`；同仓已有相反范式（`pages_onboarding_css.rs` 页面自带、`css_files.rs` 随 `panel_files.rs`）。样式与其唯一消费者被拆开。
- 6.3 `src/apps/desktop` 下无就近 `AGENTS.md`（实测 `Get-ChildItem -Recurse -Filter AGENTS*.md` 零命中），故只能对照根 `AGENTS.md` 房规 3（>800 行升审查压力、>1000 行须拆或挂 `// allow-god-file`）：829 行**形式合规**（未越 1000，也无需 allow 标注），但已贴住自登记的 830。

### 7. 复杂度热点 —— 发现 1 项，**观察项**

- 7.1 Rust 函数层**干净**：全文件 5 个函数（`40`、`753`、`769`、`794`、`808`），最长 `assert_truth_css_byte_count` 21 行，无 >80 行函数、无 >4 层嵌套、无 >6 参数、无 match（实测 `rg '^(pub )?fn |^    fn '`）。
- 7.2 真热点是**单个 691 行字符串常量** `OVERLAY_CSS`（`css.rs:55-745`）= 全文件 83.4%（691/829），内含 416 个 `{`。它是不可被编译器/clippy/rustfmt 检查的盲区：重复、死规则、级联冲突全部只能靠人读发现——这正是本次 §1/§2 findings 的产地。

### 8. 测试质量 —— 发现 3 项，**腐化证据**

内联测试仅 1 个：`assert_truth_css_byte_count`（`css.rs:807-828`）。

- 8.1 **变更探测器**：`assert_eq!(TRUTH_CSS.len(), 22240)`（`813-820`），失败消息自带解法"bump EXPECTED_BYTES here"（`818`）——守卫的官方修法就是改守卫本身，任何合法 CSS 编辑都触发一次无信息量的红灯。当前实测 `consult-room-main.css` = 22240 字节，测试通过。
- 8.2 **关键路径未覆盖**：`truth_css()`（`css.rs:40-42`，R4 真实线上 bug 的修复——BOM 导致 `:root` 全灭）**零测试**。唯一断言打在未剥 BOM 的 `TRUTH_CSS` 上（`815`、`825`），即"被修的那个函数"没有回归保护。
- 8.3 **83% 代码零覆盖**：`OVERLAY_CSS`（691 行）无任何测试引用（实测 `rg -F OVERLAY_CSS` 在 tests 域 0 命中）；`theme_toggle_svg` / `brand_logo_svg` 亦无测试。
- 8.4 干净子项：`assert!(TRUTH_CSS.contains(":root {"))`（`824-827`）是真断言、有语义（真值调色板必须在首），非恒真。

---

## judge 5 项必查

### A. 复用核查
**不通过。** 文件内 8 组重复（§2），最高一组重复 7 次；跨文件与 `pages_onboarding_css.rs:196-201` 逐条同款且注释自陈"同 css.rs 同款"。CSS 变量/`:root` token 已在真值层可用，但卡片五件套、折叠三件套没有被提成公共选择器组（例如 `.mod` 全局一次 + 各窗只写差异），而是逐窗复制。

### B. 无 owner 抽象
**发现 1 处：** `inject_stylesheet_html()`（`css.rs:753-755`）—— 为 Brief §2.6 的 `document::Stylesheet` 机制预留的注入封装，实际三窗全部改走 `dangerous_inner_html`，函数零调用者、doc 描述的机制未被采用。它是没有消费者、也没有人负责撤销的遗留抽象。

### C. 预算闸
**这是本文件最严重的系统性发现。** 三条独立硬证据：

1. **提交消息逐字**（`git show 57513b6`）："I-2: css.rs 831→830 by merging 3 CSS rules onto one line each." —— 前一提交 `82371f5` 新增 `.degraded-banner` 把文件推到 **831 行（越 830 ceiling）**，随后不是删规则，而是把三条规则并行压回 830。产物就是今天的 `css.rs:86`（一行三规则）与 `css.rs:85`（原两行折成一行）。
2. **同仓文件头逐字**（`css_files.rs:3-8`）："Lives outside `css.rs` because that file was already at the 830-line rot-budget ceiling before this round; adding the ~15 selectors … would have pushed it over. … we keep `css.rs` byte-identical" —— 新功能样式被挤到新文件，闸没拦住复杂度，只改变了它的落点。
3. **生长曲线**（逐 commit `git grep -c ""` 实测）：58 (`727f899`, 08-13) → 154 → 398 → 479 → 475 → 778 (`43d3dfb`, 08-24) → **830 (`ba91f14`, 08-24，正好贴顶)** → 831 (`82371f5`, 08-29) → 829 (`57513b6`, 08-29)。16 天 ×14 倍增长，最后三次提交在 ceiling ±1 行内贴地飞行。

结论：`file-lines` 度量在本文件上已被行合并 + 文件外溢**双重脱钩**，当前 829/830 的"合规"不代表复杂度被控制。

### D. 纯位移等价
**N/A** —— 本次为无 diff 的存量代码审查（`git status --porcelain` 对该文件为空，worktree == HEAD），不存在需要验证位移等价性的改动。

### E. 证据抽查
见下节，逐条列出。

---

## 证据抽查

每条断言 + 当次实测方法（数字全部本轮实测，未凭记忆）。

| # | 断言 | 验证方法与结果 |
|---|---|---|
| E1 | 文件 829 行 | `git grep -c "" -- src/apps/desktop/src/ui_dioxus/css.rs` → `829`；`(Get-Content).Count` → 829；Read 工具尾行 = 829。（注：`git cat-file -p ... \| Measure-Object -Line` 给出 661，为 PowerShell 管道解码伪值，已弃用该口径） |
| E2 | ceiling = 830 | `Select-String css scripts/rot-budget.json -Context 4,4` → `"god_file:src/apps/desktop/src/ui_dioxus/css.rs": { "kind": "file-lines", "ceiling": 830 }` |
| E3 | `inject_stylesheet_html` 零调用 | `rg -n inject_stylesheet_html --glob '*.rs'` → 唯一命中 `css.rs:753`（定义）；codegraph_explore blast radius 对该 fn 未列出任何 caller（同调用中 `TRUTH_CSS` 正确列出 2 callers，说明索引有效） |
| E4 | 真实注入走 `dangerous_inner_html` | `rg -n 'truth_css\(\)' --glob '*.rs'` → 8 个渲染点 + `css.rs:41/754`；`rg -n OVERLAY_CSS` → 7 个渲染点 |
| E5 | `.depth-bar/.depth-seg` 死块 | `rg -c -F -e depth-bar -e depth-seg -g '!**/css.rs' .` → 仅 `docs/design/...archive-v2.html`、`minimax-m3-archive.html`；`rg -i depth --glob '*.rs' -g '!**/css.rs' src/apps/desktop` → 仅 `api_fs.rs` 的 `max_depth`、`i18n.rs` 的 DEPTH key、`pages_archive.rs:410/441`；回读 `pages_archive.rs:405-416` 确认 410 是 i18n key、441 是 `.depth-marker` |
| E6 | `.mod` 卡块重复 7 次 | `rg -n -F 'border-top-color: var(--bevel)' css.rs` → 行 227,414,485,594,623,672,736（7 命中） |
| E7 | 折叠规则重复 6 次 | `rg -n -F 'is-folded > :not(.side-title) { display: none !important; }' css.rs` → 399,416,489,598,627,736 |
| E8 | `.w2-pin` 5 次 / `.side-title` 逐字 4 次 / `.dot-radio` 3 次 / `.sq-toggle` 2 次 / chrome hover 4 次 / 主题变量块 3 次 | 逐条 `rg -n -F <完整规则串> css.rs`，命中行号见 §2 表；`.side-title` 用 `rg -c -F` → 4；主题变量块 `rg -c -F -e '--mind-glow: color-mix'` → 3 |
| E9 | 滚动条块跨文件重复 | `rg -n -F '::-webkit-scrollbar' --glob '*.rs' .` → `css.rs:165-170` 与 `pages_onboarding_css.rs:196-201` 各 6 条同名规则；回读 `pages_onboarding_css.rs:195` 见"同 css.rs 同款"自陈 |
| E10 | membrane-node 26 行声明 / 12 处被覆盖 | `rg -n -F '#room .membrane-node' css.rs` → 26 个行号（130,131,156,157,158,252,253,254,255,259,303,306-311,318-326）；覆盖判定 = 回读源码比对同选择器同特异性的后定义（opacity: 156→259→318；width: 130→252；hover width: 131→253→324；`::before` width: 306→321；box-shadow: 307/308→322/323，310/311→325/326） |
| E11 | `room-status` 160px 死规则 | `rg -n -F '#room .room-status' css.rs` → 198 (`padding-right: 160px`) 与 293 (`padding-right: 136px`)，同选择器同特异性，293 在后 → 198 失效 |
| E12 | `room-scrim` 全仓零引用 | `rg -F room-scrim --glob '*.rs' src/apps/desktop/src` → 0 命中；回读 `css.rs:51-54`（doc 仍在讲该选择器）与 `css.rs:133-135`（规则已清空） |
| E13 | 头注释矛盾（4.1） | 回读 `css.rs:9-14` vs `css.rs:26-27`；`Test-Path docs/design/.../consult-room-main.css` → 存在，`(Get-Item).Length` → 22240 |
| E14 | `--gem-mid` 数值矛盾 | `rg -n -F gem-mid css.rs` → 126(注释),129(声明 `85px`),243(注释 `123px`),254,255(使用)；唯一声明是 129 |
| E15 | BOM 存在、字节数 22240、测试当前通过 | `[System.IO.File]::ReadAllBytes(...)[0..2]` → `239,187,191`；`len` → 22240；对比 `css.rs:813` `EXPECTED_BYTES = 22240` → 相等 |
| E16 | html `<style>` 在 27..273（注释准确） | `rg -n -F -e '<style' -e '</style' consult-room-main.html` → 27 / 273；文件共 618 行 |
| E17 | `!important` 12 处 | `(rg -o -F '!important' css.rs \| Measure-Object).Count` → 12 |
| E18 | 无 >80 行函数、5 个函数 | `rg -n '^(pub )?fn \|^    fn ' css.rs` → 40,753,769,794,808；最长跨度 808-828 = 21 行 |
| E19 | `OVERLAY_CSS` 691 行 / 83.4% / 416 个 `{` | 起止 `css.rs:55` 与 `css.rs:745`（Read 工具行号）→ 745-55+1 = 691；691/829 = 83.4%；`(rg -o '\{' css.rs \| Measure-Object).Count` → 416 |
| E20 | 预算闸证据 1（行合并） | `git show 57513b6 -- css.rs` → 提交消息含 "I-2: css.rs 831→830 by merging 3 CSS rules onto one line each"，diff 显示 4 条规则被折行 |
| E21 | 预算闸证据 2（外溢） | `Get-Content css_files.rs` 头 8 行逐字含 "already at the 830-line rot-budget ceiling … we keep `css.rs` byte-identical" |
| E22 | 预算闸证据 3（生长曲线） | 逐 commit `git grep -c "" <sha> -- css.rs`：727f899=58, 5bcc285=154, a2e60b1=92, a202028=398, 3c9fb2d=479, 7387815=475, 43d3dfb=778, ba91f14=830, 82371f5=831, 57513b6=829 |
| E23 | 测试域零覆盖 `OVERLAY_CSS` | `rg -n -F OVERLAY_CSS --glob '*tests*' src` → 0；`css.rs` 内 `mod tests` 仅 1 个 test（`css.rs:807`），断言只触 `TRUTH_CSS` |
| E24 | 无就近 AGENTS.md | `Get-ChildItem -Recurse -Filter 'AGENTS*.md' -Path src/apps/desktop` → 0 命中 |
| E25 | worktree 干净、分支 main | `git rev-parse --abbrev-ref HEAD` → main；`git status --porcelain -- css.rs` → 空；`git diff --stat HEAD -- css.rs` → 空 |

## 无法判定

1. `css.rs:244-246` 与 `css.rs:124-127` 关于 `--gem-mid`（123 vs 85）哪一段是陈旧的——代码值确定为 85px，但无法从静态代码判断 R6.1 当时是否真的想改成 123 却漏改。需要 UI 实测或作者确认。
2. `.stratum[data-depth="N"]`（`css.rs:546-553`）等 archive/space 细类是否全部有对应 DOM——本轮只逐一核了 `depth-bar`/`depth-seg`/`depth-note`/`room-scrim`/`witness-row`/`chronicle-bar`/`head-seam-fold`/`state-dot`/`w2c-*`/`term-well`，其余未逐条穷举，不做断言。
3. 三窗块（451-744）里被真值 CSS 覆盖/覆盖真值的实际级联结果，静态阅读无法完全判定（依赖运行时注入顺序），仅按同文件后定义胜出规则判定了同选择器冲突。
