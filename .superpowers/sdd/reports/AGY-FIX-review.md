# AGY-FIX Review — opencode antigravity fork 四连修（429 挂死）

- 仓库：`C:\Users\UmR\Desktop\opencode-antigravity-auth-fork`
- BASE `dc66228` → HEAD `bbbc28f`（含 `fab45f2`，双 commit）
- 派发 brief：fork `AGY-FIX-brief.md`；实现者报告：fork `AGY-FIX-report.md`
- 上游证据：NortHing `.superpowers/sdd/reports/agy-429-path-review.md` + `agy-resolution-review.md`
- 本次审查于 fork HEAD 实跑：typecheck 0 错误 / `npm test` 884 passed (29 files) / `npm run build` 落 232 文件 dist

---

## SPEC（逐条对照验收标准）

| # | 验收点 | 结论 | 证据（file:line） |
|---|---|---|---|
| 1 | `npm run typecheck` 全绿 | PASS | 本地重跑仅 2 行 npm notice，无 tsc 报错；tsc --noEmit 零输出 |
| 2 | `npm test` 884 全绿 | PASS | 本地重跑 `Test Files 29 passed (29) / Tests 884 passed | 25 todo (909)`，与报告完全一致；project.test.ts:58/66/98 已对齐 WIP daily-first + PLATFORM_UNSPECIFIED 行为，全部 PASS |
| 3 | `npm run build` 产物落 dist | PASS | 本地重跑 232 文件落 dist；`dist/src/plugin.js:876` 含 `CONTENT_REQUEST_TIMEOUT_MS = 120_000`，`dist/src/plugin/accounts.js:37` 含 `case "RESOURCE_EXHAUSTED"`，`dist/src/plugin/transform/model-resolver.js:197` 含 `(?:5|6|7|8)`，`dist/src/plugin/fingerprint.js:99` 含 `isFingerprintStale` |
| 4 | 3.7-flash 真实响应 WORKING（27s） | PASS-by-evidence | 报告贴 `> build · antigravity-gemini-3.7-flash → WORKING`；smoke 脚本 `script/test-models.ts:20` 已加入 `google/antigravity-gemini-3.7-flash`；脚本 `proc.on("close", code => success = code===0)` 且 prompt 为 `Reply with exactly one word: WORKING`，model 直接回 `WORKING` 证明 200 OK；不可在本环境复跑真实 API |
| 5 | 3.8-flash 真实响应 WORKING（16s） | PASS-by-evidence | 同上；`script/test-models.ts:21` 加入 `google/antigravity-gemini-3.8-flash` |
| 6 | 账户指纹运行时自动修复为 2.11.0 win32 | PASS-by-evidence | `isFingerprintStale`（fingerprint.ts:149-155）+ `collectCurrentFingerprint`（fingerprint.ts:118-139，UA = `antigravity/${getAntigravityVersion()} ${os.platform()}/${os.arch()}`，版本走 `getAntigravityVersion()` ≥ 2.11.0 by `initAntigravityVersion` floor 在 version.ts:85-90）。`AccountManager` 构造器 accounts.ts:372 在加载时无条件刷：isFingerprintStale → 替换为 collectCurrentFingerprint |
| 7 | commit 在 fork main，未 push | PASS | `git branch -vv` = `* main bbbc28f [origin/main: ahead 2]`；HEAD = `bbbc28f test: cover isFingerprintStale version/platform gating (AGY-FIX A)`；前序 `fab45f2 fix: unstick agy 3.7/3.8 flash after 429 ...`；未 push |

**总体 SPEC = PASS。**

---

## A 专项（指纹陈旧判定）

