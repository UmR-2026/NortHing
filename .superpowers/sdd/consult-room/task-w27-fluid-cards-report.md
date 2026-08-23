# W2.7 流体卡片打磨实现报告

## 1. 概述与改动范围

按 `task-w27-fluid-cards-brief.md` 需求完成左右侧栏卡片折叠、分卡流体拉伸、左列分组缝、内边距调整与右列终端吃底，无多余扩展。

- **修改文件**：
  - `src/apps/desktop/src/ui_dioxus/windows.rs`（758 行，< 800 硬线）
  - `src/apps/desktop/src/ui_dioxus/css.rs`（475 行，< 800 硬线）
- **未改动/锁死**：
  - `TRUTH_CSS` 真值 CSS 字节锁死（测试 `assert_truth_css_byte_count` 通过）。
  - `flags.rs` 仅用于截图临时开启，取证后已 restore 回 `false`，无 commit。
  - 左列保持单扇满高，未打开 facility 独立 OS 窗。
  - 无光标劫持（CDP `Runtime.evaluate` 纯代码触发点击）。

---

## 2. 具体实现细节

### 2.1 卡片独立折叠与指示符（§3.1）
- **左列 5 卡**（`w2c-sediment`、`w2c-rag`、`w2c-skill`、`w2c-runtime`、`w2c-axioms`）：
  - 每卡定义独立 `use_signal(|| false)`（`folded_sediment`, `folded_rag`, `folded_skill`, `folded_runtime`, `folded_axioms`）。
  - 标题行（`.side-title.w2-pin`）绑定 `onclick` 事件切换折叠状态，光标为 `cursor: pointer`，并附加 `span.fold-caret`（展开 `▾` / 折叠 `▸`）。
  - 折叠态添加 `.is-folded` 类，CSS 通过 `.mod.is-folded > :not(.side-title) { display: none !important; }` 隐藏所有列表、分段条、token 栏、全局设置按钮等子内容，高度自然收缩为标题高度。
  - 窗顶 chrome `▴ 收纳` 绑定 `fold_all`（任意展开时一键全折叠，全折叠时一键全展开）。
- **右列 3 卡**（ROUTING、PLANNER、DIFF，终端不折叠）：
  - 每卡定义独立 `use_signal(|| false)`（`folded_routing`, `folded_planner`, `folded_diff`）。
  - 标题行（`.side-title`）绑定 `onclick` 切换折叠状态与 `▸` / `▾` 符号。
  - `.side-section.is-folded > :not(.side-title) { display: none !important; }` 隐藏列表、步骤与按钮。
  - 窗顶 chrome `▴ 收纳` 绑定 `fold_all`。

### 2.2 分卡片流体拉伸（§3.2）
- **左列**：
  - `.mod` 展开态采用 `flex: 1 1 auto; min-height: 0;`，卡片内 `.w2-scroll` 设置 `flex: 1 1 auto; min-height: 0; overflow-y: auto;`。
  - `.mod.is-folded` 设置 `flex: 0 0 auto !important; min-height: 0;`。
  - 当部分卡片折叠时，展开卡自然瓜分剩余垂直高度，实现卡片内滚动与流体撑满。
- **右列**：
  - `.side-section` 采用 `flex: 0 1 auto; min-height: 0;`，折叠态为 `flex: 0 0 auto !important;`。
  - 终端 `.term-well` 设置 `margin: 0; flex: 1 1 auto; min-height: 72px; overflow-y: auto;`，吃掉窗口底部全部剩余高度，彻底消除底空。

### 2.3 左列沉积与设施分组缝（§3.3）
- 在 `w2c-skill` 与 `w2c-runtime` 之间插入 `.w2-group-seam`。
- 采用 1px dashed `var(--line)` 发丝线 + 9px muted `INNER_HEAD_FACILITY_TITLE`（三语国际化键），形成清晰的分组视觉。

### 2.4 内边距与对齐优化（§3.4）
- 标题行 `.side-title` / `.w2-pin` 水平 padding 设置为 `18px`，远离卡片左边缘。
- 列表滚动区 `.w2-scroll` 与卡尾 `.w2-foot` 同步水平 padding 为 `18px`，实现标题与列表严格左对齐。
- 窗顶拖拽条 `.w2-head` 左侧 padding 从 `2px` 增加到 `12px`，与卡片节奏呼应。

---

## 3. 验证与目验结论

### 3.1 编译与测试证据
- `cargo check -p northhing`：通过，0 error，34 warnings（与原基线持平，无 warning 倒退）。
- `cargo test -p northhing`：113/113 passed（含 `assert_truth_css_byte_count` 真值字节校验与 `dioxus_shell_default_false` 默认门禁）。

### 3.2 截图文件与 Read 目验证据
截图保存于 `C:\WINDOWS\TEMP\opencode\t7-shots\`：
1. `w27-left-dark.png`：左列 5 卡全展开（Dark），标题 18px 避让左缘，列表严格对齐，skill 与 RUNTIME 之间可见「设施」发丝缝。
2. `w27-left-light.png`：左列 5 卡全展开（Light），各组件色彩与层级正常。
3. `w27-work-dark.png`：右列 3 卡 + 终端（Dark），三卡自然 hug 内容，终端 `.term-well` 吃满底部剩余高度，无黑边断层。
4. `w27-work-light.png`：右列 3 卡 + 终端（Light），浅色主题下终端铺底正常。
5. `w27-left-folded-dark.png`：折起沉积记忆与 skill 卡，卡片仅剩单行标题与 `▸` 符号，无任何列表残留，RAG 与 RUNTIME 展开卡自然长高拉伸。
6. `w27-work-folded-dark.png`：折起 ROUTING 与 DIFF 卡，仅剩单行标题与 `▸` 符号，PLANNER 保持展开，终端自动拉伸占满下方全部空间。

**目验结论**：全部通过（折叠正常 / 拉伸正常 / 18px 内边距舒适 / 分组缝清晰 / 终端吃底无缝）。

---

## 4. 遗留与未做项（遵循 Brief 硬约束）
- 未改动 `TRUTH_CSS`。
- 未提交 Git Commit。
- `flags.rs` 已复原 `DIOXUS_SHELL = false`。
- 桌面进程 `northhing` 已完全终止。
