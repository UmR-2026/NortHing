# Probe 2 Brief — crate 准入守卫回归测试补强（中档集成单）

## 来源与验收标准

来源：PHASE-2 重审 round-2 的 Minor finding（judge-m3）：新增的"前缀相似拦截"测试 `src/apps/desktop-unregistered` 场景**实际上抓不住旧的 `\b` bug**——旧 bug 的触发条件是 surfaces.md 里有前缀相似路径但没有精确 member（如 surfaces 含 `src/apps/desktop-tauri` 而 member 是 `src/apps/desktop`）。现测试两侧都能正确拒绝，回归捕捉力弱。

**验收**：新增一个能区分新旧正则的回归测试 + 既有测试全绿。

## 任务

在 `scripts/core-boundaries/` 的测试体系（`self-test.mjs` / `check-core-boundaries.test.mjs`，读现状选合适挂载点）新增一个测试：

- **场景构造**：fixture 的 surfaces 内容只含 `src/apps/desktop-tauri`（不含 `src/apps/desktop`），workspace member 含 `src/apps/desktop`。
- **断言**：新正则下该 member 判未登记（红）；并注明——用旧 `\b` 正则时同 fixture 会误判为已登记（绿），即"这个测试在旧实现下会通过、新实现下才正确判红"——等等，注意方向：crate 准入守卫是"member 必须在 surfaces 有行"。旧 bug 是**假阳性命中**（desktop-tauri 误匹配 desktop 行）；本场景下 member=desktop 而 surfaces 只有 desktop-tauri 行——旧 `\bsrc/apps/desktop\b` 对文本 `src/apps/desktop-tauri` 会匹配（假阳性 → 放行 → 漏抓未登记），新正则必须判未登记 → 违规报出。**所以断言 = 该 fixture 下守卫必须报违规**。在旧实现下这个测试会失败（守卫放行不报），正是回归价值。
- 测试与代码注释 English-only。
- 读 `checker.mjs` 的匹配实现（PHASE-2-fix 后的 lookahead 版）与既有测试写法，保持同构。

## 约束

- 只动测试文件与 fixture 基建；**不改 checker.mjs 实现**（除非发现真 bug——那就 STOP 报 BLOCKED 附证据）。
- 不顺手清理其它测试。

## 验证（命令 + 输出进 report）

1. `node scripts/check-core-boundaries.test.mjs`（贴输出）
2. `node scripts/core-boundaries/self-test.mjs`（贴输出）
3. `node scripts/check-core-boundaries.mjs`（贴输出，主检查须仍绿）
4. `pnpm run check:rot`
5. **回归有效性实证**：把 checker.mjs 的 lookahead 临时换回 `\b` 旧式（本地改不提交），跑新测试必须红；还原后绿。贴两次输出。（这是本任务的核心证据）

## 报告

`.superpowers/sdd/task-probe2-report.md`：测试位置与挂载选择理由、验证输出（含新旧正则对照实证）、偏离声明。最后消息以 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED 开头。

## 派发元信息

- BASE `839bdd3`；worktree `E:\agent-project\.worktrees\northing-probe2`（分支 `probe/qwen38-mid-0822`）
- commit message 后缀 `(probe-2)`；只 stage 你改的文件。
