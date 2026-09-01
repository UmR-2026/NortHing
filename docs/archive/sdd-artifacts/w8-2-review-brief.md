# W8-2 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w8-2-review-package.diff`（单 commit `5d4d98a`，3 文件 +242/-188）
- 需求：`.superpowers/sdd/w8-2-memorydb-dedup-brief.md`（Spec 5 条）
- 病灶：`.superpowers/sdd/deep-rot-memorydb-lsp.md` §文件1
- 实现者报告：`.superpowers/sdd/w8-2-memorydb-dedup-report.md`

## 编排者已磁盘核实（矛盾必指出）

1. commit 单发、`git show --stat` 与汇报一致；rot 实测绿；ceiling 918→894 为下调（合规方向）。
2. 测试 23/23 含 2 新例（nan_sinks_to_bottom / clock_anomaly_skips_boost）。

## judge 重点核查项

1. **去重等价性（最高优先）**：三块复制（stmt 构造 / query_map 闭包 / 字符串→枚举 match ×3）的旧实现 vs 新 helper——逐块核对 fallback 语义逐字一致（未知字符串的处理臂不许变）；SQL 文本在拼接/参数化过程中不可有微妙差异（空格、列顺序）。
2. **§3 两处行为微调的正确性**：NaN 沉底（Greater 臂在降序语境的方向是否正确——排序方向接反 = Important）；时钟异常跳过 boost 的路径是否只在异常臂生效。
3. **死变量处置**：bm25_pos 删除后 `ScoredFact.bm25` 存储语义未变；last_mentioned_at 是删除解构还是发现了真 bug（brief 规定若是真 bug 须 BLOCKED——确认实现者没静默改 Fact 重建逻辑）。
4. **ceiling 下调数值**：manifest 894 = 实测行数（用 verify-rot-budget.mjs 同口径复核）；json 仅此一处变更。
5. **测试有效性**：两个新测试非恒真（NaN 测试是否真的构造了 NaN score 样本；时钟测试的注入方式）。
6. Spec 5 条全过；Global Constraints 逐条。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过。防腐必查：复用核查 / 无 owner 抽象（新 helper 必须两个真实消费方）/ 预算闸（manifest 仅允许下调）/ god-file 观测点（memory_db.rs 894/894，附健康度观察一句）。**Cannot verify from diff** 单独列出，禁止猜。plan-mandated 冲突交编排者。

## 输出

判决书写入 `.superpowers/sdd/w8-2-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M 计数 + 一句话理由。
