# W17-2 独立审查（judge 2/2 = reviewer-53，双 judge 车道，未读 judge 1 结论）

- 审查对象：`46c9f53`（BASE `56b752f`），5 文件：ci.yml / nightly.yml / cli-package.yml / check-repo-hygiene.mjs / tech-debt-ledger.md
- 需求源：`.superpowers/sdd/w17-2-brief.md`；报告：`.superpowers/sdd/reports/w17-2-report.md`
- 独立重跑：浅克隆 fallback 实证（clone --depth 1 本仓 @46c9f53 → warning + 全仓扫描 + EXIT=1，166 个文件 384 处违规，"约 170"成立；克隆已删）、本仓 hygiene 绿（EXIT=0，无 warning）、rot-budget 绿（数值与 report 逐项一致）、三 workflow YAML python-yaml 解析合法。

## SPEC: PASS
## QUALITY: FAIL

四改动逐条判定：

| Brief 条目 | 判定 | 证据 |
|---|---|---|
| 改动一 ci.yml fetch-depth: 2 + 注释 | PASS | ci.yml:157-160 唯一 hunk（diff 实证 ci.yml 无其它触碰）；注释含 run 33982832690 |
| 改动二 fallback fail-loud warning | PASS | check-repo-hygiene.mjs:52-54；触发条件（两数组皆空）与 :55-61 的 fallback 三元分支逐字等价，仅在真 fallback 时触发——本仓正常跑实证无 warning，浅克隆实证有 warning；英文、位于扫描前；判定逻辑/正则/skip 规则零改动（diff 仅 +3 行） |
| 改动三 矩阵收窄 + 残留引用 + 剩余 leg 自洽 | PASS（引用层面）/ 缺陷（构建层面，见 Important-1） | 两文件 grep 无 macos/ubuntu-leg/darwin/AppImage/deb/rpm/dmg 残留引用；target triple↔缓存 key（nightly-v2-windows-x64 / cli-v1-windows-x64）↔产物名（*-windows-x64）自洽；smoke/stage 的 .exe 处理正确；保留的 4 个 ubuntu job（check-changes / publish-nightly / prepare / upload-release-assets）逐一读步骤核实均为纯编排/聚合，无 cargo，report 理由准确 |
| 改动四 ledger P2-24 | PASS | ledger:253-258；四字段格式与 P2-19..23 一致，ID 顺延，`deferred` 词汇沿 P2-23 先例，内容含 brief 要求的全部四要素（症状/deferred/处置方向/关联 W17-2） |

Global Constraints：零新依赖 ✓（setup-openssl-windows.ps1 为既有本地脚本，无新 action/包）；判定逻辑零改动 ✓；输出原文进 report ✓（warning 字符串与源码逐字节一致）；ci.yml 除改动一零触碰 ✓；commit 恰为 allowlist 5 文件、单 commit、消息逐字合规、report 结尾 DONE ✓。

## Findings

- **[Important] 打包 workflow 的 Windows leg 缺 i18n locale contract 生成前置步骤，fresh checkout 构建必然 E0583** — cli-package.yml:92-120（build job 步骤序列：Checkout→OpenSSL→toolchain→cache→`cargo build -p northhing-cli`，全程无 node/pnpm 步骤、无 `node scripts/generate-i18n-contract.mjs`）；nightly.yml:80-150（package job 同样缺生成步骤）。证据链：① src/apps/cli/Cargo.toml:14 northhing-cli 直接依赖 northhing-core；② src/crates/assembly/core/src/service/i18n/mod.rs:5 无条件 `pub mod generated_locale_contract;`（service 模块链无 feature 门控）；③ .gitignore `**/generated_locale_contract.rs`（任何 checkout 均无此文件）；④ **实证**：CI run 33846866557（2026-09-04）Package (windows-x64) 在 "Build desktop app" 步骤失败，日志逐字为 `error[E0583]: file not found for module 'generated_locale_contract'`——正是同一 runner 镜像、同一模块的失败；⑤ ci.yml:52-57 已写明该前置并自带注释（"generated_locale_contract.rs is gitignored; northhing-core fails E0583 without it"），implementer 从 ci.yml 镜像了 OpenSSL 步骤却漏了这一步。其中 **cli-package 的 windows leg 是本 diff 新建的**（原 4 个非 Windows leg 被替换），交付即不可构建，与 report 宣称的 Windows 自洽适配矛盾；nightly 的 windows leg 为保留腿、缺陷系既有（33846866557 实证，非本 diff 引入），但收窄后成为唯一腿，不修则 nightly 持续全红。修复指引：两文件 build 步骤前各加一步 `name: Generate i18n locale contract / shell: bash / run: node scripts/generate-i18n-contract.mjs`（脚本仅用 node:fs/node:path 零依赖；windows-latest 自带 Node；nightly 已有 setup-node）。两文件均在 allowlist 内，属改动三"剩余 leg 自洽"的补全。

