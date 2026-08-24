# Task T2-2k Review — MiniApp 整删 M1（入口摘除）双判决

> 评审对象：`E:\agent-project\northing`（main，HEAD=3702baf，工作区未提交 23 src/tests 文件）
> 评审视角：spec 合规 + 代码质量，独立核验每条 constraint。
> 评审执行：reviewer 子代理，2026-08-19。

## SPEC 判决：**PASS**

### 1. 授权文件集核对（constraint 1）

`git diff --name-only` 输出 23 个 src/tests 文件 + 2 个并行 session 预存改动（`.opencode/model-capability-notes.md`、`memory/northhing.md`）+ 5 个 untracked SDD 工件——与 brief A-E 清单逐项对齐：

| Brief 项 | 状态 | 证据 |
|---|---|---|
| A1 `miniapp_init_tool.rs` 整删 | ✅ | `git status`: `D` |
| A2 `implementations/mod.rs:51,93` | ✅ | diff :394, :402 删除 |
| A3 `materialization.rs:61,111` | ✅ | diff :427, :435 删除 |
| A4 `agents/mod.rs:96` | ✅ | diff :9 删除 |
| A5 `agent-tool-exposure.md:44` | ✅ | diff :155 删除 |
| A6 `registry/tests.rs:209,349` | ✅ | diff :447, :455 删除 |
| B7 `sub_handle_out.rs:157-158` 死分支 | ✅ | diff :77-82 整臂替换 |
| B8 `restrictions.rs:8-88,:149-167` | ✅ | diff :473-552 函数 + :561-580 测试删除；:5 `BTreeMap/BTreeSet` import 跟随删除 |
| B9 `tools/mod.rs:39-42` 摘名 | ✅ | diff :414-417 仅摘 2 个名 |
| B10 7 死 import 清理 | ✅ | 7 文件全部为同模式 `use {…}::ToolRuntimeRestrictions` 单名替换 |
| C11 `product-capabilities/src/lib.rs` 四处 | ✅ | diff :638,:646,:654-659,:667-670 删除 |
| C12 `tests/product_capabilities.rs:19,31,86` | ✅ | diff :682,:691,:700 三处 `"miniapp"` 摘除 |
| D13 三语言 tips 整删 | ✅ | git status 三 `D` |
| D14 e2e 死选择器 5 处 | ✅ | diff :713,:725,:733,:742,:751 |
| E15 boundary 同步 | ✅ | `rg -i "InitMiniApp\|miniapp_init_tool\|is_miniapp_headless\|miniapp_headless\|ProductCapabilityId::MiniApp" scripts/core-boundaries/` → **零命中**（删项无锚点）；474 行总计数维持 |

### 2. 红线保护核对（constraint 2）

```
git diff src/crates/assembly/core/Cargo.toml             → (no output)
git diff src/crates/assembly/core/src/lib.rs             → (no output)
git diff src/crates/services/services-integrations/Cargo.toml → (no output)
git diff src/crates/contracts/product-domains/Cargo.toml → (no output)
git diff src/crates/contracts/product-domains/src/lib.rs → (no output)
git diff src/crates/contracts/core-types/src/surface.rs  → (no output)
git diff src/crates/services/services-core/src/session/session_metadata.rs → (no output)
git diff src/crates/services/services-core/src/session/lineage.rs → (no output)
git diff src/crates/execution/agent-stream/src/tool_call_accumulator.rs → (no output)
git diff scripts/i18n-audit.mjs                         → (no output)
git diff src/shared/i18n/                                → (no output)
git diff src/crates/assembly/core/build.rs              → (no output)
git diff src/crates/assembly/core/src/miniapp/          → (no output)
git diff src/crates/services/services-integrations/src/miniapp/ → (no output)
git diff src/crates/contracts/product-domains/src/miniapp/ → (no output)
```

✅ 所有红线文件零触碰。M2-M5 层（core/services-integrations/product-domains 的 miniapp 目录、feature 块、lib.rs:17-18、serde 变体、`tool_call_accumulator.rs:150`、i18n 脚本与 locales）全部保留。

