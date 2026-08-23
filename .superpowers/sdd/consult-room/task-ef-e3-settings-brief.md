# E3 settings 窗 — implementer brief

编排者只设计，你写代码。不要问问题。不要 commit。

先读：`.superpowers/sdd/consult-room/task-ef-pages-master-brief.md`  
真值：`docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-settings-v2.html`  
已在工作树（必须保留、在其上追加）：E1 `pages_archive.rs`、E2 `pages_space.rs`、`DockSide::Center`、`#nav-archive` / `#nav-space`。

## 1. 产品

独立 OS 窗 `id="settings"`，`DockSide::Center`，契约尺寸 **760×580**（plugin initial_width/height）。

**Chrome（W2.7 轻）**：标题「全局设置」+ ▴ 收纳（折/展两列所有内容卡）+ 主题钮 + ✕。frameless，skip-taskbar。无 ─□。

**双列（真值哲学，不可做成管理后台表格）**

左列「它的自我」**只读**（`cursor: default`，无 toggle 命中）：
1. 沉积记忆 SEDIMENT — 三行 # 记忆 + seg-bar + 「深渊级 · 封存层」
2. 编年史 CHRONICLES — Genesis / Event 两行只读
3. 身份 IDENTITY — 名讳 NortHing / 位格
4. 准则 AXIOMS — 三行只读 #

右列「设施」**可点 mock**（点行切换 active，不必写盘）：
1. 模型引擎 ENGINE — Claude 3.7 当前 / Gemini / GPT-4o
2. 上下文 CONTEXT — 默认可折到标题；展开：全局作用域 + seg-bar
3. 接入点 PROVIDER — Anthropic / Google
4. 能力集 MCP & SKILLS — @filesystem / @philosophy-core / @terminal（未授权用 danger 色标注，mock）
5. 工作区 WORKSPACE — 路径一行 + 「重新定位锚点」死按钮（onclick 可 noop）
6. 显示模式 DISPLAY — 生物态呼吸 / 双光学 两行 sq-toggle mock

卡语法抄 W2.7：点标题 `is-folded` 只剩标题；展开 `flex: 1 1 auto`；标题水平 padding 18px。左列与右列之间不要第三 OS 窗。

## 2. 接线（本刀核心）

`windows.rs` 里两处死按钮必须接上（约 :332 与 :512）：

```
class: "sys-config w2-foot"
「≡ 全局设置」
```

现在无 onclick。改为：`stop_propagation` + `spawn_module_window("settings", &manager, &rx, &theme)`。  
`self_app_root` / `facility_app_root` 已有 `manager` 与 geometry/theme 上下文——跟宝石 spawn 同样签名。若 theme 只有 `theme_rx`，用 E2 已暴露的 `spawn_module_window_with_theme_rx`（见 `app.rs`）。

不要在 room 状态行再加第三个导航（档案/走廊已够）。入口 = 设施卡脚那颗 ≡。

## 3. 文件

- **必须新文件** `src/apps/desktop/src/ui_dioxus/pages_settings.rs`（<800）
- `mod.rs` 声明 `mod pages_settings;`
- `registry.rs` 注册 id `"settings"`，760×580，Center，`pages_settings::settings_app_root`
- i18n 新键 × zh-CN / zh-TW / en-US（chrome、两列头、各卡 title/em；mock 行可硬编码中文标本，与 archive 地层同例）
- OVERLAY：`body[data-window="settings"]` 前缀。**`css.rs` 已 758 行，硬线 800**——本刀 overlay 增量必须 <40 行。能复用 archive/space 的 `.mod.is-folded` / 18px 规则就复用，不要复制整份卡 CSS。
- DropGuard + `register_window_with_hwnd`。无 geometry follow。
- 单测：registry 里抄 `test_space_registration_and_lifecycle` 写 `test_settings_registration_and_lifecycle`

## 4. 不要做

- 不要做 onboarding
- 不要真改 provider/MCP
- 不要五导航独立子页（真值 v2 就是双卡一屏，不是 T4 的五页浮层）
- 不要改 TRUTH_CSS 文件
- 不要 commit
- flags 取证完必须 `git checkout -- src/apps/desktop/src/flags.rs`

## 5. 验证

1. `C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo check -p northhing`
2. 同上 `cargo test -p northhing ui_dioxus` 与 `cargo test -p northhing flags`
3. `pnpm run i18n:audit`（exit 0，grandfathered warning 可保留）
4. 临时 DIOXUS_SHELL=true，`cargo build -p northhing`
5. CDP：Hidden + `--remote-debugging-port=9333`。开 room → 左宝石开沉积窗 → 点 `.sys-config` 或 `≡`。截图：
   - `C:\WINDOWS\TEMP\opencode\t7-shots\e3-settings-dark.png`
   - `e3-settings-light.png`
   - `e3-settings-folded-dark.png`（左列至少折两张卡）
6. **Read 打开三张 PNG**：左只读右可点的视觉差要在；不是表格后台；轻 chrome；18px 不贴边。
7. restore flags；`Stop-Process -Name northhing -Force`
8. 报告：`.superpowers/sdd/consult-room/task-ef-e3-settings-report.md`

## 6. 完成定义

- 点「≡ 全局设置」开出 settings 窗，关得掉，再点不叠尸
- 双列哲学在画面上成立
- 卡可折；双光学；flags=false；无 commit
