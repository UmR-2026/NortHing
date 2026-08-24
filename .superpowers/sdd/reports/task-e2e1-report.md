# Task E2E-1 Report — CLI edit 表单留空继承 keyring 被 validate 拦截（F4 规格缺口修复）

> BASE: `fc81a24`. 仅改 `src/apps/cli/src/ui/model_config_form/` 下文件。未 commit，未碰任何 in-flight 文件。

## 1. 改动文件清单

| 文件 | 改动 | 一句话理由 |
|---|---|---|
| `src/apps/cli/src/ui/model_config_form/state.rs` | `validate()` 中 api_key 非空检查仅在 `editing_model_id.is_none()` 时执行；新增 `#[cfg(test)] mod tests` 含两条单元测试 | 修复 A1/A2 — edit 模式留空放行，add 模式仍拦截；测试断言两支行为不变 |
| `src/apps/cli/src/ui/model_config_form/render.rs` | `field_placeholder(field, is_edit)` 新增 `is_edit: bool` 形参；ApiKey 分支按 `is_edit` 返回不同文案；唯一调用点 `render_field_input` 传入 `state.editing_model_id().is_some()` | 满足 A3 — edit 模式下 ApiKey 占位符提示"留空保留已存 key" |

## 2. 复用侦察

按 brief §3 要求复核（编排者 preflight 表已给基础结论，再次直接查源码防漂移）：

- `validate()` / `try_save()` / `placeholder()` / `field_label()`：codegraph + grep 复核 —— 编辑前**没有** edit/add 分支。`validate()` 在 `state.rs:335-347` 是一段直线 if 链，`placeholder()` 在 `render.rs:412` 是 match 表达式，二者均未与 `editing_model_id`/`state.editing_model_id()` 关联。结论：复用侦察通过，本任务的 edit 分支是新加的，不存在等价旧实现可复用。
- 复用现有访问器：`state.editing_model_id()` 已在 `state.rs:562-564` 提供，本任务直接复用，未新写 access。
- 复用 `show_for_edit` 构造 edit 模式测试夹具（`state.rs:121`），未重写 `ModelFormResult` 路径。
- `try_save` → `update_existing_model` → `resolve_effective_model_key` 链路（brief preflight 行 28）保持原样不动，符合 S5。

未发现可以 "lift" 的等价实现。所有新增逻辑都是这次新增。

## 3. Spec 满足度逐条核对

| Spec | 满足 |
|---|---|
| S1 | ✅ `state.rs:350` — `if self.editing_model_id.is_none() && self.api_key.trim().is_empty()` |
| S2 | ✅ name / model_name / base_url / context_window / max_tokens / JSON 检查未动 |
| S3 | ✅ `render.rs:417-424` — edit 模式返回 `"Leave blank to keep the stored key"`（35 字符，英文、无 emoji），add 模式维持 `"Enter your API key"`。实现走 `field_placeholder(field, is_edit)` 加 bool 形参，调用点 `render_field_input` 传入 `state.editing_model_id().is_some()` |
| S4 | ✅ `state.rs:626-677` 新增 `#[cfg(test)] mod tests`，含两条 `#[test]`：(a) `validate_allows_blank_api_key_in_edit_mode`、(b) `validate_blocks_blank_api_key_in_add_mode` |
| S5 | ✅ 未碰 `selectors.rs`、`keyring_keys.rs`、`update_existing_model`、`resolve_effective_model_key` |

验收逐条（brief §1）：

- A1：edit 模式 + api_key 空 → `validate()` 返回 `None`；`try_save()` 走 `Save(result)` 分支（`state.rs:538-546`）；`update_existing_model` → `resolve_effective_model_key(model_id, "")` → 读 keyring（`selectors.rs:351`、`keyring_keys.rs:51-57`）。继承链恢复可达。
- A2：add 模式 + api_key 空 → 仍返回 `Some("API Key is required")`（`state.rs:350-352`）。
- A3：edit 模式 placeholder = "Leave blank to keep the stored key"（`render.rs:420`），35 字符，英文，无 emoji。
- A4：编译期 `cargo check --tests -p northhing-cli` 全绿（见 §4.2）。运行时链接器报已知 GNU 问题（见 §4.1），brief §6 已授权用 `cargo check` 作为编译判据。
- A5：`cargo check -p northhing-cli` 无 error（§4.3）。

## 4. 验证命令 + 输出原文

### 4.1 `cargo test -p northhing-cli model_config_form`

**结论：链接失败，但属于 brief §6 已授权的 GNU 环境已知问题，不是本任务代码问题。**

错误原文（最后 13 行）：

