# Task T2-2p Review — P2-21 执行：MiniApp 契约层三处 serde/wire 残留删除

- **工作目录**: `E:\agent-project\northing`
- **HEAD (实测)**: `11189d1 docs: handoff 2026-08-19 T2-2 fully done (C8 + remote final + MiniApp M1-M5 + MiniApp final)` — 与 implementer 报告一致
- **改动状态**: 工作区未提交；6 文件 modified（4 授权 + 2 并行 session），4 untracked（brief/diff/report/handoff）

---

## SPEC 判决 — PASS

### 1. 授权文件集（约束 #1）
- `M src/crates/contracts/core-types/src/surface.rs` ✓
- `M src/crates/services/services-core/src/session/session_metadata.rs` ✓
- `M src/crates/services/services-core/src/session/lineage.rs` ✓
- `M docs/status/tech-debt-ledger.md` ✓
- 其他 2 modified（`.opencode/model-capability-notes.md`、`memory/northhing.md`）属于 brief 显式声明的"并行 session 未提交改动"，未碰。
- 4 untracked 均为 brief/diff/report/handoff 工作产物。
- **越界 = 无**。

### 2. 三处精确删除（约束 #2）
| 文件:行 | 删除前 | 删除后（实测） |
|---|---|---|
| `surface.rs:52` | `MiniApp,` | `McpManifest,` 紧跟 `ReviewReport,` — 6 变体（Diff/TerminalSnapshot/Preview/Usage/ReviewReport/McpManifest）✓ |
| `session_metadata.rs:27` | `Miniapp,` | `Subagent,` 紧跟 `DeepReview,` — 4 变体（Btw/Review/DeepReview/Subagent）✓ |
| `lineage.rs:19` | `&["btw", "review", "deep_review", "miniapp", "subagent"]` | `&["btw", "review", "deep_review", "subagent"]` — 4 元素，"miniapp" 已摘除 ✓ |

serde 属性保持原样：
- `surface.rs:45`: `#[serde(rename_all = "snake_case")]` ✓
- `session_metadata.rs:22`: `#[serde(rename_all = "snake_case")]` ✓

### 3. rg 复核（约束 #3 — 自跑）

```
$ rg -n "RuntimeArtifactKind::MiniApp" src/ tests/
(no output)

$ rg -n "SessionRelationshipKind::Miniapp" src/ tests/
(no output)

$ rg -n '"miniapp"' src/
(no output)
```

三条全部零命中 ✓。**无外部调用方需要适配**。

### 4. ledger P2-21 翻 resolved（约束 #4）

实测工作区第 237 行：
```
- **Status**: `resolved` — 用户 2026-08-19 拍板删除，本任务执行，commits 见 git log T2-2p。
```

- 状态字段从 `active (suspended / pending user decision)` 翻为 `resolved`，符合 house rule 2。
- 中文部分字节级验证（python 直读 bytes）：
  - 破折号 `—` = `e2 80 94`（U+2014，UTF-8 正确）
  - `用` = `e7 94 a8`，`户` = `e6 88 b7`，`拍` = `e6 8b 8d`，`板` = `e6 9d bf`，`删` = `e5 88 a0`，`除` = `e9 99 a4`，`本` = `e6 9c ac`，`任` = `e4 bb bb`，`务` = `e5 8a a1`，`执` = `e6 89 a7`，`行` = `e8 a1 8c`，`见` = `e8 a7 81`
  - 所有多字节 UTF-8 序列均为合法中文码点
  - **无 GBK 双重编码迹象**（在 HEAD 版本即已存在 pre-existing "mojibake" 在早期 Symptom/Proposed fix 行的中文上，但字节级实际是合法 UTF-8，乱码是 PowerShell 终端显示 artifact；本次新增的 Status 行字节级 UTF-8 干净，与既有中文行格式一致）。
- 注意："Proposed fix" 行未触动（brief 授权范围仅 status 字段），现内容仍描述保守路径悬置待决，但 Status 翻 resolved 后该行事实上过期 — 见 Minor F3。

### 5. 门禁复跑（约束 #5 — 自跑，MSVC wrapper）

