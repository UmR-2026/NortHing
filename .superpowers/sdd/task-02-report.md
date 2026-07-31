# Task 2 Report: Relay 协议层认证 + 房间生命周期原子化 + 资源限制

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 e3d0e53）
状态：**DONE**（check + 44 tests 全绿，3 轮重复运行稳定；未 git commit，由编排者统一提交）

## 改动清单（file:line）

全部改动位于 `src/crates/services/relay-core`，`src/apps/relay-server` 未改（DiskAssetStore 原样 re-export）。

### `src/lib.rs`
- L11：re-export `CreateRoomOutcome`（供 relay-server/集成测试用）。
- L143：`build_relay_router` 的 AppState 注入 `ws_idle_timeout: 90s`（embedded relay 与独立 relay 共用）。

### `src/relay/room.rs`
- L26：`MAX_CONNECTIONS = 512`（H-2 全局连接上限）。
- L32：`STALE_DESKTOP_AFTER_SECS = 90`（僵死判定：desktop 每 30s 心跳，90s = 漏 3 次；与 WS idle timeout 对齐）。
- L59：`DesktopConnection.tx` 由 `mpsc::UnboundedSender` → `mpsc::Sender`（bounded，配合 websocket 侧 `OUTBOUND_QUEUE_CAPACITY`）。
- L98：新增 `pub enum CreateRoomOutcome { Created, Conflict }`。
- L114：RoomManager 新增 `active_connections: AtomicUsize`。
- L136-157：`try_acquire_connection` / `release_connection` / `active_connection_count`（原子计数，>=512 拒收）。
- L184-250：`create_room` 重写为**单次 dashmap entry 操作内三态决策**（H-1 核心，详见设计决策）。
- L262：`send_to_desktop` 改用 `try_send`，队列满/通道关 → warn + 返回 false。
- L313：`on_disconnect` 保留房间 tombstone（`desktop = None`），不再立即删空房（TTL 清扫兜底），重连可接管。
- L324：`heartbeat` 锁序修正：先克隆 room_id 再锁 rooms（行为不变，防与 create_room 的 rooms→conn_to_room 锁序死锁）。
- 测试：`make_conn` 辅助（bounded channel + next_conn_id）+ 9 个新测试。

### `src/routes/api.rs`
- L44：AppState 新增 `ws_idle_timeout` 字段。
- L63：`AuthExtractor` 加 `#[derive(Clone)]`（测试内复用）。
- L262 / L331 / L410：`upload_web` / `check_web_files` / `upload_web_files` 增加 `auth: AuthExtractor` 参数并在入口 `auth.require(&state.api_key)?`（与 pair/command 同模式，C-1）。`api_key=None` 行为不变（embedded relay / dev）。
- 测试迁出：`#[cfg(test)] mod handler_tests;` 移入 `src/routes/api/handler_tests.rs`（L512 声明；api.rs 生产代码 512 行，整文件 812 → 拆分后达标 800 行规则）。新增 2 个上传认证测试。

### `src/routes/websocket.rs`
- L29：`MAX_WS_FRAME_SIZE = 8 MiB`（原 64 MiB，三处 max_message/max_frame/max_write_buffer 同值，注释含理由：encrypted_data 负载远小于此，HTTP command body 10 MiB 参照）。
- L34：`OUTBOUND_QUEUE_CAPACITY = 256`。
- L95-118：`websocket_handler` 加 `auth: AuthExtractor`：配置了 key 而缺失/错误 → 401 不 upgrade；`None` 放行（C-2/H-1）。随后 `try_acquire_connection()` 满 → 503；`on_failed_upgrade` 释放槽位。
- L120-176：`handle_socket` 读循环用 `tokio::time::timeout(ws_idle_timeout, next())`，超时 → debug 日志 + 关闭；teardown 顺序：`on_disconnect` → `release_connection` → `drop(out_tx)` → `write_task.abort()`（避免等死锁对端）。
- L180-245：`handle_text_message` 返回 bool；CreateRoom Conflict → `Error { message: "room already exists" }` 帧（协议已有，desktop 已处理）；出站队列满/关（慢消费者）→ warn + 断连，不阻塞读循环。
- 测试：raw TCP e2e（无 tower dev-dep，无法 `Router::oneshot`）。

## 三态语义的设计决策说明（H-1）

**问题**：旧 `create_room` 无条件 `rooms.remove(room_id)` 后重建——任何客户端可用已知 room_id 踢掉真实 desktop。

**决策**：整个决策+插入在**单次 `DashMap::entry()` 操作**（一把 shard 锁）内完成，check-then-act 不跨锁边界：

1. **Vacant**（房间不存在）→ 创建，返回 `Created`。
2. **Occupied + desktop 活跃**（`desktop.is_some()` 且 `now - last_heartbeat < 90s`）→ 返回 `Conflict`，**原房间与原 desktop 连接原样保留**（不 remove、不 disconnect）。
3. **Occupied + 断开/僵死**（tombstone 即 `desktop.is_none()`，或心跳超 `STALE_DESKTOP_AFTER_SECS`）→ 替换 desktop 接管，返回 `Created`，`info!` 日志含 room_id。

**设计要点**：
- 活跃判定用 rooms entry 内现有标记（`desktop` 存活位 + `last_heartbeat`），满足 brief"实现方式自选"（判定与插入同锁）。`on_disconnect` 的 tombstone 机制仓库本就有，未另加标记。
- 客户端自选 room_id 的合法重连路径（`relay_client.rs:261-269` 断线重建）走分支 3 接管；**不拒绝任何客户端指定 ID**（brief 兼容约束）。
- 僵死窗口 90s 与 WS idle timeout 90s 对齐：僵尸 TCP 连接两路都会收掉；活跃 desktop 心跳 30s，正常情况永远进不了接管分支。
- 保持 Vacant 分支内 `conn_to_room` 索引更新也在 shard 锁保持期完成（`_room_ref` 占位），conn 索引与 rooms 同步原子。
- 重复 CreateRoom 的 Conflict 只回 Error 帧，连接保持，不踢。

