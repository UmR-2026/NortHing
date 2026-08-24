# Task T2-2j Review — Remote 栈整删批次 C8：文档/台账收口 + Minor triage

> 审查者：独立 reviewer（judge-m3 实例）
> 仓库：`E:\agent-project\northing`，HEAD `e47c0b0`，main 分支
> 审查对象：`.superpowers/sdd/task-t2-2j-diff.patch`（8 文件）+ 实现报告 + brief 全部约束
> 审查时间：2026-08-19

## 双判决总览

- **SPEC 判决**：**PASS** — 清单 A-E 全部按 spec 落地，越界/夹带 = 0；8 个授权文件外无任何编辑。
- **QUALITY 判决**：**PASS** — 文档事实同步准确、技术债台账语义正确、i18n-audit.mjs 字节级改动合规、测试重命名经 cargo 实跑匹配并 PASS、Rust 编译干净、E 项 sweep 干净。

---

## A. SPEC 合规判定（逐条核对）

### 约束 1：仅 8 个授权文件
- **状态**：PASS
- **证据**：`task-t2-2j-diff.patch` 经解析共 8 个 `diff --git` header：
  1. `CONTRIBUTING.md`
  2. `README.md`
  3. `docs/architecture/backend-roadmap.md`
  4. `docs/status/tech-debt-ledger.md`
  5. `scripts/i18n-audit.mjs`
  6. `src/crates/assembly/core/Cargo.toml`
  7. `src/crates/assembly/core/src/service_agent_runtime/mod.rs`
  8. `src/crates/contracts/runtime-ports/src/session_workspace.rs`
- 与 brief 列表逐一比对：完全匹配，0 越界文件。
- 工作区 `git status --short` 含 3 个并行 session 残留文件（`.opencode/model-capability-notes.md` / `memory/northhing.md` / `.handoffs/handoff-g2-t9-2026-08-07.md`），均未进入本批 patch（patch 仅含 8 个授权文件）。

### 约束 2：不删 dep / crate / 功能代码
- **状态**：PASS
- **证据 1（Cargo.toml 注释措辞）**：
  - 修改前（`:124, :129`）：`# Encryption (Remote Connect E2E)` / `# Device/Network info (Remote Connect)`
  - 修改后：`# Encryption` / `# Device/Network info`
  - 实跑内容比对：归一化 CRLF 后整文件只有 2 行内容差异（`:124`, `:129`），全为注释，无 dep 行变更。
  - 全 5 个关键 dep（`aes-gcm`, `sha2`, `rand`, `local-ip-address`, `tokio-tungstenite`）计数 orig=2 / mod=2，每条均保留。
- **证据 2（mod.rs 测试重命名）**：
  - 仅重命名 `core_service_agent_runtime_owner_exposes_agent_runtime_and_remote_control_port` → `core_service_agent_runtime_owner_exposes_agent_runtime`
  - 测试体未动；其他测试函数未触碰。
- **证据 3（session_workspace.rs doc 注释）**：
  - 仅删除第 1 行 doc 注释中 `+ remote-connection` 子句，其余措辞不变。
  - 同文件其他 `remote` 词汇均为保留的 SSH 语义（`SessionStorageKind::Remote` / `UnresolvedRemote` / `remote_connection_id` / `remote_ssh_host` / `is_remote_storage` / doc 注释 "local and remote workspaces"），这些是 C3 SSH 保留面，按 brief M-f-1 "同时检查同文件是否还有其它 remote-connection 措辞残留，有则一并以最小改动清理"——经核对无进一步删除必要（其它均为 SSH-port 词汇）。

