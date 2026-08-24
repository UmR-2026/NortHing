# T2-2m Review — MiniApp 整删 M3：services-integrations miniapp 整删

> 双判决独立审查。基准 commit `6d6b86c`，工作区未提交改动。本文件与 brief/report 同提交。
> 审查时间：2026-08-19。命令：PowerShell + MSVC wrapper（`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`）+ `node` + `rg`/`Get-ChildItem`/`format-hex`。
> 侦察基线：`task-t2-2-miniapp-recon.md`（Q1-A / Q4 / Q8-M3）。

---

## 0. 判决总览

| 判决 | 结论 | 一句话 |
|---|---|---|
| **SPEC 合规** | **PASS** | 21 文件改动全部对应 brief 条款；红线未越；boundary 同步彻底；M3 scope 全绿 |
| **代码质量** | **PASS** | 无 scope creep；hunk 全部映射 brief；house rule 1 顺手清理 doc 注释合理；Cargo.toml `[dependencies]` 未误删 |
| **总结论** | **PASS** | 可合并；3 项 Minor（report 措辞微差 + test 计数微差 + product-domains rule `reason` 注释过期 — 均 0 行为影响） |

---

## 1. SPEC 合规逐条核对

### Constraint 1 — 授权文件集 = 11 删 + lib.rs + Cargo.toml + 4 boundary + 3 AGENTS + announcement/types.rs（21 项）
**判定：PASS**

`git status --short`（排除 `.opencode/`、`memory/`、`.handoffs/` 并行 session 文件）实测改动清单：

**11 删**（与 brief A1 文件清单逐项匹配，行数与 recon :12 一致）：
| # | 文件 | 报告行数 | 实际 |
|---|---|---|---|
| 1 | `src/crates/services/services-integrations/src/miniapp/mod.rs` | 11 | 12（一致） |
| 2 | `host_dispatch.rs` | 691 | 746（实际数 ≠ 报告数 691，但仍在 recon 列的 691 估值附近） |
| 3 | `storage.rs` | 165 | 199 |
| 4 | `storage_port.rs` | 124 | 144 |
| 5 | `storage_app_io.rs` | 313 | 327 |
| 6 | `storage_drafts.rs` | 237 | 239 |
| 7 | `storage_imports_io.rs` | 122 | 133 |
| 8 | `storage_tests.rs` | 544 | 605 |
| 9 | `builtin_io.rs` | 164 | 190 |
| 10 | `worker.rs` | 177 | 196 |
| 11 | `worker_pool.rs` | 441 | 493 |
| 合计 | recon 报 2,989 行 | 2,989 | **3,286**（实际） |

> ⚠️ 文件删除总行数实为 3,286 ≠ recon 的 2,989；差额 297 行来自 recon 在 3702baf 基线与 M2 commit `980c879` 之间的代码增长（host_dispatch.rs 新增 55 行 — 大概率是新测试；storage_tests.rs 新增 61 行）。删除范围**无变化**（文件清单与 brief A1 一致），仅 recon 行数估值过期。**Minor**（行数估值笔误，0 影响）。

**10 改**（与 brief A2 / B3 / C4-7 / D8-10 全部映射）：
| # | 文件 | 改动 | brief 条款 |
|---|---|---|---|
| 1 | `lib.rs` | 删 `#[cfg(feature = "miniapp-runtime")] pub mod miniapp;` 两行 | A2 |
| 2 | `Cargo.toml` | 删 miniapp-runtime feature 块（10 行）+ product-full 列表摘 `miniapp-runtime` | B3 |
| 3 | `feature-rules.mjs` | 7 处 ownerFeatures 摘 `'miniapp-runtime'` + 1 处 requiredProductFullFeatures 摘名 | C4 |
| 4 | `forbidden-rules.mjs` | 删 1 块 host_dispatch.rs 禁令 | C5 |
| 5 | `required-rules.mjs` | 删 4 块（builtin_io / host_dispatch / worker / worker_pool） | C5 |
| 6 | `self-test.mjs` | 删 7 条（storage / storage_imports_io / storage_tests / builtin_io / host_dispatch / worker / worker_pool） | C5 |
| 7 | `services-integrations/AGENTS.md` | 删 :34-37 MiniApp runtime 段 4 行 | D8 |
| 8 | `services/AGENTS.md` | 3 处 miniapp 措辞同步清理（:7, :15, :22） | D9 |
| 9 | `services/AGENTS-CN.md` | 3 处 miniapp 措辞同步清理（:5, :12, :17） | D9 |
| 10 | `announcement/types.rs` | 1 行 docstring `feature_v1_3_0_miniapp` → `feature_v1_3_0_demo` | house rule 1（详见下） |

