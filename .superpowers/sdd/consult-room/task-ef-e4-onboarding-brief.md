# E4 onboarding 窗 — implementer brief

编排者只设计，你写代码。不要问问题。不要 commit。

先读（按序）：
1. `.superpowers/sdd/consult-room/task-ef-pages-master-brief.md`（视觉语法总纲）
2. 真值：`docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-onboarding-v2.html`（602 行，**全文精读**，下文行号即指此文件）
3. 模式参照：`src/apps/desktop/src/ui_dioxus/pages_settings.rs`（E3 已落地的页窗样板：body 属性 / 双 style 注入 / chrome / 信号折叠 / DropGuard / register_window_with_hwnd / theme watch）

已在工作树（保留、在其上追加）：E1/E2/E3 三页窗、`DockSide::{LeftFull,RightFull,Center}`、room 状态行 `#nav-archive` / `#nav-space`、`spawn_module_window_with_theme_rx`（app.rs 导出）。

## 0. 裁决注（真值 > 任何转述，逐条照此执行）

handoff-20260824 §3 对真值的转述有 5 处与 HTML 实际内容不符，一律以真值 HTML 为准：

- **A 中枢**：不是「印章序 / 名知序」。真值 = avatar 字符「?」（`data-inhabited="false"` 时虚线边框无光晕，:114）+ name-line「未命名诊室」(:292) + state pill「沉寂态 · 等待人类决定第一个色彩」(:293)。
- **B 五色板**：不是「紫=开放性/深蓝=尽责性/暖珊瑚=外向性/柔绿=宜人性/冷青=神经质」。真值五色（:323-347，名字/色值/关键词逐字照抄）：**驱力 #C8714C 探索/开拓；深渊 #3F837B 凝视/沉淀；跃迁 #8B5FBF 突破/演进；凝视 #D99B48 审视/对齐；镇静 #4B8F6B 恒稳/收容**。field-label 标「性格色板 = 大五 MIND PALETTE（人类唯一可改色入口）」(:321)。
- **C deck 文案**：不是「你的言辞将被记录为见证 / 成为自己」。真值 = pledge「房间诞生完毕后，印记将融入基质」+ 光标 (:408)；primary 钮「☩ 唤醒诊室 · 开启印记」(:409)。
- **D 右抽屉**：handoff 漏了右抽屉「诞生存根」#work（仪式关卡 checklist + 印记预览 + term-well 终端，:415-435）——**必须实现**。
- **E 时序**：真值 = 点色板**即时**切 `--mind-base`（:526），软变化靠既有 CSS transition（border/avatar/containment 0.6s）。**没有** 350ms/1200ms 定时器；完成态**不自动关窗**。

## 1. 产品形态

独立 OS 窗 `id="onboarding"`，**新变体 `DockSide::Fullscreen`**：spawn 时精确覆盖 room 当前几何（x/y/w/h = room geom，视觉上是「主窗全屏被仪式替换」）；room geom 无效（≤0）时回落 registry 初始尺寸 **1280×860**。无 geometry follow。frameless + skip-taskbar（spawn 通道已有，不用管）。单例：重复点入口 = 聚焦已有窗（mark_opening 语义已有）。

布局 = 真值 `#engine`（:81, :224）：左抽屉 `#mind`（诞生前夜，默认 `mod-hidden`）+ 中央 `#room-wrap`/`#room` + 右抽屉 `#work`（诞生存根，默认 `mod-hidden`）。装饰层照真值渲染：`#containment` / `.membrane-frame` / `#global-aura` / `.room-fog` / `.membrane.l/.r`。

首启仪式页，日常不出现；本刀入口是 mock（见 §4）。

## 2. 组件清单（全部按真值转写）

