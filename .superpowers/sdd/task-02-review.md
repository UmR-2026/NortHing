# Task 2 Review: Relay 协议层认证 + 房间原子三态 + 资源限制

**审查范围**: `e3d0e53..5971c54`（1 commit，+1037/-282，5 文件）
**审查对象**: implementer 最终 diff（fixer report 仅作背景，以 diff 为准）
**commit**: `5971c54 fix(relay): 协议层认证 + create_room 原子三态 + 资源限制 (C-1/C-2/H-1/H-2)`

---

## 一、Spec 合规判决：**PASS**

### 逐项核对 brief §1-§5

| § | 需求 | 落点 | 状态 |
|---|---|---|---|
| §1 C-1 | 上传路由认证（`upload_web`/`check_web_files`/`upload_web_files` 加 `auth: AuthExtractor`，`api_key=None` 放行） | `api.rs:262` / `:331` / `:410` 三处 `auth.require(&state.api_key)?`；handler_tests `upload_routes_reject_missing_api_key_when_configured` (api/handler_tests.rs:216) 与 `upload_routes_accept_valid_api_key_and_stay_open_when_unset` (L267) 覆盖 | ✅ |
| §2 C-2/H-1 | WS 握手认证（`AuthExtractor` from request parts，配置 key 但无 key/错 key → 401 不 upgrade；`None` 放行） | `websocket.rs:95-100`：`auth.require().is_err() → StatusCode::UNAUTHORIZED.into_response()`；测试 4 个（缺/错/对/None）raw TCP 握手均过（`auth_require_gates_only_when_key_configured` L319 / `_requires_api_key_when_configured` L347 / `_rejects_wrong_api_key` L356 / `_allows_configured_api_key` L366 / `_open_when_api_key_unset` L376） | ✅ |
| §3 H-1 | create_room 三态决策（单次 dashmap entry 内，活跃→拒绝；僵死→接管；不删原房间/原 desktop） | `room.rs:206-254` 整个 `match self.rooms.entry(...)` 块，Vacant/Occupied+alive→Conflict/Occupied+tombstone→takeover 三分支在同一 shard 锁内完成；`create_room_conflict_keeps_original_room_and_desktop` (L477) 断言原 desktop.conn_id 不变、入侵者未注册 | ✅ |
| §3 H-1 | 「不可跨 await/锁边界」check-then-act | `match self.rooms.entry(...)` 是 DashMap `Entry` API（持 shard 锁）；occupied 分支 `occupied.get_mut()` 同步取数据；tombstone 判定只用 `room` entry 内部字段（`desktop.is_some()` + `last_heartbeat`），不跨第二把锁 | ✅ |
| §3 H-1 | 重复创建不删原房间、不断原 desktop | Conflict 分支 `return CreateRoomOutcome::Conflict` 在任何修改前；无 `rooms.remove()`；take-over 分支只覆盖 `room.desktop` 字段 | ✅ |
| §3 H-1 | 客户端自选 ID 兼容 desktop 重连 | 重连路径走 Occupied+tombstone 分支（`relay_client.rs:262` `CreateRoom { room_id: Some(原id) }`）；测试 `create_room_takes_over_after_disconnect` (L502) 与 `create_room_takes_over_stale_heartbeat_connection` (L532) 覆盖；`conn_to_room` 未被本任务修改，重连语义不变 | ✅ |
| §4 H-2 | WS 帧上限 8 MiB（三处） | `websocket.rs:27` `MAX_WS_FRAME_SIZE = 8 MiB`；L110-112 `max_message_size/max_frame_size/max_write_buffer_size` 三处同值；注释说明「encrypted_data 负载远小于此，HTTP command body 10 MiB 参照」 | ✅ |
| §4 H-2 | 出站队列 bounded(256) + `try_send` 失败断连 | `websocket.rs:33` `OUTBOUND_QUEUE_CAPACITY = 256`；L120 `mpsc::channel(256)`；`send_json` L531 用 `try_send` 返回 bool；`handle_text_message` 返回 bool，false → read loop `break` + warn「Disconnecting slow consumer」（L142）；测试 `slow_consumer_full_queue_signals_disconnect_without_deadlock` (L425) 验证队列 1 填满→返回 false→房间仍建→不死锁 | ✅ |
| §4 H-2 | 全局连接上限 512，>= 拒绝 503 | `room.rs:26` `MAX_CONNECTIONS = 512`；L136-148 `try_acquire_connection` 原子检查+回滚；`websocket.rs:104-107` upgrade 前调用，满 → `SERVICE_UNAVAILABLE`；测试 `connection_limit_rejects_admits_at_capacity` (L617) | ✅ |
| §4 H-2 | 释放对称（含 on_failed_upgrade） | `websocket.rs:113-115` `.on_failed_upgrade(move \|_\| manager.release_connection())`；handle_socket 收尾 L162-170：on_disconnect → release_connection → drop(out_tx) → write_task.abort() → await；测试 `connection_slot_counter_increments_and_decrements` (L600) 与 `idle_socket_is_closed_after_timeout_and_slot_released` (L394) 覆盖 acquire/release 对称 | ✅ |
| §4 H-2 | idle 超时 90s，tokio::time::timeout 包装 next() | `websocket.rs:138-140` `tokio::time::timeout(state.ws_idle_timeout, ws_receiver.next())`；超时分支 debug 日志 + break（L158-160）；`build_relay_router` L143 注入 90s 默认值；测试用 200ms 验证：`idle_socket_is_closed_after_timeout_and_slot_released` (L394) | ✅ |
| §5 测试 | 三态 + WS 认证 + 上传认证 + bounded 队列 + 连接计数 | 9 个 room.rs 测试（L460-641）+ 8 个 websocket.rs 测试（L246-510）+ 2 个 api/handler_tests.rs 测试（L216/267），全部覆盖 brief §5 清单 | ✅ |
| §5 测试 | tokio::select!/取消/超时竞态配套测试（AGENTS.md 规则 4） | `idle_socket_is_closed_after_timeout_and_slot_released` 覆盖 timeout；`slow_consumer_full_queue_signals_disconnect_without_deadlock` 覆盖 bounded；`on_failed_upgrade` 路径虽无独立测试，但 `connection_slot_counter_increments_and_decrements` 验证增/减对称 | ✅ |

