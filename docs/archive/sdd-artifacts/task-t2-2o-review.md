# Task T2-2o Review — MiniApp 整删 M5 收口批

- **Task ID**: T2-2o
- **Reviewer**: judge-m3 (independent)
- **Base**: `17eb4bb`
- **HEAD**: 工作区未提交改动（已自验）
- **Final Status**: **PASS（双判决）**

---

## 1. SPEC 判决 — **PASS**

### 1.1 授权文件集核验（约束 1）

`git status --short` 实际清单：

| 文件 | 状态 | 授权 | 判定 |
|---|---|---|---|
| `MiniApp/` 37 文件 | D | ✓ 授权 A | OK |
| `AGENTS.md` | M | ✓ | OK |
| `AGENTS-CN.md` | M | ✓ | OK |
| `README.md` | M | ✓ | OK |
| `docs/architecture/backend-roadmap.md` | M | ✓ | OK |
| `docs/status/decision-register.md` | M | ✓ | OK |
| `docs/status/surfaces.md` | M | ✓ | OK |
| `docs/status/tech-debt-ledger.md` | M | ✓ | OK |
| `docs/tech-debt-cleanup-guide.md` | M | ✓ | OK |
| `src/crates/execution/agent-stream/src/tool_call_accumulator.rs` | M | ✓ 授权 D | OK |
| `.opencode/model-capability-notes.md` | M | 并行 session（brief 豁免） | OK |
| `memory/northhing.md` | M | 并行 session（brief 豁免） | OK |
| `.handoffs/handoff-g2-t9-2026-08-07.md` | ?? | 并行 session（brief 豁免） | OK |
| `.superpowers/sdd/task-t2-2o-{brief,diff,report}.md` | ?? | 本批 artifact | OK |

**越界检查**：零越界。所有本批改动严格落在 brief 授权清单内；并行 session 预存改动未触未改。

### 1.2 serde/wire 三处残留（约束 2 — 红线）

`git diff HEAD -- src/crates/contracts/core-types/src/surface.rs src/crates/services/services-core/src/session/session_metadata.rs src/crates/services/services-core/src/session/lineage.rs` → **空 diff**。

`rg -n "MiniApp|Miniapp" .../surface.rs` → `:52 MiniApp` ✓
`rg -n "MiniApp|Miniapp" .../session_metadata.rs` → `:27 Miniapp` ✓
`rg -n "miniapp" .../lineage.rs` → `:19 "miniapp" tag` ✓

**红线严守**：三处 serde/wire 残留原封未动，已登记 P2-21 待用户拍板。

### 1.3 骨架不变量（约束 3）

`AGENTS.md:176`（修改后）：
> **Shell safety**: `guard_command_execution` is wired into the `validate_input` path of Bash/ExecCommand and writes audit entries (see `9a1575d`). New shell-like tools must call it too.

`AGENTS-CN.md:137`（修改后）：
> **Shell 安全**：`guard_command_execution` 已接入 Bash/ExecCommand 的 `validate_input` 路径并写审计日志（见 `9a1575d`）。新增 shell 类工具必须同样接入。

**判定**：仅末尾 MiniApp 分句被摘除，guard_command_execution 本体、Bash/ExecCommand 引用、9a1575d commit 锚、审计条目语义、"新增 shell 类工具必须接入"全部一字未动。中英文措辞语义完全对齐。

其他 AGENTS.md:26/35/179 与 AGENTS-CN.md:25/34/140 仅做 MiniApp 表面枚举摘除（"MiniApp runtime IO"/"MiniApp UI"），骨干不变量其余文字未动。中文行无 GBK 双重编码迹象（编辑工具走 edit，未走 PowerShell Set-Content）。

### 1.4 roadmap 收口（约束 4）

