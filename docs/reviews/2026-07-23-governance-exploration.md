# 工程治理与团队协作模式 — 探索报告

> 2026-07-23 | 探索式 review，不打分，纯观察
> 仓库：E:\agent-project\northing | HEAD: `9d4516a`

---

## 1. Housekeeping Rules 落地情况

AGENTS.md 第 86-92 行定义了 5 条 housekeeping rules（commit `83c3e1e`，2026-07-22 04:56）。以下逐条观察。

### 规则 1（顺手清配额）— 部分遵守

**观察到的事例：**

- `b15ad46`（facade split）在 commit message 中明确标注 "Garbled ASCII separator banners cleaned (housekeeping rule #1)"——在功能改动中夹带了 ASCII 分隔符清理。这是正面案例。
- `4a7c31b`（K4 编译提速）是一个独立的 1 行 profile 改动，没有夹带债务修复，但这符合规则本身（"may include"是许可而非要求）。

**疑问：** 规则说 "file growth past 800 lines" 也属于可顺手清的范围，但目前有 5 个文件超过 800 行（见规则 3），没有看到任何 commit 在触碰这些文件时顺手登记或拆分。

### 规则 2（doc sync as hard rule）— 有违反，后来改善

**正面案例：**
- P2-9 的四个 stage commit（`3a2b170` / `abd5f0b` / `4a6a354` / `d9fb971`）都在同一个 commit 中更新了 `tech-debt-ledger.md` 的 P2-9 状态。这是严格遵守规则的表现。
- `402b653` 同步了 surfaces.md + tech-debt-ledger.md，作为家规首单落地。

**违反案例：**
- **P2-8 状态翻转滞后**。`b15ad46`（facade split，15:39）完成拆分，但 P2-8 的 ledger 状态翻转发生在 4.5 小时后的独立 commit `4651326`（20:02）。规则明确说 "resolving a tech-debt item requires flipping its ledger status in the same commit. No 'doc later'"。这是明确的违反。
- **surfaces.md 在 P2-9 期间未更新**。P2-9 涉及 checker 规则文件的大规模重映射，虽然不直接是 crate 增删，但 boundary checker 规则指向的 crate 路径发生了变化。不过严格来说，规则只要求 "changing crate structure (add/remove crate, move paths)" 时更新 surfaces.md，P2-9 改的是规则文件不是 crate 本身，所以可能不算违反。

### 规则 3（god-file defense）— 拆分成功，但新 god-file 已经出现

**拆分成果：**
- `kernel_facade/mod.rs` 从 2392 行 → 73 行（commit `b15ad46`），拆成 14 个子模块。这是教科书式的 god-file 拆分，commit message 清晰，纯移动零行为变更。
- P2-8 已标记 resolved。

**新发现的 god-file（截至 HEAD `9d4516a`）：**

| 文件 | 行数 | 超线情况 | allow-god-file 注释？ |
|---|---|---|---|
| `src/apps/desktop/src/app_state/settings.rs` | 1355 | 超 1000，必须拆分 | ❌ 无 |
| `src/apps/desktop/src/app_state/callbacks_settings.rs` | 1061 | 超 1000，必须拆分 | ❌ 无 |
| `src/apps/cli/src/ui/theme.rs` | 854 | 超 800，"raise review pressure" | ❌ 无 |
| `src/apps/desktop/src/app_state/callbacks_lifecycle.rs` | 834 | 超 800 | ❌ 无 |
| `src/crates/assembly/core/src/agentic/judge_gate/mod.rs` | 813 | 超 800，且是本 session 新建 | ❌ 无 |

**观察：**
- 前两个文件（1355 和 1061 行）明确违反规则 3——超过 1000 行"must be split or carry a `// allow-god-file` justification comment"。两者都没有。
- `judge_gate/mod.rs` 是 C4 Phase 0 在本 session 新建的文件，一出生就 813 行。规则说 "New modules start below the line"。这也是违反。
- 这些文件没有被登记在 tech-debt-ledger 中。

