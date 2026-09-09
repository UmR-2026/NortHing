# W20-2 Report — P2-24 清账：hygiene 存量（archive 豁免 + 全量脱敏）

## 改动摘要

本任务严格按 `w20-2-brief.md` 要求及用户 2026-09-08 拍板方案 (a) 完成 P2-24 清账：

1. **archive 豁免**：在 `scripts/check-repo-hygiene.mjs` 增加 `localPathExemptPaths = [/^docs\/archive\//]`，并入 `scanLocalPaths = !isTestFile && !isLocalPathExempt`，保持 `docs/archive/` 历史冻结。豁免逻辑严格 ≤5 行，token 与私钥扫描对 archive 保持生效。
2. **75 文件全脱敏**：严格按附录 A 清单对 75 个文件执行 ASCII 级安全替换：
   - 74 个文件的 146 处 local-path 统一替换为 `<LOCAL_PATH>`；
   - 第 75 个文件 `.superpowers/sdd/reviews/2026-08-23-staged/brief.md` 第 21 行 token 替换为 `<TOKEN>`；
   - 替换直接作用于完整文本串，不使用行重组，完整保持原有换行符（CRLF/LF），净差为 0，全批变更行恰好 147 行。
3. **技术债务台账销号**：`docs/status/tech-debt-ledger.md` P2-24 翻转为 resolved，注明用户 2026-09-08 拍板混合方案。
4. **提交点名与范围**：commit `9714d23` 逐文件点名 `git add` 77 个文件（75 脱敏 + 脚本 + ledger），禁裸 `-A`。

### 分类账

| 类别 | 文件数 | 违规/处理行数 | 说明 |
|---|---|---|---|
| archive 豁免 | 88 文件 | 238 行 local-path | 规则豁免 local-path 扫描，保持历史冻结 |
| 存量脱敏 | 75 文件 | 147 行（146 local-path + 1 token） | 严格按决策表替换为 `<LOCAL_PATH>` 与 `<TOKEN>` |
| 永久残留 | 3 文件 | 6 行 token-like secret | 方案 (a) 终态：archive 讨论文本假串，不破历史冻结 |
| 脚本与台账 | 2 文件 | 6 行 diff（脚本 +4-1，台账 +1-1） | 闸脚本豁免逻辑与 P2-24 销号 |

---

## 验证

### 1. BASE 红态（exit 1）

在 BASE commit（`0c9acea`）浅克隆（`--depth 1`）环境下触发 fallback 全量扫描：

- 命令：`node scripts/check-repo-hygiene.mjs`
- Exit Code: 1
- 输出首两行：
  ```
  WARNING: full-repo scan fallback active — HEAD^1 unavailable or no local changes; scan scope is ALL tracked files
  Repository hygiene check failed:
  ```
- 违规分类统计原文提取：
  - 总违规条目: 391
  - local-path: 384（archive 238 + non-archive 74 文件 146 行）
  - token-like secret: 7（archive 6 + non-archive 1 行：`.superpowers/sdd/reviews/2026-08-23-staged/brief.md:21`）
  - private key: 0

### 2. 修复后浅克隆终态（方案 a 口径，exit 1 且恰好余 6 条 archive token 红）

在 TIP commit（`9714d23`）浅克隆（`--depth 1`）环境下运行，输出重定向到克隆外：

- 命令：`git clone --depth 1 file:///<REPO_ROOT> <TEMP_CLONE>` 并在克隆内运行 `node scripts/check-repo-hygiene.mjs`
- 克隆内 `git ls-files` 计数：`3876`（证明全量扫描范围 ≥ 3000）
- Exit Code: 1
- 原文输出：
  ```
  WARNING: full-repo scan fallback active — HEAD^1 unavailable or no local changes; scan scope is ALL tracked files
  Repository hygiene check failed:
  - docs/archive/handoffs/2026-08-22-final-review-fixes.md:36 contains a token-like secret.
  - docs/archive/sdd-artifacts/task-t29b3-report.md:57 contains a token-like secret.
  - docs/archive/sdd-artifacts/task-t29b3-report.md:61 contains a token-like secret.
  - docs/archive/sdd-artifacts/task-t29b3-review.md:42 contains a token-like secret.
  - docs/archive/sdd-artifacts/task-t29b3-review.md:43 contains a token-like secret.
  - docs/archive/sdd-artifacts/task-t29b3-review.md:74 contains a token-like secret.
  ```
