# Review Package — W15-1k（rot 闸红修复：两处 god-file 纯位移瘦身）

- 分支：`main`，BASE `4f2a564` → HEAD `20425b4`（2 commits：`75b9a11` core 缝迁移 + `20425b4` desktop spawner 迁移）
- diff：`git diff 4f2a564..20425b4`，补丁 = `.superpowers/sdd/packages/w15-1k-diff.patch`（6 文件，+215/-209）
- brief：`.superpowers/sdd/w15-1k-brief.md`
- report：`.superpowers/sdd/reports/w15-1k-report.md`

## 任务一句话

W15-1i/1h-fix 把 app.rs（847>800）和 memory_db.rs（920>ceiling 894）推过 rot 闸，CI run 33872662968 rot check 红。本单纯位移瘦身：app.rs 的 `spawn_module_window*`（~123 行，含 I2/T7 证据注释）→ `window_ops.rs`；memory_db.rs 的 test-only 隔离缝（~60 行）→ 新建 `test_seam.rs`；ceiling 894→859 下调。

## 验收标准（逐条判 PASS/FAIL，对应 brief §1）

1. app.rs ≤800 行（report 声称 721）。
2. memory_db.rs ≤894（report 声称 849）。
3. `pnpm run check:rot` 绿（report 声称 12/12）。
4. `cargo check --workspace` 绿 + `cargo test -p northhing-core --features product-full memory_db` 绿（缝消费方 facts/auto_memory/continuity_selfcheck 不断）。
5. 零行为变化：纯搬移 + import/mod 调整 + ceiling 下调；I2/T7 证据注释与缝设计注释随块完整保留。

## Global Constraints（逐字）

- rot-budget.json 只允许下调 memory_db.rs ceiling（已核 diff：894→859 + note，其它条目未动——judge 复核）。
- 界外零触碰；不重构、不顺手清理。

## 重点质询（供 skeptical 校准，不是预判结论）

- 逐行比对搬移块：spawner 两函数（原 app.rs:654-776）与缝段（原 memory_db.rs 尾部）搬迁前后是否逐字一致（除 import/mod 声明）。
- `app.rs` 用 `pub use super::window_ops::{...}` 保持兼容——确认该 re-export 没有意外扩大可见性（原 spawn_module_window 是 `pub`，在 app 模块内被 pub use 转发的语义是否等价）。
- `test_seam.rs` 全文件 `#[cfg(test)]`，确认 release 编译不携带（cargo check 绿只证编译过，judge 判断 cfg 挂接是否正确）。
- ceiling 859 = 849+10 是否符合 brief 授权（实际+10）。

## 背景（非判据）

- CI 红证据：run 33872662968 rot job 日志（两条违规原文在 brief §1）。
