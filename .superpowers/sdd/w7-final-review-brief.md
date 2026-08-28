# W7 全波终审 Brief：F7 设置页 provider 编辑（2026-08-28）

只读审查。不改代码、不 commit。仓库根：`E:\agent-project\NortHing`（分支 main）。

## 范围与证据

- 审查范围：`029a5ad..e8dbcfd`（2 commits = W7-1 API 层 `2bb91ab` + W7-2 UI 层 `e8dbcfd`；7 文件 +958/-35，全在 src/apps/desktop）
- diff 包（排除 .superpowers）：`.superpowers/sdd/review-w7-final-029a5ad..e8dbcfd.diff`
- 计划：`.superpowers/sdd/plan-2026-08-28-w7-provider-edit.md`（含编排者钉死裁定 + god-file 防线表）
- 侦察：`.superpowers/sdd/w7-f7-settings-edit-recon.md`
- 任务 brief/report/review：`.superpowers/sdd/w7-1-*` 与 `.superpowers/sdd/w7-2-*` 同目录全套
- 台账：`.superpowers/sdd/progress.md` 顶部 W7 Ledger

## 波级背景

- F7 = 设置页 provider 编辑功能（用户拍板选项 A）。W7-1 建 API（edit/delete + keyring 语义，review 0C/0I/3M）；W7-2 建弹窗 UI（review 0C/0I/3M；编排者已视觉验收 4 张截图）。
- W7-1 的 +4 warnings 中间态应由 W7-2 消化：收口实测 bin warnings 44 ≤50 基线——核实属实。

## 判决要求

双判决 + 合并裁决：`SPEC: PASS/FAIL`（对照计划两个任务 Spec 逐条，file:line 证据）、`QUALITY: PASS/FAIL`（跨任务集成正确性）、裁决 `CAN MERGE / NEEDS FIXES`。Findings 分级 C/I/M 带 file:line。每个发现先读源码全文再判。

## Global Constraints（逐字复制自计划，逐条核对）

1. 分层边界：改动只在 `src/apps/desktop`；其它 crate 零改动。
2. 日志纪律：新增日志一律英文、无 emoji，带关键上下文字段。
3. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/`。
4. rot-budget：不上调任何 ceiling；god-file 防线按计划表执行（app.rs 零触碰、api.rs ≤728、pages_settings.rs ≤791、css.rs/pages_onboarding.rs 零触碰）；新文件 <800 行；`node scripts/verify-rot-budget.mjs` 收口绿。
5. 验证最小集：MSVC `cargo check -p northhing` + 聚焦测试 + rot 实测；命令与输出原文进 report。
6. commit 规则：每任务恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物。
7. 不新建无 owner 抽象；复用侦察 §5 清单里点名的既有设施。
8. i18n frozen：desktop 硬编码中文文案，不动 ftl。
9. 家规 4：本波不碰 tokio 任务生命周期/取消/关闭顺序。

## 终审特殊关注点（跨任务集成）

1. **API↔UI 接缝**：W7-2 弹窗消费的 API 签名与 W7-1 实际落地一致；`api.rs:23` glob re-export 的 unused 警告确已消化；UI 不传 key 时的"留空=不变"语义端到端不断链（UI 占位提示 → API fail-closed 三臂）。
2. **PartialEq 一致性**：W5-4 给 ModuleAppProps 立的结构比较先例 vs W7-2 ProviderEditModalProps 手动 PartialEq（忽略回调）——两个实现的语义取向是否一致、文档注释是否各自说清。
3. **keyring 生产/测试隔离**：PRODUCTION_KEYRING 只在生产路径；测试全走 MockKeyring；无真 keyring 写入（W5-3 教训回扫）。
4. **删除链路**：默认拒删（core GlobalConfig 单事实源判定）→ delete_model_config → delete_api_key 顺序与失败臂；UI 两段确认；ponytail 注释（无会话引用扫描）在位。
5. **wire_format 显式映射**：全链路未经 `infer_provider_wire_format`（编辑路径）；类型下拉的值集与 sync.rs 实际接受集一致。
6. **累积 Minor triage 队列**（逐条给"修一记一 / accept-and-close / defer-with-owner"建议+一句理由）：
   - W7-1-M1 测试 mock 语义误导（MockKeyring 按 id 存，assert 文案误导）
   - W7-1-M2 `delete_api_key` best-effort 吞 Err（pre-existing，keyring.rs:233-239）
   - W7-1-M3 +4 warnings 中间态——W7-2 已消化（44≤50），建议直接关闭
   - W7-2-M1 pages_settings.rs 776/800，下个 provider feature 应先抽 provider_row.rs
   - W7-2-M2 run_test 的 keyring 读失败 .ok() 静默（test 路径），与 save 路径 fail-closed 不对称
   - W7-2-M3 弹窗文件零 tracing 日志（save/delete 成功/失败路径）
7. **台账一致性**：progress.md W7 行与实际 commit 链一致；家规 2 无应翻未翻。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过。QUALITY 防腐必查：复用核查 / 无 owner 抽象 / 预算闸 / god-file 观测点（pages_settings.rs 731→776 已接近 800，给健康度观察一句）。**Cannot verify from diff** 单独列出，禁止猜。plan-mandated 冲突不自行裁决，交编排者。

## 输出

判决书写入 `.superpowers/sdd/w7-final-review.md`（双判决 + findings + Minor triage 表 + Cannot-verify + 合并裁决）。返回消息只给：裁决 + C/I/M 计数 + 一句话理由。
