# W15-1k Review — rot 闸红修复：两处 god-file 纯位移瘦身（app.rs / memory_db.rs）

- BASE: `4f2a564` → HEAD: `20425b4`（2 commits：`75b9a11` core 缝迁移 + `20425b4` desktop spawner 迁移）
- diff stat: 6 文件, +215/-209（与 brief 允许文件集逐字对齐）
- 实施者报告: `.superpowers/sdd/reports/w15-1k-report.md`

## 0. 工作树卫生

`git status` 干净（diff 在允许文件集内；仅有 `.superpowers/sdd/{packages,reports,w15-*}*.md` 与 `screenshots/w15-1j-*.png` 未 track，皆编排产物）。

**判**：PASS

## 1. SPEC 逐条

### 1.1 `app.rs` ≤ 800 行 — **PASS**

- HEAD `app.rs` 实测：`verify-rot-budget.mjs::countLines` = **721** 行（已实测确认 PowerShell `Measure-Object -Line` 因未尾随换行返回 787，与 rot-budget verifier 的 847→721 等关键数字对齐）。
- 与 brief 红线 847→≤800 对比：原 847 → 721 = -126 行，超出门槛 47 行还多 79 行冗余保险。

证据：`git show 20425b4:src/apps/desktop/src/ui_dioxus/app.rs | wc` 等同节点测试。

### 1.2 `memory_db.rs` ≤ ceiling 894 — **PASS**

- HEAD `memory_db.rs` 实测：**849** 行。
- 与原 920 → 894（红线）+ 实际 849（= ceiling - 5）。
- ceiling 已下调至 859（= 849 + 10，符合 brief 授权 "实际+10" 公式）。

证据：`scripts/rot-budget.json` 第 64-66 行（HEAD 树内）`ceiling: 859`，note 字段含 "W15-1k ceiling 894→859 (test seam extraction, 0 behavior change)"。

### 1.3 `pnpm run check:rot` 全绿 — **PASS**

实施者报"12/12 pass"。我亲跑复核：

```
ℹ pass 12
ℹ fail 0
Rot budget verification passed (5 grep rules [...], 3 dir rules
[dir_entries:scripts=42/42, dir_entries:docs/design=1/1,
dir_entries:.superpowers/sdd=49/400], 6 god-file rules checked across 1368 files).
```

（注：初跑 12/11 因我审查过程中在 `scripts/` 写了 8 个 `tmp_compare*.cjs` 临时文件，触发 dir_entries:scripts=50>42 红线。删除后 12/12 绿——实施者报的 `42/42` 与清理后实际一致。）

### 1.4 `cargo check --workspace` 绿 + `cargo test -p northhing-core --features product-full memory_db` 绿 — **PASS**

亲跑两命令：

- `cargo check --workspace`：**0 error**，仅 16+2+60=78 警告（全部为 pre-existing，rot-budget 报告中已附全警告原文；warnings 计数与 BASE 同等量级，无新增）。
- `cargo test ... memory_db`：**24/24 passed, 0 failed**（与报告 §6.3 完全一致）。

补充跑：
- `cargo test ... agent_memory`（覆盖缝消费方）：**70/70 passed**。
- `cargo test ... continuity_selfcheck`（含 L98 `with_test_memory_db_path` 调用）：**1/1 passed**。

缝消费方（`facts.rs:661/664/724/727`、`auto_memory.rs` 多处、`continuity_selfcheck.rs:98`）经测试实证未断。

### 1.5 零行为变化（纯搬移 + import/mod 调整 + ceiling 下调） — **PASS**

**关键质询 ①：spawner 两函数逐字一致性**

用脚本对比 BASE `app.rs:654-776`（123 行）vs HEAD `window_ops.rs:101-223`（123 行）：

```
BASE app.rs slice length: 123
HEAD window_ops.rs slice length: 123
Total line diffs: 0
```

**零差异**。`spawn_module_window` 与 `spawn_module_window_with_theme_rx` 的全部代码、I2 证据注释（13 行）、T7 裁定注释（5 行）逐字搬迁。

**关键质询 ②：I2/T7 证据注释与缝设计说明注释随块完整保留**

- I2 注释（BASE app.rs:684-691）→ HEAD window_ops.rs:131-138：完整保留，8 行 + 字符级一致。
- T7 注释（BASE app.rs:770-774）→ HEAD window_ops.rs:217-221：完整保留，5 行 + 字符级一致。
- 缝设计说明注释（BASE memory_db.rs:750-766）→ HEAD test_seam.rs:3-19：完整保留，17 行内容逐字一致。

**关键质询 ③：`pub use super::window_ops::{...}` 不扩大可见性**

- BASE `app.rs:655`：`pub fn spawn_module_window(...)`（`mod app;` 是私有的，外部 crate 不可达，内部 ui_dioxus 子树可达）。
- HEAD：`pub fn spawn_module_window(...)` 在 `window_ops`（私有 mod）+ `pub use super::window_ops::{...}` 在 `app.rs:36`。
- 外部可见路径 `crate::ui_dioxus::app::spawn_module_window*` 保留；`window_ops::spawn_module_window*` 是新增的内部路径但 `window_ops` 是私有 mod 不外泄。**等价**。
- 5 个内部调用点（`app.rs:470/482/494/626/641`）+ 3 个外部调用点（`pages_space.rs:439` `super::app::...`、`windows/facility.rs:190/203` `crate::ui_dioxus::app::...`、`windows/self_app.rs:258` `crate::ui_dioxus::app::...`）皆经 `cargo check` 实证仍可达。

**关键质询 ④：`test_seam.rs` 全文件 `#[cfg(test)]` 门控正确**

