# Review: 2026-06-26 Manual Test Review — v0.1.0 Frontend Onboarding

> **Reviewer**: Code审查员（AI）
> **Review Date**: 2026-06-26
> **Scope**: 3 commits (`97890b2`, `748f628`, `08420b0`)
> **Basis**: handoff doc + `git diff` + 单元测试验证 + Slint 端源码对照

---

## 审查结论

**APPROVE WITH OBSERVATIONS** — 两个 bug-fix commits 逻辑正确，测试全绿。但发现一个未被 handoff 提及的 latent bug（Phase C.3 / G.2 / P0-A 同样存在非 UI 线程调用 Slint setter 的问题），建议在下一个 cleanup pass 中一并修复。

---

## 一、Bug A（`97890b2`）审查

### 修复内容

Slint 端 `main.slint:219` 声明了 `open-settings => { root.open-settings(); }`，但 Rust 端没有注册 `ui.on_open_settings(...)` handler，导致点击侧边栏"设置"无反应。

修复：在 `app_state/mod.rs:1540` 添加 5 行 handler，调用 `ui.set_current_route("settings")`。

### 审查意见：✅ 逻辑正确，有一个风格观察

**正确性**: ✅ `ui_weak_open_settings` 正确使用 `upgrade()` 检查 UI 存活，避免 use-after-free。

**一个小观察（非阻塞）**: `close-settings` 在 Slint 端直接处理（`close-settings => { root.current-route = "main"; }`），不需要 Rust handler。对称地，`open-settings` 也可以直接在 Slint 端设 `current-route`（省去 callback 往返）：

```slint
// main.slint — 可以改为纯 Slint 处理（非阻塞建议）
open-settings => { root.current-route = "settings"; }
```

如果改为纯 Slint 处理，就可以删除 `callback open-settings();` 声明和 Rust handler，减少一个 FFI 边界。但当前修复也是正确的，只是多一层间接调用。

**结论**: ✅ 可以合并，上述观察留给后续 cleanup。

---

## 二、Bug B（`748f628`）审查

### 修复内容

`create_ui` 中后台线程直接调用 `ui.set_current_route("welcome")`，Slint 1.16 在非 event-loop 线程上静默丢弃属性修改。修复：用 `slint::invoke_from_event_loop` 将 setter 派发到 UI 线程。

### 审查意见：✅ 修复正确，但发现同类 latent bug 三处

**修复本身**: ✅ 正确使用了 `invoke_from_event_loop`，pattern 与 P0-A startup session thread 中的 `slint::invoke_from_event_loop` 用法一致。

**⚠️ 新发现 — 同类 bug 仍在代码中（非本次 commit 引入，但是同 pattern）**:

在 `app_state/mod.rs` 中，以下三处同样在后台线程直接调用 Slint setter，**没有** `invoke_from_event_loop`：

### Latent Bug 1：Phase C.3 model-status refresh（~行 479）

```rust
// app_state/mod.rs — Phase C.3
std::thread::spawn(move || {
    let rt = ...;
    rt.block_on(async move {
        let Some(ui) = ui_weak_provider.upgrade() else { return; };
        let status = build_model_status_string().await;
        ui.set_model_status(SharedString::from(status));  // ❌ 直接在后台线程调用
    });
});
```

### Latent Bug 2：Phase G.2 mcp-status refresh（~行 519）

```rust
// app_state/mod.rs — Phase G.2
std::thread::spawn(move || {
    let rt = ...;
    rt.block_on(async move {
        let Some(ui) = ui_weak_mcp.upgrade() else { return; };
        let status = build_mcp_status_string().await;
        ui.set_mcp_status(SharedString::from(status));  // ❌ 直接在后台线程调用
    });
});
```

### Latent Bug 3：P0-A startup session（~行 1630）

```rust
// app_state/mod.rs — P0-A
rt.block_on(async move {
    // ...
    if let Some(ui) = ui_weak_startup.upgrade() {
        ui.set_current_session_id(SharedString::from(sid.clone()));  // ❌
        refresh_sessions_ui(&ui, &sid).await;                     // ❌ 内部也调用 ui setter
    }
});
```

