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

## 1. W14 — 止血与收口（≈1.5 天，无前置依赖，**最先做**）

| 任务 | 内容 | 验收 | 成本 |
|---|---|---|---|
| W14-1 | **测试隔离改造**（原"O-1 flaky 定论"已升级，见 §1.1）：把依赖进程级单例状态的测试拆到独立测试目标，消除顺序依赖 | 新目标连跑 5 次全绿 **且** 全量套件连跑 5 次全绿（并行 + 串行各 5 次） | **L ≈4 天**（见 §1.2 仲裁裁定；原估 1 天，严重低估） |
| W14-1e | **真实记忆库污染修复**（`auto_memory.rs:575` 漏 `with_test_memory_db_path` 守卫 → 测试会写开发机真实 `memory.db` 且不清理，已磁盘复核属实） | 补守卫；`rg with_test_memory_db_path` 全仓核对无遗漏 | S 0.25 天（**优先于 W14-1c**） |
| W14-2 | **P2-2 单实例锁**：`config/app.json` 读改写加进程级锁（唯一指向数据丢失的 open 项） | 双开不再 last-write-wins；有测试 | M |
| W14-3 | **非原子写补齐**：`workspace_runtime/service/state.rs:90` 的 `runtime_layout_state.json` 改走 `write_bytes_atomic` | crash 中断不产生半文件；有测试 | S |
| W14-4 | **全量补跑 + 推送**：`check --workspace` + 全量 test（补 W11/W12/W13 后的证据链缺口）；代理端口起来后推 15 个 commit | 全量绿 + 与 origin 同步 | S（机时 0.5h） |

### 1.1 W14-1 升级说明：不是 flaky，是**测试间全局状态污染**

**实测数据**（2026-09-01，测试二进制 `target/debug/deps/northhing-7cec78aa9cf51e26.exe`，单次约 0.6s）：

| 模式 | 结果 |
|---|---|
| 默认（并行）5 次 | 1 次失败（20%）—— `test_delete_provider_default_provider_rejected` |
| `--test-threads=1`（串行）5 次 | **5 次全失败（100%）** —— 但失败的是**另一个测试**：`ui_dioxus::api::tests::test_ensure_room_session_fails_cleanly_when_uninitialized`，panic 在 `api.rs:172` `assert!(res.is_err())` |

**结论**：串行下失败是**确定性的**，并行下才是概率性的 → 这不是时序 flaky，而是**测试顺序依赖**。

**根因**：`kernel_facade()` 的 `static FACADE: OnceLock<...>` 一旦被任何测试初始化就**永不重置**；`GlobalConfig` 的 default provider 同理。凡是断言"未初始化时必须报错"或"默认 provider 必须被拒绝删除"的测试，只要排在一个会初始化全局状态的测试之后，就必然失败。并行模式下它们抢跑赢了，所以看起来只是"偶发"。

**O-1 原假设（mutex 未覆盖 `upsert_model_config`）只对一半**：加锁解决不了"必须在未初始化状态下失败"这类断言与"会初始化全局状态"的测试共存于同一进程的问题。

**改造方案（三选一，需设计裁定）**：

| 方案 | 做法 | 评价 |
|---|---|---|
| A（推荐） | 把依赖全局状态的测试迁到**独立集成测试目标**（如 `src/apps/desktop/tests/global_state_isolation.rs`）——每个 integration test 文件是独立进程，天然干净 | 干净、无侵入；代价是要把相关项从 `pub(crate)` 提到 `pub` |
| B | 加 `#[cfg(test)] unsafe fn reset_facade_for_tests()`，测试里显式重置 | 改动小，但引入 unsafe 且容易被误用 |
| C | 让断言变"宽容"（已初始化就跳过并记日志） | **不推荐**——等于把问题藏起来 |

**任务拆分**：
- W14-1a 侦察：扫出全部依赖全局状态的测试（grep `before_init` / `uninitialized` / 对 facade 的 `is_err()` 断言 / `set_default_provider` / `TEST_GLOBAL_CONFIG_MUTEX` 使用者）→ 清单（S，0.5h）
- W14-1b 设计裁定：A / B / C 选一个（可由独立子代理仲裁）（S，0.5h）
- W14-1c 实施：迁移 + 可见性调整 + 新目标接入（M，0.5-1 天）
- W14-1d 验证：新目标 5 次 + 全量并行 5 次 + 全量串行 5 次（S，1-2h，机时）

### 1.2 仲裁裁定（W14-1b，2026-09-01，minimax-m3 独立仲裁，编排者已复核）

裁决书：`.superpowers/sdd/w14-1b-arbitration.md`

**主判：A + B 混合，否决 C。**
- A（迁独立集成测试目标）解决 A 类"断言未初始化"的 5 个测试 → **一个测试一个文件**（不用 `--test-threads=1` 文档约定）——进程隔离优先于文件膨胀。
- B（`#[cfg(test)] pub` 重置 seam）解决 B 类"变更全局状态"的 22 个测试中的 14 个。
- 否决 C（断言改宽容）：等于把契约藏起来。

**侦察产出**（`w14-1a-global-state-test-inventory.md`）：全仓 **50 个涉险测试** —— A 5 / B 22 / C 24 / D 4 / E 6 / F 43；按 crate：core 28、desktop 12、services-integrations 3、installer 2、cli 2、其它 4。

**可见性规则（关键）**：禁止把 `pub(crate)` 提到 `pub`；只允许 `#[cfg(test)] pub`（release 构建中不存在）。不破坏六层分层——分层管的是**依赖方向**，`pub`/`pub(crate)` 是 API 表面，两回事。

**⚠️ 编排者对裁决书的一处更正（已磁盘复核）**：裁决书称"desktop 需先拆 `lib + bin`（0.5 人天）"——**不成立**。`src/apps/desktop/src/lib.rs` 已存在且四个模块（`app_state` / `flags` / `mcp_adapter` / `ui_dioxus`）**均为 `pub`**，bin 在 `src/main.rs`。不需要拆；desktop 的实际成本是**条目级可见性**（部分内部 helper 是 `pub(crate)`），由 B 的 `#[cfg(test)] pub` seam 覆盖。
→ **总成本由 4.25 人天修正为 ≈3.75 人天**（保守按 4 天排期）。这是今晚第二次"外部结论经磁盘复核翻案"（第一次是 R1 的 799/791 行）。

**附带条件（执行时必须遵守）**：
1. W14-1e（真实库污染）**先独立合入**并全仓 `rg` 核对，再动 W14-1c；
2. 任何非 `#[cfg(test)]` 的 `pub(crate) → pub` 一律打回；
3. CI 双轨（并行 + `--test-threads=1`）**连续 5 轮全绿**才算完成；
4. 测试总数与覆盖不许下降；
5. 不许改动 `FACADE` 的 `OnceLock` 形态本身、不许碰 global_scheduler、不许改六层依赖方向。

**排期影响**：W14 总计由 1.5 天 → **约 5 天**（W14-1 4 天 + W14-2/3/4 约 1 天）。这会让"止血"阶段从 1.5 天膨胀到 5 天，需要在 W14-1e 之后重新权衡：是先做 W15（单对话，用户可感知）还是先把测试地基修完。

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
