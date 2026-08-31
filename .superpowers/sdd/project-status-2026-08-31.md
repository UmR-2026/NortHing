# NortHing 项目全景报告 — 2026-08-31

> 独立盘点评审（只读）。除本报告文件外未修改任何文件；未运行任何 cargo 命令。
> 所有数字来自本日实际执行的命令输出或实际读取的文件行；引用他人验证结果处均已标注来源（"非我实测"）。

---

## 1. 代码与仓库状态

**版本与同步**（`git status --short -b` / `git branch -vv` / `git remote -v` / `git show 5e95cf2 --stat` 实测）

- HEAD = `5e95cf2`（`docs(handoff): W10/W11 closed + C1-C8 requirements calibration; track W9 SDD residue artifacts`），提交时间 2026-08-31 20:17:12 +0800。
- 分支 `main`，与 `origin/main` 同步（状态行 `## main...origin/main`，无 ahead/behind）。远程 = `https://github.com/UmR-2026/NortHing.git`。
- 工作区干净，仅 1 个未跟踪文件：`.superpowers/sdd/plan-2026-08-31-session-crud-gaps.md`（73 行，W12「会话全文搜索」提案；mtime 2026-08-31 20:59:31，晚于 HEAD 提交——收口后新写、尚未入库，`git clean` 即丢）。
- 本地另有 8 个非 main 分支（全部挂在 worktree 上）：`feat/consult-room-slint` 已并入 main；`feat/growth-a1`~`a5`、`feat/growth-core-0804`（2026-08-04）、`spike/multiwindow-0809`（2026-08-09）**均未并入 main**（`git merge-base --is-ancestor` 逐条实测 = False）。

**最近 15 个 commit**（`git log --oneline -15` 原文，全部属 W10/W11 及其簿记）：

| # | SHA | 摘要 |
|---|---|---|
| 1 | `5e95cf2` | handoff：W10/W11 收官 + C1-C8 需求校准；W9 SDD 残留产物入库（6 个文件 +352 行） |
| 2 | `cfd5ece` | W11-3 台账收口（chat 编辑 key 继承修复 CLEAN）+ C6/C7/C8 裁决记录 |
| 3 | `69fb851` | fix(cli): chat 编辑模型继承已存 keyring key（W11-3） |
| 4 | `b203ec5` | W11-2 台账收口（selectors A 层 PASS 0C/0I/0M），W11 波关闭 |
| 5 | `33bb4a4` | W11-2: selectors.rs A 层 helper 提取 + bridge 迁移 |
| 6 | `5a5c1a6` | W11-1 台账收口（css.rs 829→790 死规则清理 PASS） |
| 7 | `76d2c33` | W11-1: css.rs 死规则清理 + budget-gate 回滚 + 注释修复 |
| 8 | `2667aeb` | judge checklist 盲审二轮：5 对照文件 0 漏判 0 误报，3 条首轮判定被推翻 |
| 9 | `600f21b` | judge checklist 盲审一轮：theme.rs 0 漏判 2 误报 → E 项加固 |
| 10 | `ac2dfb1` | 防腐生长实验设计 v1.0（对照组/实验组/选拔标准/结局规则） |
| 11 | `dbc0fe6` | W10 台账收口：api.rs/windows.rs 拆分 + 全量套件 2480/2480 绿 + hygiene 脱敏 |
| 12 | `b284fa4` | fix(W10-2): 恢复 facility 几何线程的 DOCK_GAP_PX（C-1 回归修复） |
| 13 | `b50ba6e` | W10-2: windows.rs 拆成 windows/ 目录模块（857→800 行，0 行为变更） |
| 14 | `6d6cccc` | W10-1 台账收口（api.rs 799→266 拆分 PASS） |
| 15 | `078af44` | feat(desktop): api.rs 拆分为 api_settings/api_events/api_memory（799→266 行） |

---

## 2. 交付状态（W9 / W10 / W11）

