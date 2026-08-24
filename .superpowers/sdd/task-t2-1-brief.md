# Task T2-1 Brief: CI 补齐（构建面 + 测试面）

## Source
- backend-roadmap.md T2-1；full-review-2026-08-16.md R-11（CI 不构建产品 = 最大静默腐化源）；家规 6（merge 前 desktop check 必须过，P2-15 结转）。
- 侦察已由编排者完成（2026-08-17），下文证据直接可用。

## 现状证据（已核实）
- `.github/workflows/ci.yml` line 98: `cargo check --workspace --exclude northhing-cli --exclude northhing` —— desktop 与 CLI 在 CI 构建面之外。
- line 101: `cargo test --locked -p northhing-core` —— 测试只跑 1/31 crate。
- kernel-api dep guard **已在 CI**（kernel-api-clean job, lines 103-128）——roadmap "尚未入 CI" 是过期描述，本单顺手修正该处文档。
- 本地 MSVC 实测：`cargo check -p northhing` pass（2026-08-17 T0-3 report）；`cargo check -p northhing-cli` pass（编排者，2m30s，1 个 unused import warning）。
- ⚠️ 本机默认 cargo 是 GNU 工具链（会 dlltool 报错）；验证一律用 `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`。

## Required changes
1. **ci.yml line 98**：去掉两个 exclude，改为 `cargo check --workspace`（三 OS matrix 不变）。这使 desktop/CLI 编译成为 CI 硬门（= 家规 6 的技术面落地；GitHub 分支保护设置不在本单范围）。
2. **ci.yml line 101**：测试面扩展为 `cargo test --locked --workspace`，**仅在 ubuntu-latest 单 OS 跑**（不与 check 的 matrix 混跑；成本控制）。从 matrix job 拆出独立 job 或改现有步骤均可，选改动最小的结构。
3. **文档顺手修**：`docs/architecture/backend-roadmap.md` T2-1 行中 "cargo tree 守卫 job（北极星 §4 既有要求）" 相关表述改为"已在 CI（kernel-api-clean job）"；surfaces.md 若涉及 CI 描述过期也一并修。

## Constraints
- 不新增、不删除、不跳过任何既有测试；不为让 CI 变绿而给测试加 #[ignore]——发现 OS 相关失败如实上报（NEEDS_CONTEXT 或 DONE_WITH_CONCERNS 列出清单）。
- 不动 `.github/workflows/` 里其它文件；不动 GitHub 仓库设置。
- i18n-contract 预存失败（24 个）**不在本单范围**（i18n engineering 冻结中，CI 无 i18n job）；不要试图在本单修它。

## Verification（贴原始输出）
1. `& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace`（全 workspace 无 exclude，必须 pass——这等价于 CI 新门槛的本地预演）
2. `node scripts/check-core-boundaries.mjs`（确认未被波及）
3. ci.yml 语法：`node -e "const y=require('yaml');..."` 不可假设有 yaml 包——改用 `git diff --check` + 人工复述改动段结构在报告中。
4. 不需要本地跑全量 cargo test --workspace（CI 是验证场）；但要报告 workspace 里哪些 crate 有测试目录/#[test]，评估 ubuntu 上可能的 OS 敏感套件（PTY/keyring/enigo 类）并列出风险清单。

## Report
写 `.superpowers/sdd/task-t2-1-report.md`，首行 DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED。含 diff 摘要、验证原始输出、OS 敏感测试风险清单。
