// SPDX-License-Identifier: MIT OR Apache-2.0
//
// W9-6 file-tree / preview styles. Lives outside `css.rs` because that file
// was already at the 830-line rot-budget ceiling before this round; adding
// the ~15 selectors needed for the new files section would have pushed it
// over. By scoping the rules here (and gating them on `body[data-window]`
// the same way OVERLAY_CSS does) we keep `css.rs` byte-identical and let
// `panel_files.rs` inject the block alongside the section it renders.

/// Styles for the right-drawer file tree + preview pane. Injected into
/// the same `<style>` slot as OVERLAY_CSS so the cascade matches.
pub const FILES_OVERLAY_CSS: &str = r#"
  /* ---- W9-6 文件树 / 预览 ---- */
  body[data-window="outer"] aside#work .files-tree { font-family: var(--font-mono); font-size: 10px; color: var(--muted); padding: 4px 0; max-height: 220px; overflow-y: auto; }
  body[data-window="outer"] aside#work .files-children { padding-left: 12px; border-left: 1px dashed var(--line); }
  body[data-window="outer"] aside#work .files-row { display: flex; align-items: center; gap: 6px; padding: 2px 6px; cursor: pointer; border-radius: 3px; min-width: 0; }
  body[data-window="outer"] aside#work .files-row:hover { background: var(--bg2); color: var(--text); }
  body[data-window="outer"] aside#work .files-row.files-selected { background: var(--bg3); color: var(--accent-solid); }
  body[data-window="outer"] aside#work .files-row.files-row-error { color: var(--danger); cursor: default; }
  body[data-window="outer"] aside#work .files-icon { width: 12px; flex-shrink: 0; text-align: center; opacity: 0.85; font-size: 11px; }
  body[data-window="outer"] aside#work .files-name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; flex: 1 1 auto; }
  body[data-window="outer"] aside#work .files-size { font-size: 9px; color: var(--faint); flex-shrink: 0; }
  body[data-window="outer"] aside#work .files-preview { border-top: 1px dashed var(--line); padding: 8px 0 0; margin-top: 4px; }
  body[data-window="outer"] aside#work .files-preview-title { font-family: var(--font-mono); font-size: 9px; color: var(--faint); letter-spacing: 0.06em; display: flex; align-items: center; gap: 6px; padding: 0 0 4px; }
  body[data-window="outer"] aside#work .files-preview-placeholder, body[data-window="outer"] aside#work .files-preview-loading, body[data-window="outer"] aside#work .files-preview-empty { font-size: 11px; color: var(--faint); padding: 8px 4px; }
  body[data-window="outer"] aside#work .files-preview-error { font-size: 11px; color: var(--danger); padding: 8px 4px; }
  body[data-window="outer"] aside#work .files-preview-text { font-family: var(--font-mono); font-size: 11px; line-height: 1.45; color: var(--text); background: var(--bg2); border: 1px solid var(--line); border-radius: 4px; padding: 8px 10px; margin: 0; max-height: 240px; overflow-y: auto; white-space: pre-wrap; word-break: break-all; }
  body[data-window="outer"] aside#work .side-section.is-folded .files-tree, body[data-window="outer"] aside#work .side-section.is-folded .files-preview { display: none; }
"#;
