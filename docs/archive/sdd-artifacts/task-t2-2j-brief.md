# Task T2-2j Brief — Remote 栈整删批次 C8：文档/台账收口 + Minor triage（无产品行为代码）

> 上下文：T2-2 remote 栈整删 C1-C7 已并 main（commits fa88342..d16b037，全部 review clean）。
> 本批是纯收口：文档事实同步 + tech-debt ledger 翻状态 + 前 7 批积累的 Minor triage。
> 除明确列出的文件外，**不得触碰任何其它文件**。本批不删任何 crate / dep / 功能代码。

## 工作目录
`E:\agent-project\northing`（git 仓库，当前 HEAD = e47c0b0，分支 main）

## 环境硬事实（必读）
- cargo 一律走 MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`（仓库目录 override 是 GNU，`cargo +toolchain` 不可用）。
- PowerShell 写非 ASCII 会 GBK 双重编码 → 一律用 edit 工具改文件。
- `scripts/i18n-audit.mjs` 有 **pre-existing mojibake 语法级损伤**（约 :481 处截断字符串，文件本就无法 parse，C5/C7 双向实证）。**不许修它，也不许扩大损伤**。判据：改动后 `node --check scripts/i18n-audit.mjs` 报**同一个** SyntaxError（行号前移正常）= 未扩展损伤。
- 工作区有并行 session 的未提交改动（`memory/`、`.opencode/model-capability-notes.md`、`.handoffs/`），**勿碰勿提交**。
- i18n 工程 frozen；`src/apps/server` frozen-experimental。

## 变更清单（逐项，全部显式授权）

### A. tech-debt ledger 翻状态 — `docs/status/tech-debt-ledger.md`
1. **P1-4**（约 :47，"Mobile-web re-pairing has no guidance"）：翻为 resolved。resolution 注明：mobile-web 面已整删（T2-2 C6 commit 646f93d），条目随删除关闭；roadmap:118 已预先声明此关闭方式。P1-4b 已是 resolved，勿动。
2. **P1-7**（约 :68，"Embedded relay open mode"）：翻为 resolved。resolution 注明：relay-server + relay-core 已整删（T2-2 C5 commit f6a011b，PEND-1），embedded relay 入口不复存在，条目随删除关闭。
3. **D-2 同查（verify-only，不编辑）**：D-2 = weixin QR 登录去留（`docs/status/full-review-2026-08-16.md:231`）。编排者已实测：repo 全域 `weixin|wechat` 命中仅剩 bash/computer-use 工具的 IM app 检测（活代码，无关 QR 登录），QR 登录代码已随 remote 栈删除归零，且 tech-debt-ledger 无 D-2 条目。**预期结论：无需编辑**。在你的 report 里复述此结论；若你发现任何残留的 QR 登录代码或 ledger 里的 D-2 条目，STOP 并报告，不要自作主张。

### B. roadmap 标注 — `docs/architecture/backend-roadmap.md`
4. **:115 行 relay-server 行**（"已加固…维持；M5 解冻评估…"）：relay-server 已删除，该行事实已失效。更新为：已整删（T2-2 C5, commit f6a011b, PEND-1）。保持表格格式。
5. **:118 行 mobile-web/remote_connect 行**：标执行完毕——TH-4 删除已执行（T2-2 C1-C7, commits fa88342..d16b037）；P1-4/P1-7/D-2 已随删除关闭；"将来移动需求 = T5 协议客户端重建" 保留。
6. **:129 行第 6 条**（"relay-server fail-closed 模式：SW1-2 …直接复用其绑定/key 策略"）：relay-server 已删，该策略载体不复存在。标注失效并指向删除 commit（f6a011b）；不要整段删除 SW 列表结构。
7. **:167 行 T2-2 行**：**不要整行划掉**（MiniApp 子系统整删部分仍 active）。在 remote 栈整删语段处追加标注：remote 栈部分（含 relay-server/relay-core 整删、mobile-web、contracts 修剪、i18n 面）已完成 C1-C8（commits fa88342..本批），MiniApp 部分待执行。

### C. README / CONTRIBUTING 摘除
8. **README.md:43**：`**Frozen-experimental**: CLI, server, relay, mobile-web, MiniApp UI, SDLC harness.` → 移除 `relay, mobile-web,`，保留 CLI / server / MiniApp UI / SDLC harness。
9. **CONTRIBUTING.md:146**：删除 `| Mobile web | `pnpm --dir src/mobile-web run type-check` |` 整行（目标目录已不存在）。

### D. Minor triage（前批 review 记录，逐条见 `.superpowers/sdd/progress.md` T2-2 各行）
10. **M-c-1** `src/crates/assembly/core/Cargo.toml:124` 与 `:129`：两处 section 注释 `# Encryption (Remote Connect E2E)` / `# Device/Network info (Remote Connect)` 措辞失效。**只改注释措辞**（去掉 Remote Connect 归因，写成中性描述，如 `# Encryption` / `# Device/Network info`）。**严禁删除任何 dep 行**——aes-gcm/sha2/rand/local-ip-address/tokio-tungstenite 仍在（optional），删除超出本批范围。若你怀疑某 dep 已成孤儿，只在 report 里报告，不动手。
11. **M-c-2** `src/crates/assembly/core/src/service_agent_runtime/mod.rs:46`：测试名 `core_service_agent_runtime_owner_exposes_agent_runtime_and_remote_control_port` 中的 `_and_remote_control_port` 为无实体残留（cosmetic）。重命名为 `core_service_agent_runtime_owner_exposes_agent_runtime`，测试体不动。
12. **M-c-3** `sar_dispatch.rs:2-5` `use northhing_runtime_ports::{...}`：编排者已复核——import 列表无 remote 残留（remote.rs 已在 C4 整删）。**verify-only**，report 复述即可；若你实际看到残留，STOP 并报告。
13. **M-f-1** `src/crates/contracts/runtime-ports/src/session_workspace.rs:1`：模块 doc 残留 "remote-connection port traits"（该 port 已删）。改为中性描述（删掉 `+ remote-connection` 分句即可，保持其余措辞不变）。**同时检查同文件是否还有其它 remote-connection 措辞残留**，有则一并以最小改动清理，report 列出每处。
14. **M-g-1** `scripts/i18n-audit.mjs`：`collectConfirmedUnusedKeys`（约 :1284）已成空函数，调用点（约 :1375）是死调用。删除函数定义 + 调用点两处，其它一律不动。**逐字遵守上方 mojibake 红线**：改动前后各跑一次 `node --check scripts/i18n-audit.mjs`，两次必须报**同一个** SyntaxError（行号前移正常）；若报错变了/消失了，说明动到了不该动的字节，立即 `git checkout -- scripts/i18n-audit.mjs` 重来。除这两处外文件中所有字节（含 mojibake）必须 byte-preserved。
15. **M-g-2** `src/apps/server/README.md:5-10` relay-server 悬空链接：server 为 frozen 面，**本批不编辑**。在 `docs/status/tech-debt-ledger.md` 新增一条 P2 级条目登记：server/README.md:5-10 有 3 条指向已删 relay-server 的悬空链接，留待 server 解冻时同步（来源：T2-2g review M-g-2）。
16. **M-h** pnpm-workspace.yaml desktop-tauri orphan workspace 注册（磁盘无目录）：**独立决策项，本批不编辑**。同上一并写入新 P2 条目（来源：T2-2h review F1/M-h）。