- 断言：包含 `WARNING: full-repo scan fallback active`；恰好 6 条违规，全部属于 `docs/archive/` 下 3 个文件；local-path 违规数严格为 0。

### 3. 豁免范围探针（断言式双向）

在修复后浅克隆内测试 archive 豁免的精确边界（阳性 token 扫描有效、阴性 local-path 豁免有效）：

- 基线违规数：6 条（fallback 全量口径）
- ① 阳性探针：向 `docs/archive/HANDOFF.md` 追加一行 token 形态串（`sk-ant-api03-probe...`）：
  - 实测口径与机制一致：未 commit 时工作区变更使扫描按规则塌缩为单文件，报单条精确违规 `- docs/archive/HANDOFF.md:875 contains a token-like secret.`；commit 进浅克隆（保持 depth:1 无 HEAD^1 fallback 口径）后复现总违规数由 6 增至 7 条（证明 archive 下 token 扫描未被豁免）
- ② 阴性对照：向 `docs/archive/HANDOFF.md` 追加一行 Windows 本地绝对路径形态串：
  - 单文件塌缩口径下退出码为 0（0 违规），commit 进浅克隆 fallback 口径下总数维持 7 条，local-path 违规数量严格为 0（证明 archive 下 local-path 豁免生效）
- 探针测试完成，临时克隆已丢弃。

### 4. 主仓日常增量扫描（exit 0，绿）

在主仓工作区运行增量扫描：

- 命令：`node scripts/check-repo-hygiene.mjs`
- Exit Code: 0
- 原文输出：
  ```
  Repository hygiene check passed (77 content files scanned, 3876 filenames checked).
  ```

### 5. `git diff --stat` 复核（净差 0，全批变更行 ≤ 147）

对 commit `9714d23`（`HEAD~1..HEAD`）进行统计断言：

- 75 个脱敏文件在 diff 中全部匹配（75 files matched）；
- 每一个脱敏文件 `+` 与 `-` 严格相等，净差严格为 0；
- 75 个脱敏文件总新增 147 行，总删除 147 行，全批变更行恰好 147 行（≤ 147）；
- 全 commit（含 scripts 4+/1-，ledger 1+/1-）共 77 文件变更，152 insertions(+), 149 deletions(-)。

### 6. 脱敏抽样 diff 段与 `<REPO_ROOT>` 零命中证明

#### 抽样 diff 段 1：`.agents/skills/northhing-onboarding/SKILL.md`（Windows 本地路径）
```diff
diff --git a/.agents/skills/northhing-onboarding/SKILL.md b/.agents/skills/northhing-onboarding/SKILL.md
index dce69a9..c06d4b9 100644
--- a/.agents/skills/northhing-onboarding/SKILL.md
+++ b/.agents/skills/northhing-onboarding/SKILL.md
@@ -114,5 +114,5 @@ py build.py && py embed.py
 
 ## 关联外部文件
 
-- Kimi K3 全量 review: `<REPO_ROOT>/WorkBuddy/2026-07-17-02-25-47/northing-deep-review.md`
+- Kimi K3 全量 review: `<LOCAL_PATH>`
 - QClaw code review: `.handoffs/review-commit-997e14e_20260717.md`
```

