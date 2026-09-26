# W26 Wave Final Review（波级终审，独立 m3）

- 波窗：`3e9f4b5..5848bf5`（BASE = 波前最后 docs commit「W26 five briefs ready」；TIP = 5848bf5 = 当前 HEAD，其后无新 commit）
- 波级包：`.superpowers/sdd/review-packages/w26-wave/`（manifest.json + review.diff 3862 行）
- 输入五单终判：W26-1 APPROVE 0C/0I/1M / W26-2 AWC 0C/0I/1M / W26-3 AWC 0C/0I/0M / W26-4 AWC 0C/0I/0M / W26-5 AWC 0C/0I/0M
- 审查材料全部实读：波级 diff 全文、五单 brief 摘要（经 plan + 波 diff 内 brief 变更段）、五份 report、三份 review-m3、progress.md W26 五行、handoff `docs/handoffs/2026-09-22-w26-partial-2of5.md`（含 09-26 补记）
- 方法：独立重跑机械闸（波级 gate / hygiene / rot-budget / boundaries）+ rustfmt 亲测 + 关键正确性声明逐条 HEAD 实读复核；不重跑 implementer 已跑的 cargo 测试（report 输出为证据，逐条与 diff 对账）

---

## A. 波级一致性 — PASS

- **波窗与提交链**：`3e9f4b5..5848bf5` = 25 commits（4 个 brief 预备 docs + 5 个代码 commit + 台账/report/allowlist/manifest 回填 docs）。代码 commit 顺序：c5f2b9c(W4) → 6e324f8(W3) → f54a3fe(W2) → 03ecda0(W5) → bf1df10(W1)；**bf1df10 为全波最后一个代码 commit**（实测 `git diff bf1df10..5848bf5 --stat -- src/ Cargo.toml Cargo.lock package.json` 为空，其后 6 commit 全为 docs）。
- **文件集对账**：波 diff 57 文件 = 28 代码文件（27 `.rs` + SI Cargo.toml）+ 29 个 sdd/handoff 文档文件。28 代码文件与五单声称文件集的并集**逐一精确对账**：W26-1×4（kernel_facade/events.rs、cli chat/run.rs、cli exec.rs、contracts events/agentic.rs）+ W26-2×6（services-core dialog_turn.rs、core save.rs / turn_lifecycle.rs / turn_persist.rs / distill.rs / session_usage service.rs）+ W26-3×2（desktop main.rs、single_instance.rs）+ W26-4×1（lsp/manager.rs）+ W26-5×15（runtime-ports credentials.rs / lib.rs、core infrastructure credentials.rs / mod.rs、core mcp config service.rs、core manager lifecycle.rs / tests.rs、SI Cargo.toml、SI config service.rs、SI tests common/mod.rs、SI config_and_server_lifecycle.rs、cli keyring_keys.rs、cli main.rs、desktop keyring.rs、entry.rs）= 4+6+2+1+15 = 28。**零缺口、零越界代码文件**。
- **波级 gate 亲跑**：`node scripts/verify-task-gate.mjs verify-attempt --base 3e9f4b5 --tip 5848bf5 --allowlist .superpowers/sdd/w26-wave-allowlist.txt` → **exit 0**（仅 3 条 unfulfilled superset 警告：Cargo.lock / settings/sync.rs / core mcp/mod.rs——防御性条目，diff 中未触碰，非违规；61 行 allowlist 完整覆盖波 diff 全部 57 文件）。
- **波级包完整性**：review.diff 实测 SHA256 = `25fc57c0f3732f600beaf61aab7865c2c6e76a4caa55d5a8309b53651a1e5ad3`，与 manifest `diffSha256` 钉死值一致；我方独立生成 `git diff 3e9f4b5..5848bf5` 与之逐段比对：代码段（全部 27 `.rs` + Cargo.toml）完全一致，差异仅 progress.md 混合编码段在本审管道下的字节扰动 + 尾行换行（非材料缺陷；代码段另经 HEAD 实读交叉验证）。
- **三单并行分窗协议对称性（W26-1/2/5）**：W26-2 终窗 `831094c..7610de5` exit 0（report §6.5 双窗原文）；W26-5 代码窗 `7610de5..03ecda0` exit 0 + 文档窗 `bf1df10..1ef1b16` exit 0 + 全窗红逐文件披露（越界 7 文件全部为兄弟 W26-1 产物，report §3.8 原文）；W26-1 W1 `03ecda0..bf1df10` exit 0 + W2 `1ef1b16..a3b7b50` exit 0。分窗并集 `831094c→7610de5→03ecda0→bf1df10→1ef1b16→a3b7b50` **连续无缝覆盖**三单全部 commit；双方对称披露对方插队（W26-5 列 W26-1 文件清单 / W26-1 报 W26-5 文件并引其 commit message 对称红）→ 证据链对称完整，无人私扩 allowlist。
- **工作树残留**：`git status` 仅 3 个 CRLF 幻影 M（runtime-ports port_core.rs / runtime_facade_tests.rs / session_workspace.rs，`git diff HEAD --stat` 实测为空）+ 未跟踪 `review-packages/w26-wave/` 包目录——与 handoff §3/§8.5 预记录「勿动勿 commit」一致，**无波级 WIP 残留**；推送只送 commit，幻影行不外带。