### 「明确不做」核对

- ✅ CreateRoom 协议结构未变（`websocket.rs:200-216` 仍 `room_id: Option<String>`，`unwrap_or_else(generate_room_id)`；`relay_client.rs:46/262/373` 无 diff）
- ✅ 未加 capability token 体系（记终审 triage）
- ✅ RelayResponse/RelayCommand 转发逻辑（`websocket.rs:222-230`）未动
- ✅ 心跳协议（`Heartbeat` / `HeartbeatAck` 帧 L231-246）未动
- ✅ TTL cleanup（`cleanup_stale_rooms` L347）未动 — 只微调内部锁序（已在改动清单披露，记 §遗留疑虑 5）
- ✅ upload 的 ValidatedRelPath/ContentHash 转换未动（Task 1 已定格，仅在 handler 入口加 auth gate）
- ✅ GET catch-all 未动

### 验证命令实测

```
$ cargo check -p northhing-relay-core -p northhing-relay-server
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.67s

$ cargo test -p northhing-relay-core
running 37 tests
test routes::websocket::tests::auth_require_gates_only_when_key_configured ... ok
test relay::room::tests::connection_slot_counter_increments_and_decrements ... ok
test relay::room::tests::connection_limit_rejects_admits_at_capacity ... ok
test relay::room::tests::create_room_conflicts_with_recently_active_desktop ... ok
test relay::room::tests::create_room_takes_over_stale_heartbeat_connection ... ok
test relay::room::tests::send_to_desktop_fails_fast_when_queue_full ... ok
test relay::room::tests::send_to_desktop_delivers_on_bounded_queue ... ok
test relay::room::tests::create_room_conflict_keeps_original_room_and_desktop ... ok
test relay::room::tests::create_room_takes_over_after_disconnect ... ok
test relay::room::tests::create_room_fresh_room_succeeds ... ok
test routes::websocket::tests::healthy_queue_delivers_replies ... ok
test routes::websocket::tests::duplicate_create_room_sends_room_exists_error ... ok
test routes::websocket::tests::slow_consumer_full_queue_signals_disconnect_without_deadlock ... ok
test routes::websocket::tests::websocket_upgrade_rejects_wrong_api_key ... ok
test routes::websocket::tests::websocket_upgrade_requires_api_key_when_configured ... ok
test routes::websocket::tests::websocket_upgrade_allows_configured_api_key ... ok
test routes::websocket::tests::websocket_upgrade_open_when_api_key_unset ... ok
test routes::websocket::tests::idle_socket_is_closed_after_timeout_and_slot_released ... ok
test routes::api::handler_tests::upload_routes_reject_missing_api_key_when_configured ... ok
test routes::api::handler_tests::upload_routes_accept_valid_api_key_and_stay_open_when_unset ... ok
... (Task 1 validated 测试 + 既有 relay/room tests 全保留)
test result: ok. 37 passed; 0 failed; 0 ignored
```

