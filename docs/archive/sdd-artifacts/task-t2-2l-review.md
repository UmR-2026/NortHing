# T2-2l Review — MiniApp 整删 M2：assembly/core miniapp 整删

> 双判决独立审查。基准 commit dd2edd5，工作区未提交改动。本文件与 report 同提交。
> 审查时间：2026-08-19。命令：PowerShell + MSVC wrapper。

---

## 0. 判决总览

| 判决 | 结论 | 一句话 |
|---|---|---|
| **SPEC 合规** | **PASS** | 14 删 + 9 改 全部对应 brief 条款；红线未越；边界脚本层归属正确 |
| **代码质量** | **PASS** | 无 scope creep；hunk 全部映射到 brief；init.rs 相邻行未误删；无夹带格式化 |
| **总结论** | **PASS** | 可合并；2 项 Minor（仅 report 行数微差 / brief 文件计数笔误，0 影响） |

---

## 1. SPEC 合规逐条核对

### Constraint 1 — 授权文件集 14 删 + 9 改
**判定：PASS**

`git status --short` + `git diff --name-only`（排除 `.opencode/`、`memory/`、`.handoffs/` 并行 session）实测修改文件清单：

**14 删**（与 brief A1 文件清单逐项匹配）：
1. `src/crates/assembly/core/src/miniapp/mod.rs`（30 行）
2. `host_dispatch.rs`（40 行）
3. `js_worker.rs`（4 行）
4. `js_worker_pool.rs`（276 行）
5. `runtime_detect.rs`（2 行）
6. `compiler.rs`（33 行）
7. `exporter.rs`（33 行）
8. `storage.rs`（302 行）
9. `builtin/mod.rs`（584 行）
10. `manager/mod.rs`（469 行）
11. `manager/mgr_types.rs`（56 行）
12. `manager/mgr_registry.rs`（34 行）
13. `manager/mgr_runtime.rs`（114 行）
14. `manager/mgr_lifecycle.rs`（372 行）

合计 = 2,349 行（与 recon 完全一致，brief 估值亦写 2,349）。

**9 改**（与 brief A2 / B3 / C4-5 / D6 全部映射）：
1. `src/crates/assembly/core/src/lib.rs`（A2：删 `pub mod miniapp;` 两行）
2. `src/crates/assembly/core/Cargo.toml`（B3：删 `miniapp-runtime` 行 + `product-full`→`function-agents`）
3. `src/crates/assembly/core/src/product_domain_runtime.rs`（C4：删 miniapp use + `miniapp_runtime_facade`）
4. `src/crates/assembly/core/src/infrastructure/app_paths/path_manager.rs`（C5：doc 注释 `miniapps` 摘除）
5. `src/crates/assembly/core/src/infrastructure/app_paths/path_manager/init.rs`（C5：删启动副作用 `self.miniapps_dir(),`）
6. `src/crates/assembly/core/src/infrastructure/app_paths/path_manager/user_paths.rs`（C5：删 `miniapps_dir()` + `miniapp_dir(app_id)` 两方法）
7. `scripts/core-boundaries/rules/source/required-rules.mjs`（D6）
8. `scripts/core-boundaries/rules/source/forbidden-rules.mjs`（D6）
9. `scripts/core-boundaries/self-test.mjs`（D6）

合计 23 个改动文件。⚠️ Brief 抬头写 "22 个文件" 是 brief 自身的笔误——brief 内列出的授权条目 14+1+1+1+1+1+1+3 = 23 项，实施正好命中 23 项，与 brief 枚举一致。**笔误在 brief，不在实施。**

### Constraint 2 — 不碰红线面
**判定：PASS**

- `git diff src/crates/contracts/product-domains/ src/crates/services/services-integrations/` → 0 行变更 ✓
- `git diff --name-only | rg function_agents|tool_call_accumulator|i18n` → 0 命中 ✓
- `rg -i 'miniapp|miniapps' src/crates/assembly/core/` → 0 命中（实测 Get-ChildItem 该目录不存在 + grep 0 命中）✓
- `rg -i 'MiniAppStoragePort|MiniAppRuntimeFacade|miniapp_runtime_facade|miniapps_dir|miniapp_dir' src/crates/assembly/core/` → 0 命中 ✓
- `rg 'northhing-services-integrations/miniapp-runtime' src/` → 0 命中 ✓
- `rg 'use northhing_product_domains::miniapp' src/crates/assembly/core/` → 0 命中 ✓