## B. 跨单交互 — PASS

- **文件集两两不相交**：五单集合逐对求交，交集全部为空（A 项 28 文件并集对账即机械证明——若相交则并集 < 28）。声明属实。
- **最终 HEAD 共同编译干净**：决定性证据 = W26-1 report §6.1 隔离树 @ TIP `bf1df10` `cargo check --workspace` **0 error**（1m56s 级输出原文 + TIP 级复跑 WS_TIP_EXIT=0）。因 bf1df10 是全波最后一个代码 commit（A 项实测其后 src/ 零变更），该 workspace check **覆盖全部五单合体代码 = 最终 HEAD 代码态**（含 W26-2 error_detail、W26-3 mutex、W26-4 lsp、W26-5 全部 15 文件）。附加：家规 6 desktop compile gate（merge 前 `-p northhing` 过）由同一 workspace check（含 northhing desktop crate）+ W26-1 §6.3 cli check 满足。
- **W26-5 desktop keyring 修复与兄弟代码合体**：handoff §8 记录的 W26-5 WIP 期 3 个编译错误（entry.rs:104 E0603 / keyring.rs:312,330 E0369）在 03ecda0 修复（entry.rs 改走 `settings` 再导出，diff L2443-2446，settings/mod.rs:51 `pub use keyring::*` HEAD 实测存在；keyring.rs `is_no_entry` downcast + `matches!`，diff L2138-2143），并经 bf1df10 workspace check 验证合体干净。W26-5 §3.0 亦在宿主树（含 W26-1 WIP）`cargo check -p northhing` / `-p northhing-cli` 0 error——合体 WIP 态亦编译干净的旁证。
- **测试执行级覆盖地图**（report 证据，本审不重跑）：core lib test 目标在 bf1df10 编译并执行 56 个 kernel_facade 过滤用例（W26-1 §6.2，全 binary 1081 用例编译成功——含 W26-5 新 tests.rs 与 W26-2 save.rs 内联测试的最终态编译证明）；SI 19/0 @03ecda0（其后该 crate 零变更）；services-core 56/0 @7610de5（其后零变更）；core mcp 17/0 @03ecda0（其后 mcp 模块零变更）。**desktop crate 单测（single_instance 3 + keyring tests）与 lsp 15 测试未在最终合体代码态执行**——即两项 AWC 遗留（D 项 triage），编译级已覆盖，不构成本项 FAIL。

## C. 计划对齐 — PASS（plan W26 段五目标全部落地，附 diff 证据）

| 计划目标 | diff/HEAD 证据 | 判定 |
|---|---|---|
| W26-1 steering UX：UserSteeringInjected → Banner | events.rs 丢弃臂改映射（diff L2887-2890）+ drops 清单移除（L2941-2952）+ 钉死测试含空串子用例（L2898-2936）+ CLI chat/exec 双臂同英文文案（L2066-2082 / L2094-2102）+ agentic.rs:296 注释同步（L3256-3257）；desktop 零改动，复用 app.rs:216-220 既有 Banner 消费链（HEAD 实读确认） | 达成 |
| W26-2 P2-5 失败留痕：error_detail 字段级方案 | dialog_turn.rs 字段 + 4 serde 测试（L3377-3510，旧格式/camel 往返/snake alias/skip）+ fail_dialog_turn 落盘（L2716-2738；唯一定义+唯一调用方 HEAD 实测）+ save.rs Error 无产物合成（L2599-2607；`has_text || has_thinking` 门控 HEAD 实读确认；`[Error: ` 前缀与 live 路径 chat_state_core.rs:317 / turn_banner.rs:51-52 同族）+ 3 内联测试 | 达成 |
| W26-3 P2-2 单实例拒启动 | main.rs:66-73 守卫 + exit 1 + stderr 英文文案（diff L2234-2241）+ single_instance.rs 183 行（L2251-2434：`Local\NorthHingDesktopSingleton` 会话命名空间、手写 extern 零新 crate、RAII Drop CloseHandle、Send/Sync SAFETY 注释、PID 隔离测试名） | 达成 |
| W26-4 P2-18 预留标注 + stop_server 死分支 | 预留标注（L2964，workspace.rs:450 先例同形）+ 两处死分支清理（L2968-2982 / L2985-2994）；stop_server 恒返 `Ok(())` warn-and-continue HEAD 实读证实（本项为删错误处理的前提，正确性成立）；rollback_registration :137 活调用方保留（HEAD 实测）；836→833 行（HEAD 实测 833 < ceiling 836，rot-budget god_file 规则亲跑通过） | 达成 |
| W26-5 P1-8 CredentialStore port | runtime-ports/credentials.rs 78 行（L3266-3344：port + Null + 哨兵 + account 键 + 2 测试）+ core infrastructure/credentials.rs 110 行（L2744-2854：全局注册 + 调用时解析代理 + 测试锁）+ SI set/clear 哨兵化（L3556-3603：store 先行失败即 Err 磁盘不动、save 失败 best-effort delete 回滚）+ lifecycle 哨兵解析 fail-closed（L3024-3070，仅 `== 哨兵` 介入、明文穿透、错误消息含 server_id 不含真值）+ desktop/cli 双 adapter 注册（L2127-2201 / L1993-2041）+ 12 构造点适配 + 3 新用例 | 达成 |

