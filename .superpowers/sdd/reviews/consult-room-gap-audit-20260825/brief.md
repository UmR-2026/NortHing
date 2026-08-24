# consult-room 前端全量缺口扫描 & 修复任务

## 目标
对 consult-room Dioxus 前端线做全覆盖缺口审查，修复可自动化的缺口，产出审查报告。

## 扫描结果总览

### 架构 ✅ 健康
- 7 文件 Dioxus 模块 (entry/state/registry/css/i18n/windows/app + 4 pages)
- 3-window shell: 主诊室 + 内窗(self/facility) + 外窗(work)
- Theme 通过 watch channel 跨窗同步；生命周期 via ShellWindowManager + WindowDropGuard
- i18n: 368 行 key 定义 + LocalePack 解析，~120 键覆盖

### 页面完整度 ✅ 6/6 页面全部实现
| 页面 | 文件 | 行数 | 状态 |
|---|---|---|---|
| 主诊室 | app.rs | 652 | ✅ 全量（chrome/status/head/chat/input/membrane） |
| 沉积与设施(self) | windows.rs | ~370 | ✅ 五卡折叠 + token 计数器 |
| 档案馆 | pages_archive.rs | 459 | ✅ 8 层地层 + 3 侧栏卡 |
| 走廊(space) | pages_space.rs | ~560 | ✅ 门厅 + 沉积层 + 房间输入 |
| 全局设置 | pages_settings.rs | ~450 | ✅ 双栏 10 节布局 |
| 房间诞生仪式 | pages_onboarding.rs | ~500 | ✅ 3 步仪式 + 色板选择 |

### 缺口清单（按优先级）

#### 🔴 P1 — 数据/行为未接线 (设计已知，留注释)
这些在规格化阶段保持 mock 是合理的，但缺少 `// TODO` 注释标记后续接入点：
1. `session_mock.rs` — 全部硬编码 MockEntry，无真实数据流
2. `pages_settings.rs` — 引擎选择 / MCP 开关 / provider 切换全是 use_signal 无持久化
3. `pages_archive.rs` — STRATA 数据静态表，无真实归档查询
4. `pages_space.rs` — DOORS 静态数据，无真实 workspace 列表

**修复**: 每个交接点加 `// TODO(data): wire to <真实数据源>` 注释

#### 🟡 P2 — i18n 硬编码字符串穿透 (可自动修复)
以下页面的非 i18n 字符串应走 `locale.t(keys::)`：
1. `pages_settings.rs`: 
   - "Claude 3.7 Sonnet" / "Gemini 3.1 Pro" / "GPT-4o" (引擎列表)
   - "Anthropic API" / "Google AI Studio" (provider 列表)
   - "@filesystem" / "@philosophy-core" / "@terminal" (MCP 列表)
   - "读写存取" / "哲理外挂" / "未授权" (MCP 状态)
   - "直接连接" (provider 状态)
   - "E:\\agent-project\\northing\\" (workspace 路径)
2. `app.rs`: "architect_sub 介入中" (status pill)
3. `pages_archive.rs`: 全部 STRATA 数据 (23 段对话文案可保留为 mock 但需注释)

**修复**: 加 i18n keys → 在 i18n.rs keys 模块声明 → 在 zh-CN.ftl 加翻译

#### 🟡 P2 — SVG 图标重复 (DRY 违反)
Moon/sun 主题切换 SVG (~25 行) 在 7 个文件中完全一致：
- app.rs, windows.rs(self), windows.rs(facility), pages_archive.rs, pages_space.rs, pages_settings.rs, pages_onboarding.rs

Brand SVG logo (~35 行) 在 3 个文件中一致：
- app.rs, pages_archive.rs, pages_space.rs

**修复**: 提取两个 `const` 函数在 css.rs 或新 `icons.rs`:
```rust
pub fn theme_toggle_svg(is_dark: bool) -> &'static str { ... }
pub fn brand_logo_svg() -> &'static str { ... }
```

#### 🟢 P3 — 窗口壳样板重复 (可提取组件)
每个页面文件重复相同的 5 段样板：
1. WindowDropGuard init
2. register_window_with_hwnd effect
3. theme_rx → theme_dark use_future
4. fold_all 函数
5. close button (hide_and_close_hwnd + window().close)

**修复**: 创建 `PageShell` 组件封装样板，各页面传 props。注意：ponytail 适用——15 文件中有 ~60 行样板，提取节省净约 40 行/页 × 6 = 240 行。收益明确，做。

#### 🟢 P3 — 其他小缺口
1. `app.rs` 主诊室 chronicle-bar div 空壳无内容 (line ~368-374) — truth HTML 有渐变条动画
2. Approval 卡片 approve/reject 按钮无 handler (line 629-635)
3. `assert_truth_css_byte_count`: 断言写死 `>1000` 和 `contains(":root {")`，应改为读 TRUTH_CSS.len() 的精确值（当前文件实际字节数）
4. `WelcomeView.slint` 仍存于 src/apps/desktop/src/ui/views/ — Slint 遗留路径，若 Dioxus 正式成为唯一壳可清理

## 执行约束

1. **不触碰**: session_mock.rs 的 mock 数据内容（改文案不改结构）
2. **不触碰**: 主诊室 chronicle-bar 空白（需用户裁决是否有动画）
3. **不触碰**: Slint 遗留文件（.slint 文件删除需显式用户同意）
4. **保守提取**: PageShell 组件保持最小表面积，不引入新 trait/抽象
5. **i18n 只加不减**: 新 key 必须同时在 zh-CN.ftl 有译文
6. **verification**: `cargo check -p northhing --features ui-dioxus` 必须通过
7. **scope 优先**: 先做 P2（可自动化的），P1/P3 留注释/标记

## 交付

1. 修改后的源文件（i18n keys 注入 + SVG 提取 + PageShell 组件）
2. `.superpowers/sdd/reviews/consult-room-gap-audit-20260825/` 目录:
   - `brief.md` → 本文件
   - `diffstat.txt` → git diff --stat
   - `report.md` → 审查结果
3. 审查报告含: 改了什么 / 没改什么 / 为什么 / 剩余缺口
