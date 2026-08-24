// T1 Dioxus migration (2026-08-12) — embed the truth CSS verbatim.
//
// Brief §4.5 — "CSS 原样内联（禁翻译成 Rust 样式）". The CSS file at
// `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.css`
// (extracted from the truth HTML) must be embedded as a `&'static str`
// and injected via `document::Stylesheet` so the rendered pixels match
// the truth HTML byte-for-byte (modulo the three-window layout).
//
// Until the dedicated `.css` file is extracted, we fall back to the full
// `<style>` block from the truth HTML so the colors, keyframes, radial
// gradients and shadow tokens all line up. The conversion-annotations
// rules (color-mix 48, keyframes 21, radial-gradient 22, shadow 4式)
// are preserved verbatim below — do not edit unless the truth HTML
// itself changes.

/// CSS payload injected into every Dioxus window. The block is byte-
/// identical to the `<style>` section of `consult-room-main.html`
/// (lines 27..273 of the truth file at
/// `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.html`).
///
/// Brief §3.3 — original sample禁止任何修改/重排/翻译. We keep the
/// comment block (v4 变更 / trigger 变更) so reviewers can grep against
/// the source. If the truth HTML changes, this string must be updated
/// in lock-step — the regression test `assert_truth_css_byte_count`
/// guards against silent divergence.
pub const TRUTH_CSS: &str = include_str!("../../../../../docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.css");

/// R4 (2026-08-14): BOM strip at the injection boundary.
///
/// The truth css file starts with a UTF-8 BOM (EF BB BF). Injected
/// raw, the BOM glues onto the first selector and turns `:root` into
/// `\u{FEFF}:root`, which matches nothing — silently killing every
/// `:root` variable (`--mind-base` and everything derived from it:
/// `--mind-glow`, `--mind-intense`, `--mind-line`, `--accent-solid`,
/// `--frame`, fonts, `--breath`). That is why the avatar border /
/// radial-gradient fills / jewel glow all vanished while `font-size`
/// and `[data-theme]` variables kept working. `TRUTH_CSS` stays
/// byte-locked for the guard test; inject via this function instead.
pub fn truth_css() -> &'static str {
    TRUTH_CSS.strip_prefix('\u{FEFF}').unwrap_or(TRUTH_CSS)
}

