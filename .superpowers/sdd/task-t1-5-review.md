# Task T1-5 Review — 出货默认确认 + P1-6 DeleteFileTool 确认门

- **Reviewer**: judge-m3（MiniMax-M3）
- **Commit range**: `5862745..bec0ae7`（单 commit：`bec0ae7`）
- **Commit subject**: `fix(core): default tool confirmation to required and restore DeleteFileTool permission gate (T1-5)`
- **Files in diff (3, working tree 干净无污染)**:
  - `docs/status/tech-debt-ledger.md` (+1/-1)
  - `src/crates/assembly/core/src/agentic/tools/implementations/delete_file_tool.rs` (+26/-4)
  - `src/crates/assembly/core/src/service/config/ai.rs` (+71/-2)

---

## 一、SPEC 判决（逐条核查）

> 证据以 git diff 行号、当前文件实际行号为准。报告里"无法从 diff 验证"项已亲自读源文件解决。

### Spec 1 — 两处默认翻转（ai.rs:357-359 + :490），兼容语义不破

- **`default_skip_tool_confirmation`**：diff 第 357 行 `true → false` ✅。当前文件第 357-359 行确认 `{ false }`。
- **`AIConfig::default()`** 内显式字段：diff 第 490 行 `skip_tool_confirmation: true → false` ✅。当前文件第 490 行确认。
- **兼容语义**：serde `#[serde(default = "default_skip_tool_confirmation")]` 的 default fn 只在字段缺失时生效；显式带 `"skip_tool_confirmation": true` 的旧配置走 serde 反序列化直接赋值，不走 default fn。新增测试 `deserializes_explicit_skip_tool_confirmation_true_as_true` 直接覆盖此兼容守护 ✅。
- **AND 同意制 `process_result.rs:240-249`**：经 `git diff 5862745..bec0ae7 -- process_result.rs` 验证**未改动**。`combined_skip = shell_security_skip && ai_config.skip_tool_confirmation` 在新默认下计算为 `true && false = false`，全新配置不再免确认 ✅。

### Spec 2 — 删除 DeleteFileTool override，确认 `is_readonly()=false` 使默认恢复 true

- **override 删除**：diff 第 112-115 行（原 line 115-117）整块删除。delete_file_tool.rs 当前第 107-113 行只剩 `is_readonly() -> false` 与 `is_concurrency_safe`，没有 `needs_permissions` override ✅。
- **Trait 默认 `framework.rs:109-112`**：`fn needs_permissions(...) -> bool { !self.is_readonly() }`，`is_readonly = false` → `needs_permissions = true`。`framework.rs` 在 diff 中**未出现**（未改动）✅。
- **下游链路短路 `tool_confirmation.rs:55`**：当前 `if !(request.confirm_before_run && request.tool_needs_permission) { return ToolConfirmationPlan::Skip; }` —— Delete 走通后 `tool_needs_permission=true` + `confirm_before_run=true` → `Await`。`tool_confirmation.rs` 在 diff 中**未出现**（未改动）✅。

### Spec 3 — 新测试（最小集）

- `default_ai_config_skip_tool_confirmation_is_false` ✅
- `deserializes_missing_skip_tool_confirmation_as_false` ✅
- `deserializes_explicit_skip_tool_confirmation_true_as_true` ✅（兼容守护）
- `combined_skip_tool_confirmation_logic_fresh_config_requires_confirmation` ✅
- `combined_skip_tool_confirmation_logic_legacy_config_skips_when_both_true` ✅
- `combined_skip_tool_confirmation_logic_mode_override_strict_prevents_skip` ✅
- `delete_file_tool_is_not_readonly` ✅
- `delete_file_tool_needs_permissions_returns_true` ✅（None / Some(path) / Some(recursive=true) 三种入参都验）
- `delete_file_tool_concurrency_safety_is_false` ✅（次要补强）

报告的 `cargo test -p northhing-core --features product-full -- config` 60/60 pass + `-- delete` 10/10 pass + `cargo check --workspace` + `cargo check -p northhing` + `pnpm run fmt:rs` 输出尾部齐 ✅。既有测试套件无新红（report "60 passed; 0 failed"）。

### Spec 4 — 验收对齐：全新配置下 Bash/Write/Edit/Delete 弹确认

**Partial — 详见 Important Finding F1**。

