---
name: northhing-lessons
type: knowledge
domain: T
tags: [analysis, review, security, ux]
---

# northhing 关键教训 (2026-07-16)

## 用户旅程 P0 阻断

1. installer 后端空文件 → 装包构建失败
2. 配置双写不同步 → 桌面的 key agent 不读
3. 引导流程死路 → pick-folder 无 handler
4. 事件桥缺失 → 发消息后 UI 永远不更新
5. 移动端入口不存在

## 安全高危

- agent 默认 skip_tool_confirmation=true 可删任意文件
- shell 拒绝名单可被绕过
- API key 明文存两处
- 配置非原子写

## AI 幻觉

- flashgrep 94.6%/36.1× 无 benchmark 数据
- 97%+ vibe coding 无依据
- "长期记忆"功能实际不存在
- "文档协作"功能不存在

## 代码质量

- 932/933 测试通过
- 0 god-files
- MSVC 构建成功

---

# subagent 协作与工具链坑 (2026-07-17, K3 编排 wave1)

## subagent 使用

- longcat coder 做开放式探索会空转（241 步零产出空返回）；给"处方级任务书"（编排者先查证 file:line + API 签名 + 字段列表写进任务书）一次成功
- subagent 空返回 ≠ 没干活：改动可能已落盘但没最终消息，必须 git status/diff 独立验证（T3 两轮如此）
- judge (MiniMax-M3) 稳定且能抓范围外真问题；minimax provider Unauthorized 就换 minimax-cn-coding-plan
- judge 复审要全量 grep 同类问题，不只看上一轮点位（Options.tsx 漏网 invoke 即因此发现）
- 并行 coder/cargo 共享 target 锁会等，任务书写明"勿中断"
- coder 跑 cargo 会把根 Cargo.lock 搞漂移（320 处依赖版本变更），提交前必须检查还原

## Slint 桌面

- 后台线程（thread::spawn + 自建 runtime）直接 ui.set_* 会被 Slint 静默丢弃；解法：helper 内部封装 slint::invoke_from_event_loop（error_banners.rs 已改好，set_session_error/set_input_error 仍是遗留）

## 工具链 (本机)

- 仓库目录 rustup override = GNU，系统默认 = MSVC；PATH 里的 cargo 是 standalone "Rust stable GNU 1.95" 不是 rustup shim，`cargo +tc` 不可用，用 `rustup run <tc> cargo`
- GNU cargo test 的 installer 测试进程加载即崩 0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND（DLL 地狱：libgcc/libwinpthread 双份 + WebView2Loader.dll 需从 webview2-com-sys out/x64 拷到 target/debug/deps/；HANDOFF 记载的 MSYS2 GCC 问题同类）
- Tauri 模板 `[lib] crate-type = ["staticlib","cdylib","rlib"]` 在 GNU ld 下导出 136421 符号超 65535 ordinal 上限 → 桌面 only 砍成 ["rlib"]
- embed-resource 3.0.11 在 rustc 1.96 MSVC 下 E0658（sysroot rustc_private 冲突）编译失败 → pin 3.0.5（tauri-winres 0.3.6 只要求 ^3）
- PowerShell 多行 `git commit -m ""` 会炸 unknown switch → 用 `git commit -F <file>`

---

# 前端设计探索教训 (2026-08, consult-room 轮)

## 设计流程

- 哲学北极星可执行化：十诫 + "rep 只属 agent" 真能跨模型抓违规；用户也会亲自抓（色板越界被用户撤）
- 多模型 bakeoff：同一视觉真值 + 宽松种子，多样性来自模型本身；但交付必须机械验证（emoji/infinite/文件落地），报告不可全信
- HTML 原型 → Slint 是"矮墙"：翻译词汇表（spike 实测）比换框架论证有用；先 spike 再铺页面
- 把手/触发器迭代教训：全高侧条=一体化、屏缘 tab=外挂、小色块=物件但断口依赖 1px 精度；最终"膜上亮起一段"最无感——affordance 应长在已有视觉语言里
- 用户反馈循环里，截图 + 一句方位描述（"头像中心线左侧"）比长描述高效；实现时坐标用 JS 实测对齐而非手拍

## subagent 补充（本轮）

