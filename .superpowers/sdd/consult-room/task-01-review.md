# T1 Review — consult-room chrome 与系统层

> 待审 commit: `e311aeb`（基线 `e487cd8`）
> 范围: `git diff e487cd8 e311aeb`（15 文件 / +340 / −277）
> Brief: `.superpowers/sdd/consult-room/task-01-brief.md`
> 真值: `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.html`
> Spike: `docs/design/2026-07-22-frontend-redesign/slint-feasibility-consult-room.md`

## Spec verdict: PASS — 六项全做，窗控入流/双边界/room-fog/8s 呼吸/头像方形/ChronicleBar 与真值视觉一致

六项逐一对照：
- **§3.1 WindowChrome 重制** — 标题栏废除（`background: transparent`，无独立条）；窗控四键以 Path/Rectangle 自绘入右上（`x: parent.width - 140px; y: 8px; width: 120px; height: 28px;`），回调 `window-minimize/maximize/close` 与 `toggle-theme` 全部接 root；印章 `x: 24px; y: parent.height - 24px - self.height; opacity: 0.25`（hover 0.4）按真值；**状态行/room-head 拖拽区未接线**（brief §3.1 允许"无则报告"但 report 未记 — 见 Important §I-2）。
- **§3.2 双边界** — 外圈 `width: parent.width; height: parent.height;` + 内圈 `width: parent.width - 2px; height: parent.height - 2px;`（inset 表达式贴窗口），`visible: !RedesignTheme.dark` 仅亮色显内圈；外圈 border `RedesignTheme.dark ? mind-drive-frame : border`，亮色用 `border`（palette 无 `line` token，见 Minor）。
- **§3.3 room-fog** — `AirTint` 三层：底平铺 `air-rep/air-rep-speaking/fog-abyss`（cold 切 abyss）、顶晕 `halo-rep/halo-rep-speaking/halo-abyss` 径向、底雾 70%-100% 高度径向；`cold: current-route == "archive"` 与 `speaking` 升档均保留。
- **§3.4 呼吸 8s 单钟** — `system_constants.slint` 全局 8000ms 周期 + `Math.cos`（与真值 sin 等价，仅相位 90° 偏移，无视觉差），三档振幅：`amp-avatar` 0.85→1.0（振幅 0.15）、`amp-membrane` 0→0.35、`amp-aura` 0.65→1.0。Rectangle 全部绑 opacity（无 scale-x/y）。
- **§3.5 头像方形化** — `AvatarWrap` 三层 Rectangle `border-radius: 0px;`，`Pill`/`ToolChip` `border-radius: 0px;`（`r-pill` token 废弃）；截图确认头像/标签/状态 pill 全部近尖角，小圆点（dot-radio/state-dot）保持 50% 不动。
- **§3.6 ChronicleBar 定稿** — `height: 4px; opacity: 0.7; border-radius: 0px;`，渐变 `@linear-gradient(90deg, root.birth 0%, root.now 100%)`，暴露 `in property <color> birth/now`（默认 `RedesignTheme.t.birth` 与 `t.rep-500`，右端 ≡ 强调色同源）；换色仪式属 T2。

## Quality verdict: PASS — Slint 翻译红线 + 哲学红线全过；瑕疵集中在文件编码与报告完整性

**Slint 红线**：
- ✅ 无 box-shadow / `@keyframes` 新增 / 运行时 color-mix / 百分比 border-radius / Rectangle scale-x/y / 新 infinite
- ✅ 渐变仅 `@linear-gradient` / `@radial-gradient`
- ✅ `halo-color.with-alpha(0)` 是 color 内置方法，非 CSS color-mix，无运行时混色

**哲学红线**：
- ✅ rep 仅用于 agent 侧（头像光晕、状态 pill、印章、顶晕）；用户侧（输入框、消息气泡、按钮）无 rep 染色
- ✅ 无 dashboard 数字 / 无 emoji / 印章 opacity 0.25 / 8s 单钟 / 振幅分级（不引入新 infinite）/ 近尖角 / 编年史右端 ≡ 强调色同源

**红灯集中在三处**（详见 Findings）：
- API 清单在 report 留空（brief §7 明确要求"定稿系统 API 清单，后续任务将直接引用"）
- 状态行无 drag 接线（brief §3.1 允许，但要求"report 记录"）
- 8 个 .slint 文件新增 UTF-8 BOM + 混入 LF/CRLF，与既有文件不一致

## Findings

### Critical
- 无。

