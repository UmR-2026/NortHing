# R4-R9 视觉收口轮报告（2026-08-14，编排者亲自执行）

派发通道 cancelled，用户指示「你自己来做」。范围从 R4 brief（W1-W5）扩展到用户现场判决的 R5/R6/R7/R8/R9 五轮迭代。全部改动集中在 `ui_dioxus/{app,css,entry,i18n}.rs`，真值 HTML/CSS 未动。

## 完成项

### R4（brief 原定范围）
- **W1 frameless**：entry.rs room 窗 `with_decorations(false)` + `with_undecorated_shadow(true)` + `with_menu(None)`（菜单根因：tao `MenuBuilderState::Unset → default_menu_bar()`，显式 None 去除）。─□✕ 接真功能：`set_minimized(true)` / `toggle_maximized()` / `quit_shell()`（= `std::process::exit(0)`，规避 never-type fallback deny；关 room = 退出整个 shell，防 inner/outer 孤儿窗）。room-head onmousedown → `window().drag()`，avatar/chronicle-bar stop_propagation。
- **W2** rc-btn line-height:1 中线对齐；room-status padding-right 避让。
- **W3** 宝石 opacity 标定 .85/.45/.8；chronicle-bar 静态渐变（真值 JS 生成的转写层近似）。
- **W4** 三窗自定义滚动条（10px 槽、透明轨道、禁箭头按钮）。
- **W5** 侧栏模块化（后被 R5 推翻重来，见下）。

### 最大根因发现：BOM
真值 css 文件带 UTF-8 BOM（`EF BB BF`），`include_str!` 注入后 `:root` 变 `\u{FEFF}:root` 永不匹配 → 全部 :root 变量（--mind-base/--accent-solid 等）失效，宝石渐变/avatar 边框/渐变条全隐形；`[data-theme]` 块不受影响故亮暗正常。修复 = `css::truth_css()` 函数 strip BOM（TRUTH_CSS 常量保持逐字节，守卫测试不受影响），三处注入点 + `inject_stylesheet_html()` 全部换用。

### R5 去嵌套（用户判决「所有除了窗口的黑色部分都是要去掉的」）
room 铺满：`#engine padding:0`、`#room border/radius/shadow:none`、`#containment/.membrane-frame display:none`；侧栏 padding:0（推翻 R4 W5 的 10px——方向反了）；outer 四独立卡（aside#work 卸卡样、side-section 升级成卡）。

### R6 主面板收口（用户四张反馈图）
- 竖签「它的内在」「身外之物」元素级移除（R5.4 曾补定位；用户判「字变得多余」）。i18n key 保留 + `#[allow(dead_code)]`（词表资产，audit 基线不动）。
- 主题钮 ☀/☾ 字形回退成齿轮 → 内联 SVG 太阳/月亮；窗控排 gap 2→4px。
- 「见证说明」witness-row 移除；「挂载」文字钮 → 圆环十字 SVG 图标钮。
- 宝石位置：经 32%/68% 对角（判不和谐）、50% 同高镜像（判违背设计哲学）后，用户指定**反对角线轴对称**：左结锚头像中轴线，右结以右上↔左下对角线为轴镜像（bottom 联动）。实测标定：**截图物理 px ≠ CSS 逻辑 px（K≈1.44）**，头像中轴物理 123 → 逻辑 `--gem-mid: 85px`（与真值回落值 84 惊人一致——真值本就锚在 84）。描边实测验证：左结中心 122.5（目标 123），右结距底 128.5。

### R7（先商后做，用户两条判决）
- **收纳钮骑缝条形化**（P3+边缘条）：▴ 小点从窗控排移除（语义错位 + 视觉太不明显）。新形态 = 骑 room-head 下缘虚线的 56×3 圆角条（96×14 命中区），hover 变橙加长带光晕，折叠态常亮橙提示。折叠后 room-head 收为胶囊行但 border-bottom 保留 → 钮随缝上移仍可展开（实测 fold/unfold 均正常）。onmousedown stop_propagation 防触发拖动。
- **宝石 B 方案：实心细柱+光晕，完全贴边**。真值径向渐变在 light 主题呈「硬线+离缘雾团」双重影像（根因：4px 透明命中边框 + padding-box 裁切），用户判不优雅。视觉体改 `::before`：3×64 圆角条贴死窗缘，box-shadow 光晕；左 accent-solid / 右 --node-right；is-open 淡化语义保留（.85/.55/.8）。
- **R7.3 完全贴边的真正根因：UA body margin 8px**。真值无 margin reset（浏览器 demo 被 containment 掩盖），port 三窗自始内缩 8px 逻辑。dbg3 描边实测：内容右缘 1108 vs 窗口 1118。`body { margin: 0 }` 后三窗内容真正铺满、宝石贴死窗缘。

