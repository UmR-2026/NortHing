# 后续执行计划 — 2026-09-01（基于 2026-08-31 全仓深度审计）

> 上游：`docs/handoffs/2026-08-31-deep-audit-handoff.md`（现状总结 + 投入预估）
> 依据：`.superpowers/sdd/audit-2026-08-31-{rot-boundary,security,feature-truth,integration}.md`
> 本文件是**可执行**的任务拆分（波次 → 任务 → 验收），投入预估在上游 handoff §4。

## 0. 开工前置：6 项待用户拍板（未拍板的不许开工）

| # | 待拍板 | 影响 | 默认建议 |
|---|---|---|---|
| D1 | 12 处摆设：**做真**（接真实数据源）还是**移除**（删掉概念设计残留） | 决定 W15 规模（2-3 天 vs 0.5 天） | 走廊 Space + 左右抽屉先**移除**（它们不是 PRD 需求，是概念期残留），设置页假能量条改真 |
| D2 | `guard_command_execution` 确认门：要不要接（危险命令二次确认） | W18 是否开工 | 接——但先只接"高风险命令"子集，不做全量拦截 |
| D3 | `feat/growth-core-0804`（36 commit / +11,081 行 / 成长引擎 crate / 185 单测全绿）：移植 / 保留 ref / 放弃 | W19 是否开工 | 保留 ref 不动，等 TH-5 产品需求明确后再定 |
| D4 | 优先级：先做**对话体验**（Markdown/附件）还是先做**多会话** | W15/W16 顺序 | 先对话体验（日常使用频次最高，成本低） |
| D5 | `consult-room-build` worktree 删不删（426 MB，零独有 commit，产物已备份） | 磁盘 | 删（备份已在 `C:\WINDOWS\TEMP\opencode\worktree-backup-2026-08-31\`） |
| D6 | `target/` 133 GB 要不要 `cargo clean` | 磁盘 vs 重编译时间 | 不 clean（下次开波还要用，重编译 2-4 分钟/次不值当） |

---

## 1. W14 — 止血与收口（≈0.5 天，无需拍板，优先做）

| 任务 | 内容 | 验收 | 成本 |
|---|---|---|---|
| W14-1 | **O-1 flaky 定论**：`test_delete_provider_default_provider_rejected` 连跑 5 次统计失败率；若确认 flaky，把 `TEST_GLOBAL_CONFIG_MUTEX` 覆盖扩到 `kernel_facade().upsert_model_config(...)` 路径 | 5 次全绿 或 给出根因修复 | S |
| W14-2 | **P2-2 单实例锁**：`config/app.json` 读改写加进程级锁（唯一指向数据丢失的 open 项） | 双开不再 last-write-wins；有测试 | M |
| W14-3 | **非原子写补齐**：`workspace_runtime/service/state.rs:90` 的 `runtime_layout_state.json` 改走 `write_bytes_atomic` | crash 中断不产生半文件；有测试 | S |
| W14-4 | **全量补跑 + 推送**：`cargo check --workspace` + 全量 test（补 W11/W12/W13 后的证据链缺口）；代理端口起来后推 14 个 commit | 全量绿 + `git status` 与 origin 同步 | S（机时 0.5h） |

## 2. W15 — 对话体验（≈2-3 天，D4 优先级确认后开工）

| 任务 | 内容 | 验收 |
|---|---|---|
| W15-1 | **Markdown 渲染 + 代码高亮**（CH-02）：消息渲染从纯文本改为 Markdown，代码块高亮 | 渲染正确、无 XSS 风险（本地渲染，禁止 raw HTML 注入） |
| W15-2 | 输入框多行 + 文件/图片拖入（CH-07/CH-08） | 可拖入、可预览、进入上下文 |
| W15-3 | 重新生成（CH-05）+ 编辑消息（CH-06） | 两条链路可用，会话持久化一致 |

## 3. W16 — 摆设处置（≈0.5-3 天，**依赖 D1**）

| 任务 | 内容 | 验收 |
|---|---|---|
| W16-1 | `pages_space.rs:47-139` 走廊 7 扇假门：按 D1 结论做真（接真实会话列表）或移除 | 无硬编码假数据 |
| W16-2 | `windows/work.rs` 路由/规划/Diff/终端四处硬编码假状态 | 接真实 Agent 状态、Git 差异、token 消耗 |
| W16-3 | `windows/self_app.rs` 假 token/假词条；`app.rs:460-475` 编年史假色循环；`:518-530`/`:782-791` 无 onclick 空按钮 | 接真数据或移除空按钮 |
| W16-4 | `pages_settings.rs:383-394` Card 2 假能量条 | 接真实上下文消耗 |

## 4. W17 — 多会话（≈2-3 天，与 W16 有重叠，建议合并规划）

| 任务 | 内容 |
|---|---|
| W17-1 | 解除单 Room 锁定：`ROOM_SESSION_CACHE` 失效机制（`api.rs:121` ponytail 注释已标升级路径） |
| W17-2 | 手动新建会话入口（SE-01）+ 会话切换 UI |

## 5. 排期队列（无阻塞，穿插执行）

| 波 | 内容 | 成本 | 备注 |
|---|---|---|---|
| W18 | `guard_command_execution` 确认门接入（**依赖 D2**） | L 1-2 天 | 安全纵深，当前只靠 denylist |
| W19 | 解 `service → agentic` 13 文件逆向依赖（建 Runtime Port） | M-L 1.5 天 | 不动不影响功能 |
| W20 | selectors B 层合并（减 500+ 行）+ CLI theme.rs(989) 拆分 + god-file 下一波（memory_db 894 / pages_onboarding 859 / lsp manager 836） | M 1.5 天 | 与 rot 闸零余量直接相关，**任何桌面大改的前置** |
| W21 | 15 处 `not yet wired` 桩逐个接通 + 13 处 TODO 补 owner + rot-probe P2 的 30 处 `let _ =` | M 1-2 天 | |
| W22 | 能力透出 UI：cron（TH-6）/ dream（TH-5）/ PCS 插件 | L 各 2-3 天 | **依赖 D3**；PCS 完全未开始 |
| W23 | T2-1 CI 补齐 | M 1-2 天 | 前置：i18n-contract 24 个预存失败 |

## 6. 执行纪律（沿用本仓既有，不得简化）

- 每任务：brief 文件 → 派 coder → `git diff <parent> <commit> -- <点名文件>` 生成审查包 → 派 judge 双判决（SPEC + QUALITY）→ Critical/Important 必返修 → 通过后**立即**追加 progress.md 台账行并 commit（不攒到波末）。
- 涉 Rust 任务的 brief 必须附 `.opencode/templates/rust-brief-block.md` 全文。
- **审查包 BASE 取 implementer 的父 commit**，不要用跨 commit 的范围 diff（会混入并行任务的改动——2026-08-31 由 judge 指出）。
- 子代理返回后**一律磁盘复核**（`git log`/`git status`/读 diff），尤其带具体行数的阻塞性结论——2026-08-31 审计 R1 报的「main.rs 799 / app.rs 791 行」实测为 693 / 749，纯属虚构。
- 子代理会话被 cancel 后，**先 `git status` 查工作区残留再重派**（2026-08-31 W13-2 踩）。
- rot 闸：不上调任何 ceiling；`let _ =` 计数 388/388 零余量，**修 bug 时不要引入新的静默吞错**，也不要靠改名规避（本仓有"闸口游戏回滚"先例）。
- 宵禁 03:00；长构建走 PTY 后台 + 轮询，不要同步死等。

## 7. 交付顺序建议

`W14`（止血，0.5 天）→ `W15-1`（Markdown，性价比最高）→ 视 D1/D4 决定 `W16` 或 `W15-2/3` → 期间穿插 `W20`（给后续大改腾出 rot 余量）。
