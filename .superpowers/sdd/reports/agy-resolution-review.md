# AGY Resolution Review - opencode-antigravity-auth-fork

Date: 2026-09-04. Read-only review of C:\Users\UmR\Desktop\opencode-antigravity-auth-fork (src/).
Question: why did `antigravity-gemini-3.8-flash` and `antigravity-gemini-3.7-flash` get HTTP 429 with ~70s reset windows despite cached remainingFraction 0.92?

## Q1. Model id mapping (`antigravity-gemini-3.8-flash` -> upstream name)

Path: `src/plugin/transform/model-resolver.ts` `resolveModelWithTier()`:

- L195-196: `antigravity-` prefix stripped -> `gemini-3.8-flash` (same for 3.7).
- L106 TIER_REGEX `-(minimal|low|medium|high)$` does not match (no tier suffix), so `baseName` = full id.
- L210-211 `isGemini3` + `skipAlias` = true; L217-218 `isGemini3Flash` matches `^gemini-3(?:\.\d+)?-flash`. L220-227: flash without tier keeps the bare id (only `*-pro` gets `-low` appended). So `actualModel = "gemini-3.8-flash"` / `"gemini-3.7-flash"`.
- L233-240 catalog overlay: `catalogModel = selectAvailableModelId(baseName, level, availableModelIds)` where `availableModelIds = Object.keys(projectContext.availableModels)` (plugin.ts L2177-2179) - i.e. the live per-account catalog from `fetchAvailableModels` (project.ts L295-334). `selectAvailableModelId` (L16-44) only picks ids already present in that catalog (exact match first, else `<base>-low/-medium/...`). It cannot invent a model outside the account's catalog.
- Fork's 3.8 patch: L233 `usesCatalogTierVariants = /^gemini-3\.(?:5|6|8)-flash/` - note 3.8 IS whitelisted, 3.7 is NOT. Consequence is narrow: for 3.7 the `preferredThinkingLevel` (variant config) is ignored in favor of the tier suffix; catalog selection still runs for 3.7 via L237-239.

Could it produce an unentitled model? Two residual risks, both conditional:
1. Catalog fetch failure: `projectContext.availableModels` undefined (project.ts L468/L497-503 tolerate catalog-less contexts) -> `resolvedModel = actualModel` = bare `gemini-3.8-flash` (L240). Whether the bare id is accepted is then purely server-side.
2. Version-gating skew (see Q3): the catalog is fetched with the bumped UA 2.11.0 (discoveryHeaders -> getAntigravityHeaders, project.ts L214-220), so it can contain 3.8 tier ids gated to >=2.6 clients, while the actual generateContent call goes out with the account's stored fingerprint UA `antigravity/2.0.6`. The mapping then sends a catalog id the account is entitled to - but under a client version the server may treat as below the gate.

## Q2. Endpoint selection (daily vs prod)

- Default content endpoint: `ANTIGRAVITY_ENDPOINT = ANTIGRAVITY_ENDPOINT_DAILY` (constants.ts L63), used at request.ts L718 (`headerStyle === "antigravity"` -> daily).
- Actual per-request endpoint list: plugin.ts L2142-2147 = `[projectContext.effectiveEndpoint, ...ANTIGRAVITY_ENDPOINT_FALLBACKS]` deduped. `effectiveEndpoint` is whichever endpoint served `fetchAvailableModels` (project.ts L305-327, buildResult L381-410), tried in `ANTIGRAVITY_LOAD_ENDPOINTS` order: daily, daily-sandbox, prod, autopush (constants.ts L53-58).
- So after the UA bump, discovery succeeds on daily first, and content requests also hit daily first. This is consistent pre/post patch; the patch did not switch request endpoints. What DID change: with UA 2.0.6 the daily catalog hid `gemini-3.8-flash*` (constants.ts L77 comment), so resolution could not see tier ids; with 2.11.0 the daily catalog now returns them.
- Quota-vs-endpoint mismatch worth noting: the cached `remainingFraction 0.92` comes from `fetchAvailableModels` quotaInfo aggregated per family via `Math.min` across flash models (quota.ts L114-163, L301-322), i.e. the same daily catalog. The Gemini-CLI quota probe separately hits PROD `retrieveUserQuota` (quota.ts L180-197). Neither represents a per-minute bucket; quotaInfo.remainingFraction is a long-window (daily/weekly) balance. A 429 with ~70s reset is a rate-limit bucket, invisible to the cached 0.92 - the two facts are not contradictory.

## Q3. Fingerprint / UA mismatch (stored 2.0.6 Mac vs sent 2.11.0 Windows)

The bump does NOT reach generateContent requests:

- Content requests: plugin.ts L2176 passes `fingerprint: account.fingerprint`; request.ts L1437-1440 sets `User-Agent = fingerprint.userAgent` (stored value, e.g. `antigravity/2.0.6 darwin/x64`), falling back to session fingerprint only if null. `getRandomizedHeaders` (which would use 2.11.0) is only the fallback when the fingerprint has no UA (request.ts L1432, L1440).
- `Client-Metadata` and `X-Goog-Api-Client` are deliberately NOT sent on content requests (request.ts L1434-1436 comment: "AM only sends User-Agent on content requests"). So the "platform WINDOWS" part of `getAntigravityHeaders` never appears on content calls at all; ideType goes in the body via project metadata instead.
- `getAntigravityHeaders()` (2.11.0 Electron UA, platform from process.platform) is used ONLY by: project discovery `loadCodeAssist`/`fetchAvailableModels`/`onboardUser` (project.ts L214-220), image generation (image.ts L581), search (search.ts L287).
- Stickiness: stored fingerprint is reused on load (`acc.fingerprint ?? generateFingerprint()`, accounts.ts L362) and only regenerated after 3 capacity retries on an endpoint (plugin.ts L2266-2271). New fingerprints randomize darwin/win32 regardless of host OS (fingerprint.ts L90-97), so even a regenerated fingerprint can claim Mac on a Windows box.
- Body marker: `wrappedBody.userAgent = "antigravity"` (request.ts L1395) - constant, no version.

Assessment: the mixed identity is real and structural - discovery says `Antigravity/2.11.0` (Windows metadata), content says `antigravity/2.0.6 darwin/x64`. Whether Google applies stricter limiting for that mismatch is not decidable from this code. What IS decidable: the fork's own rationale (constants.ts L77: daily gates 3.8 behind >=~2.6 client versions) is enforced via UA, and the content path still sends 2.0.6 for this account. If the same version gate (or a deprecation throttle for old clients) is applied at generateContent, this account would be limited while a fresh account (fingerprint generated after the bump) would not. This is the strongest code-level explanation consistent with "catalog shows quota, server says 429".

## Q4. Retry amplification (one user request -> N upstream requests)

Yes, several stacked multipliers in plugin.ts's fetch loop:

1. Endpoint fan-out: up to 4-5 endpoints tried per account per request (L2142-2149).
2. Capacity retry loop: if the 429 is classified `MODEL_CAPACITY_EXHAUSTED`, the SAME endpoint is retried up to 3 extra times with 1s-8s backoff (`i -= 1; continue`, L2238-2273), then the fingerprint is regenerated and the next endpoint is tried. Worst case ~4 requests x 4-5 endpoints = 16-20 upstream calls for one user request.
3. Misclassification feeding (2): `parseRateLimitReason` (accounts.ts L66-84) checks `"resource exhausted"` BEFORE quota keywords, so Google's standard 429 body message "Resource has been exhausted (check quota)." classifies as MODEL_CAPACITY_EXHAUSTED - transient fast-retry instead of lock-and-rotate. Only an explicit `ErrorInfo.reason = "QUOTA_EXHAUSTED"` avoids this.
4. First-429 quick retry: attempt 1 sleeps 1s and retries the same endpoint (`i -= 1`, L2316-2346) before any account marking.
5. Empty-response retry: up to 4 attempts for non-streaming empty bodies (L2568-2600).
6. Claude-only warmup request (runThinkingWarmup L1999-2062) - not applicable to gemini, listed for completeness.

Against a per-minute bucket with a ~70s reset, multipliers (1)+(2)+(4) alone can drain it within seconds, and each retry re-hits the bucket before the reset window elapses.

## Q5. Does the code distinguish quota-exhausted vs per-minute 429?

Yes, partially:

- Body parsing `extractRateLimitBodyInfo` (plugin.ts L953-1023) reads `ErrorInfo.reason`, `RetryInfo.retryDelay`, `quotaResetDelay`/`quotaResetTimeStamp` metadata, and a "reset after <duration>" message regex. The ~70s windows in the account state file almost certainly came from retryDelay/quotaResetDelay via `markRateLimitedWithReason` (accounts.ts L623-648).
- Classification `parseRateLimitReason` (accounts.ts L45-93) returns QUOTA_EXHAUSTED / RATE_LIMIT_EXCEEDED / MODEL_CAPACITY_EXHAUSTED / SERVER_ERROR / UNKNOWN, with different backoffs (accounts.ts L95-122) and different behavior (QUOTA_EXHAUSTED skips the 1s quick retry, plugin.ts L2316).
- Caveat (same as Q4.3): the message heuristic order sends the canonical gRPC RESOURCE_EXHAUSTED message ("resource exhausted") to the capacity bucket, so in practice the quota-vs-rpm distinction only works when Google includes an explicit ErrorInfo reason string. When it does not, a real quota 429 is treated as transient capacity and amplified.

## Summary of most-likely 429 causes (ranked by code evidence)

1. Content requests still carry the old fingerprint UA `antigravity/2.0.6` (request.ts L1437-1440 + accounts.ts L362) while the fork's own gate analysis says 3.8 needs >=~2.6; the 2.11.0 bump only covers discovery/image/search. Fix direction: regenerate the account fingerprint after a version bump, or source the content UA from `getAntigravityVersion()` instead of the frozen stored fingerprint.
2. Retry amplification (plugin.ts L2238-2273, L2316-2346) combined with the "resource exhausted" -> capacity misclassification (accounts.ts L70) can burn a small per-minute bucket in one user request.
3. The cached 0.92 is a long-window quota from the daily catalog; it says nothing about per-minute rate limits, so its coexistence with 429 is expected, not a bug.

Status: DONE
