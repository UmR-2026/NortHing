# Task T2-2j Report — Remote 栈整删批次 C8：文档/台账收口 + Minor triage

## 1. 任务概述

本批次（T2-2j / C8）为 remote 栈整删的纯文档事实同步、台账状态翻转与 Minor triage 收口批次。不引入任何产品行为代码变更，不修改任何依赖项与功能逻辑。

- 工作目录：`E:\agent-project\northing`
- 基础提交：`e47c0b0`（main 分支）
- 最终状态：**DONE**

---

## 2. 变更清单详细记录（文件:行 + 前后摘要）

### A. tech-debt ledger 翻状态 (`docs/status/tech-debt-ledger.md`)
1. **P1-4 (:49-53)**: 状态由 `active (mobile-web: frozen surface)` 翻转为 `resolved`。
   - 改动前：`- **Status**: active (mobile-web: frozen surface)`
   - 改动后：`- **Status**: \`resolved\` — mobile-web 面已整删（T2-2 C6 commit \`646f93d\`），条目随删除关闭；\`docs/architecture/backend-roadmap.md:118\` 已预先声明此关闭方式。`
2. **P1-7 (:70-74)**: 状态由 `active (registered 2026-08-04...)` 翻转为 `resolved`。
   - 改动前：`- **Status**: active (registered 2026-08-04, P1-5 standalone mitigation complete; a startup `warn!` has been added at `embedded_relay.rs`)`
   - 改动后：`- **Status**: \`resolved\` — relay-server + relay-core 已整删（T2-2 C5 commit \`f6a011b\`，PEND-1），embedded relay 入口不复存在，条目随删除关闭。`
3. **D-2 同查（verify-only）**:
   - 结论：确认 repo 全域 `weixin|wechat` 仅在 docs/archive、i18n 资源文案及 bash_tool / computer_use / playbooks 的活跃 IM app 探测代码中出现，QR 登录代码（`weixin_qr_login.rs`）已在前期删除，tech-debt-ledger 中无 D-2 条目，**无需编辑**。
4. **M-g-2 登记为 P2-19 (:218-223)**:
   - 新增 P2-19：`src/apps/server/README.md:5-10` 包含 3 条指向已删 relay-server 的悬空链接，留待 server 面解冻时处理。
5. **M-h 登记为 P2-20 (:225-230)**:
   - 新增 P2-20：`pnpm-workspace.yaml` 中注册孤儿工作区 `desktop-tauri`，登记为独立决策项待清理。

### B. backend-roadmap 事实标注 (`docs/architecture/backend-roadmap.md`)
6. **:115 行 (`apps/relay-server`)**:
   - 改动前：`| \`apps/relay-server\` | 已加固（fail-closed 绑定、自动 key、CORS localhost 默认，2026-08-04） | 维持；M5 解冻评估时按 surfaces 协议走 |`
   - 改动后：`| \`apps/relay-server\` | 已整删（T2-2 C5, commit f6a011b, PEND-1） | 随删除关闭；原维持/解冻评估规划失效 |`
7. **:118 行 (`mobile-web/remote_connect`)**:
   - 改动前：`| mobile-web/remote_connect | 已决删除（论题 v1.1） | TH-4 删除执行单入 T2-2；P1-4/P1-7/D-2 随删除关闭；将来移动需求 = T5 协议客户端重写 |`
   - 改动后：`| mobile-web/remote_connect | 已整删（TH-4 删除已执行，T2-2 C1-C7, commits fa88342..d16b037） | P1-4/P1-7/D-2 已随删除关闭；将来移动需求 = T5 协议客户端重建 |`
8. **:129 行第 6 条**:
   - 改动前：`6. **relay-server fail-closed 模式**：SW1-2 修 embedded relay 时直接复用其绑定/key 策略；`
   - 改动后：`6. **relay-server fail-closed 模式**（已失效：relay-server 已整删，T2-2 C5 commit \`f6a011b\`）：原 SW1-2 复用策略随载体删除失效；`
