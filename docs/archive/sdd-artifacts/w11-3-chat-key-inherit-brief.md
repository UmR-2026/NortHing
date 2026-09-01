# Task Brief — W11-3: 修复 CLI chat 侧编辑模型丢 key（决策 C7，用户拍板立即修）

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/cli` 仅 CLI crate。

## Bug（编排者已双侧核实代码）

- `modes/chat/model_config.rs` `update_existing_model`（约 L200）：`api_key: result.api_key.clone()` 直写——**编辑时 key 字段留空会把已存 key 清掉**（Scheme C 下等于删 key）。
- 正确语义参照 `ui/startup/selectors.rs` `update_existing_model`（约 L327）：`resolve_effective_model_key(&model_id, &result.api_key)`（keyring_keys.rs:51——留空→keyring 取已存 key；非空→用新值）。

## Spec

1. chat 侧 `update_existing_model` 改用 `crate::keyring_keys::resolve_effective_model_key(&model_id, &result.api_key)`，与 startup 侧逐字同语义。
2. 同文件若有其它直写 `result.api_key.clone()` 的编辑路径（区别于新建路径），一并核查——新建（save_new_model）语义本就应收新 key，不动；**只有编辑/更新路径需要继承语义**。
3. 附回归测试：`resolve_effective_model_key` 的留空继承/非空覆盖两臂（若该 helper 已有测试则改为覆盖编辑路径调用点的测试；keyring 部分用既有测试设施，不触真 keyring）。
4. 不动 startup 侧。

## 验证集（命令+输出原文进 report `.superpowers/sdd/w11-3-chat-key-inherit-report.md`）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing-cli`：0 error
2. `+stable-msvc test -p northhing-cli`：全绿（含新测试）
3. `node scripts/verify-rot-budget.mjs`：绿

## Global Constraints

1. 只动 `src/apps/cli`；行为变化仅限本 bug 修复。
2. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止整树 git 操作；开工先 `git status`。
3. commit：恰好一个（消息对齐 `fix(cli): ...` 风格）；不含 `.superpowers/`。
4. 涉 keyring：测试不得触生产存储。
5. 遇编译错误先加载对应 rust skill。

## 派发元信息

- DONE/BLOCKED + commit SHA + git show --stat + 验证输出尾部 + 偏离清单。
