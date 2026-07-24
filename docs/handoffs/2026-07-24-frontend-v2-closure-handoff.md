# Frontend v2 Design Closure Handoff — 2026-07-24

> 本 session（编排者 qwen3.8）覆盖**前端 v2 设计线 + 视觉自迭代闭环 + Slint 落地 POC**。与 `2026-07-24-session4-handoff.md`（M-P0-1 FTS5 / 构建环境修复线）平行，文件集不相交。新 session 按主题各取。

## 1. 需求基线
- 用户删了某 OD 设计项目需重做 → 改用 northing 定稿设计语言重建全部设计稿，锚点 `docs/design/2026-07-22-frontend-redesign/northing-home-v1-final.html` + 编年史模型 + `northing-frontend-design-handoff.md`。
- 用户拍板：代表色 = 全界面驱力色（非头像私有）；以 agent 为视觉中心（居中在场区）；整屋空气染色；暗色双皮肤；**人类除首次 onboarding 外不可改色**（控制权规则）。
- 外部 reviewer 称 HTML→Slint 是"系统性墙/建议换框架" → 编排者决策=路2 POC 实测，**排除换框架**。
- 用户立长期纪律："前端做好后截图 review，迭代到我认为最好再交付"。

## 2. 今日完成（northing main，本 session design commits）
| commit | 内容 |
|---|---|
| `8fcf113` | `redesign-v2-plan.md` 处方（v2 范式逐页清单 + chrome + 圆角 + 红线 + 真值声明） |
| `ca681cf` | 归档 9 页 v2 原型进 `prototypes/` + `prototypes/README.md` 索引；plan 补仓库副本指针 |
| `e9e1f3b` | `slint-feasibility-poc.md`（POC 结论 + CSS→Slint 语法对照 + FR-T3 处方） |
| `9a934c2` | 同步 `prototypes/theme-system.html` 定稿（亮 v2 + 暗 v4 ambient 自迭代，修暗色漏覆盖 #app 背景 bug） |

> 链上 `e465fb8` 为 session4 的 wip（FTS5），与本线文件不相交。
> OD 沙盒 9 项目（迭代用，归档基线在仓库 `prototypes/`）：`northing-theme-system`(范式真值) / `-self-cognition-onboarding` / `-empty-state` / `-set-a-general` / `-set-b-models` / `-set-c-ws-skills` / `-set-d-mcp` / `-set-e-access` / `-archive`。

## 3. 关键结论
- **Slint 落地 = 矮墙非天花板，换框架排除**。POC（`poc_v2_visual_probe.slint` + main.slint 三处标记块，已回滚退场）实测 9 项 v2 特性全过 slint-compiler。呼吸无限循环用 `animation-tick()`+`Math.sin()` 纯 Slint 可行（**证伪编排者"需 Rust Timer"预判**，已记 `.opencode/memory/.learnings/ERRORS.md`）。径向 `at` 只吃 px（绑 `parent.width/2` 解）、linear 要角度、color-mix 预计算——均机械可解。整体预期折扣=低。
- **视觉自迭代闭环**：theme-system 范式页亮 v1→v2（平涂改纵向衰减+体温收敛+气泡浮起）+ 暗 v3→v4（抓修 dark 块漏覆盖 #app 背景），每改一次 Edge headless 截+read 亲眼判，亮暗都达标才停。

## 4. 队列（blocking 边 + 并行可行性）
| 序 | 单 | 依赖/备注 |
|---|---|---|
| 1 | **FR-T3 组件骨架换绑**（Slint，照常翻译） | 依赖 T1✅+T2✅；第一个组件=空态在场区头像，**必带 `desktop:dev` 视觉走查**，顺手拧两螺丝：呼吸 scale 因子接到 `scale-x/y`、径向 `at` 绑 parent 表达式；暗色 ambient 对称覆盖（见 facts/northing-frontend-design.md） |
| 2 | FR-T4 空态出生态+懒建+sess-tag 菜单 | T3 |
| 3 | FR-T5 设置·通用页（显示模式接 RedesignTheme.dark） | T1✅，可与 T3/T4 并行（不同文件集，同 crate） |
| 4 | 档案馆 v1（Slint 设置页 nav + B9/B10✅ 后端） | T5 |
| 5 | 9 页 HTML 设计稿抽查验收 | 用户未再提，低优；编排者可 headless 截+read 独立验 |