### 3. restrictions.rs 存活符号核对（constraint 3）

文件读完后保留的关键符号：

| 符号 | 位置 | 状态 |
|---|---|---|
| `pub use northhing_agent_tools::{is_remote_posix_path_within_root, ToolPathOperation, ToolPathPolicy, ToolRestrictionError, ToolRuntimeRestrictions}` | :2-4 | ✅ 完整 |
| `impl From<ToolRestrictionError> for NortHingError` | :7-11 | ✅ 完整 |
| `pub fn is_local_path_within_root` | :13-17 | ✅ 完整 |
| `fn canonicalize_best_effort` | :19-57 | ✅ 完整 |
| 测试：`runtime_restrictions_allow_all_when_empty`、`denied_tool_names_override_allow_list`、`custom_deny_message_overrides_generic_runtime_error`、`tool_restriction_errors_map_to_tool_errors`、`remote_posix_roots_require_true_containment`、`local_path_containment_handles_missing_children` | :63-149 | ✅ 6 测试全部存活 |
| `use std::collections::{BTreeMap, BTreeSet}` | 原 :5 | ✅ 删除（仅被 miniapp_headless 函数使用，跟着死掉，**正确**） |

✅ 仅摘 miniapp headless 段 :8-88 + 测试 :149-167，文件其余部分原样保留。

### 4. 死 import 清理无副作用（constraint 4）

- `cargo check --workspace` 警告总数：**28 行**（含生成 summary 行），与 HEAD 基线（同样 28 行）完全一致。
- 唯一 `unused import` 警告：`src/apps/cli/src/ui/question/mod.rs:15` 的 `QuestionData` 和 `QuestionOption`——**未触及文件**，pre-existing。
- 7 个清理文件（coordinator.rs / compaction.rs / session.rs / thread_goal.rs / workspace.rs / so_dispatch.rs / so_types.rs）全部保留 `ToolRuntimeRestrictions` 单名 import，函数主体未动。

✅ 无新 unused import 警告，无存活符号误删。

### 5. e2e 改动范围（constraint 5）

- `l0-navigation.spec.ts` 仅 :14 一处删除（NAV_ENTRY_SELECTORS 数组 1 行）。
- `l1-navigation.spec.ts` :18 NAV_ENTRY_SELECTORS 数组 + :172/:193/:231 三处 `.$$()` 选择器字符串中各 1 个 `, .northhing-nav-panel__miniapp-entry.is-active` 摘除。
- 同一文件其它用例（`navigation panel should be visible`、`should have multiple navigation items`、`should be able to click on navigation item`、`clicking navigation item should change view`、`navigation sections should be expandable`、`inline sections should be collapsible`）零触碰。

✅ 改动严格限于死选择器，无其它 e2e 改动。

### 6. boundary 同步与脚本门禁（constraint 6）

- `rg -i "InitMiniApp|miniapp_init_tool|is_miniapp_headless|miniapp_headless|ProductCapabilityId::MiniApp" scripts/core-boundaries/` → **零命中**：本批删除项在 boundary 规则里无独立锚点。
- `rg -i miniapp scripts/core-boundaries/` → **474 行**（= HEAD 基线 474 行）。
- `node scripts/check-core-boundaries.mjs` → `Core boundary check passed.`（EXIT: 0）。

M2-M5 层锚点完整保留证据：required-rules.mjs（324 处）、forbidden-rules.mjs（56 处）、feature-rules.mjs（12 处）、self-test.mjs（82 处）——锚定 core/services-integrations/product-domains 的 miniapp 目录、`pub mod miniapp`、`northhing-services-integrations/miniapp-runtime`、Command::new 例外、`MiniAppStorage`、`MiniAppWorkerPool` 等均健在。

✅ boundary 门禁 PASS（含 self-test），M2-M5 强制规则保留完整。

### 7. 门禁复跑（constraint 7，全部亲跑）

