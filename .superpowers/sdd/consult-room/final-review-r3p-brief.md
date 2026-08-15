# Final Review brief — R3' 终审（room 迁移 Dioxus 全线收口）

> 审查类型：**终审**（R3' 关账 + 合并前质量门）。双判决（spec 合规 + 代码质量）缺一不算通过。
> 你未参与本范围任何实现/修复轮，是独立审查者。全部材料走文件路径。

## 0. 坐标

- worktree：`E:\agent-project\northing\.worktrees\consult-room-build`（分支 `feat/consult-room-slint`）
- 审查范围：`8428d22..ef1c1db`（14 commits，全 R3' 迁移 + Bug B 修复链 + F1-F5 验收批 + R4-R9 视觉收口）
- **diff 文件**：`.superpowers/sdd/consult-room/final-review-r3p.diff`（4324 行，`src/` + `Cargo.lock`；
  SDD 文档 4 件刻意排除在 diff 外——过程产物非代码）
- 仓库文件可直接读全源码（diff 上下文不够时追完整文件）。
- commit 链（旧→新）：
  d1a7540（Cargo.toml feature+deps）→ b805033（lib/main 接线）→ 727f899（ui_dioxus 8 件初版）
  → 65cd994（Bug A locale_path+容错）→ 743ea8c（ftl 63 键×3 语）→ 273f113（mount-once LocalePack+变更守卫）
  → be5a352（theme watch 事件化）→ 900c571（几何 tao 钩子+线程 follow）→ a2e60b1（F1-F5 前置 A+B+C）
  → 5bcc285（F1-F5 验收批）→ a202028（R4-R9 视觉收口）→ ef1c1db（docs，diff 外）。

## 1. 被审系统（一句话）

`ui_dioxus/` = 三窗 Dioxus consult-room shell（room + inner + outer 三个平级 OS 窗，同进程，
共享 WebView2 数据目录），dioxus `=0.8.0-alpha.1`（精确锁，用户裁定）；与既有 Slint 壳两栈并存，
运行时闸门 `flags::DIOXUS_SHELL`（**committed 默认必须 = false**，有回归测试锁）。

## 2. Constraints（逐字生效，逐条核）

### 2.1 plan.md §3 Global Constraints 适用条

> 2. **哲学红线**：rep 只属 agent（用户/见证者侧不染边/底）；禁 dashboard 数字；禁 emoji；
> 品牌入状态行，不独立成印（brand-inline：logo 15px + Fraunces italic 12px 字标，整组 opacity .7，色随状态行文字）；
> 8s 单钟呼吸、振幅分级（主体>边界>结构）、不新增 infinite；近尖角语言（头像/条/pill 方形化 radius 0，极小圆点除外）；
> 编年史右端 ≡ 界面强调色同源。
> 3. **i18n**：新增 UI 文案走 i18n 契约；日志 English-only。

（§3.1 是 Slint 机制红线，本栈为 WebView2/CSS，机制条不适用；等价约束见 2.2 末两条。）

### 2.2 task-migrate-room-brief.md §5 边界（骨干纪律，逐字）

> - **红线**：禁改任何验证/审计脚本（`scripts/i18n-audit.mjs` 等）；验证失败必须写进报告，不得削弱断言、不得加过滤、不得删用例。违者整轮作废。
> - **编码纪律**：禁用 PowerShell 重定向/Add-Content 写源文件（BOM 前科）；一律用专用编辑工具（write/edit）；不动既有文件的行尾与无关注释。
> - **路径白名单**（其余一律只读）：`src/apps/desktop/{Cargo.toml, build.rs, src/main.rs, src/lib.rs, src/flags.rs, src/ui_dioxus/**}` + `src/crates/assembly/core/locales/*.ftl`（三语必须对称新增）。
> - 不动 core/其它 crate 公共 API；不动 Slint ui/ 既有代码（两栈并存）。
> - 日志走 `tracing`（仓库既有纪律）；无 emoji；无新增仓库外依赖（除 §2.6 清单）。
> - 真值 JS 全 mock 不移植；color-mix/keyframes 计数零新增；阴影仅四式；无 backdrop-filter；glyph 集受限（全文见 conversion-annotations）。

（「阴影仅四式」较真值既有四式；R7-R9 新增 box-shadow 形态属 §3 已裁定偏离 1/2，核实现质量即可。）

### 2.3 R4 视觉轮约束（task-r4-visual-brief.md §4 逐字，R4-R9 全程生效）

> - **禁止**改动 TRUTH_CSS 字节内容（`assert_truth_css_byte_count` 守卫测试必须原样通过）；所有覆盖/新增只进 OVERLAY_CSS。
> - **禁止**新增 locale key、禁止改 locale 文件（i18n:audit 154 基线不增不减）。
> - **禁止**在 room 类组件引入带 sleep/timers 的 `use_future`（r3p4 教训：busy-spin + Poll 风暴）。
> - 文件长度红线：单文件 <800 行（css.rs/app.rs 已接近，新增代码优先内联紧凑，超限需披露）。

## 3. 已裁定偏离清单（用户判决在案，裁定本身不审；**实现质量仍须审**）