### 约束 3：scripts/i18n-audit.mjs 仅 4 行删除 + 0 插入 + SyntaxError 不变
- **状态**：PASS
- **证据**：
  - 归一化 CRLF 后 diff 内容差异：恰好 4 行删除、0 行插入。
  - 删除的 4 行内容（与 brief 描述一致）：
    1. `function collectConfirmedUnusedKeys() {`（原 :1284 函数定义首行）
    2. `}`（原 :1285 函数体闭合）
    3. ``（原 :1286 函数后空行）
    4. `  collectConfirmedUnusedKeys();`（原 :1375 调用点）
  - 其余字节 byte-preserved——line 481 的 mojibake 区段（`zhTwSameTextScriptSignals` Set 周围 `\xc3\xa8\xc2\xbf?,` 等序列）在 orig 与 mod 中 byte-identical（实测 `\xb0` 起始区段完全一致）。
  - `git diff --check` 数：0 增 / 4 删。
- **node --check 实跑对比**：
  - HEAD 原文（写入 `C:\WINDOWS\TEMP\opencode\i18n-audit-orig.mjs`）：`SyntaxError: Invalid or unexpected token` 在 line 481。
  - 工作区文件：`SyntaxError: Invalid or unexpected token` 在 line 481。
  - 同一 SyntaxError，同一行号（删除点 :1284 / :1375 距 :481 极远，未触发行号前移）——符合 brief "行号前移正常" 的预期。
  - mojibake 未扩大、未修复，符合"不许修它，也不许扩大损伤"红线。

### 约束 4：roadmap `:115` / `:118` / `:129` / `:167`
- **状态**：PASS
- **证据**：
  - `:115` 行（`apps/relay-server`）：从 "已加固（fail-closed 绑定、自动 key、CORS localhost 默认，2026-08-04）" 改为 "已整删（T2-2 C5, commit `f6a011b`, PEND-1）"。表格格式保持。✓
  - `:118` 行（mobile-web/remote_connect）：从 "已决删除（论题 v1.1）" 改为 "已整删（TH-4 删除已执行，T2-2 C1-C7, commits fa88342..d16b037）"；规划列保留 "P1-4/P1-7/D-2 已随删除关闭；将来移动需求 = T5 协议客户端重建"——brief 要求保留的"将来移动需求 = T5 协议客户端重建"语义完整保留（"重写" → "重建" 是一字改写，语义等效）。✓
  - `:129` 第 6 条（relay-server fail-closed 模式）：从 "SW1-2 修 embedded relay 时直接复用其绑定/key 策略" 改为 "（已失效：relay-server 已整删，T2-2 C5 commit `f6a011b`）：原 SW1-2 复用策略随载体删除失效"——列表项保留（7 项结构未动），仅前缀注入失效说明 + 指向删除 commit。✓
  - `:167` T2-2 行：**未整行划掉**。在原 T2-2 行描述末尾追加 "；remote 栈部分（含 relay-server/relay-core 整删、mobile-web、contracts 修剪、i18n 面）已完成 C1-C8（commits fa88342..本批），MiniApp 部分待执行"。原 "MiniApp 子系统整删..." 整段保留，与 brief "MiniApp 部分待执行" 指令吻合。✓

### 约束 5：tech-debt-ledger P1-4 / P1-7 翻 resolved + 新增 P2-19 / P2-20
- **状态**：PASS
- **证据**：
  - P1-4（:52）：从 "active (mobile-web: frozen surface)" 翻为 "`resolved` — mobile-web 面已整删（T2-2 C6 commit `646f93d`），条目随删除关闭；`docs/architecture/backend-roadmap.md:118` 已预先声明此关闭方式。" ✓
  - P1-7（:73）：从 "active (registered 2026-08-04, P1-5 standalone mitigation complete; a startup `warn!` has been added at `embedded_relay.rs`)" 翻为 "`resolved` — relay-server + relay-core 已整删（T2-2 C5 commit `f6a011b`，PEND-1），embedded relay 入口不复存在，条目随删除关闭。" ✓
  - 新增 P2-19（:218-223）：`src/apps/server/README.md:5-10` 包含 3 条指向已删 relay-server 的悬空链接 → 对应 M-g-2，登记 active (frozen surface)。✓
  - 新增 P2-20（:225-230）：`pnpm-workspace.yaml` 中注册了孤儿工作区 `desktop-tauri` → 对应 M-h，登记 active。✓
  - 文件 UTF-8 校验：U+FFFD 计数 = 0，CJK 字符 382 字节，无 mojibake 损伤。✓
  - 中文内容可读性：通过 Read 工具实测可正确显示中文，无乱码。