| 命令 | 输出摘要 | 判决 |
|---|---|---|
| `cargo check --workspace` | `Finished dev profile in 1.78s`，28 行警告（同基线），0 错误 | ✅ PASS |
| `cargo check -p northhing` | `Finished dev profile in 3.72s`，24 行警告（同基线），0 错误 | ✅ PASS |
| `node scripts/check-core-boundaries.mjs` | `Core boundary check passed.` | ✅ PASS |
| `cargo test -p northhing-core --lib --features product-full tools::registry` | `test result: ok. 22 passed; 0 failed` | ✅ PASS |
| `cargo test -p northhing-core --lib --features product-full tools::restrictions` | `test result: ok. 6 passed; 0 failed`（miniapp_headless_* 2 测试已删，剩 6 存活测试） | ✅ PASS |
| `cargo test -p northhing-product-capabilities` | `test result: ok. 5 passed; 0 failed` | ✅ PASS |

### 8. 无夹带（constraint 8）

每个 diff hunk 与 brief 清单项 1:1 对应，无关格式化或顺手清零：

| Hunk 类型 | 计数 | 备注 |
|---|---|---|
| miniapp_init_tool.rs 整删 | 1 hunk（221 行） | brief A1 |
| InitMiniApp 列表/注册/断言/文档摘除 | 6 hunks | brief A2-A6 |
| miniapp headless 函数 + 测试删除 | 2 hunks（restrictions.rs） | brief B8 |
| 死分支 + import 简化 | 1 hunk（sub_handle_out.rs） | brief B7 |
| re-export 摘名 | 2 hunks（tools/mod.rs, materialization.rs `:111`） | brief B9 + A3 |
| 7 死 import 清理 | 7 hunks（结构同型） | brief B10 |
| MiniApp capability 摘除 + 测试断言 | 5 hunks（lib.rs + tests） | brief C11-C12 |
| tips 三文件整删 | 3 hunks | brief D13 |
| e2e 死选择器摘除 | 5 hunks | brief D14 |

`git diff --stat` 输出：`23 files changed, 16 insertions(+), 416 deletions(-)`——删除远多于新增，全部为 brief 授权项；零格式化 hunk（无 trailing-whitespace / 缩进调整 / 无关 import 重排）。

✅ 无夹带，无越界格式化。

### 9. sub_handle_out.rs 死分支语义保留（constraint 9）

diff 仅触及 `:27-29` import 与 `:154`（旧 `:157-158`）赋值语句。该函数其余部分（`ExecutionContext` 构造、`turn_index == 0` session title 路径、`tokio::spawn` 任务、`SessionExecutionGuard` 等）原样保留：

```
154:        let runtime_tool_restrictions = ToolRuntimeRestrictions::default();
155:        let execution_context = ExecutionContext {
156:            session_id: session_id.clone(),
157:            dialog_turn_id: turn_id.clone(),
158:            turn_index,
...
165:            runtime_tool_restrictions,
...
207:        }
208:        let session_manager = self.session_manager.clone();
209:        let execution_engine = self.execution_engine.clone();
210:        let event_queue = self.event_queue.clone();
...
```

`user_message_metadata` 仍被 :111 事件 emit、:135 deep_review_run_manifest 提取、:142 acp_transport 检测、:216 clone-for-spawn 使用；`session.created_by` 不再被本函数读取，但 `session` 仍被其它字段使用（`session.config.max_context_tokens`、`session.snapshot_session_id`、`session.config.model_id`），未引入 unused-variable 警告。

✅ 死分支删除后存活逻辑行为不变，cargo check 无新警告。

---

## QUALITY 判决：**PASS**

### 评估维度