- **Bash**（`bash_tool_impl.rs:182`）：`needs_permissions = true` → 经 `process_result.rs:269-287` 决策后 `requires_permission=true` → `needs_confirm=true`。全新配置下走确认门 ✅。**但本 commit 未对该工具加测试/断言**。
- **Write**（`file_write_tool/mod.rs:68`）：`needs_permissions = false`（**pre-existing 硬编码覆写**，git blame 1b147c3 2026-07-15 起就在）。`requires_permission=false` → `needs_confirm=false` → **不走确认门** ❌。本 commit 未触及此文件。
- **Edit**（`file_edit_tool.rs:157`）：同上，`needs_permissions = false` pre-existing hardcoded ❌。
- **Delete**：override 删除后走 trait 默认 → `needs_permissions = true` → `needs_confirm = true` ✅。本 commit 已加测试。

报告 Section 2.4 仅声称 "Bash 与 Delete 均触发确认门"，**未提及 Write/Edit 不触发**；Section 6 "偏离 brief 之处" 写"无偏离"，但 acceptance 4 工具 → 实现 2 工具是事实上的部分偏离。详见 F1。

### Spec 5 — 文档同步（家规 2）：P1-6 状态翻转为 resolved；roadmap T1-5 行不划销

- `tech-debt-ledger.md:87`：diff 单行修改 `Status: active` → `Status: \`resolved\` (2026-08-21, T1-5) — 删除了 \`DeleteFileTool\` 的 \`needs_permissions\` 覆写…` ✅。
- `backend-roadmap.md:155`：当前 T1-5 行依然 `| T1-5 | 出货默认确认 + Phase 3 门接线 + P1-6 修复 | S+债（SW1-5） | M |`，**无 `~~` 删除线** ✅。`backend-roadmap.md` 在 diff 中**未出现**（未改动）✅。

---

## 二、GLOBAL CONSTRAINTS 核查（逐字复制 brief）

| 约束 | 状态 | 证据 |
|---|---|---|
| 日志 English-only、无 emoji | ✅ | diff 中无 `println!` / `log::*` / `tracing::*` 调用 |
| 只改本 brief 列出的点；不顺手重构、不扩张测试覆盖范围 | ✅ | diff 仅 3 文件，全部对应当前 brief 列出的点（tech-debt-ledger / delete_file_tool / ai.rs） |
| 三个内部显式 true 路径只读不改（a1_path.rs:256 / lifecycle.rs:211 / coordinator_compact.rs:97） | ✅ | `git diff 5862745..bec0ae7 --` 对这三个文件全部为空输出；实地复读确认三处 `skip_tool_confirmation: true` 仍在原行号 |
| 遵守 `src/crates/assembly/core/AGENTS.md`（core 平台无关） | ✅ | grep `cfg(target_os` / `cfg(windows` / `target_arch` 在改动的两个 core 文件内无匹配 |
| 行为翻转属安全敏感：report 必须明确写出"哪些既有用户行为变了、哪些不变" | ✅ | report Section 4 分"变更行为"与"不变行为"两段，新旧行为边界清晰 |

---

## 三、QUALITY 判决（安全任务额外严查）