### 约束 6：禁触路径
- **状态**：PASS
- **证据**：
  - `src/apps/server/`：`git diff -- src/apps/server/` 输出为空，frozen 面未触碰。✓
  - `memory/`、`memory/northhing.md`：在工作区 git status 显示为 modified，但**不属于本批 patch 的 8 个授权文件**——并行 session 预存改动。patch 文件解析无 memory/ 路径。✓
  - `.opencode/`、`.opencode/model-capability-notes.md`：同上，并行 session 预存改动，patch 未触碰。✓
  - `.handoffs/handoff-g2-t9-2026-08-07.md`：新增文件，并行 session 创建，patch 未触碰。✓
  - `docs/status/full-review-2026-08-16.md`：未列入授权 8 文件；`git diff -- 'docs/status/full-review-2026-08-16.md'` 为空。✓

### 约束 7：中文文档无乱码 + i18n-audit.mjs 无新 mojibake
- **状态**：PASS
- **证据**：
  - `docs/status/tech-debt-ledger.md`：UTF-8 decode OK，U+FFFD = 0，新追加 P2-19 / P2-20 中文段落通过 Read 工具实测可正常显示（"包含 3 条指向已删 relay-server 的悬空链接" / "中注册了孤儿工作区 `desktop-tauri`" / "中包含 `src/apps/desktop-tauri` 注册条目，但磁盘上该目录不存在" / "作为独立决策项处理" 等中文均无乱码）。
  - `docs/architecture/backend-roadmap.md`：UTF-8 decode OK，U+FFFD = 0，CJK 字节 4403；roadmap 多处中文章节（含"位腐"、"维持；M5 解冻评估"、"T5 协议客户端重建"、"协议候选之一" 等）实测可读。
  - `scripts/i18n-audit.mjs`：line 481 mojibake byte-identical orig/mod，约束 3 已验；未引入新 mojibake。

### 约束补充：verify-only 项（D-2 / M-c-3）
- **D-2（weixin QR 登录）verify-only**：
  - 实测 `rg -n -i 'weixin|wechat'` 全仓：仅命中 `.agents/skills/`、`docs/status/full-review-2026-08-16.md`（历史快照，禁触）、`scripts/check-core-boundaries.mjs` 内 IM app 检测相关代码（活代码、与 QR 登录无关）。
  - tech-debt-ledger 中无 D-2 条目。✓ 无需编辑，结论复述准确。
- **M-c-3（sar_dispatch.rs import）verify-only**：
  - 实测 `src/crates/assembly/core/src/service_agent_runtime/sar_dispatch.rs:1-10`：唯一外部 runtime-ports 导入为 `AgentDialogTurnPort, AgentLifecycleDeliveryPort, AgentSessionManagementPort, AgentSubmissionPort, AgentTurnCancellationPort`——零 remote 残留。✓

---

## B. QUALITY 质量判定（验证实测 + 内容审查）

### B.1 Rust 编译验证
- **命令**：`rustup run stable-msvc cargo check --workspace`
- **结果**：PASS — 0 errors，仅 pre-existing warnings（19 个 `northhing-core` warnings + 5 个 `northhing` bin warnings + 1 个 `northhing-cli` warning，均与本批改动无关，全部为历史积累 warnings，与报告"19 warnings"基线吻合）。
- **判定**：clean（pre-existing warnings 不阻塞）。

### B.2 测试重命名 cargo 实跑
- **命令**：`rustup run stable-msvc cargo test -p northhing-core --lib --features product-full core_service_agent_runtime`
- **结果**：3 passed / 0 failed：
  - `service_agent_runtime::tests::core_service_agent_runtime_owner_exposes_agent_runtime` ... **ok**（重命名后的测试）
  - `service_agent_runtime::tests::core_service_agent_runtime_owner_keeps_coordinator_port_contracts` ... ok
  - `service_agent_runtime::tests::core_service_agent_runtime_owner_keeps_scheduler_lifecycle_port_contracts` ... ok