**影响评估**:
- Phase C.3 / G.2：model-status 和 mcp-status 显示可能不更新（静默失败），用户看到过期状态
- P0-A：首次启动的 auto-create session 可能不刷新 sidebar（但 session 本身已创建，只是 UI 不更新）

**建议**: 在下一个 cleanup pass 中，将这三处也改为 `invoke_from_event_loop` 包裹，与 Bug B 的修复保持一致。

---

## 三、测试验证

### 单元测试

| 测试套件 | 结果 | 备注 |
|---|---|---|
| `cargo test -p northhing --lib` | ✅ **40/40 passed** | 4 个 warnings（unused variables/dead code），不影响正确性 |
| `cargo test -p northhing-relay-server --lib` | ✅ **3/3 passed** | — |

### 手动测试

handoff 中报告 1/56 手动测试通过（其余被自动化工具限制阻塞），2 个 bug 已修复。

**验证建议**: 如 handoff 所述，人类后续 pass 应覆盖 W-01（点击侧边栏"设置"确认 SettingsView 渲染）。

---

## 四、Observations（非阻塞）

### O1：`pnpm-lock.yaml` 的 pre-existing diff noise

handoff 中已提及，`pnpm-lock.yaml` 有 5158 行 diff noise。建议在独立 `chore:` commit 中处理，保持 working tree 清洁。

### O2：`session_metadata` 的 desktop-side cache

`AppState::session_metadata: Mutex<HashMap<String, SessionMeta>>` 是 Phase 5 wire-up 的临时缓存。当 `core` 将 `provider_id` / `workspace_path` 添加到 `SessionSummary` 后，可以移除这个缓存。代码中已有 doc-comment 记录这个迁移路径，无需现在行动。

### O3：Slint padding warning

`cargo test` 输出中有多个 Slint padding warning（`padding only has effect on layout elements`）。这些是 `IdentityCreatorView.slint`、`WelcomeView.slint`、`SidebarView.slint` 等文件中的 padding 属性用在了非 layout 元素上。不影响功能，但是噪声。建议后续 cleanup 中检查并移除无效的 padding 声明。

---

## 五、决策

**APPROVE** — 两个 bug-fix commits 都是正确的最小修复，测试全绿，可以合并。

**一个条件建议（非阻塞）**: 在合并后的下一个 cleanup pass 中，修复三处 latent bug（Phase C.3 / G.2 / P0-A 的非 UI 线程 Slint setter 调用），与 Bug B 的修复保持一致性。

---

## 六、下一步行动（按优先级）

### 立即（合并前，可选）

- 无阻塞项，可以立即合并

### 短期（合并后 1 周内）

1. **修复 Phase C.3 / G.2 / P0-A 的 latent bug**
   - 用 `slint::invoke_from_event_loop` 包裹 `set_model_status`、`set_mcp_status`、`set_current_session_id` 的调用
   - 验证修复：在首次启动场景下，确认 model-status / mcp-status 正确更新

2. **考虑简化 `open-settings` 为纯 Slint 处理**
   - 将 `main.slint:219` 改为 `open-settings => { root.current-route = "settings"; }`
   - 删除 `callback open-settings();` 和 Rust handler
   - 这是风格改进，不阻塞当前合并

### 长期（1 个月内）

3. **清理 `pnpm-lock.yaml` diff noise**
   - 独立 `chore:` commit，重新生成 lockfile

4. **修复 Slint padding warnings**
   - 检查 `IdentityCreatorView.slint`、`WelcomeView.slint`、`SidebarView.slint` 中的 padding 用法
   - 移除无效的 padding 声明

---

## 七、审查方法

1. **读 handoff 文档** — 理解 bug 背景和修复范围
2. **看 `git diff`** — 确认修复的最小性和正确性
3. **对照 Slint 源码** — 验证 callback 声明和 Rust handler 的对称性
4. **搜索同类 pattern** — 发现 Phase C.3 / G.2 / P0-A 的 latent bug
5. **跑单元测试** — 验证 40/40 + 3/3 全绿
6. **写 review report** — 按项目命名习惯保存到 `docs/handoffs/`

---

**审查完成时间**: 2026-06-26 16:49 (GMT+8)  
**审查耗时**: ~45 分钟  
**发现总问题数**: 1 个新 latent bug（3 处），2 个观察项