| 约束点 | 文件:行 | 判定 | 证据 |
|---|---|---|---|
| PCS-3 语义段原样保留 | `backend-roadmap.md:190-206` | OK | git diff 该范围无 hunk 触及 |
| T2-2 行整行标完成 | `backend-roadmap.md:167` | OK | 行头加 `~~T2-2~~` + 末尾 `**Done**` + "已完成（2026-08-19）" + 合计 ≈40k+ 行 |
| T2-5 行只摘 miniapp::manager | `backend-roadmap.md:185` | OK | 仅 `miniapp::manager / ` 被摘，其余 `password_vault / mcp::auth / facts` 保留 |
| T1-1 已有划掉标注 | `backend-roadmap.md:151` | OK | 已是 `~~T1-1~~`（pre-existing） |
| T3-5 已有划掉标注 | `backend-roadmap.md:216` | OK | `~~T3-5~~` + 末尾补 "随 T2-2 M1-M5 完成" |
| SW1-1 随整删关闭 | `backend-roadmap.md:85` | OK | 加 `~~SW1-1~~` + "随 MiniApp 整删关闭（moot）" |
| :96 依赖关系更新 | `backend-roadmap.md:96` | OK | 标注 `MiniApp 已整删（commits a930c93..T2-2o）` |
| :117 MiniApp host 行 | `backend-roadmap.md:117` | OK | 状态改为 "已整删（T2-2 M1-M5, commits a930c93..T2-2o）" |
| :247 第三方生态失效 | `backend-roadmap.md:247` | OK | 加 `~~MiniApp 第三方生态~~` + "已失效" |

**判定**：所有 roadmap 收口点按约束执行；PCS-3 自足语义段完整保留。

### 1.5 P2-21 台账条目（约束 5）

`docs/status/tech-debt-ledger.md:232-237` 新增条目核查：

| 字段 | 内容 | 判定 |
|---|---|---|
| 编号 | P2-21 | OK（紧接 P2-20，无跳号） |
| Symptom | 三处 file:line 列出（surface.rs:52 / session_metadata.rs:27 / lineage.rs:19）+ 各自变体名 + 零构造零生产者陈述 | OK |
| Evidence | T2-2 MiniApp recon Q7 引用 + `rg` 全仓零业务构造 | OK（Q7:176 实测含三处残留讨论） |
| Proposed fix | "2026-08-19 用户决策超时未拍板，默认保守路径悬置待决"+ 后续路径（确认无迁移负担则整删 / 加 serde alias 后删） | OK |
| Status | `active (suspended / pending user decision)` | OK |

**判定**：条目内容完整，三要素（位置 + 悬置原因 + 决策超时事实）齐备。

### 1.6 tool_call_accumulator.rs 测试切除（约束 6）

文件 diff hunks 仅一行删除（行 150：`("InitMiniApp", "Markdown Viewer"),`）。

`rg -ni "initminiapp|markdown viewer" src/crates/execution/agent-stream/src/tool_call_accumulator.rs` → **空**（确认 0 残留）。

测试循环体（152-163）未改动——仅用例数组缩窄一格，单字段工具断言路径（`assert_eq!(finalized.arguments, json!({}))` + `assert!(finalized.is_error)`）对剩余 10 个工具（Bash/Skill/Read/GetFileDiff/LS/Delete/Glob/Grep/WebSearch/WebFetch）仍生效。

**判定**：纯用例行删除，无测试语义变化，无副作用代码外溢。

### 1.7 C 项终扫（约束 8）

`rg -n -i "miniapp|mini_app|mini-app" --glob '!docs/archive/**' --glob '!docs/handoffs/**' --glob '!docs/superpowers/**' --glob '!.superpowers/**' --glob '!memory/**' --glob '!research/**' --glob '!target/**' --glob '!docs/migration-2026-07-16/**'`

代码面剩余命中（src/ 除 P2-21 三处与 accumulator 外）逐条核验：

| 命中 | 分类 | 判定 |
|---|---|---|
| `src/crates/services/services-core/src/session/session_metadata.rs:27` | P2-21 悬置 | OK |
| `src/crates/services/services-core/src/session/lineage.rs:19` | P2-21 悬置 | OK |
| `src/crates/contracts/core-types/src/surface.rs:52` | P2-21 悬置 | OK |
| `src/crates/services/services-core/AGENTS.md:25` | 负向护栏注释 "Do not add ... MiniApp storage ..."（git diff 该文件为空，pre-existing） | OK（负向守卫，保留意图明确） |

**代码面新增残留：0。**