37/37 全绿。Spec §5 全部满足。

---

## 二、代码质量判决：**PASS**

### 关键安全/正确性深度核对

#### 1. 三态决策的单锁原子性（H-1 核心）

**核验**：`room.rs:206-254` 整个 `match self.rooms.entry(room_id.to_string()) { ... }` 在 DashMap `Entry` API 的同一 shard 锁内完成：

```rust
match self.rooms.entry(room_id.to_string()) {
    Entry::Vacant(vacant) => {
        let now = Utc::now().timestamp();
        let mut room = RelayRoom::new(room_id.to_string());
        room.desktop = Some(DesktopConnection { ... });
        let _room_ref = vacant.insert(room);
        self.conn_to_room.insert(conn_id, room_id.to_string());  // 在 shard 锁保持期内
        info!("...");
        CreateRoomOutcome::Created
    }
    Entry::Occupied(mut occupied) => {
        let room = occupied.get_mut();
        let now = Utc::now().timestamp();
        let desktop_alive = room.desktop.as_ref()
            .is_some_and(|d| now - d.last_heartbeat < STALE_DESKTOP_AFTER_SECS);
        if desktop_alive {
            warn!("...rejecting...");
            return CreateRoomOutcome::Conflict;  // 早返回，无修改
        }
        room.desktop = Some(DesktopConnection { ... });  // 接管
        room.last_activity = now;
        self.conn_to_room.insert(conn_id, room_id.to_string());
        info!("...taken over...");
        CreateRoomOutcome::Created
    }
}
```

**判定不跨 await/不跨第二锁**：
- Vacant/Occupied 决策只读 `room.desktop`（rooms entry 内字段）与 `last_heartbeat`（同 entry 内字段），不查 `conn_to_room`
- 决策 + 写入同一 match arm，无 await 点
- conn_to_room 更新在 shard 锁保持期内，但 conn_to_room 自身的 insert 只持自己的 shard 锁

**僵死判定窗口**：与 `STALE_DESKTOP_AFTER_SECS = 90`（room.rs:32，对齐 desktop 30s 心跳 × 3 漏失 + WS idle 90s）— 与 brief §4 要求 90s 一致。

#### 2. Conflict 分支原样保留（H-1）

**核验**：`room.rs:233-238` Conflict 分支在 `desktop_alive=true` 时立即 `return CreateRoomOutcome::Conflict`，**在分支体内不修改任何 `room` 字段、不调用 `self.rooms.remove()`、不向原 desktop.tx 发任何消息**。原房间与原 desktop `Arc<RoomManager>` 引用完整保留。

测试 `create_room_conflict_keeps_original_room_and_desktop`（L477-501）显式断言：
- 原 desktop `room.desktop.as_ref().unwrap().conn_id == conn_orig`（L495）
- `!manager.conn_to_room.contains_key(&conn_intruder)`（L499，入侵者未注册）
- `manager.conn_to_room.contains_key(&conn_orig)`（L500，原 conn 仍在索引）

#### 3. WS 认证在 upgrade 前拒绝（C-2）

**核验**：`websocket.rs:95-100`：
```rust
pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>, auth: AuthExtractor) -> Response {
    if auth.require(&state.api_key).is_err() {
        warn!("Rejected WebSocket upgrade: missing or invalid API key");
        return StatusCode::UNAUTHORIZED.into_response();
    }
    ...
}
```

Auth 检查在函数最前，**先于** `try_acquire_connection`（L104）和 `on_upgrade` 回调（L117）。失败路径不分配 conn_id、不消耗连接槽位。

#### 4. bounded 队列慢消费者不死锁（H-2）

