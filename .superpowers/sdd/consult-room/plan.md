# 实施计划 — consult-room Slint 建构期（FR-T3b 收束）

> 2026-08-04 · 编排者 K3。分支 `feat/consult-room-slint`（基线 main 8e43dc4），
> worktree `E:\agent-project\northing\.worktrees\consult-room-build`。
> 前置：设计方向 consult-room 定稿；用户终裁五页套**全部按现状通过**（`consult-room/FINAL-RULING-20260804.md`）；
> spike 词汇已验（`slint-feasibility-consult-room.md`），mind 25 token 已入 palette（T0 已移植）。

## 1. 目标

把 consult-room 五页套（唯一视觉真值 `consult-room-main.html` + 四面板 v2）逐页翻译进桌面端 Slint
（`src/apps/desktop/src/ui/`），双光学（暗/亮）成立，shot-window 截图循环验收。不做后端接线改动；
数据仍走现有 Rust 绑定，视觉层可用 mock/既有数据呈现。

## 2. 视觉真值（唯一需求来源，建构期不得偏离）

| 页 | 规格文件（docs/design/2026-07-22-frontend-redesign/consult-room/） |
|---|---|
| 主诊室 | `consult-room-main.html` |
| onboarding | `consult-room-onboarding-v2.html` |
| settings | `consult-room-settings-v2.html` |
| archive | `consult-room-archive-v2.html` |
| space | `consult-room-space-v2.html` |

系统继承总纲：`PANELS-BRIEF.md` §1（chrome/deck/中枢/抽屉/呼吸纪律/戒律）。
翻译词汇：`slint-feasibility-consult-room.md` + `slint-retarget-notes.md` + `prototypes/slint-safe-conventions.md`。
判断基线：`visual-iter-compass_20260802.md`（戒律 §2 优先；与 consult-room 真值冲突处以真值为准——真值更新）。

## 3. Global Constraints（逐任务生效，brief 逐字复制）

1. **Slint 翻译红线**：禁 box-shadow（用 drop-shadow 或线+底色阶，spike 判 B 为默认）；禁运行时
   color-mix（预计算 hex，调色走 `oklch-to-srgb.py` 生成器重跑，勿手改 palette 派生值）；
   禁 @keyframes infinite 新增；渐变仅线性/径向；border-radius 不吃 %（圆=定值 px）；
   Rectangle 无 scale-x/scale-y（呼吸一律绑 opacity，animation-tick + Math.sin）。
2. **哲学红线**：rep 只属 agent（用户/见证者侧不染边/底）；禁 dashboard 数字；禁 emoji；
   品牌水印化（印章 opacity ~0.25）；8s 单钟呼吸、振幅分级（主体>边界>结构）、不新增 infinite；
   近尖角语言（头像/条/pill 方形化 radius 0，极小圆点除外）；编年史右端 ≡ 界面强调色同源。
3. **i18n**：新增 UI 文案走 i18n 契约（`node scripts/generate-i18n-contract.mjs` 重跑并一起提交）；
   日志 English-only。
4. **纪律**：禁裸 `cargo fmt`（会卷无关文件；如需格式用 `pnpm run fmt:rs` 或手工对齐）；
   implementer 每任务**恰好一个 commit**（message 由 brief 指定），不得多 commit/不得碰无关文件。
5. **验证最小集**（每任务必跑，report 附命令+输出）：
   ```powershell
   $env:PATH = "C:\msys64\mingw64\bin;C:\msys64\usr\bin;" + $env:PATH   # opencode shell 不加载 profile，必须前置
   $env:CARGO_TARGET_DIR='E:\agent-project\northing\target'
   $env:CARGO_PROFILE_DEV_SPLIT_DEBUGINFO='off'
   cargo check -p northhing    # GNU 工具链（repo override）；MSVC 链接在本机不可用
   ```
   UI 改动必须附改动前后截图（暗+亮双光学）：`desktop:dev` 等价
   `rustup run stable-x86_64-pc-windows-msvc cargo run -p northhing`（同环境变量），
   截图用 `powershell -NoProfile -File 'C:\Users\UmR\.local\share\opencode\worktree\16ba4143154c219fe7f43650ae6f4d297aa32c23\visual-iter\.opencode\tools\shot-window.ps1' -OutFile <path>`（独立进程，防 Add-Type 冲突），
   点击导航用同目录 `click-window.ps1`；全屏应用遮挡时先最小化遮挡窗口。截图落 `docs/design/2026-07-22-frontend-redesign/consult-room/build-shots/`，
   **不 commit**（编排者终审统一处置），report 给绝对路径。
