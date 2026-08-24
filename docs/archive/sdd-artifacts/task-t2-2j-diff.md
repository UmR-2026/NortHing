diff --git a/CONTRIBUTING.md b/CONTRIBUTING.md
index decd680..05a649e 100644
--- a/CONTRIBUTING.md
+++ b/CONTRIBUTING.md
@@ -143,7 +143,6 @@ Common local checks:
 | --- | --- |
 | Repository metadata or GitHub config | `pnpm run check:repo-hygiene && pnpm run check:github-config && git diff --check` |
 | Frontend runtime or UI | `pnpm run type-check:web`, plus the nearest focused test when behavior changed |
-| Mobile web | `pnpm --dir src/mobile-web run type-check` |
 | Rust shared runtime or services | `cargo check --workspace`, plus a focused `cargo test` when behavior changed |
 | Desktop integration | `cargo check -p northhing` |
 | i18n resources or contract | use the matching i18n row in `AGENTS.md` |
diff --git a/README.md b/README.md
index 6fb46ca..66bf267 100644
--- a/README.md
+++ b/README.md
@@ -40,7 +40,7 @@ See [`AGENTS.md`](AGENTS.md) for the layered module index, backbone invariants,
 See [`docs/status/surfaces.md`](docs/status/surfaces.md) for the complete ledger of shipping vs frozen-experimental surfaces.
 
 **Shipping (v0.1.0)**: Slint desktop + installer.  
-**Frozen-experimental**: CLI, server, relay, mobile-web, MiniApp UI, SDLC harness.
+**Frozen-experimental**: CLI, server, MiniApp UI, SDLC harness.
 
 ## Tech Debt
 
diff --git a/docs/architecture/backend-roadmap.md b/docs/architecture/backend-roadmap.md
index ab78404..1f81f6b 100644
--- a/docs/architecture/backend-roadmap.md
+++ b/docs/architecture/backend-roadmap.md
@@ -112,10 +112,10 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 | 面 | 状态 | 规划 |
 |---|---|---|
 | `apps/server` | 位腐（源码 import core 但 Cargo.toml 未声明，编译不过；内含未接线 `ai_relay.rs`/`rpc_dispatcher.rs`） | T1-8 修复（删 ai_relay、修依赖）→ **T5 升格为进程外 core 宿主**（或新建 host，T5 时定） |
-| `apps/relay-server` | 已加固（fail-closed 绑定、自动 key、CORS localhost 默认，2026-08-04） | 维持；M5 解冻评估时按 surfaces 协议走 |
+| `apps/relay-server` | 已整删（T2-2 C5, commit f6a011b, PEND-1） | 随删除关闭；原维持/解冻评估规划失效 |
 | `apps/cli` | frozen（编译产物已有 CI：cli-package.yml） | T4（= K4b CLI 半）后评估解冻 |
 | MiniApp host | frozen（沙箱语义待修） | SW1-1 修复是任何 MiniApp 开放的前置 |
-| mobile-web/remote_connect | 已决删除（论题 v1.1） | TH-4 删除执行单入 T2-2；P1-4/P1-7/D-2 随删除关闭；将来移动需求 = T5 协议客户端重写 |
+| mobile-web/remote_connect | 已整删（TH-4 删除已执行，T2-2 C1-C7, commits fa88342..d16b037） | P1-4/P1-7/D-2 已随删除关闭；将来移动需求 = T5 协议客户端重建 |
 
 ---
 
@@ -126,7 +126,7 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 3. **ACP server**：`interfaces/acp` 已实现 stdio 服务端（`AcpServer<R>` over agent-client-protocol）——T5 协议候选之一，也是"多宿主/被嵌入"战略选项的零成本通路；
 4. **持久化**：会话/轮次/prompt cache（`session_persistence/*`、`restore_session_with_turns`）+ agent memory（sqlite WAL）——T5 重启恢复的地基已存在；
 5. **动态性**：MCP 全动态（add/remove/restart）、LSP 插件热装卸——T5 之外的既有热插拔面；
