# Task 2 Brief: Relay 协议层认证 + 房间生命周期原子化 + 资源限制

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 e3d0e53）
来源：审计报告 C-1（认证部分）、C-2（room ID 部分）、H-1（房间劫持）、H-2（资源耗尽）
前置：Task 1 已完成（validated.rs 类型 + WebAssetStore 新签名可直接用）

## 已核实的现状（编排者亲验）

- `api.rs`：`AuthExtractor`（L54-91，从 `x-api-key` header 读取，`require(&state.api_key)`：expected=None 放行 dev，Some 比对）目前只挂在 `pair`（L150-153）和 `command`（L205-208）。`upload_web` / `check_web_files` / `upload_web_files` 三个上传类路由无认证。
- `websocket.rs`：`websocket_handler`（L82-87）无认证；`max_message_size/max_frame_size/max_write_buffer_size` 均 64 MiB；`handle_socket` 用 `mpsc::unbounded_channel`（L91）；无全局连接数限制。
- `room.rs` `create_room`（L111-148）：无条件 `self.rooms.remove(room_id)` 后插入——任何客户端可用已有 room_id 踢掉真实 desktop（H-1 劫持）。
- **兼容约束（已核实）**：desktop `services-integrations/src/remote_connect/relay_client.rs:261-269` 依赖客户端自选 room_id 做断线重连（`CreateRoom { room_id: Some(原id) }` 重建房间）。因此**不能**拒绝客户端指定 ID，只能让创建语义原子化。
- embedded relay（desktop 进程内）调 `build_relay_router(..., api_key: None)`——所有认证必须在 `api_key=None` 时保持现状开放。

## 需求

### 1. 上传路由认证（C-1 认证缺口）

`upload_web` / `check_web_files` / `upload_web_files` 三个 handler 加 `auth: AuthExtractor` 参数并在入口 `auth.require(&state.api_key)?`（与 pair/command 同模式）。`api_key=None` 行为不变。

### 2. WebSocket 握手认证（C-2/H-1 认证缺口）

`websocket_handler` 加 `AuthExtractor`（FromRequestParts 对 upgrade handler 同样适用）；`state.api_key = Some` 且无 key/key 错 → 返回 401 不 upgrade；`None` 放行。

### 3. create_room 原子化三态语义（H-1 核心）

改为单次 dashmap entry 操作内决策：
- 房间不存在 → 创建，返回 Created。
- 房间存在且 desktop 连接**仍活跃**（连接未断开标记） → 拒绝，向请求方发 `Error { message: "room already exists" }`（协议已有 Error 帧，desktop 客户端已处理）。
- 房间存在但 desktop 连接已断开/僵死 → 允许接管（合法重连路径），记 `info!` 日志含 room_id。
判定"活跃"用现有连接状态：查 `conn_to_room`/连接表内该 desktop conn_id 是否仍登记；若 RoomManager 当前无断开标记机制，新增最小机制：`on_disconnect` 时给 room 打 `desktop_disconnected: bool` 或在 rooms entry 存 conn 存活位。实现方式自选，但决策与插入必须在同一把 dashmap 锁/同一 entry 操作内完成（check-then-act 不可跨 await/锁边界）。
- 重复创建冲突时不得移除原房间、不得断开原 desktop。

### 4. 资源限制（H-2）

- WS 帧上限：64 MiB → 8 MiB（三处 max_message/max_frame/max_write_buffer）。理由写注释：RelayCommand/RelayResponse 的 encrypted_data 负载远小于此，8 MiB 留足余量（HTTP command body limit 为 10 MiB 作参照）。
- 每连接出站队列：`mpsc::unbounded_channel` → `mpsc::channel(256)`；发送处 `try_send` 失败或 `send().await` 满时断开该慢消费者（记 warn），不得阻塞 read 循环。
- 全局连接上限：`RoomManager` 或 handler 层加原子计数，>= 512 时拒绝新 WS 连接（426/503 或在 upgrade 前直接拒绝），断开时递减。idle 超时：WS 读循环 90s 无消息则 ping/断开（用 tokio::time::timeout 包装 next()，超时则关闭并记 debug）。

### 5. 测试（必须，AGENTS.md 并发规则强制）

- create_room 三态：新建成功；活跃时重复 ID 被拒且原房间/原 desktop 不动；断开后接管成功。
- WS 认证：api_key=Some 无 key 拒绝升级（handler 级单测可只测 require 逻辑 + router oneshot 401）。
- 上传路由：api_key=Some 无 key → 401；带 key → 非 401。
- bounded 队列：构造慢消费者（不读 rx），验证发送满后走断连分支不死锁（可用直接构造 channel 的单测，不必起真实 WS）。
- 连接上限计数：增量/减量单测。

### 明确不做

- 不改 CreateRoom 协议结构（room_id 仍 Optional<String>，兼容 desktop 重连）。
- 不加 room capability token 体系（审计建议的完整方案；本任务只做原子化，capability 记终审 triage）。
- 不动 RelayResponse/RelayCommand 转发逻辑、心跳、TTL cleanup。
- 不动 upload 的 ValidatedRelPath/ContentHash 转换（Task 1 已定格）。
- GET catch-all 动态定性归 Task 3。

## 约束（逐字）

- Logs must be English-only, with no emojis.
- 生产 .rs 文件超 800 行触发 review pressure；超 1000 行必须拆分或顶部加 `// allow-god-file` 注释。
- 改动涉及 tokio::select!/取消/超时竞态 → 必须随附自动化测试（仓库 housekeeping 规则 4）。
- 格式化只许 `pnpm run fmt:rs` 或 `cargo fmt -p northhing-relay-core -p northhing-relay-server`；**严禁裸 `cargo fmt`**（会把全 workspace 未格式化文件卷进 diff）。
- embedded relay（api_key=None）与 desktop 重连行为不得改变。
- 不 git commit（编排者统一提交）。

## 验证命令

```
cargo check -p northhing-relay-core -p northhing-relay-server
cargo test -p northhing-relay-core -p northhing-relay-server
```
（workspace 级命令被上游 embed-resource 链阻断，勿用）

## Report

写 `.superpowers/sdd/task-02-report.md`：改动清单 file:line、三态语义的设计决策说明、测试清单与命令输出、状态、遗留疑虑。
