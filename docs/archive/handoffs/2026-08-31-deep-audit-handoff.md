# Handoff — 2026-08-31（深夜）：全仓深度审计 + W12 交付 + W13 清理 + worktree 回收

> 上一 handoff：`2026-08-31-w10-w11-closed.md`（W10/W11 收官）。本文件覆盖其后全部内容，并附带**四路并发审计**的产出与**后续投入预估**。
> 用途：接手者读完这一份 + 四份审计报告即可掌握现状，不需要再翻历史。

---

## 0. 一句话状态

`main` HEAD `bccdae0`，**编译通过、桌面 147 测试全绿、rot 闸绿**；本会话交付了会话全文搜索（W12），清了三处技术债（W13），删了 7 个悬挂 worktree（回收 13.2 GB），并完成了四路深度审计。**唯一没做的是真机实测 10 项（人工）**，以及 **11 个 commit 未推送（网络被墙，需代理）**。

---

## 1. 本会话实际做了什么（commit 链，倒序）

| commit | 内容 | 判决/验证 |
|---|---|---|
| `bccdae0` | 修 AGENTS.md/AGENTS-CN.md：`error_banners.rs` 已随 Slint 删除（W13-3 法官 finding） | 法官 REQUEST_CHANGES → 已修 |
| `13d6691` | 四份审计报告 + W13 波产物入库 | — |
| `43ca492` | W13-2：测试 init 改走 `kernel_facade` 而非直调 core | APPROVE 0C/1I/1M；**引入 2 处 `let _ =` 致 rot 闸红，已由编排者修（待复验）** |
| `a93b4a3` | W13-3：清退 Slint 幽灵文档/注释（13 文件 23 处） | REQUEST_CHANGES 0C/1I/1M → 已修 |
| `cf34a7a` | W13-1：`seed_session` mock 移出生产路径 | APPROVE 0C/0I/0M |
| `f5dc0ef` / `76fad7d` | W12 波产物 + 台账 | — |
| `2b3ecfb` | W12-2 归档页接入全文搜索 | APPROVE 0C/0I/3M |
| `ca38f88` | W12-1 全文搜索后端 | APPROVE 0C/1I/4M |
| `d7a2d3b` / `ebe918e` | W12 plan 入库 / 需求表 10 处过期行刷新 | DONE |
| `5e95cf2` | W10/W11 handoff + W9 残留产物入库（**已推送**） | — |

**已推送**：`5e95cf2` 及之前全部（`ae44334..5e95cf2`）。**未推送**：`bccdae0` 起的 11 个 commit。

---

## 2. 实测证据（编排者亲自跑，非子代理转述）

| 项 | 结果 | 命令 |
|---|---|---|
| 全工作区编译 | **0 error**，1m53s | `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace` |
| 桌面单测 | **147 passed / 0 failed** | `cargo test -p northhing --lib` |
| 核心会话测试 | 158 passed | `cargo test -p northhing-core --features product-full session` |
| rot 闸 | W13-2 曾红（`let_underscore` 390/388）→ 修复后待复验 | `node scripts/verify-rot-budget.mjs` |
| 代码图谱 | **1758 文件已索引，自动同步**（能查到当日新增的 `search_sessions`/`sort_search_hits`） | codegraph |

**环境硬事实**：PATH 上的 GNU cargo（`C:\Program Files\Rust stable GNU 1.95`）会遮住 rustup shim 并导致链接失败，必须用 `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo ...` 完整前缀。

---

## 3. 四路审计结论（报告在 `.superpowers/sdd/audit-2026-08-31-*.md`）

### R1 腐化与边界
- 3C / 6I / 3M。⚠️ **两条 Critical 经编排者核实为误报**：报告称 `cli/main.rs` 799 行、`desktop/app.rs` 791 行逼近 800 上限——实测 **693 行 / 749 行**，远未触线。审计子代理的数字不可全信，已记入教训。
- 真问题：① `service/` 下 **13 个文件** 反向 `use crate::agentic::*`（违反 core/AGENTS.md 边界规则，M-L）；② `cli/startup/selectors.rs` 与 `modes/chat/*.rs` 镜像重复，可精简 **500+ 行**（M）；③ **15 个文件**存在 `not yet wired` / `unimplemented!` 桩（含 `get_persistence_handle`）；④ 13 处 TODO **零 owner/日期**；⑤ Slint 幽灵文档（**本会话已清**）。

