# W26-4 Brief — P2-18 预留标注 + stop_server 死分支清理（lsp manager.rs，单 judge）

## 1. 来源与验收标准（逐字）

Plan §1：
> "W26-4 P2-18 预留标注 + stop_server 死分支（lsp manager.rs，小）[单 judge]"

用户拍板（2026-09-09）："P2-18 加预留 API 标注 + 清 stop_server 死分支"。

ledger P2-18（tech-debt-ledger.md:211-216）：`uninstall_plugin` 无生产调用方（仅测试调用）；"also note `stop_server` always returns `Ok`, which makes the new warn branch unreachable"。

### 验收标准（机械可核对）

- AC1: `uninstall_plugin`（manager.rs:103-146）加预留标注注释，形态对齐仓内先例（`// reason: ... reserved for ...`，先例：workspace_manager/workspace.rs:450、client.rs:283）。
- AC2: 死分支清理：manager.rs:129-134（uninstall_plugin 内的 Err 分支）与 :304-306（shutdown 内的 Err 分支）——`stop_server` 恒返 Ok（:231-243，内部 warn-and-continue），这两处 `if let Err` 不可达。清理 = 调用点改为直接 `.await` + 一行注释指明 stop_server 刻意不报错（warn-and-continue 设计）。**不改 stop_server 签名/行为**（调用方含 workspace_manager/client.rs:343 的 `?` 用法，签名不动）。
- AC3: **行数硬约束**：manager.rs 现 836/836 顶格（god-file ceiling 836，rot-budget.json 登记）——本单完工后 countLines **必须 < 836（净减）**，report 贴前后行数证据。
- AC4: **台账 P2-18 翻转由编排者在波收口执行**（ledger 不进场——W26-2 同翻 P2-5，同文件并行冲突规避）；本单 report 给建议翻转文案（**限定 manager.rs 内两处死分支**；workspace_manager/format.rs:160/:187 的同形态残留不在本单声明范围）。
- AC5: §6 验证全绿。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `cc586ae`。

| 事实 | 证据 |
|---|---|
| stop_server 恒 Ok 链：manager.rs:242 末尾 `Ok(())` + process_protocol.rs:244-259 全 `let _ =` | 侦察实证 |
| 死分支两处：:129-134（6 行，含 rollback_registration 调用——死代码）与 :304-306（3 行 error! 日志） | 侦察实证 |
| uninstall_plugin 生产零调用，仅 manager.rs:791/811/828 测试调用 | 侦察实证 |
| 预留标注先例原文形态：`// reason: ensure_server_running() is reserved for the upcoming server auto-start path...` | workspace.rs:450 |
| 文件 836/836 顶格（家规：只许减不许增） | rot-budget.json + 侦察复核 |
| 非 meta-ratchet → 单 judge | workflow-policy.json |

## 3. 复用侦察（强制）

- 标注形态复用仓内 reason: 先例。report 必须有「复用侦察」节。

## 4. Spec（必须全部满足）

- S1: AC1 标注（注释内容：reserved for upcoming plugin-management surface；测试覆盖在 manager.rs 内既有用例，不新增测试文件）。
- S2: AC2 两处清理。注意 :129-134 的 `rollback_registration` 调用随死分支一起消失，但 `rollback_registration` 仍有 :139 活调用方（复审实证）——**保留该函数，不删**。report 说明处置。
- S3: AC3 行数净减（现实预期净 -3~-5 行：两处死分支 -5、标注 +1；**达标线以 AC3 <836 硬约束为准**，rollback_registration 有 manager.rs:139 活调用方，不可删）。
- S4: 禁区：lsp/ 其它文件、stop_server 签名与行为、workspace_manager/、W26 其它单领地、ci.yml、scripts/。
- S5: 不改变任何运行时行为（stop_server 本就恒 Ok；清理的是不可达分支）。

## 5. Global Constraints（逐字遵守）

- 行数净减（AC3 硬约束，越线 = FAIL）。
- 日志英文无 emoji。
- 禁整树 git 操作；只点名 add/commit。
- 测试真实执行，输出原文进 report；路径卫生（D-2）；cargo 带 rustup 前缀。

## 6. 验证（命令 + 输出原文进 report）

1. `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace` → 0 error
2. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full lsp` → 绿（manager 既有用例 :791-828 区全过）
3. 行数证据：report 贴改动前后 countLines（可用 verify-rot-budget.mjs 输出或自数）
4. `node scripts/check-repo-hygiene.mjs` → exit 0
5. `node scripts/verify-rot-budget.mjs --base cc586ae` → exit 0（god_file lsp/manager.rs 读数须 <836——净减实证）
6. `node scripts/verify-task-gate.mjs verify-attempt --base cc586ae --tip <本单最后 commit sha> --allowlist .superpowers/sdd/w26-4-allowlist.txt` → exit 0（自建 allowlist 含自身；**窗口内若出现其它 W26 单文件，停手报编排者复跑**）

## 7. 报告

路径 `.superpowers/sdd/w26-4-report.md`。章节：改动摘要 / 复用侦察 / 验证 / 疑虑 / 状态。

## 8. 派发元信息

- BASE: `cc586ae`
- 允许文件集：
  - `src/crates/assembly/core/src/service/lsp/manager.rs`
  - `.superpowers/sdd/w26-4-brief.md` / `w26-4-report.md` / `w26-4-allowlist.txt`
- 禁区：§4-S4 + 其它未列出文件。
- commit 规则：点名 git add；前缀 `chore(lsp): W26-4`，body 注「用户 2026-09-09 拍板 P2-18」。
- 并行声明：W26-1/2/3/5 同波全并行，文件集不相交。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。

