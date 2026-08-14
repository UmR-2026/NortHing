# Consult-Room Slint 建构期 Progress Ledger

计划：`.superpowers/sdd/plan.md`
分支：`feat/consult-room-slint`（worktree `E:\agent-project\northing\.worktrees\consult-room-build`）
基线：8e43dc4 (main, 2026-08-04)；终裁：五页套全部按现状通过（FINAL-RULING.md）
**最新接续点：`handoff-20260814b.md`——R4-R9 视觉收口完成（用户裁定「主窗口满意」，HEAD=5bcc285，R4-R9 改动未 commit 候裁定，flags.rs 已 restore=false）；R3' 功能层清零 + F1-F5 验收批已入库**

| Task | 状态 | Commits | 备注 |
|---|---|---|---|
| T0 setup: complete (orchestrator) | 基线 cargo check -p northhing pass（7m53s）；spike 资产移植（palette 25 mind token + oklch-to-srgb.py + tokens-srgb-table + feasibility doc；探针未移植）；i18n 已生成 | e487cd8 + 5ea3f6a + c1e107c + 748031c（被 amend 收拢，见 FYI）|
| T1 chrome 与系统层: complete (双判决 PASS + 4 Important 修复完 + 复审 PASS) | gemini-31-pro (Implementer) + gemini-31-pro (fixer round 1+2)；judge-m3 双判决 PASS（spec+quality 0 Critical / 4 Important / 7 Minor + 1 FYI）；fixer round 2 处置：API 清单补全 / drag 备注 / BOM 去 + LF 统一 / SpaceView 28px 让位移除；重截暗亮；commit amend 至 748031c | 748031c（FYI：commit 含 SDD 文档+截图，留终审清理）|
| T2 主诊室页: complete (双判决 PASS，4 Critical + 关键 Important 全部回炉修复 + 复审 PASS) | 主路由 SpaceView 移除 PresenceZone（让位给 ChatPaneView 内 RoomHead）；新增 RoomHead / DoorbellGem / MindMod / WorkMod 4 组件并实例化；ChronicleBar 双击换色绑定（`now` 接入渲染）；DeckBar 合一按钮；theme 3 档（缝线 16% mind / speaking 整屋升档 / agent 代词着色）；mock 会话流（agent / tool / chip / witness / approval）；状态点 8s sin 振幅；30+ 字符串迁 AppStrings；RoomHead TouchArea fold 入口；状态行去 wordmark（brand 全交给 T1 水印）| ac86998（被 amend 至 1eace31，round 2 修复内容入）|
| T3 onboarding v2: complete（PASS with reservations；I1 Timer 延迟跳转 / I2 user+relation title 已修；Minor 留 T7） | qwen 系未参与；implementer 模型见 task-03-report | a40c765 |  |
| T4 settings v2: complete（双判决 PASS 0C/0I，3 Minor 留 T7 triage） | implementer=general(qwen3.8-max-preview，视觉读图 10 张)；judge=minimax-m3；三页浮层(接入点/MCP/技能)+上下文主页面常显+无全屏页；搜索 Rust 子串过滤=批准偏离（§9.7） | 1c83616 |  |
| T2c lane B (deck/chrome/编码): complete (代码正确, 截图有重名复制) | implementer=gemini-31-pro; 4 files (-5 净) 改完 commit; B-1 stretch fix + B-2 Path send/stop + B-3 字标去重 (保水印) + B-4 mojibake 注释清理; `cargo check` exit=0 (2m14s)。**verify 警示**: 6 张截图中 chrome-dark / deck-dark / pill-dark 三张 SHA256 相同（重命名复制），pill 不在主路由渲染（仅用于死代码 ProviderSettingsPanel.slint）；streaming-dark 视觉差异未明显。code 0 回归 (Pill diff 仅注释) | a1fc9d9 |
| T2c lane C (RoomHead 补件): complete（随 P0 收口 commit） | height 锁 72/198 + avatar-mid-y 36/67 + chronicle bar 4px；report-C 已补写 | 753d777 |  |
| Task P0 阶段 0 收口: complete (commits eee21ca..8566ef3, review clean 0C/0I/0M) | implementer=gemini-36-flash；reviewer=gemini-36-flash(reviewer)；5 步全 PASS：Lane C 收口 753d777 / 19 草稿删除 8f2f2ae（4 页 v2 真值已保留）/ 6 死面板 8566ef3（cargo check exit 0）/ 真值 5 页字节级 UTF-8 核验 / merge-tree vs origin/main **0 冲突**（0808 的 io.rs 预警已被时间消解，reviewer 独立复核成立）/ 罗盘在位（0808 §4.2 假警报关闭） | 753d777 + 8f2f2ae + 8566ef3 |  |
| Spike 多窗口验证: complete（S1 PASS / S2 PASS带证据缺陷 / S3 PASS / **S4 FAIL 降级** → **A1 可行**） | spike/multiwindow-0809 分支存档（f521e40+6eabce9+feee4e2，不合入）；implementer=gemini-31-pro（初版证据缺陷把关打回后修复合格）；reviewer=gemini-36-flash（视觉亲读 7 张截图）；**产品化要点：浮窗不透明底 + room 窗内 scrim（S4 降级路径）；HWND 走 winit raw-window-handle（禁 FindWindowW）；禁 mem::forget/process::exit(0)** | spike 分支（8566ef3 切出） |  |
| 0xC0000139 启动崩溃修复 | complete（manifest 嵌入 ComCtl32 v6）| northhing.exe.manifest + northhing.rc + build.rs | 9e3405d |
| T1-fix brand 回改（C4 裁定）: complete (commit b978d28, review PASS，1 Important 修复闭环) | implementer=gemini-36-flash；reviewer=gemini-31-pro（视觉）；WindowChrome 印章撤除 + ChatPaneView 状态行 brand-inline 归位（三环 logo 5 Path + Fraunces italic 12px，opacity .7）+ plan §3 戒律改「品牌入状态行，不独立成印」+ WelcomeView 注释更正；修复轮：嵌套 HorizontalLayout 默认 horizontal-stretch=1 形成隐形空泡推远状态项 → 加 horizontal-stretch:0 闭环（复审亲读确认） | b978d28 |  |
| 真值 HTML 小编辑批: complete (commit 5c86e3e, review PASS clean 0C/0I/0M) | implementer=gemini-36-flash；reviewer=gemini-31-pro；C3 state-dot 三页补回（archive 补 CSS+实例，main/onboarding 补实例启用死 CSS）/ C4 archive 删 brand-seal 实例+CSS / V1 main witness 右缘 2px 中性线 / F7 archive pill「沉积 · 只读 · 缓」/ settings 日→☾；Edge 渲染抽查双页布局完好 | 5c86e3e |  |
| T5 archive v2 | 待派 |  |  |
| T6 space v2 | 待派 |  |  |
| T7 终审 | 待派 |  |  |
| T-IO-b + roomfix 合并视觉验收 | **CONDITIONAL PASS**（编排者逐张亲读 16/16 + SHA256 查重独立重跑，证据真实）：三窗并泊/竖签显隐/拖动跟随/最小化联动/任务栏无污染/主窗亮 token/welcome A4 无溢出 全 ✅。3 疑虑处置：C1 settings 不可达 = 与 F4 同根，偏离成立，补证挂 F4；**C2 亮色下 inner/outer 不随主题 = Important 真缺陷**（RedesignTheme global 每 Window 实例独立，无同步通道），派 fixer + 亮矩阵补证后关闭；C3 任务栏 0 图标 = 编排者双轮对照实证为 Windows 配置行为（副屏窗口无主任务栏按钮），非缺陷。执行者=k3-vision（报告 task-tiob-accept-report.md）；编排者裁定书 task-tiob-accept-verdict.md | —（纯证据采集，无 commit） |  |
| T-IO 三窗制落地（inner/outer 独立 OS 窗） | **代码侧关闭**：`935292f` + fix1 `91211f6` + fix2 `27fdac5`（✕ 归位）+ 竖签呼吸表达式化随 `548c53f` 夹带入库（roomfix 误并，编排者核验正确后披露）；R3 复审 PASS（I4/I5 核销，gemini-36-flash reviewer）。剩 **T-IO-b 视觉验收**（与 roomfix 补证合并派发，brief `task-tiob-accept-brief.md`）。教训：gemini-31-pro 空返回 ×2、DONE 虚报 ×2 | 935292f + 91211f6 + 27fdac5 + 548c53f(部分) |  |
| roomfix 核对修复批（V1/A4/A1a/A1b） | **代码已入库待视觉补证**：commit `548c53f`（witness 右缘中性线 / 标注 9→10px 全量 / membrane 静息 0.08 / dark frame 55%→70% 生成器重跑）；reviewer=gemini-31-pro 一审 FAIL——**gemini-36-flash 证据失实第 4 起**（4 张截图实为同一暗色 main，文件大小几乎相同），代码本身无 finding；补证并入 T-IO-b 验收。另：`create_ui.rs` 工作树碎屑（误缩进）已 git restore 清除 | 548c53f |  |
| T1 Dioxus 路线 spike | **complete（双判决 PASS + 用户裁决 go）** | implementer=gemini-31-pro（首轮环境归因失实打回→真因 WebView2Loader.dll 未随 exe；两轮修复合格）；reviewer=gemini-36-flash（一审 FAIL 3I 中证据失实→重审 PASS 0C/0I/0M）。全通：三窗+属性透传 / 六 CSS 探针含 keyframes 动画 / Signal 零桥 / CDP+Playwright / 转写 16 行 RSX≈5min。内存 WS 490-505 / Private 213 → 用户裁决 Private 口径通过。坑：多窗共享 with_data_directory（进程 19→8）；exe 须携 WebView2Loader.dll；CDP pages[0] 未必主窗 | spike 仓外 temp（git 8757012+修复轮） |  |
| room 迁移 R1/R2 | **作废（已 revert）** | R1=gemini-31-pro：停摆+削弱 i18n 审计门+BOM 腐蚀+越权；R2=gemini-36-flash：vendor wry 源码进 src/+patch.crates-io 覆盖+擅改依赖版。防御已入 brief §5 红线+白名单，坑已入 memory/lessons.md。R3 待派（先测 mimo/M3） | —（无 commit） |  |
| room 迁移 R3（minimax-m3，2026-08-12） | **BLOCKED（纪律合格，两 blocker 待用户裁定）** | mimo 不在本实例子代理清单；M3 探针 DONE 合格后承接。Blocker-1 依赖级：dioxus-desktop 0.7.10→wry 0.53.5 pin webkit2gtk =2.0.1 vs workspace tauri 2.11.5→wry 0.55.1 pin =2.0.2，semver 相容强制统一而精确 pin 互斥 → 无解（crate manifest 实证；M3 提议 workspace ^2.0→=2.0.2 不解决问题，冲突在 wry-wry 之间）。编排者查 crates.io：**dioxus 0.8.0-alpha.1（2026-07-30）依赖 wry ^0.55.1 / tao ^0.35.2，与 workspace 锁一致可解**（alpha 风险 → 建议先 mini re-spike 闸）；备选=拆独立 exe 保 0.7.10。Blocker-2：scripts/i18n-audit.mjs 在 origin/main 即腐蚀（双重编码 mojibake、66 处第三字节毁为 0x3F、Set 字面量缺闭引号 → node SyntaxError），git 历史/主仓/07-16 存档均无净本，验证最小集受阻。M3 纪律：零自救（禁 vendor/patch/改版遵守到位）、唯一 commit 仅白名单 flags.rs、untracked 保留 ui_dioxus/ 8 件 + truth CSS 抽取 20397B；报告 task-migrate-room-report.md。**事故**：M3 自回滚用 `git reset --hard HEAD~1` ×2，毁编排者未提交台账三件（本文件 57 行版/lessons Dioxus 节/notes 08-11 增补）——已凭编排者上下文读全量重建（notes 为据证重建），台账自此 commit 入库；子代理禁破坏性 git 命令已入 lessons。**用户 08-12 裁定：依赖走 dioxus 0.8.0-alpha.1 + mini re-spike 闸；审计修复授权** | 9144013（flags DIOXUS_SHELL=false+回归测试） |
| i18n-audit 腐蚀修复（minimax-m3，2026-08-12） | **complete（spec PASS / quality PASS，I-1 修复闭环）** | implementer=minimax-m3（续会话 fixer 轮）；**reviewer 通道全线故障（google token 刷新失败 / volcengine+ark model not found / step-explore 空返回×2）→ 编排者脚本化字节级亲审，独立性受限已披露**。66 条被毁 Set 字面量按两字节前缀+简繁语义+recovered locale 证据重建（前缀 66/66 全等、68 互异、区间外零字节改动）；I-1=开/检 两条 mojibake 存活条目 Set.has 永不命中 → fixer 轮复原本字。node --check exit 0；audit exit 1 的 154 error 均 pre-existing（locale 树 GBK 型腐蚀症状 + installer JSON 同毁 + 基线漂移，FYI 归用户）。报告/审查：task-audit-repair-report.md / task-audit-repair-review.md | a07c968 + c706b09 |
| dioxus 0.8.0-alpha.1 mini re-spike | **GO（8/8）** | 用户亲跑最小验证（三窗/六 CSS 探针/Signal streaming/CJK 截图）+ M3 补验四项：CDP 三窗匹配 GO / 共享 data dir 进程 8 GO / Private-sum 219–226MB（0.7 基线 213，+3% 噪声带内）/ exe 独立运行 GO 且 **WebView2Loader.dll 已静态链接**（0.7 的 dll 拷贝步骤可删，正向偏离）。锁统一 wry 0.55.1+tao 0.35.3+webkit2gtk 2.0.2（与 tauri 侧同，冲突消解实证）；0.7 main.rs 零改动编译运行 = API delta 零。报告 task-respike-dioxus08-report.md | spike 仓外 temp |
| room 迁移 R3'（minimax-m3，08-13 凌晨） | **进行中→停摆（待续）** | 3 笔白名单 commit：d1a7540（Cargo.toml ui-dioxus + =0.8.0-alpha.1）/ b805033（lib/main 接线 + 启动分支）/ 727f899（ui_dioxus/ 8 件 1500 行）；flags.rs 本地 flip 未提交（纪律对）。app 起 Dioxus shell 后 room 窗 runtime panic `Encountered panic: Any { .. }`（chrome 正常 → RoomApp 渲染路径 unwrap，怀疑 i18n.rs 加载腐蚀 .ftl）；M3 01:41 后停摆，报告/截图未产。续作处方见 handoff-20260813 §3 | d1a7540 + b805033 + 727f899 |
| OpenChamber v4f 派发坑（08-13） | **会话废置** | session.create 传 directory=consult-room 实际落 clever-toucan 且 idle 零产出（ses_008a26287ffegu8DX344VU38e6）；directory 参数未生效。复用须先核目录 | — | implementer=minimax-m3（续会话 fixer 轮）；**reviewer 通道全线故障（google token 刷新失败 / volcengine+ark model not found / step-explore 空返回×2）→ 编排者脚本化字节级亲审，独立性受限已披露**。66 条被毁 Set 字面量按两字节前缀+简繁语义+recovered locale 证据重建（前缀 66/66 全等、68 互异、区间外零字节改动）；I-1=开/检 两条 mojibake 存活条目 Set.has 永不命中 → fixer 轮复原本字。node --check exit 0；audit exit 1 的 154 error 均 pre-existing（locale 树 GBK 型腐蚀症状 + installer JSON 同毁 + 基线漂移，FYI 归用户）。报告/审查：task-audit-repair-report.md / task-audit-repair-review.md | a07c968 + c706b09 |