**room-status 行**（:272-287）：brand-inline 真 logo SVG（5 path，:274-280，与 app.rs room 窗同款可抄）+ seal-name「northing」+「房间诞生仪式 00」+ `.sp` + `.state-dot` + 状态文本 id room-state-text 初始「沉寂中 · 待注入印记」。整行 = 拖拽区（`onmousedown: window().drag()`，按钮 stop_propagation）。窗控四键**只做 ☀/☾ 主题钮 + ✕ 关窗**（─□ 不做，与 E1-E3 轻 chrome 一致；主题钮 SVG 抄 pages_settings.rs 的日/月两套）。

**room-head 中枢**（:289-294）：head-fold ▴ 钮 + `.agent-avatar`（初始「?」）+ `.name-line`（初始「未命名诊室」）+ `.state` pill。折叠 = `room-head.folded` 横排 26px（真值 CSS :108-111 已有，信号切 class 即可）。

**仪式流 chat-flow**（:296-404）：三章 `ritual-divider`（「房间诞生仪式 · 第一章：身份凝结」/「第二章：管道贯通」/「第三章：物理锚定」）+ 三张 `ritual-card`：

1. **身份印记 IDENTITY**（步骤 I / III，:300-351）：ritual-narrative 叙述 + field-grid：用户是【】（默认「见证者」）/ 你是【】（默认「NortHing」，输入联动 name-line 与 avatar 首字符，= 真值 updateAgentName :518-523）/ 你是用户的【】（full 宽，默认「思维的镜面与延伸」）+ 大五色板 `palette-picker`（§0-B 五色，swatch = circle + name + desc，选中 `.selected`）。
2. **设施接入 PROVIDER**（步骤 II / III，:356-382）：模型 ENGINE（默认 claude-3-7-sonnet）/ 基址 BASE URL（默认 https://api.anthropic.com/v1）/ 密钥 API KEY（password 输入，placeholder「凭证密钥（不落盘于规格）」）+ test-row：「↯ 测定心跳脉冲」钮 + 状态文本（初始「等待测试信号...」；mock：点击 → ok 态「✓ 心跳贯通 · 延迟 12ms · 神经元就绪」，600ms 异步延迟可选，做不稳就即时 ok 并在 report 注明）。
3. **物理边界 WORKSPACE**（步骤 III / III，:387-403）：工作文件夹路径输入（默认 specimen 路径）+「浏览...」死钮（onclick noop）。

**底部 deck room-footer**（:407-410）：witness-pledge「房间诞生完毕后，印记将融入基质」+ `.cursor` 闪烁 + primary 钮「☩ 唤醒诊室 · 开启印记」（§3 完成态）。

**左抽屉 诞生前夜 #mind**（:227-254，两张 .mod 卡）：
- 卡1 诞生前夜：状态测定 STATUS — 两行 dot-radio（「物理空间待入住」active /「思维印记未凝结」）+ seg-bar 4 段（初始 1 段 on）+ seg-note「零沉淀 · 契约准备中」。
- 卡2 设施预备（station-head facility）：底层基质 RUNTIME — 两行 sq-toggle active（「Slint 规格架构」/「双光学冷热流」）；仪式公约 COVENANT — 一行 active「人可赋予印记，不能改写自我」。

**右抽屉 诞生存根 #work**（:415-435）：仪式关卡 STEPS 三行 plan-check（I. 身份凝结 未完成 / II. 信号连通 未测试 / III. 锚定边界 待确立）+ 印记预览 PREVIEW 两行（色板状态「未着色 (灰)」/ 实体命名「NortHing」）+ term-well 终端标本（:429-433 逐字）。

**抽屉显隐**：`membrane-node` 宝石左 trig-mind / 右 trig-work（:261-262），点击 toggle 对应抽屉 `mod-hidden` + 宝石 `is-open`。抽屉卡折叠用 **W2.7 语法**（点标题 `is-folded` + fold-caret，与 E1-E3 一致），不用真值的 ▴ 钮 + display:none 方案。抽屉不做拖移（真值 data-drag 不迁，静态布局）。