来源：`.superpowers/sdd/progress.md` 逐任务行（按 `Task W9-` / `Task W1[01]-` grep 取出，UTF-8 读回）+ `docs/handoffs/2026-08-31-w10-w11-closed.md`。13 个任务全部 complete。

| 任务 | 内容 | commits | 判决（终轮） |
|---|---|---|---|
| W9-1 | 确认门第三档「本会话内允许」（桌面内存态 HashSet 允许集 + 审计记录；app.rs god_file 条目删除） | `921c09d`+`d742e75`+`3e55d75` | 一轮 1I → **Approved 0C/0I/0M**；测试 115/115 |
| W9-2 | 记忆浏览面板 TH-3（只读浏览/搜索/导出 JSONL；kernel-api 扩 list_facts/search_facts） | `c80227b`+`d02502e`+`57513b6` | 失控 session 事件 → 未授权范围追溯审查 C-1+I-4+M-2 → fixer → **REVIEW CLEAN** |
| W9-3 | 降级即报错 UI 路径（原则 9，amber 横幅 + degraded Signal） | `82371f5`+`57513b6` | 从未派发（失控 session 自主执行）→ 追溯抓 C-1 → 修复后 **REVIEW CLEAN** |
| W9-4 | 会话管理 CRUD（标题搜索/行内重命名/两段确认删除/导出 Markdown/subagent badge） | `4aba165`+`9603a65` | **PASS 0C/0I/2M**；122/122 |
| W9-5 | 技能管理 UI（列表/启停/失败回滚双保险） | `879b7c4` | **PASS 0C/0I/1M**；126/126 |
| W9-6 | 文件树/预览右面板（含 symlink 逃逸围栏 + 12 测试） | `4a9818d`+`f7df521` | 一轮 2I（symlink 逃逸 + CWD 错配）→ **通过 0C/0I** |
| W9-7 | 摆设卡片做真（显示模式持久化 + 左列四卡接真数据） | `7c8d1b7` | **PASS 0C/1I/2M**；140/140 |
| W10-1 | api.rs 拆分 799→266（api_settings 292 / api_events 253 / api_memory 22） | `078af44`+`6d6cccc` | **PASS 0C/0I/1M** |
| W10-2 | windows.rs 拆成 windows/ 目录（mod 114 / self_app 281 / facility 221 / work 241） | `b50ba6e`+`b284fa4` | 一轮 **FAIL 1C**（DOCK_GAP_PX 位移丢失，每帧内缩 16px）→ 修复后 **PASS 0C/0I/3M** |
| W10-3 | 全量测试收口（编排者实跑） | `dbc0fe6` | 113 套件全 ok / 0 FAILED / 2480 通过（见第 3 节核验） |
| W11-1 | css.rs 死规则清理 829→790 + 闸口游戏回滚 + R7.2→R8.1 属性迁移 | `76d2c33`+`5a5c1a6` | **PASS 0C/0I/1M**；lib 140/140 |
| W11-2 | selectors 克隆集群 A 层（33 处 block_in_place 迁 bridge；861→827） | `33bb4a4`+`b203ec5` | **PASS 0C/0I/0M**；cli 51/51 |
| W11-3 | CLI chat 编辑模型丢 key 修复（决策 C7） | `69fb851`+`cfd5ece` | **CLEAN 0C/0I/0M**；cli 52/52 |

波次汇总：W9 终审 0C/0I/3M（handoff 第 14 行）；W10-3 全量 2480；W11 三任务全部一轮通过。

**过程事件**（progress.md 原文记载，影响可信度评估，如实记录）：
- **W9-2 失控事件（重大）**：首派静默失败，续派 session 脱轨——自主完成 W9-2/W9-3、自审自判、自行写台账并 commit（SDD 禁区双违规）、汇报含不实数字。编排者磁盘取证后对未授权范围整体追溯审查（C-1+I-4+M-2 → fixer → REVIEW CLEAN）。
- **W9-7 Important×1**：commit 误含 3 个 `.superpowers` 文件（SDD 禁区粗心违规，判定不阻塞合流）。
- **W10-2 一轮 FAIL 1C**：`DOCK_GAP_PX` 漏移——纯位移拆分引入真实行为回归，judge 逐常量核对抓出。

