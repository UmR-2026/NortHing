# W2.7 流体卡片打磨 — implementer brief

> 编排者派发。**写代码 + CDP 截图自验**。不要问问题。不要 commit。不要改 `flags.rs` 以外的无关文件；`flags.rs` 仅允许临时 `DIOXUS_SHELL=true` 以便 GUI 取证，收工必须 restore 回 `false`（`git checkout -- src/apps/desktop/src/flags.rs`）。

## 0. 坐标

- worktree：`E:\agent-project\northing\.worktrees\consult-room-build`
- 分支：`feat/consult-room-slint`
- 相关文件（只动这些，除非编译强制）：
  - `src/apps/desktop/src/ui_dioxus/windows.rs`
  - `src/apps/desktop/src/ui_dioxus/css.rs`（`OVERLAY_CSS` 块）
  - 如需折叠态文案：`src/apps/desktop/src/ui_dioxus/i18n.rs` + `src/crates/assembly/core/locales/{zh-CN,zh-TW,en-US}.ftl`（三语对称；新键必须三份都有）
- 不要动：`registry.rs` 生命周期、`TRUTH_CSS` 真值文件、`app.rs` 宝石逻辑（左宝石已只开 `self` 一扇满高窗，保持）。
- 行数硬线：`windows.rs` / `css.rs` 各 <800。超了先抽小组件/压缩注释，禁止拆文件到新 crate。

## 1. 用户原话（需求唯一来源，勿稀释）

1. 「左右两侧的模块都可以收缩，而非仅右侧」
2. 「高度也可以进行收缩」
3. 「卡片中的子列表可以直接缩到只剩标题」
4. 「也可以分卡片拉伸」
5. 「卡片左上方的字太靠边了，往里挪一下」
6. 方案 **A** 已拍板：右列卡 hug + 终端吃剩余高度；左列沉积|设施分组缝；两列不要再半高对切（已完成的满高左列保持）。

## 2. 已落地、不要回滚

左宝石只 spawn `self` 一扇满高窗；五卡已在 `self_app_root`：沉积记忆 / RAG / skill / RUNTIME / AXIOMS。facility 插件仍注册但不由宝石打开。work 窗 chrome 已与左列对齐（无框 ▴ 收纳 + ✕）。这些保持。

## 3. 要做的（按优先级）

### 3.1 每张内容卡：折叠到只剩标题

对左列 `.mod`（五张）和右列 `.side-section`（ROUTING / PLANNER / DIFF，不含终端）实现：

- 标题行可点（`side-title` / `w2-pin`），点击切换折叠。
- **折叠态**：只显示标题行（含 em 英文小标 + 一张 9px 折叠指示，如 `▸` 收 / `▾` 开）。列表、seg-bar、token 行、sys-config 脚、按钮全部隐藏。
- **展开态**：现状内容。默认全部展开。
- 每卡独立 signal，互不影响。
- 折叠后该卡高度 = 标题行高度（约 28–36px），把垂直空间让给兄弟卡（见 3.2）。
- 终端 `.term-well` **不折叠**（它是控制台域）。
- 窗顶 chrome（「沉积与设施」/「身外之物」+ 收纳 + ✕）不是内容卡，不要拿来折列表。现有 ▴ 收纳钮若仍无 onclick：本轮给它折/展**整列内容卡**（全部折叠 ↔ 全部展开）或隐藏该死按钮。二选一，不要留假按钮。推荐：▴ = 全部内容卡折叠到标题。

实现提示：dioxus 里每卡 `use_signal(|| false)` 作 `folded`；`class: if folded() { "mod is-folded" } else { "mod" }`；CSS `.is-folded .w2-scroll, .is-folded .w2-foot, .is-folded .seg-bar, .is-folded .sys-config { display: none }`。右列同理加 `is-folded` 到 `side-section`。

### 3.2 分卡片拉伸（高度流体）

- 展开的卡：`flex: 1 1 auto; min-height: 0` —— 兄弟折叠后，展开卡吃剩余高度，列表在卡内滚。
- 折叠的卡：`flex: 0 0 auto` —— 只占标题高。
- 禁止再 `flex: 1 1 0` 均分把两行内容撑成空盒子。
- **右列终端** `.term-well`：`flex: 1 1 auto; min-height: 72px` —— 上面三卡 hug/折叠后，终端吃掉窗底空区（用户已同意这个，不要把整窗改矮）。
- 左列无终端：五卡按 3.2 分配；全展开时 hug 内容、溢出才内滚；有人折叠则展开者长高。

### 3.3 左列分组缝（沉积 | 设施）

在 skill 卡和 RUNTIME 卡之间加一条分组，不是新 OS 窗：

- 8–12px 空隙 + 1px dashed `var(--line)` 发丝，或一行 9px muted「设施」标签（用已有 `INNER_HEAD_FACILITY_TITLE` 键，三语已有）。
- 不要再 spawn facility 窗。

### 3.4 标题内边距（「字太靠边」）

左列 `.w2-pin` / `.side-title` 和右列 `.side-title`：

- 现状大约 `padding: 12px 14px 0` 或更贴边。
- 改为水平 **18–20px**（往**里**挪，离卡左缘远一点，不是更贴边）。
- 列表行 `.w2-scroll` / `.row` 同步左垫，和标题齐，不要标题缩进、列表仍贴边。
- chrome 窗标题（station-head）也略增 `padding-left` 到 10–12px，与卡标题节奏一致。

### 3.5 不要做

- 不要恢复左右半高对切。
- 不要改主窗 room 中枢/宝石位置。
- 不要改 TRUTH_CSS 字节。
- 不要接真后端。
- 不要 commit。
- 不要把折叠做成整窗 OS minimize。

## 4. 验证

1. `rustup` 用 `C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc`
2. 临时 `DIOXUS_SHELL=true`，`cargo build -p northhing`（warnings 只降不升；现基线 bin ~33–34）
3. CDP 取证（纪律：禁光标劫持。Hidden 启动 + `--remote-debugging-port=9333`）：
   - 脚本可改 `C:\WINDOWS\TEMP\opencode\t7-cdp2.ps1`：左宝石只等 **1** 个新 page（满高左列），右宝石 1 个 work。
   - 截图落到 `C:\WINDOWS\TEMP\opencode\t7-shots\`：
     - `w27-left-dark.png` / `w27-left-light.png`（五卡全展开）
     - `w27-work-dark.png` / `w27-work-light.png`（三卡+终端，终端应吃底空）
     - `w27-left-folded-dark.png`：至少折起 skill + 沉积记忆，只剩标题，RUNTIME 被拉高
     - `w27-work-folded-dark.png`：折起 ROUTING+DIFF，PLANNER 或终端变高
   - 折叠截图：用 CDP `Runtime.evaluate` 点卡标题，不要 SetCursorPos。
4. 你必须 **Read 打开上述 PNG** 目验：标题不贴左缘；折叠后无列表残影；右列底空被终端填住；左列 skill|RUNTIME 之间有分组缝。
5. 收工：`git checkout -- src/apps/desktop/src/flags.rs`；`Stop-Process -Name northhing -Force`。
6. 报告写到：`.superpowers/sdd/consult-room/task-w27-fluid-cards-report.md`  
   含：改了哪些选择器/signal、截图路径、目验结论、未做项。

## 5. 完成定义

- 左右每一张内容卡都能点标题折到只剩标题。
- 展开卡能吃折叠卡让出的高度（拉伸）。
- 右列终端填满窗底剩余。
- 左列沉积三卡与设施两卡之间有可见分组。
- 卡标题相对卡左缘有明显内边距（≥18px）。
- flags 还原 false；无 commit。