- 实现：`src/plugin/fingerprint.ts:149-155` `isFingerprintStale`，正则 `/^antigravity\/(\d+\.\d+\.\d+)\s+(\S+)\//`
- 复用：`semverLess`（version.ts:32-40，export 在本 commit）；`collectCurrentFingerprint`（fingerprint.ts:118-139 既有）
- semverLess 语义：拆分 → 数字填充 0 → 三位比较 → 任一不等即比较 → 全等返回 false。手算 `2.0.6` vs `2.11.0`：`[2,0,6]` vs `[2,11,0]`，i=0 相等，i=1 `0<11` → true。正确。`2.11.0` vs `2.11.0` → 全等 → false。正确。
- 平台比较：`match[2] !== process.platform`。UA 形态 `antigravity/<ver> <os.platform()>/<os.arch()>`（fingerprint.ts:130），`os.platform()` 返回 `win32`/`darwin`/`linux`/...，`process.platform` 同源。**case 安全 + 格式安全**。
- 再生成位置：accounts.ts:372 `AccountManager` 构造器加载时，**确在账户加载主链路上**。
- 用 `getAntigravityVersion()`（运行时值）而非字面 `ANTIGRAVITY_VERSION_FALLBACK`：略偏离 brief 字面（brief 钉 2.11.0），但更防御——版本被 `initAntigravityVersion` 抬到 ≥ 2.11.0 之后，旧指纹同样被刷；同时 floor 在 version.ts:85-90 保证不会低于 2.11.0。**偏离合理**。
- 边界：acc.fingerprint 为 null/undefined → 正则不匹配 → 返回 true → 走 collectCurrentFingerprint。✓
- 单测覆盖：fingerprint.test.ts:16-34 4 例（缺失/不可解析、2.0.6、平台不符、fresh 接受）。**单测真值列**确认语义对。

**A = PASS。**

## B 专项（429 误分类）

- 改动 1（显式 reason）：accounts.ts:63 `case "RESOURCE_EXHAUSTED": return "QUOTA_EXHAUSTED"`——gRPC `RESOURCE_EXHAUSTED(8)` 走 quota 分支而非容量重试。✓
- 改动 2（消息扫描顺序）：accounts.ts:75-77 `if (lower.includes("quota")) return "QUOTA_EXHAUSTED"` 排在 capacity/overloaded 之前（accounts.ts:80）。**Google canonical 消息 `"Resource has been exhausted (check quota)."` → 小写含 `"quota"` → 命中第一段 → 锁定+轮换**。
- 单测锁定：accounts.test.ts:1205-1211 `routes canonical quota 429 wording to QUOTA_EXHAUSTED, not capacity retry`——4 个断言全过：canonical 消息、显式 RESOURCE_EXHAUSTED、纯容量措辞 `Model capacity exhausted`、纯容量措辞 `This model is overloaded` 各自路径正确。**判定路径证据确凿**。
- 副效应：`accounts.ts:92` `if (lower.includes("exhausted") || lower.includes("quota"))` 现在 `quota` 半边成死代码（被 75 拦截），但 `exhausted` 半边仍可达（处理"无 quota 词但含 exhausted"的边角）。`parseRateLimitReason` 测试 `quota exhausted for today` (line 1197) 仍走 75 路径——向后兼容。
- 落点行为：QUOTA_EXHAUSTED 跳过 plugin.ts:2337 的 `attempt === 1` 1s 快速重试（`&& rateLimitReason !== "QUOTA_EXHAUSTED"`），改走 `markRateLimitedWithReason`（accounts.ts:633-658，按 `[60s, 5m, 30m, 2h]` 锁账户）。**锁定+轮换路径已串通**。

**B = PASS。**

## C 专项（主 fetch 超时 + AbortError 透传）