```
error: linking with `x86_64-w64-mingw32-gcc` failed: exit code: 1
  |
  = note: "x86_64-w64-mingw32-gcc" "-fno-use-linker-plugin" "-Wl,--dynamicbase" ...
  = note: some arguments are omitted. use `--verbose` to show all linker arguments
  = note: ld: E:\agent-project\northing\target\debug\deps\libaws_lc_sys-27b4b087faee3398.rlib(32ebf02b88d96293-thread_pthread.o): in function `nanosleep':
          C:/msys64/mingw64/include/pthread_time.h:84:(.text$mingw_do_backoff+0x61): undefined reference to `nanosleep64'

  = note: some `extern` functions couldn't be found; some native libraries may need to be installed or have their path specified
  = note: use the `-l` flag to specify native libraries to link
  = note: use the `cargo:rustc-link-lib` directive to specify native libraries to link with Cargo (see https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-lib)

warning: `northhing-cli` (bin "northhing-cli" test) generated 1 warning
error: could not compile `northhing-cli` (bin "northhing-cli" test) due to 1 previous error; 1 warning emitted
```

错误点位于 `libaws_lc_sys-…rlib(…thread_pthread.o)` —— aws-lc-sys crate 的 pthread 垫片，与本任务代码无关，是仓库的 GNU 工具链环境问题。Brief §6 明确指出 `nanosleep64` undefined 是已知 GNU 环境问题，要求以 `cargo check` 结果作为编译判据。运行时未执行。

唯一 warning 是 `src/apps/cli/src/ui/question/mod.rs:15` 的 `unused imports: QuestionData and QuestionOption` —— pre-existing，与本任务无关（既不在改动文件中，也不在改动文件的 import 链上）。

### 4.2 `cargo check --tests -p northhing-cli`（用于判定测试编译期正确性）

末尾原文：

```
warning: `northhing-core` (lib) generated 18 warnings (run `cargo fix --lib -p northhing-core` to apply 17 suggestions)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
warning: unused imports: `QuestionData` and `QuestionOption`
  --> src\apps\cli\src\ui\question\mod.rs:15:33
   |
15 | pub use types::{QuestionAction, QuestionData, QuestionOption, QuestionPrompt};
   |                                 ^^^^^^^^^^^^  ^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `northhing-cli` (bin "northhing-cli" test) generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli --tests` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.75s
```

**测试编译期全部通过**（pre-existing warning 不算错），确认两条新 `#[test]` 函数与 `ModelFormResult` 在测试模块中的构造都合法 —— 否则 cargo 会报 `error: … unresolved import / cannot find …` 之类。Linker 仅在 binary 输出阶段报错。

### 4.3 `cargo check -p northhing-cli`（brief §6 第 2 条）

末尾原文：

```
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.94s
```

**无 error**，编译通过。pre-existing 唯一 warning 同 §4.1。

## 5. 遇到的编译错误及修复层级

| 错误 | 层级 | 处置 |
|---|---|---|
| `nanosleep64 undefined reference` (GNU ld linker, libaws_lc_sys) | 工具链 / 环境层 —— 非本任务代码 | 不修（brief §6 已声明为已知 GNU 问题）。用 `cargo check --tests` 验证编译期正确（§4.2） |

无任何"无脑 .clone() / .unwrap()"糊编译器的痕迹：测试模块里只用了 `String::new()` 构造空白字段、`Option::unwrap()` 解 `Some("test-model".into())` 字面量，符合 brief"为糊编译器加 `.clone()`/`.unwrap()`"的禁令（第 53 行）。

## 6. 改动文件清单 vs 磁盘 diff 一致性

```
$ git diff --name-only -- 'src/apps/cli/src/ui/model_config_form/**'
src/apps/cli/src/ui/model_config_form/render.rs
src/apps/cli/src/ui/model_config_form/state.rs
```

仅这两个文件，与本报告 §1 一致。其他 working-copy 改动（`.opencode/model-capability-notes.md`、`.superpowers/sdd/progress.md`、`memory/northhing.md`、`src/crates/contracts/kernel-api/src/memory.rs`、`src/crates/contracts/kernel-api/src/turn.rs`）是 pre-existing，brief 列为 in-flight 禁区，本次未触碰（`git diff` 上无本任务新行）。

## 7. 遗留 caveat

- ~~`cargo test` 因 GNU 工具链 `nanosleep64` 链接错误未能运行时执行~~ → Round 2 已用 MSVC 工具链运行（见 §9），2/2 通过。
- Brief §6 提到 "test 若 link 失败报 `nanosleep64` undefined" 时"用 `cargo check` 结果作为编译判据" —— 那是 Round 1 期间的妥协；Round 2 在 MSVC 下已落地运行时验证。

