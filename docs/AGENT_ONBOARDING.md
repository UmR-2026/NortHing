# Agent 快速接入指南 — Northing

> **本文档目标**：让新 Agent 在 5 分钟内理解项目结构，知道从哪入手修改代码。
> **版本**：v0.1.0-rev1 (2026-06-28)
> **语言**：Rust 21-crate workspace，Slint GUI，CLI + Desktop + Server

---

## 1. 项目速览（30 秒）

```
E:/agent-project/northing/
├── Cargo.toml                 ← Workspace 根，25 个 crate 定义
├── .cargo/config.toml          ← 编译配置（sccache, target-dir, linker）
├── src/
│   ├── apps/                   ← 应用层
│   │   ├── cli/                ← 命令行入口（main.rs, chat.rs, exec.rs）
│   │   ├── desktop/            ← Slint GUI 桌面应用
│   │   ├── server/             ← WebSocket/HTTP server（空壳，待填充）
│   │   └── relay-server/       ← Relay 通信
│   ├── crates/                 ← 核心 crate
│   │   ├── assembly/core/       ← 核心编排（Coordinator, ExecutionEngine, SessionManager）
│   │   │   └── src/agentic/    ← Agent 运行时
│   │   │       ├── coordination/   ← coordinator.rs (618 行, 已拆分)
│   │   │       │   ├── dialog_turn.rs         ← 3,656 行，待二次拆分
│   │   │       │   ├── subagent_orchestrator.rs ← 1,778 行
│   │   │       │   ├── ports.rs                 ← 1,745 行
│   │   │       │   └── a1_path.rs                ← A1/A2 映射层
│   │   │       ├── execution/    ← ExecutionEngine::tick() 驱动
│   │   │       ├── session/      ← SessionManager（6,532 行 God Object，最大腐化病灶）
│   │   │       └── tools/        ← Tool 实现（ShellSafety, ImageProcessing, etc.）
│   │   ├── adapters/           ← 外部适配层
│   │   │   ├── ai-adapters/    ← 多模型 Provider（Kimi, MiniMax, OpenAI, etc.）
│   │   │   └── webdriver/      ← WebDriver 自动化
│   │   ├── execution/          ← 执行层
│   │   │   ├── agent-dispatch/   ← Actor/Dispatcher 运行时
│   │   │   ├── agent-runtime/    ← ExecutionEngine, PromptCache
│   │   │   ├── agent-stream/     ← 流式处理
│   │   │   └── tool-execution/   ← 工具执行（write_file, shell, etc.）
│   │   ├── services/           ← 服务层
│   │   │   ├── terminal/       ← 终端模拟
│   │   │   └── services-integrations/  ← SSH, 远程执行
│   │   └── contracts/          ← 类型契约
│   └── apps/...               ← 同 apps/ 目录
├── docs/                       ← 文档（handoffs, reviews, architecture, plans）
├── research/                   ← 审计报告（audit_redim*.md）
└── scripts/                    ← 脚本工具
```

---

## 2. 关键入口文件（从哪读代码）

| 你想了解... | 读这个文件 | 说明 |
|------------|----------|------|
| **Agent 如何执行一轮** | `crates/assembly/core/src/agentic/execution/execution_engine.rs` | `init_turn()` → `tick()` → `finalize_turn()` |
| **Coordinator 如何编排** | `crates/assembly/core/src/agentic/coordination/coordinator.rs` | 已拆分至 618 行，主入口是 `execute_dialog_turn()` |
| **Session 如何管理** | `crates/assembly/core/src/agentic/session/session_manager.rs` | ⚠️ 6,532 行 God Object，最急需拆分 |
| **Tool 如何定义** | `crates/execution/tool-execution/src/` | ShellSafety, write_file, image_processing 等 |
| **多模型切换** | `crates/adapters/ai-adapters/src/` | Kimi, MiniMax, OpenAI 等 Provider 适配 |
| **UI 入口** | `apps/desktop/src/app_state/mod.rs` | Slint GUI 状态管理 |
| **CLI 入口** | `apps/cli/src/main.rs` | 命令行解析 + 模式分发 |
| **Feature Flag** | `crates/execution/agent-dispatch/src/flags.rs` | `USE_LIGHTWEIGHT_ACTOR = true`（A2 已激活） |

---

## 3. 当前已知问题（读这些再做修改）

### 🔴 结构性问题（改前必读）