-6. **relay-server fail-closed 模式**：SW1-2 修 embedded relay 时直接复用其绑定/key 策略；
+6. **relay-server fail-closed 模式**（已失效：relay-server 已整删，T2-2 C5 commit `f6a011b`）：原 SW1-2 复用策略随载体删除失效；
 7. **治理设施**：core-boundaries 检查器（已入 CI）、技术债台账、surfaces 变更协议、B 线 brief/report 流程。
 
 ---
@@ -164,7 +164,7 @@ FU-1 MCP 配置写 fail-closed、FU-2 LSP uninstall 按语言键停服（`7a4bdc
 | # | 内容 | 来源线 | 量 |
 |---|---|---|---|
 | T2-1 | **CI 补齐**：check 去 exclude、test 扩面、`cargo tree -p northhing-kernel-api` 守卫已在 CI（kernel-api-clean job）、desktop check 强制门（P2-15 流程结转） | K+review | S |
-| T2-2 | 死代码删除第一批（insights / tool-provider-groups / 空 session 目录 / webdriver / enigo+screenshots / **judge_gate 适配层**（assembly/core 1,690L；**协议层 1,473L 保留**转 TH-5 词汇，2026-08-17 G15 修正）≈6.5k 行）**+ remote 栈整删（TH-4：remote_connect 11.5k + mobile-web 4.7k + embedded relay 入口先摘后删；P1-4/P1-7/D-2 随之关闭）** **+ MiniApp 子系统整删（2026-08-17 拍板：内置四件套 + 宿主 host_routing/bridge/manager/契约 ≈6k 行；permission_policy 默认拒绝语义先提炼进 PCS 设计再删码；连带关闭 T1-1、T3-5）**+ relay-server + relay-core 整删（PEND-1 拍板 2026-08-17：≈4-5k 行；surfaces.md 同 commit 同步）** + plan-compliance-checker(894L) + harness(571L，或并入 test-support)**，合计 ≈35k 行 | review+论题 | M |
+| T2-2 | 死代码删除第一批（insights / tool-provider-groups / 空 session 目录 / webdriver / enigo+screenshots / **judge_gate 适配层**（assembly/core 1,690L；**协议层 1,473L 保留**转 TH-5 词汇，2026-08-17 G15 修正）≈6.5k 行）**+ remote 栈整删（TH-4：remote_connect 11.5k + mobile-web 4.7k + embedded relay 入口先摘后删；P1-4/P1-7/D-2 随之关闭；remote 栈部分（含 relay-server/relay-core 整删、mobile-web、contracts 修剪、i18n 面）已完成 C1-C8（commits fa88342..本批），MiniApp 部分待执行）** **+ MiniApp 子系统整删（2026-08-17 拍板：内置四件套 + 宿主 host_routing/bridge/manager/契约 ≈6k 行；permission_policy 默认拒绝语义先提炼进 PCS 设计再删码；连带关闭 T1-1、T3-5）**+ relay-server + relay-core 整删（PEND-1 拍板 2026-08-17：≈4-5k 行；surfaces.md 同 commit 同步）** + plan-compliance-checker(894L) + harness(571L，或并入 test-support)**，合计 ≈35k 行 | review+论题 | M |
 | T2-9 | **功能冗余合并批次**（2026-08-17 冗余扫描）：第一批 S 级——deep_research 去重（255L×2，diff 仅 10 行注释→re-export）、ndjson_log 统一（4 个追加+轮转实现 ~1,320L）、now_unix_ms 统一（3 同名函数+25 内联）、原子写收口 json_store（顺修 P2-16 save_config 裸写；删 PersistenceService FILE_LOCKS）、初始化收口（server bootstrap 手抄 + CLI 样板×4 → init_agentic_system）；第二批 M 级——app.json↔GlobalConfig 镜像拆除（写穿 kernel API）、**事件管道收敛 A7**（BackendEvent 死管道并入 EventQueue 或删除）、**desktop NullDispatcher 空转路径移除**（agent-dispatch B2，回退直连直至 dispatcher 真接线）；延期 L 级——ExecCommand↔Bash 合并（Bash/PTY 为正）、双 ToolRegistry 迁移收尾、MCP core 包装层（3,641L）收口 | 冗余扫描 | 第一批 S / 第二批 M / 延期 L |
 | T2-10 | **连续性自检测试**：自动化"杀 core → 恢复 → diff 会话/记忆/身份"（T5"agent 不死"验收的轻量前置版，0.3 即可写，依赖 fake AI backend 提供确定性） | 论题 §3 度量 | S |
 
diff --git a/docs/status/tech-debt-ledger.md b/docs/status/tech-debt-ledger.md
index 413d41a..55f2cd3 100644
--- a/docs/status/tech-debt-ledger.md
+++ b/docs/status/tech-debt-ledger.md
@@ -49,7 +49,7 @@
 - **Symptom**: `PairingPage.tsx` has pairing logic but no re-pairing guidance when connection drops.
 - **Evidence**: `src/mobile-web/src/pages/PairingPage.tsx` — no re-pair UI.
 - **Proposed fix**: Add re-pair guidance UI to PairingPage.
-- **Status**: active (mobile-web: frozen surface)
+- **Status**: `resolved` — mobile-web 面已整删（T2-2 C6 commit `646f93d`），条目随删除关闭；`docs/architecture/backend-roadmap.md:118` 已预先声明此关闭方式。
 
 ### P1-4b: ~~Desktop Rust i18n mojibake~~ (resolved)
 
@@ -70,7 +70,7 @@
 - **Symptom**: `start_embedded_relay` binds `0.0.0.0:{port}` and passes `None` to `build_relay_router`, leaving pair/command endpoints open. This is a product-required open surface for LAN/ngrok mobile phone pairing — the pairing protocol itself must carry an out-of-band key.
 - **Evidence**: `src/crates/assembly/core/src/service/remote_connect/embedded_relay.rs:28-33` (passes `None`), `:44-46` (binds `0.0.0.0:{port}`).
 - **Proposed fix**: Thread an API key through the embedded relay path, gated by the pairing protocol handshake (design task). Options: (1) Generate ephemeral key on each desktop start and include in QR code/pairing URL. (2) Use a configurable key from desktop settings. (3) Pairing-level token exchange before relay commands.
-- **Status**: active (registered 2026-08-04, P1-5 standalone mitigation complete; a startup `warn!` has been added at `embedded_relay.rs`)
+- **Status**: `resolved` — relay-server + relay-core 已整删（T2-2 C5 commit `f6a011b`，PEND-1），embedded relay 入口不复存在，条目随删除关闭。
 
 ### P1-8: MCPServerConfig.env serialized as plaintext in app.json
 
@@ -215,6 +215,20 @@
 - **Proposed fix**: either wire plugin uninstall into the product surface or record it explicitly as an API kept for a planned surface; also note `stop_server` always returns `Ok`, which makes the new warn branch unreachable.
 - **Status**: active (low priority)
 
+### P2-19: `src/apps/server/README.md:5-10` 包含 3 条指向已删 relay-server 的悬空链接
+
+- **Symptom**: `src/apps/server/README.md:5-10` 中存在 3 条指向 `src/apps/relay-server` 的链接与描述引用，但 relay-server 已在 T2-2 C5（commit `f6a011b`）整删。
+- **Evidence**: `src/apps/server/README.md:5-10`。
+- **Proposed fix**: server 为 frozen 面，留待 server 解冻时同步修整文档链接（来源：T2-2g review M-g-2）。
+- **Status**: active (frozen surface)
+
+### P2-20: `pnpm-workspace.yaml` 中注册了孤儿工作区 `desktop-tauri`
+
+- **Symptom**: `pnpm-workspace.yaml` 中包含 `src/apps/desktop-tauri` 注册条目，但磁盘上该目录不存在（已随架构演进清理）。
+- **Evidence**: `pnpm-workspace.yaml:5`。
+- **Proposed fix**: 作为独立决策项处理，在后续工作区配置清理批次中移除（来源：T2-2h review F1/M-h）。
+- **Status**: active
+
 ## Change Protocol
 
 - **New entry**: Add with next available ID, include evidence (file:line), proposed fix, and status.
diff --git a/scripts/i18n-audit.mjs b/scripts/i18n-audit.mjs
index 1fda00e..305f14d 100644
--- a/scripts/i18n-audit.mjs
+++ b/scripts/i18n-audit.mjs
@@ -1281,9 +1281,6 @@ function collectL10nQualityCandidates(resourceGroups, allowedIdenticalMatches) {
   }
 }
 
-function collectConfirmedUnusedKeys() {
-}
-
 function auditGovernanceCategoryBudget(category, budget) {
   if (!isPlainObject(budget)) {
     reportError(`scripts/i18n-governance-baseline.json ${category} budget must be an object`);
@@ -1372,7 +1369,6 @@ function auditI18nGovernanceReport(namespaces) {
   const resourceGroups = buildResourceGroups(resourceEntries);
   const allowedIdenticalMatches = collectAllowedL10nIdenticalMatches(resourceGroups);
 
-  collectConfirmedUnusedKeys();
   collectDynamicKeyCandidates(resourceGroups);
   collectSharedTermDuplicates(resourceEntries);
   collectSameTextLocaleInventory(resourceGroups, allowedIdenticalMatches);
diff --git a/src/crates/assembly/core/Cargo.toml b/src/crates/assembly/core/Cargo.toml
index d8a8442..daf1882 100644
--- a/src/crates/assembly/core/Cargo.toml
+++ b/src/crates/assembly/core/Cargo.toml
@@ -121,12 +121,12 @@ rusqlite = { workspace = true }
 fluent-bundle = { workspace = true }
 unic-langid = { workspace = true }
 
-# Encryption (Remote Connect E2E)
+# Encryption
 aes-gcm = { workspace = true, optional = true }
 sha2 = { workspace = true }
 rand = { workspace = true, optional = true }
 
-# Device/Network info (Remote Connect)
+# Device/Network info
 local-ip-address = { workspace = true, optional = true }
 
 # QR code generation
diff --git a/src/crates/assembly/core/src/service_agent_runtime/mod.rs b/src/crates/assembly/core/src/service_agent_runtime/mod.rs
index f2c765f..a0e8a48 100644
--- a/src/crates/assembly/core/src/service_agent_runtime/mod.rs
+++ b/src/crates/assembly/core/src/service_agent_runtime/mod.rs
@@ -43,7 +43,7 @@ mod tests {
     }
 
     #[test]
-    fn core_service_agent_runtime_owner_exposes_agent_runtime_and_remote_control_port() {
+    fn core_service_agent_runtime_owner_exposes_agent_runtime() {
         fn assert_agent_runtime(
             coordinator: std::sync::Arc<crate::agentic::coordination::ConversationCoordinator>,
         ) -> Result<northhing_agent_runtime::runtime::AgentRuntime, String> {
diff --git a/src/crates/contracts/runtime-ports/src/session_workspace.rs b/src/crates/contracts/runtime-ports/src/session_workspace.rs
index aacaa4f..3222f06 100644
--- a/src/crates/contracts/runtime-ports/src/session_workspace.rs
+++ b/src/crates/contracts/runtime-ports/src/session_workspace.rs
@@ -1,4 +1,4 @@
-//! R26 sibling 2/4: session_workspace — session storage + workspace filesystem/shell + permission + clock + terminal + network + git + mcp + remote-connection port traits.
+//! R26 sibling 2/4: session_workspace — session storage + workspace filesystem/shell + permission + clock + terminal + network + git + mcp port traits.
 //!
 //! Mavis take-over (interface crate, all items `pub`).
 
