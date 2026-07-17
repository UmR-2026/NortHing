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