| 问题 | 文件 | 严重程度 | 参考文档 |
|------|------|---------|--------|
| `session_manager.rs` 6,532 行未拆分 | `agentic/session/session_manager.rs` | 🔴 P0 | `docs/handoffs/2026-06-27-r4-session-lifecycle-split-spec.md` |
| `review_platform/mod.rs` 4,866 行 | `service/review_platform/mod.rs` | 🔴 P0 | `docs/handoffs/2026-06-27-round6-review-platform-split-spec.md` |
| `dialog_turn.rs` 3,656 行 | `coordination/dialog_turn.rs` | 🟠 P1 | `docs/handoffs/2026-06-27-r4-comprehensive-cleanup-plan.md` |
| 旧 Phase 路径残留 | `coordination/coordinator.rs` | 🟠 P1 | `docs/handoffs/2026-06-28-code-rot-fix-round.md` |
| 边界泄露（CLI 穿透 core） | `apps/cli/src/` | 🟠 P1 | `research/audit_redim_v3_04.md` |

### 🟡 质量规范（所有新代码必须遵守）

- **禁止生产代码 `panic!`/`unreachable!`** → 用 `Result` 或 `warn!` + fallback
- **禁止 `let _ = Result` 静默丢弃** → 用 `if let Err(e) = ... { warn!(...); }`
- **禁止 `unwrap()` 除非编译期不变量** → 附 `// Invariant: ...` 注释
- **文件不得超过 1,000 行** → 拆分前读 `docs/code-rot-prevention-guide.md`
- **函数不得超过 50 行** → 提取子函数

**完整规范**：`docs/code-rot-prevention-guide.md`

---

## 4. 构建与测试

```bash
# 编译（Windows + MinGW）
cargo check --workspace --exclude northing-installer --exclude northing-webdriver

# 运行 CLI 聊天模式
cargo run -p northing-cli -- chat

# 测试（排除 desktop/webdriver）
cargo test --workspace --lib --exclude northing --exclude northing-webdriver

# 清理编译产物（target/debug 可能膨胀）
cargo clean
# 或仅清理增量缓存
rm -rf target/debug/incremental
```

**注意**：Windows 下 `cargo clippy` 可能不可用（msvc toolchain），用 `cargo check` 替代。

---

## 5. 如何提交修改

### 单文件修改（简单修复）

```bash
git add <file>
git commit -m "fix(module): 具体修复内容

- 改动点1
- 改动点2
Refs: docs/handoffs/2026-06-28-code-rot-fix-round.md"
```

### 多文件修改（复杂功能）

**必须**拆分为多个 commit：
1. **错误处理修复**（unwrap/unreachable/let _ =）
2. **结构拆分**（大文件拆模块）
3. **编译配置**（Cargo.toml, .cargo/config.toml）
4. **文档**（新增/修改 .md）

参考：`docs/handoffs/2026-06-28-code-rot-fix-round.md` 中的 commit 策略

---

## 6. 重要文档索引

| 文档 | 内容 | 何时读 |
|------|------|--------|
| `docs/code-rot-prevention-guide.md` | 代码腐化预防 + 修复方法 | **每次提交前** |
| `docs/handoffs/2026-06-28-code-rot-fix-round.md` | 最新修复轮记录 | 了解最近改了什么 |
| `docs/handoffs/2026-06-27-r4-comprehensive-cleanup-plan.md` | R4 全面清理计划 | 做拆分任务前 |
| `docs/architecture/core-decomposition.md` | 核心架构分解 | 理解模块边界 |
| `docs/PROJECT_STATE.md` | 项目整体状态 | 首次接入 |
| `research/audit_redim_v3_0[1-4].md` | 最新审计报告（结构/债务/质量/依赖） | 评估腐化风险 |

---

## 7. 快速检查清单（改代码前）

```markdown
□ 我的文件 < 1000 行？如果超过，先拆分再写逻辑。
□ 我没有新增 `unwrap()` 或 `panic!`？
□ 我没有 `let _ = some_result();` 静默丢弃？
□ 我添加了 `warn!` 或 `error!` 日志到错误路径？
□ 我运行了 `cargo check` 且 0 错误？
□ 我的 commit message 遵循了 `type(scope): description` 格式？
□ 我在 handoffs 目录记录了拆分/修复理由？（可选但推荐）
```

---

## 8. 联系方式

- **项目路径**：`E:/agent-project/northing/`
- **当前 HEAD**：`cc370d8` (2026-06-28)
- **目标版本**：v0.1.1（即将进入 session_manager.rs 拆分 + 旧 Phase 清理）
- **Agent 约束文件**：`docs/code-rot-prevention-guide.md`（v0.1.0-rev1）

---

*本文档由 QClaw 生成于 2026-06-28。每次重大结构变更后更新。*
