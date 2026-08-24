# Task T2-2o Report — MiniApp 整删 M5：顶层 MiniApp/ + 文档收口 + 终扫

- **Task ID**: T2-2o (MiniApp M5 收口批)
- **Base Commit**: `17eb4bb` (`sdd: T2-2n ledger line + brief/report/review/diff artifacts`)
- **Final Status**: **DONE**

---

## 1. 变更清单与前后摘要（File:Line）

### A. 顶层 MiniApp/ 整删
- `MiniApp/`（整目录删除，共 37 个文件，7,953 行）：
  - `MiniApp/Skills/miniapp-dev/`（4 个 Markdown 文档：`SKILL.md`、`api-reference.md`、`architecture.md`、`design-playbook.md`，共 696 行）
  - `MiniApp/Demo/git-graph/`（演示应用：`meta.json`、`source/*`、`ui/*` 等 27 文件，共 6,028 行）
  - `MiniApp/Demo/icon-design-system/`（演示应用：`meta.json`、`source/*` 6 文件，共 1,229 行）

### B. 文档与台账收口
1. **`docs/status/surfaces.md:22`**
   - 前：
     ```markdown
     | **Server** | `src/apps/server` | 🧊 Frozen | HTTP API surface; no auth layer. Not deployed. |
     | **MiniApp UI** | `src/crates/contracts/product-domains/src/miniapp/` | 🧊 Frozen | Built-in mini-apps (PPT live, etc.) are experimental. String-mode shell commands rejected by `guard_command_execution`. |
     | **Tauri Desktop (candidate)** | `src/apps/desktop-tauri` | 🧊 Frozen | Tauri 2 + React candidate for the next baseline; flips at F4. src-tauri is its own cargo workspace (excluded from main). |
     ```
   - 后：
     ```markdown
     | **Server** | `src/apps/server` | 🧊 Frozen | HTTP API surface; no auth layer. Not deployed. |
     | **Tauri Desktop (candidate)** | `src/apps/desktop-tauri` | 🧊 Frozen | Tauri 2 + React candidate for the next baseline; flips at F4. src-tauri is its own cargo workspace (excluded from main). |
     ```

2. **`AGENTS.md:26,35,176,179`**
   - :26 Services 层表格描述摘除 `MiniApp runtime IO, `。
   - :35 边界规则摘除 `and MiniApp runtime IO` 措辞。
   - :176 骨架不变量 Shell safety 摘除 `; MiniApp string-mode commands containing shell metacharacters are rejected.`，`guard_command_execution` 本体与其余文字保留。
   - :179 Surface baseline 冻结面枚举摘除 `MiniApp UI / `。

3. **`AGENTS-CN.md:25,34,137,140`**
   - :25 服务层表格描述摘除 `MiniApp 运行时 IO 以及`。
   - :34 边界规则摘除 `以及 MiniApp 运行时 IO` 措辞。
   - :137 骨架不变量 Shell 安全摘除 `；MiniApp string 模式命令含 shell 元字符一律拒绝。`，`guard_command_execution` 本体与其余文字保留。
   - :140 面基线冻结面枚举摘除 `MiniApp UI / `。

4. **`README.md:43`**
   - 前：`**Frozen-experimental**: CLI, server, MiniApp UI, SDLC harness.`
   - 后：`**Frozen-experimental**: CLI, server, SDLC harness.`

5. **`docs/tech-debt-cleanup-guide.md:12,75,115`**
   - :12 冻结面枚举摘除 `MiniApp 运行时 UI、`。
   - :75 web-ui 引用文档列表摘除已删除的 `MiniApp/Skills/miniapp-dev/SKILL.md`。
   - :115 surfaces 表格枚举面列表摘除 `MiniApp/`。

6. **`docs/architecture/backend-roadmap.md`**
   - :85 SW1-1 行标记为 `~~SW1-1~~ | ~~MiniApp shell/net 空 allowlist=放行（语义翻转）~~ | **随 MiniApp 整删关闭（moot）**（2026-08-17，T2-2）`。
   - :96 依赖关系行更新为已完成事实（`MiniApp 已整删（commits a930c93..T2-2o）——T1-1 / T3-5 随子系统关闭`，启动入口已全数摘除）。
   - :117 MiniApp host 状态更新为 `已整删（T2-2 M1-M5, commits a930c93..T2-2o）`。
   - :167 T2-2 行标记为 `~~T2-2~~ | **已完成**（2026-08-19）：... MiniApp 子系统整删（内置 6 套资产 / 宿主 / 顶层 MiniApp/，M1-M5 commits a930c93..T2-2o；连带关闭 T1-1、T3-5）...`（整行 Done）。
   - :185 T2-5 unwrap 治理清单摘除 `miniapp::manager`。
   - :190-206 PCS-3 权限框架语义段**保留不动**（自足设计依据）。
   - :216 T3-5 行补全关闭回链（`随 MiniApp 整删关闭（2026-08-17，随 T2-2 M1-M5 完成）`）。
   - :247 MiniApp 第三方生态行标注为 `~~MiniApp 第三方生态~~ | **已失效**（MiniApp 子系统已整删；将来如需 = 2.0 协议插件形态）`。

