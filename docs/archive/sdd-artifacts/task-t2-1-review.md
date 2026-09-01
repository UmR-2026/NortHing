# T2-1 Review: CI 补齐（构建面 + 测试面）

## Strengths

- **最小改动路径达成目标**：3 个文件、合计 +5/-4 行（ci.yml +3/-2、roadmap +2/-2、ledger +1/-1），完全符合 brief「选改动最小结构」的指示。
- **diff 结构纯净**：`git diff --check` 输出 0 whitespace errors / 0 conflict markers（CRLF/LF 警告为 Windows 工作区常态，非本单引入）。
- **实施报告 §3.1 OS 敏感测试风险清单**覆盖 Keyring/PTY/DISPLAY/路径/端口五类，标注风险等级与既有测试结构（MockKeyring/BufferFocus/临时 port），为后续 T 任务提供了一份可复用的 Ubuntu CI 风险图。
- **报告可复核性强**：cargo check 末尾 `Finished dev profile [unoptimized + debuginfo] target(s) in 2.50s` 与本次独立复跑命令的输出**逐字段一致**（含 warning 数 19/1/5、未出现 error）。
- **P2-15 flip 内容准确**：`code defect resolved in b0bfe43` + `process gate recorded in housekeeping rule 6` + `T2-1: cargo check --workspace in CI includes northhing and northhing-cli`，三要素（缺陷修复、流程规则、CI 落地）全部写入。

## Issues

### Critical

无。

### Important

无。

### Minor

- **M-1 — 工作区尚未提交**：当前 3 文件改动均在 working tree，未 commit。House rule 2「解债→同 commit 翻 ledger 状态」要求 `ci.yml` + `tech-debt-ledger.md` 同时入 commit。本次审查时 `git log` 顶端仍为 `3a6695f docs(status): track full-review 2026-08-16 evidence baseline`（2026-08-17 22:11 commit），T2-1 未落 commit。**修复建议**：编排者在 implementer commit 时显式要求将 3 文件一并提交，不得拆分（拆分即违规 house rule 2）。**这是编排者层级核查项，不是 implementer 改 diff 的修复项。**
- **M-2 — ledger 条目署名粒度**：原条目「code defect resolved (`b0bfe43`)；process gate recorded 2026-08-06 (house rule 6). CI enforcement of the desktop check is still open.」三段式信息，新版合并为单句，丢失了「CI enforcement」作为一个独立环节的语法标记。**修复建议（可选）**：可在新 status 行保留「CI enforcement of the desktop check resolved (T2-1, 2026-08-17)」为独立子句，使未来回溯 ledger 时一眼可见三个独立环节各自的解决归属。**优先级**：低；当前合并写法语义正确，不影响关闭判定。
- **M-3 — 报告 §1.2 line 编号偏差**：报告称「Line 42」与「Line 166」，经 read 实测分别在第 42 行（含中文双宽字符前导空格）与第 166 行。中文行宽与显示列存在差异，对 review 核对无影响但与西方惯例 line=column 1 不同。**修复建议**：可选改为「after line 39/163」或保留行号不修。
- **M-4 — 报告 §3.1 测试盘点表全量未独立复算**：本次 spot-check 3 个 crate（`desktop`=101、`harness`=5、`agent-runtime`=261）全部命中报告数值。剩余 27 个 crate 未逐项复核，但 implementer 用的 grep 模式 `#[test]/#[tokio::test]/#[async_test]` 与本审查所用正则相同，且 3 个抽样覆盖大/中/小三种规模。**无需修**——brief §特别核对点 §3 已明示「抽查 2-3 个 crate 的 #[test] 计数是否合理即可，不必全量复核」，已满足。

## Recommendations

