# Handoff — 2026-09-01：六项决策落地 + W14-1 重新定性 + PowerSkills 安装

> 上一 handoff：`2026-08-31-deep-audit-handoff.md`（全仓深度审计 + W12/W13 + 投入预估）。本文件覆盖 09-01 全天，接手者读这份 + 计划文件即可开工。

---

## 0. 一句话状态

`main` HEAD **`aed78fa`**，工作区干净，**ahead 18 未推送**（代理端口在的话一条命令可推）。今天把六项决策（D1–D6）落到计划里、删除了最后一个悬挂 worktree、把 W14-1 从"flaky 小修"**重新定性为测试隔离改造**、装了 PowerSkills 的 `system` + `desktop` 两个技能并验证可用。

---

## 1. 今天的 commit（倒序）

| commit | 时间 | 内容 |
|---|---|---|
| `aed78fa` | 20:37 | W14-1 改为测试隔离方案，成本 S→M，W14 总计 0.5d→1.5d |
| `8492fc2` | 16:38 | W18 命令风险预估 + 白名单制度设计（市面产品调研） |
| `4348fb3` | 16:38 | 后续计划 v2：D1–D6 六项决策入册，波次重排 |
| `8c8c99e` | 03:29 | 后续计划 W14–W23 初版 + 6 项待拍板 |
| `a4462b7` | 03:21 | W13 波法官判决书入库 |
| `46c4bf7` | 03:20 | 深度审计 handoff |
| `34b1998` | 03:20 | 修 W13-2 引入的 rot 违规（`let _ =` 390→388） |
| `bccdae0` | 02:53 | 修 AGENTS.md/AGENTS-CN.md 事实错误（W13-3 法官 finding） |

---

## 2. 六项决策（用户 2026-09-01 裁决，已入计划）

| # | 议题 | **裁决** | 落地 |
|---|---|---|---|
| D1 | 12 处摆设 | **做真**（不移除） | W16 = 接真实数据源，2-3 天；无数据源可接的元素**删掉**而非造假 |
| D2 | 命令安全门 | **风险预估 + 白名单制度**，参考市面产品 | 设计已出：`.superpowers/sdd/w18-command-risk-design.md` |
| D3 | growth-core-0804 worktree | **不开常驻 worktree** | 保留分支 ref；移植时用临时 worktree |
| D4 | 优先级 | **单对话优先** | W15 先做，W17（多会话）后置 |
| D5 | consult-room-build worktree | **删**（已执行） | 见 §3 |
| D6 | target/ 133 GB | **看情况，先排风险** | 默认不 clean；磁盘 449.5 GB 不紧张 |

### 2.1 D2 调研结论要点（9 个产品）

- **Claude Code**：`Deny→Ask→Allow` 三级 + 通配符 + 配置合流 + OS 沙箱。
- **Cursor**：LLM 分类器（Auto-review）/ Allowlist / YOLO 三模式。
- **Cline**：`allowedCommands` 前缀数组，**无 AST，易被 `&&`/`;` 复合命令绕过**（反面教材）。
- **Aider**：每次 shell 必确认，靠 Git 自动提交回滚兜底。
- **Continue / Zed / Windsurf / Gemini CLI / Codex CLI**：三档到四档不等，Zed 与 Codex 走 OS/内核沙箱（Seatbelt / Landlock）。
- **推荐**：**L0 只读免确认 / L1 常规（白名单+确认门）/ L2 高危（强制确认、禁永久加白）/ L3 灾难（硬阻断）**四级；白名单存 `GlobalConfig`（守"配置单一事实源"铁律，不新增第二个配置文件）；会话级授权只放内存。
- **最关键的一条**：模型**能被诱导自己改配置文件提权** → 需要 `Protected Path Fence`：禁止任何 Bash/FileWrite 工具路径写 `~/.northhing/`。
- 迁移 3 步，4~7 人天。只有 Gemini CLI 的 `safe-commands.toml` 完整词法规范没找到公开资料。

### 2.2 D3 判定理由

与 main 分叉 **391 提交**（main 侧 1906 文件变动），差距只会继续拉大，放着不"保值"；一个 worktree = 一份独立 `target/`（本仓单份 128 GB 量级）；当前优先级也用不到成长引擎。真要移植时 `git worktree add --detach` 开临时 worktree 即可。**另注**：该分支的 185 个单测是 08-07 的状态，"当时全绿"不能当现在的证据。

