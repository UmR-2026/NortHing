# W9-5 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w9-5-review-package.diff`（单 commit `879b7c4`，4 文件 +206）
- 需求：`.superpowers/sdd/w9-5-skills-ui-brief.md`
- 背景：前任（同模型不同 session）被中止留下不编译半成品，本实现者修复 E0716 后续用完成——核查 diff 是否有半成品残骸痕迹（不一致风格/半截注释）

## judge 重点核查项

1. **E0716 修复正确性**：api.rs skills wrapper 的 DTO 绑定方式；`HashMap<String,bool>` + clone 的取舍是否合理（一次性 clone vs 生命周期纠缠）。
2. **启停链路**：开关 → set_skill_enabled → facade；失败臂回滚（开关态必须回滚，不能只报错）。
3. **scope 语义**：用户级 scope 传参是否与 SkillScopeDto 实际变体一致；deferral ponytail 注释在位。
4. **DTO 字段对齐**：SkillInfoDto 实际字段 vs UI 使用字段（实现者称 group_key/is_builtin 未暴露——核实契约层，若暴露了而 UI 没用 = Minor）。
5. **api.rs 799/800**：压注释达标的处置是否掩盖了该拆分信号（api.rs 该登记观察还是该拆——给一句健康度判断）。
6. **测试非恒真**：4 个 truncate 测试读断言。
7. Spec 6 条 + Constraints 逐条；偏离 3 条处置合理性复核。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。一切以 diff 和实跑输出为准；实现者已跑过的测试不重跑，但验证章节命令与输出要对得上 diff。双判决缺一不算通过。防腐必查：复用核查 / 无 owner 抽象 / 预算闸 / god-file 观测点。**阻塞性数字断言磁盘实测**。Cannot verify 单独列出，禁止猜。

## 输出

判决书写入 `.superpowers/sdd/w9-5-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M + 一句话理由。
