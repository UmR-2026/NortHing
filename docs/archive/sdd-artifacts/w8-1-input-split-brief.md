# Task Brief — W8-1: input.rs 拆分（零测试 god-file 的行为零变化手术）

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/cli` 仅 CLI crate。
深审报告（先读，病灶全在其中）：`.superpowers/sdd/deep-rot-app-input.md` §2。

## 风险画像（先理解再动手）

`handle_key_event`（input.rs:101-644，543 行）是**零测试覆盖**的核心事件处理函数。本任务是行为零变化的机械手术——**你不是来改进逻辑的，是来搬家的**。任何逻辑"修正"冲动都压进 report 的观察清单，不许动代码。

## Spec（验收标准）

### 1. 目录化拆分

`src/apps/cli/src/modes/chat/input.rs` → `src/apps/cli/src/modes/chat/input/` 目录模块。建议切分（可按实际代码调整，report 说明）：
- `mod.rs`：公共入口（`handle_key_event` / `handle_non_key_event` 签名不变）+ 共享类型 re-export
- `key_popups.rs`（或按拦截层命名）：5 层 popup 拦截的 helper 函数群
- `key_actions.rs`：具体 key 动作臂
- `bridge.rs` 或并入 mod.rs：async 桥接 helper

### 2. bridge 提取（消 7 处复制）

`block_in_place(|| rt_handle.block_on(async move { ... }))` 七处（L121/135/156/181/444/504/606 附近）提取为单个 helper。**逐处核对闭包捕获差异**——7 处捕获的变量不同，helper 签名必须通用（泛型 Future），不许为了复用而改任何一处的行为。

### 3. handle_key_event 拆层

按拦截层（permission → question → global popup → info → command palette → specific popup → catch-all）抽 helper 方法/函数。**match 臂逐臂纯位移**：顺序不变、条件不变、臂体逻辑不变。

### 4. 铁律

- 行为零变化：不改任何臂逻辑、不改错误处理风格（三种风格并存是既有事实，本波不统一——记 report 观察项）
- `apply_exit_reason` 8 参数问题不动（观察项）
- 架构下沉（popup dispatch trait 化）**不做**
- `chat/mod.rs:157` 的 `pub mod input;` 路径适配是唯一允许的外部触碰点

### 5. manifest 处置

`scripts/rot-budget.json` 的 `god_file:src/apps/cli/src/modes/chat/input.rs`（ceiling 802）：文件消失 → **删除该条目**，同 commit。若新子模块有 >800 行的（不应该有）→ STOP BLOCKED。

### 6. 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing-cli`：0 error
2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-cli`：既有测试全绿（若该 crate 零测试则注明）
3. `node scripts/verify-rot-budget.mjs`：绿（input.rs 条目已清，无新 >800 文件）
4. diff 自查：`git show --stat` + 逐臂核对说明写进 report（搬了哪些臂到哪，一臂一行）

## Global Constraints（逐字，源自 plan-2026-08-28-w8-godfile-rotfix.md）

1. 分层边界：改动只在 `src/apps/cli`（+ manifest 条目处置）。
2. 日志纪律：英文无 emoji；本任务零新增日志。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；**禁止 `git restore .`/`git checkout .`/`git stash` 等整树操作**，只许点名文件 add/commit。
4. rot-budget：ceiling 只降不升；manifest 仅允许清死条目。
5. 验证最小集：上述 4 条；命令+输出原文进 report（`.superpowers/sdd/w8-1-input-split-report.md`，write 工具）。
6. commit 规则：恰好一个 commit（`git mv` 语义保留历史）；不含 `.superpowers/`。
7. 不新建无 owner 抽象；bridge helper 消费方 = 7 处既有调用点。
8. 行为零变化铁律：judge 将逐臂核对位移 diff；逻辑漂移 = Critical。
9. 遇编译错误先加载对应 rust skill（m01/m03/m04 等）trace 设计层，禁止无脑 clone/unwrap 糊编译器。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；需要澄清 = NEEDS_CONTEXT。
- 返回消息含：状态 / commit SHA / git show --stat / 验证输出尾部 / 新文件行数清单 / 偏离清单。
- 假汇报 = 停用：编排者将用磁盘 diff 逐条核对。
