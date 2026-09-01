# 审计单 R4 — 工作树/分支/整合状态核查（只读）

仓库：`E:\agent-project\NortHing`（main，HEAD `f5dc0ef`）。**只读**：禁止改任何文件，唯一可写 = 你的报告。

## 背景（编排者已核实的事实，直接采信）

- 2026-08-31 已删除 7 个悬挂 worktree：`feat/growth-a1`~`a5`、`feat/growth-core-0804`、`spike/multiwindow-0809`（**分支 ref 全部保留**，未删分支）。
- 现存 2 个 worktree：主工作树（main `f5dc0ef`）+ `.worktrees/consult-room-build`（`969d274`，分支 `feat/consult-room-slint`）。
- 删除前已把两个脏 worktree 的未提交内容备份到 `C:\WINDOWS\TEMP\opencode\worktree-backup-2026-08-31\`（19.81 MB）。

用户诉求原话：「前后端两个 worktree 我不知道整合情况」——**要搞清楚整合矩阵，特别是"有没有东西没并回来"**。

## 必须回答的清单

### A. 现存 worktree 与 main 的关系
1. `.worktrees/consult-room-build`（分支 `feat/consult-room-slint`，HEAD `969d274`）：
   - 相对 main 的提交差：`git rev-list --count main..969d274` 与 `git rev-list --count 969d274..main`，谁多谁少？
   - 它有没有**未合并进 main 的独有 commit**？若有，逐个列 SHA + 标题 + 日期 + 改动文件数。
   - 它的工作树状态（`git -C <path> status --porcelain`）：132 个未跟踪条目分别是什么类别（SDD 产物 / skills 文档 / 构建产物 / 真源码）？**有没有未提交的代码改动**（不是 `??` 而是 ` M`/`A`）？
   - 结论：这个 worktree 是「可以直接删」还是「里面有唯一资产必须先救出来」？
2. 主工作树：确认 clean、分支、远程同步状态（`git status -sb`）。

### B. 已删 worktree 的 7 个分支 —— 资产盘点
3. 逐个分支：`git log --oneline main..<branch>` 的完整列表 + `git diff --stat main...<branch>` 的文件级统计。
   - `feat/growth-a1`~`a5`：各 2 个 commit，内容是什么（端口定义/持久化状态/显式否定检测…）？与 TH-5 成长演化的关系？
   - `feat/growth-core-0804`：**36 个 commit**（最重），主题是什么？测试是否还绿（不要跑，看代码判断）？与现在 main 上的 growth 相关代码（若有）是否重叠？
   - `spike/multiwindow-0809`：3 个 commit，是一次 spike 的结论（报告性 commit 为主）还是实现？
4. 这些分支与 main 的**分叉点时间**、main 此后改了多少文件 —— 据此判断合并成本（S/M/L/不可合并）。
5. 明确回答：**有没有任何一个分支包含 main 上不存在的、且值得救回的产物**？如果有，点名是哪个分支的哪个 commit。

### C. 「前后端」到底指什么（概念澄清）
6. 仓库里到底有几个 surface？逐个列出路径 + 构建方式 + 当前状态（存在/缺失/frozen）：
   `src/apps/desktop`（Dioxus）、`src/apps/cli`、`src/apps/server`、`northing-installer`、`src/crates/interfaces/acp`、`tests/e2e`、`src/web-ui`（AGENTS.md 称 missing，核实）。
7. 前端指的是哪个？（Dioxus 桌面 UI 是本机的"前端"）它与后端（Rust core）的分界在哪一层（`ui_dioxus/api.rs` → `kernel_facade`）？**有没有前端绕过 facade 直接碰后端的违规调用**（grep `northhing_core::` 在 `ui_dioxus` 里的直接引用，列出非 facade 的）？
8. 已删除的 Slint 壳（2026-08-28）有没有残留引用？grep `slint` 在 Cargo.toml / 源码 / 文档里的残留（排除 `.agents/reference/` 与归档目录）。

### D. 磁盘与残留
9. `target/` 目录：主工作树与各 worktree 的 target 大小（若 worktree 已删则跳过），总计。
10. 全仓未跟踪文件清单（`git status --porcelain` 的 `??`），分类：临时/产物/真源码。
11. `.worktrees/` 目录现在占用多少。

## 输出格式

- **表1 整合矩阵**：每个分支/worktree × （独有 commit 数 / 是否与 main 重叠 / 资产价值 / 建议：合并 / 保留 ref / 可删）。
- **表2 surface 状态**：路径 / 构建方式 / 状态 / 最近改动。
- **表3 前端→后端调用合规性**：违规直调清单（file:line）。
- 明确结论三段：① `consult-room-build` 怎么处理 ② 7 个分支怎么处理 ③ 磁盘能再回收多少。

## 纪律

- **禁止运行 cargo/pnpm**。
- **禁止任何 git 写操作**（add/commit/checkout/restore/branch -D/rebase/clean）。只用只读子命令：log/show/status/diff/rev-list/merge-base/ls-files/branch/remote/worktree。
- 禁止删除任何东西 —— 你只给建议，处置由编排者执行。
- **禁止编造**：所有数字来自实际命令输出。
- 报告中文，英文标识符原样。