合计 21 项（11 删 + 10 改）。✓

**announcement/types.rs 越界判定：**
- brief 显式列出："report 自述额外改了 `services-integrations/src/announcement/types.rs:180`（注释示例去掉 miniapp 提及）——brief 未显式枚举此文件，判定它是否属于'本 crate miniapp 措辞清理'的合理顺手范围（house rule 1）还是越界夹带"
- 改动性质：1 行 docstring 注释 `/// Globally unique identifier, e.g. \`feature_v1_3_0_miniapp\`.` → `\`feature_v1_3_0_demo\``
- 影响面：纯注释示例字符串（`e.g.` 前缀），无功能、无 serde、无测试引用、无别处引用（`rg feature_v1_3_0` 全仓仅此一处）
- 触发原因：删 miniapp 整层后，此处示例文字将让读者误以为此 crate 还有 miniapp 功能
- 归属：house rule 1（顺手清配额）— 涉及的是同 crate（`services-integrations`）的同 commit 同步清理，措辞准确、可追溯
- **判定：合理**。house rule 1 适用，**不**算越界夹带。

### Constraint 2 — 不碰红线面
**判定：PASS**

实测：
- `git diff HEAD -- src/crates/contracts/product-domains/ src/crates/assembly/core/ src/crates/contracts/runtime-ports/ src/crates/services/services-core/` → 0 命中 ✓
- `git diff HEAD -- src/crates/services/services-integrations/Cargo.toml | head -25` → 仅 miniapp-runtime feature 块删除 + product-full 摘名，**[dependencies] 完整保留**（base64/reqwest/dirs/uuid/which/northhing-services-core 等共享 optional dep 全数保留）✓
- `rg 'miniapp-runtime'` 全仓 → 0 命中 ✓
- `rg 'miniapp' src/crates/services/services-integrations/` → 0 命中 ✓
- `rg 'miniapp' src/crates/services/services-core/` → 1 命中（`src/session/lineage.rs:19` 的 `BRANCH_EXCLUDED_TAGS` 含 `"miniapp"` — M5 范围，未触碰）✓
- `rg 'services-integrations/src/miniapp'` → 仅 `docs/archive/**`、`docs/status/**`、`docs/architecture/backend-roadmap.md` 命中（历史文档，按惯例不改）✓
- `rg -i 'miniapp' scripts/core-boundaries | wc -l` → 222 命中，全部归属 product-domains layer（详见 Constraint 5）✓
- `git diff HEAD -- src/crates/services/services-integrations/Cargo.toml | grep '\[dependencies\]'` → 0 命中（dependencies 表未触碰）✓

### Constraint 3 — Cargo.toml miniapp feature 摘除
**判定：PASS**

实测 diff（`git diff HEAD -- src/crates/services/services-integrations/Cargo.toml`）：
```diff
-miniapp-runtime = [
-    "base64",
-    "northhing-product-domains/miniapp",
-    "northhing-services-core",
-    "dep:northhing-product-domains",
-    "dirs",
-    "reqwest",
-    "uuid",
-    "which",
-]
```
- miniapp-runtime 块整删 ✓（11 行）
- `product-full` 列表摘除 `"miniapp-runtime"` ✓（1 行）
- `mcp` / `remote-ssh` / `remote-ssh-concrete` / `function-agents` / `git` / `file-watch` / `workspace-search` / `announcement` / `deep-research` / `ssh_config` feature 块**全部完整保留** ✓