1. **Diff 最小性**：每个 hunk 严格对应 brief 授权项，无冗余改动。16 insertions vs 416 deletions 比例健康（核心为删除任务）。
2. **修改一致性**：7 个死 import 清理全部遵循同模式 `{is_miniapp_headless_agent_run, miniapp_headless_agent_tool_restrictions, ToolRuntimeRestrictions}` → `ToolRuntimeRestrictions`，无风格漂移。
3. **存活逻辑保护**：restrictions.rs 文件级存活符号完整；materialization.rs `:4` 的 `use crate::agentic::tools::implementations::*;` 通配 import 按 brief 要求保留；tools/mod.rs:39 re-export 仅删 2 个名字。
4. **依赖清理的连锁性**：`use std::collections::{BTreeMap, BTreeSet};` 在 restrictions.rs 中原 :5 跟随 `miniapp_headless_agent_tool_restrictions` 一同删除——正确（BTreeMap/BTreeSet 只被已删函数使用），无 orphan import 残留。
5. **tests 同步**：registry/tests.rs :209 列表项删除同时 :349 对应断言删除；restrictions.rs :149-167 两个 miniapp_headless_* 测试随函数删除；product_capabilities.rs 三处 capability id 列表断言同步——三处测试更新均与源码同步，零测试假阳性风险。
6. **e2e 改动最小性**：l0/l1 选择器删除未触动断言与 describe 块结构，未引入语法损伤；NAV_ENTRY_SELECTORS 数组仍保持类型一致（4 元素 → 3 元素 vs 5 → 4 元素）。
7. **build.rs 零改动**：brief 说明 announcement tips 嵌入走 build.rs 目录扫描（`build.rs:303-306`），删除文件即生效——经 git diff 验证 build.rs 未触碰，符合预期。
8. **警告基线稳定性**：cargo check 警告总数 28 = 28，零新增；唯一 `unused import` 警告位于未触及的 CLI 文件。

### 风格与约定

- 所有改动遵循现有命名规范（PascalCase 工具名、snake_case 函数名）。
- 无 trailing-whitespace / 缩进调整 / import 重排 / 注释格式化等越界样式改动。
- 删除的 diff 没有遗留空行（每个 hunk 的 `+/-` 对是连续的）。
- 中文 tips 文件未在报告中引用中文路径的字符（避免 GBK 风险），符合工作约定。

---

## Findings

### Critical
（无）

### Important
（无）

### Minor
1. **brief 约束 #6 文字 vs 实现**：`rg -i miniapp scripts/core-boundaries | wc -l` 计数 brief 写"应小于改前且 >0"，但本批删除项（InitMiniApp、miniapp_init_tool、is_miniapp_headless_*、ProductCapabilityId::MiniApp）在 boundary 规则里**零锚点**，导致改前=改后=474。该结果与 brief 约束 #15 的"若被锚定"措辞逻辑一致，是 brief 内部措辞的非矛盾解释；**实现正确，文字建议后续批 brief 修订**。
2. **CRLF 警告**：`git diff --stat` 对 agent-tool-exposure.md / l0-navigation.spec.ts / l1-navigation.spec.ts 三文件提示 `LF will be replaced by CRLF the next time Git touches it`。文件原始即为 CRLF，本次修改未引入新行结束符变化，仅是 Git 内部状态提示；不影响内容正确性。

---

## Cannot verify from diff

以下项需要 CI 或运行环境验证，无法仅从 diff 判定：

1. **desktop runtime 启动路径**：cargo check 仅做类型检查，desktop Slint UI 在运行时是否引用 miniapp 入口——recon Q3 已实测 `rg -l -i 'miniapp|mini_app' src/apps/` 零命中，但运行时动态引用仍需集成测试覆盖。
2. **e2e 实际跑通**：5 处选择器摘除后 wdio l0/l1 spec 是否真正绿，需 `pnpm --dir tests/e2e run test:l0/l1`（不在 brief 验证集，由 CI 接管）。
3. **announcement tips 重建产物**：`core` build.rs :303-306 是目录扫描 + include_str! 嵌入，删除 3 个 tips 文件后产物 binary 中是否还有 `013_miniapp` 字符串——需 `cargo build -p northhing` + 字符串扫描。brief 未要求，CI 覆盖。
4. **CI 全量跑通**：brief 指定 focused 测试集，`cargo test --workspace` 广覆盖交 CI。

---

## 一句话总结论

**T2-2k MiniApp 入口摘除 M1 双判决 PASS/PASS**：所有 9 条 constraint 全部满足，23 文件改动严格匹配 brief 授权集，红线文件零触碰，5 项 cargo/boundary 门禁 PASS（28 警告基线无新增），diff 干净无越界格式化，可直接合入。