# 审计报告 R4 — 工作树 / 分支 / 整合状态核查

**审计时间**：2026-08-31  
**基线版本**：main 分支 HEAD `f5dc0ef`  
**执行环境**：只读检查（无 cargo/pnpm 锁冲突，无 git 写操作，无文件删除）

---

## 1. 核心结论摘要

1. **`.worktrees/consult-room-build`（`feat/consult-room-slint`，HEAD `969d274`）**：
   - **完全被 main 包含**：`main..969d274` 为 **0 个 commit**，`969d274..main` 为 146 个 commit，merge-base 就是 `969d274` 本身。
   - **零未提交源码修改**：`git status --porcelain` 中 ` M`/`A`/`D` 为 **0**，全部 132 个未跟踪条目均为 SDD 文档（129 个）、临时 skill 文档（1 个）、截图（1 个）和大小写重复目录（1 个），且均已在 `C:\WINDOWS\TEMP\opencode\worktree-backup-2026-08-31\` 完成全量备份。
   - **处置建议**：**可直接移除 worktree 并删除分支 ref**，可立即释放 **426.81 MB** 磁盘。

2. **已删 worktree 的 7 个分支资产盘点**：
   - `feat/growth-a1` ~ `a5`（各 2 个 commit）：**100% 被 `feat/growth-core-0804` 线性吸收**，建议直接删除分支 ref。
   - `feat/growth-core-0804`（**36 个 commit**）：包含完整的 `northhing-agentic-growth` crate（+11,081 行，185 测试全绿，双 PASS 终审）。虽然与 main 分叉达 391 提交（1906 个改动文件），但属于高价值完整设计实现，**强烈建议保留分支 ref**，待后续需要时按专题移植。
   - `spike/multiwindow-0809`（3 个 commit）：为 Dioxus 多窗口实验代码与截图，结论已沉淀至架构文档，分支 ref **可直接删除**。

3. **前后端划分与调用合规性**：
   - 前端指 **Dioxus 桌面 UI**（`src/apps/desktop/src/ui_dioxus/`），后端指 **Rust Core**（`src/crates/assembly/core` 及底层 crates）。
   - **违规直调数：3 处**（均直接调用了 `northhing_core::service::config::initialize_global_config().await` 绕过了 `kernel_facade`），另有 1 处在组件内内联调用 facade 而非封装于 `ui_dioxus/api.rs`。

4. **磁盘可回收量**：
   - `.worktrees/consult-room-build`：**426.81 MB**（0.42 GB）
   - `target/` 目录：主工作树 **128.58 GB** + installer **4.57 GB** = **133.15 GB**

---

## 2. 表1 整合矩阵（分支与 Worktree）

| 分支 / Worktree | HEAD SHA | 分叉基线 (merge-base) | 独有 commit 数 | main 领先数 | 与 main 重叠 / 状态 | 资产价值 | 处置建议 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`.worktrees/consult-room-build`** (`feat/consult-room-slint`) | `969d274` | `969d274` (2026-08-24) | **0** | 146 | 已全部合并回 main，HEAD 是 main 的直系祖先 | 低（均已在 main 上，未提交项已备份） | **移除 worktree 并删除分支** |
| `feat/growth-a1` | `32192c2` | `f2a16c7` (2026-08-04) | 2 | 391 | 100% 被 `growth-core-0804` 包含 (`5eb5fbf`) | 无（完全冗余） | **删除分支 ref** |
| `feat/growth-a2` | `2816b47` | `f2a16c7` (2026-08-04) | 2 | 391 | 100% 被 `growth-core-0804` 包含 (`c9dcb58`) | 无（完全冗余） | **删除分支 ref** |
| `feat/growth-a3` | `2de4186` | `f2a16c7` (2026-08-04) | 2 | 391 | 100% 被 `growth-core-0804` 包含 (`7e3e279`) | 无（完全冗余） | **删除分支 ref** |
| `feat/growth-a4` | `414d822` | `f2a16c7` (2026-08-04) | 2 | 391 | 100% 被 `growth-core-0804` 包含 (`6294760`) | 无（完全冗余） | **删除分支 ref** |
| `feat/growth-a5` | `07b986f` | `f2a16c7` (2026-08-04) | 2 | 391 | 100% 被 `growth-core-0804` 包含 (`1488a0d`) | 无（完全冗余） | **删除分支 ref** |
| `feat/growth-core-0804` | `1e1f009` | `f2a16c7` (2026-08-04) | **36** | 391 | main 上无 `src/agentic`，主干推进了 SQLite 重构导致部分函数有冲突 | **极高**（完整 Agentic Growth 引擎） | **保留分支 ref**（勿删） |
| `spike/multiwindow-0809` | `feee4e2` | `8566ef3` (2026-08-09) | 3 | 383 | Spike 实验产物，结论已沉淀至架构文档 | 低（原型示例与测试截图） | **删除分支 ref** |

---

## 3. 详细分析：各分支与 Worktree

### 3.1 `.worktrees/consult-room-build`
- **提交统计**：
  - `git rev-list --count main..969d274` = `0`
  - `git rev-list --count 969d274..main` = `146`
  - `git merge-base main 969d274` = `969d274f699c82012eb9ed7bade8fd43e1634e3a`
- **工作树状态**：
  - `git status --porcelain` 跟踪修改（` M`, `A `, `D `）：**0 行**。
  - 未跟踪文件（`??`）：共 **132 项**。
    - 129 项：`.superpowers/sdd/consult-room/` 下的 task briefs、reports、reviews 及 commit-msg 记录。
    - 1 项：`.agents/skills/northhing-dioxus-frontend/` 技能文档目录。
    - 1 项：`docs/design/2026-07-22-frontend-redesign/consult-room/build-shots/` 截图目录。
    - 1 项：`northhing-Installer/`（因 Windows 大小写产生的残留目录）。
  - 所有未跟踪文件已在备份目录 `C:\WINDOWS\TEMP\opencode\worktree-backup-2026-08-31\` 完整保存（19.81 MB）。
- **结论**：**无任何未合并提交，无任何源码修改丢失风险，可安全删除**。

### 3.2 `feat/growth-core-0804`（36 个 Commit 深度盘点）
- **开发范围**：G1~G2 阶段（T1 至 T9），包含：
  1. `src/agentic`（`northhing-agentic-growth` crate）：
     - `ports.rs`（Growth ports 定义）
     - `state.rs`（持久化状态与状态机）
     - `topics/extract.rs`（无外部依赖的主题抽取）
     - `topics/score.rs`（两层检索评分与主题主导度）
     - `topics/competition.rs`（竞争组归一化与自然抑制）
     - `negation.rs`（保守显式否定检测）
     - `distill/parse.rs` & `prompt.rs`（蒸馏提示词与解析纯逻辑）
     - `review/propose.rs` & `route.rs` & `verdict.rs`（竞争评审提议、规划与裁决）
     - `scheduler.rs`（轮次调度纯决策）
  2. `src/crates/assembly/core`：
     - `growth_adapter.rs`（适配器实现与 34 个测试）
     - `self_cognition.rs`（自我认知存储与迁移）
     - `competition_review.rs`（定期竞争关系评审 sweep 与 10 个测试）
     - `memory_db/competition_groups.rs`（SQLite 竞争组持久化）
- **质量状态**：
  - `northhing-agentic-growth` 单测：185 passed, 0 failed
  - `competition_review` 测试：10 passed, 0 failed
  - G2-T9 终审：`task-g2-t9-review-round3.md` 给出 SPEC PASS / QUALITY PASS
- **与 main 冲突与合并成本**：
  - 分叉于 2026-08-04（`f2a16c7`），main 自此后演进了 391 commits、1906 个文件。
  - main 在 PHASE-1B 对 `turn_persist.rs` 和 `facts.rs` 进行了 SQLite 改造，直接 `git merge` 会发生冲突（合并难度评级：**L / 大**）。
  - **建议**：保留分支 ref，未来若接入 Agentic Growth 采用 SDD 计划按模块挑单重放。

---

## 4. 表2 Surface 状态矩阵（前端与后端 Surfaces）

| Surface | 路径 | 构建 / 运行方式 | 状态 | 最近改动 / 说明 |
| :--- | :--- | :--- | :--- | :--- |
| **Dioxus 桌面 UI (当前唯一前端)** | `src/apps/desktop` | `cargo run -p northhing`<br>`pnpm run desktop:dev` | ✅ Active (Shipping) | 2026-08-28 物理删除 Slint 壳，Dioxus consult-room 成为唯一桌面前端 |
| **Installer** | `northing-installer/` | `pnpm run installer:build`<br>Tauri 1 + Svelte (rlib) | ✅ Active (Shipping) | `embed-resource` 锁定 3.0.5，独立安装器 |
| **CLI** | `src/apps/cli` | `pnpm run cli:dev`<br>`cargo run -p northhing-cli` | 🧊 Frozen | 可编译，无独立发布构建，doctor 命令存在假阳性 |
| **Server** | `src/apps/server` | `cargo check -p northhing-server` | 🧊 Frozen | Axum HTTP 接口层，无鉴权，未部署 |
| **ACP Interface** | `src/crates/interfaces/acp` | Cargo 依赖编译 | ✅ Active | Layer 1 协议接口 crate |
| **E2E Tests** | `tests/e2e` | `cargo test -p northhing-e2e` | ✅ Active | 跨 Surface 集成测试工程 |
| **Web UI** | `src/web-ui` | `pnpm run dev:web` (不可用) | ❌ Missing | 仅存在 i18n 生成合约与空目录，React 代码在 v0.1.0 快照中缺失 |
| **Tauri Desktop** | `src/apps/desktop-tauri` | 无 | ❌ Deleted | 目录不存在（已移出工作区） |

---

## 5. 表3 前端→后端调用合规性（`ui_dioxus` 直调 Core 审计）

依据架构规范，前端（`ui_dioxus`）与后端分界应严格经由 `northhing_kernel_api` DTO 与 `northhing_core::kernel_facade::kernel_facade()`。

### 5.1 违规直接调用清单（绕过 Facade）

| 序号 | 文件与行号 | 违规代码 | 违规原因与影响 | 整改建议 |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:176` | `northhing_core::service::config::initialize_global_config().await;` | 绕过 `kernel_facade` 直接调用 core 内部 service | 移入 `KernelSettingsApi` / facade 方法 |
| 2 | `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs:293` | `northhing_core::service::config::initialize_global_config().await;` | 绕过 `kernel_facade` 直接调用 core 内部 service | 移入 `KernelSettingsApi` / facade 方法 |
| 3 | `src/apps/desktop/src/ui_dioxus/api_settings.rs:254` | `northhing_core::service::config::initialize_global_config().await;` | 绕过 `kernel_facade` 直接调用 core 内部 service | 移入 `KernelSettingsApi` / facade 方法 |

