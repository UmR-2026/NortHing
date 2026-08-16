# Northing 代码腐化修复轮 — 2026-06-28

> **范围**: 5 个并行 Agent 修复代码腐化问题 + 阻塞项解除
> **触发**: 代码腐化集群审核发现 B-1 阻塞项 + 多项未修复问题
> **结果**: 12 文件修改，7 类修复完成，1 阻塞项解除，2 遗留问题待处理

---

## 修复总览

| 类别 | 修复项 | 文件数 | 修改量 | 方法 |
|------|--------|--------|--------|------|
| 信号量 unwrap | insights/service.rs 7 处崩溃 | 1 | 7 处 | `map_err` + `— ` 传播 |
| unreachable! 清理 | 4 处生产代码 | 4 | 4 处 | `warn!` / `Err` / fallback |
| theme.rs expect | 1 处 panic 语义 | 1 | 1 处 | `unwrap_or_else` + `Default` |
| let _ = Result 丢弃 | 5 个高频文件 | 5 | 75 处 | `if let Err` + `warn!` |
| stream handler | responses.rs 13 处 | 1 | 13 处 | `if let Err` + `warn!` |
| 编译配置 | .cargo/config.toml + Cargo.toml | 2 | 追加/删除 | `sccache` + `split-debuginfo` + `target-dir` |
| 编译阻塞 B-1 | `opt-level = 0` 导致 ring/aws-lc-sys 失败 | 1 | 删除 | 移除 `.cargo/config.toml` 中的 `[profile.dev]` 节 |

**合计**: 12 文件修改（`M`），2 文件新增（`docs/` 未纳入 git track）

---

## 阻塞项解除（B-1）

### 问题
`.cargo/config.toml` 中新增：
```toml
[profile.dev]
split-debuginfo = "packed"
opt-level = 0 # ← 导致 ring / aws-lc-sys C 代码编译失败
incremental = true
```

`ring` 的 pregenerated assembly 在 `-O0` 下无法编译（gcc 报错 `memcmp_invalid_stripped_check` 和 `p256-nistz.c`）。

### 修复
- 从 `.cargo/config.toml` **删除整个 `[profile.dev]` 节**
- 根 `Cargo.toml` 中的 `[profile.dev]`（`split-debuginfo = "packed" + incremental = true`）继续生效，不再重复定义
- 同时修正 `exclude` 拼写：`northhing-installer` → `northing-installer`

### 验证
`cargo test -p northhing --lib` 现在可正常编译，无 `opt-level = 0` — 突。

---

## 文件级改动清单

| 文件 | 改动类型 | 说明 |
|------|---------|------|
| `.cargo/config.toml` | M | 删除 `[profile.dev]` 节（移除 `opt-level = 0` 阻塞），保留 `[build]`/`[target]`/`[alias]` |
| `Cargo.toml` | M | `exclude` 拼写修正 `northhing` → `northing` |
| `src/crates/assembly/core/src/agentic/insights/service.rs` | M | 7 处 `sem.acquire().await.unwrap()` → `map_err(...)— ` |
| `src/crates/execution/tool-execution/src/fs/write_file.rs` | M | `unreachable!()` → 正常返回 `Ok(WriteLocalFileOutcome)` |
| `src/crates/execution/agent-stream/src/tool_call_accumulator.rs` | M | `unreachable!()` → `warn!` + `return None` |
| `src/crates/adapters/ai-adapters/src/client.rs` | M | `unreachable!()` → `Err(anyhow!(...))` |
| `src/crates/adapters/ai-adapters/src/client/quirks.rs` | M | `unreachable!()` → `warn!` + no-op |
| `src/apps/cli/src/ui/theme.rs` | M | `.expect(...)` → `.unwrap_or_else(|| { warn!(...); Default::default() })` |
| `src/crates/adapters/ai-adapters/src/stream/stream_handler/responses.rs` | M | 13 处 `let _ = tx_event.send(...)` → `if let Err` + `warn!` |
| `src/crates/adapters/webdriver/src/platform/capture.rs` | M | 21 处 `let _ =` → `warn!` / `intentionally ignored` 注释 |
| `src/crates/services/terminal/src/exec.rs` | M | 16 处 `let _ =` → `warn!` / `intentionally ignored` 注释 |
| `src/crates/services/services-integrations/src/remote_ssh/remote_exec.rs` | M | 20 处 `let _ =` → `warn!` |
| `src/crates/assembly/core/src/service/mcp/server/manager/auth.rs` | M | 5 处 `let _ =` → `warn!`（其余 10 处为 snapshot 更新，非 Result 丢弃） |
| `docs/code-rot-prevention-guide.md` | 新增 | 代码腐化预防指南（v0.1.0-rev1） |
| `docs/handoffs/2026-06-28-working-tree-review.md` | 更新 | 标记 B-1 为 FIXED，更新 C-1/C-3 状态 |

---

## 已修复问题（对应审核报告）

| 审核项 | 状态 | 说明 |
|--------|------|------|
| `insights/service.rs` 7 处信号量 unwrap | ✅ | 全部改为 `map_err` + `— ` |
| `write_file.rs` `unreachable!()` | ✅ | 改为正常返回 `Ok(...)` |
| `client.rs` `unreachable!()` | ✅ | 改为 `Err(anyhow!(...))` |
| `quirks.rs` `unreachable!()` | ✅ | 改为 `warn!` + no-op |
| `tool_call_accumulator.rs` `unreachable!()` | ✅ | 改为 `warn!` + `return None` |
| `theme.rs` `.expect()` | ✅ | 改为 `.unwrap_or_else` + `Default::default()` |
| `responses.rs` 13 处 `let _ =` | ✅ | 全部改为 `if let Err` + `warn!` |
| `capture.rs` 21 处 `let _ =` | ✅ | 分类处理：可恢复错误 → `warn!`，故意忽略 → 注释 |
| `exec.rs` 16 处 `let _ =` | ✅ | 同上 |
| `remote_exec.rs` 20 处 `let _ =` | ✅ | 同上 |
| `auth.rs` 5 处 Result 丢弃 | ✅ | 改为 `warn!` |
| `B-1` `opt-level = 0` 阻塞 | ✅ | 删除 `.cargo/config.toml` 中的 `[profile.dev]` 节 |
| `exclude` 拼写 | ✅ | `northhing` → `northing` |