9. **:167 行 T2-2 行**:
   - 在 remote 栈整删语段追加标注：`remote 栈部分（含 relay-server/relay-core 整删、mobile-web、contracts 修剪、i18n 面）已完成 C1-C8（commits fa88342..本批），MiniApp 部分待执行`，保留 MiniApp 待执行部分与整行结构。

### C. README / CONTRIBUTING 摘除
10. **`README.md:43`**:
    - 改动前：`**Frozen-experimental**: CLI, server, relay, mobile-web, MiniApp UI, SDLC harness.`
    - 改动后：`**Frozen-experimental**: CLI, server, MiniApp UI, SDLC harness.`
11. **`CONTRIBUTING.md:146`**:
    - 删除了 `| Mobile web | \`pnpm --dir src/mobile-web run type-check\` |` 整行。

### D. Minor triage
12. **M-c-1 (`src/crates/assembly/core/Cargo.toml:124, 129`)**:
    - 改动前：`# Encryption (Remote Connect E2E)` 与 `# Device/Network info (Remote Connect)`
    - 改动后：`# Encryption` 与 `# Device/Network info`
    - 严守红线：未删除任何 dep 行（`aes-gcm`, `sha2`, `rand`, `local-ip-address`, `tokio-tungstenite` 均保留）。
13. **M-c-2 (`src/crates/assembly/core/src/service_agent_runtime/mod.rs:46`)**:
    - 改动前：`fn core_service_agent_runtime_owner_exposes_agent_runtime_and_remote_control_port() {`
    - 改动后：`fn core_service_agent_runtime_owner_exposes_agent_runtime() {`
14. **M-c-3 (`sar_dispatch.rs:2-5` verify-only)**:
    - 确认 `src/crates/assembly/core/src/service_agent_runtime/sar_dispatch.rs:2-5` 的 import 列表中仅包含 `AgentDialogTurnPort, AgentLifecycleDeliveryPort, AgentSessionManagementPort, AgentSubmissionPort, AgentTurnCancellationPort`，零 remote 残留。
15. **M-f-1 (`src/crates/contracts/runtime-ports/src/session_workspace.rs:1`)**:
    - 改动前：`//! R26 sibling 2/4: session_workspace — session storage + workspace filesystem/shell + permission + clock + terminal + network + git + mcp + remote-connection port traits.`
    - 改动后：`//! R26 sibling 2/4: session_workspace — session storage + workspace filesystem/shell + permission + clock + terminal + network + git + mcp port traits.`
    - 检查同文件：其余 `remote` 词汇均为合法保留的 SSH / remote workspace 语义（`SessionStorageKind::Remote`, `remote_ssh_host` 等）。
16. **M-g-1 (`scripts/i18n-audit.mjs`)**:
    - 删除了空函数定义 `function collectConfirmedUnusedKeys() {}`（原 :1284）及在 `auditI18nGovernanceReport` 中的死调用点 `collectConfirmedUnusedKeys();`（原 :1375）。
    - 字节级保留：其余内容及 pre-existing mojibake 完全未动。

---

## 3. 验证命令与输出原文

### 1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace`
```text
    Checking northhing-runtime-ports v0.2.10 (E:\agent-project\northing\src\crates\contracts\runtime-ports)
    Checking northhing-runtime-services v0.2.10 (E:\agent-project\northing\src\crates\execution\runtime-services)
    Checking northhing-product-capabilities v0.2.10 (E:\agent-project\northing\src\crates\assembly\product-capabilities)
    Checking northhing-agent-tools v0.2.10 (E:\agent-project\northing\src\crates\execution\tool-contracts)
    Checking northhing-kernel-api v0.1.0 (E:\agent-project\northing\src\crates\contracts\kernel-api)
    Checking northhing-services-integrations v0.2.10 (E:\agent-project\northing\src\crates\services\services-integrations)
    Checking northhing-agent-dispatch v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-dispatch)
    Checking northhing-agent-runtime v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-runtime)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.01s
```
**结果**: PASS（0 errors）