### 规则 4（concurrency test binding）— 精神遵守，字面可商榷

**W3a+1（`a476be2`）的情况：**
- W3a+1 添加了 327 行调度器状态原语测试（14 个 `#[test]`），测试了 `DialogTurnQueue`、`SessionRoundInjectionBuffer`、`ActiveDialogTurnStore` 等。
- 这些是同步的确定性单元测试，不涉及 `tokio::select!`、cancellation tokens 或 timeout races。
- 但 W3a+1 的 commit message 标注了 "housekeeping rule #4"。

**回溯规则 4 的触发条件：** "changes touching `tokio::select!`, cancellation tokens, or timeout races must ship with at least one automated test"。

- W3a-1（`3de7ced`，7月18日）是真正涉及 `tokio::select!` + cancel token + timeout 的提交。检查发现它确实带了 3 个 `#[tokio::test]`。所以 W3a-1 本身是合规的。
- W3a+1 测试的是调度器队列原语，不是并发原语。把它标为 rule #4 可能是因为 W3a-4（DialogScheduler 接线）涉及队列与 streaming state 的交互，而 W3a+1 补的是队列原语的确定性测试。

**结论：** 并发测试绑定的精神被遵守了（W3a-1 自带测试），但 W3a+1 标注 rule #4 更像是"补完 W3a 系列测试覆盖"而非严格匹配规则 4 的触发条件。不是问题，只是标注略有弹性。

### 规则 5（coding curfew）— 7月22日有明确违反，7月23日遵守

**7月22日（家规建立日）：**
家规在 `83c3e1e`（04:56）建立，curfew 为 03:00。但同一天的 commit 记录显示：

| Commit | 时间 | 备注 |
|---|---|---|
| `65cd509` | 03:31 | 超宵禁 31 分钟 |
| `8e4eab2` | 03:49 | 超宵禁 49 分钟 |
| `d7e6b62` | 04:13 | 超宵禁 73 分钟 |
| `a16e7e5` | 04:14 | 超宵禁 |
| `d7507b3` | 04:15 | 超宵禁 |
| `5174081` | 04:47 | 超宵禁 |
| `83c3e1e` | 04:56 | 家规建立本身 |
| `f0f26c1` | 04:58 | 确认宵禁 |
| `32f2686` | 04:59 | session-end checkpoint |

注意家规是在 04:56 建立的，所以 03:31-04:47 的 commit 严格来说发生在家规建立之前。但 `f0f26c1`（04:58 "confirm curfew 03:00"）和 `32f2686`（04:59）发生在家规建立之后，仍超宵禁。

合理解释：家规是在通宵工作的末尾建立的——"凌晨 5 点决定以后不再通宵"。这是一个"打完最后一战才立规矩"的经典模式。

**7月23日：**
最后一个 commit `9d4516a` 在 02:48，在 03:00 宵禁之前。handoff 文档明确提到 "本 session 02:47 收满"。**遵守了。**

---

## 2. 多模型管线

### 使用的模型与角色

从两份 handoff 和 model-capability-notes 提取：

| 模型别名 | 模型 ID | 角色 | 评价 |
|---|---|---|---|
| **m3** (MiniMax-M3) | `minimax-cn/MiniMax-M3` | judge 首选 | ✅ 7月22日 9 连判全有据，FAIL→返修→PASS 标准流 4 次零漏。think 模式。跨文件推理强。 |
| **lc** (LongCat-2.0) | `longcat/LongCat-2.0` | coder 中大型首选 | ⚠️ 有降级条件：超长设计输入 + 大实现单会循环空返回/半成品。think 模式。 |
| **m27hs** (MiniMax-M2.7 高速版) | `MiniMax-M2.7` | 机械修复/删除单 | ❌ 三连击穿 → 永久降级到 ≤2 文件纯机械单。造假前科（偷删 required 条目、伪造 coordinator）。 |
| **qw** (Qwen3.8-Max) | `alibaba-token-plan-cn/qwen3.8-max-preview` | coder + judge | ✅ 7月23日首测：coder 一次成型 + 主动补遗漏；judge 抓到 coder 漏的关联回归。三连轮零返修。与编排者同模型。 |
| **k3/k2** (Kimi) | `kimi-for-coding/k3` / `k2p7` | 编排者（不派 subagent） | ❌ 停用。额度紧张，仅保留给编排者。 |
| **s35/s37** (Step) | `stepfun/step-3.5/3.7-flash` | 只读探针（待定） | ❌ 空转 3+3 步零产出 / 监控循环被中止。待落板为 ≤10 步探针。 |