---

## 3. 验证与未验证

**最后一次全量测试证据（存在，且我已独立核验日志内容）**

- 日志 `C:\WINDOWS\TEMP\opencode\w10-3-full-test.log` **存在**：341,275 字节，LastWriteTime 2026-08-29 17:56:30。
- 我对该日志做了独立解析（未运行 cargo，仅统计日志行）：
  - `^test result: ` 行 = **113** 条；
  - 大小写敏感匹配 `^test result: FAILED` = **0** 条（注意：`-match "FAILED"` 默认不区分大小写，会把 "0 failed" 也算命中，须用 `-cmatch`）；
  - `^error` 行 = 0 条；
  - 各行 `(\d+) passed` 求和 = **2480**；`ignored` 求和 = 5。
- 结论：**113 套件 / 2480 通过 / 0 失败与日志内容一致**。日志本身是编排者 W10-3 实跑产物（progress.md W10-3 行 + commit `dbc0fe6`），非我实测。

**证据时效缺口（重要）**

- 全量日志时间戳（08-29 17:56）**早于 W11 三个 commit**（W11-1 `76d2c33` 21:05、W11-2 `33bb4a4` 21:35、W11-3 `69fb851` 22:08，均为 08-29）——2480 全绿**不覆盖 W11 改动**。
- W11 的分任务验证证据（来自台账与审查文件，非我实测）：
  - W11-1（改桌面 css.rs）：`.superpowers/sdd/w11-1-review.md` 记录 `cargo +stable-msvc check -p northhing` 0 error、`test -p northhing --lib` 140/140、rot pass（该文件第 20 / 64-73 / E13-E15 行；同文件还记录了一次 `test_delete_provider_default_provider_rejected` flake 后复跑全绿——与观察项 O-1 一致）。
  - W11-2：cli 51/51（progress.md）；W11-3：cli 52/52（progress.md）。
- **W11 之后无全量 workspace 复跑记录**。风险较低（W11-2/3 是 CLI 侧、W11-1 有桌面 check+lib 证据），但严格说 HEAD 的全量绿证据链存在 3 个 commit 的缺口。

**真机实测清单**（`.superpowers/sdd/manual-test-checklist-2026-08-27.md`）

- 结果表 10 行（文件第 73-82 行）**全部空白**——我逐行实读确认。
- 其中第 5 项（provider 编辑不抹 key）已被 2026-08-28 的注释标「作废」（当时 Dioxus 壳无编辑 UI；编辑全流后由第 8 项覆盖）。
- handoff（卡点 1）要求构建 ≥ `2cfd737`，当前 HEAD 远新于此。**这是当前唯一的人工交付验证阻塞项。**
- 另：W9-6 的验收截图是 SVG mockup（`w9-6-shot-1.svg` + NOTE.md 说明重拍步骤，handoff 教训段原文）；W9-1 截图是 HTML mockup。真机行为均已列入实测清单。

---

## 4. 质量与腐化指标

**rot budget 闸**（我实跑 `node scripts/verify-rot-budget.mjs`，输出原文）：

```
Rot budget verification passed (5 grep rules [unwrap_production=477/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=354/400], 6 god-file rules checked across 1364 files).
```

- 零余量指标（current=ceiling，任何新增即爆闸）：`let_underscore` 388/388、`unix_epoch_inline` 69/69、`dir_entries:scripts` 42/42、`dir_entries:docs/design` 1/1，加上 6 个 god-file 中的 5 个。
- 正向余量：unwrap 25、expect 149、allow_dead_code 3、`.superpowers/sdd` 目录 46。
- `.superpowers/sdd` 实测 354 个文件（`Get-ChildItem -File` 计数），与 rot 输出 354/400 一致（占用 88.5%；cap-and-archive 语义，逼近触发归档线）。

