# Task 1 Review Brief — chrome 与系统层（consult-room Slint 建构期）

> 审查者：judge-m3（minimax-cn-coding-plan/MiniMax-M3）。
> 仅审查，不动代码；发现进 review 文件即可。

## 1. 位置

- worktree：`E:\agent-project\northing\.worktrees\consult-room-build`（分支 `feat/consult-room-slint`）。
- 待审 commit：`e311aeb`（基线 `e487cd8`）。
- review diff：`e487cd8..e311aeb`。
- review 报告：`E:\agent-project\northing\.worktrees\consult-room-build\.superpowers\sdd\consult-room\task-01-review.md`（不 commit）。

## 2. 必读（按序）

1. `.superpowers/sdd/consult-room/task-01-brief.md` — 实现任务 brief（需求唯一来源）；§3 是六项范围，§4 是 Global Constraints。
2. `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.html` — 视觉真值。chrome/边界/呼吸/编年史/头像以此为准。
3. `.superpowers/sdd/consult-room/task-01-report.md` — 实现报告（含三缺陷处置说明 + API 清单）。
4. `docs/design/2026-07-22-frontend-redesign/slint-feasibility-consult-room.md` — Slint 翻译词汇。
5. diff：`git diff e487cd8 e311aeb -- src/apps/desktop/src/ui/`。

## 3. 审查范围（双判决）

**Spec 合规**（实现是否覆盖 brief §3 的六项，逐一对照）：
1. WindowChrome 重制（标题栏废止 / brand-inline 状态行 / 窗控四键入主体区右上 / 印章收进房间左下 / 拖拽区）。
2. containment + membrane-frame 双边界（parent 表达式、resize 跟随、暗/亮语义）。
3. room-fog 沉积底雾（AirTint 演进，archive 冷雾 + speaking 升档语义保留）。
4. 呼吸 8s 单钟全局范式（opacity、振幅分级、常量/属性复用）。
5. 头像方形化（radius 0，近尖角语言）。
6. ChronicleBar 定稿（尖角 4px opacity .7、出生灰 → 强调色同源）。

**代码质量**：
- Slint 翻译红线遵守（§4 brief 逐条核对：禁 box-shadow/运行时 color-mix/infinite/伪元素/百分比 radius；Rectangle 无 scale-x/y，呼吸绑 opacity）。
- 哲学红线（rep 只属 agent；禁 dashboard 数字；禁 emoji；印章 opacity ~0.25；8s 单钟；近尖角；编年史右端同源）。
- commit 纪律：恰好一个产品 commit（e311aeb 已包含 .slint + 截图 + brief/report/plan/progress 等 SDD 文件——SDD 文档进产品 commit 是次要不规范，本次接受，但要在 review 留 FYI）。
- 截图：build-shots/t1-main-dark.png、t1-main-light.png（已随 commit 入库）。判读要点：四键渲染是否正常 / 双边界 / 头像方形 / 编年史 / 印章位置 / 暗/亮 aura 是否合规。

## 4. 判决与产出

- 双判决：spec PASS / FAIL，quality PASS / FAIL；缺一不算通过。
- Critical / Important → 必须修复（派 fixer 再审）。Minor → 进 ledger 终审 triage。
- 文件结构：
  ```
  # T1 Review
  ## Spec verdict: PASS|FAIL  — 简述
  ## Quality verdict: PASS|FAIL  — 简述
  ## Findings
  ### Critical
  - file:line — 描述
  ### Important
  - file:line — 描述
  ### Minor（ledger triage）
  - file:line — 描述
  ## 截图判读
  - dark: 结论
  - light: 结论
  ## 不可从 diff 判读项（仅记录）
  - …
  ## 总结
  - 一句话
  ```
- 最终回复：一句话 + review 路径。