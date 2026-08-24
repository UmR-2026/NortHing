# E4 fix1 — onboarding 窗原生滚动条 — implementer report

## 改动位置
- `src/apps/desktop/src/ui_dioxus/pages_onboarding_css.rs`: L195-L202（`ONBOARDING_CSS` 字符串常量内，追加 `::-webkit-scrollbar` 7 条规则与注释）

## 验证
1. `C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing`
   - 结果: exit 0 (Finished `dev` profile in 25.89s)
2. `pages_onboarding_css.rs` 总行数: 217 行（< 800 行）
