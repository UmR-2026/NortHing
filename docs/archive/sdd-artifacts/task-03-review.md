# Task 3 Review: Relay e2e 集成测试 + V-1 GET traversal 动态定性

**审查范围**: `5971c54..ab6a91a`（1 commit，+569，2 文件，纯测试）
**审查对象**: implementer 最终 diff（fixer report 仅作背景，以 diff+实测为准）
**commit**: `ab6a91a test(relay): e2e web asset 集成测试 + V-1 GET traversal 动态定性封闭`

---

## 一、Spec 合规判决：**PASS**

### 逐项核对 brief §1-§3

| § | 需求 | 落点 | 状态 |
|---|---|---|---|
| §1 V-1 | 9 种变形逐一断言非 200 且响应体不含 marker | `tests/e2e_web_assets.rs:467-517` `traversal_variants_never_leak_sibling_marker`，9 个变体完整；HARD 断言：body 不含 marker + 真实 traversal 必非 200 + 非 traversal 200 时 body 必须等于本房间 index.html | ✅ |
| §1 V-1 | marker 在 base **外**（兄弟位置） | `tests/e2e_web_assets.rs:54-57`：`parent = tempfile::tempdir()`；`base = parent.path().join("relay-base")`；`marker = parent.path().join("secret.txt")` — 兄弟目录 | ✅ |
| §1 V-1 | 真 traversal 硬 assert 非 200 | L501-505 `is_genuine_traversal(path) → assert_ne!(status, 200)` | ✅ |
| §1 V-1 | 记录行为归因（哪层拒）进 report | L255-277 `attribution()` 函数 + L539-543 eprintln 表格；report §3 完整记录 9 行 | ✅ |
| §1 V-1 | oneshot/loopback only（无公网监听） | L67-70 `TcpListener::bind("127.0.0.1:0")` 仅环回；无公网监听 | ✅ |
| §2 上传认证 | 无 key→401；带 key→200 落盘可读 | `upload_requires_key_then_roundtrips_to_disk_and_serve` L300-350：无 key 401（L308）、错 key 401（L312）、对 key 200 + JSON `files_written=1`（L316-319）；磁盘 `{base}/{room}/index.html` 真实存在且内容一致（L322-331） | ✅ |
| §2 GET 回读 | `/r/{room}/index.html` 200 内容一致 | L334-336 断言 body == INDEX_HTML | ✅ |
| §2 check-web-files | 已上传 hash→existing_count=1, needed 空 | `check_web_files_counts_uploaded_hashes` L355-384 | ✅ |
| §2 WS 认证 | raw TCP 握手：无 key→401、错 key→401、对 key→101 | `ws_upgrade_requires_api_key_on_full_router` L389-406 | ✅ |
| §2 GET 不存在房间 | 404 行为与现状一致 | `get_nonexistent_room_and_invalid_room_ids` L439-456：ghost-room→404、`/r/`→404、空格 room_id→404、`/r/..`→404 | ✅ |
| §3 结论回填 | report 末节含 (a) 原语存在/静态、(b) 动态验证、(c) 远程可达性已封闭 | report.md §4 (a) 引用 1b147c3 快照代码、(b) 9/9 硬通过、(c) 三道防线结论 + 2 项残留风险点 | ✅ |
| 「明确不做」 | 零生产代码改动 | diff 仅 `tests/e2e_web_assets.rs`（新增）+ `Cargo.toml`（追加 dev-deps）；生产代码未触 | ✅ |
| 「明确不做」 | 无公网监听 | 全部 loopback 127.0.0.1:0 | ✅ |
| 「明确不做」 | 不 git commit | 由编排者统一提交（report §6.4） | ✅ |

### 验证命令实测

```
$ cargo test -p northhing-relay-server --test e2e_web_assets
running 5 tests
test ws_upgrade_requires_api_key_on_full_router ... ok
test get_nonexistent_room_and_invalid_room_ids ... ok
test check_web_files_counts_uploaded_hashes ... ok
test upload_requires_key_then_roundtrips_to_disk_and_serve ... ok
test traversal_variants_never_leak_sibling_marker ... ok
test result: ok. 5 passed; 0 failed

$ cargo test -p northhing-relay-core -p northhing-relay-server
relay-core:        37 passed; 0 failed
relay-server lib:   7 passed; 0 failed
relay-server bin:   0 passed
e2e_web_assets:     5 passed; 0 failed
doc-tests x2:       0 passed
总计 49 passed, 0 failed
```

