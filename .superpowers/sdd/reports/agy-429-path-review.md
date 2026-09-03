# agy fork 429 路径只读审查

仓库：`C:\Users\UmR\Desktop\opencode-antigravity-auth-fork`（TypeScript / `src/` 行号）
时间：2026-09-04（今晚两次 antigravity-gemini-3.8/3.7-flash 零 chunk 挂起事件的事后分析）
范围：只读，不改任何代码。分析 `src/plugin.ts`、`src/plugin/request.ts`、`src/plugin/request-helpers.ts`、`src/plugin/accounts.ts`。

---

## 关键认知速览（一句话总结每个问题）

1. **429 行为链** = 解析原因 → 三档处理（capacity 指数重试 / RPM 快速重试+切账户 / quota 标记+持久化）→ 若所有账户被标则 park `sleep(waitMs, abortSignal)`。**默认上限 300s (max_rate_limit_wait_seconds)，超限抛错。**
2. **响应 AbortSignal** = ✓（sleep/while/check 三层都看 signal）✓ 主 fetch 通过 `init.signal` 浅 copy 拿到，但**主 fetch 无自己的 timeout**（这是今晚 28-44s 退不出根因；外部信号到位时实际应 < 1s）。
3. **零 chunk 挂起其他路径** = 无 promise never-resolve 分支；最大风险 = **主 fetch 无 signal.timeout 兜底 + 捕获块把 AbortError 当作普通 error 重新走 switch-account 循环**。
4. **持久化后再加载** = `accounts.ts` 加载时**直接装入内存**，但**不立即 park**；只有当 `getCurrentOrNextForFamily` 检测到所有账户都 still-rate-limited，才在主循环里 park。
5. **修复最小集** = 给主 fetch 加 `AbortSignal.timeout()`；catch 区分 `AbortError` 直接抛；显式 audit `prepareAntigravityRequest` 的 signal 字段。

---

## Q1：429 后的精确行为链（src/ 行号 + 证据）

**入口**：`src/plugin.ts:1646` `async fetch(input, init)` 是 ProviderFetchHook。`init.signal` 在 `:1686` 取到：

```ts
const abortSignal = init?.signal ?? undefined;
```

**检测**：`src/plugin.ts:2219` 命中 `429 || 503 || 529` 后进入以下三档分支：

### A. STRATEGY 1：capacity / server error（短暂重试，不污染全局 backoff 计数器）
- `:2235-2274`：1s → 2s → 4s → 8s 指数 + ±10% jitter；最多 3 次同一端点，超出则重新生成 fingerprint 继续。
- sleep 路径：`:2256` `await sleep(waitMs, abortSignal)` ✓ 响应 abort。
- 触发条件：`parseRateLimitReason()` 返回 `MODEL_CAPACITY_EXHAUSTED` 或 `SERVER_ERROR`（`accounts.ts:51-54`，529/503→capacity，500→server）。

### B. STRATEGY 2：RPM / quota / unknown — 标记 + 切账户
1. `:2280-2285` `getRateLimitBackoff()`（`plugin.ts:1091-1127`）计算指数退避（默认 max 60s，2 分钟 dedup 窗口）；同时 `calculateBackoffMs()` 给一个 `smartBackoff`。
2. **写回并持久化**：
   - `:2348` `accountManager.markRateLimitedWithReason(...)` → `accounts.ts:623-648`：把 `account.rateLimitResetTimes[key] = now + backoffMs`（model-specific key 如 `gemini-antigravity:gemini-3-flash`，TTL 默认 1h）。
   - `:2350` `accountManager.requestSaveToDisk()` → `accounts.ts:1013-1021`：1s debounce 后落盘。
   - `:2311` `getHealthTracker().recordRateLimit(...)`。
3. **首次 RPM（attempt===1）**：`:2316-2346` — toast → `await sleep(FIRST_RETRY_DELAY_MS=1000, abortSignal)`。
4. **cache_first 路径（保留 prompt cache）**：`:2321-2335` — 若 `effectiveDelayMs <= max_cache_first_wait_seconds`（默认 60s）→ `markRateLimited` + `sleep(effectiveDelayMs, abortSignal)` 等待同账户恢复。**这是 await 时间可能长的隐藏路径。**
5. **多账户切换**：`:2359` `await sleep(SWITCH_ACCOUNT_DELAY_MS=5000)`、`:2413` 同上。
6. **单账户指数 backoff**：`:2430-2452` `expBackoffMs = 1000 * 2^(attempt-1)`，max 60000ms。

