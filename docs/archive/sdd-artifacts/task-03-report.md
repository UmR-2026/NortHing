# Task 3 Report: Relay 端到端集成测试 + V-1 GET traversal 动态定性

状态：**DONE**（未暴露新的真实安全缺口；无 BLOCKED）
日期：2026-07-31
分支：`fix/backend-debug-0731`（基线 5971c54，未 commit，未 amend）

## 1. 改动文件（仅 2 处，均为本任务产物）

| 文件 | 改动 |
|---|---|
| `src/apps/relay-server/tests/e2e_web_assets.rs` | 新增：跨 crate 端到端集成测试（5 个测试），含 V-1 动态定性 |
| `src/apps/relay-server/Cargo.toml` | dev-deps 追加 `serde_json`、`base64`（均为 `{ workspace = true }`） |

未改任何生产代码。未 commit。

## 2. 测试清单（`tests/e2e_web_assets.rs`，5 测试）

| 测试 | 覆盖需求 | 断言要点 |
|---|---|---|
| `upload_requires_key_then_roundtrips_to_disk_and_serve` | §2 上传认证 + 落盘 + 回读 | 无 key→401；错 key→401；带 key→200 且 `files_written=1`；磁盘 `{base}/{room}/index.html` 真实落盘且内容一致；GET `/r/{room}/index.html` 200 内容一致；房间根 `/r/{room}/` 与缺失路径 `missing.js` → 200 index（SPA fallback 现状，记录） |
| `check_web_files_counts_uploaded_hashes` | §2 check-web-files | 已上传 hash → `existing_count=1, needed=[], total_count=1`；未知 hash → `needed=["new.js"]` |
| `ws_upgrade_requires_api_key_on_full_router` | §2 WS 认证 | 全路由上 raw TCP 握手：无 key→401、错 key→401、对 key→101（复用 Task 2 websocket.rs 模式） |
| `get_nonexistent_room_and_invalid_room_ids` | §2 GET 不存在房间 | 不存在房间无文件→404（无 SPA fallback 可用）；`/r/`→404；room_id 解码后含空格→404；`/r/..`→404 |
| `traversal_variants_never_leak_sibling_marker` | §1 V-1 核心 | 9 种变形逐一：硬断言响应体不含 marker；真 traversal 变形硬断言非 200；非 traversal 变形若 200 必须等于本房间自己的 index.html；全 base 目录递归扫描无 marker 内容、无 `secret.txt`；兄弟 marker 原样存在 |

全套：`cargo test -p northhing-relay-core -p northhing-relay-server` = **49 通过**（core 37 + server 单测 7 + e2e 5）。

## 3. V-1 变形行为表（动态实测，raw TCP wire 级请求）

环境：`build_relay_router`（relay-core）+ `DiskAssetStore`（relay-server，真实磁盘 TempDir，base 与 marker 为兄弟目录）+ `api_key=Some("test-key")`；房间已建并上传 index.html。请求经 127.0.0.1 回环 socket 逐字节发送，**无任何客户端 URI 归一化**（oneshot/http client 的归一化风险不存在）。