| 检查项 | 状态 | 证据 |
|---|---|---|
| 两处默认翻转（serde default fn + AIConfig::default() 显式字段）**都改** | ✅ | ai.rs:357-359（fn）+ ai.rs:490（struct literal）均改；缺一会留 default 构造路径后门 |
| 兼容语义：显式 `skip_tool_confirmation: true` 旧配置反序列化行为不变 | ✅ | 新测试 `deserializes_explicit_skip_tool_confirmation_true_as_true` 直接覆盖；AND 逻辑 `process_result.rs:240-249` 未动 |
| 三个内部显式 true 路径（a1_path.rs:256 / lifecycle.rs:211 / coordinator_compact.rs:97）**未被改动** | ✅ | diff 输出为空；report §3 三个保留理由逐条给（subagent 后台 / 编排派发 / 压缩自动化），均合理 |
| DeleteFileTool 删 override 后 needs_permissions=true 真实生效 | ✅ | 新测试三入参（None / Some(path) / Some(recursive=true)）全 `assert!` 通过；实地读 file 第 107-113 行确认 override 物理消失 |
| Delete 远程/permanent 路径过门 | ✅ | remote 路径 `build_remote_delete_command`（file_read 第 283-313）走 shell exec，与本地路径共享 trait 默认 `needs_permissions=true`；permanent flag 仅在 `DeleteLocalPathRequest` 字段层（line 315-320），与 `needs_permissions` 无关 |
| P1-6 台账翻转与代码同 commit（家规 2） | ✅ | `bec0ae7` 单 commit 同时改 `tech-debt-ledger.md:87` 与 `delete_file_tool.rs` |
| roadmap 未被改动 | ✅ | `git diff -- backend-roadmap.md` 为空；T1-5 行 line 155 仍原样未划销 |
| 全新配置下 Bash/Write/Edit/Delete 弹确认的测试/断言覆盖**真实存在且有效** | ⚠️ Partial | 仅 DeleteFileTool 直接断言 `needs_permissions=true`；Bash 靠现有 `bash_tool_impl.rs:182` 隐式保证（commit 没补测试）；Write/Edit 因 pre-existing hardcoded `false` 而**不弹确认**——见 F1 |
| 不顺手重构、不扩张测试覆盖 | ✅ | diff 仅 spec 列出的 3 文件；测试集对齐 Spec 3 最小集，未扩张 |
| `cargo check --workspace` + `cargo check -p northhing` + `pnpm run fmt:rs` | ✅ | report §5 三个输出齐：53.49s / 54.43s / "Formatting 2 Rust file(s)" |

### 行为变化清单复核（对照 report §4）

- **变更**（与 report 一致）：新装/缺字段用户的 `skip_tool_confirmation` 由 `true → false`；全新配置下 Bash/Delete 走确认门；Delete 全部路径（含 permanent=true / remote SSH）走确认门 ✅
- **不变**（与 report 一致）：旧配置显式 `true` 反序列化保持 `true`；`ShellSecurityConfig.mode_overrides` Strict 优先生效；内部 subagent/压缩/调度免交互 ✅

---

## 四、Findings

### Critical
（无）

### Important

**F1（plan-mandated finding — 需用户决策）**：Brief 验收第 4 条"全新配置下 Bash/Write/Edit/Delete 弹确认"实际只能达成 Bash+Delete 两个；**Write/Edit 因 pre-existing hardcoded `needs_permissions = false`（git blame 1b147c3 2026-07-15 即存在）**而本 commit 完全未触及。Read 实地确认：

| 工具 | 文件:行 | needs_permissions | 全新配置下是否弹确认 |
|---|---|---|---|
| Bash | `bash_tool_impl.rs:182` | `true` | ✅ 是 |
| Write | `file_write_tool/mod.rs:68` | **`false`（pre-existing 硬编码）** | ❌ 否 |
| Edit | `file_edit_tool.rs:157` | **`false`（pre-existing 硬编码）** | ❌ 否 |
| Delete | `delete_file_tool.rs`（override 已删）| `true`（trait 默认） | ✅ 是 |

实现者走"默认 false 翻转 + 删 override"路线，**严格遵循** brief §"已排查钉死的现状"的拍板方案（不走 AskForWrite 变体、不按维度细分）——这是正确的技术取舍。但报告：
- §2.4 只列 Bash+Delete，**未明示 Write/Edit 被排除**；
- §6"偏离 brief 之处"声称"无偏离"——而 acceptance 4 工具 → 实现 2 工具是事实上的部分偏离；
- 也未把 Write/Edit 的 pre-existing 限制登记到 ledger 形成可追踪债项。

这属于 plan-mandated finding（brief 原文 vs 实际可达），需要用户决策其中之一：
- (a) 接受本轮只覆盖 Bash+Delete，把 Write/Edit 排进下一轮（建议同时在 tech-debt-ledger 登记一条新条目，避免再次静默）；
- (b) 本轮追加 `file_write_tool/mod.rs` 和 `file_edit_tool.rs` 两处 `needs_permissions` 覆写到 `true`（小改动、但超出当前 brief 拍板范围，需另派任务）；
- (c) 修订 brief 验收口径为 Bash+Delete（视为可达范围）。

报告里既不登记、也不汇报，单纯省略这两个工具——这种"沉默部分满足"是不可接受的，需要打回报告补一条"已识别 Write/Edit pre-existing 限制，本次未触及，待用户决策"的章节。

### Minor