- **commit message 应明示三条纪律同时满足**：house rule 2（同 commit 解债）、brief Required change 3（roadmap 文档同步）、P2-15 关闭。例：`ci: close P2-15 process gate (cargo check --workspace + cargo test ubuntu-only) — T2-1`。
- **CI 合并入 main 后的观察窗**：建议在 T2-1 commit 后监控一次主分支 push CI 实际通过情况（特别是 windows-latest / macos-15 的 cargo check），确认 MSVC/LLVM 工具链与现有 GNU 默认值的差异不会引入新失败。若 windows-latest 失败，本次改动仍属「P2-15 关债」成功（cargo check 已入 CI 红门），但需新开 P2-x 跟踪 OS-specific 失败。
- **i18n-contract 24 个 pre-existing 失败**：progress.md:281 提及「T2-1 CI 扩面时需先处理」。brief Constraint 3 已明确不在本单范围。**本次不修**，但建议在 T2-1 ledger 行的末尾或 handoff 中明示「i18n 24-fail 已记录，留 T2-3」以承接既往观察，避免后续 round 又一次撞上同样发现。

## Cannot verify from diff

- **CV-1（唯一）**：house rule 2 「同 commit」要求**commit 时**才能验证，不能从 diff 验证。现状：3 文件均在 working tree，commit 尚未发生。**这不是 implementer 的修复项**——是编排者下指令 commit 时的核查项。本次 review 因此保留为 open 项，但**不阻塞 review 通过**（diff 本身合规）。

## Assessment

**Strengths** 全部对应 brief 的「改动最小」与「职责分明」；**Issues** 仅 4 条 Minor（M-1 是 commit 阶段核查项，M-2/M-3 是 cosmetic，M-4 是已被 brief 授权放过）；**Cannot verify from diff** 仅 1 项（commit strategy），diff 范围内的所有可验证证据均独立通过：cargo check 实跑 pass、ci.yml YAML 结构正确、文档更新与代码一致、house rule 2 三文件齐备未拆分。

- **spec-compliance: PASS**
  - Required change 1（ci.yml L98 去 exclude → `cargo check --workspace`）：PASS（diff 命中，独立 `cargo check --workspace` PASS）
  - Required change 2（ci.yml L100-102 测试扩面 + `if: matrix.os == 'ubuntu-latest'`）：PASS（diff 命中，YAML 条件表达式语法合法、与 matrix 中 `ubuntu-latest` 字符串精确匹配）
  - Required change 3（roadmap 两处过期描述更新 + surfaces.md 按需）：PASS（roadmap 两处命中；surfaces.md grep 确认无 CI/cargo tree/kernel-api 字面量过期描述）
  - Constraint 1（不新增/删除/忽略既有测试）：PASS（diff 仅 3 文件，无 `.rs` 测试文件改动）
  - Constraint 2（不动 `.github/workflows/` 其它文件）：PASS（`git diff --name-only -- .github/workflows/` 仅输出 `ci.yml`）
  - Constraint 3（i18n-contract 24 失败不在本单）：PASS（diff 无 i18n 相关文件；progress.md:281 的观察已由 brief 显式豁免）

- **code-quality: PASS**
  - YAML 缩进（6/8 空格）正确，`- name` / `if:` / `run:` 三键位于 step 级合法位置
  - `if: matrix.os == 'ubuntu-latest'` 是 GitHub Actions 合法的 matrix 条件语法（字符串精确匹配，无类型/转义风险），与 ci.yml L33 的 `os:` 列表项字符串精确匹配
  - 文档更新与 ci.yml 实际改动一致：roadmap L42 与 L166 的「已在 CI，kernel-api-clean job」表述，对应 ci.yml L105-129 已存在的 `kernel-api-clean` job（diff 未触及该 job，确认存在）
  - ledger L194 三要素（缺陷修复 + 流程规则 + CI 落地）写入完整，P2-15 关闭理由充分

- **ready-to-merge: With fixes（minor, commit-time）**
  - Diff 范围（3 文件）ready-to-merge
  - 唯一未闭合项（M-1 commit strategy）属于编排者下指令 implementer commit 时核查，不阻塞 review 通过
  - 建议 commit 时把 3 文件一并提交、commit message 明示 P2-15 关闭；commit 后观察主分支 push CI 三 OS 矩阵实际通过情况