- **[Minor] cli-package.yml 的 homebrew-tap 通知在 Windows-only 后语义死亡** — cli-package.yml:232-253：release 发布后 dispatch UmR-2026/homebrew-tap 更新 formula，但 Homebrew 只服务 macOS/Linux，而 release 现仅含 `northhing-cli-*-x86_64-pc-windows-msvc.tar.gz`，tap 已无可安装产物可指。步骤有 token 守卫、不会硬失败，但属收窄后的悬挂语义引用。修复指引：或删该步骤、或在 ledger 挂账待用户拍板（涉及发布分发语义，超出本 brief 授权，建议挂账）。

- **[Minor] cli-package.yml:165 陈旧平台注释** — "portable: shasum on macOS, sha256sum on Linux"：macOS/Linux leg 已删，注释成古迹（代码本身无害，git-bash 自带 sha256sum，fallback 链仍正确）。顺手改为仅提 Windows 实况即可。

- **[Minor] 恢复后的 per-commit 口径仍有三个既有盲区（判定逻辑冻结，非本 diff 可修，建议挂账）** — ① 多 commit push：fetch-depth: 2 下 HEAD^1 存在，但 `diff HEAD^1..HEAD` 只覆盖 tip commit，同 push 中间 commit 逃逸内容扫描（脚本既有语义"扫本次 commit"）；② 纯删除 commit：`--diff-filter=ACMRT` 滤掉 D，committedChangedFiles 为空 → 走全仓 fallback → 因 P2-24 存量必红（fetch-depth: 2 救不了这条路径；新 warning 能解释原因，算 fail-loud 兜底）；③ 根 commit push（孤儿分支强推等理论场景）HEAD^1 不存在 → 同样 fallback。PR 场景无洞：checkout 默认 ref 为 refs/pull/N/merge，--depth=2 取回 merge commit 及双亲，HEAD^1=base 分支 tip，diff 即全部 PR 变更。修复指引：在 P2-24 追加一句或另立 ledger 条目记录"纯删除/多 commit push 触发口径盲区"，处置与 P2-24 脱敏一并拍板；如需改 fallback 条件须另立 brief（本 brief 判定逻辑零改动约束）。

## Cannot verify from diff
- 三 workflow 推送后的真实运行结果 — 46c9f53 尚未推送（本地 main ahead 1，origin/main 在 56b752f），repo-hygiene fetch-depth: 2 与两打包 workflow 的 post-change 行为无法在本地执行验证；本判决基于静态分析 + run 33846866557 既有实证（Important-1 即其推论），建议推送后盯首个 CI run 收口。
- homebrew-tap 外部仓（UmR-2026/homebrew-tap）对 dispatch 的实际反应 — 外部仓库不可见。

## 范围外改动
- 无（diff 恰为 allowlist 5 文件；brief/report 为不入 commit 的未跟踪文件，正常）。

## 结论

SPEC PASS / QUALITY FAIL。Important-1 一项进 fixer 循环（两 workflow 各加一步生成脚本，3 行 × 2），Minor 记 ledger 指向终审 triage。


## 修复轮重审（9aa5762）

- 对象：`9aa5762`（`fix(ci): add i18n contract generation to windows packaging legs (W17-2)`）；diff `46c9f53..9aa5762` 恰 2 文件 +14/-0 纯插入，单 commit。
- 复核手段：diff 逐行核对、两 workflow 全文读取、与 ci.yml:52-57 逐字节比对、YAML 亲跑解析、gh API 取 run 33982832690 step 级结论、工作树状态核查。

