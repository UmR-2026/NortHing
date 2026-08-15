# Review brief — locale 154 腐蚀修复（双判决：spec 合规 + 代码质量）

> 坐标：worktree `E:\agent-project\northing\.worktrees\consult-room-build`，分支 `feat/consult-room-slint`，BASE=`2261d2a`。
> 改动**未 commit**（brief 要求）：tracked 5 件 M + untracked 两目录内容变更。
> diff 文件：`.superpowers/sdd/consult-room/task-locale-repair-review.diff`（31.7KB，经 `git add -N` 收进 untracked 内容）。
> 实现 brief（需求唯一来源）：`.superpowers/sdd/consult-room/task-locale-repair-brief.md` —— 先读，§2 预检事实/§3 清单/§4 红线。
> 实现报告：`.superpowers/sdd/consult-room/task-locale-repair-report.md`。

## 1. untracked 文件改动前状态（编排者预检字节证据，diff 只能显示改后）

- `northhing-Installer/src/i18n/locales/{en,zh,zh-TW}.json`：改前均为 10 字节 `FF FE 7B 00 7D 00 0D 00 0A 00`（UTF-16 LE 空对象 stub）。
- `src/web-ui/src/infrastructure/i18n/presets/namespaceRegistry.ts`：改前 106 字节 UTF-16 LE（`FF FE` 开头），内容同现语义。
- `northhing-Installer/src/i18n/generatedLocaleContract.ts`：改前 nativeName/continueLabel 为 GBK mojibake；现为生成器重跑产物（契约 `src/shared/i18n/contract/locales.json` 干净，预检已验 简体中文/繁體中文/繼續 在位）。
- `src/crates/assembly/core/src/service/i18n/generated_locale_contract.rs`（tracked）：重跑生成器后**应零 diff**——请核 git status 佐证（当前该路径不在 M 列表，已间接证明）。

## 2. Constraints（逐字核，违反即 FAIL）

逐字块见实现 brief §4（红线/白名单/编码纪律/零语义猜测/不 commit）。补充审查专用：
- 快照 `1b147c3` 是 pre-dioxus 内容的**唯一还原源**（zh-TW 必须用其自身快照，禁简体机转）。
- 尾部 63 个 dioxus-room-* 键**逐字节不动**（与 BASE 比较应为零 diff——dioxus 键段在 diff 中不应出现 +/-，或出现即 FAIL 候选）。
- en-US.ftl 不动（diff 中不应出现该文件）。
- `src/shared/i18n/contract/locales.json`、`scripts/i18n-audit.mjs`、`scripts/generate-i18n-contract.mjs` 不动（diff 中不应出现）。

## 3. 重点审查面

1. **还原保真**：抽查 zh-CN/zh-TW 还原值 vs `git cat-file -p 1b147c3:src/crates/assembly/core/locales/<lang>.ftl`（你可自行比对）；品牌映射 BitFun→northhing 是否恰好命中应改处；23 个复原键值与快照逐字一致。
2. **零语义猜测**：凡摧毁点（原 PUA/`?`）处的还原值必须能溯源快照；发现无源新译即 Important+。
3. **治理配置改动**（本任务最大风险面）：
   - `i18n-dynamic-key-allowlist.json` 删除的 11 条条目：逐条核 owner path 确实在仓库不存在（可 Test-Path / glob 验证）；**凡 owner 实存而被删 = Critical**。
   - `i18n-governance-baseline.json` sharedTermDuplicates 调成 core 15 / web-ui 0 / 总 15：是否只降不升；core 15 与修复后实测自洽（报告应附实测）。
   - `i18n-hardcoded-baseline.json` mobile-web CJK 预算 0→2：**这是上调**，brief E 授权「有注解的预算调整」；核注解是否入报告、2 是否与实测一致。
4. **白名单边界事实**（请判决）：brief 白名单列了 `scripts/i18n-governance-baseline.json` 与 `scripts/i18n-dynamic-key-allowlist.json` 两件；实现另改了 `scripts/i18n-hardcoded-baseline.json`（同类审计配置，对应 brief §3-D 的 mobile-web CJK 项「baseline 预算调整」表述）。判：合理解释 / 越界。
5. **编码终态**：zh ftl UTF-8 无 BOM、LF、0 PUA、0 U+FFFD；installer JSON 合法 UTF-8 无 BOM 且键 parity 对称（en=基线，zh/zh-TW 不多不少）；namespaceRegistry.ts UTF-8。
6. **审计 exit 0 的可信度**：report 附了输出原文（0 error / 1 warning）。你可静态推定各错误类是否都被覆盖；如需复跑 `pnpm run i18n:audit`（~5s）可自行决定。
7. **dioxus 运行时回归面**：ftl 改动后 `cargo test -p northhing ui_dioxus` 报告声称过（输出在 report）；i18n.rs 加载路径对无 BOM/LF 的兼容性可从源码静态确认。

## 4. 报告

写 `.superpowers/sdd/consult-room/task-locale-repair-review.md`：
Spec verdict / Quality verdict（各 PASS 或 FAIL）+ findings（Critical/Important/Minor/FYI 附 file:line）+ ⚠️ cannot-verify-from-diff 单列 + 结论。最终消息回复 verdicts + findings 计数。