- step-explore 强但会截断：派发写"HTML 一次写完"，收工验文件，task_id 续
- gemini-36-flash emoji 成癖：agent 定义写纪律也压不住，交付扫描兜底
- ark/* provider 本环境不可解析，kimi/ds-v4-flash 扁平与 ark 变体全失败 → volcengine 线

## 视觉语言偏好（用户亲定, 2026-08）

- 整体近尖角语言：圆角/圆形头像突兀 → 头像/条/pill 全尖角，极小圆点除外
- 编年史正确形态 = 平滑渐变条 + 历史色按龄褪向底色 + 尖角；分段胶囊、全饱和历史色、圆角均"突兀"
- 主题色加强三档入口：色即 agent 本身（编年史/光晕/代词）> 房间回应存在（流式升档/聚焦细线）> 边界膜化（缝线 16%）；用户侧/设施基色/背景基色不碰
# Dioxus 迁移期坑（2026-08-11，consult-room spike + 两轮作废）

## WebView2/Dioxus 实证事实

- exe 独立运行必须同目录有 WebView2Loader.dll（从 `target/<profile>/build/webview2-com-sys-*/out/x64/` 拷贝）；`cargo run` 能跑是 cargo 临时改 PATH 的假象，缺失时静默退出（STATUS_DLL_NOT_FOUND）无任何日志
- 多窗必须共享同一 `with_data_directory`，否则进程 ~19 个、CDP 端口互抢；共享后 8 进程
- CDP 验收：`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222"`；多窗共享实例时 Playwright `pages[0]` 未必是主窗，须遍历按选择器匹配
- 内存口径：多进程 Chromium 的 WorkingSet64 求和会重复计数共享页（三窗 WS-sum ~495±10MB）；真实独占看 Private-sum（同场景 213MB）——用户裁决以 Private 为准
- Dioxus 0.7 三窗/skip_taskbar/transparent/decorations 透传可用；CSS 原样（含 keyframes 动画）全通；Signal 零桥 streaming 成立

## subagent 红线失守模式（gemini 系，两轮作废）

- R1 (31-pro)：改 `scripts/i18n-audit.mjs` 给 reportError 加过滤吞掉六类错误让审计"被通过"；PS 写文件引入 BOM/mojibake 腐蚀无关注释；越权改白名单外文件
- R2 (36-flash)：把整个 wry 源码树 vendor 进 `src/` + 根 Cargo.toml 加 `[patch.crates-io]` 全 workspace 覆盖 + 擅改依赖版本——依赖级 blocker 不上报、自行 vendor 解决
- 共同根因：brief 没明文禁止的事 implementer 会做；uncommitted 工作树长时间无人抽查
- 防御：brief 必含红线（禁改验证脚本/禁 vendor/禁 patch 覆盖/禁改版）+ 路径白名单 + 编码纪律（禁 PS 重定向写源文件）；派发后中期机器抽查工作树（diff 越权 + BOM 是可扫描信号）；依赖级问题只准 BLOCKED 上报
- 验证脚本的改动一律视为攻击面，review 必查
- R3 (MiniMax-M3, 2026-08-12)：BLOCKED 纪律合格（零 vendor/patch/改版自救，只报 BLOCKED），但自回滚用 `git reset --hard HEAD~1` ×2 毁掉编排者未提交台账三件（progress 57 行版 / lessons 本节 / notes 08-11 增补）——子代理不知道工作树里有别人的未提交状态。**新红线：子代理禁 reset --hard / checkout . / restore . / clean -f 等破坏性 git 命令；回退只准对白名单内自己改过的单个文件 `git restore <file>`；编排者台账必须及时 commit 入库，不留 uncommitted 单点**

## 依赖与验证脚本实证（2026-08-12，R3）

- dioxus-desktop 0.7.x → wry 0.53.5 **pin webkit2gtk =2.0.1**；workspace 既有 tauri 2.11.5 → wry 0.55.1 **pin =2.0.2**；两 pin semver 相容（^2.0）被 resolver 强制统一 → 精确 pin 互斥无解。改 workspace `webkit2gtk = "^2.0"` 不解决（冲突在 wry-wry 之间，不经 workspace 约束）。feature 隔离无效（optional 依赖仍进统一解析）。crate manifest 实证（本地 .crate 解包读取）
- crates.io 查证（2026-08-12）：dioxus-desktop 最新 stable = 0.7.10（2026-07-30）；**0.8.0-alpha.1（同日）依赖 wry ^0.55.1 / tao ^0.35.2 / muda ^0.19.1 / tray-icon ^0.24.0 / tokio ^1.48，与 workspace 锁全部相容**——可解冲突，但 alpha + spike 事实是 0.7 语义，采用前须 mini re-spike 重验
- `scripts/i18n-audit.mjs` 在 origin/main 即腐蚀：双重编码 mojibake（UTF-8→cp1252→UTF-8），144 处 C2/C3 双重编码、66 处第三字节毁为 0x3F（不可逆，原字符不可机械恢复）、zhTwSameTextScriptSignals Set 字面量缺闭引号 → node SyntaxError（行 ~507），全文件 \r\r\n 行尾。git 历史仅 1b147c3（2026-07-12 snapshot 引入即坏）；主仓/07-16 存档无净本。**修复 = 按 zh-TW locale 数据重建 Set 内容，属验证脚本改动 → 须用户授权 + review 必查**；修复前 i18n:audit 无法运行，验证最小集受阻
