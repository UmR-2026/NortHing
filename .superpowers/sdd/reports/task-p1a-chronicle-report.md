# Task P1a Report — F4 编年史条（状态驱动渐变）

## 状态
DONE

## 修改文件
- `src/apps/desktop/src/ui_dioxus/app.rs`

## 变更摘要
1. 在 `room_app_root` 中添加 `mind_base` 与 `mind_history` Signal，初始值分别对齐真值 `nowC` (`#C8714C`) 与 `hist` (`["#DAD6CF", "#3F837B", "#8B5FBF"]`)。
2. 实现 Rust 端混色算法 `mix_hex`（RGB 线性通道插值）与渐变计算函数 `chronicle_gradient`（0..70% 历史沉积褪色 + 100% 当前全饱和色，含单历史除零守卫）。
3. 接线 `chronicle-bar` div：绑定内联状态驱动背景样式 `background: {chronicle_gradient}`，并在 `ondoubleclick` 事件中实现历史入栈及 MINDS 环形轮换。
4. 在 `app.rs` 的 `#[cfg(test)]` 模块中添加 4 项必测单测。
5. 遵守禁区：未修改 `TRUTH_CSS`，未引入 rAF/JS easing。

## 验证输出（Verbatim）

### 1. 编译检查 (`cargo check -p northhing --features ui-dioxus`)
```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo check -p northhing --features ui-dioxus
```

```
    Checking northhing-kernel-api v0.1.0 (E:\agent-project\northing\src\crates\contracts\kernel-api)
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.56s
```

### 2. 单元测试 (`cargo test -p northhing --features ui-dioxus -- ui_dioxus::app::tests`)
```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
& "C:\Users\UmR\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing --features ui-dioxus -- ui_dioxus::app::tests
```

```
   Compiling northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 43.21s
     Running unittests src\lib.rs (target\debug\deps\northhing-4a70ae8bdb5acd3a.exe)

running 4 tests
test ui_dioxus::app::tests::test_mix_hex_base ... ok
test ui_dioxus::app::tests::test_mix_hex_target ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_three_history ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_single ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 122 filtered out; finished in 0.00s

     Running unittests src\main.rs (target\debug\deps\northhing-248da8d1dc09dc33.exe)

running 4 tests
test ui_dioxus::app::tests::test_mix_hex_base ... ok
test ui_dioxus::app::tests::test_mix_hex_target ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_single ... ok
test ui_dioxus::app::tests::test_chronicle_gradient_three_history ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 122 filtered out; finished in 0.00s

   Doc-tests northhing

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

## 偏离与裁定
无偏离。严格按 brief 与 truth HTML 语义实现。