- `agent_memory/mod.rs:9-10`：`#[cfg(test)] pub(crate) mod test_seam;`（仅测试编译）。
- `agent_memory/mod.rs:25-26`：`#[cfg(test)] pub(crate) use test_seam::{...};`（仅测试编译）。
- `memory_db.rs:738-741` 的 `#[cfg(test)]` 分支调 `super::test_seam::test_memory_db_path_override()`，release 模式下整段编译消失，对 `test_seam` 无依赖。

**关键质询 ⑤：ceiling 859 = 849+10 符合 brief 授权**

`memory_db.rs` 实际 849，ceiling 859，差额 10，正好满足 brief §2 "新实际+10" 公式，未多占冗余。✓

**test_seam.rs 的两处语义保留性适配**（脚本对比定位，已确认非行为改变）：

```
DIFF 1: "fn test_memory_db_path_override()" → "pub(crate) fn test_memory_db_path_override()"
        原因：跨模块访问必需（BASE 同模块私有，HEAD test_seam 模块需 pub(crate) 才能被 memory_db.rs 引用）。
DIFF 2: "/// RAII guard that redirects [`default_memory_db_path`] ..." 
     → "/// RAII guard that redirects [`super::default_memory_db_path`] ..."
        原因：rustdoc 链接路径需更新（符号已不在同模块）。
```

两者均为模块边界适配，不影响运行行为；rustdoc 链接解析后指向同一函数。

## 2. QUALITY

### 2.1 复用侦察 — **PASS**

报告 §2 已读 `window_ops.rs`（91 行）+ `windows/mod.rs`（114 行）现状，并给出选择理由（与 `close_module`/`quit_shell` 主题契合、91→223 行紧凑适中、避免 `windows/` 子目录深度）。实地复核：

- `window_ops.rs` 现承担 `close_os_window` + `close_module` + `close_all_modules` + `quit_shell`（旧）+ 新增 `spawn_module_window*`（新）：**统一为"窗口生命周期操作"主题**，落点正确。
- 不在 `windows/` 下新建 `spawner.rs` 等子文件——避免目录深度膨胀，符合家规 0 (YAGNI/复用优先)。

### 2.2 无 owner 抽象 — **PASS**

`test_seam.rs` 是新模块但非"新抽象"——它是把现有 `#[cfg(test)]` 标注的代码块打包成同目录新文件，不引入任何 trait/interface/factory。`window_ops.rs` 加 spawner 同样不引入新抽象。两处都是字面意义的"搬迁"。

### 2.3 预算闸 — **PASS**

`scripts/rot-budget.json` diff 严格限于 `memory_db.rs` 一项：

```diff
-  "ceiling": 894,
-  "note": "R-14 god-file; live observation cohort (T2-6 superseded)"
+  "ceiling": 859,
+  "note": "R-14 god-file; live observation cohort (T2-6 superseded); W15-1k ceiling 894→859 (test seam extraction, 0 behavior change)"
```

仅 1 个 ceiling 下调（894→859），无任何条目上调、无新增 >800 行登记条目。其它 god-file 条目（`selectors.rs` 等）字节未动（diff 已核）。

### 2.4 条件早退测试 — **N/A**

本单无新增测试（24 个 memory_db 测试全部 pre-existing，行为零变化不应有新增），无早退路径需要审查。

### 2.5 god-file 健康度观察 — **PASS**

报告 §5：`memory_db.rs` 健康度 "更清晰"，理由"测试桩 RAII 与生产环境 SQLite 关注点分离"。

观察合理：`MemoryDbPathGuard`、`with_test_memory_db_path`、`unique_test_memory_db_path`、`TEST_MEMORY_DB_PATH` thread-local 全部是单元测试专用基础设施，搬迁到 `#[cfg(test)]` 全文件门控的 `test_seam.rs` 后，生产 `memory_db.rs` 的 release 编译产物完全不带这些代码——这是真实收益，不仅是行数瘦身。

## 3. Cannot verify from diff

无（本单是纯位移 + import 调整，diff 完整反映所有改动；无可在 diff 外推断的隐藏行为）。

## 4. Plan-mandated 冲突

无（无 plan 与实现之间的决策冲突；ceiling 下调遵循 brief §2 "实际+10" 公式）。

## 5. 提交卫生

- `75b9a11`：core 缝迁移（memory_db.rs + mod.rs + test_seam.rs + rot-budget.json），4 文件 / 75+/75-，范围清晰。
- `20425b4`：desktop spawner 迁移（app.rs + window_ops.rs），2 文件 / 140+/134-，范围清晰。
- 两 commit 拆分符合 brief §8 "可拆两个 commit"。
- commit 消息均带 `(W15-1k)` 任务标签，描述清晰。
- commit 时间差 19 秒，`author` 字段一致（Mavis <mavis@northhing.local>）。
- 未发现 `git add -A` / `git restore .` / 整树 `git checkout` 等禁操作（commit 范围仅限 6 文件，未越界）。

## 6. 总判

**APPROVE**

- SPEC：5/5 PASS（含字节级搬迁一致性、零行为变化、调用点兼容性实证）
- QUALITY：5/5 PASS（复用侦察属实、无新抽象、预算闸只降、条件早退 N/A、god-file 健康度观察合理）
- 0 Critical / 0 Important / 0 Minor

实施者已交付"零行为变化的纯位移瘦身"——123 行 spawner 字节级搬迁 + ~70 行测试缝语义保留性搬迁 + ceiling 894→859 下调，所有验证（check:rot 12/12、cargo check 0 error、memory_db 24/24、agent_memory 70/70、continuity_selfcheck 1/1）实证绿。允许文件集严格不越界。
