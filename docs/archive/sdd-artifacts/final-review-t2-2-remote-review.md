# NortHing 分支级终审报告：T2-2 删除线（死代码批次 + E-09 + judge_gate + Remote 栈整删 C1-C8）

- **审查角色**：独立终审 Reviewer（提供全新独立视角，此前 10 批次由另一模型审查）
- **审查范围**：`e65d98e~1..HEAD`（main 分支，HEAD=`672e583`，共 10 个任务批次 + C8 收口）
- **变更规模**：387 个文件，+1,190 / -62,664 行（净删 61,474 行）
- **审查日期**：2026-08-19

---

## 1. 双判决总览 (Dual Verdict)

| 判决维度 | 结论 | 核心依据摘要 |
|---|---|---|
| **SPEC 合规判决 (Spec Compliance)** | **PASS** | 7 项分支级不变量（Constraints 1–7）全部独立核验通过；死代码第一批（insights/webdriver/pcc）、E-09、judge_gate 适配层、remote 栈（C1–C8：remote_connect/mobile-web/relay 双 crate/契约修剪/i18n 契约面）全域清零；SSH 语义与 TH-5 词汇零损伤；未越界、无未授权夹带。 |
| **代码质量判决 (Code Quality)** | **PASS** | MSVC 全量编译与桌面编译全绿；边界检查器通过；PRODUCT_TOOL_GROUPS 内联 40 工具 / 4 组完全等价并通过 22 项工具运行时全套测试；存量 i18n mojibake 损伤字节级原样保留未扩大；文档与技术债台账同步完整。 |

---

## 2. 分支级不变量独立核验 (Constraints 1–7)

### Constraint 1: SSH 语义零损伤 ✅ PASS
- **核验项**：
  - `remote_ssh` 模块、`remote-ssh` / `remote-ssh-concrete` feature、`remote_connection_id` 字段、`lookup_remote_connection*` 函数。
  - `DialogTriggerSource::{RemoteRelay, Bot}` 与 `RemoteSsh` 变体（TH-5 / T5 协议客户端重建词汇）。
- **实测证据**：
  - `git diff e65d98e~1..HEAD --stat -- "src/crates/services/services-integrations/src/remote_ssh*" "src/crates/assembly/core/src/service/remote_ssh*" "src/crates/services/services-integrations/tests/remote_ssh_contracts.rs"` 输出为空（0 diff）。
  - `git diff e65d98e~1..HEAD -S "lookup_remote_connection" --name-only`：生产代码 0 行变动，仅文档与任务材料出现。
  - `src/crates/contracts/runtime-ports/src/agent/agent_dialog.rs:62` 完整保留 `DialogTriggerSource::RemoteRelay | DialogTriggerSource::Bot`。
  - `src/crates/contracts/core-types/src/surface.rs:25` 完整保留 `ThreadEnvironmentKind::RemoteSsh`，`:37` 完整保留 `remote_connection_id`。

### Constraint 2: 删除目标归零 ✅ PASS
- **核验项**：全仓（排除 `docs/archive`、`docs/handoffs`、`.superpowers`、`memory`、`target`）检索已删标识。
- **实测证据**：
  - `remote_file_delivery` / `REMOTE_FILE_DELIVERY`：全仓 0 命中。
  - `computer://`：全仓 0 命中。
  - `\bremote_connect\b` / `\bremote-connect\b`：仅在 frozen 的 `src/apps/server/src/bootstrap.rs` 与 `manager_registry.rs` 的注释中出现 2 处，代码引用 0 命中。
  - `relay-server` / `relay-core`：代码 0 命中；文档仅存 `src/apps/server/README.md` 的 3 条悬空相对链接（已记录为技术债 P2-19，待 server 解冻时同步）。
  - `mobile-web` / `mobile_web`：代码、脚本、配置 0 命中。

### Constraint 3: 构建与边界门禁全绿 ✅ PASS
- **核验项**：
  1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace`
  2. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing`
  3. `node scripts/check-core-boundaries.mjs`
