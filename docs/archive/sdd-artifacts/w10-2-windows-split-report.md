# W10-2 windows.rs 分拆报告

**状态**: DONE（审查修复通过）  
**Fix Commit**: `b284fa4`  
**日期**: 2026-08-29

## git show --stat

```
commit b50ba6e4221afc422335d3b99665cbc133539825
Author: Mavis <mavis@northhing.local>
Date:   Sat Aug 29 17:42:15 2026 +0800

    W10-2: split windows.rs into windows/ module directory (857→800 lines, 0 behavior change)

 src/apps/desktop/src/ui_dioxus/windows.rs          | 800 ---------------------
 src/apps/desktop/src/ui_dioxus/windows/facility.rs | 221 ++++++
 src/apps/desktop/src/ui_dioxus/windows/mod.rs      | 114 +++
 src/apps/desktop/src/ui_dioxus/windows/self_app.rs | 281 +++++++++++++++++++++
 src/apps/desktop/src/ui_dioxus/windows/work.rs     | 241 ++++++++++++++++++
 5 files changed, 857 insertions(+), 800 deletions(-)
```

## 验证

### 1. `cargo +stable-msvc check -p northhing`（0 error）
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.94s
```
warning 数稳定（2 unused_mut，bin 60 warnings pre-existing baseline）。

### 2. `cargo +stable-msvc test -p northhing --lib`（全绿）
```
test result: ok. 140 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 3. `node scripts/verify-rot-budget.mjs`（绿）
```
Rot budget verification passed (5 grep rules [...], 3 dir rules [...], 6 god-file rules checked across 1364 files).
```

## 新旧文件行数清单

| 文件 | 行数 |
|---|---|
| ~~windows.rs~~（删除） | ~~800~~ |
| `windows/mod.rs`（薄壳：re-export + 共享件） | 114 |
| `windows/self_app.rs`（self_app_root） | 281 |
| `windows/facility.rs`（facility_app_root） | 221 |
| `windows/work.rs`（work_app_root + fmt_tokens） | 241 |

三层拆分后，最大文件 281 行（self_app.rs），远低于 800 线红线。

## 偏离清单

| 偏离项 | 说明 |
|---|---|
| 总行数增加（800 → 857） | 模块拆分 overhead：每文件 6 行 SPDX 头部 + import 重复 + 目录壳本身。纯位移行为零变化，行数增加是结构固定成本。 |
| `fmt_tokens` 从 `windows.rs` 移至 `work.rs`（在 mod.rs 中通过 `pub use self::work::fmt_tokens` 暴露） | 计划写 `mod.rs` 薄壳 + 从 work.rs 导出；实际实现一致。 |
| `win::` FFI 模块在 mod.rs 而非 work.rs | 三个窗组件共享，保持原位置（所有三份都需）。 |
| `super::` vs `crate::ui_dioxus::` 路径 | sub-files 用 `crate::ui_dioxus::` (绝对路径) 避免多层 super:: 链不可维护性。registry.rs 的 `super::windows::x` 无需改动。 |

## 修复过程

| 阶段 | 错误数 | 修复 |
|---|---|---|
| 第1次 check | 36 | `self` 模块名冲突 → `self_app`；`window()` 无导入 → 显式 `use dioxus::desktop::window`；`WindowExtWindows` 各子文件加独立 import；用 crate-root 绝对路径替代 super:: (多层不可达) |
| 第2次 check | 12 | 同上，剩余 9 × hwnd (WindowExtWindows 缺 import) + fmt_tokens + `self in paths` |
| 第3次 check | 2 | `fmt_tokens` 双重定义 E0255 + 找不到 E0425 — 最终策略：fmt_tokens 回迁 mod.rs (pub(crate))，sub-files 用 `super::fmt_tokens` |
| 第4次 check | 1 | `Drop` impl 被删剩半边 — 补全未关闭的分隔符 |
| 第5次 check | **0** | 通过 |

## 备注

- `windows.rs` 在 rot-budget `god_file` 条目中无注册，无需清理条目。
- `docs/status/surfaces.md` 无需更新（路径语义不变，仍是 `src/apps/desktop/src/ui_dioxus/windows/` 模块）。
- `ui_dioxus/mod.rs` 无需改动：`mod windows;` 自动 resolve 到 `windows/mod.rs`。
- registry.rs 中的 `super::windows::self_app_root` / `super::windows::facility_app_root` / `super::windows::work_app_root` 路径无需改动（通过 mod.rs re-export 可见）。

---

# 审查修复记录（C-1）

## 问题

`windows/facility.rs:74` 几何跟随线程 `off_x` 遗漏 `DOCK_GAP_PX`：
- 误：`let off_x = ((280.0) * scale) as i32;`
- 正：`let off_x = ((280.0 + DOCK_GAP_PX as f64) * scale) as i32;`

同时 `use crate::ui_dioxus::entry::DOCK_GAP_PX;` import 一并丢失。

## 修复 diff（commit b284fa4）

```
 src/apps/desktop/src/ui_dioxus/windows/facility.rs | 3 ++-
 1 file changed, 2 insertions(+), 1 deletion(-)

 +use crate::ui_doxus::entry::DOCK_GAP_PX;
-let off_x = ((280.0) * scale) as i32;
+let off_x = ((280.0 + DOCK_GAP_PX as f64) * scale) as i32;
```

## 自查同类漏项结论

逐常量比对 `git show b50ba6e~1:src/apps/desktop/src/ui_dioxus/windows.rs`：

| 子文件 | DOCK_GAP_PX import | `off_x` 公式 | 结论 |
|---|---|---|---|
| `self_app.rs:75` | `use crate::ui_dioxus::entry::DOCK_GAP_PX;` ✓ | `(280.0 + DOCK_GAP_PX as f64) * scale` ✓ | 等价 |
| `facility.rs:74` | **恢复后** ✓ | **恢复后** ✓ | 修复完成 |
| `work.rs:75` | `use crate::ui_dioxus::entry::DOCK_GAP_PX;` ✓ | `(DOCK_GAP_PX as f64 * scale)` ✓（work 窗在右，不加 280，同原版） | 等价 |

无其他同类漏项。

## 重跑验证

### 1. `cargo +stable-msvc check -p northhing`（0 error）
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.91s
```

### 2. `cargo +stable-msvc test -p northhing --lib`（全绿）
```
test result: ok. 140 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 3. `node scripts/verify-rot-budget.mjs`（绿）
```
Rot budget verification passed (5 grep rules [...], 3 dir rules [...], 6 god-file rules checked across 1364 files).
```

## 偏离清单

无偏离。仅限定点修复：+1 import 行 + 修正 1 表达式，行为完全还原原版。