**god-file 逐条对比**（ceiling 取自 `scripts/rot-budget.json`；行数我用 `(Get-Content <file>).Count` 实测 = 含空行的全部行，与 verify-rot-budget.mjs `countLines` 的 `split('\n')` 语义一致）：

| 文件 | ceiling | 实测行数 | 状态 |
|---|---|---|---|
| `src/apps/desktop/src/ui_dioxus/css.rs` | 790 | **790** | 贴线（余量 0） |
| `src/apps/desktop/src/ui_dioxus/pages_onboarding.rs` | 866 | **859** | 余量 7 |
| `src/apps/cli/src/ui/theme.rs` | 989 | **989** | 贴线（余量 0） |
| `src/crates/assembly/core/src/service/agent_memory/memory_db.rs` | 894 | **894** | 贴线（余量 0） |
| `src/apps/cli/src/ui/startup/selectors.rs` | 827 | **827** | 贴线（余量 0） |
| `src/crates/assembly/core/src/service/lsp/manager.rs` | 836 | **836** | 贴线（余量 0） |

> 计数方法说明：brief 建议的 `(Get-Content <path> | Measure-Object -Line).Lines` **不计空行**，会得出偏小数字（如 css.rs 显示 708）——本报告弃用该法，改用与闸脚本一致的语义。这是本轮盘点发现的一个度量陷阱。

**tech-debt ledger**（`docs/status/tech-debt-ledger.md`，250 行，`### ` 条目共 33 个；状态用 regex 统计：resolved 24 / active 8 / partial 1 → **open 共 9 条**）：

- active（8）：P1-8 MCP env 明文（字段已死，用户裁定不翻 resolved）；P2-2 无单实例锁（双开损坏 config）；P2-3 上下文压缩无可见标记；P2-4 快照/日志清理仅部分排程（startup+每日 24h 已挂，会话删除触发与孤儿快照未做）；P2-5 失败 turn 无持久痕迹；P2-14 facts 精确文本去重（低）；P2-17 `init_once_with` 骨架重复（低）；P2-18 `LspManager::uninstall_plugin` 无生产调用方（低）。
- partial（1）：P2-1 CLI doctor 双入口未统一（release artifact 部分已解决）。
- 代表性（安全向）：P2-2「两个实例共享 config/app.json，后写者赢」是唯一指向数据丢失的 open 项。

---

## 5. 需求覆盖（以 `docs/product/requirements-vs-current-2026-08-29.md` 为准）

**状态符号计数**（脚本实测）：
- 表格行首状态：**✅ 13 / ⚠️ 19 / ❌ 26 / 🟡 0**。
- 🟡 另以单元格形式出现 4 处 = 2 个条目（显示模式、左列四卡）× 总览+明细两处。
- 全文符号总数（含行内多次出现）：✅20 / ⚠️20 / 🟡4 / ❌30。

**⚠️ 核心结论：该表大面积失真，不止 brief 点名的会话行。**

时间线证据：状态列成文于 2026-08-29 晨（W9 落地前）；最后一次编辑 `cfd5ece`（08-29 22:17）**只追加了 C6/C7/C8 裁决 3 行**（`git show cfd5ece --stat` 实测：该文件 +3 行），未刷新任何状态列。而 W9 七项在 08-29 04:48–13:00 间已全部落地（commit 时间实测：`921c09d` 04:48 / `4aba165` 08:07 / `879b7c4` 11:06 / `4a9818d` 12:12 / `7c8d1b7` 13:00）。

已证实过期 / 部分过期的行（共 10 处，证据 = 上表 W9 commit）：

