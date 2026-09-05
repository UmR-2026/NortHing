# W16-2 Brief：审查包组装脚本 manifest 化

- 任务标识：W16-2
- 波次计划：`E:\agent-project\NortHing\.superpowers\sdd\plan-2026-09-05-w16-trusted-core.md`
- 来源：D-synthesis §3 Phase -1.6 + C-meta-review F-M-1（审查包材料 1/6 双包静默缺失）
- BASE：脚本现状即 BASE（该文件未提交进 git 历史亦可，以磁盘现状为准）

## 背景（一句话）

`assemble-review-pkgs.ps1` 用硬编码 Emit 调用组装审查包，漏发 1/6 材料且无任何缺失标注——外部审查者无法区分"脚本 bug"与"刻意省略"。改造为清单驱动 + 机器可读 manifest。

## 允许文件集（diff 越出 = judge Critical）

1. `E:\agent-project\.opencode\external-review\2026-09-05\assemble-review-pkgs.ps1`（原地改造）

禁区：其它一切文件。产物（重新组装的包 md + manifest.json）用于验证，**不入 git commit**。

## 功能要求

1. **清单驱动重构**：脚本顶部声明每包的材料清单数组，每项含 `title` / `path` / `required`（$true/$false）。Emit 循环遍历清单，禁止散落的硬编码 Emit 调用。
2. **1/6 根因修复**：A 包清单显式补 1/6 材料 = `<USERPROFILE>\.config\opencode\AGENTS.md`（编排者纪律，required）；B 包 1/6 与 2/6 当前重复指向同一 SKILL.md——清单只保留一条（2/6 有条目），删除空的 1/6 标题位，材料编号由清单顺序自动生成（不要手写 "N/6"）。
3. **缺失语义（钉死）**：`required:$true` 材料文件不存在 → 脚本报错并非零退出；`required:$false` 缺失 → 包体内该位置写入 `> **OMITTED**: <title> — <reason>` 显式节 + manifest 记录 + 控制台 warning，继续组装。
4. **package-manifest.json**：每次组装在同目录输出，结构：`{ scriptSha256, generatedAt(系统时间 ISO), packages: [{ file, sha256, bytes, materials: [{ title, path, sha256, bytes, status: "EMITTED"|"OMITTED", reason }] }] }`。
5. 保持现有 Emit 的 UTF8 编码行为与包体结构（标题 + 来源 + ```text 围栏）不变。

## 验证（命令 + 输出原文进 report）

```text
pwsh -File .opencode\external-review\2026-09-05\assemble-review-pkgs.ps1   # 工作目录 E:\agent-project
```

- 正向：两包重组装成功；manifest.json 两包各材料 status=EMITTED；A 包含原 1/6 材料真实内容（AGENTS.md 文本）；包体无空标题节。
- 负向：临时把某个 required:$true 材料路径改不存在 → 脚本非零退出；改回。临时把某个 required:$false 材料路径改不存在 → 包体出现 OMITTED 节 + manifest 标注；改回。
- 全部输出原文进 report。

## 报告

写到 `E:\agent-project\NortHing\.superpowers\sdd\reports\w16-2-report.md`（不入 commit）：改动摘要 / 缺失语义设计一段 / 验证命令+输出原文 / 结尾状态词。

## 派发元信息

- commit 规则：只 `git add` 该 ps1 一个文件（在 agent-project 仓根 `E:\agent-project` 提交）；message：`feat(review): assemble script manifest-driven + OMITTED semantics (W16-2)`。
- report/brief 不入任何 commit，由编排者收口。

## Global Constraints（摘编自计划）

1. PowerShell 脚本兼容 pwsh 7；零新依赖。
2. 所有验证命令必须在 report 贴原文输出（命令 + 结果）。
3. 脚本输出 English-only；包体内容保持中文材料原样。
4. report 结尾状态词：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。
