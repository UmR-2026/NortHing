# T4c 差距报告（只读侦察，2026-08-07）

- 侦察对象：`E:\agent-project\northing\.worktrees\growth-core-0804`，分支 `feat/growth-core-0804`，HEAD `8b64aa8`（已核对，工作树干净）
- 方法：`git log / git show` + 全量读代码 + `rg` 交叉验证。未改任何源码，未跑测试（brief §证据规则允许的范围）。行号以当前 HEAD 为准，不做任何写入。
- 结论先行：**建议延期——把门面并入 T12**。T4 的实质收益已由 T4a/T4b/R-7/R-2/T6a 拿到；剩下的"四合一门面"是纯形式包装，要动活跃对话收尾主路径，且 T12 必然要重排园丁触发点，现在做等于做两遍。

---

## 1. 执行摘要

1. **T4 的每一条实质内容都已落地，唯独"门面"本身没有。** 调度判定（暂停/计数/自暂停/24h）已经是 crate 纯函数（`scheduler.rs`）；三个裸键已收进成长状态 blob；facts 门禁已是纯函数 `should_distill_facts`；话题升温/降温已接成 `boost_turn_topics`。计划里 T4 想消灭的"散落判定逻辑"今天已经不在宿主代码里了。
2. **代码里不存在 `on_turn_finalized`，也不存在 `on_session_end`。** 全仓 `rg` 零命中（计划 §130 的 `GrowthCore` 签名只是目标架构描述，从未实现）。
3. **"四处 hook"实际上已收敛为"两处显式 hook + 两处内嵌"**：episode hook（`turn_persist.rs:312`）与 facts hook（`turn_persist.rs:324`）仍在收尾函数里显式串联；话题 boost 与 dream sweep 已经嵌进 facts 函数内部（`turn_persist_facts.rs:142/237`）。
4. **当前各环节的耦合关系（回答 brief 问题 1 的核心）**：
   - episode 日志：**完全独立**——只要会话可持久化就走，不受蒸馏成败、不受会话类型影响（R-7 特意声明 episode 不受门禁影响）。
   - facts 蒸馏：受"主会话门禁 + 蒸馏暂停开关"两道闸。
   - 话题升温/降温：**只跟主会话门禁走，不跟蒸馏成败走**（即使暂停/没产出 facts 也照跑）——这是 R-4/R-2 之后的刻意设计（保证 boost/decay 永不失步、暂停期行为与旧版一致）。
   - dream sweep：**双重耦合**——只有"主会话 + 本回合蒸馏出了 facts"才可能触发，还叠了自己的一套 24h 门。这就是 Codex R-2 指出的缺陷：蒸馏没产出 → 园丁永不维护。T12 的 `on_session_end` 独立触发正是要解掉这个耦合。
5. **crate 侧 `decide_turn` / `should_run_garden_sweep` / `record_garden_sweep` 是死代码（生产零调用）。** 24h 门今天由 dream.rs 自己读旧键 `dream_last_sweep_at` 内联实现，没走 crate 纯函数——这是"半完成"的证据之一，也是 T12 要收的尾。
6. **风险对比（问题 5）**：选项 A（现在做 T4c）在"行为回归风险"和"重复劳动"上明显差于选项 B（并入 T12）。理由从真实调用图来：T4c 动的是 `sub_handle_out.rs:366 → turn_persist.rs:273-347` 这条每回合都走的活跃路径（主对话 + 子代理生命周期 4 个调用点），而 T12 本来就要在同一路径上把 dream 拆出去挂到 `on_session_end`——现在包装好、T12 再拆开，等于付两次审查和两次回归测试的钱。
7. **推荐（问题 6）：延期并入 T12**，并在 T12 验收里写死四条兜底条款（见 §7），保证 T4c 不会无声消失。

---

## 2. 当前成功回合的有序生命周期

以下为主对话回合的完整收尾调用图（文件缩写：TP=`turn_persist.rs`，TPF=`turn_persist_facts.rs`，GA=`growth_adapter.rs`，DR=`dream.rs`，DI=`distiller.rs`，SC=`scheduler.rs`，SHO=`sub_handle_out.rs`）。

### 2.1 调用顺序