## 3. 交互状态机（mock，真值 JS :439-599 翻译为 dioxus 信号）

- **selectPalette**（:525-554）：body 内联 `style="--mind-base: {hex}"` + `data-inhabited="true"` + 该 swatch `.selected`（其余摘除）+ 状态文本「{name}状态 · 房间印记已铸造」+ state pill「{name}色板 ({desc}) · 印记已注入」+ 右抽屉 step1 done（plan-check ok 色，尾注「已凝结」）+ 预览色板行「{name} ({hex})」+ 左抽屉行2 → active「思维印记已凝结」+ seg 1、2 on + seg-note「印记形成中 · 1/3 已铸造」。
- **testConnection**（:556-575）：状态 → ok + step2 done「已通畅」+ seg 3 on + seg-note「印记形成中 · 2/3 已贯通」。
- **completeRitual**（:577-599）：**未选色 → 不弹 alert**（dioxus 无 alert），状态文本改内联提示「请先选择性格色板，为诊室注入第一个 mind 色印记。」；已选色 → step3 done「已立锚」+ seg 4 on + seg-note「仪式完毕 · 诊室已正式诞生」+ term 行「status: chamber fully inhabited」+ 主钮 →「✓ 诊室已诞生 · 空间运行中」ok 配色。**窗保持不关**（✕ 关窗回 room 即完成态出口）。
- **aura 定位**：静态默认（`--aura-x: 50%; --aura-y: 200px`），不建真值 :446-454 的 rAF 追踪环。
- 主题切换：body `data-theme` dark/light 信号切（同 E3），☀/☾ 图标随之换。

## 4. 接线

- `registry.rs`：`DockSide` 加 `Fullscreen` 变体；注册 `id="onboarding"`、title「northhing - 房间诞生仪式 (dioxus)」、1280×860、`Fullscreen`、`pages_onboarding::onboarding_app_root`。单测 `test_onboarding_registration_and_lifecycle` 抄 `test_settings_registration_and_lifecycle`（断言 Fullscreen + 1280×860 + 单例拒绝 + closing）。
- `app.rs`：① spawn 几何 match 加 `DockSide::Fullscreen` 臂 = `(room_x_log, room_y_log, room_w_log, room_h_log)`（room_w/h ≤0 时回落 plugin initial）；② room 状态行 `#nav-space` 之后（:334 附近、`span.sp` 之前）加第三个导航 `#nav-onboarding`：class `status-nav-link`（**css.rs :445 已有该 class 的选择器，零 CSS 增量**）、i18n 键 `NAV_ONBOARDING`（zh-CN 文案「诞生仪式」）、onclick `spawn_module_window("onboarding", ...)`，wm/geom/theme 克隆组照 nav-archive/nav-space 的既有写法补一组。
- `mod.rs`：`mod pages_onboarding;`
- **`css.rs` / `windows.rs` / TRUTH_CSS / consult-room-main.css：零触碰。**

## 5. CSS（关键约束）

- `pages_onboarding.rs` 内联 `const ONBOARDING_CSS`：真值 `<style>`（:18-216）**忠实转写**——color-mix 表达式 12 处逐字、呼吸 8s 单钟四组 keyframes、双光学 `[data-theme]` 两套变量、`prefers-reduced-motion` 块、media query 两块、`[data-inhabited="false"]` 虚线头像规则。
- **不注入** `css::truth_css()` / `OVERLAY_CSS`：真值页自带完整样式（自包含），避免与 TRUTH_CSS 里 room 专属规则级联打架；TRUTH_CSS 零触碰自然成立。body 属性：`data-theme` / `data-window="onboarding"` / `data-inhabited`。
- 行数：`pages_onboarding.rs` **<800**。超了就把 ONBOARDING_CSS 拆到同目录新文件（如 `pages_onboarding_css.rs`）再 `mod` 声明——**仍禁止进 css.rs**（已 778/800）。

