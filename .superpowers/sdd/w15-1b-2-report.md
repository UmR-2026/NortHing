# W15-1b-2 Report — Markdown 集成单（app.rs 三渲染点 + CSS）

BASE: `329cc8f`。接手时工作树含前任 coder 未提交半成品（app.rs / markdown_render.rs / mod.rs 修改 + pages_chat_md_css.rs 新建）。本报告含对半成品的逐条裁定。

## 完成状态: DONE

## 改动清单（最终态，vs BASE）

| 文件 | 改动 |
|---|---|
| `src/apps/desktop/src/ui_dioxus/pages_chat_md_css.rs` | 新建（S1）。`pub const CHAT_MD_CSS`，仿 `pages_onboarding_css.rs` 形态。全部选择器以 `.md-rendered` 为前缀作用域（仲裁 §8.3：不漏到非 md 区域）。`.md-rendered { white-space: pre-wrap; word-break: break-word; }` 覆盖仲裁 §7#7 硬性要求。段落/标题 h1-h6/列表/`pre.md-code-block`（等宽+pre-wrap）/内联 code（`--font-mono`）/链接（`var(--accent-solid)`，磁盘核实该变量存在于 css.rs :439/:536/:699）/blockquote 左灰条/hr dashed/img/strong/em。 |
| `src/apps/desktop/src/ui_dioxus/mod.rs` | S2：`mod pages_chat_md_css;` 一行，按字母序插入（与兄弟模块形态一致）。 |
| `src/apps/desktop/src/ui_dioxus/app.rs` | S3：`:334` 在 `room_app_root` 的 head 追加 `style { dangerous_inner_html: "{pages_chat_md_css::CHAT_MD_CSS}" }`（原 :332-333 两行未动；CHAT_MD_CSS 为受信任常量，注入形态与现有同款）。S4：三渲染点 `:509`（流式 draft）、`:750`（assistant 完成态）、`:760`（witness 完成态）接 `render_markdown`，div 加 `md-rendered` class。消息文本零 `dangerous_inner_html`（仲裁 §7#2 红线，diff 中唯一新增 dangerous_inner_html 即 S3 的 CSS 常量注入）。 |
| `src/apps/desktop/src/ui_dioxus/markdown_render.rs` | S5 四处小修：`render_single_inline`/`render_single_block` 的 `key` 实参接入 RSX `key:` 属性（含列表 `li` 的 `key: "{idx}"`）；`escape_html` 补 `'` → `&#39;`；三处 `_ => {}` catch-all 加 `// ponytail:` 注释（Options::empty() 下不可达，升级路径=pulldown-cmark 升版/GFM 扩展时改穷尽匹配）；图片 alt 内格式化丢弃处加已知限制注释。 |

注入窗裁定（仲裁 §8.4）：**仅注入 1 窗（room_app_root）**。grep 证据：`.msg-agent` 与 `render_entry`/`render_entries` 只存在于 app.rs 的 room 文档树；archive/space/settings/memory/work/self/facility 等窗不渲染本三处消息正文。

## 对半成品的裁定

**保留约 95%，1 处删减，0 处重写。**

- app.rs 四处改动：**全保留**。逐点核对正确（含 `draft`/`body` 均为 `&String`，`render_markdown(&str)` 经 deref  coercion 成立；children 循环在 msg-agent div 之外，不受 md 渲染污染）。
- markdown_render.rs S5：**全保留**。四项要求全部落实且形态正确。
- mod.rs：**保留**。
- pages_chat_md_css.rs：**保留主体，删一个冗余选择器块**。前任写了 `.msg-agent.md-rendered, .body.md-rendered, .rec.witness .body.md-rendered { pre-wrap... }`，与紧随其后的 `.md-rendered { pre-wrap... }` 声明完全相同且被后者全量覆盖——删除并留一行注释指向 §7#7。其余样式逐条对照 brief S1 清单无遗漏，CSS 变量全部磁盘核实存在（`--accent-solid`/`--font-mono`/`--mind-base`/`--bg0`/`--line`/`--muted`/`--text`/`--faint`/`--mind-line`），`color-mix` 用法与 TRUTH_CSS 既有语言一致。
- 半成品的问题：**无任何验证记录、无 report**——三验证 + git diff --check 本次全部由我重跑（见下）。

## 验证输出摘录（全部本 coder 实测，C6 范式）

1. `cmd /c "cd /d E:\agent-project\NortHing && %USERPROFILE%\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing > w15b2-check-desktop.log 2>&1"` → **通过**：`Finished dev profile ... in 35.43s`，0 error；无本次改动引入的新 warning。
2. 同范式 `cargo check --workspace` → **通过**：`Finished dev profile ... in 9.33s`，0 error（w15b2-check-ws.log）。
3. 同范式 `cargo test -p northhing --lib --no-run` → `BUILD_OK`，二进制 `target\debug\deps\northhing-aba6cc17ae1929a4.exe`；cd 到 `src\apps\desktop` 直接跑二进制 → **`test result: ok. 164 passed; 0 failed; 0 ignored`**，其中 markdown_render 测试行数 = **19**（全绿，覆盖不下降）。注：cmd 等父进程退出后孙进程短暂持有日志句柄导致立即 findstr 报"文件正被使用"，等待后读取正常（正是 long-running-shell skill 预警的句柄继承现象，非测试失败）。
4. `git diff --check` → **通过**（exit 0，无空白错误）。
5. 截图：按 brief 修订剥离给编排者。**本单未启动任何 GUI 进程**。
6. 熔断：未触发，所有命令分钟级内完成（最长 35s）。

红线自查：`git diff` 对 `css.rs`、`pages_archive.rs`、`pages_archive_search.rs`、`contracts/kernel-api/src/session.rs` 全部为空 ✓；diff 中无新增 `let _ =`（rot 闸 371/388 不涨）✓；日志/注释英文（ponytail 注释按仓内既有中文注释惯例除外——app.rs 周边即中文）✓。

## 偏差（C4 记录）

- brief 行号 :502-509/:747/:757 → 磁盘实际 :509/:750/:760；注入点 brief :330-331 → 实际追加于 :334（原两行之后）。语义与 brief 一致。
- brief 写 `render_markdown(&draft)`，实现用 `render_markdown(draft)`（draft 已是 `&String`，`&&String` 写法冗余；类型等价）。
- `cargo fmt`（`pnpm run fmt:rs`）本地不可跑：`rustfmt.exe is not installed for the toolchain stable-x86_64-pc-windows-msvc`。安装组件超出本单权限，留给 CI/编排者。手工对齐了周边风格。

## 遗留 follow-up（仲裁 §7#14 要求记录）

- W15-2 候选：删除 `css_files.rs` 孤儿文件，或注册进 mod.rs（本单按裁决不处理）。
- 仲裁 §7#11 的三张视觉回归截图：由编排者验证 5' 负责（流式 draft / 完成 assistant / 完成 witness 三态）。
