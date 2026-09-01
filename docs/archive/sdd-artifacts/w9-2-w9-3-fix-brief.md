# W9-2/W9-3 修复单（追溯审查 findings 全清单）

仓库：E:\agent-project\NortHing（main）。判决书：`.superpowers/sdd/w9-2-w9-3-retro-review.md`（先读）。

## 修复清单（逐项对应判决）

1. **C-1（Critical）**：`app.rs` submit_turn Err 臂仅 match `KernelError::Runtime` → 非 Runtime 变体携带配额/余额消息时降级横幅静默绕过。修：Err 臂对**所有**变体做 `classify_ai_error_message` 分类（取错误消息文本的现有方式保持），命中 ProviderQuota/ProviderBilling 即设 degraded。附测试（构造非 Runtime 变体路径若可测，不可测则 report 说明）。
2. **I-1**：两臂文案统一（Failed 臂与 submit_turn Err 臂的 quota/billing 文案逐字一致）。
3. **I-2（rot）**：css.rs 831→≤830。优先：新 `.degraded-banner` 规则与既有同族规则合并 selector 或压缩；禁止删既有样式。
4. **I-4（rot）**：`pages_memory.rs:203` 的 `duration_since(UNIX_EPOCH)` 内联 → 换 canonical `northhing_core_types::time` helper（读该模块实际 API 再改，禁止自创）。
5. **I-3（god-file 门卫）**：app.rs 825 >800 未登记。修 = 抽离减负回 <800：本波新增的会话允许集逻辑 / degraded 横幅逻辑是天然候选（抽到 approval_card.rs 或新小模块），纯位移。若做不到 <800 → STOP BLOCKED（登记 ceiling 需用户拍板，不在你权限内）。
6. **M-1**：`TurnStateKind::Cancelled` 也清除 degraded 横幅（与 Completed 一致）。
7. **M-2（顺手）**：pages_memory.rs FactDto→FactItem 三处映射重复 → 单 helper。

## 纪律

- 只动 `src/apps/desktop` + 必要时 `src/crates/contracts/kernel-api`（不允许）——**本单只动 desktop crate**；contracts 零改动。
- 验证（全跑，输出原文进 report）：`& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`（0 error，warnings ≤47 现状基线）+ `test -p northhing --lib` 全绿 + `node scripts/verify-rot-budget.mjs` **必须转绿**（三违规全消）。
- 恰好一个 commit（消息对齐风格）；禁止整树 git 操作；禁止碰 `.superpowers/`（report 除外，写 `.superpowers/sdd/w9-2-w9-3-fix-report.md`）。

## 返回

状态（DONE/BLOCKED）/ commit SHA / git show --stat / 验证输出尾部（含 rot 全绿输出）/ 偏离清单。