plan §0 用户拍板对应项（C1 / P2-5 / P2-2 / P2-18 / P1-8 partial）全部兑现；P1-8 按 W26-5 report §4 以 partial 注记收口（env 哨兵化与存量迁移缓办在案），与 plan 口径一致。

## D. Minors triage —（表见下）

## E. 波收口预检 — PASS

handoff §1.6 三件套所需材料**全部备好**（本审逐条实读）：

1. **台账翻转文案 4 份**：P2-5 = w26-2-report.md 疑虑 6（:230）；P2-2 = w26-3-report.md §S2b（:33）；P2-18 = w26-4-report.md AC4（:21）；P1-8 partial 注记 = w26-5-report.md 疑虑 §4（:140）。tech-debt-ledger.md 四条目实测仍 open、未被提前翻转——与「收口统一执行」协议一致（翻转 = 本终审后、推送前的编排者动作，文案就绪即不欠家规 2 的账）。
2. **AWC 关闭清单**：W26-3/W26-4 两条测试执行级复证（owner/deadline 见 triage 表）——推送后动作，非本终审可闭合项。
3. **handoff 更新**：以本审 + 推送 + CI 首跑结果为输入（编排者动作）。

## F. Cannot verify from diff（禁止猜，逐条列出）

1. **CI 全 matrix 表现**——波未推送，首个 push CI run 不存在；这是 W26-3/W26-4 AWC 的最终闭合点。
2. **desktop crate 单测在最终合体代码态的执行**（single_instance 3 测试 + keyring mod tests）——各 report 只在各自 TIP 隔离树/check 级跑过；编译级由 bf1df10 workspace check 覆盖，执行级待 CI。
3. **lsp 15 测试在最终合体代码态的执行**（= W26-4 AWC 本体；最后执行于 c5f2b9c 隔离树，其后 lsp 模块零变更、core test binary 在 bf1df10 编译成功，但 15 用例未在最终态跑过）。
4. **真实 UI 行为**：desktop banner 渲染时序（`streaming` 门控 vs steering 到达相位）、CLI set_status/print_text 视觉呈现——diff/单测层不可判（W26-1 m3 同款结论，沿用）。
5. **真实 OS keyring 交互**（NoEntry downcast 判定、真实 keyring 读写/删除路径）——测试全部走 mock/in-memory double，OS 真路径零覆盖。
6. **真实双开进程的单实例互斥行为**——单测为同进程双 acquire；跨进程真实双开未实测。
7. **fmt:rs 全仓 canonical 态**——本审以宿主 GNU rustfmt 1.95（与 `pnpm run fmt:rs` 同一二进制族）亲测波内 27 个 `.rs`：**24 全净**；3 个非净文件（cli/main.rs、desktop/main.rs、lsp/manager.rs）的漂移区域**全部位于波外区域**（前两者为模块递归到的兄弟文件；后者 :104/:738/:778/:803/:817，均非波内 hunk），且三个文件的 BASE 版本同样 fail——**波内 hunk 零新增 fmt 违规**。全仓 fmt 债不在本波范围。

---

## Findings

- **Critical：0**
- **Important：0**
- **Minor：4**（本审新增 2 + 任务级累积复核确认 2）