### Important
- **I-1 — Report 缺 API 清单** — `task-01-report.md` 的「API 变动」节为空字符串（GBK 解码后）。Brief §7 明确要求「**定稿系统 API 清单**（组件名 / 属性 / 回调 / 呼吸常量位置）—— 后续任务将直接引用，务必准确」。当前 T2/T3 等后续任务需要引用以下但 report 未记载：
  - `WindowChrome` 新增 callback：`toggle-theme()`（其余 5 个 callback 名不变）；新增 in property：`signal: bool`、`inner-drawer-open: bool`、`outer-drawer-open: bool`。
  - `AvatarWrap` 默认 `initial` 从 `"知"` 改为 `""`（无回归，所有调用方传入覆盖）；`breathing` 默认 true；签名不变。
  - `ChronicleBar` 新签名：暴露 `birth: color`（默认 `RedesignTheme.t.birth`）+ `now: color`（默认 `RedesignTheme.t.rep-500`），高度 4px、opacity 0.7。
  - `AirTint` 行为：`speaking` / `cold` 入参不变；顶晕新增 `opacity: dark ? amp-aura : 0`（亮色隐藏，与真值 `display: none` 等价）。
  - 呼吸常量：新增 `src/apps/desktop/src/ui/system_constants.slint`，导出 `breathe-phase`（cos）、`breathe-progress`、三档 `amp-avatar / amp-membrane / amp-aura`。
  - 状态文件: `.superpowers/sdd/consult-room/task-01-report.md`（**已 commit**，编排者终审需剥离，见 FYI）

- **I-2 — 状态行/room-head 拖拽区未接线，report 未记** — `WindowChrome.slint:47-72` 的"Status line / drag area"Rectangle 仅有 HorizontalLayout 渲染文字，无 `TouchArea`/`pointer-event` 处理，也未向上转发 drag 信号。Brief §3.1："现状若已有拖拽接线则保持；**没有则报告**，不新造 Rust FFI"。当前 Rust 侧也无 `-webkit-app-region: drag` 等效接线（slint winit 后端不支持 CSS-style app-region，需 `WindowProperties` 自定义实现），需要：(a) report 显式记录"无 drag 接线，留待 FR-T3 框架化处理"，或 (b) 当前任务内最小可拖拽（如设置 TouchArea.mouse-drag → 调用 `WindowPosition`）。两条都未做 → 接受现状则至少要补一条 report 说明，否则编排者无从知晓。

- **I-3 — 文件编码不一致（新增 UTF-8 BOM + 混入 LF）** —
  | 文件 | BOM | 行尾 |
  |---|---|---|
  | `WindowChrome.slint` | 新增 | CRLF |
  | `AirTint.slint` | 新增 | **LF** |
  | `AvatarWrap.slint` | 新增 | CRLF |
  | `ChronicleBar.slint` | 新增 | **LF** |
  | `Pill.slint` | 新增 | CRLF |
  | `ToolChip.slint` | 新增 | CRLF |
  | `PresenceZone.slint` | 不变 | CRLF |
  | `system_constants.slint`（新文件） | 有 | CRLF |
  | `main.slint` | 新增 | CRLF |
  | `redesign_palette.slint`（未改） | 无 | CRLF |
  | `views/ChatPaneView.slint`（未改） | 无 | CRLF |

  8 个改动文件引入 BOM 而仓库其他 .slint 全部无 BOM；同一 commit 内 `AirTint.slint` 与 `ChronicleBar.slint` 使用 LF，其他 6 个改动文件用 CRLF。Slint 编译器对 BOM 容忍，但 (a) 后续 git diff 噪音、(b) `gitattributes` 未配 `*.slint text eol=lf` 时跨平台协作易混。建议 fix 时统一去除 BOM、统行为 LF（与仓库 LF 偏好一致 — `redesign_palette` 等老文件 CRLF 是历史遗留，但新文件从 LF 开始更好）。

- **I-4 — 拖拽区 28px 让位残留** — `main.slint:266` 仍保留 `space := SpaceView { x: 28px; ... }`（FR-T4-1 旧预留），但 `WindowChrome` 已废除左右把手（左侧 28px 区现在是空的，仅 watermark 占据底部），导致中央 room 视觉上偏右 28px，截图（dark/light）可见 room 框中心线略偏右 28px。Brief §3.1 "窗控四键移入主体区右上（不锚侧栏、不进独立条）" 隐含"侧栏空间让位也应取消"。建议 (a) T2 调 room 居中或 (b) report 说明 28px 让位的去留安排 — 当前两未做。

### Minor（ledger triage）

- **m-1 — 亮色外圈 border 用 `border` 替 `line`** — `WindowChrome.slint:23`：`RedesignTheme.dark ? mind-drive-frame : border`。Palette 无 `line` token，最接近是 `border`（亮 #E6E3DD vs 真值 --line #c3ccd1，差约 18 灰度）。视觉差异在 1px 线上肉眼几乎不可见，但严格按真值应新增 `line` token（生成器走 `oklch-to-srgb.py`）。建议终审追加 line token。

- **m-2 — `signal` in property 残留** — `WindowChrome.slint:9` 仍保留 `in property<bool> signal: false;` 但无任何引用（FR-T3 信号点逻辑已废除）。可清。

- **m-3 — `toggle-left/toggle-right` 失去把手后无点击触发区** — `WindowChrome.slint:11-12` 保留两个 callback（main.slint 仍接 `root.left-panel-open`/`right-panel-open`），但 UI 上无触发元素。Report 已记录"留给门铃宝石任务处理"。**仅记，不阻断**。