```
$ & "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.53s
（仅 pre-existing warnings，无 error）

$ ... cargo test -p northhing-core-types
   test errors::tests::classifies_quota_and_provider_unavailable_errors ... ok
   test errors::tests::builds_ai_error_detail_from_provider_metadata ... ok
   test session_kind_preserves_default_and_serialized_shape ... ok
   test session_kind_preserves_legacy_snake_case_deserialization ... ok
   test permission_and_capability_contracts_keep_source_identity ... ok
   test surface_contract_serializes_observational_runtime_facts ... ok
   test thread_environment_contract_does_not_require_surface_specific_fields ... ok
（2 + 2 + 3 = 7 tests PASS，0 doc-tests）

$ ... cargo test -p northhing-services-core --lib
   test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
（与 implementer 跑 `session` filter 的 48 lib tests 一致；--lib 跑了所有 52 个）
```

三道门禁 PASS ✓。

### 6. 无夹带 / TH-5 词汇未动（约束 #6）

- `DialogTriggerSource::RemoteRelay` 仍在 `agent_dialog.rs:62`、`agent_facade_tests.rs:52/92/160` ✓
- `DialogTriggerSource::Bot` 仍在 `agent_dialog.rs:62`、`agent_facade_tests.rs:56`、`subagent_ports.rs:113` ✓
- `RemoteSsh` 仍在 `surface.rs:25`、`session_usage/types.rs:51/549/559`、`tests/surface_contracts.rs:64`、`assembly/core/src/service/session_usage/snapshot.rs:56`、`assembly/core/src/service/session_usage/persist.rs:30/112/153` ✓
- 重要：implementer 报告未触及这些，未夹带无关格式化（除 F1 行尾问题）。

---

## QUALITY 判决 — PASS WITH CONCERNS（Important 1 / Minor 2）

### Findings

#### F1（Important）— CRLF 行尾回归，影响 2/4 授权文件

实测字节级 CRLF/LF 分布：

| 文件 | HEAD | WORK | 变化 |
|---|---|---|---|
| `src/crates/contracts/core-types/src/surface.rs` | 0 CRLF / 138 LF | 137 CRLF / 0 LF | **整文件 LF → CRLF**（137 行） |
| `src/crates/services/services-core/src/session/session_metadata.rs` | 0 CRLF / 373 LF | 0 CRLF / 372 LF | 仅 -1 行（Miniapp 删除），行尾 LF 未变 ✓ |
| `src/crates/services/services-core/src/session/lineage.rs` | 0 CRLF / 504 LF | 0 CRLF / 504 LF | 仅 -1 元素（"miniapp" 摘除），行尾 LF 未变 ✓ |
| `docs/status/tech-debt-ledger.md` | 0 CRLF / 243 LF | 243 CRLF / 0 LF | **整文件 LF → CRLF**（243 行） |

**证据**：`.gitattributes` 仅规定 `*.rs text eol=lf`，所以 `.rs` 文件理论应保持 LF；`git diff --check` 也确实输出：
```
warning: in the working copy of 'src/crates/contracts/core-types/src/surface.rs', CRLF will be replaced by LF the next time Git touches it
```
（仅 flag 了 surface.rs；ledger.md 因 `.gitattributes` 未覆盖 `.md` 故未被 flag，但实际同样是整文件 CRLF。）

**违反 brief 约束 #6 "无夹带/无关格式化"**：implementer 编辑器（很可能 Edit/Edit 工具或 PowerShell `Set-Content` 默认 CRLF）将 surface.rs（137 行）和 ledger.md（243 行）从 LF 整文件转为 CRLF，是与本任务核心意图无关的格式化变动。

**影响评估**：
- 由于 `core.autocrlf=true`（系统 gitconfig），commit 时 git 会自动将 working tree CRLF 规范化回 LF，所以最终 commit 不会污染 HEAD 历史。
- 但当前 working tree 不一致：`git diff` 表面显示 `1 file changed, 1 deletion(-)`，背后实际是 137 行 CRLF→LF 转换 + 1 行内容删除。
- 若 implementer 或后续 reviewer 用 Unix 风格工具（rg、awk、sed、python on linux）对比 working tree 与 HEAD，会看到 137 行错配。

**修复路径**（commit 前）：
```bash
git config core.autocrlf false  # 临时，避免再次触发
# 或在编辑器中重新以 LF 保存
# 或用 unix2dos/dos2unix 工具转换
```

