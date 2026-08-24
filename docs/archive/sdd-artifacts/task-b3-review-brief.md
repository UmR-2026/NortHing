# Task B3 审查书 — FU-3 settings 竞态收口 + FU-4 dead wrapper（含前置 build fix）

你是任务级审查者（judge）。对下列两个 commit 做**双判决**（各自独立给结论）：① spec 合规 ② 代码质量。独立取证，implementer 报告只作线索。

## 审查对象

- worktree：`E:\agent-project\northing\.worktrees\backend-followups-0804`（分支 `fix/backend-followups-0804`）
- BASE `57e4672`，HEAD `755a503`，两个 commit：
  - **C-A = `b0bfe43`** build fix：keyring v1 feature 使能 + 编译修复（P1-C3 合入后 desktop 从未编译，`cargo check -p northhing` 在 BASE 即失败——这是 main 继承缺陷，非本轮引入）
  - **C-B = `755a503`** B3 本体：FU-3 + FU-4
- diff 文件：`.superpowers/sdd/task-b3-review.diff`（= `git diff 57e4672..755a503`）

## 证据材料

- 计划：`.superpowers/sdd/plan-2026-08-04-backend-followups.md` §2 Task B3
- 债项原文：`git show 57e4672:.superpowers/sdd/tech-debt-followups.md` FU-3/FU-4（现文件已被翻转）
- implementer 任务书：`.superpowers/sdd/task-b3-brief.md`（§0 含用户对 FU-3 方式的拍板，逐字引用）
- implementer 报告：`.superpowers/sdd/task-b3-report.md`（STATUS: DONE_WITH_CONCERNS，疑虑已按编排决策处理为独立 commit C-A）

## C-A 审查标准（build fix）

- **spec**：恢复 `cargo check -p northhing` / `cargo test -p northhing --lib` 可编译可运行；**零行为变化**（Cargo.toml feature 使能、keyring.rs API 名/Lazy 修正、provider_test.rs 导入路径——逐行核对是否纯编译修复）。
- **quality 重点**：keyring.rs 是 P1-C3 评审过的安全代码，本 commit 改了它 3 行——必须对照 C3 意图（fail-closed、sentinel、迁移语义）确认改动不改变任何运行时行为，只是让代码对 keyring 4.1.6 `v1` feature 的真实 API 编译通过。任何语义漂移 = Critical。
- Cargo.lock +4 包：核对是否仅为 keyring v1 依赖链。

## C-B 审查标准（FU-3 + FU-4）

Spec 约束（计划原文 + 用户拍板偏离）：

> ### Task B3 — FU-3 + FU-4 desktop settings 竞态收口 + dead code [concurrency+hygiene，合并单]
> - **根因**：FU-3：dedup 迁移挂在只读 load 路径、未持 settings 锁 → 窄窗口竞态（重复 provider 时触发）。FU-4：Task 7 收敛写入口后旧 wrapper 成死代码（`cargo check -p northhing` warning）。
> - **修复方向**：dedup 从 load 路径剥离，改为 `update_app_settings` 内显式执行（持锁），load 纯读；删除 `save_app_settings` wrapper（先 grep 全仓确认无调用方，含测试）。
> - **测试**：并发 load+update 下 dedup 不产生竞态/重复写；`cargo check -p northhing` warning 消失。
> - **验证**：`cargo check -p northhing` + `cargo test -p northhing --lib settings`
> - **范围外**：core GlobalConfig 与 desktop 的跨模块竞态（Wave 3 决策项）。

**已授权偏离（用户 2026-08-05 拍板，不得作为 finding 提出，但其实现正确性必须审查）**：计划写于 P1-C3 之前；C3 后 load 路径另有 keyring 迁移写（C3 评审刻意保留的安全行为）。用户选"锁住公共 load"：公共 `load_app_settings` 全程持 `SETTINGS_WRITE_LOCK`，`load_app_settings_at` 保持无锁供锁内组合，迁移留 load 路径。审查实现是否忠实于该方案。

逐条取证：
1. 公共 load 是否真持锁覆盖整窗（load→dedup→keyring 迁移→写）；`_at` 是否无锁；update 事务锁内调 `_at` 是否不重入（死锁检查，读代码确认）。
2. 新并发测试 `concurrent_loads_and_updates_preserve_all_writes` 是否真能抓 BASE 竞态（对照 57e4672 静态推断失败机理）；种子是否同时触发 dedup 写与 keyring 迁移写；30s timeout 死锁防护是否有效。
3. 测试走 `load_app_settings_locked`/`update_app_settings_at` 而非字面公共函数（implementer 理由：Windows `dirs::home_dir` 不可重定向，测试不得碰真实用户配置）——判断该替代是否等价（锁是否为同一把真实静态锁、函数体是否与公共版逐字一致）。
4. FU-4：wrapper 删除彻底性（再导出、调用方、注释引用）；warning 消失证据；`save_app_settings_at` 保留。
5. 台账 FU-3/FU-4 同 commit 双翻。
6. 范围外未动（keyring.rs 逻辑在 C-A 已动——区分 C-A 编译修复 vs 逻辑改动；sync.rs、update 事务结构、core 竞态）。

## 纪律核对（两 commit 均适用）

只 commit 范围内文件；日志 English-only 无 emoji；无裸 fmt 噪声；io.rs <800 行。

## 验证命令

implementer 已贴 `cargo check -p northhing` + `cargo test -p northhing --lib settings`（79/79）+ 全 lib（118/118）原文输出。按纪律不重跑；可疑点可 focused 复核（`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH` 前缀必带）。**注意**：worktree 若缺 `generated_locale_contract.rs` 等生成物，勿自行生成后遗留——B3 implementer 报告提及过该坑。

## 交付

报告写入 `.superpowers/sdd/task-b3-review.md`：

- 第一行：`C-A SPEC: PASS|FAIL` 第二行：`C-A QUALITY: PASS|FAIL`
- 第三行：`C-B SPEC: PASS|FAIL` 第四行：`C-B QUALITY: PASS|FAIL`
- findings 分级（Critical/Important/Minor），每条带 commit 归属 + file:line 证据
- Cannot verify from diff 清单（如有）
