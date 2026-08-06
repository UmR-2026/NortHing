# Handoff — 后端 follow-ups Wave1 收官（2026-08-06）

接手者：从 `E:\agent-project` 启动的编排者会话。
前序交接：`2026-08-05-backend-followups-midround-handoff.md`（轮启动基线/陷阱仍有效，本文只记增量并**取代**其 §3 恢复点与 §7 基线）。
事实源：计划 `.superpowers/sdd/plan-2026-08-04-backend-followups.md`；台账 `.superpowers/sdd/progress.md`（Backend Follow-ups Round Ledger 段，B1-B4 + 终审五行）；债清单 `.superpowers/sdd/tech-debt-followups.md`（FU-1..FU-5 **全部 resolved**）；终审 `.superpowers/sdd/wave1-final-review.md`。

## 1. 一句话结论

**Wave1（B1-B4）全部完成并通过分支终审（SPEC PASS / QUALITY PASS，0 Critical / 0 Important）**；triage 已裁定并执行（2 项合并前修补已提交，4 项登记 tech-debt-ledger P2-15..P2-18）；用户决定**先不并 main**（main 工作区被 growth-core 线另一 session 占用 + 已过宵禁）。分支状态干净，随时可合。

## 2. 分支与 commit 表

分支 `fix/backend-followups-0804`，worktree `northing/.worktrees/backend-followups-0804`，merge-base main `41695f5`，HEAD **`6267fb1`**，工作区**干净**（无未跟踪/未提交）。

| commit | 内容 | 审查状态 |
|---|---|---|
| `d4b11b5` + `808ed65` | B1/FU-1 MCP 用户级配置写 fail-closed + tokio Mutex 串行化 RMW + 3 并发测试 | r2 双 PASS |
| `4f45f14` | B1 证据链 + ledger | docs |
| `7a4bdca` | B2/FU-2 LSP uninstall 按解析 language keys 停服 | 一轮双 PASS |
| `57e4672` | B2 证据链 + ledger | docs |
| `b0bfe43` | **build fix**：keyring v1 feature 使能（P1-C3 遗留，见 §4） | 随 B3 审查 PASS |
| `755a503` | B3/FU-3+FU-4 settings 公共 load 持锁 + dead wrapper 删除 | 4/4 判决 PASS |
| `6868377` | B3 证据链 + ledger | docs |
| `50b0f44` | B4/FU-5 `initialize_global` 双检锁 + `init_once_with` helper + 2 测试 | 一轮双 PASS |
| `e6be249` | B4 证据链 + ledger（Wave1 done） | docs |
| `8f921cc` | 终审 triage 合并前修补（B1-M2 测试数 "+7"、B3-M1 过期注释） | ling 机械单，`cargo check -p northhing` 通过 |
| `8fa0ed6` | 终审证据链 + ledger 终审行 | docs |
| `6267fb1` | tech-debt-ledger P2-15..P2-18 + AGENTS.md 家规 6 | docs |

## 3. 接手第一步（恢复点）

Wave1 已无待办任务，只剩**合并动作**：

1. `git -C northing/.worktrees/backend-followups-0804 log --oneline -3` 复核 HEAD=`6267fb1`。
2. **确认 main 工作区是否仍被 growth-core 线占用**（`git -C northing status --short`）。若仍有其未提交产物 → 先问用户，**不要**在其上做 merge。
3. 可合时：`git -C northing merge --no-ff fix/backend-followups-0804`，再跑 §5 三条回归命令。
4. 合并后：`tech-debt-followups.md` 已全 resolved，无需再动；写收官 handoff。
5. 之后是 **Wave 2**（计划 §3 的 B5 relay 批 / B6 services+assembly 批 / B7 desktop+lsp 批）与 **Wave 3**（§4 决策项，需用户拍板）。

## 4. 用户决策记录（本次会话）

1. **P1-C3 过程性缺陷**（2026-08-06）：选"登记债项 + 加流程关卡" → `docs/status/tech-debt-ledger.md` **P2-15**（含根因：报告验证段不完整 + handoff 沿用未实测基线）+ `AGENTS.md` 家规 **6**（merge to main 前 `cargo check -p northhing` 必须通过；handoff 不得沿用自己未实测的验证基线）。CI 层面的强制仍是 open。
2. **合并时机**（2026-08-06）：选"先不合并，写 handoff 收工"——理由：已过 03:00 宵禁 + main 工作区被并行 session 占用。
3. **模型选派**（2026-08-06）：用户指定 **deepseek v4 flash 优先**，**轻量机械式任务用 ling**；ark / volcengine(glm) 线无额度不可用，**qwen 额度紧勿用**，judge 用 **m3**。

