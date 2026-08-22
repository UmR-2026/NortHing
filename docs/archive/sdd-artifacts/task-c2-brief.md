# Task C2 Brief — P1-5 relay 安全默认（loopback + auto key + CORS 收紧）

> 本文件是任务的**唯一需求来源**。按此执行，不要从会话历史或猜测中补充需求。

## 位置

- Worktree（在此工作）：`E:\agent-project\northing\.worktrees\p1-security-0804`
- 分支：`fix/p1-security-0804`（已含 C1 commits，基线接续）
- 计划上下文（只读参考）：`E:\agent-project\northing\.superpowers\sdd\plan-2026-08-04-p1-security.md`

## 问题（P1-5，tech-debt-ledger active）

standalone relay-server 默认 `0.0.0.0:9700` + `api_key: None` + CORS `*`。对有文件/Shell 权限的桌面 agent 产品，开放 relay = 局域网内任意设备可 pair/command。

- 锚点：`src/apps/relay-server/src/config.rs:30`（默认 0.0.0.0:9700）、`:38`（CORS `*`）、`:39`（api_key None）、`:49`（RELAY_PORT 仍绑 0.0.0.0）；`src/apps/relay-server/src/main.rs:20,44,55-58`（from_env → build_relay_router(api_key) → bind）。

## 已核实的跨模块事实（编排者 2026-08-04 预核，勿重复调研）

- desktop（`src/apps/desktop`）**不引用** relay-server / RELAY_API_KEY——standalone relay 是独立二进制。
- 内嵌 relay 在 `src/crates/assembly/core/src/service/remote_connect/embedded_relay.rs`：`start_embedded_relay` 绑 `0.0.0.0:{port}` 且 `build_relay_router(..., None)`（open 模式），服务 LAN/ngrok 手机配对，是**产品必需**的开放面。
- **范围决定（已定，勿扩）**：本任务只改 **standalone relay-server** 的默认。embedded relay 不改绑定/认证语义（改配对协议属设计任务），仅加一条启动 warn 日志（English-only，明示 open mode + 0.0.0.0 暴露），并在 ledger 登记新条目 P1-7（embedded relay key threading，见交付要求 7）。

## 交付要求

1. **默认绑定 loopback**：`RelayConfig::default()` listen_addr 改 `127.0.0.1:9700`；`RELAY_PORT` 只改端口、继承默认 host。新增 `RELAY_BIND` env（完整 socket addr，如 `0.0.0.0:9700`）显式覆盖绑定。
2. **非 loopback 无 key = 启动 fail-closed**：解析后若 bind 地址非 loopback（非 127.x / ::1）且无 API key（env 与文件皆无）→ `from_env` 返回错误/进程启动即 bail，错误信息 English-only 且指明两条出路（设 RELAY_API_KEY 或改回 loopback）。实现方式自行决定（`from_env` 改签名返回 Result 或 main 里校验均可，保持调用方同步更新）。
3. **首次运行自动生成 API key**：无 `RELAY_API_KEY` env 时，从 key 文件读取；文件不存在则生成足够长随机串（≥32 bytes 熵，hex/base64 编码），原子写入 key 文件（模式参考仓库既有原子写：tmp+rename，见 `.superpowers/sdd/final-review.md` §3.2 三模式），权限 0600（unix；Windows 无等价则跳过并注释说明）。key 文件路径：用户数据目录下 relay 专属（如 `~/.northhing/relay/api_key`，若仓库已有 app_paths/数据目录约定则从其约定，report 写明选择依据）。后续启动复用文件 key。**RELAY_API_KEY env 永远优先于文件**。
4. **日志纪律**：启动日志说明 auth 状态与 key 来源（env/file/generated），**任何日志不得打印 key 本体**；首次生成时 stdout 一次性提示 key 文件路径（同样不打印 key）。
5. **CORS 收紧**：默认 `cors_allow_origins` 从 `*` 改为 localhost 任意端口（`http://localhost:*` 与 `http://127.0.0.1:*` 语义；若 tower-http CORS 层不支持端口通配，则枚举常见开发端口段或按 Origin host 判定，实现自定，report 说明）；新增 `RELAY_CORS_ALLOW_ORIGINS`（逗号分隔）显式覆盖。核实 CORS 配置当前实际消费点（config 字段今天可能未被 router 使用——若发现 cors_allow_origins 从未接线到 axum router，**把接线补上**并在 report 说明，这属于本条范围）。
6. **测试**（新增，全过）：
   - 默认 config：loopback + key 生成/复用（用 temp dir seam 控制 key 文件路径）
   - RELAY_API_KEY env 优先于文件
   - 非 loopback 无 key → 拒绝（断言错误）
   - 非 loopback 有 key → 放行
   - RELAY_BIND 覆盖生效
   - 既有 e2e（`e2e_web_assets`，用显式 key）保持全过
7. **ledger 翻转（同 commit）**：`docs/status/tech-debt-ledger.md`
   - P1-5 → resolved（standalone 部分；注明 embedded relay 另立 P1-7）
   - 新增 P1-7：embedded relay（`embedded_relay.rs`）0.0.0.0 open 模式无 key——LAN 配对产品必需，修复需配对协议带 key（设计任务）；本任务已加启动 warn。状态 active。

## 范围外（勿动）

- embedded relay 绑定/认证语义（只 warn + 登记 P1-7）
- relay capability token 系统（Wave 3 决策项）
- relay-core 认证逻辑本身（AuthExtractor 行为不变）

## 全局约束（仓库硬规则，逐字生效）

- 日志 English-only，无 emoji。
- 生产 `.rs` 文件 <800 行；>1000 必须拆或加 `// allow-god-file`。
- 触及 `tokio::select!` / cancellation / timeout 竞态的改动必须带自动化测试。
- 不裸跑 `cargo fmt`；新代码手工对齐既有风格。
- 只 commit 本任务范围内文件；commit 前缀 `fix(security):`。不 commit SDD 文档（brief/report/plan）。不 push。
- ledger 翻转与修复同 commit。

## 验证（最小集，必须全跑并记录输出）

```
cargo test -p northhing-relay-server -p northhing-relay-core
cargo check -p northhing-relay-server
```

embedded relay warn 日志若触及 assembly/core：追加 `cargo check -p northhing-core --features product-full`（或该 crate 实际名/feature，以 Cargo.toml 为准）。广覆盖交 CI；不跑 workspace 全量。

## Report

写到 `E:\agent-project\northing\.superpowers\sdd\task-c2-report.md`，必含：

- 状态行：`DONE` / `DONE_WITH_CONCERNS` / `NEEDS_CONTEXT` / `BLOCKED`
- 改动文件清单 + 每文件职责一句话
- key 文件路径选择依据、CORS 接线现状核实结论（是否原本未接线）
- 测试命令 + 真实完整输出（通过/失败统计）
- ledger 翻转 diff 摘要
- 任何偏离 brief 的决定及理由
- **事实核实纪律（C1 教训）**：report 中所有「机制存在/不存在」类结论必须附 file:line 证据；无法核实的写「未核实」，禁止推断成结论。
