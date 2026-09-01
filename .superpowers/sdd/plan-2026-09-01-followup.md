# 后续执行计划 — 2026-09-01（基于 2026-08-31 全仓深度审计）

> 上游：`docs/handoffs/2026-08-31-deep-audit-handoff.md`（现状总结 + 投入预估）
> 依据：`.superpowers/sdd/audit-2026-08-31-{rot-boundary,security,feature-truth,integration}.md`
> 本文件是**可执行**的任务拆分。投入预估在上游 handoff §4。

---

## 0. 六项决策（用户 2026-09-01 裁决）

| # | 议题 | **裁决** | 影响 |
|---|---|---|---|
| D1 | 12 处摆设怎么办 | **做真**（不移除） | W16 = 接真实数据源，2-3 天 |
| D2 | `guard_command_execution` 怎么接 | **风险预估 + 白名单制度**，参考市面现有产品 | W18 先出设计：`.superpowers/sdd/w18-command-risk-design.md` |
| D3 | `growth-core-0804` 要不要单开 worktree | **不开常驻 worktree**；保留分支 ref，移植时用临时 worktree | 见 §0.1 |
| D4 | 对话体验 vs 多会话谁先 | **单对话优先** | W15 先，W17（多会话）后置 |
| D5 | `consult-room-build` worktree 删不删 | **看具体情况，先排除风险** | 不急删；删前必须确认备份完整 |
| D6 | `target/` 133 GB 要不要 clean | **看具体情况，先排除风险** | 默认不 clean；按目录粒度评估，不整体 clean |

### 0.1 D3 展开：为什么不需要给 growth-core-0804 单开 worktree

- **现状**：分支 ref 全部保留（36 个 commit 都活着），但**没有 worktree**——2026-08-31 删的是 worktree，分支 ref 一个没删。要看代码必须 checkout；主工作树现在是 `main`，就地 switch 会污染主工作树并与并行任务冲突。
- **不开常驻的三条理由**：
  1. 与 main 分叉 **391 提交**（main 侧 1906 文件变动），差距只会随 main 推进继续拉大，放着不动并不"保值"；
  2. 一个 worktree = 一份独立 `target/`，本仓单份 target 是几十 GB 量级（主仓 128.58 GB），为一份 08-04 的快照长期占盘不划算；
  3. 当前优先级（单对话 + 摆设做真）用不到成长引擎，TH-5 产品需求也未定。
- **需要时怎么办**：决定移植且要连续多天作业时，开**临时 worktree**（`git worktree add --detach <path> <sha>`），做完即删。
- **风险提示**：该分支的 185 个单测是 08-07 的状态，真要移植必须先在新基线重跑——不能拿"当时全绿"当现在的证据。

---

## 1. W14 — 止血与收口（≈0.5 天，无前置依赖，**最先做**）

| 任务 | 内容 | 验收 | 成本 |
|---|---|---|---|
| W14-1 | **O-1 flaky 定论**：`test_delete_provider_default_provider_rejected` 连跑 **5 次**统计失败率；确认 flaky 则把 `TEST_GLOBAL_CONFIG_MUTEX` 覆盖扩到 `kernel_facade().upsert_model_config(...)` 路径 | 5 次全绿，或给出根因 + 修复 | S |
| W14-2 | **P2-2 单实例锁**：`config/app.json` 读改写加进程级锁（唯一指向数据丢失的 open 项） | 双开不再 last-write-wins；有测试 | M |
| W14-3 | **非原子写补齐**：`workspace_runtime/service/state.rs:90` 的 `runtime_layout_state.json` 改走 `write_bytes_atomic` | crash 中断不产生半文件；有测试 | S |
| W14-4 | **全量补跑 + 推送**：`check --workspace` + 全量 test（补 W11/W12/W13 后的证据链缺口）；代理端口起来后推 15 个 commit | 全量绿 + 与 origin 同步 | S（机时 0.5h） |

## 2. W15 — 单对话体验（D4：**最先做的功能波**，≈2-3 天）

| 任务 | 内容 | 验收 |
|---|---|---|
| W15-1 | **Markdown 渲染 + 代码高亮**（CH-02）：消息渲染从纯文本改 Markdown；代码块高亮 | 渲染正确；**本地渲染须防 raw HTML 注入**（威胁模型：不信任模型输出） |
| W15-2 | 输入框多行 + 文件/图片拖入（CH-07/CH-08） | 可拖入、可预览、进入上下文；路径与大小有上限 |
| W15-3 | 重新生成（CH-05）+ 编辑消息（CH-06） | 两条链路可用，会话持久化一致 |

