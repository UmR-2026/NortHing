# W5 全波终审 Brief：Dioxus 壳审计修复（2026-08-28）

只读审查。不改代码、不 commit。仓库根：`E:\agent-project\NortHing`（分支 main）。

## 范围与证据

- 审查范围：`86ab479..f680cf6`（8 commits = 4 实现 + 1 审查修复 + 3 docs/sdd）。wave-base = W5-1 派发前 HEAD，HEAD = W5-4 实现 commit。
- **代码 diff 包（已排除 `.superpowers/`，唯一审查对象）**：`.superpowers/sdd/review-w5-final-86ab479..f680cf6.diff`（9 文件，+733/-148）
- name-status：`.superpowers/sdd/review-w5-final-namestatus.txt`
- 计划：`.superpowers/sdd/plan-2026-08-28-w5-dioxus-shell-fixes.md`（任务定义 + 编排者裁定，逐字为准）
- 壳级审计来源（F1/F2/F4/F5/F6 原文）：`.superpowers/sdd/w4-2-dioxus-shell-review.md`
- 台账：`.superpowers/sdd/progress.md` W5 四行（每任务判决 + Minors 记录）
- 各任务 brief/report（同目录）：`w5-1-quit-graceful-{brief,report}.md`、`w5-2-event-channel-tiering-{brief,report}.md`、`w5-3-onboarding-persist-provider-{brief,report}.md`（⚠️ brief 文件当前未被 git 跟踪，直接在磁盘读）、`w5-4-partialeq-mutex-{brief,report}.md`

## commit 地图

| commit | 任务 | 内容 |
|---|---|---|
| `de60a0b` | W5-1 (F1) | quit_shell 弃 process::exit，走优雅退出链 |
| `289a2de` | docs | W5 wave 开启（计划 + ledger） |
| `87cb1f4` | W5-2 (F2) | kernel 事件桥分级：TextChunk 有损预算 + 控制事件保证投递 |
| `86803d7` | docs | W5-2 ledger |
| `fafc1fa` | W5-3 (F4) | onboarding 持久化 provider 配置 + 设默认 |
| `21f9345` | W5-3 fix | 单测改 MockKeyring 隔离（一轮审查 Important 的修复） |
| `2ebc8c3` | docs | W5-3 ledger + W5-4 brief |
| `f680cf6` | W5-4 (F5+F6) | ModuleAppProps 结构 PartialEq + entry.rs Mutex→watch |

## 判决要求

产出**双判决 + 合并裁决**：
- `SPEC: PASS/FAIL` —— 对照计划 4 个任务的 Spec 逐条判定，给 file:line 证据
- `QUALITY: PASS/FAIL` —— 跨任务集成正确性、边界守卫、可维护性、测试有效性
- 裁决：`CAN MERGE` 或 `NEEDS FIXES`

Findings 分级 Critical / Important / Minor，每条带 `file:line` 证据。**每个发现先读源码全文再判**（diff 是切片，上下文在 `src/apps/desktop/src/`）。

## Global Constraints（逐字复制自计划 §Global Constraints，逐条核对）