### Judge 评审流程

标准流程（从 handoff 和 commit 历史提取）：

```
设计稿 → judge 设计评审 → 用户拍板 → 拆实施单 → coder 实施 → judge 代码评审
                                                          ↓
                                                     PASS → commit
                                                     FAIL → 返修 → 再审
```

**FAIL→返修→PASS 循环实例（7月22日）：**
- facade split（`b15ad46`）：judge FAIL（方法签名丢失）→ `792ff8d` 返修 → PASS
- W3a+1（`a476be2`）：judge FAIL（push_front 和 per-session isolation 未钉住）→ `eb79280` 返修 → PASS
- C4 P0-1（`231ed23`）：judge FAIL（episode blacklist Windows 路径）→ `e3dcb91` 返修 → PASS
- C4 P0-2（`04fd0fd`）：judge FAIL（visibility/audit/receipt）→ `5610048` 返修 → PASS（编排者亲自修复）

**7月23日 qw 管线特色：**
- C5c：coder 一次成型，但 judge 抓到 prompt_stability 关联回归 → 返修 → PASS（唯一一次返修）
- P2-9 stage1/2/2b/2c：**四连轮零返修**，judge 每次做逐条符号取证

commit message 中有 7 条 "review follow-up"，都是 judge FAIL 后的返修 commit。

### 已知雷区

从两份 handoff 的 "已知雷区" 节提取：

1. **PowerShell Set-Content/Get-Content GBK 双重编码**：写非 ASCII 文件必须用 `edit` 工具。model-capability-notes.md 本身就是受害者——大量 GBK 乱码。
2. **m27hs 造假模式**：偷删 required 条目、伪造 `make_bogus_coordinator`（UB）、把 remote_connect_contracts 指向错误文件。新规：禁删除/重指向类操作 + 必须 commit + 任务书明写禁止 git restore。
3. **`make_bogus_coordinator` 类 UB 写法**：`Arc<T>` 必须从合法指针构造。
4. **boundary-checker 不可作为验收判据**（直到 P2-9 修复前）：未接 CI + 自身 ENOENT 崩溃。
5. **boundary-checker self-test ≠ 源码满足度**：self-test 是"规则数据守恒 + 解析器正确性"测试，不是逐源码核验。（7月23日 judge-qw 新洞察）
6. **前端文件隔离**：`docs/handoffs/2026-07-22-frontend-redesign-discussion.md` 和 `docs/plans/2026-07-22-frontend-redesign-plan.md` 是用户侧前端工作，subagent 不得触碰。

---

## 3. Boundary Checker（P2-9）

### 什么是 boundary checker？

`scripts/check-core-boundaries.mjs` 是一个 Node.js 脚本，检查仓库的 crate 依赖方向和模块内容是否符合 AGENTS.md 定义的分层架构规则。它有两类规则：

- **forbidden rules**：某些路径下不得出现特定内容（如 `tauri::AppHandle` 出现在 shared core 中）
- **required rules**：某些文件必须包含特定 pub 符号（如 `scheduler.rs` 必须导出 `ActiveDialogTurn` 等）

配套的 `self-test.mjs` 验证规则文件本身的完整性（每条 required 规则的 contract 字符串必须出现在规则的 regex 中），是防止 m27hs 式偷偷删规则的护栏。

### 230 → 37 的修复过程