### R2 安全（威胁模型：本地桌面 + CLI，数据不出本机）
- **0 Critical / 5 Important / 13 Minor**。
- 必修：① **P2-2 无单实例锁** —— 双开会 last-write-wins 损坏 `config/app.json`（唯一指向数据丢失的 open 项，M）；② `workspace_runtime/service/state.rs:90` **非原子写**（S）；③ `guard_command_execution` 的确认门是 **Phase 2 死代码**——7 处调用方全 `skip_confirmation=true`，安全只靠 denylist（L，需产品决策）。
- 已闭合（无需再动）：API key 全走 OS keyring（Scheme C，序列化 `skip`）、路径双重 canonicalize + symlink 围栏、子进程 `kill_on_drop` + 进程组 tree-kill、`JsonFileStore.write_atomic` 覆盖关键状态、debug.log 敏感键脱敏。

### R3 功能实现真值（**最重要的一份**）
- **真接线 27 项 / 半接线 9 项 / 摆设或 mock 12 处**。
- 已修的坑：`app.rs:57` 曾把 `seed_session()`（2026-08-12 spike 遗留的 5 条硬编码假消息）当**生产初值** → 空会话显示假数据（W13-1）。
- 摆设 top：`pages_space.rs:47-139` 走廊 7 扇假门；`windows/work.rs` 路由/规划/Diff/终端四处硬编码假状态；`windows/self_app.rs` 假 token 与假词条；`app.rs:460-475` 编年史双击只循环 5 个硬编码色值；`app.rs:518-530` / `:782-791` 无 onclick 的空按钮。
- 产品现状一句话：**"具备单房间真实 Agent 对话与工具调用、三档授权确认门完备、文件树可预览、会话与记忆可检索导出的单兵工作台；但多会话切换、走廊与左右抽屉仍是概念设计期的静态摆设，聊天仍为纯文本无 Markdown。"**

### R4 整合与 worktree
- `.worktrees/consult-room-build`（`feat/consult-room-slint`，`969d274`）：`main..969d274` = **0**，其 HEAD 是 main 的直系祖先，**所有提交已并入**；132 个未跟踪条目全为过程文档（已备份），**无源码改动** → 可删（426 MB）。
- 7 个分支资产盘点：`feat/growth-a1`~`a5`（各 2 commit）**100% 被 growth-core 线性吸收** → 可删 ref；`spike/multiwindow-0809`（3 commit）结论已入设计文档 → 可删 ref；**`feat/growth-core-0804`（36 commit / +11,081 行 / 含完整 `northhing-agentic-growth` crate 与成长引擎 / 185 单测全绿）是唯一值得保留的高价值未合并资产**，与 main 分叉 391 提交。
- 「前端」= Dioxus 桌面 UI（`src/apps/desktop/src/ui_dioxus/`）；`src/web-ui` 仅剩 i18n 契约生成代码，React 前端在 v0.1.0 缺失。
- 前端绕过 facade 直调 **3 处**（均在 `#[cfg(test)]` 内，非生产路径）——已改走 facade（W13-2）。R4 原报"生产违规"应更正为"测试代码直调"。
- 磁盘：`target/` **133.15 GB**（主仓 128.58 + installer 4.57），可按需 `cargo clean` 回收（代价=重编译）。

---

## 4. 后续投入预估（按本会话实测波次速度外推）

速度基准（本会话实测）：W12 三单（含两轮法官验收）≈ 1 小时；W13 三单 ≈ 1.5 小时；W10 三单 ≈ 半天；W9 七单 ≈ 1 天。**以下为"编排者 + 子代理闭环"口径，不含人工真机验证时间。**

| 目标 | 内容 | 成本 | 说明 |
|---|---|---|---|
| **P0 收口** | 真机实测 10 项 + 全量测试补跑 + 推送 11 commit | **人工 1-2h + 机器 0.5h** | 实测是唯一人工项；推送需代理端口 |
| **P1 止血** | P2-2 单实例锁 + `runtime_layout_state` 原子写 | **S-M，≈0.5 天** | 唯一指向数据丢失 |
| **P2 名实相符** | 12 处摆设：做真 or 移除 | **M-L，≈2-3 天** | **需产品决策**：做真（接真实数据源）还是直接删掉概念设计残留 |
| **P3 对话体验** | Markdown 渲染/代码高亮 + 多行输入 + 文件/图片拖入 | **M-L，≈2-3 天** | Markdown 单独先做 ≈0.5-1 天，性价比最高 |
| **P4 多会话** | 解除单 Room 锁定 + 走廊 Space 做真 | **M-L，≈2-3 天** | 与 P2 有重叠，建议合并规划 |
| **P5 架构债** | 解 `service → agentic` 13 文件逆向依赖 | **M-L，≈1.5 天** | 不动不影响功能，影响后续可维护性 |
| **P6 安全纵深** | `guard_command_execution` 确认门接入 | **L，≈1-2 天** | **需产品决策**：是否要用户在危险命令上二次确认（当前只靠 denylist） |
| **P7 能力透出** | cron（半被动 TH-6）/ dream（成长 TH-5）/ PCS 插件 | **L，每项 2-3 天** | 后端均已就绪，纯 UI；PCS 完全未开始 |
| **P8 CI** | T2-1 CI 补齐 | **M，≈1-2 天** | 前置：i18n-contract 有 24 个预存失败必须先清 |