## 5. subagent 运维（选派实证，本 session）
- **整页照抄/大输入机械单**：qw≈lc 稳（onboarding+空态、设置 A+B、C+D 全一次成）；**s37 弱区**（E+档案馆成，但 A+B、C+D 双空返回）；**srouter 不可靠**（一空一 aborted）；s35 仅极小观察单（重命名 1/1 未循环，样本1）。
- **视觉活**：派 qwen（s37/m3 视觉弱/空返回别派）。派单法=编排者先 headless 截图存盘 + 任务书让 read png + 派 qwen。
- 已蒸馏进 `.opencode/memory/facts/models.md` + `ERRORS.md`。

## 6. 已知雷区
- **并发 git 铁律**：本仓多 session 并发活跃（session3/session4 同写 northing + memory）。commit 前必 `git diff --cached --name-only` 复核 staged 全量，只 add 显式路径，永不 `git add -A`。本 session 每次 commit 均复核、零卷入。memory 仓 `episodes/2026-07-24.md` 与本 session + session4 相交，已逐行核对双方内容都在、零丢失。
- **暗色 headless 截图陷阱**（连踩两脚，记 `.opencode/memory/facts/visual-verification.md`）：① `--force-prefers-color-scheme=dark` 对靠 JS 设 data-theme 的页无效；② 写死 `data-theme="dark"` 仍被初始化 `setMode(matchMedia…)` 删掉，须**同时**文本替换 `setMode('dark')`。判据=暗色截图字节数≠亮色截图，否则暗色没生效。
- **gcc 环境债**：`pnpm run desktop:check`/`desktop:dev` 因 GNU gcc `0xc0000139` 受阻。两绕法并存：① MSVC + `CARGO_PROFILE_DEV_SPLIT_DEBUGINFO=off` + `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing`；② `$env:PATH="C:\msys64\mingw64\bin;"+$env:PATH`（session4 记，让新 gcc 优先绕旧 libgcc 劫持）。FR-T3 跑 `desktop:dev` 验渲染前先选一条修通。
- **Slint 验证**：headless 截不了原生 GUI，Slint 渲染须 `desktop:dev` 跑起来截窗口；静态截图验不了呼吸/过渡动效。
- **redesign_palette.slint 是生成器产物勿手改**；color-mix 预计算色正式入 palette 须扩 `oklch-to-srgb.py` 重跑。

## 7. 新立长期纪律（跨 session）
- **前端视觉自迭代交付闭环**：写进 `.opencode/memory/facts/conventions.md`。任何前端产物交付/归档/派翻译前：改→headless 截→read 亲眼判→迭代到编排者认为最好才停；多态全验（亮+暗+关键交互态）；动效诚实标注；subagent 产物不自信仍自截自看；OD 定稿同步回仓库 `prototypes/` 并 commit。
- 视觉验收工具链见 `.opencode/memory/facts/visual-verification.md`（三进路 + Edge headless 命令模板 + 边界）。

## 8. Suggested skills
`preflight-skill-check`（每回合）→ `verification-before-completion`（宣称完成前截图取证）→ `handoff`（收尾）。视觉走查用 headless 截图+read，不依赖 judge 视觉。

## 9. 一句话状态
前端 v2 设计线闭环：9 页原型归档进仓库 + 范式页亮暗双态自迭代定稿 + Slint POC 证"矮墙非天花板/换框架排除" + 视觉自迭代交付立为长期纪律；FR-T3 绿灯待开（首组件带 desktop:dev 走查拧呼吸/径向两螺丝）；本 session northing/memory 我的部分全 commit 干净、并发零卷入。
