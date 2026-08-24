# Task 3 Brief: Relay 端到端集成测试 + V-1 GET traversal 动态定性

仓库：`E:\agent-project\.worktrees\northing-backend-debug`（分支 `fix/backend-debug-0731`，基线 5971c54）
来源：补充报告 §5（V-1：GET catch-all 越界读取原语，远程可达性待动态验证）+ §3.4（relay-server 测试缺口）
前置：Task 1（Validated 类型 + DiskAssetStore containment）与 Task 2（认证/三态/资源限制）已完成并双 PASS

## 背景（已核实）

- `relay-core/src/lib.rs` `build_relay_router` 挂 `/r/{*rest}` → `api::serve_room_web_catchall`（Task 1 后：room_id→ValidatedRoomId、file_path→ValidatedRelPath，非法→BAD_REQUEST/NOT_FOUND）。
- relay-server 的 `DiskAssetStore` 已有 7 个 disk_tests（Task 1）。
- relay-core 有 handler_tests 6 + 认证 2（Task 1/2）。
- **缺**：跨 crate 端到端（router + DiskAssetStore 真实磁盘）测试，以及 V-1 编码 traversal 的动态定性证据。

## 需求

### 1. V-1 定性测试（核心交付）

新建 `src/apps/relay-server/tests/e2e_web_assets.rs`（integration test，可用 relay-server lib + relay-core 公共 API）：
- 用 `tempfile::TempDir` 建 base；在 base **外**（tempdir 的兄弟位置）放 marker 文件（如 `secret.txt` 内容 "must-not-leak"）。
- `build_relay_router` 挂 DiskAssetStore + api_key=Some("test-key")，tower `ServiceExt::oneshot` 发请求。
- 先建合法房间并上传 index.html（走 upload-web 认证路由）让 room 有文件。
- 攻击面测试（对 `/r/{room_id}/...` 的各变形）逐一断言**不是 200 且响应体不含 marker 内容**：
  -  literal `../` 段：`/r/{room}/../secret.txt`（注意 HTTP client/oneshot URI 解析可能自行归一化，记录行为）
  -  编码 `%2e%2e`：`/r/{room}/%2e%2e/secret.txt`
  -  双重编码 `%252e%252e`
  -  反斜杠：`/r/{room}/..\secret.txt` 与编码 `%5c`
  -  绝对风格：`/r/{room}//etc/passwd`、盘符编码 `%43%3a%5c...`（`C:\`）
  -  room_id 侧变形：`/r/..%2fsecret.txt/...`
- 每个 case 记录：HTTP 状态 + Axum 实际路由/解码行为（命中 catchall 被类型层拒 / Axum 归一化 404 / 其他），汇总成结论表。
- 断言级别：安全断言（非 200 + 无内容泄露）必须硬 assert；行为归因（哪一层拒的）记录进 report。

### 2. 端到端功能测试

同一文件继续：
- 未认证（无 x-api-key）POST upload-web → 401；带 key → 200 且文件落盘可读（GET `/r/{room}/index.html` 200 内容一致）。
- 未认证 WS upgrade → 401（raw TCP 或 oneshot 带 upgrade header，复用 Task 2 websocket.rs 测试模式）。
- `check-web-files` 已上传 hash → existing_count=1、needed 空。
- GET 不存在房间 → 404 或 SPA fallback 语义与现状一致（记录）。

### 3. 结论回填

把 V-1 定性结论写入 report 末节，格式：
- Task 1 前的原语是否存在（静态，引用审计）。
- 本测试动态验证结果：各编码变形在当前补丁后代码上的实际行为表。
- 结论：远程可达性已封闭/残留风险点。

## 明确不做

- 不改任何生产代码（纯测试任务；若测试暴露真实缺口，停下在 report 写 BLOCKED + 证据，不自行修）。
- 不起公网监听（oneshot/loopback only）。
- 不 git commit。

## 约束（逐字）

- Logs must be English-only, with no emojis.
- 严禁裸 `cargo fmt`；只许 `cargo fmt -p northhing-relay-server`。
- 测试文件放 `tests/` 目录（cargo 集成测试约定），需要的 dev-deps 若 workspace 已有（tower/tempfile/serde_json 等）用 `{ workspace = true }`，新增外部 dev-dep 前先确认 workspace 已有版本。

## 验证命令

```
cargo test -p northhing-relay-server --test e2e_web_assets
cargo test -p northhing-relay-core -p northhing-relay-server
```

## Report

写 `.superpowers/sdd/task-03-report.md`：测试清单、V-1 各变形行为表、定性结论、命令输出、状态。
