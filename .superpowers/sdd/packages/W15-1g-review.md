# Review Package — W15-1g（list_workspace_tree 符号链接检查顺序修复）

- 分支：`fix/w15-1g-symlink-fence-order`，BASE `6cbebbb` → HEAD `ea2882c`
- diff：`git diff 6cbebbb..ea2882c`（仅 `src/crates/assembly/core/src/kernel_facade/platform.rs` + 报告；工作树另有 5 个被取消任务留下的插桩 WIP，**不在本分支 diff 内**，审查时忽略）
- brief：`.superpowers/sdd/briefs/W15-1g-symlink-fence-order.md`
- report：`.superpowers/sdd/reports/W15-1g-report.md`

## 任务一句话
CI windows 双 job 红于 `list_tree_skips_symlink_to_outside_target`：围栏检查 `is_within`（canonicalize 跟随符号链接）在符号链接跳过逻辑之前开火。修复 = 顺序对调。安全敏感路径：符号链接必须仍然既不被列出也不被跟随。

## 验收标准（逐条判 PASS/FAIL）
1. 修复后 `cargo test -p northhing-core w9_6`（实现者加 `--features product-full`，因 default 基线预存编译缺口——属已知 P2-15 形态 follow-up）本地绿。
2. CI 转绿由编排者推分支后观测，不在审查范围。
3. diff 只触及允许文件集（platform.rs + report）。
4. Spec：`metadata failed` 错误语义保留；`is_within`/`resolve_within_workspace`/`pick_workspace_root` 零改动；测试文件零改动。

## Global Constraints（逐字）
- 只允许调整检查顺序，禁止削弱围栏（符号链接既不列出也不跟随）。
- 不 push（编排者执行）。