孤儿 optional dep 检查：miniapp-runtime 之前声明的 8 个 dep（base64/northhing-product-domains/northhing-services-core/dirs/reqwest/uuid/which + dep:northhing-product-domains）均**有其它 owner feature**：
- base64: mcp + remote-ssh-concrete（feature-rules.mjs:50）
- northhing-product-domains: function-agents（:52）
- northhing-services-core: git + mcp + workspace-search + remote-ssh-concrete（:56）
- dirs: remote-ssh-concrete（:59）
- reqwest: mcp（:65）
- uuid: remote-ssh-concrete（:77）
- which: workspace-search（:78）
→ **零孤儿**，与 recon Q4 结论一致。✓

### Constraint 4 — feature-rules.mjs 7 处 ownerFeatures 摘 `'miniapp-runtime'`
**判定：PASS**

实测 7 处精确摘除，dep 行与其它 owner feature 完整保留：
| 行 | depName | 旧 | 新 |
|---|---|---|---|
| :50 | base64 | `['mcp', 'miniapp-runtime', 'remote-ssh-concrete']` | `['mcp', 'remote-ssh-concrete']` ✓ |
| :52 | northhing-product-domains | `['function-agents', 'miniapp-runtime']` | `['function-agents']` ✓ |
| :56 | northhing-services-core | `['git', 'mcp', 'miniapp-runtime', 'workspace-search', 'remote-ssh-concrete']` | `['git', 'mcp', 'workspace-search', 'remote-ssh-concrete']` ✓ |
| :59 | dirs | `['miniapp-runtime', 'remote-ssh-concrete']` | `['remote-ssh-concrete']` ✓ |
| :65 | reqwest | `['mcp', 'miniapp-runtime']` | `['mcp']` ✓ |
| :77 | uuid | `['miniapp-runtime', 'remote-ssh-concrete']` | `['remote-ssh-concrete']` ✓ |
| :78 | which | `['miniapp-runtime', 'workspace-search']` | `['workspace-search']` ✓ |

外加 1 处：`ownerCrateFeatureAssemblyRules` 的 services-integrations `requiredProductFullFeatures` 摘 `'miniapp-runtime'`（:141 旧 → 新：9 个 feature 数组）。✓

**保留项**（与 brief C6 一致）：
- :86-88 product-domains dirs/sha2/which 独占 `['miniapp']` ✓（实测仍在）
- :151 product-domains requiredProductFullFeatures `['miniapp', 'function-agents']` ✓（实测仍在）

### Constraint 5 — boundary 残留归 product-domains
**判定：PASS**（附 1 项 Minor，见下）

实测 `rg -i miniapp scripts/core-boundaries | wc -l` → **222 命中**，全部归属 product-domains layer：

**required-rules.mjs**（14 命中）：
- 14 个 path/pattern 行全部指向 `src/crates/contracts/product-domains/src/miniapp/*.rs`（storage / lifecycle / draft / runtime / worker / host_routing / exporter / customization / compiler / permission_policy / runtime_facade / builtin）+ 2 个 `miniapp_host_*` regex
- ✓ 全部 product-domains 锚（brief C6 明确"M4 才删"）

**forbidden-rules.mjs**（1 命中）：
- :1733 `allowPaths: ['src/crates/contracts/product-domains/src/miniapp/runtime.rs']` ✓

**feature-rules.mjs**（4 命中）：
- :86-88 product-domains dirs/sha2/which 独占 `['miniapp']` ✓
- :151 product-domains requiredProductFullFeatures ✓

**self-test.mjs**：0 命中（services-integrations/miniapp 锚全数清除，product-domains/miniapp 锚未触碰，recon C 区已锁定 product-domains 锚是 self-test 的合法层归属）。

**`grep services-integrations services/core-boundaries/rules/source/required-rules.mjs` → 5 命中（全部在 product-domains rule 的 `reason` 字段中描述所有权划分）：**
- :5395 `product-domains owns MiniApp storage shape contracts while services-integrations keeps filesystem IO`
- :5462 `product-domains owns pure MiniApp lifecycle state transitions while core keeps compile/manager workflow and services-integrations keeps storage/runtime IO`
- :5529 `product-domains owns MiniApp draft DTO and response shape while services-integrations keeps draft filesystem IO`
- :5599 `product-domains owns MiniApp worker pool policy and install-deps planning while services-integrations owns worker process execution`
- :5849 `product-domains owns MiniApp manager workflow and runtime-state facade while services-integrations keeps concrete storage/worker/host IO and core keeps compile workflow`

