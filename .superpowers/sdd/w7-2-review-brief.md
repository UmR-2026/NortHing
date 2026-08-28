# W7-2 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w7-2-review-package.diff`（= `2bb91ab..e8dbcfd`，3 文件 +547/-0）
- 需求：`.superpowers/sdd/w7-2-provider-edit-ui-brief.md`（Spec 5 条 + Global Constraints 9 条）
- 上游 API：commit `2bb91ab`（W7-1）+ `.superpowers/sdd/w7-1-provider-edit-api-report.md`
- 实现者报告：`.superpowers/sdd/w7-2-provider-edit-ui-report.md`

## 编排者已完成（不必重复，矛盾必指出）

1. **截图 4 张已由编排者视觉验收通过**：设置页行编辑按钮、弹窗字段齐全（含"留空=保持不变"占位）、删除两段确认红警告、失败臂红色错误横幅——布局与文案无异常。
2. 硬防线磁盘核实：app.rs / api.rs / css.rs / pages_onboarding.rs 不在 diff 中 ✓；pages_settings.rs +45 ≤60 ✓。

## judge 重点核查项

1. **ProviderEditModalProps 手动 PartialEq（最高优先，F5 回声）**：实现者自报 E0369 修在设计层——手动 impl PartialEq"比对 provider 字段及回调"。**回调（EventHandler/Callback）如何比较？** 若恒 true = F5 老 hack 复活（prop 变更不触发重渲染）；若恒 false = 每次重渲染（可接受但注明）。逐行读该 impl + 挂载点 props 构造，判定语义正确性。
2. **keyring 接线**：UI 路径必须走 `PRODUCTION_KEYRING`（Lazy<ProductionKeyring>），薄包装是否在 api_provider_edit.rs 而非 api.rs；E0599 的机制层修复（trait 引入作用域）是否正确。
3. **编辑按钮 vs 行点击冲突**：Card 3 行既有"点击设默认"——编辑按钮的事件传播是否 stop_propagation，不触发设默认。
4. **保存/删除后刷新**：provider 列表与全局配置的信号刷新链路完整；弹窗状态机无泄漏（成功后关闭+重置）。
5. **错误臂**：测试失败/保存失败/删除默认被拒三臂 UI 显式中文报错；fail-closed 语义未被 UI 层吞掉（W7-1 API 的 Err 必须原样上屏）。
6. **warnings 54→44**：实现者称"减少 10 个未消费警告"——核实合理性（touch lib.rs 全量重编取真值；若 44 < 50 基线，说明还顺手清了别的，给清单）。
7. **测试有效性**：+3 例（default_base_url_mapping / is_known_default_url / supported_provider_types_coverage）非恒真。
8. Spec 5 条 + Global Constraints 9 条逐条；rot 收口绿。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过。QUALITY 防腐必查：复用核查（弹窗是否重造了既有 overlay/表单组件——先核仓内既有模式）/ 无 owner 抽象 / 预算闸（rot-budget.json 零触碰）/ god-file 观测点（pages_settings.rs 731→776，给一句健康度观察）。

**Cannot verify from diff** 单独列出，禁止猜。plan-mandated 冲突不自行裁决，交编排者。

## 输出

判决书写入 `.superpowers/sdd/w7-2-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M 计数 + 一句话理由。
