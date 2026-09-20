# W23-4 任务实施报告

## 改动摘要

- 任务目标：执行 W23-4（用户 2026-09-09 决策包⑧⑨项执行，纯台账处置）。
- BASE：`97d227a`
- 实施 Commit：`fd3fd7e` (`docs: update P2-14 and P2-17 status in tech-debt ledger (W23-4)`)
- 改动文件：仅 `docs/status/tech-debt-ledger.md` 1 个文件，通过行内替换修改了 2 个条目的 Status 行：
  1. P2-14：标记为 `resolved`（决策关单：dream-sweep 即设计答案，无代码变动）。
  2. P2-17：标记为 `frozen`（按设计暂缓：等待第三调用方出现再提取共享同步工具模块）。
- 编码卫生：严格采用行内替换，未发生整文件重写或双重编码污染，中文内容保持原样无乱码。

## 验证

### 1. 仓库卫生检查 `node scripts/check-repo-hygiene.mjs`

命令：`node scripts/check-repo-hygiene.mjs`
Exit Code: 0
输出：
```
Repository hygiene check passed (1 content files scanned, 3909 filenames checked).
```

### 2. Diff 统计 `git diff --stat 97d227a HEAD`

命令：`git diff --stat 97d227a HEAD`
Exit Code: 0
输出：
```
 docs/status/tech-debt-ledger.md | 4 ++--
 1 file changed, 2 insertions(+), 2 deletions(-)
```

### 3. Diff 全文 `git diff 97d227a HEAD docs/status/tech-debt-ledger.md`

命令：`git diff 97d227a HEAD docs/status/tech-debt-ledger.md`
Exit Code: 0
输出：
```diff
diff --git a/docs/status/tech-debt-ledger.md b/docs/status/tech-debt-ledger.md
index b02e0fd..1af870b 100644
--- a/docs/status/tech-debt-ledger.md
+++ b/docs/status/tech-debt-ledger.md
@@ -184,7 +184,7 @@
 - **Symptom**: facts.jsonl dedup uses exact text match — cannot absorb whitespace/wording variants, so the store bloats with near-duplicates. confidence is always Med and scope always Workspace; the High/Low/Global production paths are not implemented.
 - **Evidence**: External review 2026-07-23 §四.4 / §四.8; C3 facts distillation code.
 - **Proposed fix**: Normalize before dedup (or similarity-based dedup); implement confidence/scope derivation paths or remove the unused enum variants.
-- **Status**: active (low priority)
+- **Status**: resolved (2026-09-20, W23-4: user ruling 2026-09-09 — dream-sweep is the design answer; decision-only closure, no code change)
 
 ### P2-15: P1-C3 merged to main while the desktop crate did not compile (process defect)
 
@@ -206,7 +206,7 @@
 - **Symptom**: `client_factory.rs` now owns a private `init_once_with` helper implementing the double-checked-locking init skeleton, while `service/config/global.rs` `GlobalConfigManager::initialize` still hand-rolls the same pattern with its own `INIT_MUTEX`.
 - **Evidence**: Task B4 review Minor-3 + Wave1 final review §5 (2026-08-06), commit `50b0f44`.
 - **Proposed fix**: if a third caller appears, lift the helper into a shared sync utility module and migrate both call sites; not worth it at two call sites.
-- **Status**: active (low priority)
+- **Status**: frozen (2026-09-20, W23-4: user ruling 2026-09-09 — deferred by design; lift to shared sync utility only when a third caller appears)
 
 ### P2-18: `LspManager::uninstall_plugin` has no production caller
```

### 4. 两个条目改动前后 Status 行逐字对照

- **P2-14**:
  - 改动前：`- **Status**: active (low priority)`
  - 改动后：`- **Status**: resolved (2026-09-20, W23-4: user ruling 2026-09-09 — dream-sweep is the design answer; decision-only closure, no code change)`
- **P2-17**:
  - 改动前：`- **Status**: active (low priority)`
  - 改动后：`- **Status**: frozen (2026-09-20, W23-4: user ruling 2026-09-09 — deferred by design; lift to shared sync utility only when a third caller appears)`

## 状态

DONE