```
SHO:302-383  spawn 的执行任务
 ├─ SHO:304  execute_dialog_turn(...)          ── 回合执行（LLM 对话）
 │    ├─ 成功 → SHO:319 persist_completed_dialog_turn  (TP:20-98)
 │    ├─ 取消 → SHO:335 persist_cancelled_dialog_turn  (TP:100-174)
 │    └─ 失败 → SHO:349 persist_failed_dialog_turn     (TP:176-271)
 │              （这三者只做会话持久化 + 事件入队 + 通知调度器，不碰成长）
 └─ SHO:366 finalize_persisted_turn_in_workspace_if_needed (TP:273-347)   ← 成长的唯一入口
      │   门槛1: TP:285  !should_persist_session_id → return（禁用持久化 / EphemeralChild 会话 → 全部跳过）
      │   门槛2: TP:290  workspace_path 或 status 为 None → return（watchdog 超时路径会给 None）
      ├─ A: TP:297-308  finalize_turn_in_workspace（会话自身落盘）
      ├─ B: TP:312-322  append_episode_log_entry（TP:351-432）    ← hook 1（episode，无门禁）
      │        └─ 内部 warn-only 早退：PathManager 失败(377) / PersistenceManager 失败(393)
      │           / 回合未找到(405) / 读回合失败(413) / append 失败(427)；status=InProgress 跳过(371)
      └─ C: TP:332-346  facts 门禁
             ├─ resolve_distill_signals (TPF:24-48)  内存优先、持久化元数据兜底；两处都没有 → None
             ├─ should_distill_facts (TPF:54-59)    纯函数；Subagent/EphemeralChild kind、
             │   parent_session_id 非空、created_by 以 "session-" 开头 → false
             ├─ signals=None → warn + 跳过（fail-closed，TP:339-343）
             ├─ false（子代理等）→ debug + 跳过（TP:345）
             └─ true → TPF:334-338  append_facts_entry (TPF:63-238)     ← hook 2（facts）
                   ├─ TPF:82-92  load_last_assistant_text（仅 turn_index>0；warn-only → None）
                   │      └─ TPF:243-313  读上一回合 assistant 文本，TPF:303 截 500 字符
                   ├─ TPF:96    MemoryDb::open；失败 → (run_distill=true, 默认状态) 兜底（TPF:99-102）
                   ├─ TPF:100   begin_distill_turn (GA:229-240)
                   │      └─ 载成长状态(含旧键迁移) → 用户记忆意图唤醒词检测 → should_distill
                   ├─ TPF:108-119 distill_facts_with_llm (DI:57-159)
                   │      └─ 15s 超时；任何失败回落关键词蒸馏；返回 (facts, keywords)
                   │      （run_distill=false 时为空向量）
                   ├─ TPF:122-125 now_ms（墙钟）
                   ├─ TPF:127-129 finish_distill_turn (GA:252-269)     ← 计数必须在这
                   │      └─ record_distill_outcome：turns 无条件+1、命中+1、20 轮 0 命中自暂停
                   ├─ TPF:142-144 boost_turn_topics (GA:322-361)       ← 升温+降温，先于空早退
                   │      └─ LLM keywords 归一化非空 → 用它们；否则回落 extract_topics
                   ├─ TPF:146-148  ⚠️ candidates.is_empty() → return   ← 关键早退
                   │      （此后不再写库、不再 append、不再 dream）
                   ├─ TPF:150-163  逐条记 distiller review（fact_reviews）
                   ├─ TPF:181-223  facts.jsonl 一次性迁移 + 入库（每 workspace 一次，OnceLock 守卫）
                   ├─ TPF:227      append_facts_dedup（JSONL 追加 + 去重，内部 warn-only）
                   └─ TPF:237      run_dream_sweep（DR:30-151）        ← hook 4（dream，最末）
                         ├─ DR:32-35    resolve_memory_llm_client；None → return
                         ├─ DR:39-45    MemoryDb::open；失败 → return
                         ├─ DR:52-62    ⚠️ 24h 门：读旧键 dream_last_sweep_at（自己的判断，
                         │              没走 SC:142 should_run_garden_sweep；没写 GrowthState.garden）
                         ├─ DR:66-76    get_stale_facts（30 天阈值、最多 20 条）；失败 → return
                         ├─ DR:78-81    无过期 → 写 sweep 时间 → return
                         ├─ DR:84-101   7 天 keep 豁免筛选；无候选 → 写 sweep 时间 → return
                         ├─ DR:109-130  LLM 批量判决（15s 超时）；失败/超时 → 仍写 sweep 时间 → return
                         ├─ DR:133-136  空响应 → 写 sweep 时间 → return
                         ├─ DR:140      parse_verdicts(text, n, ["keep","supersede"])（crate 白名单参数化）
                         ├─ DR:143       apply_verdicts (DR:159-223)：supersede→db.supersede_fact+review；
                         │              keep→review；其它→skipped
                         └─ DR:146-150  写 sweep 时间 + info 汇总
```