## 8. 状态（Round 1）

DONE_WITH_CONCERNS → Round 2 升级为 DONE（见 §9）。

> Round 1：`DONE` 因 spec S1-S5 全满足、A1-A3 全覆盖、`cargo check`/`cargo check --tests` 全绿、磁盘 diff 与报告一致。
> Round 1：`CONCERNS` 因运行时验证（`cargo test` 真正执行）受环境层链接错误阻断，未能亲眼看到 PASS 行。
> Round 2：reviewer 指出 Round 1 测试本身有 bug（§9.1），已修复并在 MSVC 下真实运行通过（§9.2）。

## 9. Round 2 — Reviewer 反馈修复 + 真实运行

### 9.1 Reviewer 找出的 Critical 缺陷

`validate_blocks_blank_api_key_in_add_mode` 调用 `show_custom()`，但 `show_custom()` 会 `self.name.clear()` 与 `self.model_name.clear()`（`state.rs:73-74`）。`validate()` 的 name 检查（`state.rs:336-338`）跑在 api_key 检查之前，所以 `validate()` 返回 `Some("Name is required")` 而不是断言中的 `Some("API Key is required")` —— 真实运行时会失败。

Round 1 只验了 `cargo check --tests`（编译期），未真实跑测试，green-compile ≠ green-test。

### 9.2 修复 diff（仅 `state.rs`）

```diff
     #[test]
     fn validate_blocks_blank_api_key_in_add_mode() {
         // Add mode (editing_model_id is None) must still require a
-        // non-empty api_key — F4 only relaxes this for edit.
+        // non-empty api_key — F4 only relaxes this for edit. Fill
+        // name/model_name directly because `show_custom()` clears them
+        // and validate()'s name check runs before the api_key check;
+        // we need to reach the api_key branch to exercise it.
         let mut state = ModelConfigFormState::new();
         state.show_custom();
+        state.name = "Test Model".into();
+        state.model_name = "test-model".into();
         assert!(state.editing_model_id().is_none());
         assert!(state.field_value(FormField::ApiKey).is_empty());
         assert_eq!(
             state.validate(),
             Some("API Key is required".to_string())
         );
     }
```

理由：测试模块与 `ModelConfigFormState` 在同一文件（`state.rs`），可直接访问私有字段；这是 reviewer 明确授权的最小修法（"if private fields are needed, tests are in the same module so direct field assignment is acceptable"）。`base_url` 由 `show_custom()` 设为 `"https://"`，非空，无需手动填。

Test (a) `validate_allows_blank_api_key_in_edit_mode` 经 reviewer 复审确认无 bug：`sample_result()` 的 `name = "Test Model"`、`model_name = "test-model"` 经 `show_for_edit` 复制进 state（`state.rs:125-126`），能跑到 api_key 检查分支。Round 2 真实运行也证实通过（§9.3）。

### 9.3 MSVC 真实运行结果

命令：

```
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing-cli model_config_form
```

末尾原文：

```
warning: unused imports: `QuestionData` and `QuestionOption`
  --> src\apps\cli\src\ui\question\mod.rs:15:33
   |
15 | pub use types::{QuestionAction, QuestionData, QuestionOption, QuestionPrompt};
   |                                 ^^^^^^^^^^^^  ^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

warning: `northhing-cli` (bin "northhing-cli" test) generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli --tests` to apply 1 suggestion)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2m 36s
     Running unittests src\main.rs (target\debug\deps\northhing_cli-06cab19baaf502bb.exe)

running 2 tests
test ui::model_config_form::state::tests::validate_blocks_blank_api_key_in_add_mode ... ok
test ui::model_config_form::state::tests::validate_allows_blank_api_key_in_edit_mode ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 36 filtered out; finished in 0.00s
```

**2 passed; 0 failed.** 编译期 warning 与 Round 1 一致（pre-existing `question/mod.rs:15` unused imports），与本任务无关。

期间 cargo 在 Rust 编译阶段提示 `Blocking waiting for file lock on build directory`（无此提示显示在截取输出中，但 2m 36s 的 wall time 暗示等待锁）。未杀进程、未删锁文件，与 reviewer 指示一致。

### 9.4 Round 2 终态判定

- 测试真实运行通过（A4：2 passed; 0 failed）。
- 修复未引入新的 cargo check 噪声（warning 数与 Round 1 相同）。
- 改动文件清单仍仅 `src/apps/cli/src/ui/model_config_form/state.rs`（test 修复点所在）与 `render.rs`（Round 1 改动，本轮未触碰）。
- in-flight 禁区未触碰。

## 10. 状态（终态）

DONE