### 5.2 边界规范说明与其余引用分析
- **组件内内联 facade 调用**：
  - `src/apps/desktop/src/ui_dioxus/pages_onboarding.rs:701`：`northhing_core::kernel_facade::kernel_facade().create_session(...)`（虽走 facade，但未统一走 `ui_dioxus/api.rs` 封装层）。
- **合法 Facade 引入**：
  - `api_events.rs:9`、`api_fs.rs:16`、`api_memory.rs:6`、`api_provider_edit.rs:6`、`api_settings.rs:6`、`api.rs:8` 均标准引用 `northhing_core::kernel_facade::kernel_facade`。
- **Slint 残留排查**：
  - 所有 `Cargo.toml` 均**无** `slint` 依赖。
  - 生产 Rust 代码无任何 active slint 语法 / import（仅 7 处注释提及历史对比）。

---

## 6. 磁盘占用与回收建议

1. **当前磁盘占用实测**：
   - 主工作树 `target/`：**128.58 GB** (131,667.44 MB)
   - `northing-installer/src-tauri/target/`：**4.57 GB** (4,674.96 MB)
   - `.worktrees/consult-room-build`：**426.81 MB** (0.42 GB)
   - 主工作树 `??` 未跟踪文件：仅 4 个审计 brief md 文件（无体积）。
2. **可回收总量**：
   - 立即清理 worktree：可回收 **426.81 MB**。
   - 编排者在适当时机运行 `cargo clean`：可释放 **~133.15 GB**。

---

## 7. 处置建议清单（供编排者执行）

1. **执行删除**：
   - `git worktree remove .worktrees/consult-room-build`
   - `git branch -D feat/consult-room-slint`
   - `git branch -D feat/growth-a1 feat/growth-a2 feat/growth-a3 feat/growth-a4 feat/growth-a5`
   - `git branch -D spike/multiwindow-0809`
2. **保留资产**：
   - **保留分支 `feat/growth-core-0804`**（不可删除）。
3. **技术债待办**：
   - 在后续轮次中将 `ui_dioxus` 的 3 处 `initialize_global_config` 收口至 `kernel_facade`。
