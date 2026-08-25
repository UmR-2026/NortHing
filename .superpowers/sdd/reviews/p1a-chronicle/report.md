# Review Report — P1a (F4 编年史条)

> Reviewer: judge-m3 · Range: `a2e5e5a..e311cd6` (1 commit: `e311cd6`)
> Truth source: `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.html` L548-592

---

## Constraint verification

### C1 — BIRTH=#DAD6CF 恒定 stop 0% 不褪色；当前色恒在 100% — **PASS**

Evidence (`src/apps/desktop/src/ui_dioxus/app.rs` L876-904):
```rust
const BIRTH: &str = "#DAD6CF";
...
let col = if i == 0 {
    c.clone()                       // i=0 (BIRTH) bypasses mix_hex — 不褪色
} else {
    let t = 0.18 + 0.82 * frac;
    mix_hex(BIRTH, c, t)
};
...
stops.push(format!("{current} 100%"));  // 当前色恒在 100%
```
BIRTH literal `#DAD6CF` placed at position 0 (frac=0 → pos=0). i==0 short-circuit keeps the original color, matching truth's `i === 0 ? c : mixHex(...)` semantic. Current appended at 100% after the historical stops.

---

### C2 — 历史衰退 mixHex(BIRTH, c, 0.18 + 0.82*(i/(hist.len()-1)))；len==1 除零守卫 — **PASS**

Evidence (L884-897):
```rust
let (pos, col) = if n == 1 {
    (0.0, c.clone())                                   // ← 除零守卫
} else {
    let frac = i as f64 / (n - 1) as f64;              // i/(n-1)
    let pos = frac * 70.0;
    let col = if i == 0 { c.clone() } else {
        let t = 0.18 + 0.82 * frac;                    // ← 权重曲线一致
        mix_hex(BIRTH, c, t)
    };
    (pos, col)
};
```
Formula `0.18 + 0.82 * frac` with `frac = i/(n-1)` matches truth L566 character-for-character. Explicit `n == 1` guard avoids `(n-1) = 0` panic — truth's raw `i/(hist.length-1)` would NaN at len=1, so the guard is a strict improvement. Test `test_chronicle_gradient_single` covers the guard.

---

### C3 — 位置均分 0..70%（i/(n)*70 或等价），当前色 100% — **PASS**