/// R3' A+B+C 转写层覆盖样式（2026-08-14）。`TRUTH_CSS` 逐字节锁死
/// （`assert_truth_css_byte_count` 守卫必过，禁改真值 CSS 文件），任何
/// 转写层收口/覆盖规则只能落在此块，注入点：`windows.rs` inner/outer body
/// 与 `app.rs` room body——`TRUTH_CSS` 之后的第二个 `<style>` 块。
///
/// 选择器约定：
///   * `body[data-window="inner"]` / `body[data-window="outer"]` —— 只作用于
///     两个浮窗（room 主窗 body 无 `data-window` 属性，规则天然不落 room）；
///   * `#room-scrim` + `body[data-theme="..."]` —— 只作用于 room 主窗的
///     压暗层（scrim 是 S4 降级契约要求的转写层自绘，真值 CSS/HTML 无此规则，
///     见 block-contract §2 规则 4）。
pub const OVERLAY_CSS: &str = r#"
  /* ============ R3' 验收修复轮（2026-08-14）F1-F5 覆盖 ============
     真值 CSS 逐字节锁死（assert_truth_css_byte_count 必过）；转写层
     收口规则只能落在此块，注入点：三个窗的 body 第二个 <style> 块。
     选择器约定：body[data-window] 前缀 → inner/outer 浮窗；无前缀 + ID
     → room 主窗；#room-scrim 与宝石命中区是转写层自绘（真值无）。 */

  /* ---- F1. inner/outer 右缘裁切清零 ----
     浮窗视口（280/320）必命中真值媒体档：≤1180 档
     #mind{left:12px;width:240px} #work{right:12px;width:280px} 与 ≤940
     档 #mind,#work{position:static;width:100%;margin-top:12px}。
     left:12px/right:12px 位移把固定宽卡片右缘推出视口（右缘裁
     ~12-30px：fold 按钮、RAG 行、routing 状态、diff 计数），且 flex
     子项默认 min-width:auto 不收缩。收口链：位移归零 → 宽度固定为
     窗口逻辑宽 → 内部宽度链 max-width:100% + min-width:0 +
     flex-shrink 兜底。 */
  html, body { width: 100%; overflow-x: hidden; }
  body[data-window="inner"], body[data-window="outer"] { background: var(--bg0); overflow-y: auto; overflow-x: hidden; }
  body[data-window] aside { position: relative; left: auto; right: auto; margin-top: 0; width: 100%; max-width: 100%; min-width: 0; }
  body[data-window="inner"] aside#mind { width: 280px; max-width: 280px; }
  body[data-window="outer"] aside#work { width: 320px; max-width: 320px; }
  body[data-window] aside .mod,
  body[data-window] aside .card-body,
  body[data-window] aside .side-section,
  body[data-window] aside .station-head,
  body[data-window] aside .row,
  body[data-window] aside .side-title { max-width: 100%; min-width: 0; }
  body[data-window] aside .station-head, body[data-window] aside .row { overflow: hidden; }
  body[data-window] aside .station-head { display: flex; align-items: center; }
  body[data-window] aside .row > * { min-width: 0; }
  body[data-window] aside .fold-btn, body[data-window] aside .tag-x,
  body[data-window] aside .diff-add, body[data-window] aside .diff-del { flex-shrink: 0; white-space: nowrap; }
  body[data-window] aside .station-head .close-btn { margin-left: 6px; background: none; border: none; color: var(--faint); font-size: 12px; cursor: pointer; padding: 0 4px; line-height: 1; flex-shrink: 0; }
  body[data-window] aside .station-head .close-btn:hover { color: var(--accent-solid); }
  /* 终端井（C 单成果，保留）：禁断行 + 横向兜底。 */
  body[data-window="outer"] aside#work .term-well { white-space: pre; overflow-x: hidden; }

  /* ---- F1 顺带：浮窗高度撑满（窗高 820 而内容短 → 底部空区）。
     真值 #mind/#work 是三列布局侧栏（align-self 自然高）；窗内单列
     场景下撑满视口 + 内部滚动是合理转写：卡片均分高度、card-body 内
     滚；#work 的 side-sections 均分、term-well 自然贴底。 */
  body[data-window] aside { height: 100vh; max-height: none; display: flex; flex-direction: column; }
  body[data-window="inner"] aside#mind .mod { flex: 1 1 0; min-height: 0; display: flex; flex-direction: column; }
  body[data-window="inner"] aside#mind .card-body { flex: 1 1 0; min-height: 0; overflow-y: auto; }
  body[data-window="outer"] aside#work .side-section { flex: 0 1 auto; min-height: 0; overflow-y: auto; }
  body[data-window="outer"] aside#work .term-well { margin-top: 10px; flex: 0 0 auto; }

  /* ---- F2. room 填满窗高（deck 钉底）。
     真值骨架：body,html{100vh;overflow:hidden} → #engine{height:100vh;
     display:flex;padding:26px 48px} → #room{100% flex column} →
     .chat-flow{flex:1;overflow-y:auto} → deck 钉底、room-fog bottom:0。
     room 窗 880 视口命中真值 ≤940 档（#engine{display:block;height:auto;
     padding:12px} + #room{height:72vh}）→ 内容只占 ~66% 窗高。用更高
     特异性 body 前缀还原桌面骨架（覆盖层注入在媒体档之后，同特异性
     后定义胜出）。 */
  body #engine { display: flex; justify-content: center; align-items: stretch; gap: 16px; padding: 26px 48px; height: 100vh; }
  body #room-wrap { position: relative; width: min(780px, 100%); height: 100%; }
  body #room { width: 100%; height: 100%; }
  body, html { overflow: hidden; }
  body #containment, body .membrane-frame { display: block; }

  /* ---- F3. room 横向自适应。
     F2 覆盖层还原后真值桌面骨架即自适应：#engine padding 26px 48px +
     room-wrap min(780px,100%) + .rec max-width:88%——窗口缩放时 chat
     列随窗宽伸缩；880 窗宽 → 内容区 784px → room 780px，与真值 1920
     桌面同比例。≤940 档对 room 单列无其他破坏（#mind/#work 不在 room
     窗 DOM 中）。 */

  /* ---- F4. 宝石：左结可见 + 命中区 ≥20px + is-open 联动。
     左结锚头像中轴线（真值语义）。R5 铺满后布局上移，221px 失效。
     R6 标定：截图物理坐标 ≠ CSS 逻辑坐标（WebView2 渲染比例实测
     K≈1.44），头像中轴线物理 123px ÷ K → 逻辑 85px（与真值回落值
     84px 惊人一致——真值布局本就以 84 为中轴）。右结 bottom 联动
     calc(--gem-mid - 32px) 保持反对角线对称。命中区：透明 border
     扩到 20px 宽（background-clip 保视觉 12px），hover 26px 保
     视觉 18px。 */
  #room { --gem-mid: 85px; }
  #room .membrane-node { box-sizing: border-box; width: 20px; padding: 0; border-left: 4px solid transparent; border-right: 4px solid transparent; background-clip: padding-box; }
  #room .membrane-node:hover, #room .membrane-node:focus-visible { width: 26px; }

  /* ---- D（A+B+C 单成果）曾在此：room scrim 压暗层。R8 退役——
     三窗独立后 room 不再被侧栏遮挡，22% 常暗只会拉开主框与模块的
     色温（用户 2026-08-14 判决）。app.rs 元素已移除，规则清空。 */

  /* ============ R4 视觉收口轮（2026-08-14）W2-W5 覆盖 ============
     用户实测判决（逐字）：「原生与手绘窗口ui嵌套，右上角按钮视觉中心
     未对齐，宝石侧边栏把手完全没有，渐变色未显示，用了系统默认的
     滚动条；左右侧边栏：系统原生滚动条、分块未模块化是完全连接的，
     需要模块化拆解」。W1（frameless）落在 entry.rs/app.rs。 */

  /* ---- R4 W2. rc-btn 五钮中线对齐。
     真值 L74 已是 28x28 flex 居中；port 层不齐的字形根因是 glyph 在
     字体回退链里的基线差异（▴/☀ 与 ─/□/✕ 落不同字体）。统一
     line-height 压掉行盒高度差，让 flex 居中决定唯一中线。 */
  #room .room-controls { align-items: center; }
  #room .room-controls .rc-btn { line-height: 1; }

  /* ---- R4 W3. 宝石把手可见（用户判决：常态必须可辨）。
     真值语义保留：关态=实心亮段唤起件、开态=淡化（L184/L193）。
     不可见根因：inner/outer 常开 → 两结恒 is-open opacity .22 ×
     radial-gradient 78% 处已透明 × fix2 background-clip 收窄，
     三因子叠成隐形。数值标定（复核可调）：常态 .55→.85，
     is-open .22→.45（淡化但不隐形），is-open:hover .6→.8。 */
  #room .membrane-node { opacity: .85; }
  #room .membrane-node.is-open { opacity: .45; }
  #room .membrane-node.is-open:hover, #room .membrane-node.is-open:focus-visible { opacity: .8; }

  /* ---- R4 W4. 自定义细滚动条（真值零 scrollbar 规则 = 转写层新增，
     用户判决授权）。三窗统一：10px 槽、透明轨道、3px 透明边让滑块
     视觉 4px、禁掉 classic 上下箭头按钮（button/corner）。
     注：WebView2 无 overlay 滚动条开关，::-webkit-scrollbar 定制即
     支持路径。 */
  ::-webkit-scrollbar { width: 10px; height: 10px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--line); border-radius: 5px; border: 3px solid transparent; background-clip: padding-box; }
  ::-webkit-scrollbar-thumb:hover { background: var(--faint); }
  ::-webkit-scrollbar-button { display: none; width: 0; height: 0; }
  ::-webkit-scrollbar-corner { background: transparent; }

  /* ---- R4 W5. 侧栏模块化拆解（用户判决：分块完全连接 → 模块边界）。
     现状：aside 贴满窗口四缘（width 固定=窗口逻辑宽、100vh 撑满），
     真值 .mod 卡片样式（L169 border/radius/bg2/shadow）被贴缘压成
     隐形。拆解：body 让出 10px 内边距 → 卡片浮于窗口中，边框/圆角/
     分层/投影全部可辨；aside 宽高改为填满内容盒（box-sizing 全局
     border-box 已由真值 L36 保证）。卡片间 gap:14px（真值 L170）
     与 flex:1 均分（F1 顺带成果）保留不动。 */
  body[data-window] { padding: 10px; }
  body[data-window] aside { height: 100%; }
  body[data-window="inner"] aside#mind { width: 100%; max-width: 100%; }
  body[data-window="outer"] aside#work { width: 100%; max-width: 100%; }

  /* ---- R4 W3 续. chronicle-bar 静态渐变。
     真值 L100 只定义尺寸（200x4, opacity .7），背景由真值 JS L569
     运行时生成 linear-gradient（底色→代表色 stops）；port 无 JS →
     无背景 → 不可见。转写层静态近似：bg3（历史沉积端）→ accent-solid
     （现在端），与真值「底色⇐现在」语义同向。 */
  #room .chronicle-bar { background: linear-gradient(90deg, var(--bg3) 0%, var(--accent-solid) 100%); }

  /* ---- R4 W2 续. ▴ 收纳钮字形校正：元素已在 R7.1 移除（骑缝条
     取代小点），规则随元素退役。 */

  /* ---- R4 W2 续. room-status 避让 room-controls：frameless 后
     room-controls（absolute top:8px right:10px，五钮宽 148px）压盖
     room-status 行右端的 state-dot（真值布局两者即重叠，frameless
     后窗控是唯一交互钮，避让）。160px = 五钮 148 + 余量。 */
  #room .room-status { padding-right: 160px; }

  /* ============ R5 去嵌套（2026-08-14 用户判决，目标图 image-2/4/5/8）
     「所有除了窗口的黑色部分都是要去掉的」：
     - room：卡片外的 bg0 黑边（#engine padding）、containment/membrane-
       frame 橙框、卡片 border/radius/shadow 全部去掉 —— room 内容即
       窗口，铺满 100%。
     - 侧栏：推翻 R4 W5 的 body padding:10px（方向反了——黑 padding
       加重嵌套感）；卡片铺满窗宽，模块感由卡间 gap 表达。
     - outer：aside 从单卡降为透明容器，三 section + term-well 各自
       独立成卡（image-8 形态）。 */

  /* ---- R5.1 room 铺满 ---- */
  body #engine { padding: 0; gap: 0; }
  body #room-wrap { width: 100%; }
  body #room { border: none; border-radius: 0; box-shadow: none; }
  body #containment, body .membrane-frame { display: none; }

  /* ---- R5.2 侧栏铺满（推翻 R4 W5 padding） ---- */
  body[data-window] { padding: 0; }

  /* ---- R5.3 outer 模块卡片化（image-6 → image-8）。
      port DOM 里 aside#work 带 class="mod"（真值单卡设计）；用户判决
      改为四独立卡。aside 卸卡样（真值 #work 无重置规则，此处补齐），
      side-section 升级为卡（真值 .mod 五件套：border/bevel/radius/
      bg2/shadow），卡间距 margin-bottom 14px（对齐真值 #mind gap）。
      W2.7：推翻 flex:1 均分——卡 hug 内容，空盒子退役。 */
  body[data-window="outer"] aside#work { background: transparent; border: none; border-radius: 0; box-shadow: none; }
  body[data-window="outer"] aside#work .side-section {
    border: 1px solid var(--line); border-top-color: var(--bevel);
    border-radius: 4px; background: var(--bg2); box-shadow: var(--shadow);
    margin-bottom: 14px;
    flex: 0 1 auto;
    min-height: 0;
  }
  body[data-window="outer"] aside#work .side-section:last-child { border-bottom: 1px solid var(--line); }
  body[data-window="outer"] aside#work .term-well { margin: 0; flex: 0 0 auto; }

  /* ============ R6 主面板收口（2026-08-14 用户判决，四张反馈图）
     用户逐字：「左右侧光点的位置不和谐」「现在的字变得多余了【它的
     内在】【身外之物】」「右上角的图标可能需要一些调整」「【见证说明】
     的字也去掉」「【挂载】直接换成普通的附件上传按钮」。 */

  /* ---- R6.1 宝石位置：右上↔左下反对角线轴对称（用户判决，恢复
     真值设计哲学）。
     真值原构：左结=头像中轴线（--gem-mid），右结 bottom:230px ≈
     左结距顶的镜像——两结到对角线两端点（左下角/右上角）等距，
     即关于反对角线对称。R5 布局上移后重测 --gem-mid=123px；右结
     改为 bottom: calc(--gem-mid - 32px)（结高 64，中心距底 = 123 =
     左结中心距顶），窗高变化时对称性自适应保持。
     命中区扩宽 20→24px（background-clip 不变 = 视觉 12px 不动）：
     frameless 下左/右 8px 被 tao hit_test 划入缩放边，实测 x=8 点击
     被 OS 吃掉（窗口被改宽 205px），x≥10 才到 webview；扩宽后可用
     命中带 9→24px。 */
  #room .membrane-node { width: 24px; }
  #room .membrane-node:hover, #room .membrane-node:focus-visible { width: 28px; }
  #room .membrane-node.left { top: var(--gem-mid); margin-top: -32px; transform: none; }
  #room .membrane-node.right { top: auto; bottom: calc(var(--gem-mid) - 32px); transform: none; }

  /* ---- R6.1b 右结亮度标定：开态统一提至 .55（is-open 淡化语义
     保留）。渐变加宽尝试已被 R7.2 形态重做取代（见下）。 */
  #room .membrane-node.is-open { opacity: .55; }

  /* ---- R6.2 竖签移除：app.rs 已删 vlabel 元素，此处规则同步删除。
     （R5.4 曾补定位；用户判决「字变得多余」→ 元素级移除。） */

  /* ---- R6.3 右上角图标：主题钮字形回退成齿轮/雪花（☀ 在字体链里
     落进 dingbat），改内联 SVG（app.rs）；此处统一 SVG 继承色与排布：
     gap 2→4px（五钮 148px → 160px，room-status 避让 160px 仍够）。 */
  #room .room-controls { gap: 4px; }
  #room .room-controls .rc-btn svg { display: block; }

  /* ---- R6.4 挂载钮图标化：文字「挂载」→ 圆环十字 SVG（用户给参照
     image-7 形态）。尺寸对齐 input-row 行高，色用真值 attach 色
     --mind-line，hover 提亮。 */
  #room .room-input .attach { display: inline-flex; align-items: center; justify-content: center; width: 24px; height: 24px; }
  #room .room-input .attach:hover { color: var(--text); }
  #room .room-input .attach svg { display: block; }

  /* ---- R6.5 见证说明移除：app.rs 已删 witness-row 元素。 */

  /* ============ R7（2026-08-14 用户判决，先商后做） ============ */

  /* ---- R7.1 收纳钮骑缝条形化（P3 + 边缘条，用户判决）。
     ▴ 小点从窗控排退役（语义：它控制头块而非窗口；且视觉太不明显）。
     新形态：骑 room-head 下缘虚线的 56x3 圆角条，96x14 命中区；
     hover 变橙加长带光晕；折叠态常亮橙 =「有内容被收起」提示。
     room-head 有 position:relative（真值 L93），bottom:0 +
     translate(-50%,50%) 正骑缝；折叠后 border-bottom 保留 →
     钮随缝上移仍在原位。 */
  #room .head-seam-fold { position: absolute; left: 50%; bottom: 0; transform: translate(-50%, 50%); width: 96px; height: 14px; padding: 0; background: none; border: none; cursor: pointer; z-index: 32; -webkit-app-region: no-drag; display: flex; align-items: center; justify-content: center; }
  #room .head-seam-fold .seam-bar { display: block; width: 56px; height: 3px; border-radius: 2px; background: var(--faint); transition: width .2s, background .2s, box-shadow .25s; }
  #room .head-seam-fold:hover .seam-bar { width: 72px; background: var(--accent-solid); box-shadow: 0 0 8px color-mix(in srgb, var(--accent-solid) 55%, transparent); }
  #room .head-seam-fold .seam-bar.folded { background: var(--accent-solid); }
  /* 窗控排剩四钮：room-status 避让从 160 收紧（4x28 + 3x4 + 10）。 */
  #room .room-status { padding-right: 136px; }

  /* ---- R7.2 宝石 B 方案（用户判决）：实心细柱 + 光晕，完全贴边。
     真值径向「漏光」渐变卸任：light 主题下呈现为硬线 + 离缘雾团的
     双重影像（根因 = 4px 透明命中边框 + padding-box 裁切在 4px 处
     硬起渐变），用户判不优雅。视觉体改走 ::before：3x64 圆角条
     贴死窗缘（x=0，完全贴边），box-shadow 单色光晕；左结
     accent-solid、右结 --node-right；hover 条加粗光晕增强；
     is-open 淡化由元素 opacity 继承到伪元素（.85/.55/.8 不变）。
     命中区仍是元素本体 24px（x≥10 可点，避开 OS 缩放边）。 */
  #room .membrane-node { background: none; }
  /* 4px 透明命中边框把 padding-box 内推 → ::before 以 -4px 抵消，
     视觉条真正落在 x=0 / 右缘（完全贴边，用户判决）。 */
  #room .membrane-node::before { content: ""; position: absolute; top: 0; bottom: 0; width: 3px; border-radius: 2px; transition: width .2s, box-shadow .25s; }
  #room .membrane-node.left::before { left: -4px; background: var(--accent-solid); box-shadow: 0 0 10px 1px color-mix(in srgb, var(--accent-solid) 60%, transparent); }
  #room .membrane-node.right::before { right: -4px; background: var(--node-right); box-shadow: 0 0 12px 1px color-mix(in srgb, var(--node-right) 55%, transparent); }
  #room .membrane-node:hover::before, #room .membrane-node:focus-visible::before { width: 4px; }
  #room .membrane-node.left:hover::before { box-shadow: 0 0 14px 2px color-mix(in srgb, var(--accent-solid) 75%, transparent); }
  #room .membrane-node.right:hover::before { box-shadow: 0 0 16px 2px color-mix(in srgb, var(--node-right) 70%, transparent); }

  /* ============ R8（2026-08-14 用户判决：存在感太低 + 色差） ============ */

  /* ---- R8.1 存在感标定：细柱 3→4px、光晕增强、开态淡化 .55→.72
     （淡化语义保留但不再低存在）、常态 .85→.9、hover 条 4→5px。
     缝条 56x3 → 68x4，基色 --faint → --muted（亮一档）。 */
  #room .membrane-node { opacity: .9; }
  #room .membrane-node.is-open { opacity: .72; }
  #room .membrane-node.is-open:hover, #room .membrane-node.is-open:focus-visible { opacity: .95; }
  #room .membrane-node::before { width: 4px; }
  #room .membrane-node.left::before { box-shadow: 0 0 14px 2px color-mix(in srgb, var(--accent-solid) 75%, transparent); }
  #room .membrane-node.right::before { box-shadow: 0 0 16px 2px color-mix(in srgb, var(--node-right) 70%, transparent); }
  #room .membrane-node:hover::before, #room .membrane-node:focus-visible::before { width: 5px; }
  #room .membrane-node.left:hover::before { box-shadow: 0 0 18px 3px color-mix(in srgb, var(--accent-solid) 85%, transparent); }
  #room .membrane-node.right:hover::before { box-shadow: 0 0 20px 3px color-mix(in srgb, var(--node-right) 80%, transparent); }
  #room .head-seam-fold .seam-bar { width: 68px; height: 4px; background: var(--muted); }
  #room .head-seam-fold:hover .seam-bar { width: 84px; }

  /* ---- R8.2 色差：scrim 退役（app.rs 元素 + 上方 D 段规则已清）。
     退役后 dark：room bg1 #101216 vs 侧栏卡 bg2 #161920；light：
     room #f6f8f9 vs 侧栏体 #edf0f1/卡 #ffffff——回到真值设计的
     邻近阶梯，不再有 22% 常暗放大差。 */

  /* ---- R7.3 完全贴边根因修复：UA body margin 8px。
     真值 CSS 无 margin reset（浏览器 demo 里被 containment 内缩掩盖），
     port 三窗自始带 8px 逻辑 margin → 内容框四缘内缩 ~11px 物理，
     宝石/侧栏全部贴不到窗缘（dbg3 盒模型实测：内容右缘 1108 vs
     窗口 1118，左缘同构）。body margin 归零，三窗内容真正铺满
     （R5 铺满判决的补完）。 */
  body { margin: 0; }

  /* ============ R9（2026-08-14 用户判决：窗控压线 + 呼吸点） ============ */

  /* ---- R9.1 窗控排与虚线分隔线对齐（用户实测：hover 背景压线）。
     根因：状态行带高 30px（padding 8+内容 14+8），虚线在 y≈30；
     真值 room-controls top:8 + 28px 钮跨到 y36 必然压线。修法：
     行带 padding 8→10（带高 34）+ 控件 top:3 → 钮 3-31 垂直居中
     于带内，上下各让 3px，不再触线。 */
  #room .room-status { padding: 10px 18px; }
  #room .room-controls { top: 3px; }

  /* ---- R9.2 呼吸点退役 → 头像渐变呼吸（用户判决：「直接让头像的
     渐变色呼吸」）。state-dot 元素已从 app.rs 移除。8s 呼吸时钟
     （--breath，与真值同一钟）改挂头像三件：内部径向渐变填充
     （::before 叠层 tween opacity）、dark 态外光晕 box-shadow、
     light 态边框色（mind-line ↔ accent-solid）。真值原有
     breath-avatar 缩放保留（并入 animation 列表）。reduced-motion
     降级跟随真值模式（no-preference 才挂动画）。 */
  #room .agent-avatar { position: relative; }
  @media (prefers-reduced-motion: no-preference) {
    #room .agent-avatar::before { content: ""; position: absolute; inset: 0; pointer-events: none; background: radial-gradient(circle at 38% 32%, var(--mind-intense) 0%, transparent 74%); animation: breath-avatar-fill var(--breath) ease-in-out infinite; }
    #room .agent-avatar { animation: breath-avatar var(--breath) ease-in-out infinite, breath-avatar-glow var(--breath) ease-in-out infinite; }
    body[data-theme="light"] #room .agent-avatar { animation: breath-avatar var(--breath) ease-in-out infinite, breath-avatar-ring var(--breath) ease-in-out infinite; }
  }
  @keyframes breath-avatar-fill { 0%, 100% { opacity: .2; } 50% { opacity: .65; } }
  @keyframes breath-avatar-glow { 0%, 100% { box-shadow: 0 0 14px color-mix(in srgb, var(--mind-glow) 55%, transparent); } 50% { box-shadow: 0 0 30px var(--mind-glow); } }
  @keyframes breath-avatar-ring { 0%, 100% { border-color: var(--mind-line); } 50% { border-color: var(--accent-solid); } }

  /* ============ W2.7 流体卡片打磨（2026-08-24 用户定案）============
     左列五卡满高同窗 + 沉积|设施分组缝；右列三卡+终端填满窗底。
     左右每卡支持点击标题折叠到只剩标题；展开卡流体拉伸吃空区。 */

  /* 窗体永不滚动（覆盖 F1 的 overflow-y:auto）。 */
  body[data-window="inner"] { overflow: hidden; }

  /* aside 卸卡样 + 填满内容盒：宽度链沿用 F1/R4 收口（100%），高度 100% */
  body[data-window="inner"] aside#mind { width: 100%; max-width: 100%; height: 100%; display: flex; flex-direction: column; gap: 10px; background: transparent; border: none; border-radius: 0; box-shadow: none; }

  /* 独立拖拽条：左垫 12px 呼应卡标题节奏 */
  body[data-window="inner"] aside#mind > .w2-head { flex: 0 0 auto; border-bottom: 1px dashed var(--line); padding: 6px 6px 6px 12px; }
  body[data-window="inner"] aside#mind > .w2-head .fold-btn { margin-left: auto; background: none; border: none; border-radius: 0; color: var(--faint); font-size: 10px; padding: 2px 6px; cursor: pointer; }
  body[data-window="inner"] aside#mind > .w2-head .fold-btn:hover { color: var(--accent-solid); border-color: transparent; }

  /* work chrome 与左列对齐：左垫 12px + 控件右缘成组 */
  body[data-window="outer"] aside#work > .w2-head { display: flex; align-items: center; padding: 6px 6px 6px 12px; border-bottom: 1px dashed var(--line); }
  body[data-window="outer"] aside#work > .w2-head .fold-btn { margin-left: auto; background: none; border: none; border-radius: 0; color: var(--faint); font-size: 10px; padding: 2px 6px; cursor: pointer; }
  body[data-window="outer"] aside#work > .w2-head .fold-btn:hover { color: var(--accent-solid); }

  /* 标题行可点击折叠 + 呼吸对齐（水平 18px） */
  body[data-window] aside .side-title { display: flex; align-items: center; cursor: pointer; user-select: none; }
  body[data-window] aside .side-title:hover { color: var(--text); }
  body[data-window] aside .side-title .fold-caret { margin-left: auto; font-size: 9px; color: var(--faint); font-family: var(--font-mono); flex-shrink: 0; padding-left: 6px; transition: color .15s; }
  body[data-window] aside .side-title:hover .fold-caret { color: var(--accent-solid); }

  /* 左列卡尺寸策略：展开卡拉伸吃剩余高度（1 1 auto），折叠卡收缩（0 0 auto） */
  body[data-window="inner"] aside#mind > .mod { flex: 1 1 auto; min-height: 0; display: flex; flex-direction: column; }
  body[data-window="inner"] aside#mind > .mod.is-folded { flex: 0 0 auto !important; min-height: 0; }
  body[data-window="inner"] aside#mind > .mod.is-folded > :not(.side-title) { display: none !important; }

  /* 标题钉住（w2-pin）+ 列表区内滚（w2-scroll）+ 卡尾钉住（w2-foot）+ 水平 18px 内边距 */
  body[data-window="inner"] aside#mind > .mod .w2-pin { flex: 0 0 auto; margin: 0; padding: 12px 18px 0; }
  body[data-window="inner"] aside#mind > .mod.is-folded .w2-pin { padding: 10px 18px; }
  body[data-window="inner"] aside#mind > .mod .w2-scroll { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 8px 18px; mask-image: linear-gradient(to bottom, #000 calc(100% - 8px), transparent 100%); -webkit-mask-image: linear-gradient(to bottom, #000 calc(100% - 8px), transparent 100%); }
  body[data-window="inner"] aside#mind > .mod .w2-foot { flex: 0 0 auto; margin: 0; padding: 8px 18px; border-top: 1px solid var(--line); }

  /* 左列分组缝（沉积 | 设施） */
  body[data-window="inner"] aside#mind > .w2-group-seam { flex: 0 0 auto; display: flex; align-items: center; gap: 8px; padding: 2px 12px; margin: 0; color: var(--faint); font-family: var(--font-mono); font-size: 9px; letter-spacing: .1em; }
  body[data-window="inner"] aside#mind > .w2-group-seam::before,
  body[data-window="inner"] aside#mind > .w2-group-seam::after { content: ""; flex: 1 1 auto; border-bottom: 1px dashed var(--line); }

  /* 右列卡片：折叠收缩 + 水平 18px 内边距 + 终端填底吃满高度 */
  body[data-window="outer"] aside#work { background: transparent; border: none; border-radius: 0; box-shadow: none; height: 100%; display: flex; flex-direction: column; }
  body[data-window="outer"] aside#work .side-section { border: 1px solid var(--line); border-top-color: var(--bevel); border-radius: 4px; background: var(--bg2); box-shadow: var(--shadow); margin-bottom: 12px; flex: 0 1 auto; min-height: 0; padding: 12px 18px; }
  body[data-window="outer"] aside#work .side-section.is-folded { flex: 0 0 auto !important; margin-bottom: 12px; padding: 10px 18px; }
  body[data-window="outer"] aside#work .side-section.is-folded > :not(.side-title) { display: none !important; }
  body[data-window="outer"] aside#work .side-section.is-folded .side-title { margin-bottom: 0; }
  body[data-window="outer"] aside#work .term-well { margin: 0; flex: 1 1 auto; min-height: 72px; overflow-y: auto; padding: 10px 18px; }

  /* 沉积skill 候选状态词（右缘淡字）。 */
  body[data-window="inner"] aside#mind .w2-stat { margin-left: auto; color: var(--faint); font-family: var(--font-mono); font-size: 9px; flex-shrink: 0; white-space: nowrap; }

  /* RUNTIME 卡 token 消耗行 */
  body[data-window="inner"] aside#mind .w2-token { cursor: default; }
  body[data-window="inner"] aside#mind .w2-token .w2-token-label { color: var(--faint); font-family: var(--font-mono); font-size: 9px; letter-spacing: .08em; }
  body[data-window="inner"] aside#mind .w2-token .w2-token-value { color: var(--accent-solid); font-family: var(--font-mono); font-size: 11px; }
  body[data-window="inner"] aside#mind .w2-token .w2-token-clear { margin-left: auto; background: none; border: 1px solid var(--line); border-radius: 3px; color: var(--muted); font-family: var(--font-mono); font-size: 9px; line-height: 1.4; padding: 1px 8px; cursor: pointer; flex-shrink: 0; transition: color .15s, border-color .15s; }
  body[data-window="inner"] aside#mind .w2-token .w2-token-clear:hover { color: var(--accent-solid); border-color: var(--accent-solid); }
  body[data-window="inner"] aside#mind .w2-token .w2-token-clear:disabled { opacity: .4; cursor: default; color: var(--faint); border-color: var(--line); }

  /* 列表呼吸感与卡尾堆叠 */
  body[data-window="inner"] aside#mind .w2-scroll .row { padding: 4px 0; }
  body[data-window="inner"] aside#mind .w2-foot .seg-bar { margin: 0 0 6px; }
  body[data-window="inner"] aside#mind .w2-foot .seg-note { margin: 0; }

  /* em 语义着色 */
  body[data-window="inner"] aside#mind .w2c-sediment .side-title em,
  body[data-window="inner"] aside#mind .w2c-rag .side-title em { color: var(--mind-line); }
  body[data-window="inner"] aside#mind .w2c-skill .side-title em { color: var(--node-right); }
  body[data-window="inner"] aside#mind .w2c-runtime .side-title em { color: var(--accent-solid); }
  body[data-window="inner"] aside#mind .w2c-axioms .side-title em { color: var(--ok); }

  /* ============ Room 状态行档案与走廊文字链 ============ */
  #room .room-status .status-nav-link, #room .room-status #nav-archive, #room .room-status #nav-space {
    background: none; border: none; padding: 0 4px; margin-left: 8px; font-family: var(--font-mono);
    font-size: 10px; color: var(--muted); letter-spacing: 1px; cursor: pointer;
    text-decoration: underline; text-underline-offset: 3px; transition: color 0.15s; -webkit-app-region: no-drag;
  }
  #room .room-status #nav-archive:hover, #room .room-status #nav-space:hover { color: var(--accent-solid); }

  /* ============ E1 Archive 档案馆 (2026-08-24) ============ */
  body[data-window="archive"] {
    --mind-base: #3F837B; --aura-x: 50%; --aura-y: 200px;
    --mind-glow: color-mix(in srgb, var(--mind-base) 15%, transparent);
    --mind-intense: color-mix(in srgb, var(--mind-base) 40%, transparent);
    --mind-line: color-mix(in srgb, var(--mind-base) 70%, #ffffff);
    --accent-solid: var(--mind-base); --frame: color-mix(in srgb, var(--mind-base) 55%, transparent);
    overflow: hidden; background: var(--bg0); display: flex; flex-direction: column; height: 100vh; padding: 0; margin: 0;
  }
  body[data-window="archive"][data-theme="light"] {
    --mind-line: color-mix(in srgb, var(--mind-base) 76%, #101416);
    --accent-solid: color-mix(in srgb, var(--mind-base) 84%, #241108);
    --frame: var(--mind-line);
  }
  body[data-window="archive"] .archive-chrome {
    flex: 0 0 auto; display: flex; align-items: center; padding: 6px 12px 6px 18px;
    border-bottom: 1px dashed var(--line); background: var(--bg1); -webkit-app-region: drag; user-select: none;
  }
  body[data-window="archive"] .archive-chrome .archive-chrome-title { font-family: var(--font-agent); font-size: 13px; font-style: italic; color: var(--text); }
  body[data-window="archive"] .archive-chrome .archive-chrome-actions { margin-left: auto; display: flex; align-items: center; gap: 4px; -webkit-app-region: no-drag; }
  body[data-window="archive"] .archive-chrome .fold-btn { background: none; border: none; color: var(--faint); font-size: 10px; padding: 2px 6px; cursor: pointer; border-radius: 3px; }
  body[data-window="archive"] .archive-chrome .fold-btn:hover { color: var(--accent-solid); }
  body[data-window="archive"] .archive-chrome .theme-btn { width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; background: none; border: none; color: var(--muted); cursor: pointer; border-radius: 3px; padding: 0; }
  body[data-window="archive"] .archive-chrome .theme-btn:hover { background: var(--bg2); color: var(--text); }
  body[data-window="archive"] .archive-chrome .close-btn { width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; background: none; border: none; color: var(--faint); font-size: 12px; cursor: pointer; border-radius: 3px; padding: 0; }
  body[data-window="archive"] .archive-chrome .close-btn:hover { background: var(--danger); color: #fff; }

  body[data-window="archive"] .archive-engine { flex: 1 1 auto; display: flex; flex-direction: row; height: calc(100vh - 36px); overflow: hidden; }

  body[data-window="archive"] aside#archive-mind {
    width: 240px; flex: 0 0 240px; height: 100%; display: flex; flex-direction: column; gap: 10px;
    padding: 10px; border-right: 1px dashed var(--line); background: var(--bg0); overflow-y: auto; box-sizing: border-box;
  }
  body[data-window="archive"] aside#archive-mind > .mod {
    border: 1px solid var(--line); border-top-color: var(--bevel); border-radius: 4px; background: var(--bg2);
    box-shadow: var(--shadow); flex: 1 1 auto; min-height: 0; display: flex; flex-direction: column;
  }
  body[data-window="archive"] aside#archive-mind > .mod.is-folded { flex: 0 0 auto !important; min-height: 0; }
  body[data-window="archive"] aside#archive-mind > .mod.is-folded > :not(.side-title) { display: none !important; }
  body[data-window="archive"] aside#archive-mind > .mod .w2-pin { flex: 0 0 auto; margin: 0; padding: 12px 18px 0; }
  body[data-window="archive"] aside#archive-mind > .mod.is-folded .w2-pin { padding: 10px 18px; }
  body[data-window="archive"] aside#archive-mind > .mod .w2-scroll { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 8px 18px 12px; }

  body[data-window="archive"] aside#archive-mind .side-title { font-family: var(--font-mono); font-size: 9px; color: var(--muted); letter-spacing: 0.08em; display: flex; align-items: center; cursor: pointer; user-select: none; }
  body[data-window="archive"] aside#archive-mind .side-title:hover { color: var(--text); }
  body[data-window="archive"] aside#archive-mind .side-title em { font-style: normal; color: var(--mind-line); }
  body[data-window="archive"] aside#archive-mind .side-title .fold-caret { margin-left: auto; font-size: 9px; color: var(--faint); font-family: var(--font-mono); padding-left: 6px; }
  body[data-window="archive"] aside#archive-mind .row { display: flex; align-items: center; gap: 7px; padding: 4px 0; font-size: 11px; color: var(--muted); cursor: pointer; transition: color 0.15s; }
  body[data-window="archive"] aside#archive-mind .row:hover { color: var(--text); }
  body[data-window="archive"] aside#archive-mind .row.active { color: var(--text); }
  body[data-window="archive"] aside#archive-mind .dot-radio { width: 7px; height: 7px; border-radius: 50%; border: 1px solid var(--muted); flex-shrink: 0; }
  body[data-window="archive"] aside#archive-mind .row.active .dot-radio { border-color: var(--accent-solid); background: var(--accent-solid); }

  body[data-window="archive"] .depth-bar { display: flex; gap: 2px; margin-top: 10px; height: 18px; align-items: flex-end; }
  body[data-window="archive"] .depth-bar .depth-seg { flex: 1; background: var(--mind-base); transition: opacity 0.3s; }
  body[data-window="archive"] .depth-bar .depth-seg:nth-child(1) { opacity: .95; height: 100%; }
  body[data-window="archive"] .depth-bar .depth-seg:nth-child(2) { opacity: .80; height: 86%; }
  body[data-window="archive"] .depth-bar .depth-seg:nth-child(3) { opacity: .64; height: 72%; }
  body[data-window="archive"] .depth-bar .depth-seg:nth-child(4) { opacity: .48; height: 58%; }
  body[data-window="archive"] .depth-bar .depth-seg:nth-child(5) { opacity: .34; height: 44%; }
  body[data-window="archive"] .depth-bar .depth-seg:nth-child(6) { opacity: .22; height: 30%; }
  body[data-window="archive"] .depth-bar .depth-seg:nth-child(7) { opacity: .12; height: 18%; }
  body[data-window="archive"] .depth-note { font-family: var(--font-mono); font-size: 9px; color: var(--muted); margin-top: 8px; line-height: 1.6; }

  body[data-window="archive"] section#archive-room { flex: 1 1 auto; min-width: 0; height: 100%; display: flex; flex-direction: column; background: var(--bg1); position: relative; overflow: hidden; }
  body[data-window="archive"] section#archive-room .room-status { flex: 0 0 auto; display: flex; align-items: center; gap: 12px; padding: 8px 18px; border-bottom: 1px dashed var(--line); font-family: var(--font-mono); font-size: 10px; color: var(--muted); letter-spacing: 1px; -webkit-app-region: drag; }
  body[data-window="archive"] section#archive-room .room-status .sp { flex: 1; }
  body[data-window="archive"] section#archive-room .state-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent-solid); animation: breath-dot var(--breath) ease-in-out infinite; }

  body[data-window="archive"] section#archive-room .room-head { flex: 0 0 auto; position: relative; display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 20px 18px 14px; border-bottom: 1px dashed var(--line); background: radial-gradient(280px 140px at 50% 0%, var(--mind-glow), transparent 78%); transition: padding 0.35s cubic-bezier(.22,1,.36,1); -webkit-app-region: drag; }
  body[data-window="archive"] section#archive-room .room-head .head-fold { position: absolute; right: 10px; top: 8px; background: none; border: 1px solid var(--line); color: var(--muted); font-size: 9px; padding: 3px 7px; cursor: pointer; border-radius: 3px; -webkit-app-region: no-drag; }
  body[data-window="archive"] section#archive-room .room-head .head-fold:hover { color: var(--mind-line); border-color: var(--accent-solid); }
  body[data-window="archive"] section#archive-room .room-head.folded { flex-direction: row; justify-content: center; gap: 12px; padding: 8px 18px; background: linear-gradient(90deg, transparent, var(--mind-glow), transparent); }
  body[data-window="archive"] section#archive-room .room-head.folded .depth-marker { width: 26px; height: 26px; font-size: 12px; box-shadow: 0 0 12px var(--mind-glow); }
  body[data-window="archive"] section#archive-room .room-head.folded .name-line { font-size: 13px; }
  body[data-window="archive"] section#archive-room .room-head.folded .state { display: none; }
  body[data-window="archive"] section#archive-room .depth-marker { width: 48px; height: 48px; border-radius: 0; border: 1px solid var(--accent-solid); background: radial-gradient(circle at 38% 32%, var(--mind-intense) 0%, transparent 74%); display: flex; align-items: center; justify-content: center; font-family: var(--font-agent); font-style: italic; font-size: 20px; color: var(--mind-line); box-shadow: 0 0 24px var(--mind-glow); animation: breath-avatar var(--breath) ease-in-out infinite; -webkit-app-region: no-drag; }
  body[data-window="archive"][data-theme="light"] section#archive-room .depth-marker { box-shadow: none; border: 2px solid var(--mind-line); background: color-mix(in srgb, var(--mind-base) 12%, #ffffff); }
  body[data-window="archive"] section#archive-room .room-head .name-line { font-family: var(--font-agent); font-style: italic; font-size: 16px; color: var(--text); }
  body[data-window="archive"] section#archive-room .room-head .state { font-family: var(--font-mono); font-size: 10px; color: var(--mind-line); letter-spacing: 0.08em; background: var(--mind-intense); padding: 3px 9px; border-radius: 2px; }

  body[data-window="archive"] section#archive-room .strata-flow { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 10px 18px 16px; display: flex; flex-direction: column; }
  body[data-window="archive"] .stratum { padding: 14px 16px; border-top: 1px solid var(--line); transition: background 0.2s, opacity 0.2s; cursor: pointer; position: relative; }
  body[data-window="archive"] .stratum:first-child { border-top: none; }
  body[data-window="archive"] .stratum:hover { background: var(--bg2); }
  body[data-window="archive"] .stratum.active { background: var(--bg2); box-shadow: inset 3px 0 0 var(--mind-base); }
  body[data-window="archive"] .stratum-head { display: flex; align-items: baseline; justify-content: space-between; gap: 12px; margin-bottom: 6px; font-family: var(--font-mono); font-size: 10px; color: var(--muted); letter-spacing: 0.08em; }
  body[data-window="archive"] .stratum-no { color: var(--faint); }
  body[data-window="archive"] .stratum-time { color: var(--faint); font-size: 9px; }
  body[data-window="archive"] .stratum-title { font-family: var(--font-agent); font-size: 14px; color: var(--text); margin-bottom: 6px; line-height: 1.5; }
  body[data-window="archive"] .stratum-snippet { font-family: var(--font-ui); font-size: 12px; color: var(--muted); line-height: 1.6; }
  body[data-window="archive"] .stratum-meta { display: flex; gap: 10px; margin-top: 8px; font-family: var(--font-mono); font-size: 9px; color: var(--faint); letter-spacing: 0.06em; }
  body[data-window="archive"] .stratum-meta .who { display: flex; align-items: center; gap: 5px; }
  body[data-window="archive"] .stratum-meta .who-sep { color: var(--line); }

  body[data-window="archive"] .stratum[data-depth="1"] { opacity: 1.00; }
  body[data-window="archive"] .stratum[data-depth="2"] { opacity: 0.92; }
  body[data-window="archive"] .stratum[data-depth="3"] { opacity: 0.82; }
  body[data-window="archive"] .stratum[data-depth="4"] { opacity: 0.70; }
  body[data-window="archive"] .stratum[data-depth="5"] { opacity: 0.58; }
  body[data-window="archive"] .stratum[data-depth="6"] { opacity: 0.46; }
  body[data-window="archive"] .stratum[data-depth="7"] { opacity: 0.36; }
  body[data-window="archive"] .stratum[data-depth="8"] { opacity: 0.28; }

  body[data-window="archive"] section#archive-room .abyss-foot { flex: 0 0 auto; border-top: 1px dashed var(--line); padding: 10px 18px; background: var(--bg1); display: flex; justify-content: flex-end; }
  body[data-window="archive"] section#archive-room .abyss-foot-note { font-family: var(--font-mono); font-size: 10px; letter-spacing: 0.08em; color: var(--faint); border-right: 2px solid var(--line); padding-right: 12px; }
  body[data-window="archive"] section#archive-room .room-fog { position: absolute; left: 0; right: 0; bottom: 0; height: 80px; pointer-events: none; background: linear-gradient(0deg, var(--bg0), transparent); opacity: .55; }

  /* ============ E2 Space 走廊 (2026-08-24) ============ */
  body[data-window="space"] {
    --mind-base: #C8714C; --aura-x: 50%; --aura-y: 200px;
    --mind-glow: color-mix(in srgb, var(--mind-base) 15%, transparent);
    --mind-intense: color-mix(in srgb, var(--mind-base) 40%, transparent);
    --mind-line: color-mix(in srgb, var(--mind-base) 70%, #ffffff);
    --accent-solid: var(--mind-base); --frame: color-mix(in srgb, var(--mind-base) 55%, transparent);
    overflow: hidden; background: var(--bg0); display: flex; flex-direction: column; height: 100vh; padding: 0; margin: 0;
  }
  body[data-window="space"][data-theme="light"] {
    --mind-line: color-mix(in srgb, var(--mind-base) 76%, #101416);
    --accent-solid: color-mix(in srgb, var(--mind-base) 84%, #241108);
    --frame: var(--mind-line);
  }
  body[data-window="space"] .space-chrome {
    flex: 0 0 auto; display: flex; align-items: center; padding: 6px 12px 6px 18px;
    border-bottom: 1px dashed var(--line); background: var(--bg1); -webkit-app-region: drag; user-select: none;
  }
  body[data-window="space"] .space-chrome .space-chrome-title { font-family: var(--font-agent); font-size: 13px; font-style: italic; color: var(--text); }
  body[data-window="space"] .space-chrome .space-chrome-actions { margin-left: auto; display: flex; align-items: center; gap: 4px; -webkit-app-region: no-drag; }
  body[data-window="space"] .space-chrome .fold-btn { background: none; border: none; color: var(--faint); font-size: 10px; padding: 2px 6px; cursor: pointer; border-radius: 3px; }
  body[data-window="space"] .space-chrome .fold-btn:hover { color: var(--accent-solid); }
  body[data-window="space"] .space-chrome .theme-btn { width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; background: none; border: none; color: var(--muted); cursor: pointer; border-radius: 3px; padding: 0; }
  body[data-window="space"] .space-chrome .theme-btn:hover { background: var(--bg2); color: var(--text); }
  body[data-window="space"] .space-chrome .close-btn { width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; background: none; border: none; color: var(--faint); font-size: 12px; cursor: pointer; border-radius: 3px; padding: 0; }
  body[data-window="space"] .space-chrome .close-btn:hover { background: var(--danger); color: #fff; }

  body[data-window="space"] .space-engine { flex: 1 1 auto; display: flex; flex-direction: row; height: calc(100vh - 36px); overflow: hidden; }

  /* Left Sidebar: mind */
  body[data-window="space"] aside#space-mind {
    width: 200px; flex: 0 0 200px; height: 100%; display: flex; flex-direction: column; gap: 8px;
    padding: 8px; border-right: 1px dashed var(--line); background: var(--bg0); overflow-y: auto; box-sizing: border-box;
  }
  body[data-window="space"] aside#space-mind > .mod {
    border: 1px solid var(--line); border-top-color: var(--bevel); border-radius: 4px; background: var(--bg2);
    box-shadow: var(--shadow); flex: 1 1 auto; min-height: 0; display: flex; flex-direction: column;
  }
  body[data-window="space"] aside#space-mind > .mod.is-folded { flex: 0 0 auto !important; min-height: 0; }
  body[data-window="space"] aside#space-mind > .mod.is-folded > :not(.side-title) { display: none !important; }
  body[data-window="space"] aside#space-mind > .mod .w2-pin { flex: 0 0 auto; margin: 0; padding: 10px 18px 0; }
  body[data-window="space"] aside#space-mind > .mod.is-folded .w2-pin { padding: 8px 18px; }
  body[data-window="space"] aside#space-mind > .mod .w2-scroll { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 6px 18px 10px; }

  body[data-window="space"] aside#space-mind .side-title { font-family: var(--font-mono); font-size: 9px; color: var(--muted); letter-spacing: 0.08em; display: flex; align-items: center; cursor: pointer; user-select: none; }
  body[data-window="space"] aside#space-mind .side-title:hover { color: var(--text); }
  body[data-window="space"] aside#space-mind .side-title em { font-style: normal; color: var(--mind-line); }
  body[data-window="space"] aside#space-mind .side-title .fold-caret { margin-left: auto; font-size: 9px; color: var(--faint); font-family: var(--font-mono); padding-left: 6px; }
  body[data-window="space"] aside#space-mind .row { display: flex; align-items: center; gap: 6px; padding: 3px 0; font-size: 11px; color: var(--muted); cursor: pointer; transition: color 0.15s; }
  body[data-window="space"] aside#space-mind .row:hover { color: var(--text); }
  body[data-window="space"] aside#space-mind .row.active { color: var(--text); }
  body[data-window="space"] aside#space-mind .row.static { cursor: default; }
  body[data-window="space"] aside#space-mind .dot-radio { width: 7px; height: 7px; border-radius: 50%; border: 1px solid var(--muted); flex-shrink: 0; }
  body[data-window="space"] aside#space-mind .row.active .dot-radio { border-color: var(--accent-solid); background: var(--accent-solid); }
  body[data-window="space"] aside#space-mind .sq-toggle { width: 7px; height: 7px; border: 1px solid var(--muted); flex-shrink: 0; }
  body[data-window="space"] aside#space-mind .row.active .sq-toggle { border-color: var(--ok); background: var(--ok); }
  body[data-window="space"] aside#space-mind .tag-x { margin-left: auto; color: var(--mind-line); font-family: var(--font-mono); font-size: 9px; }

  /* Right Sidebar: ante (door peek) */
  body[data-window="space"] aside#space-ante {
    width: 220px; flex: 0 0 220px; height: 100%; display: flex; flex-direction: column; gap: 8px;
    padding: 8px; border-left: 1px dashed var(--line); background: var(--bg0); overflow-y: auto; box-sizing: border-box;
  }
  body[data-window="space"] aside#space-ante > .mod {
    border: 1px solid var(--line); border-top-color: var(--bevel); border-radius: 4px; background: var(--bg2);
    box-shadow: var(--shadow); flex: 1 1 auto; min-height: 0; display: flex; flex-direction: column;
  }
  body[data-window="space"] aside#space-ante > .mod.is-folded { flex: 0 0 auto !important; min-height: 0; }
  body[data-window="space"] aside#space-ante > .mod.is-folded > :not(.side-title) { display: none !important; }
  body[data-window="space"] aside#space-ante > .mod .w2-pin { flex: 0 0 auto; margin: 0; padding: 10px 18px 0; }
  body[data-window="space"] aside#space-ante > .mod.is-folded .w2-pin { padding: 8px 18px; }
  body[data-window="space"] aside#space-ante > .mod .w2-scroll { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 6px 18px 10px; display: flex; flex-direction: column; gap: 10px; }

  body[data-window="space"] aside#space-ante .side-title { font-family: var(--font-mono); font-size: 9px; color: var(--muted); letter-spacing: 0.08em; display: flex; align-items: center; cursor: pointer; user-select: none; }
  body[data-window="space"] aside#space-ante .side-title:hover { color: var(--text); }
  body[data-window="space"] aside#space-ante .side-title em { font-style: normal; color: var(--mind-line); }
  body[data-window="space"] aside#space-ante .side-title .fold-caret { margin-left: auto; font-size: 9px; color: var(--faint); font-family: var(--font-mono); padding-left: 6px; }
  body[data-window="space"] aside#space-ante .peek-sub { font-family: var(--font-mono); font-size: 9px; color: var(--muted); letter-spacing: 0.08em; margin-bottom: 4px; }
  body[data-window="space"] aside#space-ante .peek-sub em { font-style: normal; color: var(--faint); }
  body[data-window="space"] aside#space-ante .peek-plate { font-family: var(--font-agent); font-style: italic; font-size: 13px; color: var(--text); }
  body[data-window="space"] aside#space-ante .peek-line { font-family: var(--font-mono); font-size: 10px; color: var(--muted); margin-top: 3px; line-height: 1.5; }
  body[data-window="space"] aside#space-ante .peek-sed { font-size: 11px; color: var(--muted); line-height: 1.5; }
  body[data-window="space"] aside#space-ante .seg-note { font-family: var(--font-mono); font-size: 9px; color: var(--faint); margin-top: 4px; }
  body[data-window="space"] aside#space-ante .chips { display: flex; gap: 6px; flex-wrap: wrap; margin-top: 4px; }
  body[data-window="space"] aside#space-ante .artifact-chip { display: inline-block; padding: 2px 7px; border: 1px solid var(--line); border-radius: 3px; background: var(--bg0); font-family: var(--font-mono); font-size: 9px; color: var(--muted); cursor: pointer; transition: all 0.15s; }
  body[data-window="space"] aside#space-ante .artifact-chip:hover { color: var(--mind-line); border-color: var(--accent-solid); }
  body[data-window="space"] aside#space-ante .peek-log { background: none; border: none; padding: 0; font-family: var(--font-mono); font-size: 10px; color: var(--mind-line); border-left: 2px solid var(--accent-solid); padding-left: 6px; cursor: pointer; text-align: left; }
  body[data-window="space"] aside#space-ante .peek-detail { font-family: var(--font-mono); font-size: 9px; color: var(--muted); background: var(--bg0); border: 1px solid var(--line); padding: 6px 8px; line-height: 1.6; border-radius: 3px; margin-top: 6px; }
  body[data-window="space"] aside#space-ante .term-well { margin-top: auto; border: 1px solid var(--line); border-radius: 4px; background: var(--term-bg); padding: 8px 10px; font-family: var(--font-mono); font-size: 9px; line-height: 1.6; color: var(--term-fg); }
  body[data-window="space"] aside#space-ante .preview-row { color: var(--mind-line); cursor: pointer; }

  /* Center Room: space-room */
  body[data-window="space"] section#space-room { flex: 1 1 auto; min-width: 0; height: 100%; display: flex; flex-direction: column; background: var(--bg1); position: relative; overflow: hidden; }
  body[data-window="space"] section#space-room .room-status { flex: 0 0 auto; display: flex; align-items: center; gap: 8px; padding: 8px 14px; border-bottom: 1px dashed var(--line); font-family: var(--font-mono); font-size: 10px; color: var(--muted); letter-spacing: 0.05em; white-space: nowrap; overflow: hidden; -webkit-app-region: drag; }
  body[data-window="space"] section#space-room .room-status .sp { flex: 1; }
  body[data-window="space"] section#space-room .state-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent-solid); animation: breath-dot var(--breath) ease-in-out infinite; flex-shrink: 0; }

  body[data-window="space"] section#space-room .hall-head { flex: 0 0 auto; position: relative; display: flex; flex-direction: column; align-items: center; gap: 6px; padding: 18px 16px 12px; border-bottom: 1px dashed var(--line); background: radial-gradient(280px 140px at 50% 0%, var(--mind-glow), transparent 78%); transition: padding 0.35s cubic-bezier(.22,1,.36,1); -webkit-app-region: drag; }
  body[data-window="space"] section#space-room .hall-head .head-fold { position: absolute; right: 10px; top: 8px; background: none; border: 1px solid var(--line); color: var(--muted); font-size: 9px; padding: 3px 7px; cursor: pointer; border-radius: 3px; -webkit-app-region: no-drag; }
  body[data-window="space"] section#space-room .hall-head .head-fold:hover { color: var(--mind-line); border-color: var(--accent-solid); }
  body[data-window="space"] section#space-room .hall-head.folded { flex-direction: row; justify-content: center; gap: 12px; padding: 8px 16px; background: linear-gradient(90deg, transparent, var(--mind-glow), transparent); }
  body[data-window="space"] section#space-room .hall-head.folded .name-line { font-size: 13px; }
  body[data-window="space"] section#space-room .hall-head.folded .state { display: none; }
  body[data-window="space"] section#space-room .hall-head.folded .hall-note { font-size: 11px; }
  body[data-window="space"] section#space-room .hall-head .name-line { font-family: var(--font-agent); font-style: italic; font-size: 16px; color: var(--text); }
  body[data-window="space"] section#space-room .hall-head .state { font-family: var(--font-mono); font-size: 10px; color: var(--mind-line); letter-spacing: 0.08em; background: var(--mind-intense); padding: 3px 9px; border-radius: 2px; }
  body[data-window="space"] section#space-room .hall-head .hall-note { font-size: 11px; color: var(--faint); }

  body[data-window="space"] section#space-room .door-hall { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 14px 16px; display: flex; flex-direction: column; gap: 10px; }
  body[data-window="space"] .hall-mark { display: flex; align-items: center; gap: 10px; font-family: var(--font-mono); font-size: 9px; letter-spacing: 0.1em; color: var(--faint); margin-top: 4px; }
  body[data-window="space"] .hall-mark::before, body[data-window="space"] .hall-mark::after { content: ''; flex: 1; height: 1px; background: var(--line); }
  body[data-window="space"] .hall-mark.deep { color: var(--muted); }

  body[data-window="space"] .door { position: relative; display: flex; align-items: flex-start; gap: 10px; border: 1px solid var(--line); border-top-color: var(--bevel); border-radius: 4px; background: var(--bg1); padding: 10px 12px 10px 14px; box-shadow: var(--shadow); cursor: pointer; transition: border-color 0.2s, background 0.2s, color 0.2s; }
  body[data-window="space"] .door:hover { border-color: var(--accent-solid); background: var(--bg2); }
  body[data-window="space"] .door-seam { position: absolute; left: 0; top: 6px; bottom: 6px; width: 2px; background: var(--line); border-radius: 2px; transition: background 0.3s; }
  body[data-window="space"] .door-lamp { width: 28px; height: 28px; border-radius: 0; border: 1px solid var(--line); background: var(--bg2); display: flex; align-items: center; justify-content: center; font-family: var(--font-agent); font-size: 12px; color: var(--faint); flex-shrink: 0; }
  body[data-window="space"] .door-main { flex: 1; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  body[data-window="space"] .door-plate { font-family: var(--font-mono); font-size: 9px; letter-spacing: 0.08em; color: var(--faint); }
  body[data-window="space"] .door-topic { font-family: var(--font-agent); font-style: italic; font-size: 14px; color: var(--muted); line-height: 1.4; }
  body[data-window="space"] .door-inside { font-family: var(--font-mono); font-size: 9px; color: var(--faint); letter-spacing: 0.04em; display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  body[data-window="space"] .door-echo { font-size: 11px; color: var(--faint); line-height: 1.5; }
  body[data-window="space"] .door-actions { display: flex; gap: 6px; align-items: center; margin-top: 6px; flex-wrap: wrap; }
  body[data-window="space"] .btn-enter { padding: 4px 10px; font-size: 10px; letter-spacing: 0.5px; cursor: pointer; border-radius: 3px; border: 1px solid var(--accent-solid); background: var(--mind-intense); color: var(--text); transition: all 0.15s; font-family: var(--font-mono); white-space: nowrap; flex-shrink: 0; }
  body[data-window="space"] .btn-enter:hover { background: var(--accent-solid); color: var(--bg0); }

  /* 亮门独占 rep 光芒与呼吸 */
  body[data-window="space"] .door.lit { background: var(--bg2); border-color: var(--accent-solid); box-shadow: var(--shadow), var(--lift); cursor: default; }
  body[data-window="space"] .door.lit .door-seam { background: var(--accent-solid); box-shadow: 0 0 16px var(--mind-glow); }
  body[data-window="space"] .door.lit .door-plate { color: var(--mind-line); }
  body[data-window="space"] .door.lit .door-topic { color: var(--text); font-size: 15px; }
  body[data-window="space"] .door.lit .door-inside { color: var(--mind-line); }
  body[data-window="space"] .door.lit .door-echo { color: var(--muted); }
  body[data-window="space"] .door.lit .door-lamp { width: 34px; height: 34px; font-size: 15px; border: 1px solid var(--accent-solid); background: radial-gradient(circle at 38% 32%, var(--mind-intense) 0%, transparent 74%); color: var(--mind-line); box-shadow: 0 0 20px var(--mind-glow); animation: breath-avatar var(--breath) ease-in-out infinite; }
  body[data-window="space"][data-theme="light"] .door.lit .door-lamp { box-shadow: none; border: 2px solid var(--mind-line); background: color-mix(in srgb, var(--mind-base) 12%, #ffffff); }
  body[data-window="space"][data-theme="light"] .door.lit .door-seam { box-shadow: none; }

  /* 熄灯门 */
  body[data-window="space"] .door.dim .door-lamp { color: var(--faint); }
  body[data-window="space"] .door.dim .door-topic { color: var(--muted); }

  /* 沉门 */
  body[data-window="space"] .door.sunk { cursor: pointer; background: transparent; border-color: var(--line); border-top-color: var(--line); box-shadow: none; }
  body[data-window="space"] .door.sunk:hover { border-color: var(--line); background: var(--bg2); }
  body[data-window="space"] .door.sunk .door-lamp { border-style: dashed; background: transparent; }
  body[data-window="space"] .door.sunk .door-topic { font-size: 13px; color: var(--faint); }
  body[data-window="space"] .door.sunk.l1 { opacity: 0.72; margin-left: 6px; }
  body[data-window="space"] .door.sunk.l2 { opacity: 0.52; margin-left: 12px; }
  body[data-window="space"] .door.sunk.l3 { opacity: 0.36; margin-left: 18px; }
  body[data-window="space"] .sunk-tail { font-family: var(--font-mono); font-size: 9px; letter-spacing: 0.08em; color: var(--faint); padding-left: 18px; opacity: 0.6; }
  body[data-window="space"] .btn-archive { align-self: flex-start; margin-left: 18px; margin-bottom: 12px; padding: 4px 10px; border: 1px solid var(--line); border-radius: 3px; background: var(--bg0); color: var(--faint); font-family: var(--font-mono); font-size: 10px; cursor: pointer; transition: all 0.15s; }
  body[data-window="space"] .btn-archive:hover { color: var(--text); border-color: var(--muted); }

  /* Bottom console */
  body[data-window="space"] section#space-room .room-input { flex: 0 0 auto; border-top: 1px dashed var(--line); padding: 8px 14px 10px; display: flex; flex-direction: column; gap: 6px; background: var(--bg1); }
  body[data-window="space"] section#space-room .witness-row { display: flex; align-items: baseline; gap: 10px; }
  body[data-window="space"] section#space-room .witness-note { font-family: var(--font-mono); font-size: 9px; letter-spacing: 0.08em; color: var(--faint); margin-left: auto; }
  body[data-window="space"] section#space-room .input-row { display: flex; gap: 8px; align-items: center; }
  body[data-window="space"] section#space-room .attach { background: none; border: none; font-family: var(--font-mono); font-size: 10px; color: var(--mind-line); cursor: pointer; white-space: nowrap; padding: 0; }
  body[data-window="space"] section#space-room .input-box { flex: 1; border: 1px solid var(--line); border-radius: 4px; background: var(--bg2); padding: 6px 10px; font-size: 11px; color: var(--muted); white-space: nowrap; overflow: hidden; }
  body[data-window="space"] section#space-room .cursor { display: inline-block; width: 6px; height: 12px; background: var(--accent-solid); vertical-align: middle; animation: blink 1s step-end infinite; }
  body[data-window="space"] section#space-room .send { background: none; border: none; color: var(--muted); font-size: 13px; cursor: pointer; padding: 2px 6px; border-radius: 3px; }
  body[data-window="space"] section#space-room .send:hover { color: var(--mind-line); }
  body[data-window="space"] section#space-room .send.streaming { border: 1px solid var(--danger); color: var(--danger); font-size: 10px; }
  body[data-window="space"] section#space-room .send.streaming:hover { background: var(--danger); color: #fff; }
  body[data-window="space"] section#space-room .room-fog { position: absolute; left: 0; right: 0; bottom: 0; height: 60px; pointer-events: none; background: linear-gradient(0deg, var(--bg0), transparent); opacity: .55; }

  /* ============ E3 Settings 全局设置 (2026-08-24) ============ */
  body[data-window="settings"] { --mind-base: #C8714C; --mind-glow: color-mix(in srgb, var(--mind-base) 15%, transparent); --mind-intense: color-mix(in srgb, var(--mind-base) 40%, transparent); --mind-line: color-mix(in srgb, var(--mind-base) 70%, #ffffff); --accent-solid: var(--mind-base); --frame: color-mix(in srgb, var(--mind-base) 55%, transparent); overflow: hidden; background: var(--bg0); display: flex; flex-direction: column; height: 100vh; padding: 0; margin: 0; }
  body[data-window="settings"][data-theme="light"] { --mind-line: color-mix(in srgb, var(--mind-base) 76%, #101416); --accent-solid: color-mix(in srgb, var(--mind-base) 84%, #241108); --frame: var(--mind-line); }
  body[data-window="settings"] .settings-chrome { flex: 0 0 auto; display: flex; align-items: center; padding: 6px 12px 6px 18px; border-bottom: 1px dashed var(--line); background: var(--bg1); -webkit-app-region: drag; user-select: none; }
  body[data-window="settings"] .settings-chrome .settings-chrome-title { font-family: var(--font-agent); font-size: 13px; font-style: italic; color: var(--text); } body[data-window="settings"] .settings-chrome .settings-chrome-actions { margin-left: auto; display: flex; align-items: center; gap: 4px; -webkit-app-region: no-drag; }
  body[data-window="settings"] .settings-chrome .fold-btn { background: none; border: none; color: var(--faint); font-size: 10px; padding: 2px 6px; cursor: pointer; border-radius: 3px; } body[data-window="settings"] .settings-chrome .fold-btn:hover { color: var(--accent-solid); }
  body[data-window="settings"] .settings-chrome .theme-btn, body[data-window="settings"] .settings-chrome .close-btn { width: 24px; height: 24px; display: flex; align-items: center; justify-content: center; background: none; border: none; color: var(--muted); cursor: pointer; border-radius: 3px; padding: 0; }
  body[data-window="settings"] .settings-chrome .theme-btn:hover { background: var(--bg2); color: var(--text); } body[data-window="settings"] .settings-chrome .close-btn { color: var(--faint); font-size: 12px; } body[data-window="settings"] .settings-chrome .close-btn:hover { background: var(--danger); color: #fff; }
  body[data-window="settings"] .settings-engine { flex: 1 1 auto; display: grid; grid-template-columns: 1fr 1fr; gap: 12px; padding: 12px 16px; height: calc(100vh - 36px); overflow: hidden; box-sizing: border-box; } body[data-window="settings"] .settings-col { height: 100%; display: flex; flex-direction: column; gap: 8px; min-width: 0; overflow-y: auto; }
  body[data-window="settings"] .settings-col > .station-head { flex: 0 0 auto; padding: 8px 14px; border: 1px solid var(--line); border-radius: 4px; font-family: var(--font-mono); font-size: 11px; letter-spacing: 0.08em; color: var(--mind-line); background: var(--bg3); } body[data-window="settings"] .settings-col > .station-head.facility { color: var(--muted); }
  body[data-window="settings"] .settings-col > .mod { border: 1px solid var(--line); border-top-color: var(--bevel); border-radius: 4px; background: var(--bg2); box-shadow: var(--shadow); flex: 1 1 auto; min-height: 0; display: flex; flex-direction: column; } body[data-window="settings"] .settings-col > .mod.is-folded { flex: 0 0 auto !important; min-height: 0; } body[data-window="settings"] .settings-col > .mod.is-folded > :not(.side-title) { display: none !important; }
  body[data-window="settings"] .settings-col > .mod .w2-pin { flex: 0 0 auto; margin: 0; padding: 8px 18px 0; } body[data-window="settings"] .settings-col > .mod.is-folded .w2-pin { padding: 8px 18px; } body[data-window="settings"] .settings-col > .mod .w2-scroll { flex: 1 1 auto; min-height: 0; overflow-y: auto; padding: 6px 18px 8px; }
  body[data-window="settings"] .settings-col .side-title { font-family: var(--font-mono); font-size: 9px; color: var(--muted); letter-spacing: 0.08em; display: flex; align-items: center; cursor: pointer; user-select: none; } body[data-window="settings"] .settings-col .side-title:hover { color: var(--text); } body[data-window="settings"] .settings-col .side-title em { font-style: normal; color: var(--mind-line); } body[data-window="settings"] .settings-col .side-title .fold-caret { margin-left: auto; font-size: 9px; color: var(--faint); font-family: var(--font-mono); padding-left: 6px; }
  body[data-window="settings"] .settings-col .row { display: flex; align-items: center; gap: 7px; padding: 3px 0; font-size: 11px; color: var(--muted); cursor: pointer; transition: color 0.15s; } body[data-window="settings"] .settings-col .row:hover, body[data-window="settings"] .settings-col .row.active { color: var(--text); } body[data-window="settings"] .settings-col .row.readonly { cursor: default; } body[data-window="settings"] .settings-col .row.readonly:hover { color: var(--muted); } body[data-window="settings"] .settings-col .row.static { cursor: default; font-family: var(--font-mono); font-size: 10px; color: var(--text); word-break: break-all; }
  body[data-window="settings"] .settings-col .dot-radio { width: 7px; height: 7px; border-radius: 50%; border: 1px solid var(--muted); flex-shrink: 0; } body[data-window="settings"] .settings-col .row.active .dot-radio { border-color: var(--accent-solid); background: var(--accent-solid); }
  body[data-window="settings"] .settings-col .sq-toggle { width: 7px; height: 7px; border: 1px solid var(--muted); flex-shrink: 0; } body[data-window="settings"] .settings-col .row.active .sq-toggle { border-color: var(--ok); background: var(--ok); } body[data-window="settings"] .settings-col .sq-toggle.danger, body[data-window="settings"] .settings-col .row-meta.danger { border-color: var(--danger); color: var(--danger); } body[data-window="settings"] .settings-col .row.active .sq-toggle.danger { background: var(--danger); border-color: var(--danger); }
  body[data-window="settings"] .settings-col .row-meta { margin-left: auto; color: var(--faint); font-family: var(--font-mono); font-size: 9px; } body[data-window="settings"] .settings-col .row-meta.font-agent { font-family: var(--font-agent); font-style: italic; color: var(--text); } body[data-window="settings"] .settings-col .tag-x.current { margin-left: auto; color: var(--mind-line); font-family: var(--font-mono); font-size: 9px; }
  body[data-window="settings"] .settings-col .seg-bar { display: flex; gap: 3px; margin-top: 6px; } body[data-window="settings"] .settings-col .seg { flex: 1; height: 3px; background: var(--line); } body[data-window="settings"] .settings-col .seg.on { background: #3F837B; } body[data-window="settings"] .settings-col .seg-note { font-family: var(--font-mono); font-size: 9px; color: var(--muted); margin-top: 5px; }
  body[data-window="settings"] .settings-col .btn-undo { margin-top: 6px; width: 100%; padding: 4px 8px; border: 1px solid var(--line); border-radius: 3px; background: var(--bg0); color: var(--faint); font-family: var(--font-mono); font-size: 10px; cursor: pointer; transition: all 0.15s; } body[data-window="settings"] .settings-col .btn-undo:hover { color: var(--mind-line); border-color: var(--accent-solid); }
"#;

/// Build a `dioxus::desktop::wry::WebViewBuilder` attribute that injects
/// the truth CSS as a `<style>` element inside the document head.
///
/// Brief §2.6 — `dioxus::desktop::document::Stylesheet { ... }` is the
/// supported mechanism; we wrap the static CSS in `format!` once so the
/// `<style>` tag itself is part of the payload.
pub fn inject_stylesheet_html() -> String {
    format!("<style id=\"truth-css\">{}</style>", truth_css())
}

/// Theme toggle SVG (moon / sun). Returns the inner SVG markup; the
/// caller wraps it in `svg { ... }` rsx nodes with their own attributes
/// (width / height are cosmetic — the truth CSS sizes `.rc-btn` and
/// `.theme-btn` containers, so the inline dimensions are just a
/// fallback used until CSS loads).
///
/// Used by: app.rs (room main chrome), pages_archive.rs,
/// pages_space.rs, pages_settings.rs, pages_onboarding.rs.
/// Path-light refactor (2026-08-25, gap audit): the same icon was
/// inlined 10x across those files; this function is the single source.
///
/// `# ponytail: two branches returning static SVG markup, no allocation.
pub fn theme_toggle_svg(is_dark: bool) -> &'static str {
    if is_dark {
        SUN_SVG
    } else {
        MOON_SVG
    }
}

/// Inner paths for the sun icon (theme_dark = true → show sun to
/// switch to light). Static slice kept separate from the branch
/// function above so both branches stay trivial.
const SUN_SVG: &str = r#"<circle cx="8" cy="8" r="3" /><line x1="8" y1="1.4" x2="8" y2="3.2" /><line x1="8" y1="12.8" x2="8" y2="14.6" /><line x1="1.4" y1="8" x2="3.2" y2="8" /><line x1="12.8" y1="8" x2="14.6" y2="8" /><line x1="3.3" y1="3.3" x2="4.6" y2="4.6" /><line x1="11.4" y1="11.4" x2="12.7" y2="12.7" /><line x1="12.7" y1="3.3" x2="11.4" y2="4.6" /><line x1="4.6" y1="11.4" x2="3.3" y2="12.7" />"#;

/// Inner path for the moon icon (theme_dark = false → show moon to
/// switch to dark).
const MOON_SVG: &str = r#"<path d="M 13.2 9.4 A 5.6 5.6 0 1 1 6.6 2.8 A 4.5 4.5 0 0 0 13.2 9.4 Z" />"#;

/// Brand logo (northing seal) SVG. Returns the inner path markup —
/// the wrapper `svg { view_box: "0 0 200 200" }` stays at the call
/// site so consumers control their own sizing.
///
/// Used by: app.rs (status bar), pages_archive.rs (status bar),
/// pages_space.rs (status bar), pages_onboarding.rs (status bar).
/// Path-light refactor (2026-08-25, gap audit): same five-path
/// seal was duplicated 4x; this function is the single source.
pub fn brand_logo_svg() -> &'static str {
    BRAND_SVG
}

const BRAND_SVG: &str = r#"<path d="M 112.68 72.84 A 30 30 0 1 1 87.32 72.84" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" /><path d="M 126 54.97 A 52 52 0 1 1 82.28 51.22" fill="none" stroke="currentColor" stroke-width="5" stroke-linecap="round" /><path d="M 132.13 31.13 A 76 76 0 1 1 56.35 37.47" fill="none" stroke="currentColor" stroke-width="9" stroke-linecap="round" /><path d="M 56.35 37.47 Q 48 30, 44 24" fill="none" stroke="currentColor" stroke-width="8" stroke-linecap="round" /><path d="M 132.13 31.13 Q 137 24, 139 19" fill="none" stroke="currentColor" stroke-width="8" stroke-linecap="round" />"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// Truth-CSS byte-count guard. If anyone shortens or expands the CSS
    /// without updating the truth HTML, this fails — preventing silent
    /// visual divergence from the brief §3.3 "原样保留" rule.
    #[test]
    fn assert_truth_css_byte_count() {
        // Exact byte count of truth CSS file — update if truth file changes.
        // Computed from `TRUTH_CSS.len()` at the time of the gap audit (2026-08-25).
        // The file is included via `include_str!` with a UTF-8 BOM prefix (3 bytes);
        // the truth HTML is at `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.css`.
        const EXPECTED_BYTES: usize = 22240;
        assert_eq!(
            TRUTH_CSS.len(),
            EXPECTED_BYTES,
            "truth CSS byte count drifted from baseline (expected {EXPECTED_BYTES}, got {}); \
             if the truth HTML/CSS changed intentionally, bump EXPECTED_BYTES here",
            TRUTH_CSS.len(),
        );
        // Hardcoded marker: the truth CSS always starts with `:root {`
        // because palette tokens come first. If this changes, the truth
        // HTML itself changed and we need to re-derive.
        assert!(
            TRUTH_CSS.contains(":root {"),
            "truth CSS no longer opens with `:root {{` — re-derive from consult-room-main.html"
        );
    }
}
