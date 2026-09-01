# W7-1 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w7-1-review-package.diff`（= `029a5ad..2bb91ab`，4 文件 +411/-35）
- 需求：`.superpowers/sdd/w7-1-provider-edit-api-brief.md`（Spec 5 条 + Global Constraints 9 条）
- 侦察（雷区 R1-R7）：`.superpowers/sdd/w7-f7-settings-edit-recon.md`
- 实现者报告：`.superpowers/sdd/w7-1-provider-edit-api-report.md`

## 编排者已磁盘核实（矛盾必指出）

1. sync.rs：`resolve_effective_api_key` 已删、`resolve_edit_api_key` 保留且零 `allow(dead_code)` 残留 ✓
2. app.rs / pages_settings.rs 零触碰 ✓；api.rs +8 行（预算 ≤10）✓
3. **警告基线争议（编排者未决，交你裁定）**：implementer 报 bin warnings 54 vs brief 红线 ≤50。已坐实归属本任务的只有 `api.rs:23 unused import: api_provider_edit::*`（glob re-export 等 W7-2 UI 消费，波内自行消化）。另 3 个疑似新增（settings/mod.rs:88 methods never used、integrity.rs `SessionIntegrityIssue` never constructed、`validate_session_integrity` never used）在本 diff 找不到因果——疑似增量编译缓存致计数漂移。**裁定要求**：强制全量重编 desktop crate（touch lib.rs 后 check）得真值，并同法在 W6-1 HEAD（`11a4e5e`，可用 `git worktree` 到临时目录）测基线，给出真实归属判定。若确有本任务引入的可消除警告（除 api.rs:23）= Important。
4. rot 收口绿（unwrap 474/expect 940/let_ 388/dead_code 106），但 dead_code 106 = 删 2 增 2——**查新文件是否新增 2 处 `#[allow(dead_code)]`**；新代码原则上不该需要死代码标注，若加了必须给逐处理由，无理由 = Important。

## judge 重点核查项

1. **key 语义（I1 教训，最高优先）**：留空继承/覆盖/fail-closed 三臂代码逐行走查；尤其 keyring 读失败路径——确认不会被 flatten 成"空 key 继承"（I1 的老坑就是吞错）。对照测试 ③ 是否真断言"拒绝保存且零写入"。
2. **删除守卫**：默认 provider 拒绝删除的判定数据源是 core GlobalConfig（单一事实源）而非 AppSettings 镜像；delete_model_config 与 delete_api_key 的顺序与失败臂（config 删了 keyring 没删 = 可接受？report 怎么说）。
3. **wire_format**：确认实现未调 `infer_provider_wire_format`；`provider_wire_format_from_str` 的字符串集与 report 声明一致。
4. **校验复用**：`validate_provider_input` 是真复用还是重写（重写 = 复用核查违规，Important 起评）。
5. **测试有效性**：7 例非恒真（judge 可抽查把断言取反是否会红，或直接读断言逻辑）；`test_edit_provider_keyring_read_error_fails_closed` 的 mock 失败注入方式是否真触发 fail-closed 分支。
6. **Spec 5 条 + Global Constraints 9 条**逐条。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过。QUALITY 防腐必查：复用核查 / 无 owner 抽象（新文件 403 行——每个 pub 项是否都有 W7-2 或测试的真实消费方）/ 预算闸（rot-budget.json 零触碰）/ god-file 观测点（本 diff 未触登记文件则免）。

**Cannot verify from diff** 单独列出，禁止猜。与计划原文冲突（plan-mandated）不自行裁决，列出交编排者。

## 输出

判决书写入 `.superpowers/sdd/w7-1-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M 计数 + 一句话理由。