7. **`docs/status/decision-register.md:40`**
   - P-14 行补充执行回链：`已执行：T2-2 M1-M5，commits a930c93..T2-2o`。

8. **`docs/status/tech-debt-ledger.md`**
   - 新增 **P2-21** 条目：
     ```markdown
     ### P2-21: MiniApp 契约层三处 serde/wire 残留（零构造零生产者，反序列化兼容悬置待决）

     - **Symptom**: MiniApp 子系统整删后，契约层保留了三处 serde/wire 残留：`core-types/src/surface.rs:52` `RuntimeArtifactKind::MiniApp`、`services-core/src/session/session_metadata.rs:27` `SessionRelationshipKind::Miniapp`、`services-core/src/session/lineage.rs:19` `"miniapp"` tag。当前代码中零构造、零生产者，但直接删除存在旧会话/工件数据反序列化兼容风险。
     - **Evidence**: T2-2 MiniApp recon Q7 (`.superpowers/sdd/task-t2-2-miniapp-recon.md`)；`rg` 实测全仓零业务构造。
     - **Proposed fix**: 2026-08-19 用户决策超时未拍板，默认保守路径悬置待决。后续若确认无旧数据迁移负担可整删变体，或在反序列化层增加 serde alias/fallback 后删除。
     - **Status**: active (suspended / pending user decision)
     ```

### D. 测试残留切除
9. **`src/crates/execution/agent-stream/src/tool_call_accumulator.rs:150`**
   - 删除了测试用例表中的 `("InitMiniApp", "Markdown Viewer"),` 单行，其余单字段工具用例（Bash/Skill/Read/GetFileDiff/LS/Delete/Glob/Grep/WebSearch/WebFetch）完整保留，48 个单元测试全部通过。

---

## 2. 验证命令与输出原文

### 验证 1: Workspace 编译检查
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
输出：
```text
    Checking northhing-agent-stream v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-stream)
    Checking northhing-ai-adapters v0.2.10 (E:\agent-project\northing\src\crates\adapters\ai-adapters)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 47.10s
```

### 验证 2: Desktop MSVC 门禁检查
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing
```
输出：
```text
    Checking northhing-agent-stream v0.2.10 (E:\agent-project\northing\src\crates\execution\agent-stream)
    Checking northhing-ai-adapters v0.2.10 (E:\agent-project\northing\src\crates\adapters\ai-adapters)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 46.52s
```

### 验证 3: agent-stream 单元测试
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-agent-stream
```
输出：
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 10.60s
     Running unittests src\lib.rs (target\debug\deps\northhing_agent_stream-09c8cb060f03cdfa.exe)