| # | 变形 | 请求路径（逐字上 wire） | 状态 | 泄露 marker | 行为归因（哪层拒的） |
|---|---|---|---|---|---|
| 1 | literal `../` | `/r/e2e-room/../secret.txt` | 400 | 否 | handler 层 `ValidatedRelPath`（`..` 段拒绝） |
| 2 | 编码 `%2e%2e` | `/r/e2e-room/%2e%2e/secret.txt` | 400 | 否 | handler 层 `ValidatedRelPath`（axum 解码后为 `../secret.txt`，拒绝） |
| 3 | 双重编码 `%252e%252e` | `/r/e2e-room/%252e%252e/secret.txt` | 200 | 否 | 非 traversal：axum 仅解码一次 → 字面 `%2e%2e/secret.txt` 通过校验 → 磁盘无此字面目录 → **SPA fallback 返回本房间自己的 index.html**（与 `missing.js` 行为完全一致，见 §4 风险点） |
| 4 | 反斜杠 literal | `/r/e2e-room/..\secret.txt` | 400 | 否 | handler 层 `ValidatedRelPath`（`\` 归一化为 `/` 后拒绝；probe 证实 hyper/axum 接受原始 `\`，见下） |
| 5 | 编码 `%5c` | `/r/e2e-room/..%5csecret.txt` | 400 | 否 | handler 层 `ValidatedRelPath`（解码 `..\secret.txt` 拒绝） |
| 6 | 绝对风格 `//` | `/r/e2e-room//etc/passwd` | 400 | 否 | handler 层 `ValidatedRelPath`（文件段 `/etc/passwd` 根目录拒绝） |
| 7 | 盘符编码 `C:\` | `/r/e2e-room/%43%3a%5csecret.txt` | 400 | 否 | handler 层 `ValidatedRelPath`（解码 `C:\secret.txt` drive-letter 拒绝） |
| 8 | room_id 侧 `..%2f` | `/r/..%2fsecret.txt/...` | 404 | 否 | handler 层 `ValidatedRoomId`（解码后 room=`..` 拒绝） |
| 9 | room_id 侧 literal `..` | `/r/../secret.txt` | 404 | 否 | handler 层 `ValidatedRoomId`（room=`..` 拒绝；matchit 不做 dot 段归一化，`..` 作为普通段命中 catch-all，已读 matchit 0.8.4 源码确认） |

辅助 probe（#4 归因判别）：
- `GET /\`（原始反斜杠、无路由可匹配）→ **404**：hyper/axum 接受原始 `\` 于 request-target，故 #4 的 400 必来自 handler 而非协议解析层。
- `GET /r/no-such-room/..\x`（catch-all 形状 + 原始反斜杠 + `..`）→ **400**：handler 校验确认。

归因依据（静态）：axum 0.8.9 `UrlParams` 对 wildcard 参数统一 `percent_decode` 一次（`axum-0.8.9/src/routing/url_params.rs`）；matchit 0.8.4 无 dot 段归一化；handler `serve_room_web_catchall`（`relay-core/src/routes/api.rs:459`）room_id→`ValidatedRoomId`（404）、file→`ValidatedRelPath`（400）。

## 4. V-1 定性结论

**（a）Task 1 前原语是否存在（静态审计）**：是，且可远程触达。
- 基线快照 `1b147c3` 的 `src/apps/relay-server/src/routes/api.rs`：`serve_room_web_catchall` 对 `Path(rest)` 不做任何校验，room_id/file_path 原样传入 `asset_store.get_file(room_id, lookup_path)`。
- 同快照 `src/apps/relay-server/src/lib.rs` 的 `DiskAssetStore::get_file`：`room_dir.join(path)` 无 containment 检查，`../` 直接越过房间目录边界（可读 `{base}/secret.txt`，`../../` 可越过 base）。
- 组合：`GET /r/{room}/../secret.txt`（及编码变形，axum 0.8 本就对 wildcard 解码）→ 越界读，响应体回显目标文件 → **真实 OOB-read 原语**。此即补充报告 §5 V-1 所述。

**（b）本测试动态验证（当前补丁后）**：9 种变形的安全断言全部硬通过（无任何响应含 `must-not-leak`；真 traversal 变形无一 200；磁盘递归扫描 base 下无 marker 内容）；上传/认证/check-web-files/SPA fallback 现状语义均被固化进测试。

**（c）结论：V-1 远程可达性已封闭。** 攻击面被三道防线封死：URL 单次解码后 → 类型层校验（`ValidatedRoomId`/`ValidatedRelPath`）→ `DiskAssetStore` 落盘 containment（canonicalize 前缀校验 + fail-closed）。动态证据：9/9 变形无泄露。

**残留风险点（记录，非本任务修复范围）**：
1. **SPA fallback 使任意不存在路径返回 200 + 房间自身 index.html**（`DiskAssetStore::get_file` 的既定语义，与 `missing.js` 行为等价）。双重编码 `%252e%252e` 落在该类：200 但响应体=本房间公开的 index.html，**不构成越界读**。若未来要求"非 200"严格语义，需改 `get_file` fallback 策略（如仅对无扩展名路径 fallback）——属产品语义决策，不在纯测试任务内。
2. 单次解码语义依赖 axum 版本行为（0.8.9 wildcard 解码一次）。若升级 axum 改变解码次数或引入二次解码，本表需重验（测试已就位，回归可自动捕获）。

## 5. 验证命令输出

```
> cargo test -p northhing-relay-server --test e2e_web_assets
running 5 tests
test ws_upgrade_requires_api_key_on_full_router ... ok
test get_nonexistent_room_and_invalid_room_ids ... ok
test check_web_files_counts_uploaded_hashes ... ok
test upload_requires_key_then_roundtrips_to_disk_and_serve ... ok
test traversal_variants_never_leak_sibling_marker ... ok
test result: ok. 5 passed; 0 failed

> cargo test -p northhing-relay-core -p northhing-relay-server
relay-core: test result: ok. 37 passed; 0 failed
relay-server (lib): test result: ok. 7 passed; 0 failed
relay-server (bin): test result: ok. 0 passed
relay-server (tests/e2e_web_assets): test result: ok. 5 passed; 0 failed
doc-tests x2: ok. 0 passed
总计 49 passed, 0 failed, 0 warnings（最终一次运行零警告）
```

格式化：`cargo fmt -p northhing-relay-server`（`--check` 先行确认仅本任务新文件有 diff，未触碰 pre-existing 文件；未使用裸 `cargo fmt`）。

## 6. 偏差说明（按简报约束）

1. **未用 tower `ServiceExt::oneshot`，改用 raw TCP 回环**（Task 2 websocket.rs 已验证模式）：`tower` 与 `http-body-util` **均不在 workspace.dependencies**（仅存在于 Cargo.lock 传递依赖，tower 0.5.3 / http-body-util 0.1.3），无法 `{ workspace = true }`；按"新增外部 dev-dep 前先确认 workspace 已有版本"的约束，raw TCP 方案零新增依赖且 wire 级逐字节发送消除了简报提示的"HTTP client/oneshot URI 解析归一化"歧义——V-1 动态定性因此更贴近真实远程攻击面。所需 dev-deps 仅 serde_json/base64（均 workspace 已有，`{ workspace = true }`）。
2. **`%252e%252e` 变形的"非 200"硬断言**：该变形单次解码后为字面字符路径（非 traversal 形态），现状语义为 SPA fallback 200（与本房间 `missing.js` 一致）。硬断言落地为：所有变形无泄露（硬）+ 真 traversal 变形非 200（硬）+ 非 traversal 变形若 200 则 body 必须等于本房间 index.html。详见 §4 风险点 1。
3. **worktree 预存改动未触碰**：`git status` 显示大量 pre-existing 修改（ai-adapters/webdriver/assembly/services 等整树），非本任务产生，未改动、未格式化、未提交，本报告如实说明。
4. 未 git commit（简报明令）。
