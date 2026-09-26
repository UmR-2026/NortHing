# W27 前置探查：agent 成长容器现状 recon

- 日期：2026-09-26（会话跨至 09-27）
- 性质：只读探查，本报告为唯一写入文件；所有 file:line 相对 `E:\agent-project\northing`，以工作树当前内容实测（HEAD `4d25367`）
- 工作树注记：`src/crates/contracts/runtime-ports/{port_core.rs, runtime_facade_tests.rs, session_workspace.rs}` 存在并行 session 未提交改动，本报告**未引用**这三个文件
- 产品公理（探查框架）：agent 优先的成长容器；人类只能用言语干预；memory 与日志不可手动编辑；人类唯一绝对权力 = 彻底销毁

---

## 一、steering 入史与否

**事实（file:line）**

- 事件契约：`AgenticEvent::UserSteeringInjected`（`src/crates/contracts/events/src/agentic.rs:297-304`），带 `content`（模型所见原文）与 `display_content`（人所见版本）双通道。
- 入模型上下文：轮边界注入时包裹 `<system_reminder>` 进 `state.messages` 并调 `session_manager.add_message`（`src/crates/assembly/core/src/agentic/execution/turn_tick.rs:436-458`），随后强制续轮（`:478-481`）。
- add_message 有真实落盘：context_store 写入 + 每轮上下文快照持久化（`src/crates/assembly/core/src/agentic/session/session_manager_metadata.rs:478-483` → `session_persistence/save.rs:130-176`；快照文件路径 `src/crates/services/services-core/src/session/layout.rs:69`）；restore 时快照优先于 turn 重建并直接灌回运行时上下文（`src/crates/assembly/core/src/agentic/session/restore_apply.rs:222-251, 291`）——steering 内容**跨重启活在 agent 自己的记忆里**。
- 不入 UI 会话史：`DialogTurnData` 持久化只含 user_message + model_rounds（`src/crates/assembly/core/src/agentic/session/session_persistence/turn_lifecycle.rs:371-433`；turn_persist 的完成/取消/失败路径 `dialog_turn/turn_persist.rs:20-98` 均不写注入消息）；UI 历史重建 `get_messages` 走 `build_messages_from_turns`，只还原 ActualUserInput + assistant 文本（`session_persistence/save.rs:45-128`、`session_manager_metadata.rs:449-460`）。
- 事件出口只有一扇门：facade 把它映射为 Banner（`src/crates/assembly/core/src/kernel_facade/events.rs:397-400`）；桌面消费是流式期间的瞬时横幅（`src/apps/desktop/src/ui_dioxus/app.rs:216-220`），非消息列表条目；CLI chat/exec 打一行预览（`src/apps/cli/src/modes/chat/run.rs:409-419`、`src/apps/cli/src/modes/exec.rs:371-376`）。
- 桌面没有 steering 输入门：Enter 仅在 `!streaming()` 发送、发送钮流式期间变 STOP（`app.rs:563-582`）——内核路由（`src/crates/execution/agent-runtime/src/scheduler/sched_filter.rs:177-207`）就绪但桌面 UI 无人触发。

**→ 对公理的意义**：言语干预已是「agent 记得、人不入史」的单向透明设计——steering 进 agent 的上下文并持久于快照，但在人类侧只是转瞬 Banner，且唯一出货面连插话的门都没开；W27 若做「言语是唯一干预」，缺的是 UI 门，不是内核链路。

---

## 二、delete_session 的清理范围

**事实（file:line）**