### 2.2 哪些失败是 warn-only

成长路径全部 warn-only（计划 §7 硬约束）：
- episode 链所有内部失败（TP:377/393/405/413/427）——`warn!` 后 return。
- facts 链：DB 打不开（TPF:99-102 走兜底）、LLM 失败/超时（DI:112/118 回落关键词）、插入失败（TPF:217）、迁移失败（TPF:209 静默丢弃行）、review 记录失败（TPF:161 `let _`）。
- dream 链：每一处（DR:42/73/89/117/124/134）都是 `warn!` 后 return。
- 计数/升温失败：`begin/finish_distill_turn`、`boost_turn_topics` 全部吞错（GA:217 warn 落库、GA:352/359 warn）。
- 注意：`persist_*_dialog_turn` 内部失败用的是 `error!`（TP:52/70/90/122/131/147/165/187/214/239/256），同样不向上传播——只是日志级别更高。

### 2.3 会挡住后续成长的每个早退（按影响排序）

| # | 位置 | 条件 | 后果 |
|---|---|---|---|
| 1 | TP:285 | 会话不可持久化（`enable_persistence=false` 或 EphemeralChild） | episode、facts、boost、dream **全部**不做 |
| 2 | TP:290 | `workspace_path` 或 `status` 为 None（watchdog 超时等路径） | 同上，全部不做 |
| 3 | TP:339-343 | 会话元数据两处都拿不到（内存 + 持久化都 miss） | 跳过 facts/boost/dream；episode 已做。fail-closed 设计（R-7） |
| 4 | TP:345 | 非主会话（子代理等） | 跳过 facts/boost/dream；episode 已做（R-7 安全门禁） |
| 5 | TPF:146 | 蒸馏无产出（LLM 空/回落为空） | **dream 永不触发**（R-2 指出的核心耦合）；review/入库/JSONL 不做；但计数与升温已做完 |
| 6 | DR:32-62 等 | LLM 不可用 / DB 打不开 / 24h 未到 / 无过期事实 | dream 自己内部的正常跳过 |
| 7 | TPF:127/142 | `MemoryDb::open` 失败 | 计数与升温也不做（兜底只救"蒸馏照跑"，救不了计数） |

### 2.4 四件事独立还是耦合（brief 问题 1 的直答）

- **episode 日志：独立。** 不依赖蒸馏、不依赖会话类型、不依赖 dream。唯一依赖是"会话可持久化 + 参数齐全"。
- **facts 蒸馏：独立于 episode，但受两道闸**（主会话门禁 `should_distill_facts` + 暂停开关 `should_distill`）。
- **话题升温/降温：与"facts 成败"解耦，但与"主会话门禁"耦合。** 它挂在 facts 函数里、但在 `candidates.is_empty()` 早退**之前**执行——所以"没产出 facts"的回合照样升温降温。这是刻意设计（TPF:131-141 注释：保证 boost/decay 失步就不会单调膨胀；R-4 之后暂停期回落 extract_topics 与旧版一致）。
- **dream sweep：与"蒸馏成功"强耦合**（见早退 #5），并且自己叠 24h 门。它是整条链的最末一步，任何前面的早退都能挡住它。

---

## 3. T4 需求矩阵（计划 T4 原文逐子句核对）

计划 T4 原文（`plan-2026-08-04-growth-core.md:189-191`）：
> "把 `turn_persist.rs:458-512` 的暂停门/计数/自暂停阈值（20 轮 0 命中）与 `dream.rs:52-62` 的 24h 间隔判断逐字搬为纯函数 `decide`；`turn_persist` 4 处 hook（`:310`/`:324`/`:590`/`:606`）收敛为一个 `GrowthCore::on_turn_finalized`；episode 与 facts 先后顺序不变；`load_last_assistant_text`（`:612`）截 500 字符行为不变；warn-only 语义保持。测试：调度决策表 + core 集成测试断言一次 finalize 仍产出 facts + episode。"