**未做**（brief 明确不做）：capability token 体系（记终审 triage）、CreateRoom 协议结构不变、RelayResponse/RelayCommand 转发/心跳协议/TTL cleanup 不动、upload 验证转换不动、GET catch-all 归 Task 3。

## 测试清单与命令输出

验证命令（brief 指定，workspace 级命令被 embed-resource 链阻断，未用）：

```
cargo check -p northhing-relay-core -p northhing-relay-server
cargo test -p northhing-relay-core -p northhing-relay-server
```

结果（3 轮重复运行，无 flake）：

```
cargo check: Finished dev profile — 0 warnings, 0 errors
cargo test : 37 passed; 0 failed (northhing-relay-core)
             7 passed; 0 failed (northhing-relay-server)
```

### 新增测试清单

**room.rs（9 个）**
- `create_room_fresh_room_succeeds`：新建 → Created + 注册 desktop + conn 索引。
- `create_room_conflict_keeps_original_room_and_desktop`：活跃 desktop 重复 ID → Conflict，原房间/原 desktop 不动，入侵者无登记。
- `create_room_takes_over_after_disconnect`：断开 tombstone 后同 ID 接管成功（重连路径）。
- `create_room_takes_over_stale_heartbeat_connection`：心跳超窗僵死 → 接管成功。
- `create_room_conflicts_with_recently_active_desktop`：窗口内（未超时）仍判活跃 → Conflict。
- `send_to_desktop_fails_fast_when_queue_full` / `send_to_desktop_delivers_on_bounded_queue`：bounded 队列满 → false（不阻塞）；正常 → 送达。
- `connection_slot_counter_increments_and_decrements` / `connection_limit_rejects_admits_at_capacity`：上限计数增/减、满 512 拒收。

**websocket.rs（8 个，raw TCP e2e）**
- `websocket_upgrade_requires_api_key_when_configured`（无 key → 401 不升级）/ `websocket_upgrade_rejects_wrong_api_key`（错 key → 401）/ `websocket_upgrade_allows_configured_api_key`（对 key → 101）/ `websocket_upgrade_open_when_api_key_unset`（None → 101）。
- `idle_socket_is_closed_after_timeout_and_slot_released`（200ms idle，静默客户端被关，槽位回 0）。
- `slow_consumer_full_queue_signals_disconnect_without_deadlock`（队列 1 预填满 → 断连分支，房间仍建，不死锁）。
- `healthy_queue_delivers_replies`（RoomCreated 帧回达）。
- `duplicate_create_room_sends_room_exists_error`（重复 CreateRoom → "room already exists" 帧，原房间原 desktop 保留）。
- `truncate_preview`（原有）。

**api.rs handler_tests（+2 认证测试，原 6 个保留）**
- `upload_routes_reject_missing_api_key_when_configured`：三个上传路由无 key → 401（先于任何状态触达）。
- `upload_routes_accept_valid_api_key_and_stay_open_when_unset`：正确 key 通过；`api_key=None` 保持开放。

## 状态

- 需求 1（C-1 上传认证）、2（C-2/H-1 WS 握手认证）、3（H-1 三态原子化）、4（H-2：8 MiB 帧限 + bounded 队列 + 512 连接上限 + 90s idle 超时）、5（测试全覆盖，AGENTS 并发规则）全部完成。
- 格式化：用 `cargo fmt -p northhing-relay-core -p northhing-relay-server`（brief 允许项）。**未用 `pnpm run fmt:rs`**：其脚本会收集 workspace 所有已变更 .rs 文件并可能 `git restore` 非目标 collateral 文件——当前 worktree 存在大量外来改动，跑它会毁掉那些改动（详见遗留疑虑）。
- 行数：api.rs 512 / websocket.rs 536 / room.rs 640 / handler_tests.rs 297，全部 < 800。
- 未 git commit（编排者统一提交）。

## 遗留疑虑

1. **worktree 非我产生的改动（未触碰）**：`git status` 显示整个 workspace 有大量修改（覆盖 ai-adapters/cli/desktop 等，含 `services-integrations/src/remote_connect/relay_client.rs`、`tests/pairing_qr_relay.rs`），疑似一次裸 `cargo fmt` 全 workspace 格式化残留。这些不是我产生、与本任务无关，我未触碰；提交时需与它们隔离（只挑 relay-core 相关文件）。
2. 僵死接管窗口：90s 是"漏 3 次心跳"的推断（30s 心跳 × 3）；若桌面端网络抖动 >90s 且连接未断，新连接可接管其房间。属预期行为，已注释说明。
3. 连接上限拒绝码用 503（upgrade 前直接响应）；brief 允许 426/503 或 upgrade 前拒绝，此处选了 503。
4. 无 capability token 体系——完整方案按 brief 记终审 triage（房间劫持在认证+原子化后仅剩已认证 peer 间的 ID 猜测面，配合随机 room_id 生成已显著收窄）。
5. `heartbeat` 锁序修正为内部实现细节（行为等价），未走协议变更，已在改动清单披露。
6. `api.rs` 测试模块迁至 `src/routes/api/handler_tests.rs` 是"顺手清配额"（housekeeping 规则 1）：拆分 812 行文件以符合 800 行 review pressure 规则。
7. `ensure_room` 测试辅助传 conn_id 0（next_conn_id 从 1 起），与真实连接 id 空间不重叠，仅测试用。