1. 分层边界：改动只在 `src/apps/desktop`；其它 crate 零改动。
2. 日志纪律：新增日志一律英文、无 emoji，带关键上下文字段。
3. 并发测试绑定（家规④）：触碰 tokio 任务生命周期/取消/关闭顺序的改动必须随附至少一个自动化测试；无法自动化处由编排者在 brief 里显式豁免并说明理由。
4. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/`。
5. rot-budget：不上调任何 ceiling；不新增 >800 行文件。
6. 验证最小集：`cargo check -p northhing` + 本任务指定的聚焦测试；命令与输出原文进 report。
7. commit 规则：每任务恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物。
8. 不新建无 owner 抽象；优先复用既有通道/设施（brief 里已点名）。

## 终审特殊关注点（跨任务集成，重点）

1. **W5-1 关闭链 × W5-4 watch 化的交互**：两者同触 `entry.rs` / `registry.rs` / `app.rs`。核查：窗口进入 Closing 后 watch `send_modify` 是否可能写到已关闭接收端；`mark_all_closing_targets` 原子迁移与 watch 发送的时序；关闭链任一环节在 watch 化后是否引入新 footgun。
2. **W5-2 分级机制声明对账**：声称"app.rs 消费端零改动、recv 同签名"——核对 diff 是否属实；TextChunk AtomicUsize 预算的计数准确性（过计/漏计窗口是否如台账所述仅理论）；控制事件 unbounded 直通的背压风险定级。
3. **W5-3 生产路径无污染**：MockKeyring/KeyringBackend 注入是否只在测试路径；`app_state/settings/sync.rs` 与 `tests.rs` 的改动是否全部归属 F4 范围；keyring account 命名是否遵循 `app_state/settings/keyring.rs` 既有约定；三失败臂（测试失败/persist 失败/设默认失败）UI 显式报错是否如 report 所述。
4. **W5-4 行为零变化声称**：`PartialEq` 从恒 true 改为结构比较——重渲染触发条件实际变多还是不变（plugin_id+gen 身份比较 vs 旧恒 true 永不重渲染，确认不会引入渲染回归）；`send_modify` 塌缩原 Mutex+send 两步是否等价。
5. **退出链路完整性（W5-1 核心）**：`rg "process::exit" src/apps/desktop/src` 应仅剩 init 失败路径；✕ → room+module 窗关闭 → `launch()` 返回 → main.rs `shutdown_tx.send(())` + `shutdown_mcp_servers()` 每一环 file:line 走查。
6. **累积 Minor triage 队列**（逐条给"修一记一 / accept-and-close / defer-with-owner（指名 owner）"建议 + 一句理由）：
   - W5-1-M1 report 缺 test 执行输出原文（证据纪律）
   - W5-1-M2 走查行号 off-by-one×2
   - W5-1-M3 room 双关闭冗余宜加 ponytail 注
   - W5-1-M4 WindowDropGuard 复用声称为未验证声明
   - W5-2-M1 counter 过计理论窗口
   - W5-2-M2 `pending_text_chunks` 可降 `pub(crate)`
   - W5-2-M3 丢 chunk 用 debug! 运营不可见
   - W5-2-M4 控制侧 unbounded 无显式上限
   - W5-3-M1 `infer_provider_wire_format` URL 启发式脆弱（proxy 路径含 anthropic 字样会误分类）
   - W5-4-M1 send 丢弃语义可加注释
   - W5-4-M2 PartialEq 可加 rx-Arc 变体用例
   - W5-4-M3 registry.rs 678/800 接近警戒线
7. **文档/台账一致性**：`progress.md` W5 四行的 commit 范围与本包一致；家规 2（doc sync）有无应翻未翻的条目；w5-3 brief 未跟踪这一 housekeeping 缺口记 observation。
8. **合并前阻塞清单**：列出合并 main 前必须先做的事（若有）。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过。QUALITY 三个防腐必查项：
- **复用核查**：report 声称的复用/无既有实现，抽查独立验证；复制既有能力而不复用 = Important 起评。
- **无 owner 抽象**：diff 中每个新增抽象必须绑定当前真实消费方；投机性抽象 = Important 起评。
- **预算闸**：diff 若触碰 `scripts/rot-budget.json` 且是上调 ceiling/放松规则，除非有用户拍板原文，一律 SPEC FAIL。
- **god-file 观测点**：diff 触及的超 800 行登记文件，附一句健康度观察。

**Cannot verify from diff**：无法从 diff 判定的项单独列出，禁止猜。发现与计划原文冲突时（plan-mandated），不自行裁决，列出并交编排者。

## 输出

完整判决书用 write 工具写入 `.superpowers/sdd/w5-final-review.md`（含双判决、findings 列表、Minor triage 表、Cannot-verify 清单、合并裁决），返回消息只给：裁决 + C/I/M 计数 + 一句话理由。

## 验证纪律

工具链实测可用（MSVC cargo、rg），需要时可自行只读核查（grep/read/cargo check），禁止盲目全信报告。不重跑 implementer 已跑过且输出对得上 diff 的测试。
