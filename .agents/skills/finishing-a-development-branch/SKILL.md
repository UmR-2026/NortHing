---
name: finishing-a-development-branch
description: "Use when a v3 development branch's tasks are complete. Verifies tests, runs final checks, presents merge/keep/discard options, and cleans up the worktree. Trigger this when all tasks in a plan are done, or when you decide to stop working on a branch."
---

# Finishing a Development Branch (northhing v3 适配— > 来源：obra/superpowers finishing-a-development-branch，Cargo workspace + Rust 适配

## When to trigger

- 计划中的所— task 都完— 你决定停止当前分支的工作（切换优先级、放弃方向）
- subagent-driven-development 完成 final task
- **不要跳过这个步骤**— worktree 堆积是真实的管理负担

---

## Step 1: 最终验— (Final Verification)

### 必须全部通过

```bash
cd E:/agent-project/northhing-v3

# 1. 格式检— cargo fmt --check

# 2. Clippy（workspace 全量，warnings as errors— cargo clippy --workspace -- -D warnings

# 3. 全量测试
cargo test --workspace --lib

# 4. CLI 编译（如果改动涉— CLI— cargo build -p northhing-cli

# 5. const flag 完整性（所— flag 默认 false 时行为不变）
# 手动检查：所— const FLAG: bool = true 改为 false，cargo build 应该通过
```

### 如果任何一步失— 退回修复，不要进入 Step 2。用 `systematic-debugging` skill 排查— ### ⚠️ 不要构建 northhing-desktop

`northhing-desktop` (Tauri) build 会因— `mobile-web/dist` 缺失而失败。这是已知问题（P2-TBD），不阻塞分支完成— ## Step 2: 检查分支状— ```bash
# 有没有未提交的改动？
git status

# 有多— commit— git log --oneline main..HEAD

# diff 统计
git diff main...HEAD --stat
```

---

## Step 3: 更新项目文档

在结束分支前，确保以下文档是最新的— ### PROJECT_STATE.md
- 添加新的 phase 章节（如果完成了 v3.x phase— 更新 token 节省总计
- 标记完成日期

### CODE_REVIEW.md
- 如果发现/修复— bug，更新对应条目的状— 如果发现新的 P 级问题，添加条目

### HANDOFF.md
- 更新 "现在 v3-restructure 分支 N — commit"
- 更新 "接下来该做的" 任务列表
- 如果有新— const flag，更— "紧— Rollback" 章节

---

## Step 4: 选择处置方案

基于分支状态，选择一个：

### Option A: 合并— main（本— merge— ```
何时选择— 分支工作完成且验证通过
- 准备— v3 改动合入主线
- 坐标：E:\agent-project\northhing (main 分支)
```

```bash
cd E:/agent-project/northhing
git merge v3-restructure --no-ff -m "merge: v3.x <description>

- <key changes summary>
- <rollback instructions>"
```

### Option B: 保留分支（继续开发）

```
何时选择— 还有后续 task 要做
- 等待 review 或其他依— 不急于合并
```

不做任何操作。记录当前状态在 HANDOFF.md 中— ### Option C: 创建 PR（如果配置了 remote— ```
何时选择— 需要他— review
- — remote 配置

注意：northhing v3 目前没有 remote，此选项通常不可用— ```

### Option D: 丢弃分支（实验失败）

```
何时选择— 方向走错— 实验性探索，不打算保— ```

```bash
cd E:/agent-project/northhing
git worktree remove E:/agent-project/northhing-v3
git branch -D v3-restructure
# ⚠️ 这会丢失所有未合并的工作！
```

---

## Step 5: 清理 worktree（如果合并或丢弃— ### 如果选择— A (merge) — D (discard)

```bash
cd E:/agent-project/northhing

# 列出所— worktree
git worktree list

# 移除已完成的 worktree（只移除自己创建的）
git worktree remove E:/agent-project/northhing-v3

# 清理 worktree 残留
git worktree prune

# 验证
git worktree list
```

### 如果选择— B (keep)

确保 worktree 状态干净— ```bash
cd E:/agent-project/northhing-v3
git status # should show "working tree clean"
```

---

## v3 特定：const flag 完整性检— 在合并前，做一— const flag 全局审计— ```bash
# 列出所— v3 const flags
grep -r "const.*bool.*=.*v3\|const.*USE_\|const.*DISABLE_\|const.*DROP_\|const.*COLLAPSE_\|const.*INCLUDE_" --include="*.rs" src/

# 确认每个 flag 都有— # 1. 注释说明用途和 rollback 方式
# 2. Regression test
# 3. PROJECT_STATE.md 中有记录
```

---

## 完成清单

结束分支前，过一遍这个清单：

- [ ] `cargo fmt --check` 通过
- [ ] `cargo clippy --workspace -- -D warnings` 通过
- [ ] `cargo test --workspace --lib` 通过— 21+ tests— [ ] `cargo build -p northhing-cli` 通过
- [ ] PROJECT_STATE.md 已更— [ ] HANDOFF.md 已更新（如果保留分支— [ ] CODE_REVIEW.md 已更新（如果有新发现— [ ] 所— const flag 有对应测— [ ] 所— const flag 有注释说— rollback 方法
- [ ] 分支处置决策已执行（merge/keep/discard— ## 与其— skill 的关— **using-git-worktrees**: 创建 worktree — 工作 — **— skill 完成并清— *
- **verification-before-completion**: Step 1 的验证使用此 skill
- **code-review**: 合并前建议触— code review
- **subagent-driven-development**: 最后一— task 完成后触发本 skill
