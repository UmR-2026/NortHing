# W11-2 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w11-2-review-package.diff`（单 commit `33bb4a4`，9 文件 +626/-555；selectors.rs 861→827）
- 需求：`.superpowers/sdd/w11-2-selectors-cluster-brief.md`（A 层全做 + B 层只出地图）
- 实现者报告：见返回消息（逐项证据在内）+ `.superpowers/sdd/w11-2-*-report.md`（若有）

## judge 重点核查项

1. **行为零变化逐块核对（最高优先）**：33 处 block_in_place → bridge 迁移——抽查 ≥5 处确认闭包体逐字未变；尤其 selectors.rs 两个 self-borrow case 从 `async move` 改回 `async {}` 的语义差异（实现者称这是正确修法——核实 borrow 语义确实等价）。
2. **复用核查**：bridge 复用 W8-1 `input/bridge.rs`（未新建）；provider_display_name/model_display_name/parse_custom_headers 归 `ui/model_selector.rs`；两处调用点都切换（rg 验证零残留副本）。
3. **魔数/哨兵**：常量值与原字面量一致（128_000/8_192/"primary"）。
4. **B 层纪律**：diff 中不应有 chat/{session,skill,subagent,theme}.rs 的页面级合并动作（helper 调用切换除外）；Scheme C 不对称等 4 处腐化点**保持不动**（行为零变化铁律）。
5. **manifest**：861→827 下调，无其它变更。
6. **偏离 4 的"in-flight 残留"**：实现者称补完了上一 session 的残留（unused imports + model_config 内联字面量）——核实 diff 确实含这些清理且无越界。
7. Spec/Constraints 逐条；测试 51/51；rot 绿实测复核。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。一切以 diff 和实跑输出为准。双判决缺一不算通过。防腐必查 5 项（含证据抽查节）。**阻塞性数字断言磁盘实测**。Cannot verify 单独列出，禁止猜。

## 输出

判决书写入 `.superpowers/sdd/w11-2-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M + 一句话理由。
