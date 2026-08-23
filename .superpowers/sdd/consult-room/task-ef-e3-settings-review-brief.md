# E3 settings 终审 brief

只读。不改代码、不 commit。

需求：`task-ef-e3-settings-brief.md` + 总 brief `task-ef-pages-master-brief.md`  
报告：`task-ef-e3-settings-report.md`

## 必读

- `git diff`（相对 HEAD `7387815`）：`pages_settings.rs`（新）+ mod.rs / registry.rs / app.rs / css.rs / i18n.rs / windows.rs（两处 ≡ 全局设置接线）+ 三份 locales
- 截图 Read 打开：
  - `C:\WINDOWS\TEMP\opencode\t7-shots\e3-settings-dark.png`
  - `e3-settings-light.png`
  - `e3-settings-folded-dark.png`
- `flags.rs` 必须 `DIOXUS_SHELL=false`
- TRUTH_CSS 字节未改

## 验收

1. 独立 OS 窗 760×580，Center，轻 chrome（▴ 收纳 + 主题 + ✕，无 ─□）
2. 双列哲学：左「它的自我」只读视觉、右「设施」可点 mock，非管理后台表格
3. 卡可折到标题；展开卡流体拉伸
4. windows.rs 两处 ≡ 全局设置已接 spawn（stop_propagation 在位）
5. css.rs 增量 ≤40 行（报告称 20）；全文件 <800
6. i18n 三语对称；audit exit 0
7. E1 archive / E2 space 未被破坏（pages_* 仍在、registry 三插件齐）
8. flags=false；无 commit

输出：`task-ef-e3-settings-review.md`  
总判决 PASS/FAIL；Spec/Quality；C/I/M/F；CAN MERGE。