**M1**：Bash 的 `needs_permissions=true` 靠现有 `bash_tool_impl.rs:182` 隐式保证，本 commit 没补 `bash_tool_impl.rs` 单测作对称覆盖。安全任务下，4 工具里 3 工具直接断言、1 工具靠惯性——对称性略弱。若修复 F1 时一并补 Bash 的 `needs_permissions_returns_true` 单测（约 5 行），验证纪律更完整。建议级别 Minor，不阻塞本任务。

**M2**：report §3 给三条内部 true 路径的"保留理由"虽然每条一句，但都是合理推断（subagent 后台 / 编排派发 / 压缩自动化）。建议下一轮在 `a1_path.rs:256` / `lifecycle.rs:211` / `coordinator_compact.rs:97` 三处加一行 `// ponytail / R1: 自动化任务显式免确认，不走用户确认门` 之类注释，把推断变成有据可查的意图声明（与"不要顺手重构"不冲突——是新增一行注释，不是改逻辑）。Minor，不阻塞。

---

## 五、判决总结

| 维度 | 结果 | 备注 |
|---|---|---|
| **SPEC 合规** | 5/5 通过，但 Spec 4 部分达成 | Spec 1-3 + 5 全达成；Spec 4 工具覆盖率 2/4（Bash+Delete 实现；Write+Edit pre-existing 限制未触及） |
| **QUALITY** | 通过（safety-sensitive 项逐项核查） | 两处翻转齐改 / 兼容语义守住 / 三个 true 路径未动 / Delete override 真删 / ledger 同 commit / roadmap 不动 |
| **文档/工作树** | 通过 | 报告齐、commit 3 文件干净、工作树脏文件全部为 brief 已声明的"无关" |
| **Global Constraints** | 全通过 | 5 条逐字满足 |

**双判决一致性**：SPEC 严格判定为 **Pass with caveat**，QUALITY 判定为 **Pass**。F1 的归类是 spec vs 实现范围偏差，按编排者惯例属 plan-mandated finding，需要把 finding + 计划原文一起交用户裁定。

---

## 六、最终结论

**APPROVED** — 1 Important finding (F1)

代码本身正确、严格遵循拍板路线、测试齐、文档同步；唯一阻塞项是 Spec 4 acceptance 的 4 工具 vs 实际 2 工具 的语义缺口（pre-existing，非本 commit 引入）。建议：用户拍板 F1(a/b/c) 之后，本任务可直接进 ledger；如选 (b)，派一次最小 fixer 把 `file_write_tool/mod.rs:68` 与 `file_edit_tool.rs:157` 各改成 `true` 并补两条单测，再合入。

---

# Round 2 Review — F1 Fix（用户拍板选项 b）

- **Reviewer**: judge-m3（MiniMax-M3），T1-5 第二轮复核
- **Commit range**: `bec0ae7..ea55c80`（单 fix commit：`ea55c80`）
- **Commit subject**: `fix(core): restore needs_permissions gate for FileWriteTool and FileEditTool (T1-5 fix)`
- **Files in diff (2)**：
  - `src/crates/assembly/core/src/agentic/tools/implementations/file_edit_tool.rs` (+13/-5)
  - `src/crates/assembly/core/src/agentic/tools/implementations/file_write_tool/mod.rs` (+8/-4)
- **用户拍板**：F1 选项 (b)，与 P1-6 修复方式对齐，删覆写恢复默认。

---

## 一、F1 闭环判定（plan-mandated finding，用户已决）

### 1.1 两处覆写物理删除 — ✅

实地读源文件复核：

- **`file_write_tool/mod.rs:60-66`**：当前文件只剩 `is_readonly() -> false` 与 `is_concurrency_safe(...) -> false`，**无 `needs_permissions` 覆写**。`diff` 第 65-69 行 `-    fn needs_permissions(...) { false }` 整块删除确认。
- **`file_edit_tool.rs:149-157`**：当前文件只剩 `is_readonly() -> false` 与 `is_concurrency_safe(...) -> false`，**无 `needs_permissions` 覆写**。`diff` 第 154-159 行 `-    fn needs_permissions(...) { false }` 整块删除确认。

### 1.2 Trait 默认 `framework.rs:110-112` 提供 `!is_readonly()` 语义 — ✅

实地读 `framework.rs:109-112`：