### C. 全部账户被限速 → 主循环 park
- `:1735-1862` `while (true)`：
  - `:1757` `getCurrentOrNextForFamily()` 返回 null 表示无可用账户。
  - **软配额路径**：`:1787-1814` `getMinWaitTimeForSoftQuota(...)` → `await sleep(softQuotaWaitMs, abortSignal)`。
  - **硬限速路径**：`:1819-1862` `getMinWaitTimeForFamily(...) || 60_000`（fallback 60s）→ `:1839-1852` **若 waitMs > maxWaitMs（默认 300s）抛 `throw new Error(...)`** ✓ 给调用方一个清晰错误。**否则 toast 一次 + `await sleep(waitMs, abortSignal)` + continue**。

### sleep 内部响应 abort（`plugin.ts:1185-1209`）
```ts
function sleep(ms, signal) {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) reject(...);
    const timeout = setTimeout(() => { cleanup(); resolve(); }, ms);
    const onAbort = () => { cleanup(); reject(...); };
    signal?.addEventListener("abort", onAbort, { once: true });
  });
}
```
✓ 立即 reject、清理 timer + listener。

### isRateLimited 检查链
- 启动加载：`accounts.ts:357` `rateLimitResetTimes: acc.rateLimitResetTimes ?? {}` — **直接装入内存对象**，没有 park 副作用。
- 选择路径：
  - `getCurrentOrNextForFamily` `accounts.ts:491-575`：
    - `:558-567` 当前账户 → 检查 `isRateLimitedForHeaderStyle(current, family, headerStyle, model)` + `isAccountCoolingDown`，命中即跳过。
    - `:569-574` 下一个：内部 `getNextForFamily`，filter 掉 `isRateLimitedForFamily` / cooling down。
  - `isRateLimitedForHeaderStyle` `accounts.ts:195-213`：先 model-specific，再 base family，**且调用 `clearExpiredRateLimits` 自动清过期**（`:215-223`，删 `now >= resetTime` 的条目）。

**结论 Q1**：429 后的行为不是简单"抛错或重启"——是 **mark + 持久化 + 切换/重试/cancel-cache + 整轮 park** 的复合反应。**有上限（默认 300s 整轮，switch 5s，cache_first 60s 等待，单账户指数 max 60s）**但**没有针对主 fetch 自身的超时**。

---

## Q2：路径是否响应 AbortSignal？取消后 28-44s 符合预期吗？

### 应当响应（已实现）
| 位置 | 响应方式 |
| --- | --- |
| `plugin.ts:1185-1209` sleep() | ✓ abort event → 立刻 reject |
| `plugin.ts:1689-1693` checkAborted() + `:1737` while 顶部 | ✓ 每次循环顶部 throw |
| `plugin.ts:2212` `fetch(prepared.request, prepared.init)` | ✓ 通过 `prepareAntigravityRequest` 浅 copy `{...init}` 保留 `init.signal` 字段（`request.ts:639`）|
| `plugin.ts:2647` THINKING_RECOVERY_NEEDED catch | 部分响应，但详细见 Q3 |

### 不响应 / 兜底缺失（这里是坑）
1. **主 fetch 无自己的 `AbortSignal.timeout(...)`** —— `grep "AbortSignal.timeout"` 在主路径 (`plugin.ts:2212`) 命中数为 0；其他附属端点（image / search / project / quota / version / antigravity.oauth）均有 `fetchWithTimeout` + `AbortSignal.timeout(...)` 兜底（`request-helpers.test.ts:...` / `project.ts:194-198` / `quota.ts:166-170` / `image.ts:586` / `search.ts:292` / `version.ts:43-45` / `oauth.ts:120-126`）。
2. **catch block 把 AbortError 当普通 error** —— `plugin.ts:2639` `catch (error)` 通用捕获：
   ```ts
   } catch (error) {
     if (tokenConsumed) getTokenTracker().refund(...);
     if (error.message === "THINKING_RECOVERY_NEEDED") { ... }
     if (i < requestEndpoints.length - 1) { lastError = error; continue; }
     trackAccountFailure(...); shouldSwitchAccount = true; break;
   }
   ```
   → 如果 fetch 被 signal abort 抛 AbortError，这里**会走到 try-next-endpoint / switch-account**分支，再 sleep（虽然 sleep 立刻抛，但因为 shouldSwitchAccount 路径在 `:2693-2716` 还可能 throw `lastError`）。
