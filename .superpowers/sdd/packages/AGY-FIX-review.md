# Review Package — AGY-FIX（agy 渠道 3.7/3.8 flash 修复）

- 仓库：`C:\Users\UmR\Desktop\opencode-antigravity-auth-fork`（**不是** NortHing）
- BASE `dc66228` → HEAD `bbbc28f`（含 `fab45f2`；diff = `git diff dc66228..bbbc28f`）
- brief：`<fork>/AGY-FIX-brief.md`；report：`<fork>/AGY-FIX-report.md`
- 上游证据：`.superpowers/sdd/reports/agy-429-path-review.md` + `agy-resolution-review.md`（NortHing 仓）

## 任务一句话
agy 渠道 3.7/3.8-flash 请求 429→零 chunk 挂死。四修：A 指纹 UA 陈旧再生成（主根因：目录层 UA 2.11.0 过版本门、内容请求层却发持久化指纹 2.0.6-Mac）；B 429 误分类（"resource exhausted" 启发式先于 quota 判定 → 重试放大 ~16-20x）；C 主 fetch 无超时 + AbortError 吞噬；D tier 白名单缺 3.7。

## 验收标准
1. typecheck / vitest 884 全绿 / build 产物含四修。
2. 真实冒烟：3.7-flash（27s）与 3.8-flash（16s）真实 API 响应 WORKING；账户指纹运行时自动修复为 2.11.0 win32。
3. commit 在 fork main，未 push。

## 已知偏离（实现者已申报，审查判定合理性）
- C 未套用既有 fetchWithTimeout（私有且会覆盖 init.signal），改用同款模式 + AbortSignal.any 组合；超时 120s 对齐 IMAGE_TIMEOUT_MS 而非 30s。
- commit 混合了 fork 既存 09-03 WIP（与 A-D 同文件）——逐 hunk 区分两批，WIP 部分审查其合理性但不算本任务引入。
- project.test.ts 3 处断言对齐（实现者称系 09-03 WIP 批次自相矛盾的预存破，非本次引入）——核实此说法。

## Global Constraints
- 不动 OAuth authorize/exchange；不动账户轮换策略语义；指纹再生成不改 accounts.json schema（version=4）。
- 禁区：NortHing 仓库；fork 的 package.json / tsconfig。