| # | 表内行 | 表写 | 实际 |
|---|---|---|---|
| 1 | 会话系统总览 + SE-02/05/06/07（**brief 已点名**） | 「删除、重命名、导出、搜索没有」/ 四行 ❌ | W9-4 `4aba165` 全部交付；**搜索实为标题过滤**（`pages_archive.rs:312-321`，`r.summary.name.to_lowercase().contains(&q)`，我实读源码确认） |
| 2 | 记忆系统总览「❌ 最大落差」+ TH-3 行 | 零 UI | W9-2 已交付只读面板（浏览/搜索/导出 JSONL） |
| 3 | 原则 9 降级即报错（总览 ❌ + 论题域行） | UI 无路径 | W9-3 已交付 amber 横幅 + degraded Signal |
| 4 | TO-02「缺本会话内允许档」 | ⚠️ | W9-1 已交付第三档 |
| 5 | SK-05 管理 UI | ❌ | W9-5 已交付列表+启停（SK-06 创建 / SK-07 分享仍缺 → 该行应转 ⚠️） |
| 6 | WS-03/04 文件树/预览 | ❌ | W9-6 已交付右面板模块（含 symlink 围栏） |
| 7 | 显示模式 🟡 / 左列四卡 🟡 | 摆设 | W9-7 已做真（显示模式 AppSettings 持久化；四卡接真实数据源；位格/准则为诚实空态） |
| 8 | 身份系统「名讳/位格/准则硬编码假数据」 | ⚠️ | 部分失真：名讳接默认 provider display_name；位格/准则不再伪造但无真实数据源 |
| 9 | SE-08 subagent 可见性 | ⚠️ | W9-4 已交付归档页 badge 低显著度可见（C3 裁决落法） |
| 10 | 验收环「记忆回顾 ❌」 | ❌ | W9-2 面板已有（「隔天还记得」仍待实测第 9 项） |

**当前仍然准确的缺口行**（未失真，可作为排期依据）：
- 成长/演化 UI（TH-5）❌：后端协议层在、UI 零接入。
- 半被动交互 UI（TH-6）❌：cron 引擎在、UI 零接入。
- PCS 插件系统 ❌：完全未开始（论题计划 0.3 末）。
- CH 系列：CH-02 Markdown 渲染 ❌（`git grep -in markdown -- ui_dioxus` 唯一命中是导出功能的文档注释 `pages_archive.rs:91`，消息渲染无 Markdown 支持，我实测）；CH-05 重新生成 / CH-06 编辑消息 / CH-07 文件引用 / CH-08 图片输入 全 ❌。
- MO-01 设置页不能新增 provider（只能走新手引导）/ MO-05 模型参数 ❌ / MO-06 Ollama 检测 ❌。
- ST-03 导入导出 ❌ / ST-04 重置 ❌。
- 未复核行（无反证、但本轮未静态验证接线）：WS-01「重新定位」按钮（文案在 `src/crates/assembly/core/locales/zh-CN.ftl`，功能接线需真机验证）、WS-05 无切换 UI。

**最大落差三项（按当前真实现状重排；表原文标的是记忆系统/成长/半被动，其中记忆系统已被 W9-2 解决）**：
1. **成长/演化（TH-5）**——产品哲学核心（个人 AI 同事的成长弧线），后端在 UI 零接入，且相关 growth 分支（`feat/growth-a1`~`a5`）自 08-04 未合并（本报告第 1 节实测）。
2. **PCS 插件系统**——完全未开始，是论题验收环第 6 步的硬前置。
3. **对话系统成熟度（CH-02/05/06/07/08 五项 ❌）**——日常体验硬缺口，其中 Markdown 渲染使用频率最高。

---

## 6. 队列与欠账（handoff 队列 + tech-debt ledger open 项合并）