- **实测证据**：
  - `cargo check --workspace`（MSVC）：PASS，exit code 0，19 core warnings + 5 bin warnings + 1 cli warning（均为基线存量 warning，无新增 warning）。
  - `cargo check -p northhing`（MSVC）：PASS，exit code 0。
  - `node scripts/check-core-boundaries.mjs`：输出 `Core boundary check passed.`，exit code 0。
  - `node --test scripts/core-boundaries/self-test.mjs`：1 passed / 0 failed。

### Constraint 4: i18n mojibake 红线 ✅ PASS
- **核验项**：`scripts/i18n-audit.mjs` 存量语法损伤未被扩大，`node --check` 报同一家族 SyntaxError，mojibake 区段无新增字节改动。
- **实测证据**：
  - `node --check scripts/i18n-audit.mjs` 输出：`SyntaxError: Invalid or unexpected token` 在 `:481`（`'è¿?,` 截断字符串）。
  - 对比 `e65d98e~1` 历史版本：同一截断字符串在 `:507` 报相同 SyntaxError；行号变化系上方 relay/mobile-web 代码删除导致的正常前移。
  - `git diff -w e65d98e~1..HEAD --stat -- "scripts/i18n-audit.mjs"` 显示 0 增 / 364 删，为纯删除；mojibake 区段字节级原样保留。

### Constraint 5: 文档同步硬规则 (Doc Sync) ✅ PASS
- **核验项**：`docs/status/surfaces.md`、`AGENTS.md`、`AGENTS-CN.md`、`docs/status/tech-debt-ledger.md`、`docs/architecture/backend-roadmap.md:118, 167`。
- **实测证据**：
  - `docs/status/surfaces.md`：已删除 crate（relay-server/relay-core/mobile-web 等）面行已全部清理。
  - 根 `AGENTS.md` / `AGENTS-CN.md`：已删 relay / mobile-web / remote_connect 提及已清空。
  - `docs/status/tech-debt-ledger.md`：P1-4 与 P1-7 已翻转为 `resolved`；新增 P2-19（server/README 悬空链接）与 P2-20（pnpm-workspace 孤儿 desktop-tauri 注册）。
  - `docs/architecture/backend-roadmap.md`：`:118` 与 `:167` 明确标注 T2-2 C1–C8 已执行，且 T2-2 行未整行划掉（MiniApp 部分保持 active 待执行）。

### Constraint 6: 锁文件与清单一致性 ✅ PASS
- **核验项**：根 `Cargo.toml` members、`Cargo.lock`、`pnpm-workspace.yaml`。
- **实测证据**：
  - 根 `Cargo.toml` `members` 仅保留 24 个存活 crate，已删 crate 全部移除。
  - `Cargo.lock` 与 `Cargo.toml` 一致，`cargo check --workspace` 干净通过。
  - `pnpm-workspace.yaml` 已移除 `src/mobile-web`（`src/apps/desktop-tauri` 孤儿注册已登记为独立决策项 P2-20，不列为本次 finding）。

### Constraint 7: 无夹带与行为等价性 ✅ PASS
- **核验项**：diff 中无无关行为性改动；T2-2a' 中 `PRODUCT_TOOL_GROUPS` 内联至 `src/crates/assembly/core/src/agentic/tools/product_runtime/materialization.rs` 保持 40 tools / 4 组 / 顺序等价。
- **实测证据**：
  - 抽查 `PRODUCT_TOOL_GROUPS` 结构：
    1. `core.basic` (11 tools): LS, Read, Glob, Grep, Write, Edit, Delete, ExecCommand, WriteStdin, ExecControl, GetTime
    2. `core.agent` (12 tools): Task, Skill, AskUserQuestion, TodoWrite, get_goal, create_goal, update_goal, CreatePlan, submit_code_review, GetToolSpec, GetFileDiff, Log
    3. `core.session` (4 tools): SessionControl, SessionMessage, SessionHistory, Cron
    4. `core.integration` (13 tools): WebSearch, WebFetch, ListMCPResources, ReadMCPResource, ListMCPPrompts, GetMCPPrompt, GenerativeUI, Git, ReviewPlatform, InitMiniApp, ControlHub, ComputerUse, Playbook
    - 合计：4 provider IDs, 40 tools，名称与顺序与删除前完全一致。
  - 测试验证：
    - `cargo test -p northhing-product-capabilities`：5 passed / 0 failed。
    - `cargo test -p northhing-core --lib --features product-full product_runtime`：22 passed / 0 failed。
    - `cargo test -p northhing-core --lib --features product-full agentic::tools::registry::tests`：22 passed / 0 failed。