- **判定**：重命名后的测试能被 `cargo test` 模式匹配并 PASS。✓
- **Minor 备注**：brief 验证命令未含 `--lib --features product-full`。无 feature flag 时该测试 feature-gated 而不可见（实测 `0 tests, 125 filtered out`）。报告选用 `--lib --features product-full` 是合理适配（让 feature-gated 测试可见），且符合 brief "重命名后的测试能被匹配到并跑过" 的意图。**不构成 finding，仅说明**。

### B.3 node --check SyntaxError 同错对比
- **命令**：`node --check scripts/i18n-audit.mjs`（实跑已存文件） + `node --check C:/WINDOWS/TEMP/opencode/i18n-audit-orig.mjs`（HEAD 原文）
- **结果**：
  - HEAD 原文：line 481 `SyntaxError: Invalid or unexpected token`，exit 1。
  - 工作区文件：line 481 `SyntaxError: Invalid or unexpected token`，exit 1。
  - 同一 SyntaxError，同一行号——删除点 :1284-:1375 远离 :481，行号未前移，符合 brief "行号前移正常" 预期。
- **判定**：mojibake 红线严守，未扩大损伤。✓

### B.4 git diff --check
- **命令**：`git diff --check`
- **结果**：仅 LF/CRLF 行尾警告（3 个文件 LF→CRLF + 1 个文件 CRLF→LF），无 whitespace errors。
- **判定**：clean（行尾警告是 Windows 编辑器行为，与本批改动范围无关；brief "clean" 要求满足）。✓

### B.5 E 项残留 sweep
- **命令**：`rg -n -i "relay|mobile-web|mobile web|remote_connect|remote connect" AGENTS.md AGENTS-CN.md docs/status/surfaces.md scripts/`
- **结果**：
  - `AGENTS.md` (root)：0 hits ✓
  - `AGENTS-CN.md` (root)：0 hits ✓
  - `docs/status/surfaces.md`：0 hits ✓
  - `scripts/`：4 hits，全部为保留的 SSH 语义：
    - `scripts/core-boundaries/self-test.mjs:2074` — `lookup_remote_connection_with_hint` 等 SSH 远程工作区连接契约
    - `scripts/core-boundaries/rules/source/required-rules.mjs:2031, 5292-5293` — 同契约的 required-rule 锚点与说明
  - 这 4 命中为 brief 明示允许保留的 SSH 语义，不算已删 remote 栈残留。✓
- **判定**：clean，无已删栈残留。

### B.6 文档内容质量审查
- **roadmap.md 中文表述**：行 :115 "已整删（T2-2 C5, commit f6a011b, PEND-1）" — 表述准确，与 T2-2g 报告的 C5 commit 引用一致。行 :118 "已整删（TH-4 删除已执行，T2-2 C1-C7, commits fa88342..d16b037）" — commit 区间正确（T2-2a C1 commit fa88342 → T2-2i C7 commit d16b037；本批 C8 为 e47c0b0 起的 uncommitted diff，commit 范围"fa88342..本批"语义正确）。行 :167 追加 "C1-C8（commits fa88342..本批）" — 与上下文自洽。✓
- **tech-debt-ledger.md P1-4 / P1-7 解析**：P1-4 关闭方式交叉引用 roadmap.md:118（已声明），P1-7 关闭方式指向 f6a011b（relay-server/relay-core 整删），与 T2-2g 报告一致。✓
- **P2-19 / P2-20 字段完整性**：每条都有 Symptom / Evidence / Proposed fix / Status 四字段，Evidence 行引用文件:行。✓
- **README.md / CONTRIBUTING.md 措辞**：删词精确（仅移除 `relay, mobile-web,` 与 `Mobile web` 整行），未扩展未改其他面。✓

