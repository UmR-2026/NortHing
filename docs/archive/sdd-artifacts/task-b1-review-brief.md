# Task B1 审查书 — FU-1 save_user_config fail-closed

你是任务级审查者（judge）。对下列 commit 做**双判决**：① spec 合规 ② 代码质量。两判决各自 PASS/FAIL，缺一不算通过。独立取证，不信任 implementer 报告的文字结论；报告只作线索。

## 审查对象（第 2 轮复审）

- worktree：`E:\agent-project\northing\.worktrees\backend-followups-0804`（分支 `fix/backend-followups-0804`）
- commits：`d4b11b5`（首轮）+ `808ed65`（fix1，补并发保护与并发测试）；BASE `41695f5`，HEAD `808ed65`
- diff 文件：`.superpowers/sdd/task-b1-review.diff`（与 `git diff 41695f5..808ed65` 一致，可复核）
- 第 1 轮判决：SPEC FAIL / QUALITY PASS，阻塞项 Important-1（并发写不丢条目测试缺失）。用户拍板选 (a) 加锁+并发测试。本轮重点复核 Important-1 是否闭合 + 全量回归不放松。
- fix1 报告：`.superpowers/sdd/task-b1-fix1-report.md`

## 证据材料

- 计划（spec 来源之一）：`.superpowers/sdd/plan-2026-08-04-backend-followups.md` §2 Task B1
- 债项（spec 来源之二）：`.superpowers/sdd/tech-debt-followups.md` FU-1（注意：该文件已被本 commit 修改翻状态，spec 原文以 `git show 41695f5:.superpowers/sdd/tech-debt-followups.md` 为准）
- implementer 任务书：`.superpowers/sdd/task-b1-brief.md`
- implementer 报告：`.superpowers/sdd/task-b1-report.md`

## Spec 约束（计划原文逐字复制）

> ### Task B1 — FU-1 save_user_config fail-closed [security]
> - **锚点**：`services-integrations/src/mcp/config/service.rs:212-237`（save_user_config）、`:255-` 起 `delete_server_config`（同类 read-modify-write，**纳入范围**，同漏洞类）；strict 参照 `:128` `load_project_configs_strict`（Task 6 已建模式）；config store 实现层 `assembly/core/src/service/mcp/config/service.rs:19/:29`（get/set_config_value），读错误语义需追到该层核实。
> - **根因**：用户级 `mcp_servers` 读-改-写对读取阶段失败容错过宽（与 H-7 修复前同类）；并发/磁盘抖动下可能丢配置或写残缺 JSON。
> - **修复方向**：套用 Task 6 strict 变体——读取失败按 ErrorKind 分类（NotFound/键缺失=合法空态，其它=Err 中止写）；写入确认走原子落盘（核 set_config_value 下游，未原子化则参考 json_store::write_atomic / Task 7 模式）。
> - **测试**：读取注入 IO 错误 → fail-closed 且既有配置不丢；并发写不丢条目。
> - **验证**：`cargo test -p northhing-services-integrations --features product-full mcp`
> - **范围外**：project 级路径（Task 6 已修）；config store 其它 key 的语义审查。

债清单 FU-1 原文补充（验证要求）：

> `cargo test -p northhing-services-integrations --features product-full mcp`；新增并发写 + 读取注入 IO 错误的测试，断言 fail-closed 且不丢既有配置。

## 全局纪律约束（判 spec 合规时核对）

- 解债 commit 必须同 commit 翻转 `tech-debt-followups.md` 对应项状态（doc sync 硬规则）。
- implementer 只 commit 范围内文件。
- 日志 English-only、无 emoji。
- 不裸 `cargo fmt` 污染（diff 中不得出现与修复无关的格式化噪声）。
- 生产 .rs <800 行。

## 审查方法要求

1. **spec 判决**：逐条对照上述 spec 约束取证（file:line）。特别核对：
   - 错误分类是否真按 ErrorKind 区分"缺 key/NotFound"与"真实失败"（读 `classify_config_read` 实现与 `ConfigService.config` 实际语义，判断分类是否正确，不能只看测试绿）。
   - 读侧宽容路径（`load_all_configs` 兜底）在层 A 改严格后是否仍成立。
   - spec 要求的"并发写不丢条目"测试是否存在或是否有正当理由缺失。
   - 范围外路径（project 级、其它 key）是否确实未动。
2. **quality 判决**：错误语义一致性、命名、测试有效性（断言是否真能抓住回归）、有无引入新的 fail-open、god-file 线（service.rs 现 289+ 行，未超线）。
3. **验证命令**：不重跑 implementer 已跑且报告含原文输出的测试；但对任何可疑点可运行 focused 命令复核（`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH` 前缀必带）。
4. diff 无法验证的项（如运行时行为、环境差异）标注 "Cannot verify from diff"，说明你需要什么才能判定。

## 交付

报告写入 `.superpowers/sdd/task-b1-review-r2.md`（第 1 轮报告 `task-b1-review.md` 保留勿动）：

- 第一行：`SPEC: PASS|FAIL` 第二行：`QUALITY: PASS|FAIL`
- findings 分级列表：Critical / Important / Minor，每条带 file:line 证据与修复建议
- 你实际运行的复核命令与输出摘要（如有）
- Cannot verify from diff 清单（如有）