## 6. i18n

- 新键 ×3 语：`src/crates/assembly/core/locales/{zh-CN,zh-TW,en-US}.ftl` + `i18n.rs` keys 模块（参照 NAV_ARCHIVE / SETTINGS_* 既有键）。
- 走 i18n：窗 title、chrome aria（主题/关窗）、状态行「房间诞生仪式 00」、抽屉头（诞生前夜/设施预备/诞生存根）、卡 title+em（身份印记 IDENTITY / 设施接入 PROVIDER / 物理边界 WORKSPACE）、步骤号、field label、按钮文案（测定心跳脉冲/唤醒诊室/浏览...）、初始状态三句（沉寂中 · 待注入印记 / 沉寂态 · 等待人类决定第一个色彩 / 未着色 (灰)）、NAV_ONBOARDING。
- 可硬编码中文标本（E1 地层同例）：ritual-narrative 三段叙述、mock 默认值（见证者/NortHing/思维的镜面与延伸/claude-3-7-sonnet/路径/终端 specimen）、动态合成句（「{name}状态 · 房间印记已铸造」等）、左抽屉 mock 行。
- zh-TW/en-US 翻译覆盖走 i18n 的键即可，硬编码标本不要求翻译。

## 7. 不要做

- 不动 room 路由/room 组件本体（只加状态行一个导航钮）；不接真 provider / 真首启检测（入口本刀就是 mock）；不做 ─□ 最小化最大化；不做抽屉拖移 / aura 追踪环；无 emoji（真值的 ❖↯◈☩✓▴▾ 是字符 glyph，照抄不算 emoji）；不 commit；flags 取证完必须 restore。

## 8. 验证（门禁，全绿才算完）

1. `C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing`（exit 0）
2. 同上 `cargo test -p northhing ui_dioxus`（基线 7 passed → 本刀后 **8 passed**）与 `cargo test -p northhing flags`（3 passed 不变）
3. `pnpm run i18n:audit`（exit 0，1 条 grandfathered warning 可保留）
4. 临时 `flags.rs:41 DIOXUS_SHELL=true`，`cargo build -p northhing`
5. CDP：先 `Stop-Process -Name northhing -Force`；Hidden 启动 + `--remote-debugging-port=9333`；开 room → 点状态行 `#nav-onboarding` → 对 onboarding 窗截图三张到 `C:\WINDOWS\TEMP\opencode\t7-shots\`：
   - `e4-onboarding-dark.png`（暗色初始灰寂态）
   - `e4-onboarding-light.png`（亮色初始态）
   - `e4-onboarding-selected.png`（点任一色板后：整屋着色 + 右抽屉打开露 step1 done）
   找窗用差分法 + retry-classify（前篇 handoff-20260823 §4 定版）；CDP 优先禁光标劫持。
6. **Read 打开三张 PNG 目验**：初始灰（#7e8896 未着色）/ 头像虚线 / 三章卡全 / 五色板全 / 着色态整屋换色 / 双光学差异真实存在。判据以目验终裁。
7. `git checkout -- src/apps/desktop/src/flags.rs`（restore false 实证）；`Stop-Process -Name northhing -Force`
8. 报告写 `.superpowers/sdd/consult-room/task-ef-e4-onboarding-report.md`：门禁命令+输出、截图路径、与 brief/真值的任何偏差及理由、行数统计（pages_onboarding.rs / app.rs / registry.rs 增量后行数）。

## 9. 完成定义

- room 状态行「诞生仪式」点开 onboarding 窗，精确覆盖 room；关得掉；再点不叠尸
- 灰寂初始态 → 点色板即时整屋着色（data-inhabited true，头像虚线转实）→ 完成钮 mock 态成立
- 双光学、左右抽屉显隐、中枢折叠、三章仪式流全在画面上
- flags=false、无 commit、门禁全绿、报告在案
