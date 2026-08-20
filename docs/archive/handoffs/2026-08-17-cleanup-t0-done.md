# Handoff 2026-08-17：大整理收官 + T0 批完成 + 决策全闭环 → 0.3a 开跑

> 上一篇：2026-08-14-wave2-session-b-done（Wave 2 收官）。本篇之后权威状态以本篇 + 登记册 + roadmap 为准。

## 需求基线状态

- **产品论题 v1.4**（`docs/product-thesis.md`）：六步验收环（P-17 补第六步「用 PCS 给它装一个插件，看它用起来」）。
- **决策登记册**（`docs/status/decision-register.md`）：P17/T9/E8/R4+1组合，**全部生效，无悬置项**。关键新条目：T-02 命名修正为 **NortHing（诺森）**；R-5 key 不轮换（用户拍板：项目专用 key 泄露无影响）；P-16 PCS 权限=用户安装时批准；P-10 遥测边界=v1 数据不离机；E-07 0.3 拆 a/b；E-08 M 线归 growth session。
- **执行序**：`docs/architecture/backend-roadmap.md` T0-T6；0.3a = T0✅ + T2-1 + T2-2 + T1×5；0.3b = T2-9 + PCS-1/2 + T2-10。
- **外部审计不定期**（用户 08-17）：所有"等审计"项不依赖审计周期，验证摊入执行点。

## 今日已完成（commit 表）

northing 仓库（main 分支，未 push；repo 不可公开推送状态未变）：

| commit | 内容 |
|---|---|
| `5160141` | docs: 208 篇旧 handoff 归档 docs/archive/handoffs/ + sdd .gitignore（*.patch/*.diff） |
| `b598cdd` | decisions: T-02 修正 NortHing + R-5 + T0-3 立项 |
| `6f6b0f3` | decisions: PEND-1 删 relay（并入 T2-2 ≈35k 行）+ E-07 0.3 拆分 + E-08 M 线归属 |
| `c65004a` | **T0-2**：run_script bash 条件反转修复（双判决 PASS/0 findings） |
| `e6d6386` | **T0-3**：品牌标记统一 NortHing（slint/installer locale/shared terms/README；双判决 PASS/1 Minor） |
| `25b486d` | sdd: T0-2/T0-3 台账行 |
| `eabc10a` | decisions: A1/A2/A3 → P-16/P-10/P-17，thesis v1.4 |

根仓库：`cf1cb08` 根 .gitignore（H-5 围堵）。记忆仓：`1e1ff4c`（当日决策全闭环指针）。

**磁盘**：E: 空闲 218.5 → 403 GiB（回收 ≈184.5 GiB：target 135G + target-msvc 7.3G + 6 个已合并 worktree ≈60.7G）。

## 进行中卡点 / 立即下一步

1. **用户重启 opencode/OpenChamber**（agent 变体文件改动需重启生效）。
2. 重启后第一件事：**探针 `google-vertex/gemini-3.7-flash`**（五件套变体已改指；`variant: high` 在 models.dev 注册表模型上是否生效未实证，探针时盯）。备胎：`google/antigravity-gemini-3.7-flash`（免费，已探针 ✅）。
3. 探针绿 → 开 **T2-1（CI 补齐）**，implementer = `gemini-37-flash` 首单实战。

## 队列（blocking 边）

| 序 | 单 | 阻塞关系 |
|---|---|---|
| 1 | **T2-1 CI 补齐**：去 exclude、test 扩面、kernel-api cargo tree 守卫入 CI、desktop check 强制门 | **前置障碍：i18n-contract 24 个测试 main 上预存失败**（冻结期漂移，非 T0-3 引入；T0-3 反而修好 1 个）。扩面前先清。blocks T2-2 |
| 2 | **T2-2 ≈35k 行删除**：remote 栈/MiniApp/judge_gate 适配层/relay/pcc/harness | blocked by T2-1（盲删 = P1-C3 重演）；**另两个前置**：① MiniApp `permission_policy` 默认拒绝语义提炼进 PCS-3 设计段落（编排者写文档，先于删码）；② UI 入口摘除（feature/配置/UI）需前端 session 配合 |
| 3 | T1 五项安全：T1-4 ComputerUse 接 guard / T1-5 出货默认确认门+P1-6 / T1-6 安装器三修 / T1-8 apps/server 修复 / T1-10 低危批量 | 在 T2-2 瘦身后的代码上做 |
| 4 | 0.3b：T2-9 冗余合并三批 → PCS-1/2 → T2-10 连续性自检 | blocked by 0.3a |

并行可行性：T2-2 的删除子批内部可拆串行小批；不与 growth/前端 session 的文件集相交（growth = agent_memory/growth crate；前端 = slint UI）。**三 session 撞点**：T2-2 前 UI 摘入口（前端）；TH-2 CI 门禁（growth 出策略、编排线接线）；TH-3 面板（growth 数据 + 前端 UI）。

## Subagent 运维变更（2026-08-17）

- **coder 主推 = `gemini-37-flash`** → 指向 **`google-vertex/gemini-3.7-flash`**（1M ctx/64k out/图文音视频+pdf/effort low-med-high）。⚠️ **按量计费（$0.75/$3.75 per MTok）**——但用户明示不用替 TA 省额度，按任务风险选档即可。
- 实证记录：T0-2 用 `general/deepseek-v4-flash-free_general`（免费档）一行修复干净落地；reviewer 用 `reviewer/gemini-36-flash_reviewer` 两单均双 PASS（T0-3 还抓到 report-diff 不一致的 Minor）。
- 运维注意：`pty_spawn` 本 session 调用失败一次（原因未查），长任务用 bash 长 timeout 替代可行；`decide_show_plan` 超时未响应一次（decision-ui 未实战验证，文本提问兜底可靠）。
- 根目录 `.cluster/`、`.ohmyagent/` 两目录来路不明，未动未 ignore，待用户确认。
- consult-room-build worktree（61.6G）= **前端 session 的代码**，保留勿动。

## Suggested skills（下个 session）

- `subagent-driven-development`（T2-1 起的任务循环）
- `verification-before-completion`（每个收口前）
- `requesting-code-review`（reviewer 派发）
- `systematic-debugging`（i18n 漂移清理若超预期）
- `handoff`（本轮结束时）

## 验证基线

- T0-2：cargo check -p northhing-core 44.07s pass（MSVC；PATH cargo 是 GNU 缺 gcc.exe，已知环境怪癖）
- T0-3：cargo check -p northhing 0 errors；installer type-check 0 errors；i18n:contract:test 13 pass/24 fail（**预存漂移**，T2-1 清理对象）
- ⚠️ target 已删：下次 cargo 是全量冷编译（15-20 分钟），T2-1 首单会把基线重新量出来