---

## 3. 已执行的清理

- **`consult-room-build` worktree 已删**（D5）。删前风险排除：① 备份完整（未跟踪 378 文件 / 19.81 MB）；② 审计确认 132 个未跟踪条目**零源码改动**（全是过程文档）；③ 追加了一次整目录备份（2234 文件 / 76.5 MB，排除 node_modules）到 `C:\WINDOWS\TEMP\opencode\worktree-backup-2026-08-31\consult-room-build-FULL\`。
- `git worktree remove` 首次报 "Directory not empty"（残留目录），用 `Remove-Item -Recurse -Force` 清掉，空的 `.worktrees/` 目录一并删除。
- **分支 ref 一个没删**（8 个分支仍在）。
- 磁盘：**449.5 GB** 空闲；`target/` 133 GB 按 D6 未动。

---

## 4. ⚠️ W14-1 重新定性：不是 flaky，是测试间全局状态污染

**这是今天最重要的发现，改变了任务性质。**

实测（测试二进制 `target/debug/deps/northhing-7cec78aa9cf51e26.exe`，单次约 0.6 秒）：

| 模式 | 结果 |
|---|---|
| 默认（并行）×5 | 1 次失败（20%）—— `test_delete_provider_default_provider_rejected` |
| `--test-threads=1`（串行）×5 | **5 次全失败（100%）**—— 失败的是**另一个测试**：`ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized`，panic 于 `api.rs:172` `assert!(res.is_err())` |

**根因**：`kernel_facade()` 的 `static FACADE: OnceLock<...>` 一旦被任何测试初始化就**永不重置**，`GlobalConfig` 的 default provider 同理。凡是断言"未初始化时必须报错"或"默认 provider 必须被拒绝删除"的测试，只要排在会初始化全局状态的测试之后，就**必然失败**。并行模式下它们抢跑赢了 80%，所以历史上一直被误判为"~25% flaky"。

**O-1 原假设（mutex 未覆盖 `upsert_model_config` 路径）只对一半**：加锁解决不了"必须在未初始化状态失败"与"会初始化全局状态"两类测试共存于同一进程的问题。

**三个方案**（推荐 A，计划文件 §1.1 有完整表）：
- A 迁到独立集成测试目标（每个 integration test 文件 = 独立进程）—— 干净，代价是可见性要从 `pub(crate)` 提到 `pub`
- B 加 `#[cfg(test)] unsafe fn reset_facade_for_tests()` —— 改动小但有 unsafe
- C 断言改宽容 —— **不推荐**，等于藏问题

**成本重估**：W14-1 由 **S（0.5 天）→ M（≈1 天）**，拆为 a 侦察 / b 设计裁定 / c 实施 / d 验证；**W14 总计 0.5 天 → 1.5 天**。

---

## 5. PowerSkills 安装（裁剪版）+ shell 纪律

