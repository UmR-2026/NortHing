# Model Capability Notes

## 2026-08-01 后端安全修复分支 fix/backend-debug-0731（8 任务 + 终审）

### Implementer
- **opencode/deepseek-v4-flash-free (variant=max)**：Task 1-7 实现全部一次成功。质量高：typed-newtype 防线、原子写复刻、并发事务测试均符合 brief。两次 `cargo fmt` 污染需编排者 revert 兜底（brief 已禁后未复发）。**T8 派发时免费额度耗尽**（next retry 约 3 小时后），勿在无备选时依赖。
- **ark/glm-5.2**：Task 8（M-9 LSP ValidatedPluginId + staging 原子安装 + M-2 async 修复）一次成功。API 适配清单完备、设计决定（get_server_path 内部校验 vs 改签名）有据。可作 implementer 标准档备选。

### Reviewer (judge)
- **minimax-cn-coding-plan/MiniMax-M3**：Task 1-8 全部任务审查。双判决稳定输出，spec/quality 分离，Minor 分级准确（~20 项均给出 file:line）。曾一次性漂到 agnes-2.5-flash（T5），质量仍合格但需留意 session.create 显式指定 model（send 会漂回默认）。
- **ark/glm-5.2 (judge-glm)**：整分支终审（373 行报告）。跨任务一致性分析（两套 ID 校验逐字符对比、三处原子写特性矩阵）、triage 逐项处置表、合并冲突面确认均高质量。终审级首选。

### 编排者经验
- 终审 judge 用未参与单任务审查的模型（judge-glm）提供真独立视角，效果优于复用任务级 judge。
- 工具坑：`session.send` 模型漂移、`session.messages wait` 不可靠（用 tokens 判活）、`git status` stat 噪声（update-index --refresh 兜底）、裸 `cargo fmt` 卷无关文件。

### Task 9（回归修复: 6 个 pre-existing 测试失败, 2026-08-01）
- **kimi-for-coding/k3-256k (implementer)**：DONE_WITH_CONCERNS 一次交付，三组修复全过（目标 14/14, 全量 1134/1134 x2）。亮点是独立根因验证：main 基线 stash 复跑 + 临时插桩日志定位，推翻 brief 对 cancel 测试的 TOCTOU 归因（真因 = execution_task ~0.84s LLM 网络往返 > 50ms cancel 窗口），并仍按 brief 修复了 B-1 真 TOCTOU 产品 bug。双路径（产品修复 + hermetic 化）判断准确。
- **minimax-cn-coding-plan/MiniMax-M3 (judge)**：APPROVED WITH NOTES (spec PASS / quality PASS w/ 2 Minor + 5 FYI)。对 implementer 的根因修正做了独立复核（FYI #4/#5 确认论证可信），file:line 证据充分。
- 编排者经验：brief 归因与真实根因可能不一致——implementer 的独立验证（基线复跑 + 插桩）是关键防线；DONE_WITH_CONCERNS 携带的根因修正需要 judge 复核背书后才可采信。