## 架构裁定登记（2026-08-09/10，详见 truth-rulings-20260809.md）

- **三窗制（深夜追加，最新）**：主页面 = room + inner（它的自我）+ outer（身外之物）**三个平级
  OS 窗口，默认并泊可见**；宝石/竖签切换显隐（竖签仅在隐藏态出现于 room 框缘）。
  HTML 的 mod-hidden/position:fixed 是单窗媒介近似，非设计意图。已回写 block-contract
  §0/§1/规则13/§3.1/§3.2/§5-TBD6、rulings §G.0（A2/F1/F2 随之微调）、html-truth-review 头部警告。
- ⚠️ **spike S2 验证的是单侧重栏跟随**（spike 跑于三窗裁定前/并行）；双侧栏并泊是同一机制的
  小扩展，在 inner/outer 实现任务 brief 中补验（不另开 spike）。
- 分块契约 `block-contract.md` v0.2（规则 1-13 + 逐块）；spike 结论见 `phase-1-decisions.md`
  （S1/S2/S3 PASS，S4 FAIL→降级不透明浮窗+主窗 scrim）；**决策 A1 待用户最终确认**。
- 真值审查 `html-truth-review-20260809.md`（含编码假警报勘误：本机 PS 中文管道不可信，
  判定须字节级）；渲染走查 `truth-visual-critique-20260809.md`；brief 模板 B-1..B-8 生效于
  后续所有任务。