1. **[Minor]（新）W26-5 keyring.rs 含 2 处未申报的 rustfmt 驱动 hunk**：store_env 签名合行（diff L2114-2119）与测试断言拆行（L2210-2214），越出 report §1.3 声明的特性范围（仅 is_no_entry / DesktopMcpCredentialStore / register）。与 W26-2 已逐条披露的 distill.rs fmt hunk 同类同性质（家规 1 顺手清、fmt-clean 实测通过），但**未在 report 中披露、m3 审查亦未逐项清点**——披露不对称。处置：accept-and-close；后续单 report 按 W26-2 先例逐条列出 fmt 波及 hunk。
2. **[Minor]（新）波级包材料槽位不对称**：`w26-wave/manifest.json` 只钉 w26-1-report.md 为 report（assemble 工具单 report 槽位限制），其余四份 report 与三份 review-m3 文本未进波级 manifest。补偿链成立：各单 `reviews/w26-*/manifest.json` 已 sha256 钉死各自 brief/diff/report（commit 6073c1e 入库）+ 波级 diff 钉死全波终态。处置：accept-and-close（双轨可审计，无证据缺口）。
3. **[Minor]（累积确认）W26-1 report S5 计数笔误**：report 写「facade debug! 丢弃臂 12 个」，HEAD 实测 **13** 个（events.rs:350-402 逐行：350/354/358/362/366/370/374/378/382/386/390/394/402）。ledger 行已带正确计数（12 vs 13 差 1 已注记）。处置：accept-and-close。
4. **[Minor]（累积确认）W26-2 distill.rs 4 个 rustfmt hunk 越钉死区**（:39-44 / :66-70 / :148-152 / :483-489）：fmt-clean 本审独立复核属实（distill.rs 在 27 文件亲测中全净）。处置：accept-and-close。

## Minors triage 表（累积 Minors + AWC 遗留逐条处置）

| # | 项 | 来源 | 处置 | owner / deadline | 依据 |
|---|---|---|---|---|---|
| 1 | W26-1 report 丢弃臂计数 12 vs 13 | w26-1 m3 | **accept-and-close** | — | 本审 HEAD 实测 13；ledger 行已带正确数；零代码影响 |
| 2 | W26-2 distill.rs 4 处 fmt hunk 越钉死区 | w26-2 m3 | **accept-and-close** | — | rustfmt --check 实测 clean；家规 1 范围内；零功能影响 |
| 3 | W26-3 AWC：CI 复跑 `cargo test -p northhing single_instance` + `cargo check -p northhing` | w26-3 | **defer-with-owner** | 编排者 + implementer；推送后首个 CI run 完成时 | handoff §1.6②；CI run 尚不存在（F-1/F-2） |
| 4 | W26-4 AWC：波末隔离树复证 `cargo test -p northhing-core --features product-full --lib lsp` | w26-4 | **defer-with-owner** | implementer；波收口动作或首个 post-push CI（先到者） | 15 lsp 用例最后执行于 c5f2b9c 树；bf1df10 编译级已覆盖、执行级待复证（F-3） |
| 5 | W26-5 观察项：fmt:rs 并行树陷阱 | w26-5 | **accept-and-close** | — | 流程知识已入库（report §4 + handoff §8）；无代码动作 |
| 6 | 宵禁越界：W26-3 代码 commit 6e324f8 @ 03:53:32（越 53 分钟，handoff §5 已记）；**本审新观察** W26-4 代码 commit c5f2b9c @ 03:04:58（边际越 ~5 分钟，未被记录） | handoff §5 + 本审 | **accept-and-close** | — | 健康流程项非代码缺陷；缓解已立（后续 brief 写明 03:00 前完成 commit） |
| 7 | W26-5 keyring.rs 2 处未披露 fmt hunk | 本审 | **accept-and-close** | — | findings #1；fmt-clean 实测；披露改进建议已附 |
| 8 | 波级包 manifest 材料槽位不对称 | 本审 | **accept-and-close** | — | findings #2；per-task manifest 双轨补偿，无审计缺口 |

## 终判

**APPROVE_WITH_CONCERNS**

五单 SPEC/QUALITY 全部过线且本审独立复核（映射臂/落盘/门控/mutex/死分支/哨兵语义等关键正确性声明、13 丢弃臂、833 行、stop_server 恒 Ok、rollback 活调用方、app.rs 既有消费链、翻转文案 4 份）**全部属实**；波级机械闸（gate/hygiene/rot-budget/boundaries）本审亲跑全绿、波内 hunk 零新增 fmt 违规、五单文件集两两不相交、最终代码态（bf1df10 = 最后代码 commit）workspace check 0 error 覆盖全波合体；唯 desktop/lsp 测试**执行级**复证（W26-3/W26-4 AWC）与 CI matrix 表客观上待推送后闭合，按 workflow-policy「AWC 为一等判决、须 owner + deadline」放行——owner/deadline 已在 triage 表钉死。