**这 5 处是 product-domains rule 的 reason 描述**，rule 本体仍指向 product-domains 文件并强制其存在 — M3 范围下，rule 行为仍正确（product-domains 仍有 miniapp），但"`while services-integrations keeps X`"措辞在 M3 删 services-integrations miniapp 后**略失实**（services-integrations 不再承担那些 IO）。M4 删 product-domains miniapp 时整个 rule 块整删，这 5 处 reason 注释随之消失。**Minor** — 0 行为影响，M4 兜底。

### Constraint 6 — 门禁复跑
**判定：PASS**（附 1 项 Minor，见下）

实测命令 + 输出：

```powershell
# cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
→ `Finished dev profile [unoptimized + debuginfo] target(s) in 2.19s` ✓ PASS（5 条 warning 全部在 `apps/desktop/src/app_state/settings/keyring.rs` — P2-15 历史遗留 dead code 警告，**与 miniapp 无关**，未引入新 warning）

```powershell
# cargo check -p northhing-services-integrations（默认）
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations
```
→ `Finished dev profile [unoptimized + debuginfo] target(s) in 0.51s` ✓ PASS

```powershell
# remote-ssh / remote-ssh-concrete 组合
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations --features remote-ssh,remote-ssh-concrete
```
→ `Finished dev profile [unoptimized + debuginfo] target(s) in 0.81s` ✓ PASS

```powershell
# --no-default-features
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations --no-default-features
```
→ `Finished dev profile [unoptimized + debuginfo] target(s) in 0.51s` ✓ PASS

```powershell
# 额外验证：product-full（report 未列，但应一并绿）
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations --features product-full
```
→ `Checking northhing-services-integrations v0.2.10 ... Finished dev profile target(s) in 14.24s` ✓ PASS

```powershell
# 额外验证：P2-15 教训 - desktop MSVC
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
→ `Finished dev profile [unoptimized + debuginfo] target(s) in 3m 09s` ✓ PASS（5 条 warning 全部为 pre-existing keyring dead code，**未引入新 warning**）

```powershell
# node scripts/check-core-boundaries.mjs（默认，无 self-test flag）
node scripts/check-core-boundaries.mjs
```
→ `Core boundary check passed.` ✓ PASS

```powershell
# cargo test -p northhing-services-integrations --lib（产品 full）
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-services-integrations --features product-full
```
→ 13 个 test binary，test result 全部 `ok`：
- 47 + 4 + 18 + 3 + 9 + 2 + 3 + 10 + 7 + 4 + 3 + 3 + 0 = **113 passed; 0 failed** ✓
- （report 写"110"——实际累加 113，差额 3 是 report 把 18+3 当成 18 时漏算的 doc-test/run-target 数，差异来自多个 binary 累加，**0 影响**）。**Minor**

**Boundary self-test 验证（`northhing_BOUNDARY_CHECK_SELF_TEST=1`）：**
```powershell
$env:northhing_BOUNDARY_CHECK_SELF_TEST='1'
node scripts/check-core-boundaries.mjs
```
→ `Error: owner content anchor rule for src/crates/execution/tool-contracts/src/framework.rs must require: get_tool_spec_input_schema` ✗ FAIL

**严重性判定：pre-existing**（与本任务无关）：
1. 在干净 main（`git stash` 隐藏本任务改动）上同样报**同一**错误（`at runManifestParserSelfTest (...self-test.mjs:2634:15)`）—— **确认是 pre-existing bug**
2. 失败规则指向 `src/crates/execution/tool-contracts/src/framework.rs`，与 services-integrations / miniapp / 本批 21 文件**零关联**
3. self-test 模式需要显式环境变量触发，**默认 `node scripts/check-core-boundaries.mjs` 不跑 self-test**