Evidence (L888-889): `let pos = frac * 70.0` with `frac = i/(n-1)`. Verified for:
- n=1: pos=[0.0] (guard) → "0.00%" + "100%"
- n=3: pos=[0, 35, 70] → "0.00%, 35.00%, 70.00%" + "100%" (matches truth's initial hardcoded tgt)
- n=4: pos=[0, 23.33, 46.67, 70]
- n=0: BIRTH 0.00% + current 100% (sensible fallback)

Test `test_chronicle_gradient_three_history` asserts "0.00%", "35.00%", "70.00%", "100%" — all present.

> **Spec note** (non-blocking, see M2): brief specifies `i/(n-1)*70`, truth's `retarget()` (called on dblclick) uses `i/n*70`. Initial settled state matches truth; post-dblclick positions differ. Brief documents this as acceptable discrete approximation. Constraint text "i/(n)*70 或等价" admits the equivalent `i/(n-1)*70` form.

---

### C4 — mixHex 在 Rust 侧实现（逐通道线性插值），不用 CSS color-mix() — **PASS**

Evidence (L849-874): `parse_hex_rgb` → per-channel `ar + (br - ar) * t` (R/G/B independently) → `format!("#{:02X}{:02X}{:02X}", ...)`. Linear interpolation per channel exactly mirrors truth's `Math.round(v + (pb[i] - v) * t)`. `t.clamp(0.0, 1.0)` adds defensive bound truth lacks (truth would silently produce out-of-range values on bad input).

Grep confirms no `color-mix()` or `mixHex` call anywhere on the chronicle-bar path. (`color-mix` does appear in pre-existing CSS for other surfaces — settings/space-mind --mind-* variables — but the diff does not touch those rules.)

---

### C5 — 事件驱动：只在 Signal 变化时重渲；无 rAF / 无 keyframes / 无 idle 动画 — **PASS**

Evidence (L109-110, L482, L487-493):
```rust
let mut mind_base = use_signal(|| "#C8714C".to_string());
let mut mind_history = use_signal(|| vec![...]);
...
style: format!("background: {}", chronicle_gradient(&mind_history.read(), &mind_base.read())),
...
ondoubleclick: move |_| { /* mutates signals */ }
```
- Inline `style:` attribute is bound to signal reads → Dioxus re-renders only when signals change. ✓
- No `requestAnimationFrame`, no `setInterval`, no `use_future` added by this diff.
- No CSS transition / `@keyframes` / `@property` rule added by this diff.
- Pre-existing `use_future` calls at L115, L128 are window-manager subscription and event-channel receiver — unrelated to chronicle and not in the diff.

CSS grep for `chronicle-bar` shows only the pre-existing static fallback `linear-gradient(90deg, var(--bg3) 0%, var(--accent-solid) 100%)` in `css.rs:190`. The inline `style` attribute overrides it at runtime. ✓

---

### C6 — dblclick 演示：旧当前色入栈历史 → MINDS 轮转 — **PASS**

Evidence (L487-493):
```rust
ondoubleclick: move |_| {
    let cur = mind_base();
    mind_history.write().push(cur.clone());
    let minds = ["#C8714C", "#3F837B", "#8B5FBF", "#D99B48", "#4B8F6B"];
    let next = minds[(minds.iter().position(|m| *m == cur).unwrap_or(0) + 1) % 5];
    mind_base.set(next.to_string());
}
```
- Old `cur` pushed onto `mind_history` ✓
- `MINDS` array matches truth L552 (same 5 hex values, same order) ✓
- Cyclic rotation via `(index + 1) % 5` matches truth L588 ✓

> **Skeptical check** (judgment, see M3/M4): truth does `pos.push(100)` so the OLD `nowC` lingers at 100% briefly before drift eases it leftward. Rust does no such analog — OLD `nowC` jumps instantly to position 70% (rightmost historical slot under `i/(n-1)*70`). The "旧色慢慢沉向左" visual semantic is degraded to "旧色瞬移到70%". Brief explicitly accepts discrete jump.

---

### C7 — 不动 TRUTH_CSS / truth HTML / P0b/P0c 路径 — **PASS**

Diffstat shows only `src/apps/desktop/src/ui_dioxus/app.rs` modified in `src/`. Brief and report files are `.superpowers/sdd/` artifacts, not source.

App.rs changes:
- Functional additions (chronicle logic, mix/gradient helpers, tests)
- Three pure-formatting refactors with identical semantics:
  - L247-250: `let (left_open, right_open) = { ... }` moved out of `rsx!` block (was redundant — not a JSX child)
  - L661-705: `spawn_module_window_with_theme_rx` rewrapped if-expressions multiline
  - L786-787: destructuring `{ resolved, state_text, .. }` on one line
  - L827: `render_entry` signature unwrapped

None of the formatting changes touch P0b/P0c functional paths (send/approval/tool-confirmation). TRUTH_CSS untouched (verified via `TRUTH_CSS.strip_prefix` etc. references — content not modified). ✓

---

### C8 — 4 个单测（mix_hex t=1/t=0、gradient 单历史、gradient 三历史）— **PASS**

All 4 present at L910-935:
- `test_mix_hex_target` (t=1.0 → #3F837B) ✓
- `test_mix_hex_base` (t=0.0 → #DAD6CF) ✓
- `test_chronicle_gradient_single` (contains "0.00%" + "100%") ✓
- `test_chronicle_gradient_three_history` (contains "0.00%", "35.00%", "70.00%", "100%") ✓

Verified by re-running: `cargo test -p northhing --features ui-dioxus --lib ui_dioxus::app::tests` →
```
running 4 tests
test ui_dioxus::app::tests::test_mix_hex_base ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_single ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_three_history ... ok
test ui_dioxus::app::tests::test_mix_hex_target ... ok

test result: ok. 4 passed; 0 failed
```

---

## Skeptical checks (explicit)

| # | Check | Result |
|---|---|---|
| S1 | Truth formula `mixHex(BIRTH, c, 0.18 + 0.82*(i/(n-1)))` vs Rust | **Match** — same weights, same divisor. ✓ |
| S2 | Truth `i === 0 ? c : ...` short-circuit vs Rust | **Match** — `if i == 0 { c.clone() } else { ... }`. ✓ |
| S3 | Truth stop format `${col} ${pos.toFixed(2)}%` vs Rust `format!("{col} {pos:.2}%")` | **Match** — same 2-decimal %. ✓ |
| S4 | Truth retarget `i/n*70` vs Rust `i/(n-1)*70` | **Divergence** — initial state matches truth's hardcoded tgt, but post-dblclick layout differs. Brief documents this as acceptable discrete approximation. (M2) |
| S5 | `dblclick` "old nowC 100% then ease to target" vs Rust instant jump | **Degraded** — no drift; old nowC jumps from 100% to 70% slot. Brief accepts. (M3/M4) |
| S6 | Initial `hist = [BIRTH, '#3F837B', '#8B5FBF']` truth ↔ Rust `mind_history` initial | **Match** ✓ |
| S7 | Initial `nowC = '#C8714C'` truth ↔ Rust `mind_base` initial | **Match** ✓ |
| S8 | Hex case: Rust outputs `#7FA59D` (upper), truth `#7fa59d` (lower) | Cosmetic divergence, CSS-equivalent. (M1) |

---

## Findings

### Minor

- **M1** (cosmetic) — Hex output case divergence: Rust `format!("{:02X}")` produces uppercase, truth produces lowercase. Same numeric values, CSS treats identically. No spec violation; consider `{:02x}` if literal parity matters later.
- **M2** (documented divergence) — Post-dblclick position formula `i/(n-1)*70` vs truth's `i/n*70`. For n=4, Rust: [0, 23.33, 46.67, 70]; truth: [0, 17.5, 35, 52.5]. Initial settled state matches. Brief explicitly chose `i/(n-1)*70`. Pointer to terminal triage.
- **M3** (documented degradation) — `dblclick` produces a discrete jump, not the truth's smooth drift. Old `nowC` relocates instantly from 100% to 70% (rightmost historical slot). The "旧色慢慢沉向左" polish is lost. Brief accepts.
- **M4** (related) — No `pos.push(100)` analog; truth's old-nowC-at-100% pre-drift visual overlap is absent. Brief accepts.
- **M5** (cosmetic refactor) — `let (left_open, right_open) = { ... };` moved out of `rsx!` block at L247. No semantic change; was redundant inside `rsx!`.
- **M6** (out of scope) — `mind_history` grows unbounded across repeated dblclicks; no upper bound or trim. Not in scope for P1a; F6 persistence task will own the long-term story.

### Important
None.

### Critical
None.

---

## ⚠️ Cannot verify from diff
None — every spec property checked was observable in source. Tests re-run independently and pass.

---

## Verdicts

### SPEC verdict — **PASS**

All 8 brief constraints satisfied. The two divergences from truth semantics (M2 position formula, M3/M4 no-drift) are explicit brief design choices for "本轮离散跳变即满足语义" — they are not spec violations against the brief, only fidelity losses against the truth HTML.

### QUALITY verdict — **PASS**

- Per-channel linear interp correct, includes `t.clamp(0,1)` defensive bound.
- `n == 0` and `n == 1` both handled — guard is strictly safer than truth's raw formula.
- 4 unit tests cover all behavior mandated by brief; assertions are tight and pass on re-run.
- Signal-driven re-render; no idle animations.
- Style: `parse_hex_rgb` returns `Option` with asymmetric fallback in `mix_hex` (returns `b` if `a` unparseable, vice versa). Brief didn't specify, behavior is reasonable.
- Code is concise and self-contained; test module follows repo convention.
- Minor formatting drift in unrelated functions is non-blocking cosmetic.

---

## Final verdict — **APPROVE**

Constraint coverage 8/8. Brief's explicit divergence choices (discrete jump, `i/(n-1)*70` formula) are honored. Tests verified independently. Code is correct, minimal, and event-driven. No Critical/Important findings to gate on; Minor items go to terminal triage per AGENTS ledger convention.