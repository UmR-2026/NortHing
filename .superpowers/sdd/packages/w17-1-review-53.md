# W17-1 Review (judge 2/2, reviewer-53) — CI Windows-only matrix + CLI zero-warning + tech-debt ledger

- Commits: `77b69df` + `4bc3fb1`（BASE `1c9ac2f`）
- 需求源：`.superpowers/sdd/w17-1-brief.md`；报告：`.superpowers/sdd/reports/w17-1-report.md`
- 独立取证：全部结论出自 diff、源码与本人实跑（cargo check / node scripts / gh CI 日志 / YAML 解析），未采信 report 自述。

## SPEC: PASS
## QUALITY: PASS
## Findings
- [Minor] ledger 新状态词 `deferred` 不在该文件 Change Protocol 的状态词表内 — `docs/status/tech-debt-ledger.md:251`（新条目 Status 用 `deferred`）vs `:257`（协议词表仅 "active / frozen / resolved"）。brief 改动三明确要求 `deferred`，故属 brief 合规、仅是文件内部轻微不自洽。修复指引：下次触碰该文件的顺手清任务中把 257 行词表扩为 "(active / frozen / deferred / resolved)"（一词改动）；或维持现状，零紧迫性。

## Cannot verify from diff
- 收窄后的 ci.yml 在 GitHub Actions 上实际跑绿（windows-latest leg）——两 commit 尚未 push（`git log origin/main..main` 含 W16 尾部 9 个 commit 均本地），无对应 run 可查。风险评估：低——被删的正是红了两平台的 leg，Windows leg 的编译路径（OpenSSL 前置、toolchain、cache、i18n 生成）零改动，`cargo check --workspace` 本机等价路径已绿。push 后首跑即可闭环。

## 范围外改动
- 无。diff 恰为允许文件集三文件（ci.yml / mod.rs / tech-debt-ledger.md），无越界。

---

## 验收证据（逐条）

### 改动一：ci.yml 矩阵收窄（commit 77b69df）
1. **矩阵收窄**：diff 显示 `os:` 删 `ubuntu-latest`、`macos-15`，仅留 `windows-latest`（现文件 `ci.yml:33-34`）；本人 YAML 解析实测 `matrix.os = ["windows-latest"]`，7 个 job 全部在位。
2. **Linux 步骤整删**：`Install Linux system dependencies (Tauri)` 整块（原 44-74 行）从 `- name:` 到 `tesseract-ocr-eng` 完整移除，无残块。
3. **OpenSSL 步骤完好**：`ci.yml:38-41` 步骤 + `if: runner.os == 'Windows'` 条件原样保留，脚本引用 `./scripts/ci/setup-openssl-windows.ps1` 未动。
4. **注释合规**：`ci.yml:32` `# Windows-only per user decision 2026-09-05; non-Windows builds currently broken (terminal-core E0624), see tech-debt-ledger` —— 拍板日期 + 伤情出处 + ledger 指针三要素齐备。
5. **strategy 残留合法性**：`fail-fast: false`（`ci.yml:30`）配单元素矩阵完全合法（无害冗余）；`Run workspace Rust tests` 的 `if: matrix.os == 'windows-latest'`（`ci.yml:63`）恒真但 brief 明令"只允许指定删除/注释"，保留正确。
6. **其余内容零改动**：diff 仅 3 个 hunk（矩阵+注释、Linux 块删除），`paths-ignore`/`concurrency`/`permissions`（`ci.yml:3-21`）及其余 6 个 job（rust-tests-serial / kernel-api-clean / core-boundaries / rot-budget / repo-hygiene / i18n-contract）逐字未动。
7. **副作用核查**：
   - 分支保护：`gh api repos/UmR-2026/NortHing/branches/main/protection` → 404 "Branch not protected"，无 required status check 引用被删 leg，收窄不破坏任何门禁。
   - 覆盖面损失与拍板一致性：ubuntu/macos 不再编译 workspace，正是用户 2026-09-05「Windows 限定」拍板的直接落实；非 Windows 的脚本型 job（kernel-api-clean 走 cargo tree 仅元数据不编译、core-boundaries / rot-budget / repo-hygiene / i18n-contract 走 node）仍在 ubuntu 运行，基础信号未全失。
   - 挂账防遗忘充分性：P2-23 记录平台级伤情 + 恢复前置条件（"若未来恢复跨平台支持需先修此项"），且 nightly.yml（工作日 cron）仍在非 Windows leg 实际构建（见范围外观察①），债项持续可见而非静默遗忘。