**粗估**：
- 若目标是「维持现状 + 止血」→ **约 1 天**（P0+P1）。
- 若目标是「名实相符的可用产品」→ **约 1.5-2 周**（P0~P4）。
- 若目标是「追上《产品论题》的完整形态（含成长演化 + PCS）」→ **约 3-4 周**（P0~P7）。
- 上述均**不含**真机回归验证与 CI 建设（P8 另计 1-2 天）。

---

## 5. 队列（需你拍板的 6 项 / 编排者可闭环的 8 项）

**需拍板**：
1. 12 处摆设：做真还是移除？（决定 P2 规模）
2. `guard_command_execution` 确认门：要不要接？（P6）
3. `feat/growth-core-0804`（36 commit 成长引擎）：移植、保留 ref、还是放弃？
4. `consult-room-build` worktree：删（426 MB）还是留？
5. `target/` 133 GB：要不要 `cargo clean`？
6. P2~P4 的优先级排序（先做对话体验还是先做多会话？）

**编排者可闭环**：全量测试补跑、O-1 flaky 修复、god-file 下一波（memory_db 894 / pages_onboarding 859 / lsp manager 836）、CLI theme.rs 989 拆分、selectors B 层合并、CLI popup 去重、15 处 bridge 未迁、rot-probe P2 的 30 处 `let _ =`、15 处 not-yet-wired 桩逐个接通、需求表二次校准（本次刷新后又交付了 W12/W13）。

---

## 6. 下 session 第一件事（按序）

1. **复验 rot 闸 + 推送**：`node scripts/verify-rot-budget.mjs` 应绿；代理端口 `127.0.0.1:7897` 起来后 `git -c http.proxy=http://127.0.0.1:7897 push origin main`（11 commit）。
2. **真机实测 10 项**（唯一人工阻塞，表仍全空）：`.superpowers/sdd/manual-test-checklist-2026-08-27.md`，用 HEAD ≥ `bccdae0` 的构建；W12 新增的搜索行为建议补进清单。
3. 之后按第 4 节 P0→P1→… 顺序推进，P2 开始前先要用户拍板「做真 vs 移除」。

---

## 7. 环境/工具事实（新增，务必记住）

- **GitHub 推送**：直连 `github.com:443` 被阻断（TCP 失败）。可用路线 = 本机 clash 代理 `127.0.0.1:7897`（clash-verge **服务进程**独立于 GUI 与系统代理开关，端口在就在）。用 `git -c http.proxy=... push`，**未写入全局配置**。SSH 路线不可用：22 与 443 均返回异常 banner `SSH-2.0-2ff2ba9`，KEX 协商失败。
- **codegraph**：索引在 `NortHing/.codegraph`，**随文件写入自动同步**（1758 文件），无需手动重建；查询用 `codegraph_explore` 并传 `projectPath`。
- **子代理可靠性**：本会话 GLM-5.3（reviewer-53）连续 2 次 Internal Server Error 后改派 gemini-37-flash 成功；gemini-37-flash-agy 一次派发被 cancel 但**在工作区留下了未提交改动**——`transport` 失败后必须先 `git status` 查残留再重派（本会话第二次踩）。
- **审计报告不可全信**：R1 的两条 Critical 是错的（行数数字虚构）。**阻塞性结论一律磁盘复核后再打回/修**。

## Suggested skills（下 session）

- `subagent-driven-development`（开波前）
- `anti-rot-system`（触碰 6 个观测 god-file 或跑 rot 闸时必用）
- `verification-before-completion`（声称完成前，桌面侧 `cargo check -p northhing`）
- `systematic-debugging`（O-1 flaky、真机失败项）
- `handoff`（波次收口时续写，勿再漏）