### 复核要点逐条

1. **注入与语义等价：PASS** — cli-package.yml:114-119（build job 步序 Checkout:93→OpenSSL:98→toolchain:103→cache:108→**i18n:114**→`cargo build --release --target x86_64-pc-windows-msvc -p northhing-cli`:121-127）；nightly.yml:137-142（package job 步序 Checkout:81→OpenSSL:83→pnpm:88→node:91→toolchain:97→cache:102→pnpm install:108→patch version:111→**i18n:137**→`Build desktop app`=`pnpm run installer:build`:144-145）。两处步骤体（name / `shell: bash` / `node scripts/generate-i18n-contract.mjs` / 产物断言 `test ! -d northhing-Installer && test -f northing-installer/src/i18n/generatedLocaleContract.ts` / E0583 注释）与 ci.yml:52-57 逐字节一致（diff 实证），均在 checkout 后、首个 cargo 调用前。生成脚本零依赖（generate-i18n-contract.mjs 仅 import node:fs / node:path），无需 pnpm install，断言可达。
2. **shell: bash × windows runner 兼容性：PASS** — ci.yml `rust-build-check` = `windows-latest`（ci.yml:33-34 matrix 唯一 os）+ `shell: bash` + job 内无 setup-node（预装 Node），与 cli-package 注入位形态完全同组合（同样无 setup-node）；nightly 更有 setup-node@v4（node 20）前置。实证：gh API run 33982832690（commit d133f40）job `Rust Build Check (windows-latest)` 的 step `Generate i18n locale contract` conclusion=**success**；该步骤自 867ae2d（W14-1c-3e）引入后 5 个连续 run（33872662968 / 33906435041 / 33962324438 / 33964321637 / 33982832690）该 job 全绿——步骤无 if 门控，job 绿 ⟹ 步骤成功，且 `cargo check --workspace` 未 E0583 ⟹ Rust contract 确实生成。nightly 侧旁证：run 33846866557 的 package(windows) 全部前置 bash 步骤（含 patch version 的 jq/sed/rm）跑通至 Build 才 E0583，注入位环境无未实证变量。
3. **除注入外零其它改动：PASS** — `git diff 46c9f53..9aa5762` 仅上述 2 hunk，无删除行、无第三文件；工作树仅 4 个未跟踪 sdd 文档（brief/report/packages×2，不入 commit，正常）。
4. **report 更新节与验证输出：PASS** — §2 修复说明（issue / 与 ci.yml:52-57 parity / 注入位置）、§5 含 9aa5762 的双 commit 记录、Verification 5 含 fix commit task-gate 输出原文（base 46c9f53 tip 9aa5762，`Attempt verification passed: all modified files are within allowlist.`）。Verification 4 的 YAML 校验时点无法从 report 断定是否在 fix 后重跑——本人在 HEAD=9aa5762 亲跑 python-yaml 解析 nightly.yml / cli-package.yml 均 OK，缺口已补。（report 行号口径 137-143/114-120 vs 实际 137-142/114-119，尾行差 1，非实质。）

### 判定

- 原 Important-1（Windows 打包 leg 缺 i18n 前置 → fresh checkout 必 E0583）：**已修复**。风险链闭环：.gitignore:41 证实 contract 文件任何 checkout 均缺失 → 注入步骤在 cargo 前生成（逐字节镜像一个已在同 runner/shell/Node 环境连续 5 run 绿灯的步骤）。
- 本轮新发现：0。原 3 Minor（homebrew-tap 悬挂语义 / cli-package.yml:172 陈旧平台注释 / hygiene 口径盲区挂账）不变，仍指向终审 triage。
- 残留 Cannot verify（缩窄）：9aa5762 未推送（main ahead 2，origin/main=56b752f），两 packaging workflow 修复后首跑仍待推送后观察——注入步骤与已 5-run 绿灯的 ci.yml 步骤逐字节相同且环境同构，静态+同构证据已足，不阻塞结论。

## SPEC: PASS
## QUALITY: PASS

**结论：APPROVE（发现数 0）**