- 超时常量：plugin.ts:1061 `const CONTENT_REQUEST_TIMEOUT_MS = 120_000`。
- 套用形态：plugin.ts:2220-2233 — AbortController + setTimeout + AbortSignal.any 组合 `[headerTimeoutController.signal, callerSignal]`。
- **调用方 signal 保留**：line 2224 `const callerSignal = prepared.init.signal ?? undefined`；line 2225-2227 `callerSignal ? AbortSignal.any([..., callerSignal]) : headerTimeoutController.signal`；line 2230 `{ ...prepared.init, signal: fetchSignal }` spread 后覆盖为组合 signal。**`AbortSignal.any` 用法正确**，Node ≥20 满足。
- **AbortError 上抛位置**：plugin.ts:2660 catch 开头，line 2661-2665 `getTokenTracker().refund(account.index); tokenConsumed = false;` 在 line 2669-2671 `if (error.name === "AbortError") throw error;` **之前**——**已扣额度必先退再上抛**，无泄漏。
- **超时中止不进 AbortError 分支**：line 2222 `headerTimeoutController.abort(new Error(...))` 自定义 Error（name=`"Error"`，非 `"AbortError"`）——fetch 中止时 throw 的错误继承 abort reason 的 name（`"Error"`），catch 的 `error.name === "AbortError"` 不命中 → 走 next-endpoint 正常回退。**有意设计**，与 brief 不冲突。
- **未套用 quota.ts:166 既有 fetchWithTimeout 的偏离已申报**：spread `{...options, signal: controller.signal}` 会覆盖 `options.signal`，会丢调用方取消能力。报告已声明。**偏离合理**。
- 120s vs 30s：上游 agy-429-path-review.md Q5 建议 30s（stream 头到达毫秒级），但 CONTENT_REQUEST_TIMEOUT_MS 仅兜底**响应头等待**（finally clearTimeout 头到即撤防），流式头到达毫秒级不受影响；非流 generateText 长上下文响应可能 >30s，120s 与 IMAGE_TIMEOUT_MS（120s，对齐附属端点完整 generateContent 惯例）一致。**判断合理，理由充分**。

**C = PASS。**

## D 专项（resolver tier 白名单）

- 改动：model-resolver.ts:233 `/^gemini-3\.(?:5|6|7|8)-flash(?:-|$)/i`，新增 `7`。
- 误匹配核查（手工 + 思路）：
  - `gemini-3.7-flash` / `gemini-3.7-flash-low` / `gemini-3.7-flash-high` / `gemini-3.7-flash-extra-low` → ✓ 匹配
  - `gemini-3.0-flash` / `gemini-3.1-flash` / `gemini-3.2-flash` / `gemini-3.3-flash` / `gemini-3.4-flash` / `gemini-3.9-flash` / `gemini-3.10-flash` → ✗ 不在字符类中
  - `gemini-3.7-pro` / `gemini-3.7-pro-low` → ✗ 需 `-flash`，不匹配
  - `gemini-2.7-flash` → ✗ 需 `gemini-3.`，不匹配
  - `gemini-3.7-flashers` → ✗ `(?:-|$)` 需 `-` 或 EOL
  - `claude-3.7-flash` / `openai-3.7-flash` → ✗ 需 `gemini-`
  - `gemini-3.7-flash-image` → 匹配（`flash-` 后是 `image`，满足 `(?:-|$)`，再由 `IMAGE_GENERATION_MODELS` / `isImageModel` 早退处理，不致污染内容请求）
- 单测：model-resolver.test.ts:47-64 `honors the preferred tier for Gemini 3.7/3.8 flash catalog variants` 锁定 3.7/3.8 的 high variant 解析正确。✓
- 配合新加的 `availableModelIds` 路径（`selectAvailableModelId`，model-resolver.ts:16-44），3.7 走 catalog tier 流程。

**D = PASS。**

---

## QUALITY

### 复用侦察（抽查）
| 项 | 报告声称 | 实证 |
|---|---|---|
| A 用 `semverLess` | ✓ | version.ts:32-40 本 commit 新 export；fingerprint.ts:14 import；test_models 未重复实现 |
| A 用 `collectCurrentFingerprint` | ✓ | accounts.ts:8 import；fingerprint.ts:118-139 既有；**非**新写 |
| B 用 `parseRateLimitReason` 既有三分段 | ✓ | accounts.ts:45-103 既有结构，仅加 1 行 case + 1 段 `if` |
| C 沿用 AbortController+setTimeout+clearTimeout 模式 | ✓ | 同款模式已在 quota.ts:166、project.ts:194、image.ts:586、search.ts:292、version.ts:43 出现；plugin.ts:2220-2233 一致 |
| C 用 `AbortSignal.any` 组合调用方 | ✓ | 标准库（Node 20+），未加依赖 |
| C 超时值对齐 `IMAGE_TIMEOUT_MS` | ✓ | constants.ts:286 `IMAGE_TIMEOUT_MS = 120*1000` |
| D 既有正则字符类扩数字 | ✓ | model-resolver.ts:233，仅加 1 个 `7` |