| 子句 | 状态 | 证据（文件/行/提交） |
|---|---|---|
| 暂停门/计数/自暂停阈值搬为纯函数 | **complete** | `SC:108 should_distill`、`SC:164-195 record_distill_outcome`（20 轮 0 命中自暂停、turns 无条件 +1）、`SC:69 DISTILL_AUTO_PAUSE_TURNS`；提交 `1c986a4`（T4a，ledger 已核） |
| 24h 间隔判断搬为纯函数 | **partial** | `SC:142 should_run_garden_sweep` + `SC:72 GARDEN_SWEEP_INTERVAL_MS` 已存在且测试齐全（SC:407-436），**但生产零调用**：DR:52-62 仍内联读旧键 `dream_last_sweep_at` 自判。`decide_turn`(SC:147)/`record_garden_sweep`(SC:261) 同样零调用（`rg` 全仓验证）。T4b ledger 明确"园丁不动、双真相来源待 T12"（progress.md:82） |
| 4 处 hook 收敛为单一 `on_turn_finalized` | **not started** | 全仓 `rg on_turn_finalized|on_session_end` 零命中；TP:312（episode）/TP:324（facts）两处显式 hook 仍在收尾函数里，boost（TPF:142）与 dream（TPF:237）嵌在 facts 内部。计划里"4 处 hook"的行号（:310/:324/:590/:606）已全部过时 |
| episode 与 facts 先后顺序不变 | **complete** | TP:312（episode）先于 TP:334（facts）——与计划一致；S-1 拆分时 judge 做过规范化对比，R-7 声明"Episode logging is unaffected"（progress.md:91）；当前顺序与 1c986a4 前的宿主顺序相同（T4b/S-1 审查结论） |
| assistant-text 截 500 字符不变 | **complete** | TPF:303 `text.chars().take(500)`；crate 侧同参 `MAX_ASSISTANT_TEXT_CHARS=500`（`src/agentic/src/distill/prompt.rs:9`）并有截断测试（prompt.rs:111-122） |
| warn-only 语义保持 | **complete** | 全链只 log 不传播（§2.2 逐条列出）；计划 §7 硬约束在 review 中逐任务核对 |
| 集成证据：一次 finalize 仍产出 facts + episode | **partial / Needs confirmation** | 未发现断言"一次 `finalize_persisted_turn_in_workspace_if_needed` 全链产出 episode + facts"的测试。现有测试是 helper 级：GA 30 个（growth_adapter/tests.rs）、TPF 11 个门禁单测（turn_persist_facts.rs:316-385）、crate SC 22 个纯函数测试。baseline `turn_persist` 12 个是会话生命周期测试（session_manager_lifecycle_tests_ephemeral_lineage.rs 等），属 filter 命中的名称匹配，不是全链集成测试。是否确实缺失需跑一次 `cargo test` 或翻测试清单确认（本报告未跑测试，见 §8） |

**一句话**：T4 的"判定纯函数化 + 状态收敛 + 门禁 + 接线"全部完成；唯一没做的恰恰是它名字里的"收敛为一个门面"——而那部分现在只剩纯形式价值。

---

## 4. 最小诚实 T4c-now 改动集（若现在做）

> 只列方案与边界，不写代码（brief 要求）。

### 4.1 门面放哪：**core 宿主适配层，不进 crate**

理由（硬约束，不是偏好）：
- 门面要调用的操作全是 IO：DB（`MemoryDb`）、文件系统（JSONL facts、`PathManager`）、LLM（`AIClient`）、会话/episode（`SessionManager`、`episodes`）、工作区路径解析。crate 是纯逻辑层：`src/agentic/src/lib.rs:7-8` 明写 "performs no IO"；`src/agentic/AGENTS.md §1` 明写不得依赖 assembly/service/adapter，存储只能经注入端口。
- crate 已有 `EpisodeLog` 端口（`src/agentic/src/ports.rs:198`）但从未接线——那是最小化方向，但把整个门面塞进 crate 需要把所有 IO 抽象成端口，等于把 T4c 做成 T2 的第二次，明显超出"最小诚实"。
- 纯判定已经全部在 crate（SC），门面只需要**编排**，不需要**决策**。编排是宿主职责。

### 4.2 建议的函数边界