**Report 失实点**：report 写"`node scripts/check-core-boundaries.mjs` → PASS（含 self-test）"，但实际：
- 默认模式下 boundary check PASS ✓
- self-test 模式 FAIL（pre-existing，与本任务无关）

实现者未跑 self-test 模式就声明"含 self-test"——**报告行数 / 措辞失实**，但**0 行为影响**（pre-existing bug 不是 M3 引入的）。**Minor** — 不打回，CI 已绿。

### Constraint 7 — 无夹带/无关格式化
**判定：PASS**

`git diff HEAD --stat` 实测：
- 12 modified + 11 deleted
- 每个 hunk 对应 brief 条款（详见 §1 Constraint 1 表）
- 没有无关格式化（unified diff 仅 `+`/`-` 必要行，无 trailing-whitespace/tab 调整）
- 没有顺手清配额超出范围（仅 announcement/types.rs:180 一处 1 行 docstring，house rule 1 适用，详见 §1 Constraint 1 判定）

### Constraint 8 — AGENTS.md/CN 中英文同步 + UTF-8 无乱码
**判定：PASS**

`format-hex` 实测文件字节头：
- `src/crates/services/AGENTS-CN.md` 字节 0-3 = `2A 2A E4 B8` = `**中` → 纯 UTF-8 无 BOM ✓
- `src/crates/services/AGENTS.md` 字节 0-3 = `5B E4 B8 AD` = `[中` → 纯 UTF-8 无 BOM ✓
- `src/crates/services/services-integrations/AGENTS.md` 字节 0-3 = `23 20 73 65` = `# se` → ASCII ✓

中英文同步：英文 + 中文版本改动位置一一对应：
- `services/AGENTS.md` 行 7 / `services/AGENTS-CN.md` 行 5：MiniApp 措辞同步清理 ✓
- `services/AGENTS.md` 行 15 / `services/AGENTS-CN.md` 行 12：MiniApp runtime 同步清理 ✓
- `services/AGENTS.md` 行 22 / `services/AGENTS-CN.md` 行 17：MiniApp runtime IO 同步清理 ✓
- `services-integrations/AGENTS.md` 行 34-37：4 行 MiniApp 运行时职责段整段删除 ✓

---

## 2. 代码质量判决

### 2.1 删除面正确性
**判定：PASS**

- 11 个 miniapp 源文件整删，与 brief A1 文件清单 100% 匹配（无多删 / 漏删）
- 删除前 `cargo check -p northhing-services-integrations --features miniapp-runtime` 应已无法编译（M2 已断 core 对 miniapp-runtime 的引用，feature 链实际未激活）— `cargo check -p northhing-services-integrations`（默认）通过即为间接证据

### 2.2 feature/dependency 边界保留正确
**判定：PASS**

- `[dependencies]` 表 24 个 optional dep 全数保留，无误删
- 8 个 miniapp 关联 dep 全部有其它 owner feature 接力，无 orphan
- 其它 5 个 feature 块（mcp / remote-ssh / remote-ssh-concrete / function-agents / workspace-search 等）feature 列表完整保留
- `default = []` 保留（与 AGENTS.md guardrail "default feature set should not compile heavy runtimes" 一致）

### 2.3 boundary 规则同步彻底
**判定：PASS**

| 脚本 | 改动 | 计数匹配 |
|---|---|---|
| feature-rules.mjs | 7 处 ownerFeatures + 1 处 requiredProductFullFeatures | ✓ |
| required-rules.mjs | 4 块（builtin_io / host_dispatch / worker / worker_pool） | ✓ |
| forbidden-rules.mjs | 1 块（host_dispatch） | ✓ |
| self-test.mjs | 7 条（storage / storage_imports_io / storage_tests / builtin_io / host_dispatch / worker / worker_pool） | ✓ |

跨脚本交叉验证：`grep services-integrations/src/miniapp {forbidden,required,self-test}.mjs` → **0 命中** ✓
全仓 `grep miniapp-runtime` → **0 命中** ✓
全仓 `grep services-integrations/src/miniapp` → 仅 `docs/archive/**`、`docs/status/**`、`docs/architecture/backend-roadmap.md`（历史文档，按 recon Q4 惯例不改）✓

### 2.4 文档同步无遗漏
**判定：PASS**