## 3. W16 — 摆设做真（D1：**做真，不移除**，≈2-3 天）

| 任务 | 内容 | 真实数据源 |
|---|---|---|
| W16-1 | `pages_space.rs:47-139` 走廊 7 扇假门 | 接真实会话/工作区列表 |
| W16-2 | `windows/work.rs` 路由/规划/Diff/终端四处假状态 | 真实 Agent 状态、Git 差异、token 消耗 |
| W16-3 | `windows/self_app.rs` 假 token/假词条；`app.rs:460-475` 编年史假色循环；`:518-530`/`:782-791` 无 onclick 空按钮 | 接真数据；确实无功能的空按钮**删除**（不是做真） |
| W16-4 | `pages_settings.rs:383-394` Card 2 假能量条 | 真实上下文/token 消耗 |

> 纪律：做真时若发现某处**没有真实数据源可用**，不许造假数据填充——走 NEEDS_CONTEXT 上报，或按 W16-3 的做法把元素删掉。

## 4. W17 — 多会话（D4：**后置**，≈2-3 天）

| 任务 | 内容 |
|---|---|
| W17-1 | 解除单 Room 锁定：`ROOM_SESSION_CACHE` 失效机制（`api.rs:121` ponytail 注释已标升级路径） |
| W17-2 | 手动新建会话入口（SE-01）+ 会话切换 UI |

---

## 5. 排期队列（穿插执行）

| 波 | 内容 | 成本 | 备注 |
|---|---|---|---|
| W18 | **命令风险预估 + 白名单制度**（D2） | L 1-2 天 | **设计先行**，产出见 `.superpowers/sdd/w18-command-risk-design.md`；设计未定稿前不许开工实现 |
| W19 | 解 `service → agentic` 13 文件逆向依赖（建 Runtime Port） | M-L 1.5 天 | 不动不影响功能 |
| W20 | selectors B 层合并（减 500+ 行）+ CLI theme.rs(989) 拆分 + god-file 下一波（memory_db 894 / pages_onboarding 859 / lsp manager 836） | M 1.5 天 | 与 rot 闸零余量直接相关，**桌面侧大改的前置** |
| W21 | 15 处 `not yet wired` 桩逐个接通 + 13 处 TODO 补 owner + rot-probe P2 的 30 处 `let _ =` | M 1-2 天 | |
| W22 | 能力透出 UI：cron（TH-6）/ dream（TH-5）/ PCS 插件 | L 各 2-3 天 | **依赖 D3 后续裁决**；PCS 完全未开始 |
| W23 | T2-1 CI 补齐 | M 1-2 天 | 前置：i18n-contract 24 个预存失败 |

## 6. 执行纪律（沿用本仓既有，不得简化）

- 每任务：brief 文件 → 派 coder → `git diff <父 commit> <commit> -- <点名文件>` 生成审查包 → 派 judge 双判决（SPEC + QUALITY）→ Critical/Important 必返修 → 通过后**立即**追加 progress.md 台账行并 commit（不攒到波末）。
- 涉 Rust 任务的 brief 必须附 `.opencode/templates/rust-brief-block.md` 全文。
- **审查包 BASE 取 implementer 的父 commit**，不用跨 commit 范围 diff（会混入并行任务改动——2026-08-31 由 judge 指出）。
- 子代理返回后**一律磁盘复核**（`git log`/`status`/读 diff），尤其带具体行数的阻塞性结论——审计 R1 曾报「main.rs 799 / app.rs 791 行」，实测 693 / 749，纯属虚构。
- 子代理会话被 cancel 后**先 `git status` 查残留再重派**（W13-2 踩过）。
- rot 闸：不上调任何 ceiling；`let _ =` **388/388 零余量**，修 bug 不许引入新的静默吞错，也不许靠改名规避（本仓有"闸口游戏回滚"先例）。
- 宵禁 03:00；长构建走 PTY 后台 + 轮询，不同步死等。

## 7. 交付顺序（D1/D4 已定稿）

```
W14 止血（0.5天）
  └─ W15-1 Markdown（性价比最高）
       └─ W15-2/3 附件 + 重新生成/编辑
            └─ W16 摆设做真（穿插 W20 腾 rot 余量）
                 └─ W17 多会话（后置）
```
W18 的设计可与 W15 并行推进（不抢同一批文件），设计定稿后插队实现。