- 链路：facade `delete_session`（`src/crates/assembly/core/src/kernel_facade/session.rs:148-159`）→ coordinator（`dialog_turn/coordinator_session.rs:158-160` → `dialog_turn/turn_cancel.rs:284-291`，发 `SessionDeleted` 事件、facade 端直接丢弃 `kernel_facade/events.rs:357-358`）→ `SessionManager::delete_session`（`agentic/session/session_manager_lifecycle.rs:297-446`）。
- 实际清掉的：① 快照系统资源（`:312-320`）；② 内存态 context/prompt-cache/skill 快照/file-read 态（`:344-348`）；③ 会话持久目录——turns/snapshots/metadata 物理删除 + 索引更新（`:362-364` → `agentic/persistence/session_subhandlers.rs:289-296` → `src/crates/services/services-core/src/session/metadata_store.rs:363-376` `remove_dir_all`）；④ 该会话的 cron 任务（`:372-392`）；⑤ terminal 绑定（`:400-422`）；⑥ 内存 session 表项（`:430-436`）。
- `run_file_cleanup`（`:332` 调用，定义 `:493-498`）名不符实：它跑的是全局保洁 `CleanupService::cleanup_all`——temp >7 天、logs >30 天、cache >1GB（`infrastructure/storage/cleanup.rs:22-31, 59-88`），**与会话无关、也不清任何会话数据**。
- 销毁后残留（成长产物全部存活）：memory.db 事实（用户级单库 `service/agent_memory/memory_db.rs:738-748`，fact 带 session_id/turn_id provenance 但删除链无任何按会话清理；`delete_fact` 无生产调用方 `:369`）；episodes JSONL（`data_dir/northhing/episodes/<slug>/`，按工作区非按会话，`agentic/episodes/store.rs:11-28`）；distiller/judge 状态与关键词权重（judge_mom/keywords 表，`turn_persist.rs:482-511`）；工作区 persona 文件（`service/bootstrap/bootstrap_impl.rs:105-128`）与 `.northhing/memory` 文件层。

**→ 对公理的意义**：「彻底销毁」目前**不成立也不原子**——烧掉一个会话，它蒸馏进事实库、episode 日志、关键词权重里的痕迹原封不动；W27 的销毁语义需要一条新的「按 session provenance 级联清除成长痕迹」链路（数据都在，钩子全缺，`SessionDeleted` 事件甚至没人订阅）。

---

## 三、memory 写路径盘点

**事实（file:line）**

生产写入口（全部是 agent 自动路径，均在轮终态 hook 之后）：

1. **蒸馏写事实**：`finalize_persisted_turn_in_workspace_if_needed` → `append_facts_entry`（`agentic/coordination/dialog_turn/turn_persist.rs:274-336, 426-563`）：LLM 蒸馏（`distiller.rs:34-95`，<20 字符降级关键词表 `facts.rs:237-296`；受 `GlobalConfig.memory.distiller_enabled` 门控 `distiller.rs:51` + `config/memory.rs:5-8`）→ `insert_fact`（`turn_persist.rs:555`）+ `record_fact_review(reviewer="distiller")`（`:519-531`）+ 权重衰减（`:559`）。
2. **自学习刹车**：`set_judge_state`（distill_turns/distill_hit_turns/distiller_paused，judge_mom KV 表，`turn_persist.rs:476-512`）。
3. **梦清理**：`run_dream_sweep`（24h 一轮、30 天陈旧、LLM 裁决；`dream.rs:21-30, 156-186`——`supersede_fact` + `record_fact_review(reviewer="dream")`），由写路径尾部触发（`turn_persist.rs:562`）。
4. **召回强化**：用户消息 BM25 召回时 `touch_fact`（`session.rs:220` → `auto_memory.rs:264, 327`）。
5. **一次性迁移**：jsonl→sqlite `migrate_facts_jsonl_once`（`facts.rs:68-141`，`insert_fact` at `:113`）。

**人类可触达的写入口：零。** `KernelMemoryApi` 只有 list/search 三个读方法（`kernel_facade/memory.rs:9-137`，全文件）；桌面 wrapper 注明 read-only（`ui_dioxus/api_memory.rs:4`）；记忆浏览器只有搜索+导出 JSONL、无删除/编辑按钮（`ui_dioxus/pages_memory.rs:3, 176-211, 213-309`）；CLI 无 memory 子命令（全 apps 目录 grep 零命中）。人唯一合法杠杆是 config 级开关 `distiller_enabled` / `distiller_model`（`config/memory.rs:5-8`）。注：memory.md 文件层与 persona 文件在磁盘上人可用编辑器改（产品外行为），但那是绕过 UI 承诺的暗面。

读面：`{PERSONA}`/`{AGENT_MEMORY}` 提示槽（`prompt_builder/system_prompt.rs:43-70`）、查询感知召回轮首注入（`session.rs:218-236`）、工作区 memory 文件上下文（`user_prompt.rs:127`）、记忆浏览器（只读）、facts/episodes facade。