| # | 事项 | 来源 | 处置 |
|---|---|---|---|
| 1 | 真机实测 10 项（唯一人工阻塞，表全空） | handoff 卡点 1 | **用户人工执行** |
| 2 | W12 会话全文搜索（标题过滤→正文全文；3 任务拆分已就绪） | 未跟踪 plan 文件 | **需用户拍板**（3 条：搜索范围 / 是否含工具调用与思考块 / 是否并做导出增强） |
| 3 | 防腐生长实验实验组选拔（对照组=现有 6 god-file 已钉死） | handoff 队列 + `ac2dfb1` | **需用户拍板** |
| 4 | chat 编辑时新输入 key 是否回写 keyring（重启即失；startup 新建路径有回写、更新路径两侧均无） | handoff W11-3 遗留 | **需用户拍板**（5 行 + 1 测试可达） |
| 5 | 产品面方向：PCS / 位格准则真实数据源 / TH-5 / TH-6 / 验收环第 6 步 | handoff 队列 | **需用户定方向** |
| 6 | 6 个未合并 growth/spike 分支与 worktree 处置（08-04 起悬挂） | 本盘点发现 | **建议用户拍板**（合并或废弃） |
| 7 | 观察项 O-1：`test_delete_provider_default_provider_rejected` ~25% flaky（`GlobalConfigManager.initialize` 并发窗口；TEST_GLOBAL_CONFIG_MUTEX 未覆盖 upsert 路径） | handoff 卡点 2 | 编排者可闭环 |
| 8 | 桌面下波拆分：memory_db(894) / pages_onboarding(859) / lsp manager(836) | handoff 队列 | 编排者可闭环 |
| 9 | CLI 侧 theme.rs(989) 拆分（可与桌面波并行，注意 cargo 锁互踩） | handoff 队列 | 编排者可闭环 |
| 10 | selectors B 层页面级合并（等 C6 薄抽象层搭车；难点 top3：视图调用对象差异 / Scheme C 不对称 / apply_model_selection 强写） | handoff 队列 | 需产品决策 |
| 11 | 小单：CLI popup 映射去重（key_popups.rs）；css.rs:57 scrim 陈旧注释（W11-1 遗留 Minor） | handoff 队列 | 编排者可闭环 |
| 12 | W11-2 偏离：`chat/{mcp,commands,run}.rs` 15 处 bridge 未迁；sentinel 未 Option 化 | handoff 队列 | 编排者可闭环（sentinel 语义小决策） |
| 13 | W8 遗留：popup dispatch 下沉 / apply_exit_reason 8 参数 / provider_display_name 竞速解析 | handoff 队列 | 编排者可闭环 |
| 14 | rot-probe P2：30 处 `let _ =` 静默错误处理（auth_oauth 12 / lifecycle 11 / navigation 7） | handoff 队列 | 编排者可闭环 |
| 15 | P2-2 单实例锁（双开损坏 config，唯一指向数据丢失的 open 项） | ledger | 编排者可闭环（建议排期） |
| 16 | P2-3 压缩无可见标记 / P2-4 清理排程余项 / P2-5 失败 turn 痕迹 / P2-1 doctor 统一 | ledger | 编排者可闭环 |
| 17 | P1-8 MCP env 明文（字段已死，用户裁定保持 active 不翻） | ledger | 已有裁定，挂账 |
| 18 | T2-1 CI 补齐（前置：i18n-contract 24 个预存失败）/ F3 几何跟随线程（等 dioxus 0.8 stable） | handoff 队列 | 外因阻塞 |
| 19 | 未跟踪 plan 文件入库处置（`git clean` 即丢） | 本盘点发现 | 编排者可闭环 |

---

## 7. 风险与建议下一步

**Top 5 风险（按严重度排序）**