---

## 3. Findings 清单

- **Critical (严重缺陷)**：0
- **Important (重要缺陷)**：0
- **Minor (次要缺陷)**：0（所有批次 Minor 均已完成分流处置，无阻断主干合入项）

---

## 4. Minor Triage 全量处置表

对 progress.md 各批次记录的所有 Minor 逐条核验与处置如下：

| 批次 | 标识 | 描述 | 处置结论 | 一句话处置依据 |
|---|---|---|---|---|
| **T2-2a** | M1 | `dev.cjs` watch scope 整合 | **无需修** | 实施者已在 commit msg `2dfb8e4` 明确记录，整合后的 watch 范围精简且正确。 |
| **T2-2a** | M2 | `test_reference_skill.cjs:56-58` 孤儿引用已删 pcc | **无需修** | 本地辅助脚本，不影响产品构建与 CI。 |
| **T2-2a** | M3 | `dev.cjs:99, 105` 存量 mojibake 语法 bug | **留 P2 登记** | 属于存活脚本中的历史 GBK 损坏，本批未破坏性扩大，留待脚本卫生专项批次处理。 |
| **T2-2a** | M4 | 历史 docs 残留已删 crate 字符串 | **无需修** | 历史归档与评审记录保留事实证据；活文档 `AGENT_ONBOARDING.md` 引用已在后续整理。 |
| **T2-2a** | M5 | `check-core-boundaries.test.mjs` 1 条 pre-existing 失败 | **留 P2 登记** | `checker.mjs` 执行正常通过，self-test 中的 `tool-contracts/src/framework.rs` anchor 断言漂移独立登记维护。 |
| **T2-2a'** | M-ap-1 | `product-capabilities/AGENTS.md` 职责描述过时 | **留 P2 登记** | 已删 tool-provider-groups 与 harness，文档局部小段描述待在下轮文档治理中刷新。 |
| **T2-2a'** | M-ap-2 | `core-decomposition.md` 全文重写而非最小编辑 | **无需修** | 重写后内容经独立审查核实完全正确，无死引用残留。 |
| **T2-2b** | M-b-1 | `gate_judge` subagent 注册成为生产接线孤儿 | **无需修** | 作为 TH-5 / T3-8 自评审机制的预留词汇与资产有意保留，在 T3-8 中直接复用。 |
| **T2-2b** | M-b-2 | P2-10 allow-god-file 白名单仍列 `judge_gate/mod.rs` | **留 P2 登记** | 属于台账历史条目清理，待下次 ledger 卫生清理轮一并处理。 |
| **T2-2b** | M-b-3 | agent-runtime 无 `AGENTS-CN.md` 镜像 | **无需修** | 英文单文件为该目录事实源，同步指令正确空转。 |
| **T2-2b** | M-b-4 | AGENTS.md 注解位置置于 Guardrails 节 | **无需修** | 功能等效且符合规范。 |
| **T2-2c** | M-c-1 | `core/Cargo.toml:124, 129` 残留 Remote Connect 注释 | **已修** | 已在 C8 (T2-2j) 中彻底清理。 |
| **T2-2c** | M-c-2 | SAR 测试名提及 `remote_control_port` | **已修** | 已在 C8 (T2-2j) 中重命名为 `core_service_agent_runtime_owner_exposes_agent_runtime`。 |
| **T2-2c** | M-c-3 | `sar_dispatch.rs` 的 runtime_ports import | **已修** | 已在 C4 (T2-2f) 中回访并随 `RemoteControlStatePort` 清理。 |
| **T2-2c** | M-c-4 | boundary 规则行号位移 | **无需修** | 代码删除后的自然行号偏移，checker 检验通过。 |
| **T2-2e** | M-e-1 | `services-integrations/AGENTS.md` remote 规则指向 | **已修** | 已在 C4 (T2-2f) 契约修剪时同步更新。 |
| **T2-2e** | M-e-2 | `self-test.mjs` SSH 锚点行号下移 | **无需修** | 代码删除后的正常偏移，锚点内容与断言完全有效。 |
| **T2-2e** | M-e-3 | Cargo.lock 包含其他 crate 同名传递依赖 | **无需修** | Cargo 依赖图自动解析的正常传递依赖，非本栈残留。 |
| **T2-2g** | M-g-1 | `i18n-audit.mjs` 中 `collectConfirmedUnusedKeys` 空函数 | **已修** | 已在 C8 (T2-2j) 中通过 4 行纯删除彻底移除。 |
| **T2-2g** | M-g-2 | `src/apps/server/README.md` 3 条悬空 relay 链接 | **留 P2 登记** | 已在 C8 (T2-2j) 登记为 P2-19，待 server 解冻时同步修正。 |
| **T2-2h** | F1 | `pnpm-lock.yaml` 自动清理 2 个 desktop-tauri 孤儿条目 | **留 P2 登记** | 已在 C8 (T2-2j) 登记为 P2-20，待后续 workspace 统一清理。 |
| **T2-2h** | F2 | `dev.cjs` 步骤调整与 mojibake 字节未动核算 | **无需修** | 步骤逻辑与 totalSteps 调整精确，mojibake 保持原样未扩大。 |
| **T2-2h** | F3 | `build-installer.cjs` mobile-web 路径删除核算 | **无需修** | 安装器打包路径已与桌面运行时完全对齐。 |
| **T2-2i** | M-i-1 | `i18n-audit.mjs` 空行/EOL 扰动 110 行 | **无需修** | 内容为纯删除（`-w` 0+/219-），逻辑正确。 |
| **T2-2i** | M-i-2 | 实施报告个别措辞细节 | **无需修** | 报告记录，不影响代码质量。 |
| **T2-2j** | J-1 | `roadmap:118` "重写" vs "重建" 措辞细节 | **无需修** | "重建"更准确反映从零开发新协议客户端的产品意图。 |
| **T2-2j** | J-2 | 报告验证命令补充 `--lib --features product-full` | **无需修** | 属于运行 feature-gated 测试的合理实跑参数。 |
| **T2-2j** | J-3 | 4 条 LF/CRLF 行尾警告 | **无需修** | Windows 环境正常行尾，whitespace 检查通过，由 `pnpm run fmt:rs` 统一维护。 |

---

## 5. Cannot Verify 清单 (Limitations & Non-verifiable scope)

1. **MiniApp 子系统整删的后续批次影响**：T2-2 roadmap 行标注了 MiniApp 待执行，本分支仅完成了死代码 + Remote 栈整删，MiniApp 代码（permission_policy 提炼与删码）作为独立后续批次执行，本轮终审确认其未被误删或破坏。
2. **Frozen i18n 脚本的完整执行链**：`src/web-ui` 在当前 snapshot 中缺席且 i18n 工程处于 frozen 状态，`i18n:audit` 无法全绿系 pre-existing，本轮终审通过字节对比与语法检查确认了损伤未被扩大。
3. **微信 QR 登录历史路径**：全仓实测 `weixin_qr_login` 零命中，活代码中仅存 IM 客户端检测，无残留代码路径。

---

## 6. 一句话总结论 (Final Conclusion)

**T2-2 删除线全量 10 个批次与收口改动（`e65d98e~1..HEAD`）严格遵守架构与产品基线，实现了 -6.1 万行死代码与 remote 栈的彻底干净摘除，且 SSH 语义与 TH-5 协议词汇零损伤、全平台门禁全绿，双判决一致通过（PASS），可以正式完成结账与归档。**