实测 49/49 全绿（含 Task 1+2 既有测试不被回归）。

---

## 二、代码质量判决：**PASS**

### 测试有效性深度核对

#### 1. marker 位置 + 真实在 base 外

**核验**：`tests/e2e_web_assets.rs:54-57`：
```rust
let parent = tempfile::tempdir().expect("tempdir for base + marker siblings");
let base = parent.path().join("relay-base");
let marker = parent.path().join("secret.txt");
std::fs::write(&marker, MARKER).expect("write sibling marker");
```

`tempfile::tempdir()` 返回 `<OS-temp>/<随机>.tmp` 形式的独占目录。`base` 是其下子目录 `relay-base/`，`marker` 是其下文件 `secret.txt` — 二者**互为兄弟**，marker 绝不在 base 子树内。

防御假阳性：测试结束后 `assert!(env.marker.is_file(), "sibling marker must still exist")`（L562）+ `assert_eq!(std::fs::read(&env.marker), MARKER.as_bytes())`（L563-566）确保 marker 未被任何代码路径误删/误改。

#### 2. 磁盘递归扫描真实执行

**核验**：`tests/e2e_web_assets.rs:280-292` `collect_files` 递归遍历 + L547-561 全文件 marker 字节扫描：
```rust
let mut files = Vec::new();
collect_files(&env.base, &mut files);
for file in &files {
    let data = std::fs::read(file).unwrap_or_default();
    assert!(
        !data.windows(MARKER.len()).any(|w| w == MARKER.as_bytes()),
        "marker content found under base dir: {}",
        file.display()
    );
    assert_ne!(
        file.file_name().map(|n| n.to_string_lossy().into_owned()),
        Some("secret.txt".to_string()),
        "a secret.txt appeared under base dir"
    );
}
```

- `collect_files` 对每个 entry 递归（`is_dir` → 递归调用，`is_file` → 推入列表）
- 字节扫描 `data.windows(MARKER.len()).any(|w| w == MARKER.as_bytes())` 对每个文件全量比对，**不是只看文件名** — 即使 marker 内容被改名/嵌入合法文件也会被捕获
- 额外防 `secret.txt` 文件名出现在 base 下（L557-560）

#### 3. raw TCP 逐字节无归一化

**核验**：`tests/e2e_web_assets.rs:96-123` `raw_http` 直接 `TcpStream::connect` + `write_all`，**无 HTTP 客户端库参与**。请求头通过 `format!` 拼字面字符串发送（`get_head(path)` L144 直接把传入的 `path` 拼到 `GET {path} HTTP/1.1\r\n`）。

- 防御假阳性：`%2F` 这类被 HTTP 客户端库「友好归一化」的风险不存在
- `Connection: close` 让服务器响应后关闭，避免 keep-alive 阻塞
- `dechunk` 处理 chunked 编码（虽然这些端点实际不会用 chunked）
- 对响应体做完整 `read` 至 EOF

防御假阳性：如果某变体被 `raw_http` 截断了响应（例如 `Connection: close` 不被尊重），`String::from_utf8_lossy(&resp.body)` 会得到空字符串，断言 `assert_eq!(body, INDEX_HTML)` 会失败（而不是假阳性通过）。✓

#### 4. 双重编码变形的断言严谨性

**核验**：`is_genuine_traversal`（L225-247）模拟 axum 单次解码后判定；测试断言二分（`traversal_variants_never_leak_sibling_marker` L500-514）：
- 真实 traversal → 必非 200
- 非 traversal + 200 → body 必等于 INDEX_HTML
- 所有变体 → body 必不含 marker

**关键变体 #3 `%252e%252e` 逐字节 trace**：
- wire：`/r/e2e-room/%252e%252e/secret.txt`
- axum 单次 percent_decode 后（与 `percent_decode_once` L194-218 同语义）：`%252e%252e` → `%2e%2e`
- handler 收到的 rest：`e2e-room/%2e%2e/secret.txt`
- find('/') 取 room_id=`e2e-room`、file_path=`%2e%2e/secret.txt`
- `ValidatedRelPath::try_from("%2e%2e/secret.txt")`：split → `["%2e%2e", "secret.txt"]`，非 `..`/`.`/drive letter，accepted
- `DiskAssetStore::get_file` 找 `<base>/e2e-room/%2e%2e/secret.txt` → not found → SPA fallback → 返回本房间 `index.html`（= INDEX_HTML） → 200