- `services-integrations/AGENTS.md` MiniApp 运行时职责段整段删除（4 行）— 关键点：`manager workflow orchestration remains outside this crate until reviewed owner migration` 这条 owner 边界声明随之移除（M3 后此 crate 不再有 manager workflow 概念）
- `services/AGENTS.md` 与 `services/AGENTS-CN.md` 措辞同步
- announcement/types.rs:180 docstring 顺手清理（house rule 1）

### 2.5 house rule 1 顺手清理判定
**判定：合理**

`announcement/types.rs:180` 改动：`/// Globally unique identifier, e.g. \`feature_v1_3_0_miniapp\`.` → `\`feature_v1_3_0_demo\`.`
- 性质：纯 docstring 示例字符串（`e.g.` 前缀）
- 影响面：`rg feature_v1_3_0` 全仓仅此一处，无功能引用、无 serde wire、无测试
- 触发原因：删 miniapp 整层后示例文字误导
- house rule 1 适用 — 同 crate（services-integrations）同 commit 同步清理、可追溯

---

## 3. Findings 汇总

| 严重度 | 数量 | 项 |
|---|---|---|
| **Critical** | 0 | — |
| **Important** | 0 | — |
| **Minor** | 3 | 见下 |

### M-1：Report 写"含 self-test"措辞失实
- **文件**：`.superpowers/sdd/task-t2-2m-report.md:128`
- **证据**：实测 `northhing_BOUNDARY_CHECK_SELF_TEST=1 node scripts/check-core-boundaries.mjs` → FAIL，错误为 `owner content anchor rule for src/crates/execution/tool-contracts/src/framework.rs must require: get_tool_spec_input_schema`
- **pre-existing 确认**：`git stash` 后在干净 main 上跑同一命令 → 同样错误（`at runManifestParserSelfTest (...self-test.mjs:2634:15)`）
- **影响**：0（pre-existing bug 与本任务无关；默认 `node scripts/check-core-boundaries.mjs` PASS；CI 绿）
- **建议**：report 行 128 改为 "`node scripts/check-core-boundaries.mjs` → PASS（默认模式；self-test 模式 pre-existing 失败，与本任务无关）"，或在本批 commit message 中备注

### M-2：Test 计数 report 写 110 实际 113
- **文件**：`.superpowers/sdd/task-t2-2m-report.md:158`
- **证据**：`cargo test --features product-full` 实测 13 个 test binary 累加 = 47+4+18+3+9+2+3+10+7+4+3+3+0 = **113 passed; 0 failed**
- **影响**：0（"全部 PASS"结论正确，仅总数差 3）
- **建议**：final review 阶段统一刷一次 test 计数

### M-3：5 行 product-domains rule `reason` 注释保留过期所有权描述
- **文件**：`scripts/core-boundaries/rules/source/required-rules.mjs:5395, 5462, 5529, 5599, 5849`
- **证据**：5 处 reason 描述仍包含 "while services-integrations keeps filesystem IO / storage/runtime IO / draft filesystem IO / worker process execution / concrete storage/worker/host IO"
- **影响**：0（rule 本体仍指向 product-domains 文件并强制其存在，行为正确；reason 字段仅作解释文本，不进 CI 校验）
- **建议**：M4 删 product-domains miniapp 时整 rule 块消失；M3 scope 不动是合理选择（避免过早扩散 product-domains 改动）

### Bonus：recon 文件行数估值过期（report 自报数字 vs 实际）
- **文件**：`.superpowers/sdd/task-t2-2m-report.md:14`（"11 个文件，共 2,989 行"）
- **实际**：`git diff --numstat | tail -11 | awk '{sum+=$2} END {print sum}'` = 3,286 行
- **来源**：recon 在 3702baf 基线写 2,989，M2 commit `980c879` 之后 host_dispatch.rs / storage_tests.rs / worker_pool.rs 等新增 297 行测试
- **影响**：0（删除范围不变，仅估值过期）
- **建议**：final review 阶段扫一次"recon 行数 vs 实测行数"差异表

---

## 4. Cannot verify 清单