### E. 残留 sweep（verify-only）
17. 编排者已实测：根 `AGENTS.md`/`AGENTS-CN.md`/`docs/status/surfaces.md`/boundary 脚本（`scripts/*.mjs`）中 relay/mobile-web/remote_connect 残留 = 0（各批已同步）。你复跑一遍同样 sweep：`rg -n -i "relay|mobile-web|mobile web|remote_connect|remote connect" AGENTS.md AGENTS-CN.md docs/status/surfaces.md scripts/`（排除 archive/docs-handoffs/historical docs 与本批明确允许保留的 SSH 语义命中）。若有新命中，STOP 并报告，不擅自扩大范围。

## 不做（红线）
- 不删任何 dep / crate / 功能代码；不修 mojibake；不碰 `memory/`、`.opencode/`、`.handoffs/`、并行 session 文件。
- 不碰 `src/apps/server/` 任何文件（frozen）。
- 不改 `full-review-2026-08-16.md`（历史快照文档）。
- 不做任何超出清单 A-E 的"顺手"改动；发现新问题写进 report。

## 验证（每条都要跑，report 贴命令 + 输出原文）
1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace` → PASS（改了 .rs 文件）
2. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core core_service_agent_runtime` → PASS（重命名后的测试能被匹配到并跑过）
3. `node --check scripts/i18n-audit.mjs`（改前改后各一次）→ 同一 SyntaxError
4. `git diff --check` → clean
5. E 项 sweep 的 rg 输出原文
6. `git status --short` → 仅含本批清单内文件 + 并行 session 的预存改动

## Report 要求
写到 `.superpowers/sdd/task-t2-2j-report.md`：每项变更一段（文件：行 + 改动前→后摘要），验证命令+输出原文，D-2/M-c-3 verify-only 结论复述，E 项 sweep 结果，任何偏离/发现。假汇报 = 停用；编排者将 diff 逐条核对。

## 完成后
不要自己 commit。回到 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED 状态汇报。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