**严重程度**：Important — 不阻塞任务实质完成，但违反约束 #6 显式条款；建议在 commit 前修复，否则预存改动污染下游审查。

---

#### F2（Minor）— 范围外文档 guardrail 残留 MiniApp 字符串

实测：
```
$ rg -n -i "miniapp" src/ tests/
src/crates/services/services-core/AGENTS.md:25: - Do not add remote SSH, MiniApp storage, tool-result persistence, `PathManager`
```

这处是 `services-core` 模块 AGENTS.md 的 guardrail 条款，禁止把 MiniApp storage 添加到该 crate。**不在 brief 授权的 4 文件集内**，且与 brief 的契约层删除目标无关 — 该行是文档层历史决策，不是 serde/wire 残留。

**严重程度**：Minor — 范围外观察，不在本次 fix 范围。建议未来若 MiniApp 整删决策进一步推进（如 sdd/cleanup），可单独任务处理该 AGENTS.md 措辞。

---

#### F3（Minor）— ledger P2-21 "Proposed fix" 行事实过期

实测 P2-21 条目（工作区 line 232–237）：
```
### P2-21: MiniApp 契约层三处 serde/wire 残留（零构造零生产者，反序列化兼容悬置待决）

- **Symptom**: ...（未动）
- **Evidence**: ...（未动）
- **Proposed fix**: 2026-08-19 用户决策超时未拍板，默认保守路径悬置待决。后续若确认无旧数据迁移负担可整删变体，或在反序列化层增加 serde alias/fallback 后删除。
- **Status**: `resolved` — 用户 2026-08-19 拍板删除，本任务执行，commits 见 git log T2-2p。
```

Status 翻 resolved 后，**"Proposed fix" 行仍描述"未拍板/悬置待决"的旧判断**，与新 Status 不一致。

但 brief 明确授权范围仅 status 字段（"P2-21 条目翻 resolved（house rule 2 同 commit），resolution 注明：..."），未授权修改 Proposed fix 行。implementer 严格守界。

**严重程度**：Minor — 内部一致性瑕疵；当前阅读体验仍可读懂（Status 是 source of truth），但长期 ledger 会保留过期的"Proposed fix"判断直到下一次清理。如 ledger 后续做历史归档，可一并清理。

---

### Cannot verify from diff（已逐条自行解决）

- ⚠️ **rg 命令的精确版本/路径假设** — 已自跑实测 `rg -n "..." src/ tests/` 三条，全零命中。✓
- ⚠️ **cargo MSVC wrapper 命令格式** — 已自跑 `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...` 三条，全 PASS。✓
- ⚠️ **Chinese 中文 UTF-8 编码正确性** — 已用 python 读 raw bytes 验证新 Status 行所有多字节序列为合法 UTF-8 码点，无 GBK 双重编码。✓
- ⚠️ **ledger P2-21 翻 resolved 的实际状态文本** — 已 Read line 237 比对；与 brief 期望一致。✓

无可遗留的"Cannot verify"项。

---

## 总结论

**SPEC**: PASS — 三处删除精确，rg 零命中，4 文件授权范围未越界，三道门禁（check/test×2）全 PASS，TH-5 词汇保留，serde 属性保留，UTF-8 字节级干净。

**QUALITY**: PASS WITH CONCERNS — 一处 Important（surface.rs 与 ledger.md 整文件 CRLF 回归，违反"无夹带/无关格式化"）+ 两处 Minor（AGENTS.md 范围外 guardrail 残留；ledger "Proposed fix" 行事实过期但属守界）。

**最终状态**: DONE（修复 F1 后即可 commit；F2/F3 不阻塞，作为 ledger 维护观察在终审统一 triage）。

**给后续 fixer 的最小补丁**（如需 pre-commit 修复 F1）：
1. 将 `surface.rs` 与 `tech-debt-ledger.md` 重新以 LF 行尾保存。
2. 不要触动 Status 行、变体删除、BRANCH_EXCLUDED_TAGS 元素 — 这些已正确。
3. 重跑 `git diff --check` 确认无 CRLF 警告。
4. 重跑三道 cargo 门禁确认仍然 PASS（不应受影响）。

**审查者备注**: 本次 review 未触动 implementer 任何 commit 行为；ledger 进度文件追加、commit、PR 创建等下游动作由编排者在 fixer 后另行调度。