**复用属实，无重复造轮。**

### 无投机性抽象
- 未新增 helper 模块、未新加 config 键、未新加 type union。
- `isFingerprintStale` 4 行正则 + 3 行判断 + 1 个 export，**正交**于既有 `isFingerprintStale` 调用点。
- `selectAvailableModelId`（model-resolver.ts:16-44）由 WIP 引入，22 行+早退 + 兜底，**与既有 MODEL_FALLBACKS 平行而非替换**（model-resolver.ts:240 `catalogModel ?? MODEL_FALLBACKS[actualModel] ?? actualModel`），有兜底。
- C 的 fetch 兜底是 13 行内联，未抽 `fetchWithTimeoutMain` 之类**避免无用抽象**。✓

### 早退/条件短路
- A：`isFingerprintStale` 4 个早退分支（null UA、正则不匹配、版本旧、平台不符）→ 任一命中即返回 true。✓
- C：`AbortSignal.any` + callerSignal 短路（`callerSignal ? AbortSignal.any(...) : headerTimeoutController.signal`）——调用方无 signal 时不创建无意义组合。✓
- B：`lower.includes("quota")` 命中即 return，不再做后续 capacity/overloaded 扫描。✓

### 单测质量
- 新单测均测**真值列**而非"断言自己"：
  - `fingerprint.test.ts` 用真 UA 字符串、动态 process.platform、动态 `getAntigravityVersion()`。
  - `accounts.test.ts:1205-1211` 4 个真实错误消息+状态码。
  - `model-resolver.test.ts:47-64` 真实 `availableModelIds` 数组，3.7/3.8 各一条。
  - `image.test.ts` 3 个真实候选列表。
  - `request.test.ts:670-695` 用真 `prepareAntigravityRequest` 跑 + 反查 `result.init.body`。
- **未发现循环断言**（测自己构造的值）。

### 报告"验证"章节与 diff 对账
- typecheck 0 输出 ✓
- 29 files / 884 passed / 25 todo ✓
- build 产物含四修 ✓
- smoke 两条 27s/16s 贴 WORKING ✓
- 报告基线说明中"3 处既有失败" → 本次对齐为 project.test.ts:58/66/98（daily-first 顺序、PLATFORM_UNSPECIFIED metadata、sandbox endpoint 期望），实测当前断言确实对齐 WIP 行为。**承诺兑现**。

---

## 09-03 WIP 混合部分越权扫描

