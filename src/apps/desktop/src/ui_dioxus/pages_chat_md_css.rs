// SPDX-License-Identifier: MIT OR Apache-2.0
//
// W15-1b-2 (2026-09-02) — Markdown chat styling.
//
// Stylesheet for Markdown rendered messages in the consult-room chat surface.
// Scoped exclusively to `.md-rendered` subtrees to prevent style leaks to non-Markdown UI.

pub const CHAT_MD_CSS: &str = r#"
  /* ============ W15-1b-2 Chat Markdown Stylesheet ============
     Scoped strictly to .md-rendered container to protect host UI layout.
     Aligns with TRUTH_CSS serif body font and color system. */

  /* Arbitration §7#7 hard requirement: pre-wrap + break-word on every
     rendered subtree (.msg-agent / .body carry .md-rendered too). */
  .md-rendered {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .md-rendered p.md-p {
    margin: 0 0 8px 0;
    line-height: inherit;
  }
  .md-rendered p.md-p:last-child {
    margin-bottom: 0;
  }

  .md-rendered h1.md-h1,
  .md-rendered h2.md-h2,
  .md-rendered h3.md-h3,
  .md-rendered h4.md-h4,
  .md-rendered h5.md-h5,
  .md-rendered h6.md-h6 {
    margin: 12px 0 6px 0;
    font-family: inherit;
    font-weight: 600;
    line-height: 1.3;
    color: var(--text);
  }
  .md-rendered h1.md-h1:first-child,
  .md-rendered h2.md-h2:first-child,
  .md-rendered h3.md-h3:first-child,
  .md-rendered h4.md-h4:first-child,
  .md-rendered h5.md-h5:first-child,
  .md-rendered h6.md-h6:first-child {
    margin-top: 0;
  }
  .md-rendered h1.md-h1 { font-size: 1.35em; }
  .md-rendered h2.md-h2 { font-size: 1.22em; }
  .md-rendered h3.md-h3 { font-size: 1.12em; }
  .md-rendered h4.md-h4 { font-size: 1.05em; }
  .md-rendered h5.md-h5,
  .md-rendered h6.md-h6 { font-size: 1.0em; }

  .md-rendered ul.md-ul,
  .md-rendered ol.md-ol {
    margin: 0 0 8px 0;
    padding-left: 20px;
  }
  .md-rendered ul.md-ul:last-child,
  .md-rendered ol.md-ol:last-child {
    margin-bottom: 0;
  }
  .md-rendered li.md-li {
    margin: 2px 0;
  }
  .md-rendered li.md-li > p.md-p {
    margin: 0;
  }

  .md-rendered code.md-inline-code {
    font-family: var(--font-mono);
    font-size: 0.9em;
    padding: 2px 5px;
    border-radius: 3px;
    background: color-mix(in srgb, var(--mind-base) 10%, var(--bg0));
    border: 1px solid var(--line);
  }

  .md-rendered pre.md-code-block {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    padding: 10px 12px;
    margin: 8px 0;
    background: var(--bg0);
    border: 1px solid var(--line);
    border-radius: 4px;
    overflow-x: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .md-rendered pre.md-code-block code.md-code {
    font-family: inherit;
    font-size: inherit;
    background: none;
    border: none;
    padding: 0;
    color: inherit;
  }
  .md-rendered pre.md-code-block:last-child {
    margin-bottom: 0;
  }

  .md-rendered blockquote.md-blockquote {
    margin: 8px 0;
    padding: 4px 12px;
    border-left: 3px solid var(--line);
    color: var(--muted);
    background: color-mix(in srgb, var(--mind-base) 5%, transparent);
    border-radius: 0 3px 3px 0;
  }
  .md-rendered blockquote.md-blockquote:last-child {
    margin-bottom: 0;
  }
  .md-rendered blockquote.md-blockquote > *:last-child {
    margin-bottom: 0;
  }

  .md-rendered a.md-link {
    color: var(--accent-solid);
    text-decoration: underline;
    text-underline-offset: 2px;
    transition: color 0.15s;
  }
  .md-rendered a.md-link:hover {
    color: var(--mind-line);
  }

  .md-rendered hr.md-hr {
    border: none;
    border-top: 1px dashed var(--line);
    margin: 12px 0;
  }

  .md-rendered img.md-img {
    max-width: 100%;
    height: auto;
    border-radius: 3px;
    border: 1px solid var(--line);
    margin: 6px 0;
  }
  .md-rendered span.md-img-alt {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--faint);
  }

  .md-rendered strong.md-strong {
    font-weight: 600;
  }
  .md-rendered em.md-em {
    font-style: italic;
  }
"#;
