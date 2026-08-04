# Model Capability Notes

## 用户指令（长期生效）
- **2026-08-04：k3 系不做 coder/implementer 任务**（用户明示）。implementer 选 gemini-31-pro / glm-5.2 / minimax-m3 / step-explore 等；k3 仅可考虑用于审查/架构（如需独立视角）。

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

## 2026-08-02~03 前端视觉探索轮（consult-room 方向定稿，设计 bakeoff ×4 轮 + spike）

### 设计探索子代理
- **gemini-31-pro（首测）**：纪律全场最稳（emoji 0、阴影零滥用），设计判断稳（settings 双区、jewel 触发器）。纪律型页面首选。
- **step-explore**：结构发明力最强（space 走廊=亮门独占光源、jewel 断口、膜结细化），会自跑 Edge headless 截图 + rect 量测自验。**两次最终消息中途截断、文件未落地** → 派发须写明"HTML 一次写完"，收工验文件存在，失败用 task_id 续会话。
- **gemini-36-flash**：快、氛围强，但 **emoji 惯性成癖**（⚡/🌙/ 跨多轮复发，写进 agent 定义后仍犯）；一次空结果未落文件 → 交付必须机械扫描 emoji + 验文件，发现即修。
- **minimax-m3**：craft/质感最强（枯山水/窑变/archive 地层），tradeoff 自陈诚实。氛围页首选。
- **kimi-k3**：volcengine 线（general/kimi-k3_general）严谨可用；**ark provider 在本环境不可解析**——kimi-k3 扁平、ds-v4-flash 扁平、ark-kimi-k3_* 变体均派发失败（"Model not found: ark/kimi-k3"），一律走 volcengine。
- **qwen3.8-max-preview**：编排者本体兼 qwen 槽（bakeoff 页、spike 均自做，质量与子代理同级）。项目级 coder-qw/judge-qw 在 `E:\agent-project\.opencode\agents\`，worktree 会话不加载。

### Slint 翻译词汇（spike 实测，详 `docs/design/2026-07-22-frontend-redesign/slint-feasibility-consult-room.md`）
- Rectangle 无 scale-x/scale-y → 呼吸改绑 opacity（animation-tick + Math.sin）；border-radius 不吃 %。
- 双 `slint_build::compile` 共存时后者覆盖 SLINT_INCLUDE_GENERATED → 探针先于 main 编译。
- 新 worktree 缺 gitignore 生成文件 → 先跑 `node scripts/generate-i18n-contract.mjs`。
- mind 五色 × 双主题 25 预计算 token 已扩生成器入 palette（color-mix(in srgb) = gamma 逐通道插值，透明端 8 位 alpha）。
## 2026-08-04 P1 安全债修复轮（fix/p1-security-0804: C1 trash + C2 relay + C3 keyring）

### Implementer
- **volcengine-agent-plan/deepseek-v4-flash（general/deepseek-v4-flash_general 派发）**：C1/C2/C3 三个任务均一次成功（C3 一轮 fixer 修文档与一处 M-8 测试加固）。亮点：报告事实纪律严格执行，所有「机制存在/不存在」结论均带 file:line（继承自 C1 教训）；fail-closed / sentinel / 密钥迁移等敏感改动审慎。**深度+速度优于 kimi-k3 + 额度充足**，确立为本轮 implementer 默认档。
- **ark/kimi-k3 flat 派发失败**（"Model not found: ark/kimi-k3"）—— 一律走 volcengine 线。flat kimi-k3 + ark-* 变体在本环境均不可解析。

### Reviewer (judge)
- **minimax-cn-coding-plan/MiniMax-M3**：C1/C2/C3 三轮审查。**C1 第一次抓出 implementer 对远程确认门的事实捏造**（spec FAIL → DONE_WITH_CONCERNS → fix 轮修正）；C2/C3 8-10 项机制核验全部 file:line 独立确认 0 捏造。C3 唯一 Important = 本环境 ring/aws-lc-sys gcc 缺失导致测试未实跑（环境约束，非代码），fix 轮已显式记录 + 援引 C2 同根因。任务级稳定档。

### 编排者经验（C1 教训）
- **报告纪律是 spec 判决的一部分**：implementer 报告里若编造/推断机制存在性结论（无 file:line 证据），即 spec FAIL，与代码正确与否无关。Brief 必须明示「所有机制性结论带 file:line，无法核实写未核实」。
- **fix 轮派回原 task_id 续会话**：fixer 延续上下文，最小成本；若另开新会话会丢规范与 diff 状态。
- **环境约束显式化**：当本机 gcc/PATH 等缺导致验证不可跑，report 必须明示而非默略——既避免下游「成功数字」假象，又给 CI 留明确接管路径。