- **m-4 — `AvatarWrap.initial` 默认值变更** — `"知"` → `""`（无回归，仅内部清理；两处调用方都传入值）。

- **m-5 — `ChronicleBar` 渐变简化** — 真值用 3+ 段 stop（含按龄褪色混合），实现用 2 段 `birth 0% → now 100%`。Brief §3.6 描述了"按龄褪向出生灰"的语义但**未要求**实现该混合 JS；当前简化与"暴露属性供后续任务绑定"自洽。换色仪式在 T2 完成，可接受。

- **m-6 — `cos` 而非 `sin`** — `system_constants.slint:5` 用 `Math.cos(animation-tick() / 8000ms * 360deg)`。与真值 sin 半周期偏移，振幅与起止点位置一致。视觉无差。

- **m-7 — Pill/ToolChip comment 头部 GBK mojibake 显示** — 在 Windows 默认 codepage 下 `read` 工具会渲染为 "閫氱敤寰界珷/鏍囩"，**实际文件 UTF-8 内容正确**（已字节验证 `E5 9F BA E7 A1 80...` = "基础"）。Slint 编译器正常识别，不影响。仅记 review 工具渲染层面的混淆。

### FYI（编排者终审 commit 清理）

> ⚠️ `e311aeb` 用 `git commit --amend` 收拢，把 brief §7 之外的 SDD 文件捎进了产品 commit：
> - `.superpowers/sdd/consult-room/plan.md`（+7 −?）
> - `.superpowers/sdd/consult-room/progress.md`（0 字节变化）
> - `.superpowers/sdd/consult-room/task-01-brief.md`（新增）
> - `.superpowers/sdd/consult-room/task-01-report.md`（新增）
> - `docs/design/2026-07-22-frontend-redesign/consult-room/build-shots/t1-main-dark.png`
> - `docs/design/2026-07-22-frontend-redesign/consult-room/build-shots/t1-main-light.png`
>
> Brief §7.1 明确"不含截图"且 report/brief 不应 commit。本次接受，但请终审在合并到 `feat/consult-room-slint` 前 `git rm --cached` 上述文件 + 单独 commit "chore(consult-room): 剥离 T1 误捎 SDD/截图"。

## 截图判读

- **dark (`t1-main-dark.png`)** — 四键渲染清晰（☀ Path 太阳、─ 横线、□ 方框、✕ Path 叉号），外圈 mind-drive-frame 暗橙发光线可见，内圈亮色专属线在 dark 下正确隐藏；头像方形带 rep-400 边框 + 径向渐变背景 + "序"字；编年史条在 "知序" 与 "在场" 之间隐约可见（opacity 0.7）；印章 "northing" 左下淡灰；整屋空气底染 + 顶晕暗色 mind-glow 正确呈现；room 框因 §I-4 的 28px 让位略偏右（可见偏移）。**结论**：与真值一致，✓。

- **light (`t1-main-light.png`)** — 四键全部切换至 light 配色（path 颜色走 `RedesignTheme.t.muted`，close 仍走 danger）；双边界清晰（外圈浅 line + 内圈浅 line 1px，因 `visible: !dark` 在亮色下显示）；avatar 白底 + mind-line 边框 + "序" 文字（无 box-shadow glow）；编年史条可见；印章左下淡；**顶晕在亮色下 opacity:0 正确隐藏**，与真值 `[data-theme="light"] #global-aura { display: none; }` 等价；room 框同样偏右 28px。**结论**：与真值一致，✓。

## 不可从 diff 判读项

- **`cargo check -p northhing` 输出** — report 未附命令与输出（§5 验证要求"report 附命令+输出"）。状态 DONE 不可独立证实。建议补：`$env:PATH = "C:\msys64\mingw64\bin;C:\msys64\usr\bin;" + $env:PATH; $env:CARGO_TARGET_DIR='E:\agent-project\northing\target'; $env:CARGO_PROFILE_DEV_SPLIT_DEBUGINFO='off'; cargo check -p northhing 2>&1 | tail -30`。
- **8s 呼吸的肉眼走查** — spike 探针结论遗留项（spike §遗留.1）。静态截图无法验证动画相位。需 `desktop:dev` 走查 + 多帧采样。
- **drag 接线是否被 Winit 后端拦截** — 当前 `WindowChrome` 无 drag 处理，需要确认 `AppWindow` 是否需要 `WindowProperties` 自定义或保持无 drag（FR-T3 处理）。
- **字体 fallback 在四键上的实际像素** — 截图渲染正常，但不同 Windows 字体环境下 Path 是否仍 1px 锐利需走查。

## 总结

T1 在视觉真值、Slint 翻译、哲学红线三轴上完整达成；6 项范围全做、双边界/8s 呼吸/方形化/ChronicleBar 与真值高度一致；Critical 0；Important 集中在 report 完整性（API 清单 + drag 状态记录）与文件编码一致性 — 编排者下一轮 fixer 派发补 4 项即可放行。