## 功能实测
- 宝石：左右点击切换 inner/outer 显隐正常（用户手动确认；我脚本点击在窗缘被 tao hit_test 缩放边吃掉过——x<10 物理进 OS 缩放区，宝石命中区已扩至 24px 补偿）。
- □ 最大化/还原 ✅（1118×1035 ↔ 2578×1398）；─ 最小化/还原 ✅（IsIconic 验证）；✕ 关闭 ✅（进程整体干净退出，三窗同消）。
- 主题切换三窗同步 ✅（dark/light 双态截图验证宝石/缝条/图标）。
- 稳态 CPU 0%（前轮实测）。

## 门禁
- `cargo build -p northhing` exit 0，warnings 回到基线 lib 19 / bin 40（i18n key 加 allow(dead_code) 后无新增）。
- `cargo test -p northhing ui_dioxus` ok（TRUTH_CSS 字节守卫过）。
- `pnpm run i18n:audit` = 154 errors，与基线持平。

## 偏离披露
1. 宝石形态偏离真值「墙缝漏光而非漆条」注释 → 用户现场判决 B 方案（实心细柱+光晕），真值文件不改，偏离记录在此。
2. 竖签/见证说明/挂载文字 = 真值元素被用户判决移除；i18n key 保留未用。
3. `--gem-mid` 从真值回落 84 → 显式 85px（逻辑），物理/逻辑标定系数 K≈1.44 来自描边实测。
4. `flags.rs` 的 `DIOXUS_SHELL=true` 仍保持脏状态——GUI 复核迭代进行中，用户宣告完成后才 `git restore`。
5. 脚本点击工具在窗缘 8px 内不可靠（OS 缩放边）；窗内控件（窗控排/缝条/主题钮）脚本验证有效。

## R8（2026-08-14 用户判决「存在感太低 + 主框与模块颜色差别非常大」）
- **色差根因 = scrim 常开**。真值 scrim 是侧栏浮于 room 之上时的 22% 压暗（block-contract §2 规则 4 降级形态）；三窗独立后 room 不被遮挡，scrim 常暗只放大色温差。实测：light 主题 scrim 态 room 中心 RGB(204,206,204) vs 侧栏 RGB(254)——退役后 room 回到 #f6f8f9，与侧栏 #edf0f1/#ffffff 恢复真值邻近阶梯（dark：#101216 vs #161920）。app.rs 元素 + css.rs 规则同步移除（偏离 block-contract 降级形态，用户判决授权）。
- **存在感标定**：宝石细柱 3→4px、光晕加强（左 75%/右 70% color-mix）、开态淡化 .55→.72、常态 .85→.9、hover 4→5px；缝条 56×3→68×4、基色 --faint→--muted、hover 68→84px。
- **主题同步复核**：用户 image-4（room 亮/inner 暗）为瞬态——当前构建实测两次翻转三窗同步跟随（dark 三窗 RGB≈(12-22) / light 三窗 RGB≈(246-255)），同步链路（GlobalTheme watch → inner/outer use_future follower）完好。工具坑复记：冷启动后首点击可能仅激活窗口不投递，第二次才生效。

## R9（2026-08-14 用户判决「窗控排压虚线 + 呼吸点突兀」）
- **窗控对齐根因**：`.room-status` 行带无显式高度，absolute 定位的 `.room-controls` 按旧 top 值下坠，按钮压穿行带底缘虚线。修法：`.room-status { padding: 10px 18px }`（行带锁高 34 逻辑）+ `.room-controls { top: 3px }` → 钮区 3-31 在带内垂直居中，虚线完整让出。截图复核：虚线物理 y≈49（34×1.44），钮区 10-45 无交叉。
- **呼吸点退役，8s 呼吸钟移植到头像渐变**：app.rs 删 `state-dot` 元素（右上仅剩 ☀ ─ □ ✕）；css.rs 新增 `breath-avatar-fill`（::before 叠层渐变填充呼吸）、dark `breath-avatar-glow`（box-shadow 光晕呼吸）、light `breath-avatar-ring`（边框色在 --mind-line ↔ --accent-solid 间呼吸）；全部 `@media (prefers-reduced-motion: no-preference)` 门控（系统减动画时静止，随真值降级模式）；保留真值 `breath-avatar` 缩放动画，与渐变呼吸共用 8s 钟，律动一致。
- **验证**：r9a/r9b 两帧间隔 4s（= 8s 钟的谷/峰），头像区 102 个采样点差异 → 动画确实运行；暗色 = 内渐变+外光晕呼吸，亮色 = 边框色呼吸。
- 门禁：build exit 0、warnings 基线 lib19/bin40 守住、cargo test -p northhing ui_dioxus ok、i18n:audit 154 持平。

## 待办（下一轮候选，已向用户列出）
- 「见证者」消息右缘竖线悬孤（真值 border-right 设计），可收拢或去掉。
- 发送钮 ➤ 偏淡。
- 模块「独立隐蔽/拉开」交互入口未定义（真值本有 data-drag 设计）。