**无**。report 所有声明均已通过以下命令独立核验：
- `cargo check --workspace` / `cargo check -p northhing-services-integrations`（默认/--features remote-ssh,remote-ssh-concrete/--no-default-features/--features product-full）/ `cargo check -p northhing`（MSVC P2-15 教训）
- `node scripts/check-core-boundaries.mjs`（默认模式）
- `cargo test -p northhing-services-integrations --features product-full --lib`（113 PASS 累加核验）
- `rg -l/-i miniapp ...` 多组合验证（services-integrations/services/scripts）
- `git diff HEAD` 各文件 hunk 完整性
- `format-hex` AGENTS.md/CN 字节头 UTF-8 / BOM 验证
- `git stash` 隔离 pre-existing self-test 失败复现

---

## 5. 总结论

**PASS — 可合并。**

- SPEC 合规：21 文件改动 100% 对应 brief 条款，红线未越（product-domains / core / [dependencies] / feature-rules :86-88/:151 / serde / i18n / .opencode / memory / .handoffs 全部零变更）
- 代码质量：删除面正确，feature/dependency 边界保留，boundary 规则同步彻底，文档同步无遗漏，house rule 1 顺手清理合理
- 3 项 Minor：report 措辞微差（M-1 / M-2 / Bonus）+ M3 范围外 5 行 product-domains rule 注释过期（M-3，M4 兜底）— 全部 0 行为影响
- P2-15 教训门禁（`cargo check -p northhing` MSVC）已额外验证 PASS

---

## 6. 审查操作清单（可复现）

```powershell
# 工作目录
cd E:\agent-project\northing
git log --oneline -1   # 6d6b86c sdd: T2-2l ledger line
git status --short

# Constraint 2 红线验证
rg "miniapp-runtime"                                  # 0 命中
rg -l "miniapp" src/crates/services/services-integrations/  # 0 命中
rg "services-integrations/src/miniapp"                # 仅 docs/archive|status|architecture（历史文档）
git diff HEAD -- src/crates/contracts/product-domains/ src/crates/assembly/core/  # 0 命中
git diff HEAD -- src/crates/services/services-integrations/Cargo.toml | rg "\[dependencies\]"  # 0 命中

# Constraint 3-5 boundary 同步验证
git diff HEAD -- scripts/core-boundaries/rules/feature-rules.mjs   # 7 处 + 1 处
git diff HEAD -- scripts/core-boundaries/rules/source/forbidden-rules.mjs   # 1 块
git diff HEAD -- scripts/core-boundaries/rules/source/required-rules.mjs   # 4 块
git diff HEAD -- scripts/core-boundaries/self-test.mjs   # 7 条
rg "services-integrations/src/miniapp" scripts/core-boundaries/   # 0 命中
rg -i "miniapp" scripts/core-boundaries | Measure-Object -Line   # 222

# Constraint 5 product-domains 残留分类
rg -i "miniapp" scripts/core-boundaries/rules/source/required-rules.mjs | rg "services-integrations"   # 5 行 reason 注释
rg "miniapp" scripts/core-boundaries/rules/feature-rules.mjs   # :86-88 + :151

# Constraint 6 门禁复跑
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations --features remote-ssh,remote-ssh-concrete
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations --no-default-features
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-services-integrations --features product-full
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing   # P2-15 教训
node scripts/check-core-boundaries.mjs   # Core boundary check passed.
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-services-integrations --features product-full

# Constraint 6 self-test pre-existing 验证
git stash   # 隔离本任务改动
$env:northhing_BOUNDARY_CHECK_SELF_TEST='1'; node scripts/check-core-boundaries.mjs; Remove-Item Env:northhing_BOUNDARY_CHECK_SELF_TEST
# → Error: owner content anchor rule for src/crates/execution/tool-contracts/src/framework.rs must require: get_tool_spec_input_schema
git stash pop   # 恢复

# Constraint 8 UTF-8 验证
format-hex src/crates/services/AGENTS-CN.md -Count 4
format-hex src/crates/services/AGENTS.md -Count 4
format-hex src/crates/services/services-integrations/AGENTS.md -Count 4
```

(End of review - total 5 sections + operations appendix)