**核验**：`websocket.rs:120` `mpsc::channel(256)`（替换原 unbounded）；L268-271 write_task 持续 `out_rx.recv().await`；L130-166 读循环调 `handle_text_message` 返回 false → `break`，**不阻塞**。

测试 `slow_consumer_full_queue_signals_disconnect_without_deadlock`（L425-462）：构造 channel(1) 预填满 → `handle_text_message` 返回 false（`tx.try_send` 失败）→ 房间已建但后续读循环断连 → 无死锁。

#### 5. 连接计数对称（H-2）

**acquire/release 路径表**：

| 路径 | acquire | release |
|---|---|---|
| 认证失败 | ✗（L98 早返回，无 acquire） | N/A |
| 超 512 拒绝 | ✗（L105 早返回，无 acquire） | N/A |
| upgrade 失败（客户端断 / 不支持的 upgrade） | ✓ L106 | `on_failed_upgrade` L113-115 |
| upgrade 成功 → 正常关闭（Close/Error） | ✓ L106 | L162 `release_connection` |
| upgrade 成功 → idle 超时 | ✓ L106 | L162 `release_connection` |
| upgrade 成功 → 慢消费者断连 | ✓ L106 | L162 `release_connection` |
| upgrade 成功 → handle_socket 任务 panic | ✓ L106 | ⚠ **未测，可能泄漏**（见 Minor-1） |

测试 `connection_slot_counter_increments_and_decrements`（L600）+ `idle_socket_is_closed_after_timeout_and_slot_released`（L394）覆盖正常增/减与 idle 释放。

#### 6. 锁序无死锁

| 函数 | 锁顺序 | 备注 |
|---|---|---|
| `create_room` Vacant | rooms shard（entry 持有）→ conn_to_room shard | shard 锁保持期 |
| `create_room` Occupied | rooms shard（entry 持有），不取 conn_to_room shard（已在前置 line 194 释放过） | 单 shard |
| `on_disconnect` | conn_to_room（line 314）→ rooms（line 315） | **顺序释放，非同时持有** |
| `heartbeat` | conn_to_room（line 329-332）→ rooms（line 333） | **顺序释放**（旧版本持有 conn_to_room 同时调 rooms.get_mut 被修正） |
| `send_to_desktop` | rooms（line 262-272） | 单 shard |
| `cleanup_stale_rooms` | rooms.retain（L365）→ conn_to_room.remove 二次 pass（L379-381） | 故意分两 pass（修复 cross-shard 锁竞争） |

**死锁分析**：
- create_room 与 heartbeat 锁方向相反，但 heartbeat **不同时持有**两锁（先克隆 room_id 释放 conn_to_room 再取 rooms），所以不存在 AB-BA 死锁。
- on_disconnect 同 heartbeat 模式。
- create_room Vacant 同时持有 rooms+conn_to_room shard，但其他路径都不同时持有这两把锁（都是顺序获取），无死锁风险。

#### 7. 兼容性核对

- **api_key=None 全部放行**：
  - `auth.require(&None)` 返回 Ok（`websocket.rs:319-326` 单元测试 `auth_require_gates_only_when_key_configured` 覆盖）
  - WS 升级：`websocket_upgrade_open_when_api_key_unset`（L376）验证 101
  - 上传：`upload_routes_accept_valid_api_key_and_stay_open_when_unset`（L267）验证 `state.api_key=None` 时无 key 通过
  - build_relay_router 注入 `api_key` 不变（L132-145）
- **relay_client.rs 重连**：`services-integrations/src/remote_connect/relay_client.rs:262` `CreateRoom { room_id: Some(原id) }` 未改；diff 中无此文件改动。重连路径走 create_room Occupied+tombstone 分支接管（实测 `create_room_takes_over_after_disconnect` L502 通过）。
- **Conflict Error 帧协议形态**：`{ "type": "error", "message": "room already exists" }`（`websocket.rs:213-217`），测试 `duplicate_create_room_sends_room_exists_error`（L471-510）断言精确字符串。`OutboundProtocol::Error` 协议形态未变（`websocket.rs:90-92` 既有定义）。

#### 8. 日志 English-only 检查

所有新增/修改的日志（`grep "tracing\|info!\|warn!\|debug!\|error!"` 全扫）：