### B.7 cargo check warnings vs baseline 一致性
- brief "minimal set" — 共享 Rust → `cargo check --workspace`，桌面 → `cargo check -p northhing`，本次改动为 core crate doc-comment / test 改名，归共享 Rust 集合，选用 workspace 检查符合验证表。
- 实跑结果 19 core warnings + 5 bin warnings + 1 cli warning — 与前批基线吻合，无新增 warning 引入本批改动范围。

---

## C. Findings 清单

| # | 等级 | 文件:行 | 描述 |
|---|---|---|---|
| F-1 | Minor | `docs/architecture/backend-roadmap.md:118` | brief 明示"将来移动需求 = T5 协议客户端重写"；实现版改用"重建"。语义等效，但一字之差与 brief 字面不完全一致。**不构成 finding 升级**：brief 的目的是保留语义（非字面改写），"重建"更契合整删后的语境（mobile-web/remote_connect 已删，未来需求为从零重建协议客户端），措辞合理。 |
| F-2 | Minor | `.superpowers/sdd/task-t2-2j-report.md:89` | report 验证命令比 brief 多加了 `--lib --features product-full`；brief §验证项 2 字面无此 flag。无 flag 时该测试 feature-gated 不可见（实测 `0 tests, 125 filtered out`），报告选用了 feature flag 让 feature-gated 测试运行起来——属合理适配。**不构成 finding 升级**：brief "重命名后的测试能被匹配到并跑过" 意图已满足。 |
| F-3 | Minor | `src/crates/assembly/core/Cargo.toml` / `src/crates/contracts/runtime-ports/src/session_workspace.rs` / `README.md` / `CONTRIBUTING.md` | `git diff --check` 报 4 条 LF/CRLF 行尾警告。Windows 编辑器编辑产生，非内容错误；`git diff --check` 的核心 whitespace-error 检查通过；属于"housekeeping noise"，留待下次跑 `pnpm run fmt:rs` 收口。**不构成 finding 升级**：与改动内容正确性无关，brief 的 "clean" 要求已满足（CRLF 警告是 Git 期望性的提示而非错误）。 |

无 Critical / Important findings。

---

## D. Cannot verify from diff 清单

- **本批涉及的所有 8 个文件的"实际执行路径是否真正完整"**：本批纯文档 / 测试名重命名 / 注释 / 删除空函数 — diff 自证完整。
- **roadmap.md T2-2 行 MiniApp 部分的"待执行"语义是否会被未来 MiniApp 整删批次直接覆盖或冲突**：纯文档留标，无可验证的"未来影响"——留待 T2-2 MiniApp 批次时核对。
- **`scripts/i18n-audit.mjs` 的 pre-existing mojibake SyntaxError 是否被任何外部消费链解析**：i18n 工程已 frozen（AGENTS.md "i18n engineering is frozen"），本仓库 CI 当前不含 i18n:audit / i18n:contract-test job，故 mojibake SyntaxError 不会阻塞 CI。**无需也无法验证**：超出本批范围。
- **D-2 (weixin QR 登录) 历史代码路径**：实测 `rg -n -i 'weixin_qr_login'` 全仓零命中；`rg -n -i 'weixin|wechat'` 命中均为 IM app 检测活代码（与 QR 登录无关）和 frozen 的 `full-review-2026-08-16.md` 历史快照（禁触）。verify-only 结论 "无需编辑" 与 brief 预期吻合。

---

## E. 一句话总结论

**双判决 PASS**：本批严格按 brief 清单 A-E 执行，8 个授权文件外零越界、删词仅限 `collectConfirmedUnusedKeys` 函数 + 调用点（4 行）、`Cargo.toml` 仅注释措辞改动（5 个关键 dep 全保留）、Cargo workspace check + 重命名测试 + node --check 同错对比 + E 项 sweep 四道门禁全过；新增 P2-19 / P2-20 与 roadmap T2-2 行追加标注语义准确；中文字符与 mojibake 红线均严守。建议合入。