- 位置：`dialog_turn/` 下新建一个模块（如 `turn_growth.rs`），**不要**塞进 `growth_adapter.rs`——GA 已 ~740 行（T6a ledger：余量已不多，再扩张必拆）。
- 签名（与现收尾函数的成长部分同参）：
  ```
  on_turn_finalized(
      session_manager, session_id, turn_id, turn_index,
      agent_type, user_input, workspace_path, resolved_session_storage_path,
      status, user_message_metadata,
  ) -> ()            // warn-only，无返回值
  ```
  内部顺序 = 现 §2.1 的 B+C 段原样搬入：episode → 门禁 → facts(计数→升温→入库→dream)。
- 输入为何如此：与 `finalize_persisted_turn_in_workspace_if_needed`（TP:273-347）当前调用完全对齐，零新增参数、零行为歧义。
- **一次调用能不能成？能。** 数据依赖是线性的：distill 产出的 keywords 喂 boost；candidates 非空才进 dream；计数与升温必须在空早退之前。整条链顺序执行、全 warn-only、无跨 task 依赖，一个 async fn 足够。不需要多阶段。
- 哪些移进门面：`append_episode_log_entry`（TP:351-432）+ 门禁块（TP:332-346，含 `resolve_distill_signals`/`should_distill_facts`/`append_facts_entry` 及内部全部）。
- 哪些必须留在外面：`finalize_turn_in_workspace`（会话自身落盘，非成长）；`should_persist_session_id` 与 `(wp, st)` 两重门槛（TP:285/290，属会话持久化守卫，不是成长）；`persist_*_dialog_turn` 三兄弟（事件/通知，非成长）。
- 结果形态：TP:273-347 的 B+C 两段收缩成一行 `Self::on_turn_finalized(...)`，TP 减小，新模块承接。

### 4.3 需要的等价性测试（证明行为等价 + 不重复执行）

| 测试 | 断言 |
|---|---|
| 顺序等价 | 一次调用后，episode 写入发生在 facts 写入之前（可用假 EpisodeLog 端口记录顺序） |
| 单次性/不重复 | 一次调用恰好写 1 条 episode、1 批 facts；不存在"旧调用 + 新门面"双写（这是重构期最常见的回归） |
| 门禁保持 | 子代理信号 → 无 facts、无升温、无 dream，episode 仍写（R-7 语义钉死） |
| 空产出保持 | candidates 空 → 计数 +1、升温仍执行、dream 不触发（TPF:146 语义钉死） |
| 暂停保持 | paused → 不蒸馏、仍计数、升温回落 extract_topics（R-2/R-4 语义钉死） |
| dream 24h 门保持 | 未到 24h → 不 sweep（dream 自身逻辑不受门面影响） |
| 既有回归 | crate 165、GA 30、TPF 11、TP/session 12 全绿；`cargo check` warnings 19 不增 |

### 4.4 预估改动量（给区间，不给假精确值）

- 生产文件 **2 个**（新模块 1 + `turn_persist.rs` 收窄），若坚持放 GA 则变成 1 个文件但会顶穿 800 行上限——不建议。
- 测试文件 **2~3 个**（新模块的等价测试 + 既有测试微调）。
- 净 diff **约 200~450 行**（生产 ~120~250，测试 ~80~200）。不含任何 T12 内容。

---

## 5. 与 T12 的重叠矩阵（brief 问题 4）

T12 必改的六件事（计划 `:244-251` + R-1/R-2 修订）与每个 T4c 候选改动的命运：

| T12 必改项 | 对 T4c 的冲击 |
|---|---|
| **dream→garden 语义重写**（去 supersede、四动作 + R-1 画像摘要第五动作） | T4c 门面若调 `run_dream_sweep` → **modified again**（调用点保留、被调方整体替换） |
| **候选筛选迁入 crate**（中性输入类型 + 三参数登记 + 以当前时刻入参的纯函数，计划 `:249`） | 同上，dream 调用点的参数/位置会再变一次 → **modified again** |
| **`on_session_end` 独立触发园丁**（R-2：与蒸馏成败解耦；该入口现零调用方） | T4c 门面若把 dream 收进"每回合一次"的调用里 → **deleted by T12**（T12 必须把它从回合路径拆出去挂到会话结束点，否则 dream 会每回合/每会话双重触发）。这是 T4c-now 与 T12 唯一**结构性冲突** |
| **状态迁移：`dream_last_sweep_at` 旧键 → `GrowthState.garden`**（T4b 已声明留待此步，progress.md:82） | T4c 不碰状态键则无冲突；若 T4c 顺手"接线" `record_garden_sweep` → **deleted by T12**（提前制造第二个真相来源） |
| **画像摘要（R-1）** | 只影响 dream.rs 内部，与门面无关 → 无冲突 |
| **supersede 边界规则**（T7a 转入：同提交补 forbidden-rules 并证明触发） | 只影响 dream.rs + crate，与门面无关 → 无冲突 |

