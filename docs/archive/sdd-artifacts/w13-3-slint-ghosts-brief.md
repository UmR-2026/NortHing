# Task Brief — W13-3: 清退 Slint 幽灵文档与契约注释

仓库：`E:\agent-project\NortHing`（main，BASE = `f5dc0ef`）。纯文档/注释单，不涉行为。

## 背景

Slint 壳于 2026-08-28 物理删除（`707e414` + `0c95aa6`），Dioxus 是唯一壳（根 AGENTS.md 骨干不变量已更新）。但文档与代码注释里仍有 Slint 残留，属于"幽灵文档"——读了会被误导。

审计（R4）结论：所有 `Cargo.toml` 零 Slint 依赖，生产 `.rs` 零 active 引用（仅历史对比注释）；R1 另有发现：文档/契约陈旧漂移 6 处。

## Spec

1. **全仓扫描**（你自己 grep，不要只信下面清单）：
   `git grep -in "slint"`，排除这些**不要动**的位置：
   - `.agents/reference/**`（历史参考资料，保留原貌）
   - `docs/archive/**`、`docs/design/**` 下的历史设计文档（保留，但**若文件通篇在讲 Slint 且已被 Dioxus 取代**，在文件头加一行 `> 历史文档：Slint 壳已于 2026-08-28 删除，现行唯一壳 = Dioxus。本文仅作历史记录。`）
   - `.superpowers/**`
   - 归档的 handoff（保留原貌）
2. **要改的**：
   - `src/apps/desktop/README.md`（若通篇 Slint）→ 重写为 Dioxus 现状，或加历史标记 + 更新为现状
   - `src/crates/contracts/runtime-ports/src/mcp.rs:108` 附近的 Slint 契约注释 → 改为与现行实现一致（不确定现行实现是什么就 **NEEDS_CONTEXT 上报**，不要编）
   - 其它生产 `.rs` 里描述 Slint 的注释/文档注释（如 "Slint properties 必须从事件循环线程写" 这类）→ 若规则已不适用于 Dioxus，删除或改写；**拿不准就保留并加 TODO 注明待确认**（本仓有 13 处 TODO 无 owner 的教训，所以 TODO 要写清 owner/日期：`// TODO(orch 2026-08-31): ...`）
   - 根 `AGENTS.md` / 就近 `AGENTS.md` 里若仍有 Slint 表述（R1 提到 desktop/README.md 等）→ 更正为 Dioxus。**注意**：根 AGENTS.md 的骨干不变量段里已写"唯一壳 = Dioxus（Slint 已于 2026-08-28 物理删除）"，不要重复改错；只处理仍把 Slint 当现状的表述。
3. **不许**改任何代码行为、不许删文件、不许改 `.gitignore`。

## Constraints

1. 只碰：README / 文档 / 代码注释 / AGENTS.md 里确实过期的 Slint 表述。
2. 中文文档用中文，代码注释用英文（本仓规则：日志英文无 emoji；注释可英文，与文件既有语言保持一致）。
3. **SDD 禁区**：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止 `git add -A`。
4. 恰好一个 commit。
5. **不许跑 cargo**（无代码行为改动；若你确实动了 `.rs` 注释，`cargo check -p northhing` 跑一次确认没写坏语法，但注释改动不该触发编译问题）。

## 验证（输出原文进 report）

```powershell
cd E:\agent-project\NortHing
git show --stat
git diff
git grep -in "slint" -- "src" "docs" "*.md" | Measure-Object   # 改动前后的命中数对比
```
report 里给出：**改前命中数 → 改后命中数**，以及"剩余未改的都在哪、为什么保留"。

## 报告

路径：`.superpowers/sdd/w13-3-report.md`
含：状态词、commit SHA、`git show --stat`、逐处改动清单（文件:行 + 原文 → 新文）、保留项及理由、命中数对比。

## 派发元信息

BASE = `f5dc0ef`；禁区：`.superpowers/`（除报告）、`progress.md`、`.agents/reference/**`（保留原貌）、任何代码行为改动。
