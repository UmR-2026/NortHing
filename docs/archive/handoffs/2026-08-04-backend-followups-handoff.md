# Handoff — 后端 follow-ups 轮启动（2026-08-04）

接手者：从 `E:\agent-project` 启动的编排者会话（该位置才加载 `coder-qw`/`judge-qw`）。本文是**后端 follow-ups 轮**的启动交接，读它即可开干，无需回溯会话。
事实源指针：计划 `.superpowers/sdd/plan-2026-08-04-backend-followups.md`（`ae44334`）；债清单 `.superpowers/sdd/tech-debt-followups.md`；进度账本 `.superpowers/sdd/progress.md`；模型台账 记忆库 `facts/models.md`。

## 1. 一句话结论

用户 2026-08-04 拍板：继续**后端 follow-ups 线**（FU-1..5，Wave1 B1-B4），**本轮 subagent 优先 `coder-qw`/`judge-qw`**。此前两轮（后端审计 8+1 任务、P1 安全 C1/C2/C3）均已双判决通过并 `--no-ff` 并 main。**growth-core 是另一 session 的并行线，本轮勿碰**（见 §9）。

## 2. 为什么要换到 E:\agent-project 会话

启动本交接的会话运行在 opencode worktree（`.../worktree/<hash>/back`），**不加载** `E:\agent-project\.opencode\agents\` 的 `coder-qw`/`judge-qw`（探针 `judge-qw` → "Unknown agent type"）。只有从 `E:\agent-project` 启动的会话才加载这批项目级子代理。故用户换会话。

## 3. 基线与分支

- **基线**：main HEAD。写本文时 main 最后代码提交为 `f2a16c7`（= P1 安全合并）；仅本文作为 docs commit 后 main 前进一步（growth 在途证据属并行 session，不随本 commit）。**派单前 `git log --oneline -3` 复核真实 HEAD，勿信本行**。
- **分支**：`fix/backend-followups-0804`，worktree 隔离（参照 `.worktrees/p1-security-0804` 做法）。
- **计划**：`.superpowers/sdd/plan-2026-08-04-backend-followups.md`（Wave1 B1-B4 / Wave2 批量 / Wave3 决策项）。

## 4. FU 任务与锚点漂移警示（重要）

计划 §0 的 FU 锚点于 `8e43dc4` 复核过，但 main 已前进到 `f2a16c7`（P1 安全合并落在计划之后）。实测 `git diff --stat 8e43dc4 f2a16c7`：

- **FU-1 / FU-2 / FU-5 锚点文件未变** → 计划里 file:line 大概率仍准（派前快速复核即可）。
- **FU-3 / FU-4 锚点文件 `src/apps/desktop/src/app_state/settings/io.rs` 漂移 +78/-5**：P1-C3 keyring 迁移新增 `settings/keyring.rs`（349 行）、改 `io.rs`/`sync.rs`/`mod.rs`。计划里 `io.rs:31-49`/`:79`/`:135` 行号**已失效**，必须在 `f2a16c7` 重新定位（dedup-on-load 路径、`save_app_settings` dead wrapper 位置），并留意新增 keyring 路径是否与 FU-3 竞态/FU-4 删除面交互。

各 FU 的根因/修复方向/验证命令见 `tech-debt-followups.md` + 计划 §2。

## 5. Wave1 任务与验证

| 任务 | = | 内容 | 验证（focused -p） |
|---|---|---|---|
| B1 | FU-1 | `save_user_config` fail-closed（含同类 `delete_server_config`） | `cargo test -p northhing-services-integrations --features product-full mcp` |
| B2 | FU-2 | LSP uninstall 停服映射（plugin_id→language） | `cargo test -p northhing-core --features product-full --lib lsp` |
| B3 | FU-3+FU-4 | desktop settings 竞态收口 + dead wrapper（同文件合并单） | `cargo check -p northhing` + `cargo test -p northhing --lib settings` |
| B4 | FU-5 | `AIClientFactory::initialize_global` TOCTOU | `cargo test -p northhing-core --features product-full --lib` |

互不依赖，**串行派发**（本轮不并行多 implementer）。

## 6. 选派（本轮 qwen 优先）

- **implementer 用 `coder-qw`，任务级 judge 用 `judge-qw`**（用户 2026-08-04 指定）。qw 于 2026-07-31 用户解锁并实证（coder-qw R7 收尾单一次成、judge-qw 视觉评委合格）；历史 coder 达 lc 级、judge 达 m3 级。
- **派前先探针验 qwen 额度**（qw 曾在 07-27 周额度尽、07-29 停派）。无额度则回落：implementer `coder-lc`/glm-5.2，judge `judge-m3`。
- **分支终审用未参与单任务审查的独立模型**（如 glm-5.2），勿复用任务级 judge。

## 7. 执行纪律（逐字进每个 brief）

- 一次派发一个任务；brief 文件是需求唯一来源；implementer 不续会话、不粘历史。
- **不裸 `cargo fmt`**（两次污染前科）；格式手工对齐。日志 English-only、无 emoji。生产 `.rs` <800 行（>1000 必拆或 `// allow-god-file`）。触及并发竞态必带自动化测试。
- **解债 commit 必须同 commit 翻转 `tech-debt-followups.md` / `final-review.md` 对应项状态**（doc sync 硬规则）。
- implementer 只 commit 范围内文件；**收口核对 `git log` 而非信任报告**（越权 commit 前科）。
- 验证最小集 = 各任务 focused `-p`；`cargo check --workspace` 被上游 embed-resource 3.0.11 阻断（非代码），按 crate 验 + 交 CI。
- cargo 命令带 `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`；core 测试必带 `--features product-full`。
- 不重跑 implementer 已跑过的测试；review 双判决（spec + quality）缺一不算通过。