T4c 中**可被 T12 原样复用**的部分：
- 门面包装 episode + facts + 升温/降温（不含 dream）→ **reused unchanged**（T12 不碰蒸馏调度、不碰话题升温）。
- 主会话门禁 `should_distill_facts` + `SessionSignals` → **reused unchanged**。
- 顺序（episode→facts）、500 截断、warn-only → **reused unchanged**。

**结论**：T4c-now 的"安全子集"（不含 dream）会被 T12 完整保留；T4c-now 的"完整版"（含 dream）会有 1 处被 T12 删除重做。而"不含 dream 的 T4c"恰好就是"形式大于内容"的部分——因为它把已经由 T4a/T4b/T6a/R-7/R-2 拿走的收益再包装一层，没有新增任何能力。

---

## 6. 选项 A（现在做 T4c）/ 选项 B（并入 T12）风险对比

评分依据全部来自 §2 的真实调用图，特别是：活跃收尾路径（SHO:366 → TP:273-347，5 个调用点：主对话 1 + 子代理生命周期 3 + cleanup 1）、早退 #1-#7、升温/降温的单次性、dream 的单次触发、facts/episode 写入的不可丢。

| 维度 | A：现在做 T4c | B：并入 T12 |
|---|---|---|
| 行为回归风险 | **high**——动每回合活跃路径；5 个调用点里任何一处 status/参数语义不一致就会丢 facts 或 episode 或造成双写；R-7 的 fail-closed 门禁和 R-2 的暂停恢复都是这条链上钉死的语义，重包一层必然重审重测 | **low**——现在不动它；T12 本来就必然动这条路径（拆 dream、加 on_session_end），回归测试在 T12 一次性做，且 T12 自带集成测试纪律（四动作 + 负向测试） |
| 重复劳动 | **medium**——若门面含 dream，T12 必须拆掉（deleted）；若不含 dream（安全子集），复用度高但"包装层"仍要在 T12 时因 dream 调用点挪走而重新过一遍等价测试 | **low**——只写一次门面，而且是在园丁最终形态上写，不返工 |
| 审查负担 | **medium-high**——重构活跃路径（等价重构审查成本高，m3 大 diff 会挂，见 handoff §5 模型画像） | **medium**——T12 本来就大，多包一层门面是增量；一次审查覆盖"功能 + 重构 + 边界规则"内聚变更，两次审查变一次 |
| 回滚清晰度 | **medium**——T4c 是纯重构，回滚干净；但 T4c + T12 两轮变更在 dream 触发点上互为反义（先收进来、再拆出去），回滚历史留下两段互斥代码 | **medium-high**——T12 整体是一个原子提交（计划 `:247` 要求 supersede 规则同提交），回滚就是回滚整个 T12；门面不会独立残留在半吊子状态 |
| 对 G2 价值的延迟 | **low**（两边都是）——T4c 不产生任何 G2 功能，也不阻塞 T8-T13。G2 的价值全部来自 T8-T11 的语义工作，与门面无关 | **low**——同上 |

**专项核对（brief 要求逐条过）**：
- 活跃对话收尾路径：A 直接改它；B 现在不改、T12 必须改——A 的风险发生在"无关重构"上，B 的风险发生在"本就需要的改造"上。
- 早退：两种选项都必须保住 #1-#7 全部早退语义；但 A 多做了一次"保住它们"的验证成本，B 只在 T12 验证一次。
- 重复升温/降温：A 的风险点是"旧调用 + 新门面"并存造成每回合 decay 两次（权重 0.99² 加速冷却）——这是重构期最容易出现的静默回归，需要专门测试防；B 不存在此风险（没有第二份调用）。
- 重复 dream：A 若把 dream 收进门面，T12 再加 on_session_end 时若不同步删回合内触发，dream 会"每回合 + 每会话"双跑（LLM 成本 ×2 + sweep 时间键互相覆盖）；B 从设计上只有 on_session_end 一个触发点。
- facts/episode 丢失：两种选项都要求不丢；但 A 的丢法（新门面少传一个参数、漏一个状态）是纯新增风险，B 的丢法只可能来自 T12 自身的行为变更（有 T12 验收兜底）。