- `constants.ts`: ANTIGRAVITY_VERSION_FALLBACK 1.15.8 → 2.11.0；ANTIGRAVITY_ENDPOINT_DAILY 改 `daily-cloudcode-pa.googleapis.com`；新增 `ANTIGRAVITY_ENDPOINT_DAILY_SANDBOX`；LOAD_ENDPOINTS 顺序 prod-first → daily-first；FALLBACKS 重排；IMAGE_MODEL `gemini-3-pro-image` → `gemini-3.1-flash-image`；新增 `IMAGE_MODEL_CANDIDATES`。
- `version.ts:85-90` 新增 `if (semverLess(version, fallback)) setAntigravityVersion(fallback);`——远程版本被本机 2.11.0 底线钉住，**正是 brief 提到的"2.11.0 UA 底线"**，合理。
- `project.ts`: 444 行重写，引入 daily-first 发现、`fetchAvailableModels`、`extractManagedProjectId` 递归+resource name 抽取、`buildResult` 持久化 managedProjectId。**正是 brief 提到的"目录 tier 模型/daily-first 发现顺序"**。
- `image.ts`: 新增 `selectImageModel`（基于 catalog 选 Nano Banana 2 / Pro 候选）+ discovery.endpoint 前置到 endpointsToTry。**WIP 一致**。
- `quota.ts`: 删除独立 fetchAvailableModels，复用 `projectContext.availableModels`（cache hit）+ 兜底走 project.ts 暴露的同名 export。**减少重复**。
- `model-resolver.ts`: 大增 `availableModelIds` / `preferredThinkingLevel` / `selectAvailableModelId`；原 `startsWith("gemini-3-flash")` 改为 `^gemini-3(?:\.\d+)?-flash(?:-|$)` 增强版本门。**支撑 3.7/3.8 真正生效所必需**。
- `request.ts`: 加 `x-goog-api-key` 头删除（line 660-662）+ `availableModelIds` plumbing + `preferredThinkingLevel` 从 generationConfig.thinkingLevel 解析（嵌套 request/顶层 fallback）。**WIP 一致**。
- `config/updater.ts`: `provider.google` → `provider[ANTIGRAVITY_PROVIDER_ID]`。**ANTIGRAVITY_PROVIDER_ID = "google"**（constants.ts:157），语义无差，code 风格统一。
- `types.ts`: ProjectContextResult 加 `effectiveEndpoint` + `availableModels`。**WIP 一致**。
- `request-helpers.ts`: 抽取 `thinkingLevel` 而非 `thinkingBudget`（line 845-848），与 OpenCode 3.x variant 形态对齐。
- `config/models.ts`: 新增 5 个模型定义（`antigravity-gemini-3.1-pro`、`-3.5-flash`/`-3.6-flash`、`-3.1-flash-image`/`-3-pro-image`）。
- `plugin.ts` 集成：line 1423-1440 `imageDiscovery` plumbing；line 1433 `authRecord = projectContext.auth`（WIP 可持久化 managed project，必要）；line 1452 `authRecord.access!`（承接 line 1419 null check）；line 1651 `apiKey: "antigravity-oauth-placeholder"` + line 660-662 `headers.delete("x-goog-api-key")` 配对；line 2148-2153 `requestEndpoints` 把 `projectContext.effectiveEndpoint` 前置。

**未发现越权或可疑改动**——所有 WIP 改动都聚类于"AGY 2.0 模型目录/daily-first 发现/UA 2.11.0 底线"主题，与 brief 描述的"09-03 WIP（UA 底线+stale 守卫+tier 白名单）"范围一致。`apiKey: ""` → `"antigravity-oauth-placeholder"` 与 x-goog-api-key 删除配对，行为等价（关键 API 不再误把空字符串当 key 转发，placeholder 也被立即删除），但**改动了 OpenCode AI SDK 可见的 provider.apiKey**——非功能但需注意，**Minor 风险**（终审若上线后用户配置出现 provider.apiKey 展示问题，可补 brief）。

---

## Findings

