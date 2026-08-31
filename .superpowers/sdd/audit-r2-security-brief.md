# 审计单 R2 — 安全面核查（只读）

仓库：`E:\agent-project\NortHing`（main，HEAD `f5dc0ef`）。**只读**：禁止改任何文件，唯一可写 = 你的报告。

威胁模型：**本地桌面 + CLI + 本地 server 的个人 AI 助手**，数据不离开本机（遥测边界已确认零外部端点）。所以重点是：本机数据完整性、凭据不落盘、工具执行的沙箱逃逸、子进程泄漏。**不要把"缺少 Web 认证/HTTPS"当 finding**（不适用）。

## 必须回答的清单

### A. 凭据与密钥
1. keyring 通路：API key 存在哪？是否**从不**写入磁盘。核实 Scheme C（core 不持久化 `api_key`）——grep `api_key` 在 config 序列化路径上的所有出现，确认 `AppSettings`/`GlobalConfig` 的 serde 字段里没有 key 字段。
2. **P1-8 MCP env 明文**（台账 active 项）：`store_env`/`load_env` 的实现，MCP server 的 env 是否明文落盘？落哪？是否可被其它本机进程读取？给 file:line + 实际风险定级。
3. 日志泄露：grep 所有 `tracing::debug!/trace!` 里是否打印 token/key/Authorization header；`debug.log`（`src/apps/desktop/.northhing/debug.log`）是否会记录敏感内容。
4. 测试是否写真 keyring：历史上 W4-1 把测试 key 写进真 OS keyring。核实现在测试是否都用 MockKeyring。

### B. 工具执行与路径沙箱
5. `guard_command_execution` 接线范围：哪些工具调用它？**哪些 shell 类工具没调用？**（AGENTS.md 骨干不变量要求新 shell 类工具必须调用）给未覆盖清单。
6. 路径逃逸：`api_fs.rs` 的文件树/读取是否做了 workspace_root 前缀校验 + symlink 解析后校验（W9-6 修过 symlink 围栏）；抽查是否还有别的路径入口（workspace 切换、session 导入导出、memory 导出固定路径）没做校验。
7. 导出固定路径（`<config>/northhing/exports/`）的目录权限与覆盖行为：是否会被覆盖写、是否可预测被劫持。

### C. 进程与生命周期
8. MCP/LSP/flashgrep 子进程清理（W5 的 I4+I5 修过）：核实 `kill_on_drop` / 进程组 kill / 750ms 超时是否仍接线；grep `Command::new` 与 `spawn` 的调用点，列出**未纳入清理机制**的 spawn。
9. P2-2 无单实例锁（台账 active，唯一指向数据丢失的项）：`config/app.json` 的读写路径，实测分析双开时的竞态窗口有多大（读-改-写是否是原子的）。

### D. 数据完整性
10. 原子写：`write_bytes_atomic`（I6 修过）覆盖了多少写路径？列出仍用 `fs::write` 直写的关键状态文件（state.json / app.json / session metadata / memory db）。
11. 并发：sessions/messages 的写是否受锁保护；`TEST_GLOBAL_CONFIG_MUTEX` 未覆盖 `kernel_facade().upsert_model_config` 路径（O-1 flaky 根因）是否属实。

### E. 内存安全与依赖面
12. `unsafe` 使用点清单（数量 + 每处一句话说明是否有 SAFETY 注释）。
13. `unwrap()` 477 / `expect()` 940（rot 计数）：抽查生产路径（非测试、非启动自检）里**会 panic 导致进程崩溃**的位置 top 10，给 file:line。区分"启动时一次性"与"运行中"。
14. Cargo.lock 里可疑/陈旧依赖：列出明显过久未更新的关键依赖（无法联网跑 audit，就按版本号人工判断并注明"未联网核实"）。

## 输出格式

分级清单：`[Critical|Important|Minor] 结论 — file:line — 本机可利用性（是/否/需前提）— 修复成本 S/M/L`。
末尾：① 真正需要马上修的（≤3 条）② 可以排期的 ③ 属于"理论风险、本机威胁模型下可接受"的。

## 纪律

- **禁止运行 cargo/pnpm**（会与编排者的编译抢锁）；`node scripts/*.mjs` 可跑。
- 禁止修改任何项目代码/配置；禁止 git 写操作。
- **禁止编造**：每条结论必须指到 file:line 或命令输出。拿不到写「无法验证（原因）」。
- 不要建议引入新依赖或大改架构——这是本地个人工具，成本要匹配威胁模型。
- 报告中文，英文标识符原样。