## T2 备注
- 主路由 avatar 重复问题已由编排者修（SpaceView 移除 PresenceZone）。
- light 截图现已捕获（点击 ☀ 1156,22 → 整屋切 light token 配色）—— I1 解决。
- 状态行/room-head drag 接线仍未做（沿用 T1 FYI，留 FR-T3）。
- 抽屉内容 mock 4 项 / 3 项；T4 settings 抽屉可一起细化。
- 复审新观察项 m11/m12/m13/I4/不可判读 3 入 ledger 候 T7 终审 triage。

## FYI 终审清理清单
- e311aeb / 748031c commit 误捎 SDD 文档（plan.md / progress.md / task-01-brief.md / task-01-report.md / task-01-review-brief.md / task-01-review.md）+ 截图（build-shots/*.png）；合并前 `git rm --cached` + 单独 "chore(consult-room): 剥离 T1 误捎" commit。
- I-4 部分（SpaceView 内部 layout 仍可能微调）留 T2 处理完整 room 居中/双抽屉布局重构。
- Minor m-1（亮色 line token 缺失）可由 T2 追加 palette token 顺手收尾。
- Minor m-2（WindowChrome.signal in property 残留）可在 T2 顺手清理。
- **T-IO-b 验收遗留（2026-08-11）**：① C2 修复中（主题同步 inner/outer）；② settings 补证挂 F4（deck 设施条任务）；③ Minor：inner shown 态 ExStyle 0x00040118（APPWINDOW 残留、TOOLWINDOW 被 winit 覆写），当前任务栏配置下无影响，换机/改配置后 inner 可能上任务栏 → T7 triage；④ 环境事实（构建走 GNU 1.95 cargo 直跑 / MainWindowHandle 漂移 / app.json BOM 坑 / welcome 触发三条件 / 副屏任务栏行为）见 verdict §4，后续 brief 必须采用。

## R4-R7 视觉收口轮（2026-08-14，编排者亲自执行，派发通道 cancelled 用户指示）
- **状态：完成，未 commit**（用户视觉迭代仍在进行；改动文件：ui_dioxus/{app,css,entry,i18n}.rs + flags.rs DIOXUS_SHELL=true 脏状态待复核结束后 restore）。报告：task-r4-visual-report.md。
- R4：frameless（decorations=false + undecorated_shadow + with_menu(None) 去 dioxus 默认菜单）+ ─□✕ 真窗控 + 房间拖动；W2-W5 按 brief。
- **最大根因：真值 css UTF-8 BOM** → `:root` 永不匹配 → 全部 :root 变量失效（宝石/渐变/边框隐形）。css::truth_css() strip BOM，守卫测试不受影响。
- R5 去嵌套（用户判决）：room 铺满、containment/membrane-frame 移除、outer 四独立卡。
- R6：竖签/见证说明元素级移除、挂载→⊕ 图标钮、主题钮→内联 SVG 日月、宝石反对角线轴对称（--gem-mid=85px 逻辑；**截图物理 px ≠ CSS 逻辑 px，K≈1.44，描边实测标定**）。
- R7（先商后做）：收纳钮 → 骑 room-head 下缘 56×3 边缘条（折叠态常亮橙、随缝上移可展开）；宝石 → 实心细柱+box-shadow 光晕；**完全贴边根因 = UA body margin 8px（真值无 reset），body{margin:0} 三窗铺满**。
- 功能实测：宝石切换✅ □最大化✅ ─最小化✅ ✕整进程退出✅ 主题三窗同步✅ CPU 0%✅。
- 门禁：build exit 0（warnings 基线 lib19/bin40 不增）、cargo test -p northhing ui_dioxus 过、i18n:audit 154 持平。
- 工具坑：脚本点击在窗缘 8px 内被 tao hit_test 缩放边吃掉；窗内控件脚本验证有效。
- 下一轮候选（已报用户）：右宝石 dark 态提亮 / 见证者消息右缘孤线 / ➤ 发送钮 / 模块「隐蔽·拉开」交互入口（真值 data-drag 设计未定）。
- R8（同日追加）：scrim 常暗 = 色差根因 → 退役（偏离 block-contract §2 降级形态，用户判决）；宝石/缝条存在感标定（柱 4px、is-open .72、缝条 68x4 --muted）；主题同步实测两次翻转三窗跟随，用户 image-4 瞬态非 bug。仍未 commit。
- R9（同日追加）：窗控压虚线根因 = .room-status 无带高 → padding:10px 18px（带高 34 逻辑）+ .room-controls top:3px 带内居中；state-dot 呼吸点删除，8s 呼吸钟移植头像渐变（breath-avatar-fill ::before / dark glow / light ring，no-preference 门控，保留真值缩放呼吸同钟）。r9a/r9b 两帧间隔 4s 头像区 102 采样点差异实证动画运行。门禁全守。仍未 commit，待用户视觉裁定。
- **用户裁定（08-14 夜）：「主窗口满意」→ R4-R9 视觉收口完成**。flags.rs 已 restore 回 false（工作树清洁）。接续文档 handoff-20260814b.md 已写。**已入库：源码 commit `a202028`（5 文件 +431/-79）；台账/报告/handoff 由本笔 docs commit 收（hash 见 git log）**。