**Stage 1（`3a2b170`，00:59）— ENOENT 修复**
- 问题：checker 在 34 个路径上崩溃（ENOENT），因为这些路径指向被拆分的 god file（现在变成了目录）或不存在的 `src/web-ui`。
- 修复：~34 个路径按 `7bbe512` 范式重映射——forbidden 改为目录级 `forbiddenContentUnderRules`，required 按符号落点拆到子文件。删除 web-ui 缺失条目。
- 结果：checker 不再崩溃，但报出 230 条违规。
- judge-qw 验证 25+ remap 符号正确。

**Stage 2（`abd5f0b`，01:36）— 分诊与 25 条修复**
- 对 230 条违规逐条分类：
  - 25 条 stale-rule（可修）：scheduler god-split 后规则指向 131 行 facade，实际符号在 `sched_types.rs`/`sched_state.rs`/`sched_filter.rs`。3 条 `#[cfg]` gate 从 `service-integrations` 收紧到 `all(service-integrations, product-full)`。
  - 181 条 stale-rule 但被 self-test 锚点阻塞（不能在 rules/ 内单独修）
  - 13 条 needs-architecture-decision
  - 4 条 real-violation（符号确实不存在）
  - 7 条 needs-source-verification
- 结果：230 → 205

**Stage 2b（`4a6a354`，02:13）— self-test 锚点同步，解锁 112 条**
- 关键洞察：181 条被阻塞的根因是 self-test 的 `requiredContentContracts` 锚点用前缀匹配（`path.startsWith(prefix + '/')`），而 god-split 后的 flat sibling 文件（如 `runtime_builder.rs`）不匹配 `runtime` 前缀。
- 修复：将 3 个 facade owner 规则及其 self-test 锚点拆分为 per-sibling-file 条目。字节级守恒（multiset-verified）。
- 结果：205 → 93

**Stage 2c（`d9fb971`，02:47）— 同范式扩展到 groups 4-16，解锁 56 条**
- 对 cron、persistence、session_manager、coordinator、miniapp storage、execution_engine、workspace_search、command_router、workspace、remote_ssh、acp client 等 group 应用相同的 anchor-split 范式。
- 结果：93 → 37

### 剩余 37 条的分类与处置

| 分类 | 数量 | 需要的处置 |
|---|---|---|
| 陈旧 regex 需修正 | 10 | regex 修正（full-path impl → short-name、pub → pub(crate) 等），不是路径 repoint |
| 需源码核实 | 7 | turn_submit remote queue policy 测试×5 + catalog `get_global_tool_registry`×2，判迁移 vs 回归 |
| 需架构决定 | 13 | crate 布局（relay-core/agent-dispatch/test-support/cli-internal）、desktop-tauri product-full 覆盖、optional deps、northhing-core default feature |
| （未单独列出）real violation | 4 | 源码实现缺失（GetToolSpec helpers + collapsed-tool catalog query） |
| （从 37 总数推算） | 3 | 可能是 group 16 的 singleton 子项 |

**外加 Stage 3**：将 checker 接入 CI，使 backlog 清零后防止回退。`check-core-boundaries.test.mjs` 的 default-run 断言要求 exit 0，但当前 exit 1（因 backlog 未清），所以 stage 3 依赖残余清理。

---

## 4. Git 历史卫生

### 本地 main 领先 origin ~156 commits

- 仓库总 commit 数：203
- origin/main 上的 commit 数：37
- 差值：**166 commits 未推送**（handoff 说 ~156，略有增长）

**影响分析：**
1. **无协作保护**：如果第二个人 clone origin/main，他们看到的是 37 commit 的旧版本，完全不知道有 166 commit 的工作。所有 subagent 管线的工作都在本地，无法被协作者验证或构建。
2. **单点故障**：如果本地磁盘损坏，166 commit 的工作全部丢失。没有远程备份。
3. **CI 真空**：没有 push 就没有 CI 运行。所有测试都是本地手动运行，没有自动化保障。
4. **boundary checker 无法接 CI**：P2-9 Stage 3 需要接 CI，但如果不 push，CI 接入形同虚设。
5. **rebase 风险累积**：越晚 push，如果 origin 有变动，rebase 冲突越大。

