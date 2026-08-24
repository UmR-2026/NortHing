# Task B5 Brief — relay 批 (relay-core + relay-server)

## 1. 任务定位与范围
- **目标分支与工作区**: `E:\agent-project\northing\.worktrees\wave2-relay`
- **允许修改的文件集**:
  - `src/crates/services/relay-core/`
  - `src/apps/relay-server/`
  - `.superpowers/sdd/task-b5-report.md` (报告)
- **严格禁区**:
  - 严禁修改 `src/crates/assembly/core/`
  - 严禁修改 `src/crates/services/services-integrations/`
  - 严禁修改 `src/apps/desktop/`
  - 严禁修改 `memory/`、`.graph/`、`frontend-redesign-*`、`growth` 线文件

## 2. 需求与要点明细 (7 项)

### 2.1 T1 Q-3 & T1 Q-4: validated.rs 规范化与清理
- **位置**: `src/crates/services/relay-core/src/validated.rs:162-182`
- **T1 Q-4**: 将 `validate` 中原有的两个分别循环 `normalized.split('/')`（前缀/盘符检查与 `"."` CurDir 检查）合并为单一 split 循环。
- **T1 Q-3**: 移除 `Component::Normal(part)` 中冗余的 `is_drive_letter(part)` 检查（该情况已在前置 split 循环中完全覆盖），并修正/清理过时或误导的注释。

### 2.2 T1 M-4: disk_tests 测试名实对齐与补齐
- **位置**: `src/apps/relay-server/src/lib.rs` (disk_tests)
- **现象**: `map_to_room_preserves_existing_dest_on_validation_failure` 实际测试的是写入后用新内容覆盖目标文件。
- **要求**:
  1. 将该测试改名为名实相符的名称（例如 `map_to_room_overwrites_existing_dest_with_new_content`）；
  2. 补齐一个真正的验证失败/非法参数下现有目标文件保持不变且不被损坏的单元测试（例如 `map_to_room_preserves_existing_dest_on_validation_failure`）。

### 2.3 T2 M-2: WebSocket 连接槽 RAII guard 机制 (融入插件化可逆回收)
- **位置**:
  - `src/crates/services/relay-core/src/relay/room.rs`
  - `src/crates/services/relay-core/src/routes/websocket.rs`
- **根因与要求**:
  - 原 `handle_socket` 若发生 panic / 异常中止，不会执行 `release_connection()`，导致连接槽泄漏。
  - 在 `relay-core` 内设计并实现局部的 `ConnectionSlotGuard`（**仅在 relay-core 内部实现，不抽取通用 crate**）。
  - `RoomManager::try_acquire_connection` 在成功时返回 RAII Guard（例如 `Option<ConnectionSlotGuard>`），Guard 在 `Drop` 时自动扣减 `active_connections`（调用 `release_connection` 或原子递减）。
  - `websocket_handler` 获取 Guard 后转移给升级后的 socket 处理任务 `handle_socket(socket, state, guard)`。
  - 若升级失败（`on_failed_upgrade`），Guard 被 drop 自动释放；若 `handle_socket` 正常结束、返回或发生 panic，Guard 都会在作用域结束时自动执行 Drop 释放，确保连接槽绝对不泄漏。
  - 增加针对 Guard 的 Drop 自动释放和 panic 安全性的单元测试。

### 2.4 T2 M-3: handle_text_message 返回风格统一
- **位置**: `src/crates/services/relay-core/src/routes/websocket.rs:180-245`
- **要求**: 统一 `handle_text_message` 的控制流与返回值书写风格，使各 match 分支/错误分支具有一致清晰的返回表达，消除不一致的混合代码风格。

### 2.5 T3 M-1: is_genuine_traversal 锚点注释与防漂移
- **位置**: `src/apps/relay-server/tests/e2e_web_assets.rs:225-247`
- **要求**: 在 `is_genuine_traversal` 助手函数处添加明确的行号/逻辑锚点注释，指向 `northhing_relay_core::routes::api::serve_room_web_catchall` 的实际路径拆分实现（line ~467-471），明确记录此测试镜像关系，防止后续 handler 改动与测试判定发生逻辑漂移。

### 2.6 FR-3: 补充 api_key=None 全路由 e2e 测试 (闭合 Gap 2)
- **位置**: `src/apps/relay-server/tests/e2e_web_assets.rs`
- **要求**:
  - 新增 1 个完整的 e2e 测试用例（例如 `e2e_open_relay_when_api_key_none_accepts_all_routes_without_auth`），基于 `setup(None)` 启动无 API Key 的 full router。
  - 测试在不提供 `x-api-key` 请求头的情况下：
    1. WebSocket 握手成功；
    2. 上传文件接口成功；
    3. 静态文件访问接口成功。
  - 完整闭合 final-review §6 Gap 2。

## 3. 执行与代码纪律
1. **工作区与文件操作**:
   - 所有代码修改必须落在 `E:\agent-project\northing\.worktrees\wave2-relay` 内。
   - 所有改动必须通过 git commit 提交在当前分支 `fix/wave2-relay`。
2. **格式与规范**:
   - **禁止执行裸 `cargo fmt`**，所有代码格式手动对齐。
   - 日志与注释全部使用英文（English-only），严禁出现 emoji。
   - 生产代码单一文件行数保持在 800 行以内。
3. **验证命令**:
   - 必须使用 MSVC 工具链实跑：
     `& "C:\Users\UmR\.cargo\bin\cargo.exe" +stable-x86_64-pc-windows-msvc test -p northhing-relay-core -p northhing-relay-server`
   - 必须实跑全套测试，在报告中逐字粘贴真实的命令输出，严禁编造或推测。
4. **交付物报告**:
   - 写入 `E:\agent-project\northing\.worktrees\wave2-relay\.superpowers\sdd\task-b5-report.md`。
   - 报告需逐项说明 T1 Q-3, T1 Q-4, T1 M-4, T2 M-2, T2 M-3, T3 M-1, FR-3 的实现详情、提交 commit hash 及完整测试输出。