## 5. 验证基线（本轮实测，取代前序 handoff §7）

| Crate | 命令 | 基线 |
|---|---|---|
| northhing-core | `cargo test -p northhing-core --features product-full --lib` | **1141 总**（1140 passed + 1 ignored；B2 后 1139 + B4 新增 2） |
| services-integrations | `cargo test -p northhing-services-integrations --features product-full` | 212 passed |
| northhing（desktop） | `cargo check -p northhing` + `cargo test -p northhing --lib` | check 通过（19+5 既有 warning）；**118/118** |

cargo 前缀：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`。`cargo check --workspace` 仍被 embed-resource 3.0.11 阻断（非代码，交 CI）。

## 6. subagent 运维（本次实测，回填 `.opencode/memory/facts/models.md`）

- **`coder-dv4f`（opencode deepseek-v4-flash-free）首次实证 ✅**：B4（并发修复 + 测试设计，中等复杂度单）一次成型，DONE 汇报与磁盘完全一致、无造假；自主判定方案 A 不 hermetic 并给出可核证据后走方案 B（抽 helper 测试），judge 与终审均认定等价。**取代 CORE 中"dv4f 空汇报暂勿派"的旧结论**（该结论基于更早一次任务）。
- **`coder-ling`（opencode ling-3.0-flash-free）首次实证 ✅**：两行文字修补机械单，严格只改 2 文件 2 行、未越权、跑了 `cargo check -p northhing` 并贴输出。硬约束 + 精确字符串给足时可靠。
- **`judge-m3`**：既做 B4 单任务一审，又做分支终审。终审独立盘点 6 把锁的拓扑与调用链（含 desktop 锁内是否 await core 侧锁）、独立核 Cargo.lock +4 包依赖来源、逐项裁定 12 条 triage —— 深度足够，**继续作为 judge 首选**。
- **不可用/慎用**（本时段实测）：`sensenova/deepseek-v4-flash` **模型不存在**（`coder-sn` 报 Model not found）；`volcengine/glm-5.2` **模型不存在**（`judge-glm` 报错，正确 provider 前缀是 `volcengine-agent-plan`）；ark 线用户告知无额度；qwen 额度紧（用户指示勿用）。

## 7. 已知陷阱（累积，仍有效）

- PowerShell 写非 ASCII 会 GBK 双重编码 → 记忆/文档一律用 edit/write 工具，禁 `Set-Content`。派 subagent 时也要在任务书里写明。
- 不裸 `cargo fmt`；只 `pnpm run fmt:rs`。
- desktop 测试不得碰真实 `~/.northhing/config/app.json`（Windows `dirs::home_dir` 不可重定向）→ 走 `_at`/`_locked` 变体 + tempdir。
- tokio Mutex 非重入：settings 公共 load 持锁，`_at` 必须无锁。
- `io.rs` 的 `mod io_tests;` 解析到 `io/io_tests.rs`。
- worktree 可能缺 gitignore 生成物 `generated_locale_contract.rs` → `node scripts/generate-i18n-contract.mjs` 补齐，副产物勿入 commit。
- 进程级 `OnceLock` 全局单例（`GLOBAL_AI_CLIENT_FACTORY` 等）在 lib 测试二进制内共享：初始化它会让 `subagent_ports` 的 spawned task 在有真实凭据的机器上发起真实 LLM 调用 → 相关并发测试要走可测 helper 而非本体（B4 先例）。

## 8. 并行线与禁区（不变）

- growth-core 线由**另一 session** 推进，main 工作区有其大量未提交产物（`task-a*`、progress.md Growth 段、`model-capability-notes.md` 等）——**不碰、不 commit、不纳入本分支**；合并 main 前必须先确认它的状态。
- `northing/memory/` + `northing/.graph/`（产品/其它 agent 记忆）与 `frontend-redesign-*` 两文件（用户侧前端工作）勿碰。

## 9. Suggested skills（下个会话）

- `verification-before-completion`（合并前回归扫）
- `subagent-driven-development`（Wave 2 的 B5/B6/B7 循环）
- `writing-plans`（若 Wave 3 决策项要落设计任务）
- `handoff`（Wave 2 中断或收官时）
