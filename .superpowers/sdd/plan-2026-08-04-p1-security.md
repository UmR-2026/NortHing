# P1 安全债关闭计划（2026-08-04）

来源：`docs/status/tech-debt-ledger.md` P1-2 / P1-3 / P1-5（均 active，锚点 2026-08-04 复核确认）。
用户指令（2026-08-04）：**在加新功能（src/agentic 成长核心）前先关 P1 安全债**；P1-3、P1-5 尤其优先。
基线：main HEAD `ae44334`（含本日归档 3 笔，已推 origin）。

## 0. 锚点复核（2026-08-04 HEAD ae44334 实测）

| 项 | 锚点 | 现状 |
|---|---|---|
| P1-3 | `src/crates/execution/tool-execution/src/fs/delete_path.rs:52-74`（本地 `fs::remove_dir_all`/`remove_dir`/`remove_file`）、`:76-82`（remote `rm -rf`/`rm -f`） | open，无任何 trash 路径 |
| P1-5 | `src/apps/relay-server/src/config.rs:30`（默认 `0.0.0.0:9700`）、`:38`（CORS `*`）、`:39`（`api_key: None`）、`:49`（RELAY_PORT 仍绑 0.0.0.0） | open，RELAY_API_KEY 可选 |
| P1-2 | `src/apps/desktop/src/app_state/settings/types.rs:58-59`（`pub api_key: String`，注释自认 plaintext in app.json） | open |
| P1-1 | ledger 仍标 active，实际已被 Task 7（commit `9be74ec`，H-9 原子落盘）解决——final-review §3.2 背书 | **ledger 漏翻**，C1 顺带翻 |

## 1. 分支与任务

分支：`fix/p1-security-0804`，worktree `E:\agent-project\northing\.worktrees\p1-security-0804`，基线 ae44334。
顺序：C1 → C2 → C3（串行派发，不并行）。每任务 brief→implementer→judge 双判决。

### Task C1 — P1-3 删除走回收站（用户点名优先）

- **范围**：`delete_local_path` 本地删除默认改走 OS 回收站（`trash` crate）；remote 命令构造保持但确认链路必须有显式确认。
- **修复方向**：
  1. tool-execution 引入 `trash` crate；本地删除默认 `trash::delete`（文件/目录同路径），**trash 不可用时 fail-closed 返回 Err，禁止静默回落永久删除**。
  2. `DeleteLocalPathRequest` 增加显式 `permanent` 语义（或等价开关），仅当调用方显式要求时走旧 `fs::remove_*`；该开关的上游确认要求由 implementer 核实工具确认链路（tool framework confirmation）并写入 report 事实。
  3. remote：`build_remote_delete_command` 不改语义（远端无回收站），report 必须给出"remote 删除是否已过确认门"的核实证据；若无确认门，列为 Critical/Important finding 交审查，不擅自扩范围。
  4. 测试：注入 seam（trash 后端可替换）断言默认走 trash、permanent 走 fs、trash 失败 fail-closed；remote 命令构造回归不变。
- **验证**：`cargo test -p northhing-tool-execution`（或该 crate 实际名，implementer 以 Cargo.toml 为准）+ `cargo check -p` 通过。
- **顺带**：同 commit 翻 ledger P1-1 → resolved（证据 `9be74ec` + final-review §3.2）与 P1-3 → resolved（doc sync 硬规则）。

### Task C2 — P1-5 relay 安全默认（用户点名优先）