| 位置 | 字符串 | 状态 |
|---|---|---|
| `websocket.rs:99` | `"Rejected WebSocket upgrade: missing or invalid API key"` | EN ✓ |
| `websocket.rs:106` | `"Rejected WebSocket upgrade: connection limit reached"` | EN ✓ |
| `websocket.rs:125` | `"WebSocket connected: conn_id={conn_id}"` | EN ✓ |
| `websocket.rs:142` | `"Disconnecting slow consumer conn_id={conn_id}: outbound queue is full or closed"` | EN ✓ |
| `websocket.rs:148` | `"WebSocket close from conn_id={conn_id}"` | EN ✓ |
| `websocket.rs:152` | `"WebSocket error conn_id={conn_id}: {e}"` | EN ✓ |
| `websocket.rs:158-160` | `"WebSocket idle timeout for conn_id={conn_id} (no message for {:?})"` | EN ✓ |
| `websocket.rs:175` | `"WebSocket disconnected: conn_id={conn_id}"` | EN ✓ |
| `websocket.rs:190` | `"Invalid message from conn_id={conn_id}: {e}"` | EN ✓ |
| `websocket.rs:211` | `"Room {room_id} create conflict for conn_id={conn_id}"` | EN ✓ |
| `room.rs:223` | `"Room {room_id} created by desktop {device_id} (conn {conn_id})"` | EN ✓ |
| `room.rs:235-237` | `"Room {room_id} already exists with active desktop conn {existing_conn}; rejecting create from conn {conn_id}"` | EN ✓ |
| `room.rs:252` | `"Room {room_id} taken over by desktop {device_id} (conn {conn_id}): previous desktop disconnected or stale"` | EN ✓ |
| `room.rs:271` | `"Room {room_id}: desktop outbound queue full or closed, dropping message ({e})"` | EN ✓ |
| `room.rs:317` | `"Desktop disconnected from room {room_id}"` | EN ✓ |

无 CJK、无 emoji。✓

#### 9. 行数 / god-file 压力

| 文件 | 行数 | 阈值 | 状态 |
|---|---|---|---|
| `src/crates/services/relay-core/src/relay/room.rs` | 640 | 800 | ✓ 无压力 |
| `src/crates/services/relay-core/src/routes/websocket.rs` | 536 | 800 | ✓ 无压力 |
| `src/crates/services/relay-core/src/routes/api.rs` | 512 | 800 | ✓ 无压力（test 拆出后从 812 降到 512，housekeeping 规则 1 顺手清配额） |
| `src/crates/services/relay-core/src/routes/api/handler_tests.rs`（新文件） | 297 | 800 | ✓ 无压力 |

无 `// allow-god-file` 注释需求。

---

## 三、Findings（按 Critical/Important/Minor 分级）

### Critical

无。

### Important

无。

### Minor

**M-1（理论）：`on_disconnect` 中 conn_to_room.remove 与 rooms.get_mut 之间存在微观窗口**

- 证据：`src/crates/services/relay-core/src/relay/room.rs:313-321`
  ```rust
  pub fn on_disconnect(&self, conn_id: ConnId) {
      if let Some((_, room_id)) = self.conn_to_room.remove(&conn_id) {  // ① conn_to_room 锁释放
          if let Some(mut room) = self.rooms.get_mut(&room_id) {       // ② 重取 rooms 锁
              if room.desktop.as_ref().is_some_and(|d| d.conn_id == conn_id) {
                  info!("Desktop disconnected from room {room_id}");
                  room.desktop = None;
              }
          }
      }
  }
  ```
- 窗口描述：① 与 ② 之间释放了 conn_to_room shard 锁，尚未取 rooms shard 锁。此时并发 create_room（NEW conn_id 接管同 room_id）能通过 `rooms.entry → Occupied` 分支，看到 `desktop.is_some()=true`（tombstone 尚未设），判 `desktop_alive=TRUE` → 返回 **Conflict**。合法重连被错误拒绝。
- 影响：
  - 窗口极窄（两次连续 DashMap 操作之间，亚微秒级），常规运行不触发
  - 非安全漏洞，是可用性微抖动
  - Desktop 客户端接 Conflict 后会重试，下一次 create_room 看到 tombstone 已设 → 接管成功
  - 替代顺序（先设 tombstone 再 remove conn_to_room）会引入新竞态：takeover 已完成时，on_disconnect 用旧 conn_id 设 `desktop = None` 会把新 conn 误标 tombstone。**当前顺序是更优选择**