---

## 7. 推荐与 T12 兜底验收条款

**推荐：选项 B——延期，把门面并入 T12。** 理由浓缩为三句：

1. T4 要消灭的"散落判定"已经消灭（T4a/T4b/R-7/R-2/T6a 逐条证据在 §3），门面现在是纯形式。
2. 门面要动的是本仓库风险最高的每回合收尾路径，而 T12 在同一路径上的改造（dream→garden、on_session_end）会与"把 dream 收进门面"的 T4c 版本**正面冲突**（§5 唯一一条 deleted by T12）。
3. 现在做 A 版门面 → T12 拆 → 两次审查、两次等价测试、一次双触发风险，收益为零新增。

**T12 验收必须追加（否则 T4c 会无声消失——T4c 不在任何 T12 验收里 = 被遗忘）：**

1. **门面落地条款**：T12 提交内必须有单一回合收口入口（`on_turn_finalized` 或等价命名），将 episode → facts → 园丁（garden）按序编排；且 episode→facts 顺序、500 字符截断、warn-only、主会话门禁（`SessionKind`）四条行为各有测试证据。这条同时覆盖原 T4"四合一"与 R-2"维护与写入解耦"两处计划文本。
2. **独立触发条款**：园丁由 `on_session_end` 独立触发，与蒸馏成败无关（R-2 修订原文）；`rg on_session_end` 在 T12 完成后必须有生产调用方（现在是零）。若园丁仍保留回合内触发，必须证明与 `on_session_end` 不会双跑（提供防重入/单次触发的测试）。
3. **状态迁移条款**：`dream_last_sweep_at` 旧键的写入方改为 `GrowthState.garden`（`record_garden_sweep` 上线），且迁移后无双真相来源（dream.rs 不再自读旧键）。
4. **候选筛选条款**：原 T4/T5b 已并项的三参数（`STALE_THRESHOLD_MS`/`MAX_STALE_FACTS`/`DREAM_KEEP_EXEMPTION_DAYS`）以中性输入类型迁入 crate 并登记 `src/agentic/AGENTS.md §4`（计划 `:249` 原文）。

若执行者倾向"删掉 T4c 这个词"而不是"并入 T12"：请同步改写计划 T4 节（把"收敛为一个 `on_turn_finalized`"标记为**并入 T12**，引用上面第 1 条验收），否则 T4 文本会一直像"有未完成任务"。两条路等效，但**必须落在计划文本里**，不能只靠本报告。

---

## 8. Needs confirmation

1. **"一次 finalize 全链集成测试"是否存在**（§3 最后一行）。依据现有测试清单（GA 30 / TPF 11 / SC 22 / session 12）与代码阅读，判定"疑似不存在"，但没有跑测试、没有逐条翻全量测试列表。若用户在意这条，下步应 `cargo test -p northhing-core --features product-full` 后按测试名核对。
2. **watchdog 超时后，spawn 后台任务是否必然完成 finalize**（SHO:390-405）。从代码看 finalize 在 spawn 任务内、select 超时只放弃外层等待；但若执行引擎卡死不返回，后台任务的 finalize 也不会执行。这是运行时行为，静态读码无法 100% 断定。
3. **`turn_persist` 12 个测试的具体内容**。我用的是 handoff 基线数字，未逐条读（它们分布在 session lifecycle 测试族里，与本报告结论无直接依赖）。
4. **`should_persist_session_id` 的 `unwrap_or(true)` 分支**（session_manager_persistence_predicate.rs:77-84）：会话从内存消失时默认"可持久化"→ 收尾照走。这个兜底与"fail-closed"原则方向相反，是否是有意设计需确认（现状如此，本报告只描述不评判）。

---

## 9. 证据索引