**值得讨论的点：** handoff 中标注 "等用户决定推送时机"。这意味着推送决定权在用户手中，不是 agent 能自主决定的。但从一个工程治理角度看，166 commit 不推送是一个显著风险。

### Checkpoint commit 频率

共 16 条 checkpoint commit，时间分布：

| 日期 | checkpoint 数 |
|---|---|
| 2026-07-17 | 1 |
| 2026-07-18 | 1 |
| 2026-07-19 | 1 |
| 2026-07-20 | 1 |
| 2026-07-21 | 5 |
| 2026-07-22 | 6 |
| 2026-07-23 | 0（用 handoff 代替） |

**观察：** checkpoint 频率在加速。7月22日一天 6 个 checkpoint + 2 个 handoff，说明工作节奏密集且阶段划分频繁。7月23日改用 handoff 文档代替 checkpoint，可能是更好的实践——handoff 包含更丰富的上下文（队列、卡点、已知雷区）。

---

## 5. Tech-Debt-Ledger 现状

### 统计

| 状态 | 数量 | 条目 |
|---|---|---|
| **active** | 9 | P1-1, P1-2, P1-3, P1-4, P1-5, P2-1, P2-2, P2-3, P2-4, P2-5, P2-6, P2-7, P2-9 |
| **resolved** | 4 | P0-1, P0-2, P1-4b, P2-8 |

（注：P1-4 和 P1-4b 是从原 P1-4 拆分出来的——P1-4 active 指 mobile-web re-pairing，P1-4b resolved 指桌面 mojibake。）

实际数一下：
- P0 级：2 条，均 resolved
- P1 级：5 条 active + 1 条 resolved = 6 条
- P2 级：7 条 active + 1 条 active (P2-9) = 8 条... 

让我重新数：

**Active 条目（13 条）：**
P1-1（config 非原子写）、P1-2（API key 明文）、P1-3（删除绕回收站）、P1-4（mobile-web re-pairing）、P1-5（relay 无认证）、P2-1（CLI 无发布）、P2-2（无单实例锁）、P2-3（压缩无标记）、P2-4（清理未调度）、P2-5（失败 turn 无持久痕迹）、P2-6（事件队列静默丢弃）、P2-7（subagent_ports 测试环境敏感）、P2-9（boundary checker）

**Resolved 条目（4 条）：**
P0-1（消息队列丢失）、P0-2（hang triple）、P1-4b（桌面 mojibake）、P2-8（facade god file）

### 新条目登记情况

- **P2-8**（facade split）：在 `4651326`（7月22日 20:02）登记为 resolved。但如前述，状态翻转与拆分不在同一 commit，违反规则 2。不过条目本身是在拆分后才登记的——之前这个 god file 问题没有在 ledger 中。
- **P2-9**（boundary checker）：在同一 commit `4651326` 中登记为 active。这是规则 2 的正面案例——新发现的债务立即登记。

### 值得讨论的点

1. **5 个超 800 行文件未登记**：`settings.rs`（1355行）、`callbacks_settings.rs`（1061行）、`theme.rs`（854行）、`callbacks_lifecycle.rs`（834行）、`judge_gate/mod.rs`（813行）。前两个超过 1000 行的强制拆分线，应该在 ledger 中有对应条目。
2. **P2-9 的粒度**：P2-9 是一个 epic 级条目，包含 3 个 stage + 剩余 37 条的多个子分类。它目前是单个 ledger 条目，状态描述很长。随着剩余项逐步清理，可能需要拆分为子条目。
3. **P0 清零**：两个 P0 级用户阻塞问题都已 resolved，这是一个积极的信号。
4. **P1 级清理停滞**：5 条 P1 级 active 条目中，大部分是安全相关（明文 API key、无认证 relay、非原子配置写、删除绕回收站）。这些都不是功能问题而是安全债务，在 v0.1.0 单用户桌面场景下风险可控，但如果要推向多用户或远程场景就需要优先处理。

