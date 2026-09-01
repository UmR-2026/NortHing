# W10-2 Review（judge 验收单）— **第 2 轮（重审）**

**判决**: **PASS — CAN MERGE**
**SPEC**: PASS · **QUALITY**: PASS
**Critical / Important / Minor**: **0C / 0I / 3M**（3M 沿袭上轮，无新增）

## 一句话理由
fixer commit `b284fa4` 已修复 C-1：`windows/facility.rs` 补回 `use crate::ui_dioxus::entry::DOCK_GAP_PX;` 并把 `off_x` 改回 `((280.0 + DOCK_GAP_PX as f64) * scale) as i32`；与原版 `b50ba6e~1:windows.rs` 三处 off_x 语义逐字等价（self_app/facility = `280+DOCK_GAP_PX`，work = `DOCK_GAP_PX`），`cargo check -p northhing --lib` 0 error / 2 warning 基线持平。

---

## C-1 重审证据（fixer commit `b284fa4`）

```
commit b284fa41a3c502b3e7451ce276c4b55b58f78171
 src/apps/desktop/src/ui_dioxus/windows/facility.rs | 3 ++-
 1 file changed, 2 insertions(+), 1 deletion(-)
```

**off_x 三方对照表**（git show 比对）：

| 组件 | 原 `b50ba6e~1:windows.rs` | 当前 `main` |
|---|---|---|
| self_app_root | `((280.0 + DOCK_GAP_PX as f64) * scale)` | `windows/self_app.rs:75` 同式 ✓ |
| facility_app_root | `((280.0 + DOCK_GAP_PX as f64) * scale)` | `windows/facility.rs` 经 b284fa4 修复后同式 ✓ |
| work_app_root | `(DOCK_GAP_PX as f64 * scale)` | `windows/work.rs:75` 同式 ✓ |

**import 对照**：
- `windows/facility.rs` line 6: `use crate::ui_dioxus::entry::DOCK_GAP_PX;` ✓（新增）
- `windows/self_app.rs:6` 与 `windows/work.rs:6` 早前已有 ✓
- `windows/mod.rs:21` 仍保留但子文件独立 import，重复但无功能影响（沿 M-1）

**门禁实测**：`cargo +stable-msvc check -p northhing --lib` → Finished，2 warnings（与首轮基线持平，全部为预存 unused_mut，非本 commit 引入）。

**fixer 自查覆盖**：按 brief 指示对全文件做同类漏项扫描；其它 import / 公式 / 折叠逻辑逐项已对照原版（self_app/work 起点本就正确）。

---

## 双判决核验（第 2 轮）

| 判决 | 依据 | 结果 |
|---|---|---|
| SPEC | brief §1 三 app_root + fmt_tokens 与原 windows.rs 等价 | **PASS**（C-1 已修复，三 off_x 逐字等价） |
| SPEC | brief §2 5 轮修复无残骸（FIXME/TODO/重定义/悬空 impl） | PASS（rg 全空） |
| SPEC | brief §3 外层 caller 路径不变 | PASS（registry.rs / pages_*.rs 全部走 `super::windows::` re-export） |
| SPEC | brief §4 mod.rs 薄壳 + fmt_tokens 归属 | PASS（fmt_tokens `pub(crate)` 在 mod.rs；偏离记录诚实） |
| SPEC | brief §5 rot 收口绿 + manifest 无残留 | PASS（rot-budget.json 无 windows.rs 条目；verify-rot-budget 绿） |
| QUALITY | god-file 防御（800 → 最大 281） | PASS |
| QUALITY | 5 轮修复最终态无半截残留 | PASS（Drop impl 完整、fmt_tokens 单一定义点、所有悬空 closure 已闭合） |
| QUALITY | 行为等价性 | **PASS**（C-1 修复后三窗几何公式与原版字节一致） |

## 沿袭 Minor（无新增，开放终审 triage）

- **M-1** `windows/mod.rs:16-27` 残留未用 import（`dioxus::prelude::*` / `Rc` / `watch` / `css` / `entry::DOCK_GAP_PX` / `i18n::{keys,LocalePack}` / `state::Geometry` / cfg-gated `WindowExtWindows`）。子文件已独立 import，mod.rs 这 8 项属死引用。
- **M-2** `windows/mod.rs` 缺尾部换行（diff 末行 `\ No newline at end of file`）。
- **M-3** god-file 拆分缺「逐函数 diff 比对」自验——这是本轮 C-1 出现的根因（实现者只跑了 cargo check / cargo test，未做 facility_app_root 几何公式的 byte-level diff）。下轮 god-file 拆分应在 brief 强制该步骤，避免同类漂移。

3 项均为格式化/流程建议，不阻合并。

## 与首轮关系

首轮（`w10-2-review.md` 第 1 版）判定 **FAIL / 1C / 0I / 3M**（C-1 facility off_x 漏 `DOCK_GAP_PX`）。本轮为修复后重审，**C-1 已闭合**，3M 沿袭不变。结论反转：**CAN MERGE**。

## 备注

- 不需追加 ledger —— 终审一次性合并即可（ledger 追加时机为终审 PASS 后，本轮已是终审）。
- 下一站可考虑把 M-1/M-2 顺手清掉（house rule 1「在范围内小修」），但不在 W10-2 范围，可作为后续清洁 PR。