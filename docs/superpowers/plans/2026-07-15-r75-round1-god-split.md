# R75 Round 1 — God file split plan (2026-07-15)

> 状态: plan | Mavis authored | **不 dispatch**，留给下下 session
> Source: `E:/agent-project/northing/` (post B3-T6, post source recovery)
> Done criterion: 2 candidates picked + sub-plans written + `cargo check -p northhing-cli` passes AFTER dispatch

## 1. 候选池 (post-R74remainder)

### God tier (750+ lines)

| 文件 | 行数 | 方法数 | 适合 split | 备注 |
|---|---|---|---|---|
| `ui/model_config_form.rs` | 962 | 27 | ✅ 首选 | 自包含，struct/render 边界清晰 |
| `ui/chat/render.rs` | 867 | 24 | ✅ 次选 | 通过 `chat.rs include!` 内联，改 `mod` 需配合 |
| `ui/theme.rs` | 871 | — | ❌ | Theme struct 字段定义为主，不可拆 |
| `desktop/app_state/settings.rs` | 788 | — | 中 | desktop 层，优先级低 |
| `ui/startup/selectors.rs` | 767 | — | 中 | 已在 selectors 目录，可能已部分拆过 |

### Rising tier (500-749 lines, top 5 by size)

`input.rs`, `acp_cli.rs`, `command_palette.rs`, `contracts/events/src/agentic.rs`, `desktop/app_state/callbacks_lifecycle.rs`

---

## 2. Round 1 选定

| 优先级 | 文件 | 行数 | 理由 |
|---|---|---|---|
| **1** | `ui/model_config_form.rs` | 962 | 最大 god，自包含，无 include! 耦合 |
| **2** | `ui/chat/render.rs` | 867 | 方法多 (24fn)，渲染逻辑可分子组件清晰 |

---

## 3. Sub-plan 1: `model_config_form.rs` (962 行 / 27 fn)

### 当前结构

- `ModelFormResult` (struct, 14 fields)
- `ModelFormAction` (enum: None/Save/Cancel)
- `FormField` (enum: 13 variants, Basic + Advanced)
- `ModelConfigFormState` (struct, 16 fields + UI state)
- 渲染逻辑: `render_form`, `render_field`, `render_advanced_section` 等
- 事件处理: `handle_key`, `handle_input`

### Split 方案

```
model_config_form.rs          → 保留，pub use 重新导出 (facade, ~30 行)
model_config_form/
├── mod.rs                   → pub use 各子模块
├── types.rs                 → ModelFormResult, ModelFormAction, FormField (~60 行)
├── state.rs                 → ModelConfigFormState + field navigation (~120 行)
└── render.rs                → 渲染逻辑 (~700 行)
```

### 边界

- `types.rs` 零逻辑，纯数据结构 (copy/transmute 级别安全)
- `state.rs` 持状态 + Tab/Shift-Tab navigation + field validation
- `render.rs` 纯渲染，接受 `&ModelConfigFormState` 不持有

### 验证

- `cargo check -p northhing-cli` → `Finished`
- `grep -r "ModelFormResult\|ModelFormAction\|FormField\|ModelConfigFormState"` 确认无外部直接字段访问需要改
- 测试不变 (state_split_tests 模式)

---

## 4. Sub-plan 2: `chat/render.rs` (867 行 / 24 fn)

### 当前结构

`chat.rs` 用 `include!("chat/render.rs")` 内联进 `impl ChatView`。
24 个方法: render, render_header, render_messages, render_status_bar, render_input,
render_command_menu, render_model_selector, render_agent_selector, render_session_selector,
render_skill_selector, render_subagent_selector, render_mcp_selector, render_mcp_add_dialog,
render_provider_selector, render_model_config_form, render_theme_selector, render_shortcuts,
calculate_shortcuts_height, calculate_status_height 等。

### Split 方案 (需配合 chat.rs 改 mod)

**Step 1**: chat.rs 把 `include!("chat/render.rs")` → `pub mod render;`

**Step 2**: render.rs → render/mod.rs + 子模块

```
chat.rs                      → include!("chat/render.rs") 改成 pub mod render;
render.rs                    → 移走
render/
├── mod.rs                  → pub use 子模块 + impl ChatView 主入口 (~50 行)
├── layout.rs               → render(), calculate_shortcuts_height(), calculate_status_height() (~80 行)
├── header.rs                → render_header() (~60 行)
├── messages.rs              → render_messages() (~120 行)
├── status_bar.rs            → render_status_bar() (~80 行)
├── input.rs                 → render_input() (~100 行)
├── selectors.rs              → 所有 render_*_selector() 方法 (~200 行)
└── overlays.rs              → render_permission_overlay, render_question_overlay, command_palette, info_popup (~150 行)
```

### 边界

- 所有方法仍在 `impl ChatView` (不同文件中可重复 impl 同类型)
- `mod.rs` 做 facade，pub use 所有子模块方法
- `layout.rs` 持有主布局 + 调用各子模块渲染的 orchestrator
- `selectors.rs` 最大，包含 8 个 `render_*_selector()` 方法

### 验证

- `cargo check -p northhing-cli` → `Finished`
- `grep -r "ChatView" src/apps/cli/src/` 确认 impl 分布正确
- 无 dead_code warning 新增 (所有方法都是 pub，仍被调用)

---

## 5. 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| model_config_form 有外部直接字段访问 | 中 | dispatch 前 grep 字段引用，如有则 inline pub(super) 或 getter |
| chat/render include! → mod 改后 break | 低 | 方法签名不变，只改声明位置 |
| render 方法跨文件共享 helper | 中 | helper 放 mod.rs 或独立 helpers submodule |
| 962/867 行拆分后某子模块仍 >500 | 低 | 可 further split (如 render.rs 再拆) |

---

## 6. Dispatch 顺序

1. **model_config_form.rs** 先 dispatch (自包含，低风险)
2. 验证通过后 → **chat/render.rs** dispatch
3. 每个 subagent 完成后 Mavis 必跑 `cargo check -p northhing-cli` 找 `Finished`
4. M3 take-over 作为 safety net (subagent 超时时)

## 7. 时间估计

| 阶段 | 估时 |
|---|---|
| model_config_form split | 60-90 min (LongCat) |
| chat/render split | 90-120 min (LongCat, include! 改 mod 多一步) |
| Mavis review per subagent | 15-20 min |
| **总** | **3-4 h** |