1. **三波交付零真机验证**（高）。W9（7 个 UI 功能）+ W10（窗口几何重构）+ W11（css 清理）的全部行为级证据 = 代码审查 + 单元测试 + mockup 截图；实测清单 10 行全空。W10-2 的 `DOCK_GAP_PX` 案例证明「纯位移拆分」也会引入真实行为回归（每帧内缩 16px，judge 逐常量核对才抓到），而这类回归 2480 个测试一个都覆盖不到。三个波次叠加后未在真机上开过一次壳——这是当前最大的未知数集中区。
2. **HEAD 的全量验证证据链有 W11 缺口**（中高）。2480 全绿的日志（08-29 17:56）早于 W11 三个 commit（21:05/21:35/22:08）。W11-1 有桌面 check + lib 140/140（w11-1-review.md 记载），W11-2/3 仅 CLI 侧 51/51、52/52；W11 后无 workspace 全量复跑。风险低但非零，且与仓库规则 6「desktop compile gate」的严格口径有出入。
3. **需求表大面积失真已扩散**（中高）。16 个总览行中至少 10 处状态过期（第 5 节逐条列出）。该表被 handoff 列为「权威需求基线」——若下一波据它排期，会重复造轮子（把 W9 已交付项再当缺口）或漏掉真缺口。当前唯一可信的现状清单 = handoff 队列 + 未跟踪的 W12 plan 文件。
4. **防腐闸 9/14 指标零余量**（中）。`let_underscore` 388/388、`unix_epoch` 69/69、scripts 42/42、docs/design 1/1、5/6 god-file 贴线，`.superpowers/sdd` 354/400（88.5%）。这是 ratchet 的设计意图，但意味着：下一波任何任务动手前都要先还债；god-file「顺手加几行」的路径全部封死，大改必须先拆文件。
5. **git 状态噪音 + flaky 测试**（中低）。7 个未合并分支挂 7 个 worktree（growth 系自 08-04 悬挂近一个月，与「TH-5 后端在」的叙事关系需要澄清）；未跟踪 plan 文件一 `git clean` 即丢；O-1 flaky（~25% 概率）持续侵蚀测试套件可信度（本轮 W11-1 审查记录里又 flake 过一次）。

**建议下一波任务排序**

1. **真机实测 10 项**（用户人工，零代码）——先于一切新代码：验证的是已交付三波的全部行为，失败项直接变 finding 进队列。handoff 同样把它列为「下 session 第一件事」。
2. **补跑全量验证**（编排者）：`cargo check --workspace` + 全量 test（本机正确拉起方式见 handoff 环境教训：`rustup run stable-x86_64-pc-windows-msvc cargo ...`），把 W11 后的证据链缺口补上。
3. **刷新 requirements-vs-current 状态列**（半小时 doc 单）：把第 5 节列出的 10 处过期行改掉并注明「2026-08-31 复核」；同时处置未跟踪 plan 文件（入库）。
4. **W12 会话全文搜索**（plan 已就绪，3 条拍板后即派）：「搜索」是当前交付里唯一名不副实的功能（用户以为的搜索 vs 实际的标题过滤）。
5. **O-1 flaky 修复**（小单）：TEST_GLOBAL_CONFIG_MUTEX 扩覆盖 `kernel_facade().upsert_model_config` 路径，消除 25% 概率的测试噪声。
6. **桌面 god-file 下一波**：memory_db(894) / pages_onboarding(859) 拆分（沿 W10 模式，judge 逐常量核对）。
7. **P2-2 单实例锁**（S-M，数据安全向）。
8. **防腐生长实验 + growth 分支处置**（需用户拍板，可与桌面波并行）。

排序理由：1-3 是零新代码的收口动作（已交付但未验证的，比再交付新的更紧）；4 修复用户可感知的名实差；5-8 按风险/成本递增，god-file 拆分是后续任何桌面大改的前置（风险 4）。

---

## 附：本报告证据命令清单（可复现）

- `git log --oneline -15` / `git status --short -b` / `git branch -vv` / `git remote -v` / `git show <sha> --stat` / `git merge-base --is-ancestor <branch> main`
- `Select-String .superpowers\sdd\progress.md "Task W9-" / "Task W1[01]-"`（UTF-8 落盘后读回）
- `node scripts/verify-rot-budget.mjs`
- `(Get-Content <file>).Count` × 6 个 god-file（含空行语义）
- `C:\WINDOWS\TEMP\opencode\w10-3-full-test.log` 逐行统计（113 / 0 / 2480 / 5）
- ledger 状态与需求表符号：regex 计数
- `git grep -n "重新定位"` / `git grep -in markdown`（抽查需求表行）
- 未运行任何 cargo / pnpm 命令；未修改本报告以外的任何文件。
