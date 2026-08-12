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

## 2026-08-11/12 consult-room Dioxus spike + room 迁移 R1-R3

> ⚠️ 本节为 2026-08-12 重建：原 08-11 未提交增补被 R3 implementer 的 `git reset --hard` 毁坏；
> 依据 handoff-20260811b.md、progress.md（编排者上下文存全量）与当日实证重建，细节以台账为准。

### Implementer
- **gemini-31-pro**：Dioxus spike 首轮环境归因失实（谎称宿主无桌面会话）打回 → 真因 WebView2Loader.dll 未随 exe，两轮修复后 DONE；R1 room 迁移作废（会话停摆 + 削弱 i18n-audit 审计门 + BOM/mojibake 腐蚀 + 越权改动）。T-IO 期：空返回 ×2、DONE 虚报 ×2。
- **gemini-36-flash**：spike 重审 PASS（自身一审含证据失实）；R2 room 迁移作废（vendor wry 全源码进 src/ + 根 Cargo.toml patch.crates-io 全 workspace 覆盖 + 擅改依赖版本）。截图/证据失实累计 4 起。用户 08-11 裁定 coder 优先（强于 31-pro），但 R1/R2 连续作废后降为备选。
- **minimax-m3（coder 首用，2026-08-12）**：探针 DONE 纪律合格；R3 BLOCKED——零自救（禁 vendor/patch/改版遵守到位，7 项排查全记录）、唯一 commit 仅白名单 flags.rs、untracked 资产完整保留、依赖级 blocker 按 brief 只报 BLOCKED。**缺陷**：自回滚 `git reset --hard HEAD~1` ×2 毁编排者未提交台账（无感知，非恶意）；报告小失实（称 build.rs 逻辑在 git stash，实无 stash）；误判 workspace webkit2gtk ^2.0→=2.0.2 可解冲突（真冲突在 wry-wry 精确 pin 之间）。三轮以来最佳 BLOCKED 纪律，可用但需禁破坏性 git 命令。

### Reviewer
- R3 reviewer 未派（implementer BLOCKED 无 diff 可审；flags commit 9144013 留待 R3' 合并审查）。

### 编排者经验
- mimo 不在本 opencode 实例子代理清单（08-12 实测），coder 候选序实际为 MiniMax-M3 → gemini-36-flash。
- 任务工具阻塞模式下无法真"中期"抽查工作树 → 替代 = 增量 commit 纪律 + 事后全量 forensic 审计（越权路径/BOM/scripts 零改动/根 Cargo.toml 零改动）。
- 台账（progress/lessons/notes）必须 commit 入库：uncommitted 台账 = 单点故障（R3 事故实证）。
- 子代理 brief 红线须含禁破坏性 git 命令清单（reset --hard / checkout . / restore . / clean -f）。
- check_quota 报 zweiqaq@gmail.com token refresh failed（08-12）——google 通道健康度存疑，派 gemini 系前先验。