测试断言：`is_genuine_traversal` 返回 false（`%2e%2e` 非 `..`）+ status 200 + body == INDEX_HTML ✓

**关键变体 #2 `%2e%2e` 逐字节 trace**：
- wire：`/r/e2e-room/%2e%2e/secret.txt`
- axum 单次解码：`%2e%2e` → `..`
- handler 收到 rest：`e2e-room/../secret.txt`
- file_path = `../secret.txt`
- `ValidatedRelPath::try_from("../secret.txt")` → ParentDir → **400**

测试断言：`is_genuine_traversal` 返回 true（`..` 是 traversal）+ status != 200 ✓

**防解码次数漂移**：若 axum 升级为「零次解码」（不变），变体 #2 变体 #3 都会退化为「字面文件名 + SPA fallback 200」，测试 L502-505 会捕获（变体 #2 应当 traversal 但实际 200）。若 axum 升级为「二次解码」，变体 #3 退化为 traversal → 400，测试 L506-514 的 200 分支不触发，body 断言不执行。两种漂移方向至少有一种会被硬捕获。✓

#### 5. 房间侧变形的归因

**核验**（变体 #8 `/r/..%2fsecret.txt/...`）：
- wire 含 `%2F`（encoded slash）。axum 是否解码 %2F 决定 room_id 解析
- report 静态归因「handler 层 `ValidatedRoomId`（解码后 room=`..` 拒绝）」隐含 axum 解码 %2F
- 测试断言：`is_genuine_traversal` 返回 true + status != 200（实测 404）
- 若 axum 不解码 %2F：room_id = `..%2fsecret.txt`，`ValidatedRoomId::try_from("..%2fsecret.txt")` 含 `.` → InvalidCharacter → 404。同结果（404）。✓ 测试在两种 axum 行为下都通过。

**核验**（变体 #9 `/r/../secret.txt`）：
- wire 无 percent encoding。room_id = `..`（literal）
- report 静态归因「matchit 不做 dot 段归一化，`..` 作为普通段命中 catch-all，已读 matchit 0.8.4 源码确认」
- 测试断言：status != 200（实测 404）。✓

#### 6. parser_probe 验证归因

**核验**（`tests/e2e_web_assets.rs:519-537`）：
```rust
let parser_probe = raw_http(env.addr, &get_head("/\\"), "").await;
// eprintln "[probe] raw backslash at unmatched path: status {} (404 => hyper/axum accept it)"
let handler_probe = raw_http(env.addr, &get_head("/r/no-such-room/..\\x"), "").await;
// eprintln "[probe] raw backslash + dotdot in catch-all shape: status {} (400 => handler validation)"
```

