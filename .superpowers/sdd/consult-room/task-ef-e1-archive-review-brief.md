# E1 archive 终审 brief

只读。不改代码、不 commit。

需求：`task-ef-e1-archive-brief.md` + 总 brief `task-ef-pages-master-brief.md`  
报告：`task-ef-e1-archive-report.md`

## 必读

- `git diff`：pages_archive.rs（新）+ mod.rs / registry.rs / app.rs / css.rs / i18n.rs / windows.rs（pub win）+ 三份 locales
- 截图 Read 打开：
  - `C:\WINDOWS\TEMP\opencode\t7-shots\e1-archive-dark.png`
  - `e1-archive-light.png`
  - `e1-archive-folded-dark.png`
- `flags.rs` 必须 `DIOXUS_SHELL=false`
- TRUTH_CSS 字节未改

## 验收

1. 独立 OS 窗，轻 chrome，Center 非左右泊
2. abyss `#3F837B` 冷色，无珊瑚 rep 主导
3. ≥8 层地层，越深越淡，选中有左缘条
4. 三张侧卡可折到标题；中枢可折
5. 18px 级内边距；叙事化计数无 dashboard
6. 只接「档案」入口，无走廊假按钮
7. 新文件 <800；未堆爆 windows.rs

输出：`task-ef-e1-archive-review.md`  
总判决 PASS/FAIL；Spec/Quality；C/I/M/F；CAN MERGE。