### Constraint 3 — lib.rs 只删 :17-18
**判定：PASS**

`git diff src/crates/assembly/core/src/lib.rs` 实测：
```diff
 #[cfg(feature = "product-domains")]
 pub mod function_agents; // Function-based agents
 pub mod infrastructure; // AI clients, storage, logging, events
-#[cfg(feature = "product-domains")]
-pub mod miniapp; // AI-generated instant apps (Zero-Dialect Runtime)
 #[cfg(feature = "product-full")]
 pub mod product_assembly;
```
仅删 miniapp 两行，function_agents 门控完整保留。当前 lib.rs:14-15 即 function_agents（行号因前面删除而位移）。✓

### Constraint 4 — product_domain_runtime.rs 只删 miniapp 段
**判定：PASS**

`git diff` 实测：
```diff
-//! module keeps the concrete MiniApp and function-agent runtime bindings in
+//! module keeps the concrete function-agent runtime bindings in
 ...
-use northhing_product_domains::miniapp::ports::{MiniAppRuntimeFacade, MiniAppStoragePort};
 ...
-    pub(crate) fn miniapp_runtime_facade(storage: &dyn MiniAppStoragePort) -> MiniAppRuntimeFacade<'_> {
-        MiniAppRuntimeFacade::new(storage)
-    }
-
     pub(crate) fn function_agent_git_adapter() -> CoreFunctionAgentGitAdapter {
```
当前 `product_domain_runtime.rs` 完整保留：
- `function_agent_git_adapter()` ✓
- `function_agent_ai_adapter()` ✓
- `function_agent_runtime_facade()` ✓
- 加上 2 个 `generate/analyze_function_agent_commit_message|work_state` 包装方法 ✓

doc 注释摘 "MiniApp and" 是必要的措辞清理（否则注释与代码不一致）。

### Constraint 5 — boundary 脚本层归属正确
**判定：PASS**

实测 `rg -i miniapp scripts/core-boundaries/` 改后 = **293 行**（brief 要求改前 474 → 改后 >0）。所有残留锚点逐条核验：
- `self-test.mjs`：20 处路径，全部 `src/crates/services/services-integrations/src/miniapp/*` 或 `src/crates/contracts/product-domains/src/miniapp/*` ✓
- `feature-rules.mjs`：12 处 `miniapp-runtime`/`miniapp` ownerFeatures 引用（属于 services-integrations :78-87 + product-domains :22 feature，本批不动） ✓
- `required-rules.mjs`：剩余 path 锚点全部 `services-integrations/src/miniapp/*` 或 `contracts/product-domains/src/miniapp/*` ✓
- `forbidden-rules.mjs`：395 行 services-integrations host_dispatch.rs（M3 待删）、1743 行 contracts/product-domains/runtime.rs（M4 待删）✓

`rg 'assembly/core/src/miniapp' scripts/core-boundaries/` → **0 命中** ✓（core 层全部已摘）
`rg 'pub mod miniapp' scripts/core-boundaries/` → **0 命中** ✓
`rg 'MiniAppRuntimeFacade|MiniAppStoragePort|miniapp_runtime_facade' scripts/core-boundaries/` → 仅剩 product-domains / services-integrations 层 3 处（self-test.mjs:2208 + 2382 + required-rules.mjs:5966），均归属正确 ✓

### Constraint 6 — 门禁全 PASS
**判定：PASS**

| 命令 | 结果 |
|---|---|
| `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace` | Finished `dev` profile in 2.36s（仅 warnings，无 errors） ✓ |
| `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing` | Finished `dev` profile in 1.96s（仅 warnings，无 errors） ✓ |
| `node scripts/check-core-boundaries.mjs` | `Core boundary check passed.` ✓ |
| `cargo test -p northhing-core --lib --features product-full function_agents` | **8 passed; 0 failed**（function_agents 路径完整存活） ✓ |

8 个测试名称与 report 完全一致：
- 4 个 `function_agents::port_adapters::tests::*`（owner construct、commit/startchat snapshot × 4）
- 4 个 `function_agents::runtime_services::tests::*`（parse_commit_response / parse_complete_analysis 各自两个 product-domain policy 保留测试）

### Constraint 7 — 无夹带 / 无关格式化
**判定：PASS**

每个 hunk 都映射到一个 brief 条款：
- 14 个删除文件 → brief A1
- lib.rs 2 行 → brief A2
- Cargo.toml 2 行 → brief B3
- product_domain_runtime.rs 3 处变更 → brief C4
- path_manager/init.rs 单行 → brief C5
- path_manager/user_paths.rs 2 方法 → brief C5
- path_manager.rs doc 单行 → brief C5
- required-rules.mjs / forbidden-rules.mjs / self-test.mjs 全部 → brief D6

