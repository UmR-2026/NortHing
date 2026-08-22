SPEC: PASS
QUALITY: PASS

# Task B1 第 2 轮复审报告 — Important-1 闭合复核（fix commit 808ed65）

审查范围：git diff 41695f5..808ed65（累计 4 文件 +386/-30）；增量 d4b11b5..808ed65（2 文件 +176/-1：services-integrations mcp/config/service.rs + tests/config_and_server_lifecycle.rs）。提供的 task-b1-review.diff 解码后与 git diff 41695f5..808ed65 逐字一致（21110 字符 IDENTICAL）。第 1 轮报告 task-b1-review.md 未改动。fix1 报告仅作线索，全部结论独立取证。

## 一、Important-1 闭合判定：已闭合

### 1. 锁覆盖全部读-改-写路径 — PASS
- `MCPConfigService` 新增 `write_lock: tokio::sync::Mutex<()>`（service.rs:33），`new` 初始化（:40）。
- 三条 mutating 路径入口持锁，guard 生存期=整个函数，覆盖 get→改→set 全程：save_user_config（:224）、save_project_config（:254，锁在 load_project_configs_strict 读之前）、delete_server_config（:271）。
- 读路径（load_user_configs/load_project_configs/load_all_configs/get_server_config）不入锁——与 spec"读侧宽容语义保持"一致，读行为零变化。
- 无死锁：持锁三函数互不调用；save_server_config 仅分派不持锁（:166-174）；set/clear_remote_authorization 先无锁读再经 save_server_config 单次加锁（:200/:219）；load_project_configs_strict 仅被持锁的 save_project_config 调用且自身不取锁。tokio::sync::Mutex 不可重入，但无重入路径。
- 锁选型正确：临界区跨 await（store get/set 均异步），tokio::sync::Mutex 为正确选择。
- 生产有效性：MCPService::new 创建单一 MCPConfigService 并经 Arc 共享（assembly/core/src/service/mcp/mod.rs:51-52），另有 GLOBAL_MCP_SERVICE OnceLock 单例路径（mod.rs:78-88）→ 进程内典型并发写经同一实例，锁有效。跨实例/跨进程限制已在字段文档（service.rs:30-32）与报告观察项明示，与 FU-1 债项范围（应用内读-改-写）相符。

### 2. 并发测试真能抓 lost-update — PASS（实证）
- 审查书要求"对照 BASE 无锁版应能失败"。我在临时 scratch worktree（`git worktree add --detach <tmp> d4b11b5`，审查 worktree 零改动，用后即 `worktree remove --force` 清理）中，把 808ed65 的测试文件拷入无锁的 d4b11b5 源码树实测：3 个并发用例全部 FAILED——
  - concurrent_user_saves：仅存 2/10 条目（丢 8 次更新）
  - concurrent_project_saves：仅存 3/10 条目
  - concurrent_user_save_and_delete：终态 7≠5（含被复活的已删条目，双向丢失更新均被抓）
- 对照：808ed65（有锁）同样 3 用例全绿。测试有效性为实证，非静态推断。
- 断言强度：精确计数 + 逐 key 存在/缺失断言；混合用例同时覆盖 save 丢失与 delete 复活两个方向。

### 3. 稳定性（不 flaky）— PASS
- 有锁后读-改-写串行化，终态确定（混合用例终态可静态推演：5 save + 5 delete 从 5 条初始 → 恰 5 条）。
- 我独立连跑 5 次 focused 过滤（concurrent，3 用例）全绿；implementer 报告 §3.2 含 3 次全二进制连跑原文输出（19 passed ×3）。合计 8 次无失败。
- multi_thread(worker_threads=4) 形态恰当：真实多线程并行才能稳定触发 RMW 竞态（无锁对照实测亦一次性全触发，窗口足够宽）。

### 4. project 级一并加锁的合规性
计划范围外声明保护的是 Task 6 的 fail-closed 语义（不得改动），加锁不改 project 级任何语义（同 strict 读、同写形态、同错误），仅串行化窗口；且用户拍板的 (a) 方案经我 r1 修复指引明示"对称含 project 级"，两 key 共享同一实例、单锁统一保护避免半保护。project 级既有用例（save_project_fails_closed_*、upsert 契约）仍全绿（报告 §3.1）。判定：经批准的对称扩展，非范围违规。

## 二、全量回归不放松（第 1 轮已 PASS 项复核）