### 2. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --lib --features product-full core_service_agent_runtime`
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.33s
     Running unittests src\lib.rs (target\debug\deps\northhing_core-caf8370156ade402.exe)

running 3 tests
test service_agent_runtime::tests::core_service_agent_runtime_owner_exposes_agent_runtime ... ok
test service_agent_runtime::tests::core_service_agent_runtime_owner_keeps_coordinator_port_contracts ... ok
test service_agent_runtime::tests::core_service_agent_runtime_owner_keeps_scheduler_lifecycle_port_contracts ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1037 filtered out; finished in 0.00s
```
**结果**: PASS（重命名后的测试 `core_service_agent_runtime_owner_exposes_agent_runtime` 成功匹配并执行通过）

### 3. `node --check scripts/i18n-audit.mjs`
改动前与改动后输出完全一致：
```text
E:\agent-project\northing\scripts\i18n-audit.mjs:481
  'è¿?,
  ^^^^^

SyntaxError: Invalid or unexpected token
    at checkSyntax (node:internal/main/check_syntax:72:5)

Node.js v24.19.0
```
**结果**: PASS（改动前后报出同一个 SyntaxError，未扩大损伤）

### 4. `git diff --check`
```text
warning: in the working copy of 'CONTRIBUTING.md', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'README.md', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'docs/architecture/backend-roadmap.md', LF will be replaced by CRLF the next time Git touches it
warning: in the working copy of 'src/crates/contracts/runtime-ports/src/session_workspace.rs', CRLF will be replaced by LF the next time Git touches it
```
**结果**: PASS（0 whitespace errors）

### 5. E 项 Sweep: `rg -n -i "relay|mobile-web|mobile web|remote_connect|remote connect" AGENTS.md AGENTS-CN.md docs/status/surfaces.md scripts/`
```text
scripts/core-boundaries\self-test.mjs:2074:      contracts: ['ServiceRemoteWorkspaceSearchService', 'impl RemoteWorkspaceSearchProvider for CoreRemoteWorkspaceSearchProvider', 'lookup_remote_connection_with_hint', 'open_exec_channel', 'RemoteWorkspaceSearchStdioProtocol'],
scripts/core-boundaries\rules\source\required-rules.mjs:2031:        regex: /lookup_remote_connection_with_hint/,
scripts/core-boundaries\rules\source\required-rules.mjs:5292:        regex: /\blookup_remote_connection_with_hint\b/,
scripts/core-boundaries\rules\source\required-rules.mjs:5293:        message: 'missing preferred remote connection lookup adapter',
```
**结果**: PASS（`AGENTS.md` / `AGENTS-CN.md` / `docs/status/surfaces.md` 中 0 命中；`scripts/` 中命中项全部为保留的 SSH 远程工作区连接契约 `lookup_remote_connection_with_hint`，无任何已删 remote 栈残留）

### 6. `git status --short`
```text
 M .opencode/model-capability-notes.md
 M CONTRIBUTING.md
 M README.md
 M docs/architecture/backend-roadmap.md
 M docs/status/tech-debt-ledger.md
 M memory/northhing.md
 M scripts/i18n-audit.mjs
 M src/crates/assembly/core/Cargo.toml
 M src/crates/assembly/core/src/service_agent_runtime/mod.rs
 M src/crates/contracts/runtime-ports/src/session_workspace.rs
?? .handoffs/handoff-g2-t9-2026-08-07.md
?? .superpowers/sdd/task-t2-2j-brief.md
?? .superpowers/sdd/task-t2-2j-report.md
```
**结果**: PASS（仅包含清单明确授权的 8 个编辑文件 + 报告文件 + 并行会话已存在的未提交文件）

---

## 4. Rust 编译与错误分类

本批次涉及 Rust 文件变更仅包含注释修正、测试重命名及 doc 注释调整：
- 遇编译错误数：**0**
- 无机制层/设计层修复项需要登记。

---

## 5. 最终状态

**DONE**：清单 A-E 全部编辑与验证均严格按 brief 要求执行完毕，所有验证命令通过，无任何偏离或自作主张的范围扩展。