**→ 对公理的意义**：「memory 不可手动编辑」在应用面已经成立（写全是 agent 自动路径、UI 零写门），公理缺口在另一半——人也没有任何**言语之外**的合法修正通道，而蒸馏误报（关键词表「别」触发 `facts.rs:237-296`）正等待 W27 用「言语纠错」补位。

---

## 四、growth-core 落地率（main 上逐项判定）

**事实（file:line）**

| 特性 | 判定 | 链路/证据 |
|---|---|---|
| episodes 蒸馏（distill/store/types） | **活，写入链完整** | `turn_persist.rs:313-323 → :414-421` → `episodes/distill.rs` → `store.rs:34-71`（5MB 单代轮转 `:12-13`）；读面仅人类浏览（facade `list_episodes` `kernel_facade/memory.rs:9-56`）；「agent 不读」是刻意设计（`episodes/mod.rs:4-5`） |
| 竞争抑制 | **main 不存在**（仅分支） | 全 crates grep `competition|suppress`：生产零命中；实现在 `feat/growth-core-0804`（`src/agentic/src/topics/competition.rs`、`memory_db/competition_groups.rs`、`competition_review.rs`，分支 `git diff --name-status` 在案） |
| negation 检测 | **main 不存在**（仅分支） | 分支 `src/agentic/src/negation.rs`：短语表 → LLM 二阶段确认 → 解析，注释自称「成长系统唯一 hard-retire 记忆的路径，精度优先于召回」 |
| judge-mom 审查 | **半死：表名活着，角色不存在** | `judge_mom` 表实为 KV 状态存储（`memory_db.rs:132-151, 707-735`），生产内容只有刹车计数+迁移标记（`facts.rs:74-86`）；`"judge-mom"` 字符串只在测试里作为 reviewer 出现（`memory_db_tests.rs:526, 547`）；生产 reviewer 只有 `"distiller"`/`"dream"`（`turn_persist.rs:524`、`dream.rs:94,164,181`） |
| self-cognition / persona 注入 | **persona 活；self-cognition 有读无写** | persona 四文件模板→`<persona>` 注入（`bootstrap_impl.rs:105-253` → `system_prompt.rs:14-56`）；identity.md 读路径每次建 prompt 都走（`system_prompt.rs:27-38`），写路径死（见第七节） |

**→ 对公理的意义**：main 的成长闭环只有「蒸馏-刹车-梦」三件套 + episode 单向账本；公理需要的「遗忘语法」（negation）与「topic 竞争代谢」整支躺在未收编分支——W27 若要 agent 自主成长，这是收编窗口而非重写时机。

---

## 五、六条 growth 分支盘点

**事实（file:line + git 证据）**

- `feat/growth-a1..a5`：均为 2026-08-04 同一基座 commit `7e96126`（scaffold `northhing-agentic-growth` crate）+ 各一个纯函数 commit（2 ahead / 2 behind main，diff 约 600-830 行）——a1 growth ports+持久状态+legacy key 迁移；a2 无依赖话题提取；a3 双层检索评分（topic dominance）；a4 竞争组归一化+自然抑制；a5 保守显式 negation 检测。这五个 commit 的内容与 `growth-core-0804` 中 `5eb5fbf/c9dcb58/7e3e279/6294760/148a0d` 一一对应（同主题、同顺序）——**是核心分支的拆条预演，已被完全包含**。
- `feat/growth-core-0804`：36 commits（08-04~08-07），57 文件 +11081/-605。内容：growth crate（state/selfcog/topics/negation/review{propose,route,verdict,merge}/scheduler/garden/promote/distill 纯逻辑）+ 宿主接线（`agentic/growth_adapter.rs` 生长状态落 judge_mom kv、`self_cognition.rs` 存储 + identity.md 一次性迁移、T3b「self-cognition 注入 store 优先、identity.md 兜底」、竞争组持久+冷话题抑制+LLM 全局确认 sweep T8/T9、facts 蒸馏限主会话、distiller 带关键词输出+话题权重 boost 接线、boundary 规则禁 dream/judge 写 self-cognition 密集路径）。merge-base `f2a16c7`（08-04），落后 main 36 commits，期间 main 的 turn_persist / distiller / dream 已重构演化，**不能直接 merge，须 rebase 式收编**。
- 收编价值判定：a1-a5 **废弃**（子集）；growth-core-0804 **值得 cherry-pick 式收编**——它是公理方向的完整原型（agent 独占 self-cognition 写权、言语驱动的 negation 遗忘、话题代谢），且自带审查整改轮（T9 fixer）与边界规则，成熟度非草案级。

