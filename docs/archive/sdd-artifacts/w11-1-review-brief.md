# W11-1 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w11-1-review-package.diff`（单 commit `76d2c33`，2 文件 +20/-59；css.rs 829→790，ceiling 830→790）
- 需求：`.superpowers/sdd/w11-1-css-dead-rules-brief.md`
- 病灶：`.superpowers/sdd/blind-review-css-2026-08-29.md`
- 实现者报告：返回消息（逐条零引用证据表在内）

## judge 重点核查项

1. **R7.2→R8.1 属性迁移（最高优先，唯一非纯删点）**：实现者自述把 `left::before/right::before` 的 `left:-4px/right:-4px/background` 迁进 R8.1 覆盖规则（313-314 行）以保持零视觉变化——**逐属性核对迁移前后级联结果等价**（特异性、源顺序、!important 关系）。这是 CSS 级联语义等价性问题，任何疏漏 = 视觉回归。
2. **死规则零引用证据抽查**：抽查 3 个删除点（depth-bar 块、membrane-node 死声明链、inject_stylesheet_html）的 rg 结论真伪。
3. **覆盖关系断言核实**：`padding-right:160px` 被 136px 覆盖、opacity 死链（.85→.55→.9 等）——CSS 源顺序+特异性核实"后定义胜出"是否成立。
4. **行合并回滚**：line 86 区域恢复一行一条后，规则清单与合并前逐字一致（无丢规则）。
5. **manifest**：830→790 下调、无其它变更。
6. Spec 5 节逐条 + Constraints。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。一切以 diff 和实跑输出为准。双判决缺一不算通过。防腐必查 5 项（含证据抽查——判决书必须含"证据抽查"节列出你验证过哪些断言、怎么验的）。**阻塞性数字断言磁盘实测**。Cannot verify 单独列出，禁止猜。

## 输出

判决书写入 `.superpowers/sdd/w11-1-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M + 一句话理由。