3. **`transformAntigravityResponse` streaming 路径**（`request.ts:1583`）`response.body.pipeThrough(streamingTransformer)` — 依赖 undici 的 abort propagation；signal 触发时通常几秒内传播完成，**但如果 fetch 还在等 response 头，永远进不到 pipeThrough**。

### 28-44s 数字分析
| 推测来源 | 估算 |
| --- | --- |
| 调用方（OpenCode AI SDK）自身的 stream read timeout | 通常 30s（可配），与今晚观测一致 |
| 编排者视角的"28-44s 后取消生效" | ≈ 调用方放弃流读取触发 abort → 传播到插件 → sleep 立即抛 → throw → plugin fetch 返回 AbortError |
| 插件主动抛 abort 时的延迟 | < 1s（sleep 设计如此） |

**结论 Q2**：28-44s **来自调用方（OpenCode）的 stream read 兜底超时，不是插件 sleep 的拖时**。插件本身的设计**应当 < 1s 退出**，但前提是调用方 abort 信号到位。**如果调用方根本不传 signal**（或 signal 不触发），**主 fetch 没有自我 timeout，会永久挂起**——这正是旧坑的余毒。

---

## Q3：零 chunk 挂起的其他可能路径

主路径 `request.ts` / `request-helpers.ts` / `accounts.ts` 内的 `new Promise` 使用：

| 位置 | 内容 | 永不 resolve 风险 |
| --- | --- | --- |
| `accounts.ts:1027` `flushSaveToDisk` | 防抖 timer 触发 `saveToDisk()` 然后 resolve | 无 |
| `project.ts:98-100` `wait(ms)`（项目发现） | `setTimeout(resolve, ms)`，**不接 abort** | 是孤立超时，无 signal 链接 |
| `project.ts:194-198` `fetchWithTimeout` | ✓ 自身 AbortController（`signal: controller.signal` + setTimeout abort） | 无 |
| `quota.ts:166-170` 同上 | ✓ | 无 |
| `search.ts:284-292` 同上 | ✓ | 无 |
| `version.ts:43-45` 同上 | ✓ | 无 |
| `oauth.ts:120-126` 同上 | ✓ | 无 |
| `plugin.ts:1185-1209` sleep | ✓ | 无 |
| `plugin.ts:2693-2717` 切账户 throw `lastError` | 路径正确 | 无 |

**主 fetch（`plugin.ts:2212`）的具体风险**：
```ts
const response = await fetch(prepared.request, prepared.init);
```
- 如果服务端不返回响应头（TCP 连接 hang / HTTP/2 中间盒丢包）—— **没有 signal.timeout 兜底**。
- `prepared.init.signal` 走的是外部 `init?.signal`（`request.ts:639` shallow copy）—— **依赖调用方传**。

**`transformAntigravityResponse` 行 1592 `await response.text()`**：如果上述 fetch 已经拿到 response 但 body 流不投递，会等；不过这种情形下客户端能 cancel 流就 OK。

**`transformAntigravityResponse` 行 1583 streaming pipeThrough**：未配置 abort controller，依赖 fetch 的 abort —— 同一根因。

**主路径唯一不传 abortSignal 的 fetch**：内部 helper `:509`（account verification）自己 `AbortController` + 20s timeout。✓ 不影响主请求路径。

**结论 Q3**：**没有 promise 永不 resolve 的死分支**；但 **fetch 自身无超时兜底 + catch 把 AbortError 当普通 error** 是当前最大挂起诱因。

---

## Q4：rateLimitResetTimes 持久化后是否"开头检查并 park"？

### 加载时（`accounts.ts:357`）
```ts
rateLimitResetTimes: acc.rateLimitResetTimes ?? {},
```
内存对象直接复用磁盘值。**启动时无 park、无恢复 sleep。**