文档面剩余命中归类（与 report §3 表一致）：
- 台账：`tech-debt-ledger.md:232,234,235`（P2-21 条目本身）
- 路线图：`backend-roadmap.md:85,96,117,151,167,179,190,192,216,247`（均为授权收口点）
- 决策记录：`decision-register.md:19,40,64,71`（其中 :40 为本批改动，其余 pre-existing）
- 论题：`product-thesis.md:3,51,52`（pre-existing 历史基线）
- 历史审计/规划：`full-review-2026-08-16.md`、`2026-07-23-p2-9-stage2-triage.md`、`core-decomposition.md`、`agent-runtime-services-design.md`、`agent-kernel-northstar.md`、`sdlc-harness/**`、`security/r1-shell-exec-audit.md`、`PRD-v0.1.0.md`、`plans/**`、`reviews/**`（均 pre-existing，brief 豁免）

**判定**：终扫合规，代码面零新增残留，文档面剩余均为授权保留或 pre-existing 历史。

---

## 2. QUALITY 判决 — **PASS**

### 2.1 门禁复跑（约束 7 — reviewer 自验）

| 命令 | 预期 | 实测 | 证据 |
|---|---|---|---|
| `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace` | PASS | ✅ PASS | `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 2.18s`（含 pre-existing 5 条 warning，无 error） |
| `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing` | PASS | ✅ PASS | `Finished 'dev' profile in 1.79s`（含 pre-existing 5 条 warning） |
| `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-agent-stream` | PASS（48 测试） | ✅ PASS | `test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| `node scripts/check-core-boundaries.mjs` | PASS | ✅ PASS | `Core boundary check passed.` |
| `rg -i miniapp scripts/core-boundaries` | 0 hits | ✅ 0 hits | (empty output) |

**判定**：四道门禁 + 自检全 PASS，与 report §2.1-§2.5 一致（reviewer 独立复跑验证，非信任 report）。

### 2.2 改动集审计（约束 1 + 9）

- **零夹带**：git status 仅本批授权 10 文件 + 3 并行 session 文件（`.opencode/model-capability-notes.md` / `memory/northhing.md` / `.handoffs/handoff-g2-t9-2026-08-07.md`），无意外文件改动。
- **零无关格式化**：所有 diff hunks 均为精准文字删除（MiniApp 相关字符串），无空白调整、无 reformat、无注释微调。`git diff HEAD --shortstat --` 全部授权修改文件均落在合理 +N/-N 区间（典型 1 文件 1 处改动）。
- **UTF-8 无 GBK 双重编码**：本批所有中英文改动（含 AGENTS-CN.md、backend-roadmap.md、tech-debt-ledger.md、decision-register.md、tech-debt-cleanup-guide.md）中文均正常 UTF-8 渲染，无 GBK mojibake。报告本身也走 edit 工具写入（避免 PowerShell 编码陷阱）。
- **pre-existing GBK mojibake 隔离**：`docs/architecture/core-decomposition.md`、`docs/architecture/agent-runtime-services-design.md` 等历史文档含 GBK 双编码字符（`莽潞?domain` 等），但 `git diff HEAD --` 对这些文件为空——本批未引入新 GBK 问题，历史问题属另案。

### 2.3 工程纪律

- ✅ 实现者未自 commit（report §0 状态声明 + 工作区 git log HEAD 未推进 = `17eb4bb` 维持原值）。
- ✅ 实现者未碰并行 session 文件（diff 中无 `.opencode/model-capability-notes.md` / `memory/northhing.md` / `.handoffs/handoff-g2-t9-2026-08-07.md` 的改动——这三条的 M/?? 状态来自另一并行 session）。
- ✅ 不动三处 serde/wire 残留、不动 PCS-3 段、不动 guard_command_execution 本体——三项红线全严守。
- ✅ 文件改动严格在 brief 授权集内，未越界、无夹带、无关格式化。

---

## 3. Findings

### Critical（0）

无。

### Important（0）

无。

### Minor

#### M-1 — 行数预估偏差（brief 起源，非本批实现引入）

- **位置**: `.superpowers/sdd/task-t2-2o-brief.md:20` 与 `.superpowers/sdd/task-t2-2o-report.md:12`
- **现象**: brief/report 均声明 MiniApp/ "7,953 行"；`git diff --stat HEAD -- MiniApp/` 实测 **8,684 行**。
- **细分差异**：
  - `Skills/miniapp-dev/`：brief 696 / 实测 891（差 +195）
  - `Demo/git-graph/`：brief 6,028 / 实测 6,418（差 +390）
  - `Demo/icon-design-system/`：brief 1,229 / 实测 1,375（差 +146）
  - 总差 +731 行
- **影响**: 零功能影响。MiniApp/ 整删本身完整、37 文件清单一致；仅数字摘要偏小。
- **性质**: brief 估算偏差（recon 时未 `wc -l`），实现者忠实照抄到 report。
- **建议**: 报告里若要修正数字，可在下批（若仍有 miniapp 残留收口）一并订正，或在本批 commit message 内回写 "实测 8,684 行（brief 7,953 为估算）"。**非阻塞**。

#### M-2 — `git status --short` 输出截断（报告 §2.6）

- **位置**: `.superpowers/sdd/task-t2-2o-report.md:246-248`
- **现象**: 报告验证 6 的 git status 列出 `?? .superpowers/sdd/task-t2-2o-brief.md` 但省略了同批产生的 `?? .superpowers/sdd/task-t2-2o-diff.md` 与 `?? .superpowers/sdd/task-t2-2o-report.md`。
- **影响**: 零功能影响，仅报告自述的完整性瑕疵。
- **性质**: 报告输出截断，未影响本批改动正确性判定。
- **建议**: 下次报告复制 git status 时用 `git status --short | rg -v "^$"` 取全行，避免手抄漏行。

#### M-3 — P-14 行措辞扩展（轻微超出 brief 字面）

- **位置**: `docs/status/decision-register.md:40`
- **现象**: brief 仅要求 "补执行回链（T2-2 M1-M5，commits 区间，本批 commit 占位）"。实现者实际写法："**MiniApp 子系统整删**（内置六套资产 + 宿主 ≈11.2k rs/test + 55.9k 资产 + 顶层 8k 行，已执行：T2-2 M1-M5，commits a930c93..T2-2o）"。
- **判定**: 该扩展合理（执行回链本身就是事实陈述），且 P-14 行原决策文本（"permission_policy 默认拒绝语义删除前提炼进 PCS 权限框架"等）保留不动。**未违反 brief 红线**，属允许的事实细化。
- **影响**: 零。
- **建议**: 无需修。

---

## 4. Cannot verify 清单

无。所有 brief 约束均已通过 reviewer 自验命令核验：

| 约束 | 自验手段 | 状态 |
|---|---|---|
| 1 授权文件集 | `git status --short` + 授权清单比对 | ✅ |
| 2 三处 serde/wire 未动 | `git diff HEAD --` 三文件 = 空 | ✅ |
| 3 骨架不变量 | 读 `AGENTS.md:176`、`AGENTS-CN.md:137` 完整行 | ✅ |
| 4 roadmap 收口 | 逐行读 `backend-roadmap.md` 与 diff 对照 | ✅ |
| 5 P2-21 条目 | 读 `tech-debt-ledger.md:232-237` 全文 | ✅ |
| 6 tool_call_accumulator.rs | 读 diff hunk + `rg -ni initminiapp|markdown viewer` = 空 | ✅ |
| 7 四道门禁 | reviewer 独立跑 4 命令 | ✅ |
| 8 终扫归类 | reviewer 独立跑 `rg` + 命中归类 | ✅ |
| 9 中文 UTF-8 + 无夹带 | 读全 diff 无 GBK mojibake | ✅ |

---

## 5. 一句话总结论

**SPEC PASS + QUALITY PASS**：本批严格按 brief 收口 MiniApp/ 顶层目录（37 文件、实测 8,684 行整删）、文档层 9 文件按红线对齐收口、AGENTS.md/CN 骨架不变量严守、三处 serde/wire 残留原封不动（已登记 P2-21）、PCS-3 语义段原样保留、四道门禁 reviewer 独立复跑全 PASS、代码面零新增残留；仅发现 3 条 Minor（行数预估偏差 + git status 截断 + P-14 措辞轻微扩展），均非阻塞，建议下批或 commit message 顺手订正。

**最终建议**：通过。可提交本批收口 commit。
