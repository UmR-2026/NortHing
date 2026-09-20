# W23-1 任务报告 — 文档/注释诚实化批

## 改动摘要

本单为纯文档/注释诚实化任务，严格限定在允许文件集（4 文件），无任何行为变更与非注释代码改动：

1. **AGENTS.md & AGENTS-CN.md（分层表 + 验证表修正）**
   - **分层表补齐第 7 行 support 层**：`src/crates/support`，职责为测试支撑与内部工具（豁免生产分层约束的辅助 crate），模块包含 `test-support`、`cli-internal`，层文档标明无就近 `AGENTS.md`。
   - **模块列表补齐遗漏 crate**：contracts 层补充 `kernel-api` 与 `disposable`；services 层补充 `debug-log`；确认 interfaces 层 `acp` 已在列。
   - **分层表下方增加权威对照源声明**：「机械对照：以根 `Cargo.toml` workspace members + `scripts/core-boundaries/rules/crate-layout.mjs` 为准。」
   - **验证表定向测试指引修正**：保留原有的 `cargo check --workspace, plus the nearest focused cargo test when behavior changed` 表述，并在同单元格末尾追加注记：`（northhing-core 定向测试须带 --features product-full，裸跑 E0433）`。

2. **docs/status/tech-debt-ledger.md（新增 P2-25 条目）**
   - 在 P2-24 之后追加 P2-25 债务条目，固定采用五字段结构（`- **Symptom**:` / `- **Mitigation**:` / `- **Evidence**:` / `- **Proposed fix**:` / `- **Status**:`）。
   - Mitigation 插在 Symptom 之后，记录管线层现有确认流的缓解事实；Evidence 精确标注函数本体 `:225-248`、`"allow-stub"` `:244` 及 Phase 3 注释 `:238-239`。保持 Change Protocol 段不变。

3. **src/crates/support/cli-internal/src/main.rs（注释诚实化）**
   - 仅修改顶部模块文档 SECURITY 说明，如实表述 capability token gate 目前仅为 format-only 长度校验（length ≥ 32），加密校验 deferred。零代码行变更。

---

## 验证

### 1. 代码库卫生检查
- **命令**：`node scripts/check-repo-hygiene.mjs`
- **输出**：
  ```text
  Repository hygiene check passed (1 content files scanned, 3895 filenames checked).
  ```
- **Exit Code**：0

### 2. Rust 编译检查
- **命令**：`<RUSTUP> run stable-x86_64-pc-windows-msvc cargo check -p northhing-cli-internal`
- **输出**：
  ```text
      Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.45s
  ```
- **Exit Code**：0
- **编译错误层级说明**：零编译错误（本单全为文档/注释变更，无语法或类型问题）。

### 3. 分层表与 Cargo.toml 25 个 Workspace Members 对账说明

对照根 `Cargo.toml` 的 `workspace.members` 25 项与 `scripts/core-boundaries/rules/crate-layout.mjs`：

| # | Workspace Member Path | Crate Name | 分层表归属层级 | 表中对应条目 |
|---|---|---|---|---|
| 1 | `src/apps/cli` | `northhing-cli` | Layer 1: Interfaces | `CLI` |
| 2 | `src/apps/desktop` | `northhing` | Layer 1: Interfaces | `desktop` |
| 3 | `src/apps/server` | `northhing-server` | Layer 1: Interfaces | `server` |
| 4 | `src/crates/interfaces/acp` | `northhing-acp` | Layer 1: Interfaces | `acp` |
| 5 | `src/crates/assembly/core` | `northhing-core` | Layer 2: Assembly | `core` |
| 6 | `src/crates/assembly/product-capabilities` | `northhing-product-capabilities` | Layer 2: Assembly | `product-capabilities` |
| 7 | `src/crates/adapters/ai-adapters` | `northhing-ai-adapters` | Layer 3: Adapters | `ai-adapters` |
| 8 | `src/crates/services/services-core` | `northhing-services-core` | Layer 4: Services | `services-core` |
| 9 | `src/crates/services/services-integrations` | `northhing-services-integrations` | Layer 4: Services | `services-integrations` |
| 10 | `src/crates/services/terminal` | `northhing-terminal` | Layer 4: Services | `terminal` |
| 11 | `src/crates/services/debug-log` | `northhing-debug-log` | Layer 4: Services | `debug-log` |
| 12 | `src/crates/execution/agent-dispatch` | `northhing-agent-dispatch` | Layer 5: Execution | `agent-dispatch` |
| 13 | `src/crates/execution/agent-runtime` | `northhing-agent-runtime` | Layer 5: Execution | `agent-runtime` |
| 14 | `src/crates/execution/agent-stream` | `northhing-agent-stream` | Layer 5: Execution | `agent-stream` |
| 15 | `src/crates/execution/tool-contracts` | `northhing-tool-contracts` | Layer 5: Execution | `tool-contracts` |
| 16 | `src/crates/execution/runtime-services` | `northhing-runtime-services` | Layer 5: Execution | `runtime-services` |
| 17 | `src/crates/execution/tool-execution` | `northhing-tool-execution` | Layer 5: Execution | `tool-execution` |
| 18 | `src/crates/contracts/core-types` | `northhing-core-types` | Layer 6: Contracts | `core-types` |
| 19 | `src/crates/contracts/events` | `northhing-events` | Layer 6: Contracts | `events` |
| 20 | `src/crates/contracts/kernel-api` | `northhing-kernel-api` | Layer 6: Contracts | `kernel-api` |
| 21 | `src/crates/contracts/runtime-ports` | `northhing-runtime-ports` | Layer 6: Contracts | `runtime-ports` |
| 22 | `src/crates/contracts/disposable` | `northhing-disposable` | Layer 6: Contracts | `disposable` |
| 23 | `src/crates/contracts/product-domains` | `northhing-product-domains` | Layer 6: Contracts | `product-domains` |
| 24 | `src/crates/support/test-support` | `northhing-test-support` | Layer 7: Support | `test-support` |
| 25 | `src/crates/support/cli-internal` | `northhing-cli-internal` | Layer 7: Support | `cli-internal` |

注：Layer 1 中还包含非 workspace crate 的宿主/测试条目（`installer`、`E2E`）。所有 25 个 workspace members 均 1:1 精确对应，对账完成。

### 4. P2-25 条目原文

```markdown
### P2-25: shell_safety.rs 命令执行确认闸为 Phase 2 stub（Phase 3 未接线）

- **Symptom**: `src/crates/assembly/core/src/agentic/tools/implementations/shell_safety.rs` 的 `guard_command_execution` 确认段为 Phase 2 stub，仅记录 `"allow-stub"` 审计日志，Phase 3 未接线，未实际执行用户确认拦截。
- **Mitigation**: 管线层真实确认流在位（tool_confirmation.rs / exec_retry.rs + AND 语义 process_result.rs:225-234 + 默认需确认），实际风险低但 stub 必须入账。
- **Evidence**: `src/crates/assembly/core/src/agentic/tools/implementations/shell_safety.rs` 函数本体 `:225-248`、`"allow-stub"` 审计 `:244`、Phase 3 未接线自曝注释 `:238-239`。
- **Proposed fix**: Phase 3 接 `request_user_confirmation`。
- **Status**: active
```

---

## 状态

DONE