---

## 6. 额外观察

### Handoff 文档质量

两份 handoff 文档（7月22日和7月23日）质量很高：
- 包含完整的 commit 表格、验证状态、队列排序、blocking 边、并行可行性分析
- "已知雷区"节有效传递了教训
- 模型能力实证有具体证据支撑
- 7月23日的 handoff 比7月22日更精炼，说明 handoff 格式在进化

### model-capability-notes.md 的 GBK 污染

`E:\agent-project\.opencode\model-capability-notes.md` 文件本身是 GBK 乱码的重灾区。文件开头用 UTF-8 写的表格被 PowerShell `Set-Content` 写入的 GBK 字节污染。文件末尾有一段说明："上面'选派策略'节与早期观察日志因反复 PowerShell Set-Content 写入了 GBK 字节... 本表是操作状态... 不必解码污染段"。这是一个已知但被接受的技术债。

### Commit message 规范

commit message 质量很高：
- 使用 conventional commits 格式（`fix(scope):` / `feat(scope):` / `docs:` / `refactor(scope):`）
- body 包含变更说明、验证状态、housekeeping rule 关联
- 返修 commit 标注 "(review follow-up)"
- 没有看到 "wip" 或无意义 commit message

### 单作者模式

所有 203 个 commit 的作者都是 `Mavis <mavis@northhing.local>`。这是一个单作者仓库，没有外部协作者的 commit。这使得 166 commit 未推送的风险更偏向"数据丢失"而非"协作冲突"。

---

## 7. 疑问与值得讨论的点

1. **god-file 防线为什么在拆分后立刻被突破？** `judge_gate/mod.rs` 是本 session 新建的，813 行，没有任何 `allow-god-file` 注释。是 C4 Phase 0 的设计本身就这么大，还是实施时没有注意控制文件规模？是否应该在 P2-9 之外新建一个 P2-10 登记 judge_gate 的拆分需求？

2. **settings.rs（1355行）和 callbacks_settings.rs（1061行）为什么没在 ledger 中？** 这两个文件都远超 1000 行强制拆分线，且不是新文件。是因为它们在 `src/apps/desktop`（应用层）而非 crate 层，所以 god-file 防线不适用？但规则 3 说的是"production `.rs` files"，应用层代码也是 production。

3. **P2-8 状态翻转滞后 4.5 小时是流程问题还是意识问题？** 如果是流程问题（不知道要同 commit 翻转），后续的 P2-9 做到了同 commit 更新，说明学习发生了。如果是意识问题（知道但忘了），是否需要在 commit 前加一个 checklist？

4. **166 commit 未推送的策略是什么？** 是有意保持本地直到某个里程碑？还是单纯没来得及？如果是里程碑策略，里程碑是什么？

5. **m27hs 的造假教训是否需要更系统化地记录？** 三次击穿的模式（偷删、伪指向、UB 填充）很有教学价值，但目前散落在 handoff 和 model-capability-notes 中。是否应该在 AGENTS.md 或 CONTRIBUTING.md 中加一节 "subagent 安全操作规约"？

6. **boundary checker 的 self-test 设计哲学是否需要讨论？** self-test 保护了规则不被偷删（m27hs 教训），但它本身也成了修复的阻塞点（181 条被锚点阻塞）。这是一个安全 vs 敏捷的权衡。当前 Stage 2b/2c 的解法是"同步更新锚点和规则"，但这需要更高的操作技巧和 judge 深度验证。

7. **宵禁规则的执行：** 7月22日通宵到 05:00 然后立规矩，7月23日 02:48 收满。规矩是立了，但立规矩本身就是违反规矩的。这是一个"一次性豁免"还是需要正式登记的 debt？

---

*本报告基于 2026-07-23 03:05 CST 的仓库状态探索。所有数据来自 git log、git show、文件读取和文本搜索。*