无超出授权范围的逻辑变更；doc 注释措辞更新（path_manager.rs:9、product_domain_runtime.rs:4）是必要的代码-注释一致性维护，不算夹带。

### Constraint 8 — init.rs 不误删相邻存活目录
**判定：PASS**

`git diff init.rs` 实测：
```diff
             self.user_cron_dir(),
             self.user_rules_dir(),
-            self.miniapps_dir(),
             self.logs_dir(),
             self.temp_dir(),
```
仅删 `self.miniapps_dir(),` 单行。其余 11 个目录（northhing_home_dir、projects_root、assistant_workspace_base_dir(None)、user_config_dir、user_agents_dir、cache_root、user_data_dir、user_cron_dir、user_rules_dir、logs_dir、temp_dir）全部完整保留。✓

---

## 2. 代码质量审查

### 双判决完整性
- **Spec 判决**：所有 8 条 constraint 逐条 grep / read / cargo test 复核，0 条违规。
- **Quality 判决**：diff 统计 25 files / 161 insertions / 3260 deletions（实际生效于 M2 范围的减法操作 3,099 行删除 vs +161 注释/规则更新），与 brief 估算 ≈2.5k 行删除一致。

### 实施质量
1. **路径切除手术精确**：每个 anchor（mod.rs / host_dispatch.rs / js_worker.rs / js_worker_pool.rs / runtime_detect.rs / compiler.rs / exporter.rs / storage.rs / builtin/mod.rs / manager/* 5 文件）在 boundary 三脚本中都被独立定位并整段删除。
2. **Cargo feature 重排安全**：product-domains feature 块由 5 项（ai-adapter-runtime / dep / function-agents / miniapp-runtime / product-full）缩为 4 项（ai-adapter-runtime / dep / function-agents / function-agents），把 `product-full`（聚合）替换为更细粒度的 `function-agents` 单项，是正确的"收紧权限"动作。
3. **path_manager 副作用摘除**：启动时建 `~/.config/northhing/data/miniapps/` 目录是 roadmap 显式要求的"拉起路径"摘除点，已同步删除。Doc 注释中 `miniapps` 单词同步删除，避免注释与代码不一致。
4. **product_domain_runtime.rs 文本一致性**：删除 miniapp 方法后，doc 注释中 "MiniApp and" 措辞同步清理。`function_agent_*` 三核心方法 + 2 包装方法全部完整保留，函数体未受影响。

### 是否引入技术债
无。本批纯减法，无新代码逻辑。所有 hunk 都可追溯到 brief 条款。

### 兼容性 / 远程面
M2 不动 services-integrations 与 product-domains 的 Cargo.toml / lib.rs / 内容，故远程面与 desktop 面均保留原有 surface；无新失败模式。

---

## 3. Findings

### Critical
**无**

### Important
**无**

### Minor

1. **report §A1 行数轻微虚高**（`task-t2-2l-report.md:12-25`）：
   - `mod.rs 32 lines` 实际 = 30（与 recon 一致）
   - `builtin/mod.rs 638 lines` 实际 = 584（与 recon 一致）
   - `js_worker_pool.rs 305 lines` 实际 = 276（与 recon 一致）
   - 总和 2349 行（brief 与 recon 估值一致），仅个别文件 report 行数与 recon 略差；不影响 spec 合规判定。**建议未来 report 统一从 `git show HEAD:path | Measure-Object -Line` 取值。**

2. **brief §材料抬头写 "22 个文件"，实际授权条目 23 个**（`task-t2-2l-brief.md:0`）：
   - brief 列出 14+1+1+1+1+1+1+3 = 23 项，实施命中 23 完全符合
   - "22" 是 brief 抬头笔误，**与本次实施无关**，不在报告/findings 范围内
   - 仅记一笔供后续 brief 校对。

### Cannot verify from diff
**无** — 所有 constraint 均可在 working tree 上 grep / read / cargo test 实测核验，且均已实测通过。

---

## 4. 一句话总结论

M2 批 14 删 + 9 改精确命中 brief 授权面 23 文件，红线全守，cargo check / boundary check / function_agents 测试三门禁全 PASS，可合并；唯一 Minor 是 report 中个别文件行数与 recon 略差（不影响合规判定）。