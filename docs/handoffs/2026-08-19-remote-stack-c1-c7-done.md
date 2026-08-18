# Handoff 2026-08-19 — T2-2 remote 栈整删 C1-C7 完成（停于 curfew，剩 C8 + 终审 + MiniApp 批）

> 状态权威源：`.superpowers/sdd/progress.md` T2-2 段（本文件只做导航，不复制其内容）。
> 上一篇：`2026-08-18-t2-1-t2-2a-done-e09.md`。

## 需求基线状态

- Roadmap T2-2（`docs/architecture/backend-roadmap.md:167`）的 **remote 栈整删部分（TH-4 + PEND-1）C1-C7 全部落地并 review clean**，每批双判决（spec+quality）通过、ledger 有行。
- **剩余**：C8 收口（纯文档/台账，无产品代码）→ remote 栈分支终审 → T2-2 另一半 MiniApp 子系统整删（独立大批，PCS-3 权限语义已提炼于 roadmap:190 §PCS-3，可直接作为删码依据）。
- 批次划分权威文档：`.superpowers/sdd/task-t2-2c-recon.md` §建议删除批次划分（C1-C8）。

## 已完成（commit 表，均在 main，顺序即依赖序）

| 批 | commit | 一句话 | artifacts（brief/report/review/diff 同名在 `.superpowers/sdd/`） |
|---|---|---|---|
| C1 | fa88342 | core `service/remote_connect` 48 文件 + SAR remote 适配器摘除，-13,843 行 | task-t2-2c-* |
| C2 | 02c6520 | agentic `remote_file_delivery` 整链，25 文件 -173 行 | task-t2-2d-* |
| C3 | 0bc8d81 | services-integrations `remote_connect` + `remote-connect` feature + 10 orphan deps，-7,637 行 | task-t2-2e-* |
| C4 | 46bbf68 | contracts 修剪：remote.rs 整删 + 6 项 wire 词汇（brief 显式授权清单） | task-t2-2f-* |
| C5 | f6a011b | relay-server + relay-core 整删 + relay i18n 面（从 C7 划入） | task-t2-2g-* |
| C6 | 646f93d | `src/mobile-web/` 整删 + 构建管道摘除（package.json/dev.cjs/ci.yml 等） | task-t2-2h-* |
| C7 | d16b037 | mobile-web i18n 契约面摘除（locales.json + 三脚本 + baselines） | task-t2-2i-* |
| sdd 收尾 | 8fbec0d | ledger + artifacts 最后落盘点 | — |

**全程零损伤证据**：SSH（remote_ssh 模块 / remote-ssh feature / remote_connection_id / lookup_remote_connection*）每批 judge 独立复核；`DialogTriggerSource::{RemoteRelay,Bot}` 保留（T5 协议客户端重建时的词汇）；`RemoteSsh` 变体与 `remote_connection_id` 字段保留。

## 进行中卡点

- 无。工作区干净（仅剩并行 session 的 `memory/`、`.opencode/` 未提交改动，勿动）。
- 停手原因：家规 5 coding curfew（03:00 后不做编码），用户休息时停于 06:20，用户未对继续方式拍板（decide_pick_one 超时）。

## 队列（下一步，按序）

1. **C8 收口**（纯文档/台账，无产品代码；recon 已做，要点如下）：
   - `docs/status/tech-debt-ledger.md`：P1-4（mobile-web re-pairing 无引导）与 P1-7（embedded relay 开放模式）翻 resolved（随删除关闭，roadmap:118 已声明）；D-2 同查。
   - `docs/architecture/backend-roadmap.md`：:118 行标执行完毕；:167 T2-2 行标注 remote 栈部分 done（MiniApp 部分仍 active，别整行划掉）。
   - README.md / CONTRIBUTING.md 的 relay/mobile-web/remote_connect 提及摘除。
   - Minor triage（逐条见 progress.md T2-2 各行）：M-c-1 core/Cargo.toml:124,129 stale "Remote Connect" 注释（dep 已删注释残留）；M-c-2 SAR 测试名 `..._remote_control_port` 无实体（cosmetic）；M-c-3 sar_dispatch runtime_ports import 复核；M-f-1 session_workspace.rs:1 模块 doc 残留；M-g-1 i18n-audit.mjs `collectConfirmedUnusedKeys` 空函数+死调用；M-g-2 server/README.md 3 条 relay-server 悬空链接（server frozen，建议留到 unfreeze）；M-h desktop-tauri orphan workspace 注册（pnpm-workspace.yaml 两行，磁盘无目录——独立决策项，不随本批）。
   - 建议：单批派一个 implementer（文档编辑密集但机械），judge 走轻量审查。
2. **remote 栈分支终审**：review-package 用 merge-base（T2-2a 之前）..HEAD 全量视角；派独立 reviewer；findings 一个 fixer 带完整清单。
3. **MiniApp 子系统整删**（T2-2 另一半）：内置四件套 + 宿主 host_routing/bridge/manager/契约 ≈8k 行；PCS-3 语义已固化；注意 T2-2a 侦察发现 tool-provider-groups/harness 的活消费教训——先 recon 再派。

## 环境/运维事实（下一 session 必知）

- **cargo 一律 MSVC wrapper**：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`（仓库目录 override 是 GNU，`cargo +toolchain` 不可用）。
- **i18n 工程 frozen 且本快照不可跑**：`src/web-ui` 缺席 → i18n:audit / i18n-contract.test 失败是 pre-existing；**i18n-audit.mjs:503 有 pre-existing mojibake 语法级损伤**（文件本就无法 parse，T2-2g 双向实证；与 dev.cjs:99/105 同家族）。解冻时一并修。修改这些脚本时的判据：`node --check` 报**同一** SyntaxError（行号前移）= 未扩展损伤。
- **gemini-3.7-flash 静默失败模式**：本轮 3 次零产出/半产出的空返回（网络不稳时段）。处置：不硬重试 → 基线验证间隔数分钟 → 重派或同 task_id 续派（续派会接管工作区已落盘的部分编辑，实测流畅）。
- **judge-m3 agent type 未注册**：审查位用 `minimax-m3`（本轮 5 场审查全合格，含抓出 2 个真实 Important）。可选位 `reviewer/gemini-37-flash_reviewer`。
- **模型台账**：`.opencode/model-capability-notes.md` 2026-08-19 条目（该文件被外层 repo gitignore，纯本地）。
- 测试/文档里的 `computer://` 残留 = 0（C3 已清）；contracts 层 `RemoteConnect` 残留 = 0（C4 已清）；剩余 "remote" 词命中均为 SSH 语义或注释。

## Suggested skills

- 续作 C8：`subagent-driven-development`（brief→implementer→reviewer 循环）。
- MiniApp 批启动前：`writing-plans` 或直接在 recon 基础上拆批；参考 C1-C7 的 brief 模板。
- 下次停手/交接：`handoff`（本文件即模板）。
