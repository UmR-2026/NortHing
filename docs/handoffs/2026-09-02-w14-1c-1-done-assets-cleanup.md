# Handoff — 2026-09-02（凌晨）：资产整理日 + W14-1c-1 收官

> 上一份：`2026-09-01-w14-1e-done-w15-1-next.md`。本份覆盖：memory 全量整理、项目资产整理、用户改判（E-13 翻转）、W14-1c-1 实施与验收。

---

## 0. 一句话状态

HEAD `9cd72f4`（+ 后续 docs commit 见 §1），**推送状态以 `git status -sb` 当场为准**。W14-1c-1（A 类 5 测试进程隔离迁移）**已完成并验收通过**；按用户 09-02 改判，**W14-1c 余下切片继续，W15 顺延**。

---

## 1. 本段 commit（倒序）

| commit | 内容 |
|---|---|
| `9cd72f4` | doc(hidden) 标记 W14-1c-1 的 3 处测试专用可见性（fix-forward，fixer=gemini-37-flash-agy） |
| `3e085f1` | W14-1c-1：A 类 5 测试迁独立 tests/*.rs（implementer gemini-37-flash） |
| `1d7e5ad` | E-13 翻转登记 + W14-1c-1 brief 入库 |
| `e151b54` | AGENTS×2 补 agent-dispatch + surfaces.md 去 Slint |
| `f7379cb` | sdd 预防性归档 372 件（388/400 → 16，cap 危机解除） |
| `fab148d` | D1-D6 入 decision-register（E-10..E-15） |
| `0ff4903` | handoffs 归档 19 篇（仅留最新 2 篇） |

memory 仓同日 6 commits：GBK 修复 / 蒸馏纠过期 / 热路径瘦身（BOOTSTRAP 6.3k→2.8k）/ 决策回填 / 台账切分（32k→12k+档案段）/ LEARNINGS +2。

---

## 2. 用户 09-02 改判（已登记，勿再走旧顺序）

- **E-13 翻转**：W14-1 测试隔离线**先于** W15（原 D4"单对话优先"被用户本人推翻）。
- **W15-1 依赖准入已拍板**：允许引入 markdown crate；**选型是技术细则 → 走独立仲裁闭环，不问用户**。
- 红线重申（W15-1 开工时）：本地渲染**禁 raw HTML 注入**（模型输出不可信）；css.rs 790/790 零余量，样式只许新独立 CSS 文件或复用 class。

## 3. W14-1c-1 验收要点（含一条重要机制教训）

- 5/5 迁移忠实、测试数三仓守恒（147/1071/22）、串行+并行双绿、rot 绿且 let_underscore 388→371 反降。
- **仲裁 C2 约束有技术缺陷**：`#[cfg(test)]` 对 `tests/` 集成测试不可见（集成测试链正常 lib 构建）→ 迁出即需无条件 `pub`。实现者撞墙后透明披露（report §3），裁定 fix-forward 不打回：`#[doc(hidden)]` 标记 + 仲裁书补遗修订规则（w14-1b-arbitration.md 末）。
- 仲裁另两处误估被 brief 预检纠正：desktop 已有 lib.rs（无需拆 lib+bin）；terminal accessor 已 pub（无需 seam）。
- **「Task cancelled」≠ 没跑**：派发返回 cancelled 但 commit+report 已在盘上。派发返回异常后先 `git log`/`git status`，不凭返回状态定论。

## 4. 队列（当前有效顺序）

1. **W14-1c 余下切片**（仲裁书 `w14-1b-arbitration.md` 步骤 4-11，含补遗规则）：
   - 切片 2 = B-1 迁移 + B-2 seam（步骤 4/5/6，≈1.25 人天；含 AgentRegistry `unregister_for_test` 永久 API）
   - 切片 3 = init gate 局部重写（步骤 7，并发敏感）
   - 切片 4 = C/D 锁纪律扫描 + E 类余 5 条（步骤 8/9/10）
   - 切片 5 = CI 双轨 + 5 轮连绿（步骤 11）
2. W15-1 Markdown 渲染（依赖准入已批，选型仲裁 → 实现）
3. W15-2/3 → W16 摆设做真 → W17 多会话（后置）→ W18 命令风险门（设计已出，可并行）
4. W19-W23 见 plan §5。

## 5. 工作区/资产现状（今日整理后）

- worktree 零悬挂；分支 7 支（growth 线 6 + spike 1，D3/D5 拍板保留；`feat/consult-room-slint` 已删——已全并入 main）。
- stash 全清（4 个 7 月死 stash 已删）。
- `.superpowers/sdd/` = 17 项（400 cap 安全）；旧产物在 `docs/archive/sdd-artifacts/`（639 件）。
- 根目录已清：Slint 时代截图、`.opencode/sdd` 7 月遗物、`.ohmyagent/`、`.cluster/` 已删（GLM 系集群 playbook 评估后判定无参考价值）。
- memory 仓：热路径 = BOOTSTRAP(2.8k) + CORE(6.9k) + index + 最新 handoff×2。

## 6. 环境/雷区（沿用，全部仍有效）

- cargo 必走 `rustup run stable-x86_64-pc-windows-msvc`；输出重定向用 `cmd /c`（禁 PowerShell 管道）；重复测试先 `--no-run` 再直跑二进制；长任务 PTY；跑完查僵尸进程。**先读 skill `long-running-shell`**。
- rot 闸零余量：`let _ =` 371/388（W14-1c-1 腾出 17）、`css.rs` 790/790、`unix_epoch_inline` 69/69。
- 推 GitHub 先试直连，失败上 clash `127.0.0.1:7897`；SSH 始终不可用。
- 宵禁 03:00。

## 7. 下 session 第一件事

1. `git status -sb` 校准。
2. 派 **W14-1c 切片 2**（B 类 seam 批）：brief 从仲裁书步骤 4/5/6 + 补遗规则出发，预检时逐条磁盘核实 file:line（仲裁 §6 自己声明了 4 处未核实项）。
3. 切片 2 完成后依次 3/4/5；全波完成后 W15-1（先派 markdown crate 选型仲裁）。

## Suggested skills

- `long-running-shell`（任何构建/测试前）
- `subagent-driven-development`（切片派发）
- `verification-before-completion`（验收口径）
- `handoff`（收口）
