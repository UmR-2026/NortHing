# Slint 落地可行性 POC 结论与翻译处方（2026-07-24）

> 背景：外部 reviewer 称 v2 HTML 设计范式 → Slint 1.17 存在"系统性 gap / 一堵墙"，建议评估换框架。
> 编排者决策=路2：最小 POC 实测 gap 真实大小，**排除换框架**（26 个 .slint + 全 Rust 回调重写，成本实证不可接受）。
> POC 探针文件 `src/apps/desktop/src/ui/poc_v2_visual_probe.slint`（临时，结论留档后回滚；本文件为结论处方，不依赖探针留存）。

## 总判

**reviewer 的"墙"实测为"矮墙"——纯语法翻译层，无能力天花板。** 9 项 v2 特性全部通过 slint-compiler 编译，零不可行项。换框架无必要。
但 POC 的"全 0 折扣"略乐观：呼吸循环与径向定位两项是"机制可行、端到端闭环待 FR-T3 走查拧完最后一个螺丝"，准确整体折扣=**低**。

## Slint 可行性映射表 v2（POC 实测）

| v2 特性 | Slint 实现法（编译过） | 折扣 | 备注 |
|---|---|---|---|
| 整屋空气染色 | 全屏 `Rectangle { background: <预计算hex>; }` 平铺 | 0 | color-mix 改预计算色 |
| 顶晕径向 | `@radial-gradient(circle at <Xpx> <Ypx>, c1, c100 70%)` | 低 | `at` 只吃 px 不吃 % → 绑 `parent.width/2` 表达式 |
| 头像体温径向 | `@radial-gradient(circle, c30, c00 70%)` 叠头像径向 | 低 | 同上 |
| 呼吸无限循环 | `property<float> s: 1.0 + 0.015*Math.sin(animation-tick()/6000ms*360deg);` 绑 `scale-x`/`scale-y` | 低-中 | **纯 Slint 可行，无需 Rust Timer**；POC 未把 s 接 scale，FR-T3 接上+走查 |
| 编年史 linear | `@linear-gradient(90deg, ...)` | 低 | 用角度，不用 CSS `to right` |
| 活跃轮竖线+面 | 面=预计算 hex Rectangle + 竖线 `@linear-gradient(180deg,...)` 2.5px | 0 | |
| color-mix 预计算 | 手算 sRGB alpha 混合硬编码；透明端用 8 位 hex `#RRGGBB00` 避黑边 | 低 | 无原生 color-mix；正式做法=扩 `oklch-to-srgb.py` 生成器产出混合色进 palette |
| 可变字体轴 | `import "./fonts/Fraunces-Display.ttf"; font-family:"Fraunces Display";` | 0 | WONK1+SOFT60 已烘焙进静态 ttf（FR-T2），Slint 无 font-variation-settings 但无需 |
| 暗色翻转 | `dark ? DARK : LIGHT` 三元（RedesignTheme 已落地） | 0 | |

## CSS → Slint 语法对照速查（FR-T3 翻译用）

- `color-mix(in srgb, C p%, BG)` → 构建期预计算 hex（或 8 位 alpha hex 叠在 BG 上）；正式入 `redesign_palette.slint` 须经生成器，**勿手改该文件**。
- `radial-gradient(circle at 50% 30%, ...)` → `@radial-gradient(circle at (parent.width/2) (parent.height*0.3), ...)`（at 用 length 表达式，不用 %）。
- `linear-gradient(to right, ...)` → `@linear-gradient(90deg, ...)`；`to bottom` → `180deg`。
- `@keyframes breathe { 50%{scale:1.015} }` 无限循环 → `animation-tick()` 驱动 + `Math.sin()`，结果绑 `scale-x`/`scale-y`（`Math.sin` 参数须 angle：`.../周期ms * 360deg`；无 `Math.PI`）。
- `::before/::after` 伪元素叠层 → 显式嵌套 `Rectangle`。
- `:hover` → Slint `TouchArea` + `has-hover` / `states`。
- `[data-theme="dark"]` 变量覆盖 → `RedesignTheme.dark` 三元（已有）。
- `font-variation-settings` → 不需要，用预实例化 ttf。

## FR-T3 翻译注意事项（处方）

1. **呼吸**：第一个用到呼吸的组件（在场区头像）必须把 `animation-tick()` 算出的 scale 因子**真正绑到 `scale-x`/`scale-y`**，并 `desktop:dev` 肉眼确认在动——POC 只验了"算得出"，没验"接得上+看得见"。
2. **径向定位**：所有 `@radial-gradient(... at ...)` 的位置用 `parent.width`/`parent.height` 表达式，禁硬编码 px（否则窗口 resize 偏）。
3. **color-mix**：FR-T3 起若需大量混合色，扩 `docs/design/.../oklch-to-srgb.py` 把混合色算进 `redesign_palette.slint` 的 struct（重跑生成器），组件读 `RedesignTheme.t.<token>`；POC 的局部硬编码 hex 仅探针用，不进产品。
4. **验证命令环境债**：当前工作区 `pnpm run desktop:check` 因目录级 GNU override 的 gcc `0xc0000139`（DLL 地狱，ERRORS.md 已记根因）不可用；等价 verify = `CARGO_PROFILE_DEV_SPLIT_DEBUGINFO=off` + `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing`。FR-T3 真做前宜统一工具链或修 MSYS2 gcc，否则每次走 MSVC 绕路。
5. **暗色**：直接读 `RedesignTheme.t`，不要另起变量体系。

## 红线（翻译时仍守）

品牌 logo 不染 rep；思考块底不染 rep（abyss 冷）；沉积轮不染当前 rep；用户气泡不染 rep 边/底；正文不染 rep；z-index 类叠层用显式 Rectangle 顺序，无 `#app>*` 通配概念但同理勿用全局覆盖压住绝对定位元素。