**工作树文件（`E:\agent-project\northing\.worktrees\growth-core-0804\` 下）：**
| 路径 | 行 | 内容 |
|---|---|---|
| `src/crates/assembly/core/src/agentic/coordination/dialog_turn/turn_persist.rs` | 285/290 | 两个收尾门槛 |
| 同上 | 310-322 | "Hook: distill episode"注释 + episode 调用（hook 1） |
| 同上 | 324-346 | "Hook: distill facts"注释 + facts 门禁（hook 2） |
| 同上 | 351-432 | `append_episode_log_entry` 全函数 |
| `.../dialog_turn/turn_persist_facts.rs` | 24-59 | `resolve_distill_signals` / `should_distill_facts` |
| 同上 | 63-238 | `append_facts_entry` 全函数（begin/finish/boost/空早退/dream） |
| 同上 | 243-313 | `load_last_assistant_text`（303 行 500 字符截断） |
| 同上 | 316-385 | 门禁单测 11 个 |
| `.../agentic/growth_adapter.rs` | 229-269 | `begin_distill_turn` / `finish_distill_turn` |
| 同上 | 322-361 | `boost_turn_topics`（LLM keywords 优先 + 回落 + 升温/降温成对） |
| `.../service/agent_memory/dream.rs` | 30-151 | `run_dream_sweep`（24h 门 52-62、keep 豁免 84-101、parse 140、apply 159-223） |
| `.../service/agent_memory/distiller.rs` | 57-159 | `distill_facts_with_llm`（回落语义） |
| `.../service/agent_memory/mod.rs` | 13-15 | 导出（`run_dream_sweep` 仅此处导出，无后台调度调用） |
| `.../agentic/coordination/dialog_turn/sub_handle_out.rs` | 302-383 | 主对话回合收尾（persist→finalize 串联） |
| `.../agentic/coordination/subagent_orchestrator/so_lifecycle/lifecycle.rs` | 41-76/327-336 | 子代理路径 finalize 调用 |
| `.../agentic/coordination/subagent_orchestrator/so_lifecycle/cleanup.rs` | 42/108 | cleanup 路径 finalize 调用 |
| `src/agentic/src/scheduler.rs` | 69-72/108/132/142-152/164-195/261 | 纯函数 + 常量（`decide_turn` 等零生产调用） |
| `src/agentic/src/state.rs` | 14/33-35 | `LEGACY_KEY_DREAM_LAST_SWEEP` / `GardenCursor` |
| `src/agentic/src/lib.rs` | 7-8 | "performs no IO" 契约 |
| `src/agentic/src/ports.rs` | 198 | `EpisodeLog` 端口（未接线） |
| `src/agentic/AGENTS.md` | §1/§3/§4 | 层位置、无 supersede、参数登记 |
| `src/agentic/src/garden/cleanup.rs` | 1 | "Filled by task G2-T12"（占位确认） |

**提交（`git log`/`git show` 核对）：**
| 提交 | 内容 |
|---|---|
| `1c986a4`（T4a） | scheduler.rs 纯函数（`git show --stat`：+325） |
| `985bbb9`（T4b） | 状态 blob 接线（`git show --stat`：turn_persist.rs -51/GA +132；ledger 记"未写 GrowthState.garden"） |
| `27c9738`（T6a） | boost/decay 接线 |
| `6365cf5`（R-7） | facts 主会话门禁（fail-closed + SessionKind） |
| `d1d6d92`（R-2） | 暂停恢复 + 命中窗口重置 |
| `38d1e8d..c3d2b31`（S-1） | turn_persist / turn_persist_facts 拆分（S-1 即 T4 现场拆分） |
| `8b64aa8`（T5b，HEAD） | `parse_verdicts` 白名单参数化 |

**计划/台账文档（`E:\agent-project\northing\.superpowers\sdd\`）：**
| 文档 | 用途 |
|---|---|
| `plan-2026-08-04-growth-core.md:189-191` | T4 原文（需求矩阵对照源） |
| `plan-2026-08-04-growth-core.md:244-251` | T12 原文（T7a/T5b 转入 + R-1/R-2 修订） |
| `plan-2026-08-04-growth-core.md:347-357` | R-1（画像摘要）/ R-2（on_session_end 独立触发） |
| `plan-2026-08-04-growth-core.md:277-292` | 全局约束（warn-only、无 supersede、纯函数、行数上限） |
| `progress.md:77-83` | T4a/T4b 完成记录 + 两条延后裁定（园丁不动、四合一入口延后=T4c 待办） |
| `progress.md:91/101/114` | R-7/R-2/S-1 记录 |
| `handoff-2026-08-06.md:26-41` | 交接时的 T4c 侦察结论与三选项 |