running 48 tests
test tests::derives_watchdog_timeout_from_stream_idle_timeout ... ok
test tool_call_accumulator::tests::ask_user_question_truncated_mid_options_is_recovered ... ok
test tool_call_accumulator::tests::ask_user_question_truncated_mid_chinese_string_is_recovered ... ok
test tool_call_accumulator::tests::json_string_arguments_for_single_field_tools_are_schema_errors_not_rewritten ... ok
test tool_call_accumulator::tests::does_not_wrap_incomplete_json_object_as_raw_string_argument ... ok
test tool_call_accumulator::tests::invalid_json_becomes_error_with_empty_object ... ok
test tool_call_accumulator::tests::git_duplicate_subcommand_in_args_is_left_for_tool_schema_diagnostic ... ok
test tool_call_accumulator::tests::does_not_infer_git_operation_from_ambiguous_args_only_object ... ok
test tool_call_accumulator::tests::does_not_repair_incomplete_json_object_for_multifield_tools ... ok
test tool_call_accumulator::tests::fenced_raw_arguments_for_single_field_tools_stay_invalid_json ... ok
test tool_call_accumulator::tests::incomplete_json_object_for_single_field_tools_stays_invalid ... ok
test tool_call_accumulator::tests::does_not_repair_raw_string_arguments_for_multifield_tools ... ok
test tool_call_accumulator::tests::does_not_execute_truncated_incomplete_json_object ... ok
test tool_call_accumulator::tests::bash_truncated_mid_command_still_errors_but_records_truncation ... ok
test tool_call_accumulator::tests::does_not_repair_object_without_key_value_payload ... ok
test tool_call_accumulator::tests::git_args_only_object_is_left_for_tool_schema_diagnostic ... ok
test tool_call_accumulator::tests::empty_argument_delta_is_ignored ... ok
test tool_call_accumulator::tests::finalizes_complete_json_only_at_boundary ... ok
test tool_call_accumulator::tests::finalized_arguments_preserve_object_fields ... ok
test tool_call_accumulator::tests::id_only_orphan_is_dropped_on_finalize ... ok
test tool_call_accumulator::tests::id_only_prelude_is_attached_to_following_payload_without_id ... ok
test tests::skips_duplicate_finalized_tool_call_id_from_tail_chunks ... ok
test tests::keeps_collecting_tool_args_across_usage_chunks ... ok
test tests::whitespace_only_text_is_not_effective_output ... ok
test tests::marks_token_limit_truncated_text_as_partial_recovery ... ok
test tests::replaces_tool_args_when_snapshot_chunk_arrives ... ok
test tests::preserves_empty_reasoning_presence_for_replay ... ok
test tests::token_limit_with_tool_calls_is_not_partial_recovery ... ok
test tests::finalizes_tool_after_same_chunk_finish_reason ... ok
test tests::keeps_interleaved_indexed_tool_calls_separate ... ok
test tests::does_not_repair_tool_args_with_one_extra_trailing_right_brace ... ok
test tests::natural_stop_finish_reason_is_not_partial_recovery ... ok
test tool_call_accumulator::tests::json_with_one_extra_trailing_right_brace_stays_invalid ... ok
test tool_call_accumulator::tests::manages_multiple_pending_tool_calls_by_index ... ok
test tool_call_accumulator::tests::raw_string_arguments_for_single_field_tools_stay_invalid_json ... ok
test tool_call_accumulator::tests::repair_closes_nested_brackets_in_correct_order ... ok
test tool_call_accumulator::tests::repair_preserves_escaped_quote_inside_truncated_string ... ok
test tool_call_accumulator::tests::repair_refuses_truncation_after_colon ... ok
test tool_call_accumulator::tests::repair_refuses_truncation_after_comma ... ok
test tool_call_accumulator::tests::repair_returns_none_for_already_valid_json ... ok
test tool_call_accumulator::tests::repairs_git_json_string_command_arguments ... ok
test tool_call_accumulator::tests::repairs_git_raw_command_arguments ... ok
test tool_call_accumulator::tests::replace_arguments_overwrites_partial_buffer ... ok
test tool_call_accumulator::tests::todo_write_truncated_mid_content_is_recovered ... ok
test tool_call_accumulator::tests::write_like_recovery_classification_matches_tool_presentation_contract ... ok
test tool_call_accumulator::tests::write_truncated_mid_content_string_is_recovered ... ok
test tool_call_accumulator::tests::write_truncated_with_chinese_multibyte_is_recovered ... ok
test tests::recovers_partial_text_when_cancellation_allows_partial_recovery ... ok

test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

### 验证 4: Core 边界规则检查
```powershell
node scripts/check-core-boundaries.mjs
```
输出：
```text
Core boundary check passed.
```

### 验证 5: 边界规则 miniapp 命中清零复核
```powershell
rg -i miniapp scripts/core-boundaries
```
输出：
```text
(no output - 0 hits)
```

