# Task B1 — FU-1 save_user_config / delete_server_config fail-closed [security]

分支：`fix/backend-followups-0804`（worktree `E:\agent-project\northing\.worktrees\backend-followups-0804`，基线 main `41695f5`）。
计划：`.superpowers/sdd/plan-2026-08-04-backend-followups.md` §2 Task B1；债项：`.superpowers/sdd/tech-debt-followups.md` FU-1。
所有锚点已由编排者于 `41695f5` 实测复核。

## 1. 问题（两层 fail-open，都要修）

用户级 MCP 配置（key `mcp_servers`）的读-改-写路径对读取失败容错过宽，并发/磁盘抖动下可能丢配置或写残缺 JSON。与已修的 H-7（project 级，Task 6）同漏洞类。

### 层 A — 生产 store 适配器吞掉全部读错误（根因核心）

`src/crates/assembly/core/src/service/mcp/config/service.rs:19-27` `CoreMCPConfigStore::get_config_value`：

```rust
match self.config_service.config::<serde_json::Value>(Some(key)).await {
    Ok(value) => Ok(Some(value)),
    Err(_) => Ok(None),   // ← 一切读错误都伪装成"键不存在"
}
```

后果：`services-integrations` 层 `save_user_config`（`:212-237`）与 `delete_server_config`（`:255-288`）里的 `get_config_value(...).await?` 永远拿不到 Err，磁盘/解析错误被当成空配置，随后的 `set_config_value` 整体覆写 `mcp_servers` → 丢配置。

**修复方向**：区分"键不存在/NotFound = 合法空态 → `Ok(None)`"与"真实 IO/解析失败 → `Err(MCPRuntimeError::configuration(...))` 中止写"。必须先读 `crate::service::config::ConfigService`（`src/crates/assembly/core/src/service/config/`）搞清 `config::<Value>(Some(key))` 的错误语义（缺 key 返回什么、IO 错误返回什么），按 ErrorKind 分类，不得把缺 key 当错误。

### 层 B — 未识别格式静默覆写

`src/crates/services/services-integrations/src/mcp/config/service.rs`：

- `save_user_config` `:219-223`：`current_value.get("mcpServers")` 非 object 时静默从空 map 重建 → 覆写未识别的既有值。
- `delete_server_config` `:262-269`：同模式（非 object 时返回 not_found，但把未识别格式与"不存在"混同，未区分保护）。

**参照物（Task 6 已建模式，勿发明新轮子）**：同文件 `:128-148` `load_project_configs_strict` —— 未识别格式返回 `Err(MCPRuntimeError::configuration("Refusing to overwrite project MCP configs with unrecognized existing format"))`。用户级照抄该语义（措辞改 user-level）。

### 写入原子性核查

`set_config_value`（core 适配器 `:29-38`）下游是 `ConfigService::set_config`。追到落盘实现，确认是否原子写（temp+rename，参考 services-core `json_store::write_atomic` 模式）。若已原子，brief 报告里写明结论即可；若非原子且改造面小（仅影响本 key 的落盘调用点），顺手原子化；改造面大则记入报告"范围外观察项"，不强改。

## 2. 范围

**范围内**：
- 层 A：`CoreMCPConfigStore::get_config_value` 错误分类（core 适配器）。
- 层 B：`save_user_config` + `delete_server_config` 未识别格式 fail-closed（services-integrations）。
- 新增测试（§3）。
- 同 commit 翻转债状态（§5）。

**范围外（勿动）**：
- project 级路径（Task 6 已修，`save_project_config`/`load_project_configs_strict` 不改）。
- `load_user_configs`/`load_project_configs`/`load_all_configs` 的读侧宽容语义（`load_all_configs` `:57-80` 已对 user/project load 错误 warn+empty 兜底，层 A 改严格后读侧兜底天然保持——验证它仍然成立即可）。
- config store 其它 key 的语义审查。

## 3. 测试要求（并发/竞态相关，家规 4 强制带测试）

测试文件：`src/crates/services/services-integrations/tests/config_and_server_lifecycle.rs`。已有基座：
- `FailingMCPConfigStore`（get/set 均失败）→ 用例 `mcp_config_service_keeps_load_failures_as_empty_baseline` `:65-92`（注意其中 `:81-91` 已断言 User 级 save 在 trait 层读错误时 fail-closed——层 A 修复不得破坏该契约）。
- `RecordingFailingGetMCPConfigStore` `:450-`（get 失败、记录 set 调用）→ 用例 `:94-119`（project 级读错误 fail-closed）。
- `InMemoryMCPConfigStore`（values map）→ 用例 `:121-151`（project 级未识别格式拒写）。

新增（镜像 project 级用例，location 换 `ConfigLocation::User`，key 换 `mcp_servers`，格式为 `{"mcpServers": {...}}` cursor format）：
1. User 级 save 在 store 读错误时 fail-closed：返回 `MCPRuntimeErrorKind::Configuration`，且 `set_config_value` 未被调用（用 RecordingFailingGetMCPConfigStore）。
2. User 级 save 遇未识别既有格式（如 `json!(42)`）拒写：错误文案含 "unrecognized existing format"，既有值原样保留。
3. User 级 delete 同样两款（读错误 fail-closed；未识别格式不误删/不覆写）。
4. 层 A（core 适配器）错误分类测试：在 `src/crates/assembly/core/src/service/mcp/config/service.rs` 的 `mod tests`（`:104-`）内新增——构造"缺 key"与"真实读错误"两种情形，断言前者 `Ok(None)`、后者 `Err`。若 `ConfigService` 无法在测试中注入读错误，用你能构造的最小真实情形（如指向损坏/不可读的配置源），并在报告说明手段。

## 4. 验证命令（按序全跑，贴原文输出进报告）

```powershell
$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH
cargo test -p northhing-services-integrations --features product-full mcp
cargo test -p northhing-core --features product-full --lib mcp
cargo check -p northhing-core --features product-full
```

基线：integrations 172/172（改后会增加）；core lib 1134/1134 中与 mcp 相关子集全绿。`cargo check --workspace` 被上游 embed-resource 3.0.11 阻断（非代码问题），勿跑、勿修。

## 5. 纪律（硬规则，违反=任务失败）

- **解债 commit 必须同 commit 翻转** `.superpowers/sdd/tech-debt-followups.md` FU-1 状态：`open` → `resolved`（附 commit 计划说明）。该文件在 worktree 内。
- 只 commit 范围内文件；commit 前 `git status` 核对，勿带入任何无关文件（主仓工作区的 growth-core 在途产物与你无关）。
- **不裸 `cargo fmt`**（两次污染前科）；格式手工对齐现有风格。可用 `pnpm run fmt:rs`（只格式化改动/暂存的 .rs）。
- 日志 English-only、无 emoji；生产 .rs <800 行。
- 不改范围外代码；发现范围外问题 → 记入报告"观察项"，不动手。
- commit message 风格参照 `git log --oneline -5`（conventional 前缀，中文正文可）。建议：`fix(security): MCP user-level config writes fail-closed on read errors (FU-1)`。

## 6. 交付

1. 一个 commit（代码 + 测试 + tech-debt-followups.md FU-1 翻状态）。
2. 报告写入 `.superpowers/sdd/task-b1-report.md`，含：
   - STATUS: DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED（第一行）
   - 改动文件清单 + 每文件改动摘要
   - `ConfigService.config` 错误语义调查结论（缺 key vs IO 错误如何区分）
   - 写入原子性核查结论
   - §4 三条验证命令的原文输出（至少测试计数行）
   - 观察项（如有）
   - 与计划偏离处（如有，必须显式声明）
