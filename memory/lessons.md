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