1. scrim 元素+规则移除（偏离 block-contract §2 降级形态，R8 用户判决；app.rs/css.rs 双侧移除是否干净）。
2. 宝石形态 = 实心细柱 ::before + box-shadow 光晕（偏离真值「墙缝漏光而非漆条」注释，R7 用户判决）。
3. 竖签/见证说明元素级移除（R6 用户判决；i18n key 保留 + `#[allow(dead_code)]`，audit 154 不动）。
4. `--gem-mid` 真值回落 84 → 显式 85px 逻辑（K≈1.44 物理/逻辑标定在案）。
5. `body{margin:0}` 转写层新增（真值无 reset，UA 8px 根因修复）。
6. `css::truth_css()` 运行时 strip UTF-8 BOM（TRUTH_CSS 常量逐字节不动，守卫测试原样过）。
7. state-dot 呼吸点删除；8s 呼吸钟移植头像渐变（fill/glow/ring 三式，`@media prefers-reduced-motion: no-preference` 门控，真值 breath-avatar 缩放保留同钟）。
8. `quit_shell()` = `std::process::exit(0)`（注释称规避 never-type fallback deny；关 room = 退整个 shell 防孤儿窗）。
9. dock follow 线程无优雅停止（随进程消亡）；16ms `std::thread::sleep` OS 级轮询 + Win32 SetWindowPos（非 dioxus task）。
10. chronicle-bar 静态渐变（真值 JS rAF 动态生成按红线不移植，转写层近似）。

## 4. 已验收事实（运行时证据在案，勿再诉；代码与证据矛盾才可 flag）

- 功能实测：宝石切换 inner/outer ✅、□ 最大化/还原 ✅、─ 最小化 ✅、✕ 整进程干净退出 ✅、
  主题两次翻转三窗同步 ✅、头像呼吸动画运行（两帧采样差异）✅、稳态 CPU ~0%（R9 轮）✅。
- 用户裁定（08-14 夜）：「主窗口满意」——视觉层已收口，本审不做视觉再裁。
- 门禁（R4-R9 每轮）：`cargo build -p northhing` exit 0；warnings 基线 lib 19 / bin 40 不增；
  `cargo test -p northhing ui_dioxus` ok（TRUTH_CSS 字节守卫过）；`pnpm run i18n:audit` = 154 持平。
- 编排者 08-15 Private-sum 复测（本 session 新证据）：t+15s 296.92 MB → t+3min 304.58 MB
  （主进程 ~14.1 MB + msedgewebview2 后代 9 个 ~282.8→290.5 MB；wv2 比 spike 多 1 个）。
  超出 spike 期预期带 219–245 MB，仍远低于 re-spike 报告的 500 MB 上限。**带外事实已知**，
  若代码侧存在可归因的内存浪费/泄漏形态请 flag；数值本身不作为 finding。

## 5. 重点质量核查点（编排者预检列出的高风险面，不限于此）

1. **Bug B 红线**：room 类组件零 sleep/timer `use_future`；事件流（Moved/Resized）→ 通道 →
   线程 的链路是否有回灌 dioxus task 系统的隐蔽路径（Poll 风暴前科）。
2. **follow 线程**：HWND 获取/缓存是否跨窗口重建漂移（ledger FYI「MainWindowHandle 漂移」前科）；
   SetWindowPos 调用线程安全性；Arc<Mutex<_>> 锁粒度与 poison 处理。
3. **panic 面**：UI 路径 unwrap/expect/panic 清单化核查（R3' 停摆史：RoomApp 渲染路径 unwrap
   致 runtime panic）；i18n 加载容错（65cd994）是否覆盖腐坏 ftl 全形态。
4. **编码纪律**：diff 内源文件零 BOM、零 mojibake、无 PowerShell 写入痕迹；ftl 三语键集对称
   （en-US/zh-CN/zh-TW 各 +65 行，键名逐一对齐）。
5. **依赖**：Cargo.toml 精确锁 `=0.8.0-alpha.1`；Cargo.lock 中 wry 0.55.1 / tao 0.35.3 /
   webkit2gtk 2.0.2 与 workspace tauri 侧一致；无 vendor/patch/私改版。
6. **flags 纪律**：committed flags.rs `DIOXUS_SHELL=false` + 回归测试在位（本 diff 不含 flags.rs，
   核 main.rs 分支逻辑即可）。
7. **死代码面**：`#[allow(dead_code)]` 使用清单（i18n 保留键等）是否逐项有注释依据。
8. **文件长度红线**：app.rs 739 / css.rs 398+新增 / windows.rs 615 / entry.rs 307 / i18n.rs 228
   —— 全线 <800 是否守住。

## 6. 报告要求

写 `.superpowers/sdd/consult-room/final-review-r3p-review.md`：
1. **Spec verdict**（PASS/FAIL）+ 逐条 constraints 核对表（状态+证据 file:line）；
2. **Quality verdict**（PASS/FAIL）+ findings 清单（Critical/Important/Minor/FYI，各附 file:line + 理由）；
3. ⚠️ Cannot-verify-from-diff 项单列（编排者逐条解决，不许含糊带过）；
4. 合并建议（CAN MERGE / NOT YET）。
