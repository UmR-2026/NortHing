# W15-1b-3 Implementation Report — markdown_render.rs 拆测试回 rot 闸

## 改动内容

- 剪切 `src/apps/desktop/src/ui_dioxus/markdown_render.rs` 中 `#[cfg(test)] mod tests { ... }` 测试体至新模块文件 `src/apps/desktop/src/ui_dioxus/markdown_render/tests.rs`。
- 原文件末尾替换为 `#[cfg(test)] mod tests;`。纯位移，测试体逻辑与实现体完全不变。
- 拆后行数：
  - `src/apps/desktop/src/ui_dioxus/markdown_render.rs`: 509 行（< 800 门限，原 857 行）
  - `src/apps/desktop/src/ui_dioxus/markdown_render/tests.rs`: 346 行

## 验证证据

### 1. Rot budget 检查（转绿）

命令：
```
node scripts/verify-rot-budget.mjs
```

输出：
```
Rot budget verification passed (5 grep rules [unwrap_production=483/502, expect_production=940/1089, let_underscore=371/388, unix_epoch_inline=69/69, allow_dead_code=106/109], 3 dir rules [dir_entries:scripts=42/42, dir_entries:docs/design=1/1, dir_entries:.superpowers/sdd=44/400], 6 god-file rules checked across 1367 files).
```

### 2. 桌面端编译检查

命令：
```
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing
```

输出：
```
warning: `northhing` (bin "northhing") generated 61 warnings (2 duplicates)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.61s
```

### 3. markdown_render 测试套件

命令：
```
C:/Users/UmR/.cargo/bin/rustup.exe run stable-x86_64-pc-windows-msvc cargo test -p northhing --lib markdown_render
```

输出：
```
running 19 tests
test ui_dioxus::markdown_render::tests::test_render_hard_and_soft_break ... ok
test ui_dioxus::markdown_render::tests::test_render_paragraph ... ok
test ui_dioxus::markdown_render::tests::test_sanitize_url_scheme_whitelist ... ok
test ui_dioxus::markdown_render::tests::test_render_blockquote ... ok
test ui_dioxus::markdown_render::tests::test_render_ordered_list ... ok
test ui_dioxus::markdown_render::tests::test_render_headings_h1_to_h6 ... ok
test ui_dioxus::markdown_render::tests::test_render_emphasis_and_strong ... ok
test ui_dioxus::markdown_render::tests::test_render_code_block ... ok
test ui_dioxus::markdown_render::tests::test_render_whitelisted_image ... ok
test ui_dioxus::markdown_render::tests::test_render_horizontal_rule ... ok
test ui_dioxus::markdown_render::tests::test_render_inline_code ... ok
test ui_dioxus::markdown_render::tests::test_render_links_whitelisted ... ok
test ui_dioxus::markdown_render::tests::test_render_unordered_list ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_1_raw_script_tag ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_2_javascript_scheme_link ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_4_raw_img_onerror ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_3_data_scheme_image ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_6_nested_script_tags ... ok
test ui_dioxus::markdown_render::tests::test_xss_vector_5_vbscript_scheme_link ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 145 filtered out; finished in 0.00s
```

### 4. Git diff check

命令：
```
git diff --check
```

输出：clean (0 output)

## 状态

DONE
