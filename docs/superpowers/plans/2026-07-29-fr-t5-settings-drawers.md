# FR-T5 设置统一 + 抽屉外扩 + 外物重做

> 2026-07-29 用户目验 FR-T4 后拍板。前置：FR-T4 代码完成（judge PASS），22 commit 待 push（网络不通）。
> 设计真值：`docs/design/2026-07-22-frontend-redesign/prototypes/`（theme-system.html CSS 唯一 + settings-*.html 5 页）。
> 纪律沿用 FR-T4：任务书→coder→judge-m3/glm；无 hex；padding 只挂 layout；Flickable 用 preferred-height；palette 只走生成器。

## 用户拍板（2026-07-29）
1. 抽屉要**窗口真实变宽**的外扩，不要向内遮蔽（无遮罩）。
2. **右抽屉「外物」= 生成物 / 浏览器 / subagent worktree 等非 agent 自身的东西**（用户此前已明确定位）；Skills/MCP 收进设置；主题切换入设置通用页。
3. 设置页要统一成新设计（用户看到旧 GUI：工作文件夹页未迁移 + 旧壳/nav）。

## W1 设置统一（先行，纯 .slint，收益最大）
- [ ] **T5-1 设置壳重做**：照 settings-*.html 共有壳——左 nav 样式（选中态/字号/间距）、头部、关闭按钮位置与样式、淡雾壳（air/halo-rep-settings 已有 token）。现状：旧 Material nav + 底部「关闭」大按钮。Files: SettingsView.slint。
- [ ] **T5-2 工作文件夹页迁移**：WorkspaceSettingsPanel 照 settings 系列壳范式重做（添加文件夹/列表/IDENTITY.md 状态/删除确认 modal 已用 scrim token）。设计稿无专页 → 照 settings-general 壳范式 + 现有功能保留。
- [ ] **T5-3 五页校订**：AI 服务/技能/MCP/通用/访问权限按截图+prototypes 复核数值（T4-6a/6b judge PASS 后的目验级校订：间距/卡片层次/空态）。
- [ ] **T5-4 收纳确认**：Skills 开关、MCP 管理在设置页功能完备（已存在）；右抽屉对应入口删除（随 W3）。

## W2 抽屉外扩（架构级，先 POC）
- [ ] **T5-5 方案+POC**：Slint 窗口真实变宽 320px——Rust 侧 winit window set_inner_size + 动画（帧驱动插值）；frameless 自绘 win-ctrl 下的 resize 行为；主内容像素不动、抽屉面板占扩展区、无遮罩。POC 验收：开窗动画顺滑、内容不跳、最小宽度约束、双抽屉同开=左右各扩 320。
- [ ] **T5-6 全面铺开**：InnerDrawer/OuterDrawer 去遮罩改外扩形态；把手 z 序；设置/archive 路由时把手可见性（截图显示设置页边缘仍露出把手，需明确：路由页全覆盖 vs 把手常驻）。
- 风险：Slint 1.16 window API 能力边界（winit 透传）；动画掉帧则降级为瞬时变宽。

## W3 右抽屉「外物」重做
- [ ] **T5-7 收摊**：删 Skills 列表/MCP 状态行/主题切换（主题入设置通用页——已有，确认链路）；「会话设置→」入口去向拍板（保留 or 入设置）。
- [ ] **T5-8 外物空态**：生成物/浏览器/subagent worktree 三类占位空态（功能未存在）；等浏览器/生成物功能落地再接。
- [ ] **T5-9 deck `/` 调 skill 列表**：新交互（输入框 `/` 弹出 skill 选择）——依赖 skill 调用链路设计，可独立成单。

## W4 杂项（顺手/小单）
- [ ] **T5-10 设置页顶部怪 glyph**（截图右上角 win-ctrl 旁多一个 □ tofu 框）排查。
- [ ] **T5-11 降级项收尾**：deck-bar access 文案「自治·完全」、think 段数对真值、WindowChrome 把手 gap 10px + hover 背景。
- [ ] **T5-12 onboarding 四字段+5 色板拍板**（用户未决；现 5 轮 Q&A 保留中）。

## W5 功能断点（探索式 review 2026-07-29 发现，编排者取证确认）
- [ ] **T5-13 Identity Creator Rust 接线**（P0，onboarding 卡死）：`llm-generate(question_id, answer, current_draft)` 无 handler（点了永久 spinner）、`on-saved(draft)` 无落盘。需：接现有 provider LLM 调用通路生成 identity 草稿 + 保存写 IDENTITY.md（sett­ings/sync.rs 通路）。Files: IdentityCreatorView.slint 透传 + callbacks/*.rs + app_state。属 Rust 功能单，任务书需先列通路设计。
- [ ] **T5-14 export-markdown Rust 接线**：ArchiveView/InnerDrawer 4 处调用无 handler。实现：汇总当前会话消息 → markdown 文本 → rfd 保存对话框写文件（rfd 已在依赖，WelcomeView pick-folder 同款）。
- [ ] **T5-15 open-session-settings 去向设计**（5 处调用无 handler）：会话级设置面板尚不存在——先拍板形态（modal？设置页会话分区？），暂可将 OuterDrawer 入口置 disabled+TODO 防裸奔。
- [x] ~~FTS trigger CJK bug~~ **驳回**：review 误报。trigger 复制 `facts.text_fts` 列，插入路径已 `segment_for_fts`（memory_db.rs:64/87/213 取证），两条路都是分词文本。
- [ ] **T5-16 housekeeping**：callbacks_lifecycle.rs 917 行（距 1000 拆分线 83 行）下次触及先拆；AGENTS.md 补 K4a invariant（northstar K5 要求，滞后 3 天）。

## 选派
- W1：glm（壳/大页）+ ling（校订小改）；W2 POC：glm 或编排者亲自（Rust+架构）；W3：ling；W4：bp/mimo 机械单。
- judge：m3 首选 / glm 备选。⛔ qw 无额度停派。

## 验证
每单：`$env:CARGO_PROFILE_DEV_SPLIT_DEBUGINFO='off'; rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing` 零 error 零新 warning；运行用 `CARGO_TARGET_DIR=target-msvc` + PATH 前置 MSVC toolchain bin（run-desktop.ps1 已落在 temp）。