- 层 A classify_config_read：增量 diff 未触碰 assembly/core 文件，r1 验证（ErrorKind 分类对 ConfigService 语义正确）原样成立。
- 层 B fail-closed：HEAD 源码逐行核对，未识别格式拒写（service.rs:229-236/:281-289）、delete None→not_found（:273-278）、条目缺失→not_found（:292-297）均原样保留。
- 读侧宽容兜底：load_all_configs warn+empty（:68-91）未变；锁不涉读路径。既有用例 keeps_load_failures_as_empty_baseline 仍绿（报告 §3.1）。
- 台账翻转：`git grep -c resolved 808ed65` = 2，FU-1 resolved 状态保留；FU-2..FU-5 仍 open。
- 纪律：fix1 仅 2 个范围内文件（git diff --stat 核实）；审查 worktree git status 仅未追踪审查工件；无 fmt 噪声（4 个 hunk 全为逻辑改动）；无新日志行（既有日志 English-only）；service.rs 310 行 <800；commit 为独立新 commit（未 amend，d4b11b5 保留）；message conventional 风格。
- 验证命令：fix1 报告 §3.1-3.3 含原文输出（mcp 过滤 18 passed；3×稳定性 19 passed；cargo check -p northhing-core 0 error）。按纪律未重跑这些原样命令；我另跑 focused 复核见下节。
- 家规 4（并发改动带测试）：本次加锁随附 3 个自动化并发测试，满足。

## 三、Findings

### Critical
无。

### Important
无（Important-1 已闭合，见第一节）。

### Minor
1. （承接 r1 Minor-1，仍开放）写入非原子：ConfigManager::save_config（mgr_load.rs:158 `fs::write`）仍直写整文件。fix1 报告 §4.2 已承接说明（锁消除同实例 lost-update，跨进程+非原子落盘为独立更深层问题），但 tech-debt-followups.md 仍未登记独立债项。建议分支终审文档收口时新登记 FU 项"GlobalConfig 文档原子落盘（参照 json_store::write_atomic 模式）"。不阻塞。
2. （新）台账 FU-1 注记轻微过期：tech-debt-followups.md:12 仍写"新增测试：integrations +4、core lib +2"，fix1 后实际 integrations 累计 +7（含 3 并发用例）且新增了写串行化。状态行正确、硬规则（同 commit 翻转）已在 d4b11b5 满足，此处仅描述性滞后。建议终审时补一行 fix1 说明（如"808ed65 补读-改-写串行化 + 并发测试 ×3"）。不阻塞。
3. （承接 r1 Minor-2，记录在案）两个 trait 层读错误用例在 BASE 亦通过的回归护栏性质不变，无需动作。

## 四、实际运行的复核命令（本轮）

1. `git log --oneline -3` / `git status --short` / `git diff --stat 41695f5..808ed65` 与 `d4b11b5..808ed65` → 双 commit 链正确，增量 2 文件 +176/-1，工作区仅未追踪工件。
2. 提供 diff 文件（UTF-16LE）解码比对 `git diff 41695f5..808ed65` → IDENTICAL。
3. focused 稳定性：`cargo test -p northhing-services-integrations --features product-full --test config_and_server_lifecycle concurrent` ×5 → 每次 `3 passed; 0 failed`（EXIT=0）。
4. 测试有效性实证（审查书点名项）：`git worktree add --detach <tmp>/b1-nolock d4b11b5` → 拷入 808ed65 测试文件（scratch 树 service.rs grep write_lock = 0 确认无锁）→ 同命令跑 concurrent → `FAILED. 0 passed; 3 failed`（2/10、3/10、7≠5）→ `git worktree remove --force` 清理，审查 worktree 复核仍干净。
5. `git grep -c resolved 808ed65 -- tech-debt-followups.md` → 2；`git grep -n "integrations +4"` → :12（Minor-2 证据）。
6. 生产装配核对：读 assembly/core/src/service/mcp/mod.rs:46-88（单实例 + Arc + OnceLock 单例）。
7. 按纪律未重跑 implementer 已贴原文的 §3.1-3.3 命令。

## 五、Cannot verify from diff

1. fix1 报告 §3.1/§3.3 的运行时通过状态依赖报告原文输出（按纪律采信）；本轮 focused 自跑样本（§四.3/4）已独立覆盖并发用例的通过与失败两面。
2. 跨实例/跨进程多写者场景：按设计为范围外（字段文档 service.rs:30-32 明示），diff 内无法验证亦无需验证——若未来出现多实例写同一 app.json 的装配，需另行评估（可与 Minor-1 原子落盘合并立项）。

## 六、结论

SPEC: PASS — 计划/债清单全部约束闭合：ErrorKind 分类 fail-closed（r1）、未识别格式拒写（r1）、读注入 IO 错误测试（r1）、并发写不丢条目测试（本轮实证有效且稳定）、验证命令、台账翻转、范围纪律。
QUALITY: PASS — 锁设计正确（选型/覆盖/无死锁/读路径不受影响）、测试断言强且经 BASE 对照实证、无新 fail-open、命名注释清晰、行数合规、diff 干净。
建议：Minor-1/2 交分支终审文档收口；本任务可进入下一任务或终审。