**→ 对公理的意义**：成长容器的「言语驱动遗忘 + agent 独占自我存储」实现已经写过一遍并过了双审查，W27 的正确姿势是把它 rebase 回 main 而不是从零设计。

---

## 六、日志现状

**事实（file:line）**

- 8MiB 轮转：`DEBUG_LOG_MAX_BYTES = 8 * 1024 * 1024`（`src/crates/services/debug-log/src/lib.rs:28`）；NDJSON 追加到 `<cwd>/.northhing/debug.log`（`:30-39`）；`rotate_if_oversized` 单代备份 `.1.log`（`:192-206`），在 `append_log_async` 写前触发（`:220-224`）；并发竞态下第二条 rename 失败即**丢该行**，fire-and-forget 语义（`:189-191, 267-269`）。
- 写入面：全产品只有一条通道——桌面 `log_debug_event` → mpsc → 单消费者线程 `log_event`（`src/apps/desktop/src/app_state/log.rs:22-57, 82-118`）。
- 分类：`log_event` 白名单仅 6 个 `COMP_*` 常量——app_lifecycle / session_lifecycle / mode_routing / skill_panel / actor_runtime / ui_dioxus_win（`debug-log/src/lib.rs:258-265`，未知组件归一 `unknown` `:289-297`）。全部是**工程事件**，无一为 agent 行为记录而设。
- 占比实测：活样本 `E:\agent-project\.northhing\debug.log` 共 56 行，component 分布 ui_dioxus_win 36 / actor_runtime 8 / skill_panel 4 / app_lifecycle 3 / session_lifecycle 3 / mode_routing 2 = **100% 工程遥测，0% agent 行为**；消息为窗口开关、heartbeat tick、skill 开关、"user deleted session" 等。agent 行为记录在别处：turns/snapshots JSON（第一节）、episodes JSONL（第四节）、tracing 文本日志（`logs/`，30 天保留 `cleanup.rs:25`）。

**→ 对公理的意义**：「日志不可手动编辑」目前只有工程含义（8MiB 轮转是防爆盘不是防篡改——文件可被随意改写，无完整性保护）；且成长容器叙事里「agent 的自述历史」并不在 debug-log 里，W27 谈日志公理需先区分 debug.log（工程）、tracing logs（运维）、episodes/facts（行为记忆）三套东西。

---

## 七、身份链路复核

**事实（file:line）**

- **config 级 identity.md**（`<config_dir>/northhing/identity.md`，`src/crates/assembly/core/src/agentic/identity.rs:14-23`）：
  - 读路径活：每次构建系统 prompt 都查存在性并注入 `# Self-cognition` 段（`prompt_builder/system_prompt.rs:6, 14-41`，并进 `{PERSONA}` 槽 `:47-56`）。
  - 写路径死——复核成立：`save_identity`（`identity.rs:92-99`）生产零调用方，唯一调用在测试（`session_manager_lifecycle_tests/continuity_selfcheck.rs:222`）；`build_identity_prompt` + `IdentityConfig`（`:6-12, 101-121`，「生成自我认知提示词」管道）零调用方；`clear_identity`（`:85-90`）连测试都没有。真实安装的 identity.md 永远不存在，`# Self-cognition` 永远为空（与 W26 habitability 审查第五节一致）。
  - **唯一例外（复核给出）**：生产中确有一条自动写路径，但它写的是**另一个文件**——工作区根 `IDENTITY.md` 模板由 bootstrap 创建/回填（`service/bootstrap/bootstrap_impl.rs:115, 158, 167` `ensure_markdown_placeholder`，工作区创建 `:105-128` + 每次 prompt 构建前回填 `:131-185`），内容由 BOOTSTRAP 仪式引导 agent 自己填写（`templates/BOOTSTRAP.md`，注入侧 bootstrap 强制提醒 `bootstrap_impl.rs:223-239`）。config 级 identity.md 无任何产品内写者。
