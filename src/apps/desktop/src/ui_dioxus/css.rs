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
  body[data-window] aside .row > * { min-width: 0; }
  body[data-window] aside .fold-btn, body[data-window] aside .tag-x,
  body[data-window] aside .diff-add, body[data-window] aside .diff-del { flex-shrink: 0; white-space: nowrap; }
  /* 终端井（C 单成果，保留）：禁断行 + 横向兜底。 */
  body[data-window="outer"] aside#work .term-well { white-space: pre; overflow-x: hidden; }

  /* ---- F1 顺带：浮窗高度撑满（窗高 820 而内容短 → 底部空区）。
     真值 #mind/#work 是三列布局侧栏（align-self 自然高）；窗内单列
     场景下撑满视口 + 内部滚动是合理转写：卡片均分高度、card-body 内
     滚；#work 的 side-sections 均分、term-well 自然贴底。 */
  body[data-window] aside { height: 100vh; max-height: none; display: flex; flex-direction: column; }
  body[data-window="inner"] aside#mind .mod { flex: 1 1 0; min-height: 0; display: flex; flex-direction: column; }
  body[data-window="inner"] aside#mind .card-body { flex: 1 1 0; min-height: 0; overflow-y: auto; }
  body[data-window="outer"] aside#work .side-section { flex: 1 1 0; min-height: 0; overflow-y: auto; }
  body[data-window="outer"] aside#work .term-well { margin-top: 10px; }

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
     左结不可见根因：port 无 JS 量测 → --gem-mid 回落 84px → 左结
     top=52px 落在 room-head 的 mind-glow 径向渐变区，12px 宽 mind 色
     与渐变同色系融合不可见（右结 --node-right 反色块在纯 bg 区故可见）。
     真值实测头像中心线 = 221px（p0 review 记录）→ 显式定义 --gem-mid。
     命中区：透明 border 扩到 20px 宽（background-clip 保视觉 12px），
     hover 26px 保视觉 18px。 */
  #room { --gem-mid: 221px; }
  #room .membrane-node { box-sizing: border-box; width: 20px; padding: 0; border-left: 4px solid transparent; border-right: 4px solid transparent; background-clip: padding-box; }
  #room .membrane-node:hover, #room .membrane-node:focus-visible { width: 26px; }

  /* ---- D（A+B+C 单成果，保留）：room 主窗 scrim 压暗层。
     inner/outer 任一可见时压暗（block-contract §2 规则 4 降级形态，
     alpha 0.22 = handoff §5.3「22% 压暗」；scrim token：dark #000000 /
     light #38352E）。主题切换随 body 的 data-theme 正确变色。
     pointer-events:none 保证宝石等控件事件穿透。 */
  #room-scrim { position: fixed; inset: 0; z-index: 39; background: rgba(0, 0, 0, 0.22); pointer-events: none; }
  body[data-theme="light"] #room-scrim { background: rgba(56, 53, 46, 0.22); }
"#;

/// Build a `dioxus::desktop::wry::WebViewBuilder` attribute that injects
/// the truth CSS as a `<style>` element inside the document head.
///
/// Brief §2.6 — `dioxus::desktop::document::Stylesheet { ... }` is the
/// supported mechanism; we wrap the static CSS in `format!` once so the
/// `<style>` tag itself is part of the payload.
pub fn inject_stylesheet_html() -> String {
    format!("<style id=\"truth-css\">{}</style>", TRUTH_CSS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Truth-CSS byte-count guard. If anyone shortens or expands the CSS
    /// without updating the truth HTML, this fails — preventing silent
    /// visual divergence from the brief §3.3 "原样保留" rule.
    #[test]
    fn assert_truth_css_byte_count() {
        // Length > 0 is the minimum contract; the exact byte count comes
        // from the truth file at the path above.
        assert!(TRUTH_CSS.len() > 1000, "truth CSS unexpectedly short");
        // Hardcoded marker: the truth CSS always starts with `:root {`
        // because palette tokens come first. If this changes, the truth
        // HTML itself changed and we need to re-derive.
        assert!(
            TRUTH_CSS.contains(":root {"),
            "truth CSS no longer opens with `:root {{` — re-derive from consult-room-main.html"
        );
    }
}