- **范围**：relay-server 默认安全化：loopback 绑定 + 自动 key + CORS 收紧。
- **修复方向**：
  1. 默认 bind 改 `127.0.0.1:9700`；新增 `RELAY_BIND` env 显式覆盖（`RELAY_PORT` 只改端口、继承默认 host）。
  2. **非 loopback 绑定无 API key = 启动 fail-closed**（明确错误信息），不提供"裸奔 0.0.0.0"模式。
  3. 首次运行自动生成 API key（足够长随机串），持久化到 relay 数据目录（0600，写入走原子模式），后续启动复用；`RELAY_API_KEY` env 优先于文件。启动日志说明 auth 已启用与 key 来源（**日志不打印 key 本体**；首次生成时 stdout 一次性提示存放位置）。
  4. CORS 默认从 `*` 收紧为 localhost 任意端口（`http://localhost:*` / `http://127.0.0.1:*`），`RELAY_CORS_ALLOW_ORIGINS` 可覆盖。
  5. **集成风险（必答）**：desktop 内嵌 relay 场景（api_key=None 路径，见 final-review §6.2 Gap 2）——implementer 必须核实 desktop 如何起内嵌 relay/如何配对，确保新默认不破坏桌面 pairing；若 desktop 直接依赖 relay-server 默认无认证，须同步改 desktop 侧读取生成 key。
  6. 测试：默认配置断言（loopback + key 生成 + 非 loopback 无 key 拒绝启动）；e2e `e2e_web_assets` 保持全过（其用显式 key，应不受影响）。
- **验证**：`cargo test -p northhing-relay-server -p northhing-relay-core`；desktop 侧若被触及则 `cargo check -p northhing`。
- **顺带**：同 commit 翻 ledger P1-5 → resolved。

### Task C3 — P1-2 API key 迁移 OS keyring

- **范围**：provider api_key 从 app.json 明文迁入 OS keyring（`keyring` crate：Win Credential Manager / macOS Keychain / Linux Secret Service）。
- **修复方向**：
  1. `ProviderConfig.api_key` 序列化形态改为 keyring 引用（app.json 存 secret 引用/标记，不存明文）；读取路径经 keyring 解析。
  2. **迁移**：load 时发现旧明文 → 写入 keyring → 从 app.json 抹除并原子落盘（Task 7 模式）；迁移失败 fail-closed（不得丢 key、不得留双份语义）。
  3. keyring 不可用（headless Linux 等）：fail-closed + 明确错误指引，**禁止静默回写明文**。
  4. 测试：keyring seam（mock store）覆盖存/取/迁移/不可用四路径；并发迁移幂等。
  5. 日志永不打印 key（既有注释承诺保持）。
- **验证**：`cargo test -p northhing --lib settings` + `cargo check -p northhing`。
- **顺带**：同 commit 翻 ledger P1-2 → resolved。

## 2. Wave 结构外的已知关联项（不在本轮扩入）

- ledger P2-2（单实例锁）与 P1-2 同文件域但独立，不动。
- relay capability token（上轮 final-review deferred）仍属 Wave 3 决策项，不与 C2 混。
- `save_user_config` fail-open 等 FU 项由 `plan-2026-08-04-backend-followups.md` 另行负责，不重叠。

## 3. 执行纪律（逐字进 brief）

- 一次一任务，brief 文件是需求唯一来源；不续会话粘历史。
- 不裸 `cargo fmt`；日志 English-only 无 emoji；生产 .rs <800 行；并发/取消相关改动必带测试。
- tech-debt ledger 翻转必须与修复同 commit（housekeeping 规则 2）。
- implementer 只 commit 范围内文件；收口核对 `git log`。
- 验证最小集 = focused `-p`；`cargo check --workspace` 上游 embed-resource 阻断，交 CI。
- 模型：implementer = deepseek-v4-flash（volcengine-agent-plan 线；用户 2026-08-04 指定：k3 额度低不做 coder，dv4f 快稳）；judge = minimax-cn-coding-plan/MiniMax-M3；终审 = volcengine-agent-plan/glm-5.2。勿用 m27 系做 judge。ark provider 本环境不可解析（flat kimi-k3 派发 2026-08-04 复测失败）。
- Curfew：03:00 后不派编码任务。

## 4. 完成定义

C1-C3 双判决通过 → 分支终审双 PASS → `--no-ff` 并 main → 回归扫（tool-execution / relay / desktop 对齐基线 + core 1134/1134 抽查）→ ledger P1-1/2/3/5 全 resolved → push origin → 更新 handoff。此后才启动 S1（src/agentic 成长核心）。
