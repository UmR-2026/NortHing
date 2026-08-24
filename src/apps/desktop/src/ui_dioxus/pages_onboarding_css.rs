// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Task EF-E4 (2026-08-24) — Onboarding CSS definitions.
//
// Self-contained stylesheet for the consult room onboarding window,
// faithfully converted from `consult-room-onboarding-v2.html`.

pub const ONBOARDING_CSS: &str = r#"
  :root {
    --mind-base: #7e8896; /* 初始未着色灰 */
    --aura-x: 50%; --aura-y: 200px;
    --mind-glow: color-mix(in srgb, var(--mind-base) 15%, transparent);
    --mind-intense: color-mix(in srgb, var(--mind-base) 40%, transparent);
    --mind-line: color-mix(in srgb, var(--mind-base) 70%, #ffffff);
    --accent-solid: var(--mind-base);
    --frame: color-mix(in srgb, var(--mind-base) 55%, transparent);
    --font-ui: 'Space Grotesk', 'Noto Sans SC', sans-serif;
    --font-mono: 'JetBrains Mono', Consolas, monospace;
    --font-agent: 'Fraunces', 'Noto Serif SC', serif;
    --breath: 8s;
  }
  [data-theme="dark"] {
    --bg0: #0b0c0e; --bg1: #101216; --bg2: #161920; --bg3: #1c2029;
    --line: #262b34; --bevel: rgba(255,255,255,0.06);
    --text: #e6e8ec; --muted: #7d8590; --faint: #7d8791;
    --warn: #D99B48; --danger: #E5484D; --ok: #46A758;
    --term-bg: #08090a; --term-fg: #58c26a;
    --shadow: 0 1px 0 rgba(0,0,0,0.4);
    --lift: 0 12px 32px rgba(0,0,0,0.28);
    --node-right: #E8E1D6;
  }
  [data-theme="light"] {
    --bg0: #edf0f1; --bg1: #f6f8f9; --bg2: #ffffff; --bg3: #eef1f2;
    --line: #c3ccd1; --bevel: #ffffff;
    --text: #17222a; --muted: #51616a; --faint: #5c6d76;
    --warn: #8a5a14; --danger: #c02334; --ok: #2e7350;
    --term-bg: #12181a; --term-fg: #5fbf68;
    --shadow: 0 1px 0 rgba(23,34,42,0.08);
    --lift: 0 10px 26px rgba(23,34,42,0.10);
    --mind-line: color-mix(in srgb, var(--mind-base) 76%, #101416);
    --accent-solid: color-mix(in srgb, var(--mind-base) 84%, #241108);
    --frame: var(--mind-line);
    --node-right: #17222A;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body, html { width: 100vw; height: 100vh; overflow: hidden; background: var(--bg0); color: var(--text); font-family: var(--font-ui); font-size: 13px; }
  button, input, select { font-family: inherit; }
  :focus-visible { outline: 2px solid var(--mind-line); outline-offset: 2px; }

  @media (prefers-reduced-motion: reduce) {
    *, *::before, *::after { animation-duration: 0.01ms !important; animation-iteration-count: 1 !important; transition-duration: 0.01ms !important; }
    #global-aura { animation: none; opacity: 0.85; }
    .state-dot, .cursor, .agent-avatar, .membrane, .membrane-frame { animation: none; }
  }

  /* 生物态呼吸 */
  @keyframes breath-avatar { 0%, 100% { transform: scale(1); } 50% { transform: scale(1.03); } }
  @keyframes breath-aura   { 0%, 100% { opacity: 1; } 50% { opacity: .65; } }
  @keyframes breath-membrane { 0%, 100% { opacity: 0; } 45%, 55% { opacity: .35; } }
  @keyframes breath-dot { 0%, 100% { opacity: 1; } 50% { opacity: .45; } }
  @keyframes blink { 50% { opacity: 0.15; } }

  /* 收容框 + 活膜 */
  #containment { position: fixed; inset: 12px; border: 1px solid var(--frame); pointer-events: none; z-index: 5; transition: border-color 0.6s; }
  [data-theme="light"] #containment { box-shadow: 0 0 0 1px var(--line); }
  .membrane-frame { position: fixed; inset: 13px; border: 1px solid var(--mind-base); pointer-events: none; z-index: 5; opacity: 0; animation: breath-membrane var(--breath) ease-in-out infinite; }
  #global-aura { position: fixed; inset: 0; pointer-events: none; z-index: 1; background: radial-gradient(circle 640px at var(--aura-x) var(--aura-y), var(--mind-glow) 0%, transparent 74%); animation: breath-aura var(--breath) ease-in-out infinite; }
  [data-theme="light"] #global-aura { display: none; }

  /* 布局引擎 */
  #engine { position: relative; z-index: 10; height: 100vh; display: flex; justify-content: center; align-items: stretch; gap: 16px; padding: 26px 48px; }
  #room-wrap { position: relative; width: min(780px, 100%); height: 100%; }
  #room { width: 100%; height: 100%; display: flex; flex-direction: column; background: var(--bg1); border: 1px solid var(--line); border-radius: 6px; box-shadow: var(--shadow), var(--lift); overflow: hidden; position: relative; }
  .membrane { position: absolute; top: 0; bottom: 0; width: 1px; background: var(--mind-base); opacity: 0; animation: breath-membrane var(--breath) ease-in-out infinite; pointer-events: none; }
  .membrane.l { left: 0; }
  .membrane.r { right: 0; }

  /* 窗控四键 */
  .room-controls { position: absolute; top: 8px; right: 10px; display: flex; gap: 2px; z-index: 30; -webkit-app-region: no-drag; }
  .rc-btn { width: 28px; height: 28px; display: flex; align-items: center; justify-content: center; color: var(--muted); cursor: pointer; font-size: 11px; background: none; border: none; border-radius: 3px; transition: all 0.15s; }
  .rc-btn:hover { background: var(--bg2); color: var(--text); }
  .rc-btn.close:hover { background: var(--danger); color: #fff; }

  /* 品牌与状态 */
  .brand-inline { display: flex; align-items: center; gap: 8px; opacity: .7; pointer-events: none; user-select: none; color: var(--text); }
  .brand-inline svg { width: 15px; height: 15px; display: block; }
  .seal-name { font-family: var(--font-agent); font-style: italic; font-size: 12px; color: var(--text); }
  .room-status { display: flex; align-items: center; gap: 16px; padding: 8px 18px; border-bottom: 1px dashed var(--line); font-family: var(--font-mono); font-size: 10px; color: var(--muted); letter-spacing: 1px; -webkit-app-region: drag; }
  .room-status .sp { flex: 1; }
  .state-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent-solid); animation: breath-dot var(--breath) ease-in-out infinite; }

  /* 房间中枢 */
  .room-head { position: relative; display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 22px 18px 14px; border-bottom: 1px dashed var(--line); -webkit-app-region: drag;
               background: radial-gradient(280px 140px at 50% 0%, var(--mind-glow), transparent 78%); transition: padding 0.35s cubic-bezier(.22,1,.36,1); }
  .room-head .state { background: var(--mind-intense); padding: 3px 9px; border-radius: 2px; font-family: var(--font-mono); font-size: 10px; color: var(--mind-line); letter-spacing: 0.08em; }
  .head-fold { position: absolute; right: 10px; top: 8px; -webkit-app-region: no-drag; background: none; border: 1px solid var(--line); color: var(--muted); font-size: 9px; padding: 2px 6px; cursor: pointer; border-radius: 3px; }
  .head-fold:hover { color: var(--mind-line); border-color: var(--accent-solid); }
  .room-head.folded { flex-direction: row; justify-content: center; gap: 12px; padding: 8px 18px; background: linear-gradient(90deg, transparent, var(--mind-glow), transparent); }
  .room-head.folded .agent-avatar { width: 26px; height: 26px; font-size: 12px; box-shadow: 0 0 12px var(--mind-glow); }
  .room-head.folded .name-line { font-size: 13px; }
  .room-head.folded .state { display: none; }
  .agent-avatar { width: 52px; height: 52px; border-radius: 0; border: 1px solid var(--accent-solid); background: radial-gradient(circle at 38% 32%, var(--mind-intense) 0%, transparent 74%); display: flex; align-items: center; justify-content: center; font-family: var(--font-agent); font-size: 22px; color: var(--mind-line); box-shadow: 0 0 26px var(--mind-glow); animation: breath-avatar var(--breath) ease-in-out infinite; -webkit-app-region: no-drag; transition: all 0.6s; }
  [data-theme="light"] .agent-avatar { box-shadow: none; border: 2px solid var(--mind-line); background: color-mix(in srgb, var(--mind-base) 12%, #ffffff); }
  [data-inhabited="false"] .agent-avatar { border-style: dashed; box-shadow: none; }
  .room-head .name-line { font-family: var(--font-agent); font-style: italic; font-size: 17px; color: var(--text); }

  /* 仪式内容流 */
  .chat-flow { flex: 1; overflow-y: auto; padding: 20px 22px; display: flex; flex-direction: column; gap: 18px; }
  .ritual-divider { display: flex; align-items: center; gap: 12px; font-family: var(--font-mono); font-size: 9px; letter-spacing: 0.1em; color: var(--faint); }
  .ritual-divider::before, .ritual-divider::after { content: ''; flex: 1; height: 1px; background: var(--line); }

  .ritual-card { background: var(--bg2); border: 1px solid var(--line); border-top-color: var(--bevel); border-radius: 4px; padding: 14px 16px; box-shadow: var(--shadow); display: flex; flex-direction: column; gap: 12px; }
  .ritual-card-head { display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid var(--line); padding-bottom: 8px; }
  .ritual-card-title { font-family: var(--font-mono); font-size: 10px; letter-spacing: 0.08em; color: var(--mind-line); display: flex; align-items: center; gap: 6px; }
  .ritual-card-title em { font-style: normal; color: var(--faint); }
  .ritual-card-step { font-family: var(--font-mono); font-size: 9px; color: var(--faint); }
  .ritual-narrative { font-family: var(--font-agent); font-size: 13px; color: var(--muted); line-height: 1.6; }

  .field-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  @media (max-width: 600px) { .field-grid { grid-template-columns: 1fr; } }
  .field-group { display: flex; flex-direction: column; gap: 5px; }
  .field-group.full { grid-column: 1 / -1; }
  .field-label { font-family: var(--font-mono); font-size: 9px; color: var(--faint); letter-spacing: 0.06em; }
  .field-label em { font-style: normal; color: var(--muted); }
  .field-input { border: 1px solid var(--line); border-radius: 3px; background: var(--bg0); padding: 7px 10px; font-family: var(--font-mono); font-size: 11px; color: var(--text); transition: border-color 0.2s; width: 100%; }
  .field-input:focus { border-color: var(--accent-solid); outline: none; }
  .field-input::placeholder { color: var(--faint); opacity: 0.6; }

  /* 色板选择器：人类唯一改色入口 */
  .palette-picker { display: grid; grid-template-columns: repeat(5, 1fr); gap: 8px; margin-top: 4px; }
  .palette-swatch { border: 1px solid var(--line); border-radius: 4px; background: var(--bg0); padding: 8px 6px; display: flex; flex-direction: column; align-items: center; gap: 6px; cursor: pointer; transition: all 0.2s; text-align: center; }
  .palette-swatch:hover { border-color: var(--text); }
  .palette-swatch.selected { border-color: var(--swatch-color); background: color-mix(in srgb, var(--swatch-color) 12%, var(--bg0)); box-shadow: var(--shadow); }
  .swatch-circle { width: 18px; height: 18px; border-radius: 50%; background: var(--swatch-color); box-shadow: 0 0 8px color-mix(in srgb, var(--swatch-color) 40%, transparent); }
  .swatch-name { font-family: var(--font-mono); font-size: 10px; color: var(--text); }
  .swatch-desc { font-family: var(--font-mono); font-size: 8px; color: var(--faint); line-height: 1.2; }

  .test-row { display: flex; align-items: center; gap: 12px; margin-top: 4px; }
  .ritual-btn { border: 1px solid var(--line); background: var(--bg0); color: var(--muted); padding: 7px 14px; font-family: var(--font-mono); font-size: 10px; cursor: pointer; border-radius: 3px; transition: all 0.15s; display: inline-flex; align-items: center; gap: 6px; }
  .ritual-btn:hover { border-color: var(--accent-solid); color: var(--mind-line); }
  .ritual-btn.primary { border-color: var(--accent-solid); background: var(--mind-intense); color: var(--text); font-weight: 600; }
  .ritual-btn.primary:hover { background: var(--accent-solid); color: var(--bg0); }
  .test-status { font-family: var(--font-mono); font-size: 10px; color: var(--faint); }
  .test-status.ok { color: var(--ok); }

  /* 底部提交栏 / deck */
  .room-footer { border-top: 1px dashed var(--line); padding: 12px 18px; display: flex; justify-content: space-between; align-items: center; background: var(--bg1); }
  .witness-pledge { font-family: var(--font-agent); font-style: italic; font-size: 12px; color: var(--muted); }
  .cursor { display: inline-block; width: 7px; height: 13px; background: var(--accent-solid); vertical-align: middle; animation: blink 1s step-end infinite; }

  /* 膜结触发器与侧栏 */
  .membrane-node { position: absolute; z-index: 36; width: 12px; height: 64px; padding: 0; border: none; cursor: pointer; -webkit-app-region: no-drag; opacity: .55; transition: width 0.2s cubic-bezier(.22,1,.36,1), opacity 0.35s; }
  .membrane-node:hover, .membrane-node:focus-visible { width: 18px; opacity: .95; }
  .membrane-node.left  { left: 0; top: var(--gem-mid, 84px); margin-top: -32px; background: radial-gradient(70% 50% at 0% 50%, var(--accent-solid), transparent 78%); }
  .membrane-node.right { right: 0; bottom: 230px; background: radial-gradient(70% 50% at 100% 50%, var(--node-right), transparent 78%); }
  .membrane-node.is-open { opacity: .22; }
  .membrane-node.is-open:hover, .membrane-node.is-open:focus-visible { opacity: .6; }
  .room-fog { position: absolute; left: 0; right: 0; bottom: 0; height: 140px; pointer-events: none; background: linear-gradient(0deg, var(--bg0), transparent); opacity: .55; }

  .mod { position: relative; z-index: 40; border: 1px solid var(--line); border-top-color: var(--bevel); border-radius: 4px; background: var(--bg2); box-shadow: var(--shadow), var(--lift); overflow: hidden; }
  #mind { width: 280px; display: flex; flex-direction: column; gap: 14px; background: transparent; border: none; box-shadow: none; overflow: visible; align-self: flex-start; transition: width 0.35s cubic-bezier(.22,1,.36,1), opacity 0.35s; }
  #mind .mod { width: 280px; }
  #mind.mod-hidden { width: 0; height: 0; overflow: hidden; opacity: 0; pointer-events: none; margin: 0; padding: 0; }
  #work { width: 320px; align-self: flex-end; max-height: 520px; transition: width 0.35s cubic-bezier(.22,1,.36,1), opacity 0.35s, max-height 0.25s; }
  #work.folded { max-height: 40px; }
  #work.mod-hidden { width: 0; height: 0; overflow: hidden; opacity: 0; pointer-events: none; margin: 0; padding: 0; }

  .station-head { display: flex; align-items: center; justify-content: space-between; padding: 10px 14px; border-bottom: 1px solid var(--line); font-family: var(--font-mono); font-size: 10px; letter-spacing: 0.08em; color: var(--mind-line); cursor: pointer; user-select: none; }
  .station-head.facility { color: var(--muted); }
  .fold-btn { background: none; border: 1px solid var(--line); color: var(--muted); font-size: 9px; padding: 4px 8px; cursor: pointer; border-radius: 3px; }
  .fold-btn:hover { color: var(--mind-line); border-color: var(--accent-solid); }
  .side-section { padding: 12px 14px; border-bottom: 1px solid var(--line); }
  .side-section:last-child { border-bottom: none; }
  .side-title { font-family: var(--font-mono); font-size: 9px; color: var(--muted); letter-spacing: 0.08em; margin-bottom: 8px; }
  .side-title em { font-style: normal; color: var(--faint); }
  .row { display: flex; align-items: center; gap: 7px; padding: 3px 0; font-size: 11px; color: var(--muted); transition: color 0.15s; }
  .row.active { color: var(--text); }
  .dot-radio { width: 7px; height: 7px; border-radius: 50%; border: 1px solid var(--muted); flex-shrink: 0; }
  .row.active .dot-radio { border-color: var(--accent-solid); background: var(--accent-solid); }
  .sq-toggle { width: 7px; height: 7px; border: 1px solid var(--muted); flex-shrink: 0; }
  .row.active .sq-toggle { border-color: var(--ok); background: var(--ok); }
  .seg-bar { display: flex; gap: 3px; margin-top: 8px; }
  .seg { flex: 1; height: 4px; background: var(--line); }
  .seg.on { background: var(--accent-solid); }
  .seg-note { font-family: var(--font-mono); font-size: 9px; color: var(--muted); margin-top: 6px; }
  .plan-check { width: 8px; height: 8px; border: 1px solid var(--muted); flex-shrink: 0; }
  .row.done .plan-check { border-color: var(--ok); background: var(--ok); }
  .row.done { color: var(--faint); }
  .fname { flex: 1; font-family: var(--font-mono); font-size: 11px; }
  .term-well { margin: 10px 14px 14px; border: 1px solid var(--line); border-radius: 4px; background: var(--term-bg); padding: 10px 12px; font-family: var(--font-mono); font-size: 10px; line-height: 1.7; color: var(--term-fg); }
  .preview-row { color: var(--mind-line); }

  .mod.is-folded .card-body, #work.is-folded .side-section, #work.is-folded .term-well { display: none; }

  /* R4 W4 自定义细滚动条——同 css.rs 同款；本窗自包含不注 TRUTH_CSS，需自带（转写层新增，真值零 scrollbar 规则） */
  ::-webkit-scrollbar { width: 10px; height: 10px; }
  ::-webkit-scrollbar-track { background: transparent; }
  ::-webkit-scrollbar-thumb { background: var(--line); border-radius: 5px; border: 3px solid transparent; background-clip: padding-box; }
  ::-webkit-scrollbar-thumb:hover { background: var(--faint); }
  ::-webkit-scrollbar-button { display: none; width: 0; height: 0; }
  ::-webkit-scrollbar-corner { background: transparent; }

  @media (max-width: 1180px) {
    #engine { padding: 18px 20px; gap: 12px; }
  }
  @media (max-width: 600px) {
    body, html { overflow: auto; }
    #containment, .membrane-frame { display: none; }
    #engine { display: block; height: auto; padding: 12px; }
    #room { height: 78vh; width: 100%; }
    #mind, #work { width: 100%; margin-top: 12px; max-height: none; }
  }
  /* 主题色档三（主页同款 2026-08-03 铺）：横向缝线染 16% mind 色 */
  .room-status { border-bottom: 1px dashed color-mix(in srgb, var(--mind-base) 16%, var(--line)); }
  .room-head { border-bottom: 1px dashed color-mix(in srgb, var(--mind-base) 16%, var(--line)); }
  .room-input { border-top: 1px dashed color-mix(in srgb, var(--mind-base) 16%, var(--line)); }
"#;
