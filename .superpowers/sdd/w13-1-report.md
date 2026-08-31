# Task Report — W13-1: 清除生产路径的 mock 会话残留

## 状态
DONE

## Commit
`cf34a7a160d8f2c1763ef3688c04df90a512ee5c` (`cf34a7a`)

```
commit cf34a7a160d8f2c1763ef3688c04df90a512ee5c
Author: Mavis <mavis@northhing.local>
Date:   Tue Sep 1 00:25:25 2026 +0800

    fix(desktop): remove seed_session mock residue from production path (W13-1)

 src/apps/desktop/src/ui_dioxus/app.rs          | 4 ++--
 src/apps/desktop/src/ui_dioxus/session_mock.rs | 1 +
 2 files changed, 3 insertions(+), 2 deletions(-)
```

## 改动清单

1. `src/apps/desktop/src/ui_dioxus/app.rs`:
   - L30: 移除 `seed_session` 的导入，仅保留 `use super::session_mock::MockEntry;`。
   - L57: `entries` signal 初值由 `use_signal(|| seed_session())` 改为 `use_signal(Vec::<MockEntry>::new)`。
   - 保留 `MockEntry` / `messages_to_entries` / `render_child` 行为与签名完全不变。
   - 行数保持 791 行（≤800 上限，满足 god-file 约束）。
2. `src/apps/desktop/src/ui_dioxus/session_mock.rs`:
   - L55: 为 `seed_session()` 添加 `#[cfg(test)]` 属性，从生产编译与路径中彻底摘除，保留单测 `test_seed_session_has_mock_approvals_with_call_ids` 的调用。
3. `mock_stream` 调查结论：
   - 经全局代码与提交历史检索，`mock_stream` 仅在 `session_mock.rs` 顶部第 7 行的旧 spike 注释中被提及，代码库中从未存在该函数的实体定义或调用。

## 验证证据

### 1. 桌面编译检查 (`cargo check -p northhing`)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check -p northhing
```
输出尾部：
```
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.71s
```

### 2. 单元测试 (`cargo test -p northhing --lib ui_dioxus::session_mock`)
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing --lib ui_dioxus::session_mock
```
输出尾部：
```
running 6 tests
test ui_dioxus::session_mock::tests::test_messages_to_entries_empty_returns_empty ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_system_and_tool_skipped ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_reasoning_fallback ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_assistant_mixed_with_tool_calls ... ok
test ui_dioxus::session_mock::tests::test_messages_to_entries_user_text_to_witness ... ok
test ui_dioxus::session_mock::tests::test_seed_session_has_mock_approvals_with_call_ids ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 141 filtered out; finished in 0.00s
```

### 3. 生产路径引用检查 (`git grep -n "seed_session" -- "src/apps/desktop/src"`)
```powershell
git grep -n "seed_session" -- "src/apps/desktop/src"
```
输出：
```
src/apps/desktop/src/ui_dioxus/session_mock.rs:56:pub fn seed_session() -> Vec<MockEntry> {
src/apps/desktop/src/ui_dioxus/session_mock.rs:168:    fn test_seed_session_has_mock_approvals_with_call_ids() {
src/apps/desktop/src/ui_dioxus/session_mock.rs:169:        let entries = seed_session();
```

## 「seed_session 现在被谁引用」最终结论

`seed_session()` 在生产（非 test）路径中**零引用**，生产代码完全与其解耦。
目前仅且仅在 `src/apps/desktop/src/ui_dioxus/session_mock.rs` 的单测模块 `mod tests` 中的 `test_seed_session_has_mock_approvals_with_call_ids`（L169）被单元测试调用。

## 偏离清单
无偏离。严格只修改并 commit 了 `src/apps/desktop/src/ui_dioxus/app.rs` 与 `src/apps/desktop/src/ui_dioxus/session_mock.rs` 两处文件。

## 编译错误修在哪一层
本次改动直接一次性编译通过，未触发 Rust 编译错误（E0xxx）。