---

## 遗留问题（本轮未修复）

| # | 问题 | 文件 | 数量 | 优先级 | 说明 |
|---|------|------|------|--------|------|
| 1 | stream handler 修复不一致 | `anthropic.rs` | 12 处 `let _ =` | P1 | 同模式未修复，commit 时应注明 "Part 1 of 4" |
| 2 | stream handler 修复不一致 | `openai.rs` | 11 处 `let _ =` | P1 | 同上 |
| 3 | stream handler 修复不一致 | `gemini.rs` | 8 处 `let _ =` | P1 | 同上 |
| 4 | `unwrap()` 激增 | 全项目 | ~518 处 | P2 | 新增代码大量引入，需 clippy 审计 |
| 5 | 临时文件 | `_rot_scan.py`, `_rot_scan.txt` | 2 | P1 | 应删除或加入 `.gitignore` |
| 6 | `target-shared/` | 构建产物 | — | P1 | 应加入 `.gitignore` |

---

## 推荐 Commit 策略

### 方案 A：单 commit（当前状态）

```bash
git add .cargo/config.toml Cargo.toml \
 src/apps/cli/src/ui/theme.rs \
 src/crates/adapters/ai-adapters/src/client.rs \
 src/crates/adapters/ai-adapters/src/client/quirks.rs \
 src/crates/adapters/ai-adapters/src/stream/stream_handler/responses.rs \
 src/crates/adapters/webdriver/src/platform/capture.rs \
 src/crates/assembly/core/src/agentic/insights/service.rs \
 src/crates/assembly/core/src/service/mcp/server/manager/auth.rs \
 src/crates/execution/agent-stream/src/tool_call_accumulator.rs \
 src/crates/execution/tool-execution/src/fs/write_file.rs \
 src/crates/services/services-integrations/src/remote_ssh/remote_exec.rs \
 src/crates/services/terminal/src/exec.rs
git commit -m "fix(code-rot): 7类修复+解除B-1编译阻塞

- 信号量unwrap 7处 → map_err+— (insights/service.rs)
- unreachable! 4处 → warn/Err/fallback (write_file, accumulator, client, quirks)
- theme.rs expect → unwrap_or_else + Default
- let _ = 75处 → warn logging (capture, exec, remote_exec, auth, responses)
- B-1: 删除.cargo/config.toml的[profile.dev]节（opt-level=0阻塞编译）
- 编译配置: sccache + split-debuginfo + target-dir + exclude拼写修正

遗留: anthropic/openai/gemini stream handler let _ = 待修复（Part 1 of 4）
Refs: docs/handoffs/2026-06-28-working-tree-review.md"
```

### 方案 B：分 commit（更清晰）

```bash
# Commit 1: 错误处理修复（ unwrap/unreachable/expect/let _ = ）
git add src/crates/assembly/core/src/agentic/insights/service.rs \
 src/crates/execution/tool-execution/src/fs/write_file.rs \
 src/crates/execution/agent-stream/src/tool_call_accumulator.rs \
 src/crates/adapters/ai-adapters/src/client.rs \
 src/crates/adapters/ai-adapters/src/client/quirks.rs \
 src/apps/cli/src/ui/theme.rs \
 src/crates/adapters/ai-adapters/src/stream/stream_handler/responses.rs \
 src/crates/adapters/webdriver/src/platform/capture.rs \
 src/crates/services/terminal/src/exec.rs \
 src/crates/services/services-integrations/src/remote_ssh/remote_exec.rs \
 src/crates/assembly/core/src/service/mcp/server/manager/auth.rs
git commit -m "fix(error-handling): unwrap/unreachable/expect/let _ = cleanup

- 7处信号量unwrap → map_err+— 4处unreachable! → warn/Err/fallback
- theme expect → unwrap_or_else + Default
- 75处let _ = → warn logging (5 files)"

# Commit 2: 编译配置修复（B-1 阻塞项）
git add .cargo/config.toml Cargo.toml
git commit -m "fix(build): remove opt-level=0 blocker, fix exclude typo

- 删除.cargo/config.toml的[profile.dev]重复定义（opt-level=0导致ring/aws-lc-sys编译失败）
- exclude: northhing → northing"

# Commit 3: 文档（可选）
git add docs/code-rot-prevention-guide.md docs/handoffs/2026-06-28-working-tree-review.md
git commit -m "docs: add code-rot prevention guide, update working-tree review"
```

**推荐方案 B**：错误处理修复与编译配置修复是不同维度，分 commit 更清晰，回滚更安全。

---

## 参考文档

| 文档 | 说明 |
|------|------|
| `docs/handoffs/2026-06-28-working-tree-review.md` | 本轮审查原始报告（B-1 阻塞项详细分析） |
| `docs/code-rot-prevention-guide.md` | 腐化预防指南（含修复方法、预防机制、AI Agent 约束） |
| `research/audit_redim01.md` ~ `audit_redim_v3_04.md` | 三轮审核基线与重审报告（结构/债务/质量/依赖） |

---

*文档结束 — 2026-06-28*