## 8. 验证基线（P1 合并后 main）

| Crate | 命令 | 基线 |
|---|---|---|
| northhing-core | `cargo test -p northhing-core --features product-full --lib` | 1134/1134 |
| relay-core + relay-server | `cargo test -p northhing-relay-core -p northhing-relay-server` | 49/49 |
| services-integrations | `cargo test -p northhing-services-integrations --features product-full` | 172/172 |
| desktop | `cargo test -p northhing --lib` | 98/98 |

## 9. growth-core 并行线（勿动）

growth-core 成长核心由**另一 session 并行推进中**（交接时实测：A1 APPROVED；A2-A5 复审于 23:01-23:31 在进行，`task-a2-report.md` 23:31 仍在写）。其在途产物——`.worktrees/growth-*`、分支 `feat/growth-core-0804`/`feat/growth-a1..a5`、`.superpowers/sdd/task-a*`/`task-g1-t1*`、`progress.md` 的 Growth-A1 段——**本后端轮一律不碰、不 commit、不合并**。
注意：`progress.md` 的 Growth-A1 段为 **GBK 乱码**（PowerShell 写非 ASCII 双重编码所致），属该 session 责任，由其自行修复，本后端轮勿代改。

## 10. 已知陷阱（省时间）

- **worktree 会话不加载 `coder-*`/`judge-*`**——要用 qwen 子代理必须 `E:\agent-project` 会话（本交接的换会话根因）。
- 裸 `cargo fmt` / `cargo fmt -p <大crate>` 会卷无关文件（两次前科）。
- `git status` 大量 M 多为 stat 噪声：先 `git update-index --refresh` 再 `git diff --stat`。
- implementer 越权 commit：收口核对 `git log`。
- **PowerShell 写非 ASCII 会 GBK 双重编码 → 一律用 edit 工具**（progress.md 的 Growth-A1 乱码即此坑）。
- subagent 空返回 ≠ 没干：改动可能已落盘，必须 git status/diff 独立取证；文字汇报不可信。

## 11. 接手第一步

1. `git log --oneline -3` 复核 main HEAD（勿信本文写的 hash）。
2. 探针 qwen 额度（派一个最小 coder-qw/judge-qw 单或按 facts/models.md 指引）。
3. 开 worktree + 分支 `fix/backend-followups-0804`。
4. **重定位 FU-3/4 锚点**（`settings/io.rs` 已漂移，见 §4）。
5. 逐单 B1→B4：brief → `coder-qw` → `judge-qw` 双判决 → 同 commit 翻债清单状态。
6. Wave1 全绿 → 分支终审（独立模型）→ `--no-ff` 并 main → 回归扫（对齐 §8 基线）。
