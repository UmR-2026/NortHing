/* shared/theme-switch.js — R5 unified floating theme switch
 * ───────────────────────────────────────────────────────────
 * Anchored to .app/.shell container (bottom-right) with absolute positioning.
 * Double-click non-interactive area → button fades in from bottom-right.
 * Single-click button → toggles data-theme="dark" on <html>, dispatches nt-theme-changed.
 * Double-click again → button hides.
 *
 * Slint mapping: this is a web-only prototype control; no Slint equivalent.
 */
(function () {
  'use strict';

  const FADE_MS = 350;
  const ICON_SIZE = 20;

  const sunSVG = `<svg width="${ICON_SIZE}" height="${ICON_SIZE}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <circle cx="12" cy="12" r="5"/>
    <line x1="12" y1="1" x2="12" y2="3"/>
    <line x1="12" y1="21" x2="12" y2="23"/>
    <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
    <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
    <line x1="1" y1="12" x2="3" y2="12"/>
    <line x1="21" y1="12" x2="23" y2="12"/>
    <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
    <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
  </svg>`;

  const moonSVG = `<svg width="${ICON_SIZE}" height="${ICON_SIZE}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
  </svg>`;

  // ── Inject styles ──
  const style = document.createElement('style');
  style.textContent = `
    .nt-theme-fab {
      position: absolute;
      bottom: 16px;
      right: 16px;
      z-index: 50;
      width: 44px;
      height: 44px;
      border-radius: 50%;
      border: 1px solid var(--border);
      background: var(--elevated);
      color: var(--muted);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      opacity: 0;
      pointer-events: none;
      transition: opacity ${FADE_MS}ms ease-out, color 0.15s, background 0.15s, border-color 0.15s, filter 0.15s;
      filter: drop-shadow(0 2px 8px rgba(120,100,70,0.18));
    }
    .nt-theme-fab.nt-visible {
      opacity: 1;
      pointer-events: auto;
    }
    .nt-theme-fab:hover {
      color: var(--fg);
      background: var(--raised);
      border-color: var(--muted);
    }
    .nt-theme-fab svg {
      transition: transform 0.25s ease;
    }
  `;
  document.head.appendChild(style);

  // ── Create FAB ──
  const fab = document.createElement('button');
  fab.className = 'nt-theme-fab';
  fab.title = '切换亮/暗主题';
  fab.setAttribute('aria-label', '切换亮/暗主题');

  // Anchor to .app/.shell container if present
  const container = document.querySelector('.app, .shell');
  if (container) {
    const cs = window.getComputedStyle(container);
    if (cs.position === 'static') {
      container.style.position = 'relative';
    }
    container.appendChild(fab);
  } else {
    document.body.appendChild(fab);
  }

  // ── State ──
  let visible = false;
  let lastClickTime = 0;

  // ── Helpers ──
  function isDark() {
    return document.documentElement.getAttribute('data-theme') === 'dark';
  }

  function updateIcon() {
    fab.innerHTML = isDark() ? moonSVG : sunSVG;
  }

  function toggleTheme() {
    if (isDark()) {
      document.documentElement.removeAttribute('data-theme');
    } else {
      document.documentElement.setAttribute('data-theme', 'dark');
    }
    updateIcon();
    document.dispatchEvent(new CustomEvent('nt-theme-changed', {
      detail: { dark: isDark() }
    }));
  }

  function showFab() {
    visible = true;
    fab.classList.add('nt-visible');
  }

  function hideFab() {
    visible = false;
    fab.classList.remove('nt-visible');
  }

  // ── FAB click → toggle theme ──
  fab.addEventListener('click', function (e) {
    e.stopPropagation();
    toggleTheme();
  });

  // ── Double-click non-interactive area → show/hide FAB ──
  document.addEventListener('dblclick', function (e) {
    // Ignore if target is inside an interactive element
    const tag = e.target.tagName;
    if (tag === 'BUTTON' || tag === 'A' || tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA' ||
        e.target.closest('button, a, input, select, textarea, .demo-btn, .topbar-btn, .nav-item, .segment, .toggle')) {
      return;
    }
    if (visible) {
      hideFab();
    } else {
      showFab();
    }
  });

  updateIcon();
})();
