# Task Report — W11-3: 修复 CLI chat 侧编辑模型丢 key

状态：**DONE**
commit SHA：`69fb851fccc87e31368c950ba2e83d6dc3716454` (short: `69fb851`)
范围：仅 `src/apps/cli`，恰好一个 commit，未触 startup 侧。

## `git show --stat 69fb851`

```
commit 69fb851fccc87e31368c950ba2e83d6dc3716454
Author: Mavis <mavis@northhing.local>
Date:   Sat Aug 29 22:08:24 2026 +0800

    fix(cli): chat model edit inherits stored keyring key (W11-3)

    Chat-side ChatMode::update_existing_model wrote result.api_key.clone()
    straight into core, so a blank API-key field on edit wiped the key in core's
    in-memory config (Scheme C - keyring entry was the only persistent copy).
    Swap to crate::keyring_keys::resolve_effective_model_key for literal
    parity with the startup-side update_existing_model (selectors.rs:327):
    empty form field -> resolve from keyring; typed value -> use as-is.
    New-model saves (save_new_model) deliberately keep accepting the typed
    key as-is, matching startup semantics.

    Add regression test chat_edit_path_resolve_contract covering both arms
    without touching a real keyring.

 src/apps/cli/src/keyring_keys.rs            | 15 +++++++++++++++
 src/apps/cli/src/modes/chat/model_config.rs |  9 ++++++++-
 2 files changed, 23 insertions(+), 1 deletion(-)
```

## 改动摘要

### 1. `src/apps/cli/src/modes/chat/model_config.rs`

`ChatMode::update_existing_model`（chat 侧编辑路径）：

- **前**：`api_key: result.api_key.clone(),` —— 表单 api_key 直写 core，Scheme C 下空字段等于删 key。
- **后**：

```rust
// Scheme C: an empty key field on edit inherits the stored keyring
// entry instead of wiping the existing key — parity with the
// startup-side `update_existing_model`. New-model saves (above) keep
// accepting the typed key as-is.
let effective_key =
    crate::keyring_keys::resolve_effective_model_key(&model_id, &result.api_key);
...
    api_key: effective_key,
```

`save_new_model`（新建路径）按 spec 第 2 条**未改** —— 新建本就该收新键。

### 2. `src/apps/cli/src/keyring_keys.rs`

新增回归测试 `chat_edit_path_resolve_contract`，显式以聊天侧编辑路径命名，锁两臂合约：

- 空表单 + 无 keyring 条目 → `""`（不挂真实 keyring）
- 键入 → 始终获胜

不触碰真实 keyring，全部用例复用既有 helper 的"无条目 ID"路子。

## 验证输出尾部

### 1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing-cli`

```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.63s
```

> 0 error。仅仓库历史已有的 unused-variable / unused-import 警告（来自 `northhing-core` 与 `ui/question/mod.rs`），均非本改动引入。CLI 包本身无新增警告。

### 2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-cli`

```
running 52 tests
test keyring_keys::tests::typed_key_wins_over_keyring ... ok
test keyring_keys::tests::missing_keyring_entry_resolves_to_empty ... ok
test keyring_keys::tests::chat_edit_path_resolve_contract ... ok
...
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

> 新增的 `chat_edit_path_resolve_contract` 通过，全部 52 条绿。

### 3. `node scripts/verify-rot-budget.mjs`

```
Rot budget verification passed (5 grep rules [unwrap_production=477/502, expect_production=940/1089, let_underscore=388/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=350/400], 6 god-file rules checked across 1364 files).
```

## 偏离清单

- **观察（非偏离）**：startup 侧 `update_existing_model`（`ui/startup/selectors.rs:373`）在 resolve 之后还调 `store_model_key(&model_id, &effective_key)` 把结果回写 keyring；chat 侧本次仅做 resolve 替换，未加 `store_model_key` 调用。spec 第 1 条只要求"逐字同语义"对应 `resolve_effective_model_key` 这一行，且 Global Constraints 第 1 条强调"行为变化仅限本 bug 修复"，故 chat 侧与 keyring 的回写同步被刻意留作另一个决策点（如果用户后续要全 parity，告诉我，加 5 行 + 一条测试即可）。
- **观察（非偏离）**：仓库根 `docs/product/requirements-vs-current-2026-08-29.md` 已有未提交的本地修改；`.superpowers/sdd/` 下若干 brief/report/截图依旧未追踪（既有会话遗留）。两处均**未触碰**。
- 提交消息走的是 `git commit -m`，事实上 PowerShell 在第一个调用里把整段多行消息当作一条 `-m` 入了提交，文字内容与 spec 期望一致。
