# Task Brief — W12-0: 需求表过期行刷新 + W12 plan 入库（纯文档单）

仓库：`E:\agent-project\NortHing`（分支 main）。**BASE commit = `5e95cf2`**。
并行提示：同一时刻另一个子代理在改 `src/crates/contracts/` 与 `src/crates/assembly/core/` 的 Rust 源码。**你只许碰本单 §4 点名的两个文件**，不要动任何 `.rs`，也不要提交别人的文件。

## 1. 来源与验收标准

来源：2026-08-31 独立盘点报告 `.superpowers/sdd/project-status-2026-08-31.md`（第 5 节）+ 编排者复核。
问题：`docs/product/requirements-vs-current-2026-08-29.md` 的状态列成文于 2026-08-29 晨（W9 落地前），最后一次编辑 `cfd5ece` 只追加了 C6/C7/C8 三行，**状态列从未刷新** → 至少 10 处已过期，而该表被 handoff 列为「权威需求基线」，会误导排期。

验收（逐条可机械核对）：
1. 表内 10 处过期行的状态符号已按实际改对，每处改动都能在「复核说明」节查到证据。
2. 表头/说明里的复核日期更新为 2026-08-31。
3. 新增「2026-08-31 复核说明」节：10 行对照表（条目 / 原值 / 现值 / 证据 commit 或 file:line）。
4. `.superpowers/sdd/plan-2026-08-31-session-crud-gaps.md` 已入库（跟踪）。
5. 两个 commit，各含点名文件，`.superpowers/` 只含 plan 文件那一个。

## 2. 编排者预检结论（直接采信）

- W9 七个任务的 commit 时间（实测）：`921c09d` 04:48 / `4aba165` 08:07 / `879b7c4` 11:06 / `4a9818d` 12:12 / `7c8d1b7` 13:00（均 2026-08-29）。
- 需求表最后一次编辑 = `cfd5ece`（08-29 22:17），`git show cfd5ece --stat` 显示该文件 **+3 行**（只加 C6/C7/C8）。
- 会话行失真已由编排者亲自复核坐实（`pages_archive.rs:312-321` 标题过滤；`4aba165` 交付删除/重命名/导出）。

## 3. 待修正的 10 处（逐条给原值 → 现值 → 证据）

| # | 条目 | 原值 | 现值 | 证据 |
|---|---|---|---|---|
| 1 | 会话系统总览 + SE-02/05/06/07 | 「删除、重命名、导出、搜索没有」/ 四行 ❌ | 删除✅ 重命名✅ 导出✅；**搜索 ⚠️（仅标题过滤）** | `4aba165`+`9603a65`；`pages_archive.rs:312-321` |
| 2 | 记忆系统总览「❌ 最大落差」+ TH-3 行 | 零 UI | ✅ 只读面板（浏览/搜索/导出 JSONL） | `c80227b`+`d02502e`+`57513b6`（W9-2） |
| 3 | 原则 9 降级即报错（总览 + 论题域行） | ❌ UI 无路径 | ✅ amber 横幅 + degraded Signal | `82371f5`+`57513b6`（W9-3） |
| 4 | TO-02 确认交互 | ⚠️ 缺「本会话内允许」 | ✅ 第三档已接线 | `921c09d`+`d742e75`+`3e55d75`（W9-1） |
| 5 | SK-05 技能管理 UI | ❌ | ⚠️（列表+启停已有；SK-06 创建 / SK-07 分享仍缺） | `879b7c4`（W9-5） |
| 6 | WS-03/WS-04 文件树与预览 | ❌ | ✅ 右面板模块（含 symlink 围栏） | `4a9818d`+`f7df521`（W9-6） |
| 7 | 显示模式 🟡 / 左列四卡 🟡 | 摆设 | ✅ 做真（显示模式持久化；四卡接真实数据源；位格/准则为诚实空态） | `7c8d1b7`（W9-7） |
| 8 | 身份系统「名讳/位格/准则硬编码假数据」 | ⚠️ | ⚠️ 但**理由改写**：名讳接默认 provider display_name（W5-3 权宜映射，W9-7 遗留 M-2）；位格/准则不再伪造但无真实数据源 | `7c8d1b7`；`w9-7-review.md` F-MINOR-2 |
| 9 | SE-08 子代理可见性 | ⚠️ | ✅ 归档页 badge 低显著度可见（C3 裁决落法） | `4aba165`；`pages_archive.rs` subagent badge |
| 10 | 验收环「记忆回顾 ❌」 | ❌ | ✅ 面板已有（「隔天还记得」仍待真机实测第 9 项） | `c80227b` 等（W9-2） |

**纪律要求**：动手改每一行前，用 rg / 读源码**独立核实**该行现值是否属实（盘点报告也可能错）。若发现某处实际与表中所写不同（含表中已对、或盘点报告判错），**按实际改**，并在复核说明节末尾加「核实偏离」小节逐条记录。不许无脑照抄上表。

## 4. Spec

1. 只改两个文件：
   - `docs/product/requirements-vs-current-2026-08-29.md`（状态列 + 复核日期 + 新增复核说明节）
   - `.superpowers/sdd/plan-2026-08-31-session-crud-gaps.md`（未跟踪 → 入库）
2. **不改文件名**（保留 `2026-08-29` 后缀，避免破坏引用），但在文件顶部说明里写明「状态列 2026-08-31 复核」。
3. 表内状态图例（✅/⚠️/🟡/❌）保持原样，不改图例定义。
4. 复核说明节放文件末尾，格式 = markdown 表格，四列：条目 / 原值 / 现值 / 证据。
5. 中文表述，术语与英文标识符保持原样。
6. 不改任何代码文件、不新建文件、不改 `.superpowers/` 下其它文件、不动 `progress.md`。
7. 两个 commit：
   - commit 1：`docs(product): refresh 10 stale rows in requirements-vs-current (2026-08-31 recheck)`，只含需求表
   - commit 2：`docs(sdd): track W12 session-fulltext-search plan`，只含 plan 文件

## 5. 验证（输出原文进 report）

```powershell
cd E:\agent-project\NortHing
git diff --stat
git show --stat HEAD
git show --stat HEAD~1
git status --short
```
要求：`git status --short` 除你自己的报告文件外**没有别的新增未跟踪文件**（若看到 `.rs` 改动 = 别人的工作，不许 add）。

## 6. 报告

路径：`.superpowers/sdd/w12-0-docs-refresh-report.md`
必含：状态词（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、两个 commit SHA、`git show --stat` 输出、**第 3 节 10 处逐条核实结论**（哪几条按预期改、哪几条实际不同并改了什么）、验证输出。

## 7. 派发元信息

- BASE commit：`5e95cf2`
- 禁区：`progress.md`、任何 `.rs`、`.superpowers/` 下除 plan 与你的报告外的文件、`scripts/`
- commit 规则：恰好两个 commit，禁止 `git add -A` / `git add .` / `git restore .` / `git clean`；只许点名文件 add
- 本单不需要跑 cargo（纯文档）
