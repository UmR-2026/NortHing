# 审计单 R1 — 代码腐化与分层边界（只读）

仓库：`E:\agent-project\NortHing`（main，HEAD `f5dc0ef`）。**只读**：禁止改任何文件，唯一可写 = 你的报告。

## 目标

回答一个问题：**这个仓库现在烂在哪、烂到什么程度、修起来多大成本**。要能让一个没看过代码的人据此估投入。

## 必须回答的清单

### A. 分层边界（六层，见根 AGENTS.md）
1. 逐层抽查反向依赖：`contracts/` 是否依赖 execution/services/assembly/apps；`services/` 是否依赖 `agentic`；`adapters` 是否拥有产品能力选择。用 rg 扫 `Cargo.toml` 的 `[dependencies]` + 源码 `use` 路径，给违规清单（file:line）。
2. 跑 `node scripts/check-core-boundaries.mjs`（若存在）并贴原文输出；不存在就说明。
3. 「共享 core 平台无关」违规：core 里是否出现 `tauri::AppHandle`、`winapi`/`windows` 直调、硬编码 Windows 路径。

### B. 腐化指标真值
4. `node scripts/verify-rot-budget.mjs` 贴原文；逐条解释 9 个零余量指标分别代表什么债（例：`let_underscore 388/388` = 388 处静默吞错）。
5. **找出逼近线的新文件**：全仓 `.rs` 行数 top 15（排除 tests），列出 ≥650 行的，标出哪些未登记进 `scripts/rot-budget.json`（潜在定时炸弹）。
6. `allow(dead_code)` 现有 106 处：按 crate 分布统计 top 5，抽查 10 处判断是否真死（可被删）还是"假死"（反射/序列化/feature 门控导致）。
7. 克隆/重复代码：重点查 ① `selectors` 集群 B 层（W11-2 遗留页面级合并地图）② `chat/{mcp,commands,run}.rs` 的 15 处 bridge 未迁 ③ 其它 A 层已清但 B/C 层残留。给重复块清单 + 估算可减行数。
8. 死代码与不可达：扫 `unimplemented!`/`todo!`/`panic!("not implemented")`/返回 `Err(Internal("not yet wired"))` 的位置（如 `get_persistence_handle`），列出"接口存在但没实现"的清单。

### C. 文档与代码不符
9. 统计 `TODO`/`FIXME`/`HACK`/`XXX` 数量并按目录分布 top 5；标注其中写了 owner/日期的占比。
10. 抽查 10 条 `// ponytail:` 注释是否仍成立（注释描述的简化是否还在代码里）。
11. 过期注释/陈旧文档：抽查 5 个"注释说 A、代码做 B"的实例（如 css.rs:57 scrim 陈旧注释那类）。

### D. 技术债台账对账
12. `docs/status/tech-debt-ledger.md` 里 9 条 open（P1-8 / P2-1~P2-5 / P2-14 / P2-17 / P2-18）：逐条到代码里核实**是否真的还 open**（台账会骗人），给"仍 open / 实际已解决 / 描述已失真"三分类。

## 输出格式

分级清单，每条：`[Critical|Important|Minor] 一句话结论 — file:line — 修复成本估（S≤半天 / M 1-2天 / L >2天）`。
末尾给三张汇总表：① 分层违规 ② 腐化债（含可减行数估算）③ 文档失真。最后一段：**如果要让这个仓库"健康"，最短路径是什么**（按性价比排序的 3-5 个动作）。

## 纪律

- **禁止运行 cargo/pnpm**（编排者另跑，会锁冲突）。可跑 `node scripts/*.mjs`。
- 禁止任何 git 写操作；禁止修改除报告外任何文件。
- **禁止编造数字**：每个数字必须来自你实际执行的命令或读到的行。拿不到写「无法验证（原因）」。
- 全仓约 1365 个文件，用 rg/grep 批量扫，不要一个个读。
- 报告中文，英文标识符原样。
