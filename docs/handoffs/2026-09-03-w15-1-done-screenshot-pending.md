# Handoff — 2026-09-03（凌晨收口）：W14-1c 切片 4/5 落地 + W15-1 全交付，截图未拍

> 上一份：`2026-09-02-w14-1c-progress.md`。本份覆盖：CI 校准 + 切片 4 双路 + 切片 5 解锁 + W15-1 仲裁/实现全链 + gemini 渠道事故。
> 写于时间将尽，用户要求收口。

---

## 0. 一句话状态

HEAD `4bf004c`，**W15-1b-1/1b-2 均已 commit 但未推送**（ahead 2），工作树剩 3 个未跟踪 SDD 工件（brief/package）待随台账 commit。
W14-1c：5 切片完成 4 + 切片 5 已解锁（serial 移 windows）；W15-1：仲裁 → 基础单 → 集成单**全部 APPROVE**，**唯一欠账 = 仲裁 §7#11 视觉回归截图未拍**（见 §4）。

---

## 1. 本段 commit（倒序）

| commit | 内容 | 验收 |
|---|---|---|
| `4bf004c` | W15-1b-2 集成单（三渲染点 + CHAT_MD_CSS） | minimax-m3 APPROVE 0C/0I/5M |
| `329cc8f` | W15-1b-1 基础单（markdown_render.rs + 19 测） | minimax-m3 APPROVE 0C/0I/7M |
| `c14171f` | 5a 台账+brief+package（已推） | — |
| `a81202d` | 5a fix：serial job 补 OpenSSL 步骤（已推） | 重审 APPROVE |
| `0f90e94` | 5a：CI 双轨移 windows-latest（已推） | 一轮 1C → fix |
| `c603688` | 4a/4b 台账+briefs+packages（已推） | — |
| `c678e6b` | W14-1c-4b C/D 锁纪律扫描（3 处违规补 ENV_LOCK+RAII） | APPROVE 0C/0I/4M |
| `09a0c69` | W14-1c-4a init gate 局部状态重写 | APPROVE 0C/0I/4M |

## 2. 用户决策 / 超时默认（可推翻）

1. **切片 5 ubuntu→windows**：decide 超时（600s），按 W2 先例执行推荐项 A——serial + parallel 测试轨都挪到 windows-latest。仲裁 §5#4 只钉「双 job 连续 5 轮绿」未钉 OS。ubuntu/macos 编译红 = 07-17 起预存账，未动。
2. **gemini 渠道中断换 qwf**：用户拍板（见 §5）。
3. W15-1 依赖准入（markdown crate）09-02 已批，选型仲裁闭环未上交。

## 3. CI 现状（c14171f 首轮已跑完，failure）

- ✅ windows 编译墙已过（5a 修复生效）；ubuntu/macos 编译照红（预存）。
- ❌ **windows 双轨测试 job 都在「跑测试」步红**——具体失败测试未知：**无 CI 日志权限**（gh 未装、网页需登录、API 匿名 403）。**下个 session 要么本地复现（`cargo test --locked --workspace`），要么让用户配 gh/token。**
- ❌ repo hygiene 红（预存，08-31 起）——本地复现根因：CI 上扫 HEAD commit 变更文件，SDD 工件（brief/report/台账）含 `E:\agent-project\...` 绝对路径必然触发。**治法候选：checker 加 .superpowers 排除，或工件脱敏——需仲裁/用户拍板。**
- ❌ core boundary 红（预存）——本地复现根因：`services-integrations/Cargo.toml:50` anyhow 非 optional 违反边界规则，**可直接修**（optional 化 + feature 挂接）。
- rot budget / i18n / kernel-api guard 绿。

## 4. W15-1 未闭环项（下 session 第一件事）

**仲裁 §7#11：3 张视觉回归截图（draft / assistant / witness）。**
- 本 session 末 `cargo build -p northhing` 在 PTY `pty_9823fe36` 里跑（日志 `C:\WINDOWS\TEMP\opencode\build-desktop.log`，BUILD_OK/BUILD_FAIL 哨兵）；session 重启后 PTY 失效则重跑。
- 流程：build → `Start-Process target\debug\northhing.exe`（detached，**绝不直接跑会阻塞 shell**）→ 等 ~20s → `powershell .opencode\tools\shot-window.ps1 -OutFile screenshots\w15-1-*.png` → 读图验证 → `Stop-Process -Name northhing`。
- 验收点：markdown 结构（标题/列表/代码块/链接）正常渲染、衬线正文与 .rec 布局不破坏、`.md-rendered` 的 pre-wrap 生效。
- 若用户会话里没有含 markdown 的消息，需造一条（真实发一条或在测试会话里写）。

## 5. 事故与教训（已入 ledger）

- **gemini 渠道中断**：W15-1b-2 连派 4 次——vertex 首派留半成品（保留 95% 被证实可用）后被取消；agy ×2 零产出（用户见「No subagent session id on task metadata」+ shell 4000s 假象）；用户拍板换 coder-qwf 一轮交付。**渠道故障时 qwf = 实证备胎（本波第 4 单全一轮）。**
- **派发返回异常先 `git status`/`git log` 磁盘核查**（再次坐实，半成品保住了）。
- **GUI 截图步骤是 coder shell 挂起高危点**（`cargo run` 桌面应用永久阻塞）——已从 coder 职责剥离给编排者。
- **pty_write 字符串里 `\r` 会被当回车**：路径含 `\bin\rustup` 会把命令切成两截——PTY 写命令路径用正斜杠；cmd 里 LF 不执行命令，要补 CR。
- **查杀残留进程前必须看命令行**：本机 3 个"老 cmd"实为 codegraph/workbuddy MCP 服务进程，杀错会断工具链。

## 6. 队列

1. **W15-1 §7#11 截图验收**（§4）→ 拍齐后 W15-1 才算完整闭环（代码层已双 APPROVE）
2. **CI 双轨测试红排查**：本地 `cargo test --locked --workspace` 复现 / 或拿 CI 日志权限；serial 5 轮连绿观测从测试转绿后开始计
3. **预存红治理（需拍板/仲裁）**：repo hygiene（.superpowers 路径豁免）、core boundary（anyhow optional 化）、ubuntu/macos 编译（6 周陈账）
4. follow-up：core 裸 feature 编译缺口（P2-15 形态，4a 发现）；`turn_persist.rs:546` 泄漏候选（3d 发现）；css_files.rs 孤儿（删除或注册，仲裁 §7#14 建议列 W15-2）；settings push 测试触真实配置目录（E 类）
5. W15-2 输入框多行+拖入 → W15-3 → W16 → …

## 7. 环境/雷区（全仍有效 + 新增）

- cargo 走 `C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc`（PTY 里用正斜杠防 `\r` 断行）；输出 cmd 重定向；构建一次跑二进制；先读 skill `long-running-shell`。
- 推送先试直连（本 session 直推成功），失败上 clash `127.0.0.1:7897`；SSH 不可用。
- rot 闸：let _ = 371/388、css.rs 790/790 等基线均未涨（本波全守）。
- progress.md 是 **GB18030 主导编码的混合文件**——追加中文必须用字节级 GB18030 append（脚本 `C:\WINDOWS\TEMP\opencode\ledger-append2.ps1` + ledger-text.md 范式），edit 工具直接写会混 UTF-8 岛。
- 宵禁 03:00。

## Suggested skills

- `long-running-shell`（任何构建/测试前）
- `subagent-driven-development`（派发）
- `verification-before-completion`（验收口径）
- `handoff`（下轮收口）