- 建议：记终审 triage。如要消除，需引入两 map 联合原子原语（DashMap 无），或把 conn_to_room 改为 `BTreeMap<ConnId, RoomRef>` 由同一 shard 持有。

**M-2：handle_socket 任务 panic 不会自动释放连接槽位**

- 证据：`src/crates/services/relay-core/src/routes/websocket.rs:117-176`
  ```rust
  .on_upgrade(move |socket| handle_socket(socket, state))
  ```
  handle_socket 在 `release_connection()` 之前的任何 panic 都会让 `active_connections` 计数泄漏。`tokio::spawn` 的 task panic 不会自动调用 on_disconnect / release_connection。
- 影响：理论泄露，无远程触发路径（需 RoomManager/AppState 内部 bug 导致 panic），不属于安全漏洞。
- 建议：可选加固 — 把 release_connection 放在 `Drop` impl 的 RAII 守卫中；或在外层 `tokio::spawn` 块里 `.catch_unwind`（Rust async 不稳定）。记终审 triage。

**M-3：`handle_text_message` 现返回 bool，ws/handler 调用处需传播**

- 证据：`src/crates/services/relay-core/src/routes/websocket.rs:202-211`
  `CreateRoom` 分支用 match 表达式直接返回值（bool），但 `RelayResponse`（L222-230）和 `Heartbeat`（L231-246）分支显式 `return send_json(...)` 或末尾 `true`。三种返回风格混杂，可读性微降。
- 影响：cosmetic，无功能问题（实测 37/37 绿）。
- 建议：统一为末尾 `true` 表达式。

**M-4：`AuthExtractor` 加 `#[derive(Clone)]`（api.rs:62）扩大公开 API**

- 证据：`#[derive(Clone)]` 是测试复用所需（handler_tests 用 `with_key.clone()`）。
- 影响：公开类型多一个 impl trait，不引入安全面扩大。无对外消费方受连带影响。
- 建议：可选 — 把测试 helper 改为构造新实例而非 clone。但 Clone 是常见派生，保留可接受。

---

## 四、最终判决

| 维度 | 判决 | 主因 |
|---|---|---|
| **Spec 合规** | **PASS** | brief §1-§5 + 「明确不做」全部满足；实测 `cargo test -p northhing-relay-core` 37/37 绿（含新增 9+8+2=19 测试） |
| **代码质量** | **PASS** | 三态决策单锁原子、Conflict 原样保留、WS 认证在 upgrade 前拒绝、bounded 队列不阻塞、连接计数对称（日志英文、行数合规、锁序无死锁）；仅 4 项 Minor（无 Critical/Important） |

### Ledger 建议行

```
Task 2: PASS (commits e3d0e53..5971c54, review clean)
  - C-1/C-2/H-1/H-2 全部覆盖；spec §1-§6 + 兼容约束满足
  - 4 项 Minor 记终审 triage：
    M-1 on_disconnect 微观窗口（替代顺序更差）
    M-2 handle_socket panic 槽位泄漏（无远程触发路径）
    M-3 handle_text_message 返回风格混杂（cosmetic）
    M-4 AuthExtractor::Clone 公开 API 微扩（无安全面扩大）
```

---

## 五、复核任务

无（首轮审查，无 prior findings 待复核）。

---

## 六、上轮未涉及项 / 交叉影响

- **Task 1（已 PASS）兼容**：本次修改不影响 `validated.rs` / `WebAssetStore` trait / DiskAssetStore；仅在 handler 入口加 `auth.require()`。Task 1 的 6 个 handler_tests 全部保留并通过。
- **CreateRoom 协议兼容**：brief 已核实 `services-integrations/src/remote_connect/relay_client.rs:46/262/373` 未改，本次 diff 也未改该文件。desktop 客户端自选 room_id 重连语义不变。
- **embedded relay（api_key=None）兼容**：`build_relay_router(..., api_key: None)` 调用点 `relay-server/src/main.rs:43` 未改；本次仅在 router 构造时多注入一个 `ws_idle_timeout: 90s` 字段（L143）。默认行为对调用方完全透明。