- **「北」名字的存储与显示链**：
  - 代码里无「北」字（`git grep 北 -- src` 零命中）——名字是纯数据。
  - 存储：诞生仪式输入 agent_name → 写进 **provider 模型的 `display_name`**（`src/apps/desktop/src/ui_dioxus/api_settings.rs:149-157`）+ 首个会话名（`pages_onboarding.rs:673-699`）。不写 identity、不写 persona 文件、不进核心身份系统。
  - 显示：settings 身份卡显示默认 provider display_name（`pages_settings_cards.rs:88, 129-146`）；聊天发言者标签硬编码「它」（`app.rs:176, 192, 203, 530`）；房间头牌是 locale 固定串（`locales/zh-CN.ftl` dioxus-room-head-name）。名字链在 UI 层三处断（与 habitability 审查第五节 166-168 互证）。
  - 附带：桌面 `AppSettings.identity_md_path` 字段存在但恒 None（`app_state/settings/types.rs:61`、`settings/mod.rs:128`）——又一根未接的线。

**→ 对公理的意义**：「人可赋予印记，不能改写自我」的公理已有仪式文本与（分支上）agent 独占 self-cognition 存储的雏形，但 main 上自我认知文件有读无写、名字落在模型牌子上——W27 的「言语唯一干预」需要的正是把 save 侧交给 agent、把 display 侧接到真名。

---

## 对 W27 计划的五个最关键事实

1. **销毁不原子**：delete_session 只清会话目录+cron+terminal+快照（`session_manager_lifecycle.rs:297-446`），memory.db 事实、episodes JSONL、judge 状态全部存活；`run_file_cleanup` 是无关的全局 janitor（`cleanup.rs:59-88`）；`SessionDeleted` 事件在 facade 被丢弃无人订阅（`kernel_facade/events.rs:357-358`）——「彻底销毁」这条公理的级联清除链需要 W27 从零建，好消息是 facts 的 session/turn provenance 已在库（`kernel_facade/memory.rs:89-90`）。
2. **steering 已是「agent 记得、人不入史」**：注入内容进模型上下文且随上下文快照跨重启持久（`turn_tick.rs:436-458` + `restore_apply.rs:222-251`），但 DialogTurn/UI 历史无它（`save.rs:45-128`），人类侧只剩瞬时 Banner（`app.rs:216-220`）且桌面输入门未开（`app.rs:563-582`）——W27 开门是 UI 工作，内核零改动。
3. **memory 写路径全自动、人类 UI 写入口为零**：蒸馏/刹车/梦/召回强化五类写者全是 agent 侧（`turn_persist.rs:426-563`、`dream.rs:30`、`auto_memory.rs:264,327`），UI 只有只读浏览+导出（`pages_memory.rs`）——「不可手动编辑」在应用面已成立；但无任何按会话/按言语的记忆纠错通道，而言语干预是公理唯一入口，这个通道是 W27 的必答题（现成误报证据：关键词表「别」触发 `facts.rs:237-296`）。
4. **成长代谢整支在 `feat/growth-core-0804`**：self-cognition store（identity.md 一次性迁移、agent 独占写权）、negation 检测（短语表+LLM 二确认，唯一 hard-retire 路径）、竞争组+冷话题抑制+LLM sweep 全在那 36 commits（+11k 行、带 T8/T9 审查整改），a1-a5 五条是其拆条子集可直接废弃；merge-base 落后 main 36 commits、turn_persist 已重构，**收编方式必须是 rebase 式移植 + 重过审查**。
5. **身份链路可接线**：config identity.md 有读无写（写者 `save_identity` 死，唯一活写者/bootstrap 写的是工作区 IDENTITY.md 模板——复核后的唯一例外）；「北」存在 provider display_name（`api_settings.rs:149-157`）、显示三处断（「它」×4、locale「知序」、身份卡=模型名）——W27 若收编 growth-core 的 self_cognition store（T3a/T3b 已实现 store 优先+identity.md 兜底），身份写链和名牌同时接正，且天然符合「人给印记、agent 改自我」。