### 改动二：mod.rs 去警告（commit 4bc3fb1）
1. **diff 精确**：`mod.rs:15` 由 `pub use types::{QuestionAction, QuestionData, QuestionOption, QuestionPrompt};` 改为 `pub use types::{QuestionAction, QuestionPrompt};`，仅删两名字，types.rs 本体未动。
2. **引用安全性（独立 rg 复核）**：
   - `northhing-cli` 为 bin-only crate（`src/apps/cli/Cargo.toml` 仅 `[[bin]]`，无 `[lib]`）→ 不存在任何外部 crate 经该重导出的消费可能；
   - 全仓 `QuestionData|QuestionOption` 命中：cli 内仅 `types.rs`（定义）与 `question.rs:3,29`（走 `super::types::` 直接路径，不经重导出）；其余消费方（chat_state_tool_events.rs:11 / chat_state_core.rs:22 / key_popups.rs:12 / state.rs:21）只用 `QuestionPrompt`/`QuestionAction`/`render_question_overlay`（全部保留）；`agent-runtime` 的 `QuestionOption` 是另一 crate 自有类型，无关；
   - 全仓 toml 无任何 crate 依赖 northhing-cli。零消费方，删除安全。
3. **0 warning 目标（本人实跑复现）**：`rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing-cli`（全新编译，2m25s）→ `Checking northhing-cli v0.2.10` 后直接 `Finished`，**cli 自身 0 warning**；16 条 warning 全部归属 `northhing-core (lib)`（与 report 输出逐条一致）。core 源码在 diff 文件集之外且工作树无 tracked 修改 → 16 条为 BASE 既有，非本任务引入。report 用 `--bin` 直查补强判别证据，做法诚实。

### 改动三：tech-debt-ledger 挂账（commit 77b69df）
1. **编号与格式**：P2-23 紧接 P2-22 为下一个可用 ID；四字段（Symptom/Evidence/Proposed fix/Status）与既有条目同构。
2. **内容真实性（外部证据核验）**：本人经 gh 拉 CI run 33964321637 日志，实测 `error[E0624]: method deadline is private` ×2 同时出现于 `Rust Build Check (macos-15)` 与 `Rust Build Check (ubuntu-latest)`（2026-09-05T11:5x）——ledger 症状描述逐字属实。
3. **处置字段**：`deferred` + 用户拍板日期 + 恢复前置条件 + ci.yml/W17-1 关联，brief 三要素齐备（状态词表问题见 Minor）。

### 提交分域与 Global Constraints
1. **两 commit 分域**：77b69df = ci.yml + ledger（ci 域）、4bc3fb1 = mod.rs（fix(cli) 域），commit message 与 brief 规定逐字一致；逐文件 add（无裹挟）。
2. **零新依赖**：diff 无任何 manifest/lockfile 改动。
3. **ci.yml 仅限指定改动**：见证据一之 6。
4. **English-only**：ci.yml 注释、commit message 均 English；ledger 条目中文系 brief 改动三自身模板 + 该文件既有条目语言先例（P2-19~22 均中文），仓库 English-only 规则约束日志而非此文档——判定不违例（已考虑并排除）。
5. **report 一致性抽查**：rot-budget 输出与本机实跑逐 token 一致；repo-hygiene 本机 "3 content files / 3832 filenames" vs report "2/3831"，差值 = implementer 跑后新增的未跟踪 .superpowers 文件，两者均 pass，非实质差异；report 三改动 file:line 均对得上现文件（ci.yml:32-34 精确；"43-74" 为被删块旧位置，可接受）；结尾状态词 DONE。

## 范围外观察（不计 finding，供编排者参考）
1. **nightly.yml / cli-package.yml 仍有非 Windows 构建 leg**：nightly（cron 周一至五）矩阵含 ubuntu-latest/ubuntu-24.04-arm/macos-15/macos-15-intel 并跑 `pnpm run desktop:build:*`（编译 workspace），cli-package（手动触发）同含 ubuntu/macos——在 P2-23 修复前这些 leg 触发即红。属既有状态、在本单允许文件集之外不可修；P2-23 的平台级措辞已覆盖该债。是否收窄/修复属用户决策，建议作为潜在后续任务上报。
2. `scripts/check-github-config.mjs` 本机因缺 `yaml` npm 模块无法运行（模块解析失败，非校验失败；脚本未被本 diff 触碰，也不在 ci.yml 门禁内）——本审查改用直接 YAML 解析完成等效校验。环境问题，顺手 `pnpm install` 可解。
3. 工作树未跟踪残留：`.superpowers/sdd/packages/w16-final-review.md`（W16 终审遗留）、w17-1 brief/report（流程文件，按约定不入 commit）。不影响本 diff，留编排者按取消/失败卫生惯例处置。
