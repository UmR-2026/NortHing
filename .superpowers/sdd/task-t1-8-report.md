# Task T1-8 Report — apps/server 收口：删 ai_relay + rpc_dispatcher 鉴权注记 + P2-19 顺手清（SW1-8）

## 改动文件清单

- `src/apps/server/src/ai_relay.rs` (删除): 移除未接线且存在安全风险的正向代理孤儿文件。
- `src/apps/server/src/rpc_dispatcher.rs` (修改): 顶部添加英文鉴权注记 doc comment，明确未接线状态、高危敏感操作（DeepReview 队列控制/config reload 等）、接线前必须先加鉴权的要求及 T4-5 协议冻结再决去留的策略。
- `src/apps/server/README.md` (修改): 移除 3 条指向已删 `relay-server` 的悬空链接与废弃说明，标记 server 仍为 frozen-experimental 状态。
- `docs/status/tech-debt-ledger.md` (修改): 同 commit 翻转 P2-19 状态为 `resolved`。

---

## Spec 1-5 落实情况

1. **删除 `src/apps/server/src/ai_relay.rs`**:
   - 已执行 `git rm src/apps/server/src/ai_relay.rs`。
   - 全仓 `rg -n "ai_relay" src docs` 实测 `src` 零命中，`docs` 仅有历史审计/路线图记录。
2. **`rpc_dispatcher.rs` 鉴权注记**:
   - 顶部添加英文 doc comment，严格遵循 English-only、无 emoji 要求。
   - 包含未编译/未接线声明、安全风险范围（DeepReview 控制、配置重载、文件系统操作）、接线前必须实现认证鉴权硬约束以及 T4-5/ACP 协议冻结前置说明。
3. **P2-19 顺手清与台账翻转**:
   - `src/apps/server/README.md` 清除 3 处指向已删 relay-server 的链接与描述。
   - `docs/status/tech-debt-ledger.md` 将 P2-19 从 `active (frozen surface)` 翻转为 `resolved`。
4. **`docs/status/surfaces.md` 检查**:
   - 已核对 `docs/status/surfaces.md`，该账本中此前仅登记了 `src/apps/server`（状态为 `🧊 Frozen`），未登记 `ai_relay`，crate 级表面无变化，账本保持一致。
5. **不动范围遵守**:
   - `bootstrap.rs`、`Cargo.toml`、`src/apps/server/src/routes/`、`src/apps/server/src/main.rs` 均未做任何改动。

---

## 孤儿文件观察

- `src/apps/server/src/bootstrap.rs`（216 行）：
  - 当前未被 `src/apps/server/src/main.rs` 声明 `mod bootstrap;`，与已处理的 `rpc_dispatcher.rs` 同样为孤儿文件（未参与编译）。
  - 该文件实现了 `ServerAppState` 及初始化逻辑（镜像 Desktop AppState），留待后续 T5 core 宿主升级或协议解冻阶段决定重构或去留。本任务按 brief 要求保持不动。

---

## 验证命令与输出

### 1. `cargo check -p northhing-server`
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check -p northhing-server
```
输出：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.71s
```

### 2. `rg -n "ai_relay" src docs`
```powershell
rg -n "ai_relay" src docs
```
输出：
```text
docs\architecture\backend-roadmap.md:92:| SW1-8 | apps/server 修复 + 删 ai_relay | = R-21（位腐） |
docs\architecture\backend-roadmap.md:114:| `apps/server` | 位腐（源码 import core 但 Cargo.toml 未声明，编译不过；内含未接线 `ai_relay.rs`/`rpc_dispatcher.rs`） | T1-8 修复（删 ai_relay、修依赖）→ **T5 升格为进程外 core 宿主**（或新建 host，T5 时定） |
docs\architecture\backend-roadmap.md:158:| T1-8 | apps/server 修复 + 删 ai_relay | S+面（SW1-8=R-21） | S |
docs\status\full-review-2026-08-16.md:47:| M-5 | `apps/server/ai_relay.rs` = 无鉴权开放正向代理（任意 scheme://host、`usize::MAX` body、SSRF）；`rpc_dispatcher.rs` = 完整 Tauri 命令集。**均未接线但随源码存在**（当前 server 编译不过，见 R-21） | `apps/server/src/ai_relay.rs:84,127-165`、`rpc_dispatcher.rs:25` |
docs\status\full-review-2026-08-16.md:177:| SW1-8 | server 危险模块 | `apps/server/` | 修 Cargo.toml 使可编译（为 SW4 复用）；**删除** `ai_relay.rs`；`rpc_dispatcher` 暂留但加鉴权注记 | S | `cargo check -p northhing-server` 绿 | — |
```
（`src/` 目录下 0 命中）

### 3. `git diff --check`
```powershell
git diff --check
```
输出：
```text
warning: in the working copy of 'src/apps/server/README.md', LF will be replaced by CRLF the next time Git touches it
```
（无空白或格式错误）

---

## 结论

- Commit: `61ba73a9a15cae10931dd0d7e7209670cef00335` (`61ba73a`)
- 任务已按 brief 全部要求完成，无多余夹带。
- 状态：DONE

