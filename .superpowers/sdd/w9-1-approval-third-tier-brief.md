# Task Brief — W9-1: 确认门补"本会话内允许"第三档 + approval 卡片抽离

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/desktop` 仅桌面 crate。
来源：校准裁决（`docs/product/requirements-vs-current-2026-08-29.md` §五 W9-1）+ 论题原则 7（确认门默认开启）。

## 现状（编排者已核实）

- `app.rs:731-792`：approval 卡片已有「允许/拒绝」两按钮，`handle_action` 闭包走 `api::respond_to_tool_confirmation(&cid, approved)`，resolved 态切换正常。
- `api.rs:136-138`：`respond_to_tool_confirmation(tool_id, approved, None)`——**第三参数 None 是留口**，先读 `kernel_facade` 的 `respond_to_tool_confirmation` 真实签名（第三参类型与语义，疑为 remember/scope），report 引用 file:line。
- `app.rs` 当前 805 行，manifest ceiling 805——**零余量**。本任务改动若净增行数必然越线 → 强制走 §2 抽离。

## Spec（验收标准）

### 1. 第三档按钮

approval 卡片加第三按钮「本会话内允许」：
- 点击 → 调 facade 第三参携带"本会话记住"语义（按实际签名传值）→ 本会话内同类/同工具后续调用不再弹确认（以此后端真实语义为准，report 写清到底是"同工具"还是"同调用指纹"）
- 若 facade 第三参并非 remember 语义 → STOP，NEEDS_CONTEXT 上报实际签名
- UI 文案中文，按钮样式复用既有 btn-approve/btn-reject 风格族（css.rs 余量 0——优先复用既有 class，必须新增时用组件内联 style 并在 report 说明）

### 2. approval 卡片抽离（解决 app.rs 零余量）

- `MockEntry::Approval` 渲染分支（app.rs:731-792）整体抽到新文件 `src/apps/desktop/src/ui_dioxus/approval_card.rs`，app.rs 只留调用点。
- 收口后 app.rs 应 <800 → **删除 manifest 里 `god_file:...app.rs` 条目**（回归通用 800 线保护），同 commit；若仍 ≥800 → ceiling 下调到实测值。
- 抽离纯位移，行为零变化。

### 3. 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing`：0 error，warnings ≤44 基线
2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing --lib`：全绿
3. `node scripts/verify-rot-budget.mjs`：绿（app.rs 条目处置正确）
4. 截图：触发一个审批卡（或构造演示态），三按钮布局截图存 `.superpowers/sdd/w9-1-shot-1.png`（不 commit）

## Global Constraints

1. 分层边界：只动 `src/apps/desktop`（+ manifest 处置）。
2. 日志纪律：英文无 emoji；本任务零新增日志。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止整树 git 操作（restore/checkout/stash），只许点名文件 add/commit。
4. rot-budget：ceiling 只降不升/清条目。
5. 验证最小集：上述 4 条；report 写入 `.superpowers/sdd/w9-1-approval-third-tier-report.md`（write 工具）。
6. commit 规则：恰好一个 commit；不含 `.superpowers/`。
7. 不新建无 owner 抽象。
8. 行为变化仅限：新增第三按钮 + 卡片抽离位移。
9. 遇编译错误先加载对应 rust skill，禁止无脑 clone/unwrap 糊编译器。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；需要澄清 = NEEDS_CONTEXT。
- 返回消息含：状态 / commit SHA / git show --stat / 验证输出尾部 / app.rs 新行数 / 截图路径 / 偏离清单。
- 假汇报 = 停用：编排者将用磁盘 diff + 读截图逐条核对。