6. **探针纪律**：spike 探针（poc_consult_probe.slint / build.rs 探针段）留在 spike 分支存档，
   本分支不得引入。

## 4. 任务分解

| # | 任务 | 范围 | 验收 |
|---|---|---|---|
| T1 | chrome 与系统层 | WindowChrome 重制（废标题栏→brand-inline 状态行 + 页面身份；窗控四键 ☀─□✕ 入主体区右上；印章收进房间左下）；containment + membrane-frame 双边界（parent 表达式，禁硬编码 px）；room-fog 沉积底雾（AirTint 演进：底染+顶晕+底雾，archive 冷雾保留）；呼吸 8s 单钟全局范式（opacity 绑定、振幅分级）；头像方形化（AvatarWrap 等 radius 0）；ChronicleBar 定稿（尖角 4px opacity .7、历史色按龄褪向出生灰、右端 ≡ mind-base 同源变量） | main 路由挂载可见；暗/亮截图；check 过 |
| T2 | 主诊室 | `consult-room-main.html` 全量 → ChatPaneView/main.slint：room-head 可收纳胶囊（顶晕染色+状态 pill+▴收纳）；deck 合一按钮（空闲➤/流式■）+见证注右对齐+上下文聚焦收纳；membrane-node 触发器（左=头像中心线 mind 辐射/右=背景相反色辐射，位置与弹出同步，TouchArea+root 坐标拖移）；主题色三档（缝线 16% mind 色、speaking 整屋升档、agent 代词着色）；对话流语义（活跃轮暖竖线、用户气泡右对齐不染、思考块冷左缘、chip 暖→冷） | 暗/亮 + 抽屉开合截图；戒律自检；check 过 |
| T3 | onboarding v2 | `consult-room-onboarding-v2.html` → WelcomeView/IdentityCreatorView：灰寂→选色着色唤醒仪式；大五色板=人类唯一改色入口；chrome 与真值同步；provider 测试态 | 暗/亮截图（含选色前后）；check 过 |
| T4 | settings v2 | `consult-room-settings-v2.html` → SettingsView + 5 面板：「它的自我」mind 着色只读（沉积不带 rep）/「设施」中性分治；CONTEXT 段收纳语法；aura 按真值（无锚浮屏中上） | 暗/亮截图；check 过 |
| T5 | archive v2 | `consult-room-archive-v2.html` → ArchiveView：abyss 冷雾领域；12 地层透明度递降；节气轴；沉积地层禁 rep（仅 chrome 参与 mind 派生）；统计叙事化（文字不用数字） | 暗/亮截图；check 过 |
| T6 | space v2 | `consult-room-space-v2.html` → SpaceView：走廊门语法；亮门独占 rep/光晕/呼吸；换房=灯移门；沉积门 opacity 阶梯禁 rep；叙事量纲计数；新房=开一间 | 暗/亮截图；check 过 |
| T7 | 终审 | 整分支终审（MERGE_BASE..HEAD）+ 五页套双光学截图走查 + 戒律十条逐条 + 合并决定 | 双判决 PASS 才可合并 |

依赖：T1 → T2 → {T3,T4,T5,T6}（顺序执行，不并行派发）→ T7。
跨任务接口：T1 定稿系统组件 API（WindowChrome/AirTint/ChronicleBar/AvatarWrap/双边界/呼吸常量），
T1 report 必须列出最终 API 清单；后续任务 brief 引用 T1 report + 代码现状，不得重新发明。

## 5. 已知环境事实（省时间）

- `cargo check --workspace` 被上游 embed-resource 3.0.11 阻断（非代码问题）；一律 focused `-p northhing`。
- 新 worktree 缺 gitignore 生成文件 → 先 `node scripts/generate-i18n-contract.mjs`（T0 已跑）。
- 双 `slint_build::compile_with_config` 共存时 SLINT_INCLUDE_GENERATED 后者覆盖前者（本分支单 compile，无需处理）。
- mind 五色 × 双主题 25 token 已在 `redesign_palette.slint`（T0 移植自 spike）。
- 路由切换：`main.slint` 的 `current-route`（main/welcome/settings/archive）；截图导航可 click-window 或临时 dev 入口（implementer 自决，临时物不得入 commit）。

## 6. 审查纪律

- 每任务：implementer（显式模型）→ review-package（BASE=派发前 commit）→ judge 双判决
  （spec 合规 + 代码质量）→ Critical/Important 派 fixer（原 task_id 续会话）→ 重审 → ledger。
- Minor 记 ledger 进终审 triage；plan-mandated finding 交用户裁决。
- 终审用未参与任务审查的独立模型。