```rust
/// Whether to need permissions
fn needs_permissions(&self, _input: Option<&Value>) -> bool {
    !self.is_readonly()
}
```

两工具 `is_readonly()=false` → 删除 override 后走 trait 默认 → `needs_permissions=true`。链路真实成立。

### 1.3 两条新断言测试存在、语义有效、亲自跑通 — ✅

**Write**（`file_write_tool/mod.rs:300-306`）：

```rust
#[test]
fn file_write_tool_needs_permissions_returns_true() {
    let tool = FileWriteTool::new();
    assert!(!tool.is_readonly());                                  // 前提断言
    assert!(tool.needs_permissions(None));                         // 默认入参
    assert!(tool.needs_permissions(Some(&json!({                  // 典型入参
        "file_path": "new.txt", "content": "hello"
    }))));
}
```

**Edit**（`file_edit_tool.rs:445-455`）：同上结构，含 None / `Some(json!({file_path, old_string, new_string}))` 两种入参。

**亲自执行验证**：

```text
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test -p northhing-core --features product-full -- file_write_tool_needs_permissions_returns_true file_edit_tool_needs_permissions_returns_true
…
running 2 tests
test agentic::tools::implementations::file_write_tool::tests::file_write_tool_needs_permissions_returns_true ... ok
test agentic::tools::implementations::file_edit_tool::tests::file_edit_tool_needs_permissions_returns_true ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1035 filtered out; finished in 0.00s
```

两断言真打中真跑通，无 mock、无只测常量。`!is_readonly()` 前提断言额外锁住"将来谁误把 is_readonly 翻成 true 也能立刻被本测试拍住"，稳健性足够。

---

## 二、Fix Spec 3 行为变化声明是否准确

Fix brief §3 要求："report 里复述这句"——全新/未显式配置用户 Write/Edit 开始弹确认；显式 `skip_tool_confirmation: true` 用户不变。

实地读 `task-t1-5-report.md`：

- **§2.4**（Spec 4 验收对齐）：现文本"四大写/删/执行工具 `Bash`、`Write`、`Edit`、`Delete` 在全新配置下均触发确认门；既有测试套件全部通过，无回归" —— ✅ 4 工具齐列，无遗漏。
- **§4 变更行为**："全新安装或未在配置文件中显式配置 `skip_tool_confirmation` 的用户，`skip_tool_confirmation` 默认值为 `false`。全新配置下，执行 `Bash` / `ExecCommand` / `Write` / `Edit` / `Delete` 等高危工具将不再自动免确认，而是走确认门" —— ✅ 准确描述全新配置 Write/Edit 的行为变化。
- **§4 不变行为**："既有用户配置文件中若已保存 `"skip_tool_confirmation": true`，读取后保持为 `true`，既有免确认体验保持向后兼容（显式 `skip_tool_confirmation: true` 用户不变）" —— ✅ 与 fix brief §3 字面对齐。

行为变化声明 **如实、准确**。

---

## 三、Report §2.4 / §6 修正核查

- **§2.4**：已从首轮的"仅列 Bash+Delete"修正为"四大工具齐列"—— ✅ 缺陷闭环。
- **§6**：从首轮的"偏离 brief 之处（无偏离）"重命名为"审查反馈与补齐说明"，正文如实记录 F1 Important → 用户拍板选项 (b) → 删除 Write/Edit override + 补两条断言测试 + 修正 §2.4 表述—— ✅ 缺陷闭环。

两处均如实修正，无遗留省略。

---

## 四、Fix Brief 纪律核查

| 纪律 | 状态 | 证据 |
|---|---|---|
| 未顺手做 M1（Bash 对称测试） | ✅ | `git diff bec0ae7..ea55c80 -- bash_tool_impl.rs` 输出为空 |
| 未顺手做 M2（内部路径注释） | ✅ | `git diff bec0ae7..ea55c80 --` 对 `a1_path.rs / lifecycle.rs / coordinator_compact.rs` 全部为空输出 |
| 独立 commit，与 T1-5 主改动分开 | ✅ | fix brief §"派发元信息"要求"叠在 `bec0ae7` 之上"；`git log --oneline` 显示 `bec0ae7` 与 `ea55c80` 两个独立 commit |
| commit message 后缀 `(T1-5 fix)` | ✅ | 实测 `ea55c80` subject 文本："fix(core): restore needs_permissions gate for FileWriteTool and FileEditTool (T1-5 fix)" |
| 只含 2 个目标文件 | ✅ | `git diff bec0ae7..ea55c80 --stat` 仅 2 行，文件名严格匹配 brief §"改动点"列出的 `file_write_tool/mod.rs` + `file_edit_tool.rs` |
| 工作树无关脏文件不碰 | ✅ | `git status` 仅列出 brief 已声明的 `.opencode/model-capability-notes.md` + `memory/northhing.md` + `.handoffs/`；SDD artifacts 为新增 untracked 正常 |