| 档位 | file:line | 描述 |
|---|---|---|
| Minor | accounts.ts:92 | 消息扫描 `if (lower.includes("exhausted") || lower.includes("quota"))` 的 `quota` 半边被 line 75 截胡成死代码；`exhausted` 半边仍可达。建议下次清理（行内加注释说明"裸 exhausted 走 quota"亦可） |
| Minor | plugin.ts:1061 | `CONTENT_REQUEST_TIMEOUT_MS = 120_000` 与上游 Q5 建议 30s 偏离；理由（流头到 ms 级不受影响、非流 30s 误杀）已写明；建议在 plugin.ts:1061 块注释加 `// 受上游 Q5 30s 建议 vs IMAGE_TIMEOUT_MS 120s 惯例分歧影响——选 120s，stream 头到即 clearTimeout`（或同步到 docs/decisions/） |
| Minor | plugin.ts:1651 | `apiKey: "antigravity-oauth-placeholder"` 非空字符串改变了 provider.apiKey 的语义展示（vs `""`），虽被 prepareAntigravityRequest line 662 即时删除 x-goog-api-key 抵消，但**OpenCode 侧 provider 视图会显示 placeholder**。建议在 docs/changelog/或 provider 适配层注释 |
| Minor | fingerprint.ts:90-99 | `generateFingerprint` 仍随机平台（`randomFrom(["darwin", "win32"])`），capacity retry 时（plugin.ts:2274 附近）生成的指纹平台可能与 host OS 不符。**报告已申报"保持其余字段生成逻辑不变"**，故暂不修；若 strict 平台一致是后续要求，需改 generateFingerprint 用 `os.platform()` 兜底 |
| Minor | accounts.ts:63 | `case "RESOURCE_EXHAUSTED": return "QUOTA_EXHAUSTED"` 把 gRPC `RESOURCE_EXHAUSTED(8)` 一律路由到 quota 分支。Google 实际中 `RESOURCE_EXHAUSTED` 同时涵盖 quota + 短窗口 rate-limit；上游 Q5 提示这一约定不严格。建议在 case 上加注释"按 Google 现有惯例 RESOURCE_EXHAUSTED ≈ quota；RATE_LIMIT_EXCEEDED 走 rate limit"，便于未来回查 |

无 Critical / Important。

---

## Cannot verify from diff

1. **真实冒烟 27s / 16s WORKING**：本环境无 AGY 凭证 + 无外网授权，无法复跑。报告的 `> build · antigravity-gemini-3.7-flash → WORKING` 摘录由 `script/test-models.ts` 实际 `opencode run` 输出，逻辑上对应 model 200 OK + 内容回 WORKING。**置信度中等-高**（脚本逻辑正确，无作弊空间）。
2. **账户指纹运行时自动修复为 `2.11.0 win32/x64`**：本环境无 `~/.config/opencode/antigravity-accounts.json` 可验；实现路径（fingerprint.ts:149-155 → accounts.ts:372 → collectCurrentFingerprint 用 `os.platform()="win32"` + `getAntigravityVersion()="2.11.0"`）逻辑闭环。
3. **`x-goog-api-key` 删除在真实 AGY 调用中是否仍被某些 SDK 重新塞回**：`request.ts:660-662` 在 prepare 阶段删除，但若 @ai-sdk/google 在 init 后的下游链路再 set，需进一步审计（无 source 可查，本仓库无该 SDK 源码）。
4. **projectContext.availableModels 在 fail-open 路径是否真传 200 字段**：WIP 中 fetchAvailableModels 返回 null 时 plugin.ts:1437 `availableModelIds: undefined`，被 model-resolver.ts:21 早退（`if (!availableModelIds) return undefined`）→ 回落 MODEL_FALLBACKS——逻辑正确，但 catalog 空账户下的真实 3.7/3.8 体验需现场验。

---

## 结论

**APPROVE**

- A/B/C/D 四修均按 brief 落地，diff 证据、tests 证据、build 证据三向闭环。
- 偏离项（`getAntigravityVersion()` 替字面 `ANTIGRAVITY_VERSION_FALLBACK` / 120s 替 30s / AbortSignal.any 替裸 fetchWithTimeout）皆有合理工程理由，报告已逐条申报。
- 09-03 WIP 混合部分与 AGY-FIX 主题一致（daily-first / 2.11.0 UA 底线 / 目录 tier 模型 / x-goog-api-key 删除），未越权，**推荐终审抽查 IMAGES / constants.ts WIP 改动对生产行为的影响**。
- 5 条 Minor 发现不阻塞合并；终审 triage 时可一并考虑。
- 编排者应注意：未 `npm run push`、未生成 handoff；fork HEAD 已在 main + 2 ahead。

冒烟结果 + accounts.ts 单元测试 + fingerprint.test.ts + model-resolver.test.ts 已提供足够回归覆盖；建议 CI 在 main 跑 `npm run typecheck && npm test && npm run build` 三绿门槛（fork 已有 vitest，CI 配置请见 fork `.github/workflows/`）。
