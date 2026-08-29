# W10-1 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w10-1-review-package.diff`（单 commit `078af44`，5 文件 +583/-546；api.rs 799→266）
- 需求：`.superpowers/sdd/plan-2026-08-29-w10-godfile-split.md` Task 1
- 实现者报告：`.superpowers/sdd/w10-1-api-split-report.md`

## judge 重点核查项

1. **纯位移逐块核对**：抽出的三组（settings wrapper / event bridge / memory wrapper）与原 api.rs 逐函数等价；`re-export` 面（pub use）是否让既有调用点零改动（rg 抽查 3 个调用点：app.rs 的 submit_turn、pages_settings 的 list_model_configs、pages_memory 的 list_facts）。
2. **TEST_GLOBAL_CONFIG_MUTEX 归位**：放 api_settings.rs + 从 api.rs re-export 保路径——抽查既有测试的引用路径没断。
3. ** flaky 测试声明**：实现者称 `test_delete_provider_default_provider_rejected` 全量跑 flaky、单跑过，"pre-existing 与本次拆分无关"。**这是重点核查项**：该测试用全局 config + TEST_GLOBAL_CONFIG_MUTEX 串行化——拆分后 mutex 是否仍被所有相关测试正确共享（若有测试没走 mutex 就可能真引入竞态）。判定：真 pre-existing 记观察项；拆分引入 = Important。
4. **事件桥（W5-2 的分级逻辑）**：搬家后 TextChunk 预算/控制事件直通语义逐行等价。
5. Spec/Constraints 逐条；rot 收口绿复核。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。一切以 diff 和实跑输出为准。双判决缺一不算通过。防腐必查：复用核查 / 无 owner 抽象 / 预算闸 / god-file 观测点（api.rs 266、api_settings.rs 292 各一句）。**阻塞性数字断言磁盘实测**。Cannot verify 单独列出，禁止猜。

## 输出

判决书写入 `.superpowers/sdd/w10-1-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M + 一句话理由。