#### 抽样 diff 段 2：`.superpowers/sdd/reviews/2026-08-23-staged/brief.md`（第 75 文件 token 命中）
```diff
diff --git a/.superpowers/sdd/reviews/2026-08-23-staged/brief.md b/.superpowers/sdd/reviews/2026-08-23-staged/brief.md
index 98ce082..ed3c458 100644
--- a/.superpowers/sdd/reviews/2026-08-23-staged/brief.md
+++ b/.superpowers/sdd/reviews/2026-08-23-staged/brief.md
@@ -18,7 +18,7 @@
 | F3 | `SkillWatchService::sync_watched_paths` 加 `sync_lock` 互斥防双 watcher，附并发回归测试 | `core/service/skill_watch.rs` + `_tests.rs` |
 | F4 | CLI 方案 C 对等：`keyring_keys` 模块在 config init 后、factory init 前把 keyring keys 推入 core 内存；模型 add/edit 表单存 key、编辑留空继承 keyring key；keyring 服务名常量下沉 core `infrastructure/keyring.rs`（desktop 改引，单一事实源） | `cli/src/keyring_keys.rs`、`cli/main.rs`、`cli/ui/startup/selectors.rs`、`cli/Cargo.toml`、`core/infrastructure/keyring.rs`、`core/infrastructure/mod.rs`、`desktop/settings/keyring.rs` |
 | F5 | 删死契约 `update_global_config` + `GlobalConfigPatchDto`（零调用方） | `contracts/kernel-api/settings.rs`、`lib.rs`、`core/kernel_facade/settings.rs` |
-| 凭据清理 | 测试 fixture 假钥匙不再硬编码：`"test-key"` ×20 → `fixture_api_key()`（env `NORTHHING_TEST_API_KEY` 注入、默认空；值不得进断言）；`responses.rs:136` 同法；`mgr_load_tests.rs` 的 `<TOKEN>-…` 改运行时构建变量（断言同源引用，scrub 语义不变） | ai-adapters tests、responses.rs、mgr_load_tests.rs |
+| 凭据清理 | 测试 fixture 假钥匙不再硬编码：`"test-key"` ×20 → `fixture_api_key()`（env `NORTHHING_TEST_API_KEY` 注入、默认空；值不得进断言）；`responses.rs:136` 同法；`mgr_load_tests.rs` 的 `<TOKEN>-…` 改运行时构建变量（断言同源引用，scrub 语义不变） | ai-adapters tests、responses.rs、mgr_load_tests.rs |
 
 ## Constraints（仓库硬规则，逐条核）
```

#### 抽样 diff 段 3：`docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md`（Unix 路径，非 checkout 根）
```diff
diff --git a/docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md b/docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md
index 8bd19ee..ad9a93c 100644
--- a/docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md
+++ b/docs/superpowers/specs/2026-06-26-frontend-onboarding-design.md
@@ -695,14 +695,14 @@ impl AppSettings {
  ],
  "workspaces": [
  {
- "path": "<REPO_ROOT>/projects/northing",
+ "path": "<LOCAL_PATH>",
  "display_name": "northing",
  "added_at": 1782400000,
  "last_opened_at": 1782400000,
- "identity_md_path": "<REPO_ROOT>/projects/northing/IDENTITY.md"
+ "identity_md_path": "<LOCAL_PATH>"
  }
  ],
- "current_workspace": "<REPO_ROOT>/projects/northing",
+ "current_workspace": "<LOCAL_PATH>",
  "skills_enabled": [
  {"name": "memory", "global_enabled": true, "workspace_overrides": {}},
  {"name": "pdf", "global_enabled": true, "workspace_overrides": {}}
```

#### `<REPO_ROOT>` grep 零命中证明
- 在 commit `9714d23` 验收 diff（`git diff HEAD~1 HEAD`）中 grep `<REPO_ROOT>`：**0 命中（false）**
- 在附录 A 75 个脱敏文件内容中 grep `<REPO_ROOT>`：**0 命中（0 files）**

---

## 状态

DONE