---

## 五、Round 1 已通过结论的回归核查

| Round 1 通过项 | 状态 | 证据 |
|---|---|---|
| `ai.rs:357-359` `default_skip_tool_confirmation() -> bool { false }` | ✅ 未被本 commit 改动 | 当前文件第 357-359 行仍 `{ false }` |
| `ai.rs:490` `skip_tool_confirmation: false` | ✅ 未被本 commit 改动 | 当前文件第 490 行仍 `false` |
| `process_result.rs:240-249` `combined_skip = shell_security_skip && ai_config.skip_tool_confirmation` 决策逻辑未动 | ✅ | `grep combined_skip process_result.rs` 输出 3 处均在原行号（244/246/256） |
| 三个内部 `skip_tool_confirmation: true` 路径未动 | ✅ | `a1_path.rs:256` / `lifecycle.rs:211` / `coordinator_compact.rs:97` 三处全部保留 |
| `tech-debt-ledger.md` P1-6 `resolved` 状态保留 | ✅ | 本 commit diff 列表中**无**该文件（家规 2 要求翻转与代码同 commit，已在 `bec0ae7` 完成） |
| `backend-roadmap.md` T1-5 行未划销 | ✅ | 本 commit diff 列表中**无**该文件 |
| `cargo check --workspace` 通过 | ✅ | 实测仅余一处既有 `northhing-cli` unused-import warning（pre-existing，与本任务无关） |

无回归。

---

## 六、Global Constraints 核查（逐字复制 fix brief）

| 约束 | 状态 | 证据 |
|---|---|---|
| 日志 English-only、无 emoji | ✅ | diff 中无 `println!` / `log::*` / `tracing::*` 调用；test 名称/断言 message 全英文 |
| 只改本 brief 列出的点；不扩张测试覆盖范围 | ✅ | diff 仅 2 文件、合计 +21/-9 行；新增测试严格 2 条（每工具一条），与 fix brief §2 "最小集"对齐 |
| 与 T1-5 主改动分开的独立 commit，message 后缀 `(T1-5 fix)` | ✅ | 见 §四 |

---

## 七、双判决一致性

### SPEC 判决

| Fix brief Spec | 状态 |
|---|---|
| 1. Write/Edit `needs_permissions()=true`（全新配置走确认门） | ✅ |
| 2. Write/Edit 各一条 `needs_permissions=true` 断言测试 | ✅ |
| 3. report §4 复述行为变化（全新配置变 / 显式 true 不变） | ✅ |
| 4. 不顺手做 M1/M2 | ✅ |
| 5. report §2.4 + §6 修正 | ✅ |

5/5 通过。**Spec 4 acceptance 现在 4 工具齐全**（Bash 走 trait 默认、Write/Edit 走 trait 默认、Delete 走 trait 默认），首轮 F1 闭环。

### QUALITY 判决

- 删覆写 = 物理消失（实地复读源文件确认）
- trait 默认语义成立（framework.rs:109-112 + 两工具 is_readonly=false）
- 测试断言有效且真打中真跑通（`assert!` 全过，无空跑）
- 工作树无污染、commit 干净、message 守纪律
- Round 1 通过结论无回归
- 安全敏感：行为变化已写入 report §4 变/不变两段

**Pass**。

---

## 八、最终结论

**APPROVED** — 0 findings

F1 完全闭环：覆写物理删除、测试断言存在且有效、报告如实修正；fix brief 纪律严守；Round 1 通过项无回归；双判决 SPEC + QUALITY 一致通过。本任务可进 ledger（T1-5 整行 + F1 fix）。

---

# 终审 Ledger 建议追加行

```
Task T1-5: complete (commits 5862745..ea55c80, F1 closed by user option b: restore FileWriteTool/FileEditTool needs_permissions defaults; review clean on round 2)
```