### 验证 6: `git status --short`
```powershell
git status --short
```
输出：
```text
 M .opencode/model-capability-notes.md
 M AGENTS-CN.md
 M AGENTS.md
 D MiniApp/Demo/git-graph/README.md
 D MiniApp/Demo/git-graph/meta.json
 D MiniApp/Demo/git-graph/package.json
 D MiniApp/Demo/git-graph/source/build.js
 D MiniApp/Demo/git-graph/source/esm_dependencies.json
 D MiniApp/Demo/git-graph/source/index.html
 D MiniApp/Demo/git-graph/source/style.css
 D MiniApp/Demo/git-graph/source/styles/detail-panel.css
 D MiniApp/Demo/git-graph/source/styles/graph.css
 D MiniApp/Demo/git-graph/source/styles/layout.css
 D MiniApp/Demo/git-graph/source/styles/overlay.css
 D MiniApp/Demo/git-graph/source/styles/tokens.css
 D MiniApp/Demo/git-graph/source/ui.js
 D MiniApp/Demo/git-graph/source/ui/bootstrap.js
 D MiniApp/Demo/git-graph/source/ui/components/contextMenu.js
 D MiniApp/Demo/git-graph/source/ui/components/findWidget.js
 D MiniApp/Demo/git-graph/source/ui/components/modal.js
 D MiniApp/Demo/git-graph/source/ui/graph/layout.js
 D MiniApp/Demo/git-graph/source/ui/graph/renderRowSvg.js
 D MiniApp/Demo/git-graph/source/ui/main.js
 D MiniApp/Demo/git-graph/source/ui/panels/detailPanel.js
 D MiniApp/Demo/git-graph/source/ui/panels/remotePanel.js
 D MiniApp/Demo/git-graph/source/ui/services/gitClient.js
 D MiniApp/Demo/git-graph/source/ui/state.js
 D MiniApp/Demo/git-graph/source/ui/theme.js
 D MiniApp/Demo/git-graph/source/worker.js
 D MiniApp/Demo/git-graph/storage.json
 D MiniApp/Demo/icon-design-system/meta.json
 D MiniApp/Demo/icon-design-system/source/esm_dependencies.json
 D MiniApp/Demo/icon-design-system/source/index.html
 D MiniApp/Demo/icon-design-system/source/style.css
 D MiniApp/Demo/icon-design-system/source/ui.js
 D MiniApp/Demo/icon-design-system/source/worker.js
 D MiniApp/Skills/miniapp-dev/SKILL.md
 D MiniApp/Skills/miniapp-dev/api-reference.md
 D MiniApp/Skills/miniapp-dev/architecture.md
 D MiniApp/Skills/miniapp-dev/design-playbook.md
 M README.md
 M docs/architecture/backend-roadmap.md
 M docs/status/decision-register.md
 M docs/status/surfaces.md
 M docs/status/tech-debt-ledger.md
 M docs/tech-debt-cleanup-guide.md
 M memory/northhing.md
 M src/crates/execution/agent-stream/src/tool_call_accumulator.rs
?? .handoffs/handoff-g2-t9-2026-08-07.md
?? .superpowers/sdd/task-t2-2o-brief.md
```

---

## 3. 全仓终扫与归类表

执行命令：
```powershell
rg -n -i "miniapp|mini_app|mini-app" --glob '!docs/archive/**' --glob '!docs/handoffs/**' --glob '!docs/superpowers/**' --glob '!.superpowers/**' --glob '!memory/**' --glob '!research/**' --glob '!target/**' --glob '!docs/migration-2026-07-16/**'
```

### 命中归类汇总表

| 归类 | 命中文件与行号 | 性质说明 | 处理状态 |
|---|---|---|---|
| **契约层三处 serde/wire 残留** | `src/crates/contracts/core-types/src/surface.rs:52`<br>`src/crates/services/services-core/src/session/session_metadata.rs:27`<br>`src/crates/services/services-core/src/session/lineage.rs:19` | `RuntimeArtifactKind::MiniApp`、`SessionRelationshipKind::Miniapp`、`"miniapp"` tag。零构造零生产者，反序列化兼容风险 | **红线保留，登记 P2-21 待用户决断** |
| **技术债台账** | `docs/status/tech-debt-ledger.md:232,234,235` | P2-21 债项条目登记 | **已登记** |
| **路线图与决策记录** | `docs/architecture/backend-roadmap.md:85,96,117,151,167,179,190,192,216,247`<br>`docs/status/decision-register.md:19,40,64,71`<br>`docs/product-thesis.md:3,51,52` | SW1-1(moot)、T1-1/T3-5(关闭)、MiniApp host(已整删)、T2-2(Done)、P-14(已执行回链)、PCS-3 权限设计语义段自足保留、第三方生态(失效) | **已收口 / 授权保留** |
| **历史审计 / 规划快照** | `docs/status/full-review-2026-08-16.md`<br>`docs/status/2026-07-23-p2-9-stage2-triage.md`<br>`docs/architecture/core-decomposition.md`<br>`docs/architecture/agent-runtime-services-design.md`<br>`docs/architecture/agent-kernel-northstar.md`<br>`docs/sdlc-harness/**`<br>`docs/security/r1-shell-exec-audit.md`<br>`docs/product/PRD-v0.1.0.md`<br>`docs/plans/**`<br>`docs/reviews/**` | 历史版本基线、既往全量审计记录、历史 PRD/演进计划 | **历史事实，按惯例不改** |
| **负向防卫注释/护栏** | `src/crates/services/services-core/AGENTS.md:25` | 架构守则负向约束："Do not add ... MiniApp storage ..." | **保留** |

**代码面残留结论**：除授权悬置的契约层 3 处变体/tag 之外，**全仓活动代码面 MiniApp 引用已完全清零**，无任何新代码残留。

---

## 4. 编译错误与分层修复记录

- 本批次编译错误数：0。
- 零编译错误，无需机制层/设计层修复。

---

## 5. 偏离说明

- 零偏离。全部改动严格对齐 brief A-D 项清单，逐字遵守红线限制（未碰悬置 serde 契约、未改 PCS-3 段、未改 guard_command_execution 本体、未触碰并行 session 文件）。