- probe 1：`GET /\` 若返回 404 → 证明 hyper/axum 接受原始 `\` 于 request-target，**故变体 #4 的 400 必来自 handler 而非协议层**
- probe 2：`/r/no-such-room/..\x` 返回 400 → 直接命中 catch-all 形状，handler 校验拒绝

两个 probe 合并给出：「hyper 接受原始 `\`」+「handler 对 `..\x` 拒绝」的明确归因。这与变体 #4 状态 400 形成因果链。✓

### 「明确不做」核对

- ✅ 零生产代码改动：diff 仅 `tests/e2e_web_assets.rs`（新增 567 行）+ `Cargo.toml`（追加 dev-deps `serde_json` + `base64`）
- ✅ 无公网监听：所有 `TcpListener::bind("127.0.0.1:0")` 环回
- ✅ 不 git commit：report §6.4 明示

### 日志 English-only 检查

测试代码内的 `eprintln!` 调用：

| 位置 | 字符串 | 状态 |
|---|---|---|
| L528-531 | `"[probe] raw backslash at unmatched path: status {} (404 => hyper/axum accept it)"` | EN ✓ |
| L533-536 | `"[probe] raw backslash + dotdot in catch-all shape: status {} (400 => handler validation)"` | EN ✓ |
| L539 | `"=== V-1 traversal variant behavior table ==="` | EN ✓ |
| L541 | `"{name:<26} status={status:<4} leaked={leaked:<5} {attr}"` | 格式化字符串 ✓ |
| L542 | `"    path: {path}"` | EN ✓ |

无 emoji、无 CJK。✓

### 行数 / god-file 压力

| 文件 | 行数 | 阈值 | 状态 |
|---|---|---|---|
| `src/apps/relay-server/tests/e2e_web_assets.rs`（新增） | 567 | 800 | ✓ 无压力 |

---

## 三、Findings（按 Critical/Important/Minor 分级）

### Critical

无。

### Important

无。

### Minor

**M-1: `is_genuine_traversal` 是 handler 解析逻辑的「镜像」而非直接测试 handler**

- 证据：`tests/e2e_web_assets.rs:225-247`
  ```rust
  fn is_genuine_traversal(path: &str) -> bool {
      fn is_drive(seg: &str) -> bool { ... }
      let decoded = percent_decode_once(path).replace('\\', "/");
      let Some(remainder) = decoded.strip_prefix("/r/") else { return false; };
      ...
  }
  ```
- 风险：测试辅助函数复刻了 `serve_room_web_catchall` 的 split 逻辑；若 handler 改写（如 room_id 长度上限变更、空白 trim 规则变更），`is_genuine_traversal` 与 handler 可能漂移，导致测试断言与实际行为错位。
- 当前缓解：`percent_decode_once` 与 axum 0.8.9 行为吻合（实测 5/5 通过）；若 handler 改写，cargo test 也会因既有功能性测试（如 `check_web_files_rejects_invalid_room_id`）回归。
- 建议：可选 — 加注释指向 handler 行号（`relay-core/src/routes/api.rs:459-468`），或为 `is_genuine_traversal` 加 property-based 测试（仅记终审 triage，测试已稳定）。

**M-2: V-1 9 变体固定，无 mutation/fuzz 随机化**

- 证据：`tests/e2e_web_assets.rs:474-484` 9 个固定变体。
- 风险：仅覆盖 brief 列出的变形；未来若发现新的编码变形（如 `%c0%ae`、`%u002e%u002e`、`/./` 单点段、Unicode 规范化等）需要手动追加。
- 当前缓解：磁盘递归扫描（L547-561）提供泛化兜底 — 任何 leak 都会触发 `data.windows(MARKER.len()).any(...)` 失败。
- 建议：可选 — 在终审阶段补 `proptest` 随机路径生成器。记终审 triage。

**M-3: `attribution()` 仅 eprintln，不作为断言**

- 证据：`tests/e2e_web_assets.rs:255-277` 函数签名返回 String，调用点 L490 `let attr = attribution(path, resp.status);` 仅用于 L541 打印。
- 风险：归因信息（哪层拒）只出现在 stderr，不参与断言；若归因错误（hyper vs handler vs matchit），测试仍会通过。
- 当前缓解：parser_probe 块（L519-537）对 backslash 变体做辅助归因验证；其余变体的归因依赖 report.md 静态说明。
- 建议：可接受 — 归因本质是诊断信息，硬断言会过拟合当前 axum 版本行为。记终审 triage。

**M-4: `dechunk` 函数仅处理标准 chunked 编码，复杂 chunk 边界条件未测**

- 证据：`tests/e2e_web_assets.rs:126-142`
- 风险：若服务端返回非标准 chunked（罕见），`dechunk` 可能误解析；但本测试中所有端点响应都很小，axum 通常用 `Content-Length` 而非 chunked。
- 当前缓解：`Connection: close` + 小响应 → Content-Length 编码居多；实测所有 5 个测试通过。
- 建议：cosmetic，不修。

**M-5: 未测 NUL byte / 超长路径 / Unicode 归一化等次级攻击面**

- 证据：9 变体覆盖了 brief §1 列出的攻击面；`%00`、`%FF%FE` BOM、`..%252f..%252f`（变体 #8 的双重版）等未测。
- 风险：相关攻击面的防御依赖 `ValidatedRoomId`/`ValidatedRelPath` 在更底层（`b.is_ascii_control()`、drive-letter 扫描、ASCII 字符集限制），但 e2e 层无独立硬断言。
- 当前缓解：handler_tests 已有 `rel_path_rejects_control_characters` 单测覆盖 NUL/控制字符；ValidatedRoomId 单测覆盖非 ASCII。
- 建议：可接受 — 单元测试已覆盖；e2e 不重复。

**M-6: 房间 ID 大小写敏感性未测**

- 证据：所有变体使用 lowercase `e2e-room`。
- 风险：`ValidatedRoomId` 不区分大小写（仅 ASCII alnum + `-_`），但 `e2e-room` 与 `E2E-ROOM` 是不同房间。测试未验证大小写语义是否一致。
- 影响：非安全漏洞，是产品语义。
- 建议：可接受 — 非本任务范围（V-1 + §2 functional），记终审。

---

## 四、最终判决

| 维度 | 判决 | 主因 |
|---|---|---|
| **Spec 合规** | **PASS** | brief §1（9 变体 + marker 硬断言 + 行为归因）/ §2（4 个 functional e2e）/ §3（report §4 结论回填完整）全部满足；「明确不做」零生产改动、无公网、不 commit 全遵守；实测 49/49 全绿（含 Task 1+2 既有测试不回归） |
| **代码质量** | **PASS** | marker 真在 base 外（兄弟目录）+ 磁盘递归扫描 + raw TCP wire 级（无客户端归一化）+ 双重编码断言二分严谨 + parser_probe 验证归因；6 项 Minor（无 Critical/Important），均为可选加固方向 |

---

## 五、V-1 定性结论 — 独立认可

| 结论 | 独立认可 | 理由 |
|---|---|---|
| **（a）Task 1 前原语存在且可远程触达** | ✅ 认可 | 与静态审计一致：`1b147c3` 快照的 `serve_room_web_catchall` 无校验 + `DiskAssetStore::get_file` 无 containment check + axum 0.8.9 单次解码 → 真实 OOB-read 原语。 |
| **（b）当前补丁下 9/9 变形无泄露** | ✅ 认可 | 实测 5 个测试通过（含 V-1 核心）+ 磁盘递归扫描硬断言 + marker 真在 base 外 + raw TCP 排除了客户端归一化的伪证。`is_genuine_traversal` 与 axum 0.8.9 行为吻合（基于 49/49 实际通过反推）。 |
| **（c）V-1 远程可达性已封闭** | ✅ 认可 | 三道防线（类型层 ValidatedRoomId/ValidatedRelPath、磁盘层 canonicalize + fail-closed、axum 路由层 catch-all 不归一化）共同闭合；9/9 变形的硬证据足以支撑。 |

### 但有以下 2 项独立观察（非驳回，仅记录）

1. **SPA fallback 200 行为在双重编码变体下被固化**：变体 #3 `%252e%252e` 返回 200 + 本房间 index.html（与 `missing.js` 行为等价）。**不构成越界读**（response body 始终等于 INDEX_HTML，断言已硬保证），但严格语义上「非 200」未达成。report §4 残留风险 1 已记录。如未来产品要求严格 non-200，需改 `get_file` fallback 策略（仅 fallback 无扩展名路径）— 属产品决策，本任务不动。

2. **单次解码语义强依赖 axum 0.8.9 行为**：若 axum 升级改变 wildcard 解码次数或引入二次解码，变体 #3 可能从「字面文件名 200」退化为「traversal 400」，变体 #2 可能反向变化。`is_genuine_traversal` 与 `percent_decode_once` 是 axum 行为的镜像函数，**断言对解码次数漂移是部分鲁棒的**（超解码变 200 分支不触发，少解码变体 #2 会失败），但需在 axum 升级时重验。report §4 残留风险 2 已记录。

### Ledger 建议行

```
Task 3: PASS (commits 5971c54..ab6a91a, review clean)
  - 纯测试任务：1 个新文件 + Cargo.toml dev-deps，零生产代码改动
  - V-1 远程可达性已封闭（独立认可 a/b/c 三条结论，附 2 项独立观察）
  - 6 项 Minor 记终审 triage：
    M-1 is_genuine_traversal 与 handler 解析逻辑漂移风险
    M-2 9 变体固定，无 fuzz
    M-3 attribution 仅 eprintln 不断言
    M-4 dechunk 边界
    M-5 NUL/超长/Unicode 次级攻击面未 e2e 测（单测已覆盖）
    M-6 房间 ID 大小写语义
  - 残留 2 项独立观察：
    SPA fallback 200 在双重编码下被固化（产品语义决策）
    单次解码依赖 axum 0.8.9（升级需重验）
```