### 首次请求入口
- `plugin.ts:1757-1786` — 调 `getCurrentOrNextForFamily`：会按 `isRateLimitedForHeaderStyle()` 过滤被标账户，**未过期的不返回**。
- **如果仍有可用账户**：直接使用，无 park。
- **如果全部账户被标（且仍未来到期）**：进入 `:1787-1862` 的 park 分支 —— `await sleep(waitMs, abortSignal)`，**这里是真的 park**。

### 持久化时机
- `accounts.ts:1013-1021` `requestSaveToDisk()`：debounce 1s → `executeSave()` → `:976-1011` `saveToDisk()` → `saveAccounts(storage)`。
- 调用点：`plugin.ts:2350`、`:1894` 等多处；都走 debounce。

### 跨进程结论
**账户一旦被 429 标记，后续窗口期内该账户的所有请求自动跳过**（由 `getCurrentOrNextForFamily` filter 实现，无 park 副作用）。**只有当所有账户都被标**才 park。**单账户重复 429 → 1/2/4/8/.../max 60s 指数 backoff（行 2432）；连续多次 QUOTA_EXHAUSTED 命中 → backoff 数组 `[60s, 300s, 1800s, 7200s]` (`accounts.ts:28`)，第 4 次可达 2 小时**。**这是用户问题 4 的 max 边界 — 但默认 `max_rate_limit_wait_seconds=300` 会在 `:1840` 提前抛错，不会真等 2h。**

---

## Q5：最小改法（只写方案，不动代码）

按 Ponytail ladder：能改一行的不写一坨。

### 必做（最低风险）
1. **主 fetch 加 timeout 兜底** —— 在 `plugin.ts:2212` 改为 `await fetchWithTimeout(prepared.request, prepared.init, REQUEST_TIMEOUT_MS)`，新增 `const REQUEST_TIMEOUT_MS = 30_000`（或读 `config.request_timeout_ms`）。**这是根因修补。** 复用现有 `fetchWithTimeout` 的 5 行业务代码，不引入新抽象。
2. **catch block 区分 AbortError** —— `plugin.ts:2639` catch 中开头加：
   ```ts
   if (abortSignal?.aborted || (error instanceof Error && error.name === "AbortError")) throw error;
   ```
   一行。保证外部 abort 时不进入 switch-account 的尾递归再 sleep。

### 可选（增强）
3. **`prepareAntigravityRequest` 显式 audit signal** —— `request.ts:639` 后追加 `const requestSignal = init?.signal;`，所有 `init` 返回点显式 `init: { ...baseInit, headers, signal: requestSignal }`。当前浅 copy 已经保留，但显式更稳；不要为了 perf 去掉它。
4. **`max_rate_limit_wait_seconds` 暴露给 AI SDK 默认更短**（如 60-120s）—— schema 已支持（`config/schema.ts:254` 默认 300），调小一行。
5. **`max_cache_first_wait_seconds` 默认收缩** —— `config/schema.ts:322` 当前默认 60s，可缩到 15-20s。

### 不建议（过度）
- 给每个 sleep 单独包 abort helper（已有 18 处 `await sleep(..., abortSignal)`，重写 = 18 行 diff；零增益）。
- 把 "while(true)" 改成 await-loop 递归重试（结构稳定后没必要改）。
- 新增 "rate limit park 队列" 单例（多账户并发请求才需要，目前编排者只用 1 个 3.8 并发）。

---

## 排查今晚两单失败的工作链路（直接复用上面的结论）

请求时间 `21:21:19 / 21:21:59`、reset `21:22:34 / 21:23:08`，reset 窗口 ~70s。**这 ~70s < 默认 max_rate_limit_wait_seconds=300s → 应当走 :1861 sleep(70s) 路径**。

如果用户视角 28-44s 才观察到取消生效：
- 调用方（OpenCode）的 stream read timeout = 28-44s（用户或 SDK 配置）；
- 此时调用方 abort signal 触发 → sleep 立刻抛 → catch 路径走 :2701 `transformAntigravityResponse` 或 :2713 `throw lastError` → 调用方收到 AbortError；
- 用户感觉"取消生效"时间 ≈ 调用方自己 stream read 的兜底超时，**与插件 sleep 设计无关**。

**真正的症结是调用方那边的 stream 兜底，而不是插件 sleep**。**修主 fetch 超时（Q5 #1）后即使用户没有 OpenCode 的 stream timeout，插件也能 ≤ 30s 主动放弃 / 返回 AbortError**。

---

## 状态

DONE
