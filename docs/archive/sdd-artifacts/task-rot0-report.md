# Task ROT-0 Report — 顺手批（surfaces 路径 / CHANGELOG 解冻 / 双 TLS / runtime-services 核销）

## 1. Spec 逐条落实

| Spec | 要求 | 状态 | 落实说明 |
|---|---|---|---|
| **Spec 1** | `surfaces.md:50` 路径修正 + 全文抽查 | ✅ COMPLETED | `:50` 行 `src/crates/test-support` 修正为 `src/crates/support/test-support`；全文检查 21 个 crate 路径，除 `:50` 外其余均与 `src/crates/` 真实目录严格一致。 |
| **Spec 2** | `CHANGELOG.md` 补 `## [Unreleased]` 大事记 | ✅ COMPLETED | Keep a Changelog 格式（Security / Added / Changed / Removed），条目单行 + commit 锚点，覆盖 P1 安全轮、T2-1 CI、T2-2 大删除（~40k 行）、T1 安全收尾五项、Growth Core 记忆系统线、T3-4 Gemini 视觉、ROT 防腐（rot-budget + 家规 7 + ROT-1 去重）。未改动 0.2.10 及更早段落。 |
| **Spec 3** | 裁 `native-tls` + 统一 `rustls` | ✅ COMPLETED | 根 `Cargo.toml:98` reqwest features 移除 `"native-tls"`（保留 `"rustls"` 及其余 features）；`src/crates/assembly/core/src/service/review_platform/http.rs:11` `.use_native_tls()` 调整为 `.use_rustls_tls()`；`Cargo.lock` 随之剔除 `native-tls` / `tokio-native-tls` / `hyper-tls` / `openssl` 等传递依赖。MSVC 双 check 0 errors 通过。 |
| **Spec 4** | `runtime-services` 核销 | ✅ COMPLETED | 详见下方「核销证据」节，零代码改动。 |

---

## 2. 复用侦察

1. **CHANGELOG 素材来源**：
   - 全量对照 `git log` 与 `.superpowers/sdd/progress.md` 历史台账取证，提取真实 commit hash 锚点（P1: `007e513`/`7fa7d62`/`26a15a7`/`f42451d`；T2-1: `3a6695f..05905ee`；T2-2: `e65d98e..89abea6`；T1: `0b656dd`/`ea55c80`/`3891080`/`61ba73a`/`1d1d4ff`；Growth: `7e96126`/`5eb5fbf`/`fd61f5e`/`9f261cd`/`1e1f009`；T3-4: `80651bf`；ROT: `964afda`/`ded3544`/`9721f75`/`f5e7922`）。
2. **TLS 依赖与调用侦察**：
   - 根 `Cargo.toml` 声明 `rustls = { version = "0.23", default-features = false, features = ["ring", "std", "tls12"] }`；
   - `ai-adapters/src/client/http.rs` 既有生产代码已使用 `Client::builder().use_rustls_tls()`；
   - 全仓 grep `native-tls` / `native_tls` 确认无任何 crate 级直接 API 消费，仅 `review_platform/http.rs:11` 存在 reqwest builder 方法调用 `.use_native_tls()`，将其对齐改为 `.use_rustls_tls()`。

---

## 3. 核销证据（Spec 4：runtime-services）

- **Crate 规模与文件**：
  - `src/crates/execution/runtime-services/src/lib.rs`（222 行）
  - `src/crates/execution/runtime-services/src/test_support.rs`（84 行）
  - `src/crates/execution/runtime-services/tests/runtime_services_contracts.rs`（99 行）
  - `AGENTS.md`（21 行）+ `Cargo.toml`（15 行），全 crate 总计 441 行（Rust 源码 405 行）。
- **消费方链条**：
  1. `src/crates/assembly/core/Cargo.toml:144` 声明 `northhing-runtime-services` 作为 `product-full` 依赖；
  2. `src/crates/assembly/core/src/product_runtime/runtime_services.rs` 实现 `RuntimeServicesProvider` 并调用 `RuntimeServicesBuilder` 组装 Typed Services；
  3. `src/crates/execution/agent-runtime/Cargo.toml:14` 引入依赖；
  4. `src/apps/desktop` 通过 core 间接集成；
  5. 契约测试：`runtime-services/tests/runtime_services_contracts.rs` 6 项自动化测试通过；`assembly/core/tests/product_assembly.rs` 包含集成断言。
- **架构角色**：
  - 位于六层架构第 5 层（Execution Primitives），提供类型安全的服务注册与可用性探测机制（`RuntimeServicesRegistry`、`RuntimeServicesBuilder`、`RuntimeServicesProvider`），连接第 6 层端口与上层装配，职责清晰且有明确消费者，无需清理。

---

## 4. 验证输出

### 1. `cargo check --workspace` (MSVC wrapper)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
```text
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
warning: `northhing` (bin "northhing") generated 5 warnings
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.09s
```

### 2. `cargo check -p northhing` (MSVC wrapper, 家规 6)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
```text
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
warning: `northhing` (bin "northhing") generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.97s
```

### 3. `node scripts/check-core-boundaries.mjs`
```text
Core boundary check passed.
```

### 4. `pnpm run check:rot`
```text
> northhing@0.2.10 check:rot E:\agent-project\.worktrees\northing-rot0
> node scripts/verify-rot-budget.test.mjs && node scripts/verify-rot-budget.mjs

✔ compliant fixture exits 0 and reports success (98.3696ms)
✔ grep count exceeding ceiling fails and exits 1 with guidance message (93.0135ms)
✔ unregistered file exceeding 800 lines fails and exits 1 (96.5407ms)
✔ registered god-file exceeding ceiling fails (6.4572ms)
✔ exempt file generated_locale_contract.rs >800 lines is permitted without manifest entry (7.1186ms)
✔ actual workspace rot budget passes with current manifest (329.9115ms)
ℹ tests 6
ℹ suites 0
ℹ pass 6
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 637.5018
Rot budget verification passed (4 grep rules, 7 god-file rules checked across 1362 files).
```

### 5. `git diff --stat` (commit 63c34b2 对照 BASE cc0eba2)
```text
 CHANGELOG.md                                       | 28 +++++++
 Cargo.lock                                         | 92 +---------------------
 Cargo.toml                                         |  2 +-
 docs/status/surfaces.md                            |  2 +-
 .../core/src/service/review_platform/http.rs       |  2 +-
 5 files changed, 34 insertions(+), 92 deletions(-)
```

---

## 5. 偏离声明

- **零未授权偏离**。
- `review_platform/http.rs` 调整 `.use_native_tls()` -> `.use_rustls_tls()` 属于 Spec 3 明确要求的 reqwest TLS 后端收敛改动，与 `ai-adapters` 保持一致。