**位置**：`C:\Users\UmR\.agents\skills\powerskills\`（只装 `system` + `desktop`，保留 `powerskills.ps1` / `config.json` / `lib/bootstrap.ps1`，**未装** outlook / workiq / browser）

| 技能 | 状态 | 说明 |
|---|---|---|
| `system exec` | ✅ | **有硬超时**：`WaitForExit(timeout)` → 超时 `Kill()` + 返回 `exit_code 124`。默认 30 秒，必须显式 `--timeout N` |
| `system info` | ✅ | 冒烟通过，JSON 信封正常 |
| `desktop screenshot` | ⚠️✅ | 首次失败 → 根因见下 → 修复后通过（2048×1152） |

**`desktop` 的坑（已写进 skill 文档）**：本机会话 `TEMP=C:\WINDOWS\TEMP`，`desktop.ps1` 用 `Add-Type @"..."` 现编 C#，编译器临时 `.cs` 写不了 → 报"未能找到源文件"。**调用前必须覆盖**：
```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
```
修复后截图已用视觉工具确认截到真实桌面（OpenCode 模型设置页）——**真机 UI 验证这条路通了**，W9-6 那类只能交 SVG mockup 的情况以后可补真图。

**新立的 skill**：`.opencode/skills/long-running-shell/SKILL.md` —— 记录本仓 shell 四条硬纪律（rustup 前缀 / 构建一次跑多次 / **cmd 重定向而非 PowerShell 管道** / PTY 轮询）+ 判错表。起因是今晚 5 次长等待（500s/600s/3000s/4800s/8100s）。

---

## 6. 实测证据（编排者亲自跑）

| 项 | 结果 |
|---|---|
| `cargo check --workspace` | 0 error，1m53s |
| `cargo test -p northhing --lib` | 147 passed / 0 failed（**但串行跑必挂一个**，见 §4） |
| rot 闸 | 绿（`let_underscore` 388/388、sdd 380/400） |
| 代码图谱 | 1758 文件，自动同步 |
| worktree | 仅剩主工作树 1 个 |

---

## 7. 队列

**已定、可直接开工**：
- W14 止血：W14-1 测试隔离（新方案）/ W14-2 单实例锁 / W14-3 非原子写 / W14-4 全量补跑 + 推送
- W15 单对话体验（D4 优先）：Markdown 渲染 → 附件 → 重新生成/编辑
- W16 摆设做真（D1）
- W17 多会话（后置）
- 穿插 W20（god-file 拆分 + rot 腾余量）

**待拍板 / 依赖**：
- W14-1 的方案 A/B/C 二选一裁定（可交独立子代理仲裁）
- W18 命令风险门：设计已出，开工前确认 L0–L3 分档与白名单格式
- W22 能力透出（cron/dream/PCS）依赖 D3 后续
- D6：磁盘真紧张时再评估 `target/`

---

## 8. 下 session 第一件事（按序）

1. **推送 18 个 commit**：代理端口 `127.0.0.1:7897` 在的话 `git -c http.proxy=http://127.0.0.1:7897 -c https.proxy=http://127.0.0.1:7897 push origin main`（端口不在就先等）。
2. **W14-1a 侦察**：扫出全部依赖全局状态的测试，出清单（grep `before_init` / `uninitialized` / 对 facade 的 `is_err()` 断言 / `set_default_provider` / `TEST_GLOBAL_CONFIG_MUTEX` 使用者）。
3. **W14-1b 设计裁定** → **W14-1c 实施** → **W14-1d 验证**（新目标 5 次 + 全量并行 5 次 + 全量串行 5 次）。
4. 之后按 D4 走 W15-1（Markdown 渲染）。

---

## 9. 环境/工具事实（新增，务必记住）

- **推 GitHub**：直连 `github.com:443` 被阻断；走本机 clash 代理 `127.0.0.1:7897`（clash-verge **服务进程**独立于 GUI 和系统代理开关）。**SSH 不可用**：22/443 均返回异常 banner `SSH-2.0-2ff2ba9`，KEX 失败。
- **cargo 必须 rustup 前缀**：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo ...`（PATH 上 GNU cargo 会遮住 shim 并链接失败）。
- **重复跑测试**：先 `cargo test --no-run` 构建一次拿到二进制路径，再直接跑二进制（0.6s/次），不要每次调 cargo（2-5 分钟/次）。
- **PowerShell 管道会永久阻塞**：测试派生子进程继承 stdout 句柄 → 用 `cmd /c "... > log 2>&1"`。
- **长命令后查残留进程**：`Get-Process cmd,powershell` 里启动超 20 分钟的要清掉——今天清了 3 个挂 4 小时的 `cmd.exe`，它们就是 shell 反复变慢的元凶。
- **子代理会编数字**：审计 R1 报的「main.rs 799 / app.rs 791 行」实测 693 / 749。阻塞性结论一律磁盘复核。
- **子代理被 cancel 会留未提交改动**：先 `git status` 查残留再重派（W13-2 踩过）。

## Suggested skills（下 session）

- `long-running-shell`（**跑任何构建/测试前**）
- `subagent-driven-development`（开波前）
- `anti-rot-system`（触碰 6 个观测 god-file 或跑 rot 闸时）
- `verification-before-completion`（声称完成前）
- `powerskills-system` / `powerskills-desktop`（长命令加超时保险丝 / 真机截图）
- `handoff`（波次收口时续写）
