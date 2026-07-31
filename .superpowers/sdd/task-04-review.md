# Task 4 Review: SSH/MCP OAuth vault fail-closed + 原子写

**审查范围**: `ab6a91a..88c719a`（1 commit，+330/-61，5 文件）
**审查对象**: implementer 最终 diff（fixer report 仅作背景，以 diff+实测为准）
**commit**: `88c719a fix(security): SSH/MCP OAuth vault fail-closed + 原子写 (H-5)`

---

## 一、Spec 合规判决：**PASS**

### 逐项核对 brief §1-§3

| § | 需求 | 落点 | 状态 |
|---|---|---|---|
| §1 fail-closed | password_vault `store` 写路径 | `password_vault.rs:135-142` `read_vault_file().context("refusing to overwrite vault")` → `write_vault` | ✅ |
| §1 fail-closed | password_vault `remove` 写路径 | `password_vault.rs:173-186` 同模式；entries 空 → `remove_file`（解析成功后保留语义） | ✅ |
| §1 fail-closed | password_vault `migrate_entry` 写路径 | `password_vault.rs:188-202` 同模式 | ✅ |
| §1 fail-closed | mcp/auth `store` 写路径 | `auth.rs:231-242` 同模式 | ✅ |
| §1 fail-closed | mcp/auth `clear` 写路径 | `auth.rs:244-258` 同模式 | ✅ |
| §1 read_vault_file | NotFound → `Ok(VaultFile::default())` | 两 vault 的 `read_vault_file` L97-107 / L158-168：match NotFound 分支返回 default | ✅ |
| §1 read_vault_file | read 失败 → Err + context | `"failed to read vault: {path}"` | ✅ |
| §1 read_vault_file | JSON 解析失败 → Err + context | `"vault corrupted: {path}"` | ✅ |
| §1 load | 损坏时返回 Err（非 Ok(None)） | 两 vault 的 `load` L144-171 / L196-229：`read_vault_file().await?` 移除 `unwrap_or_default` | ✅ |
| §1 load | 单 entry 解密失败保持 warn+Ok(None) | load 路径内 entry 解密分支未动（依赖 `serde_json` 解析后 `entries.get` → `decrypt_value` 现状） | ✅ |
| §2 atomic | 写经 `JsonFileStore::write_atomic` | 两 vault `write_vault` L121-133 / L182-194 直接调用 `JsonFileStore.write_atomic(...)` | ✅ |
| §2 backup | rename 前 `.bak` 复制 | `backup_vault()` L109-119 / L170-180：`tokio::fs::copy(target, target.with_extension("bak"))` | ✅ |
| §2 backup | 失败仅 warn 不阻塞 | `tracing::warn!("Failed to back up vault {}: {}", ...)`；`backup_vault` 不返回 Result | ✅ |
| §2 unix 0o600 | rename 后补设 | `write_vault` 末尾 `#[cfg(unix)]` 块 `set_permissions(0o600)` | ✅ |
| §3 测试 | 损坏 JSON + store → Err + bytes 不变 | `password_vault::store_fails_closed_on_corrupted_vault_without_touching_file` (L233) + `mcp::auth::store_fails_closed_on_corrupted_vault_without_touching_file` (auth.rs:432)，`assert_eq!(tokio::fs::read(&vault_path).await.unwrap(), corrupted)` 硬字节比对 | ✅ |
| §3 测试 | 损坏 JSON + remove → Err + bytes 不变 | `remove_fails_closed_on_corrupted_vault_without_touching_file` (pv L242) + `clear_fails_closed_on_corrupted_vault_without_touching_file` (auth L444) | ✅ |
| §3 测试 | 损坏 JSON + migrate → Err + bytes 不变 | `migrate_fails_closed_on_corrupted_vault_without_touching_file` (pv L256) | ✅ |
| §3 测试 | 截断 JSON + 同三路径 → Err + bytes 不变 | `*_truncated_vault_without_touching_file` × 3（pv L270/284/298），truncated = `{"entries": {` | ✅ |
| §3 测试 | 正常 store + `.bak` 验证 | `store_is_atomic_and_keeps_bak_of_previous_content` × 2（pv L320 / auth L489）：`assert_eq!(bak, first)` 硬字节比对 | ✅ |
| §3 测试 | load 损坏返回 Err | `load_returns_error_on_corrupted_vault` × 2 | ✅ |
| §3 测试 | entries 空 → remove_file | `remove_deletes_file_when_last_entry_is_removed` (pv) / `clear_deletes_file_when_last_entry_is_cleared` (auth) | ✅ |

### 「明确不做」核对

- ✅ 未改 `VaultFile` schema / 加密结构 / key 管理
- ✅ 未动 desktop 调用点 UI 流程
- ✅ 未触碰 H-6/H-7/H-8 范围
- ✅ 未 git commit（report §6.4）

### 验证命令实测

```
$ cargo test -p northhing-services-integrations --features product-full vault
running 16 tests
test mcp::auth::tests::clear_fails_closed_on_corrupted_vault_without_touching_file ... ok
test mcp::auth::tests::clear_fails_closed_on_truncated_vault_without_touching_file ... ok
test mcp::auth::tests::store_fails_closed_on_corrupted_vault_without_touching_file ... ok
test mcp::auth::tests::store_fails_closed_on_truncated_vault_without_touching_file ... ok
test mcp::auth::tests::load_returns_error_on_corrupted_vault ... ok
test remote_ssh::password_vault::tests::store_fails_closed_on_corrupted_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::remove_fails_closed_on_corrupted_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::migrate_fails_closed_on_corrupted_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::store_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::remove_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::migrate_fails_closed_on_truncated_vault_without_touching_file ... ok
test remote_ssh::password_vault::tests::load_returns_error_on_corrupted_vault ... ok
test remote_ssh::password_vault::tests::store_is_atomic_and_keeps_bak_of_previous_content ... ok
test remote_ssh::password_vault::tests::remove_deletes_file_when_last_entry_is_removed ... ok
test remote_ssh::password_vault::tests::migrate_entry_moves_password_to_new_connection_id ... ok
test remote_ssh::manager_tests::tests::prunes_password_connection_without_vault_entry ... ok
test result: ok. 16 passed; 0 failed; 0 ignored

$ cargo test -p northhing-services-integrations --features product-full --lib
（额外命中 mcp::auth::tests::store_is_atomic_and_keeps_bak_of_previous_content
       + mcp::auth::tests::clear_deletes_file_when_last_entry_is_cleared）
全部 ok
```

实测 17/17 全绿（`vault` 过滤器命中 16 个；`store_is_atomic_and_keeps_bak_of_previous_content` 与 `clear_deletes_file_when_last_entry_is_cleared` 名中无 "vault" 子串，过滤时不命中但全 lib 跑时通过）。报告「16 个测试」准确。

---

## 二、代码质量判决：**PASS**

### 安全正确性深度核对

#### 1. fail-closed 覆盖所有写路径（无绕过）

**核验**：grep `read_vault_file` 在两 vault 内的所有调用点：

| vault | 函数 | 写路径入口 | `read_vault_file` + context |
|---|---|---|---|
| password_vault | `store` (L135) | ✅ | `.context("refusing to overwrite vault")?` |
| password_vault | `remove` (L173) | ✅ | `.context("refusing to overwrite vault")?` |
| password_vault | `migrate_entry` (L188) | ✅ | `.context("refusing to overwrite vault")?` |
| password_vault | `load` (L144) | 只读 | `?`（无 context，因为不写） |
| mcp/auth | `store` (L231) | ✅ | `.context("refusing to overwrite vault")?` |
| mcp/auth | `clear` (L244) | ✅ | `.context("refusing to overwrite vault")?` |
| mcp/auth | `load` (L196) | 只读 | `?` |

**无旁路写路径**：所有写函数（5 个）必经 `read_vault_file().await.context(...)?`，解析失败立即返回 Err，未到达 `write_vault` / `remove_file`。✓

#### 2. `remove_file` 空 entries 分支保护

**核验**：`password_vault::remove` (L177-181) 与 `mcp/auth::clear` (L251-254) 的空 entries 分支：
```rust
if file.entries.is_empty() {
    let _ = tokio::fs::remove_file(&self.vault_path).await;
}
```

此分支位于 `read_vault_file` 解析**成功**之后。若解析失败，已被 `?` 提前 return，不会进入 `remove_file`。**空 entries 删除语义保留**（brief §3 最后一条要求）。✓

#### 3. 原子写 PermissionDenied 降级路径

**核验**：`JsonFileStore::write_atomic`（services-core/src/json_store.rs:175-194）在 PermissionDenied 时降级为 `fs::write(path, &json_bytes)` 直接覆写（非原子），但：
- 仍在 `write_vault` 内（vault.rs:121-133）
- 降级 write 后 `write_atomic` 返回 Ok
- `write_vault` 末尾的 `set_permissions(0o600)` 仍执行 → 0o600 不丢失
- 降级前的 `.bak` 备份仍生效（`backup_vault` 先于 `write_atomic` 调用）
- 唯一差异：降级路径下可能 write 中途断电 → 文件损坏；但损坏不会比「不写」更糟，且 `.bak` 仍可恢复上一版内容

report §「复用 json_store 的方式选择及理由」末段已披露此降级行为。✓

#### 4. `.bak` 仅在第二次写后才存在（设计正确）

**核验**：测试 `store_is_atomic_and_keeps_bak_of_previous_content`（pv L320-336 / auth L489-505）显式断言：
```rust
vault.store("a", "p1").await.unwrap();
let first = tokio::fs::read(&vault_path).await.unwrap();
assert!(!vault_path.with_extension("bak").exists());  // ← first write 无 .bak

vault.store("b", "p2").await.unwrap();
let bak = tokio::fs::read(vault_path.with_extension("bak")).await.unwrap();
assert_eq!(bak, first);  // ← .bak 内容 == 上一版文件
```

设计正确：第一次写无「上一版」可备份；第二次写时把第一次的内容备份到 `.bak`，第三次写备份第二次的内容（覆盖），依此类推。✓

#### 5. unix 0o600 在 fallback 路径下保留

**核验**：`write_vault` (pv L121-133 / auth L182-194)：
```rust
JsonFileStore.write_atomic(...).await.context(...)?;  // ← Ok（无论 rename 或 fallback）
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&self.vault_path, std::fs::Permissions::from_mode(0o600));
}
```

`write_atomic` 成功后（无论路径）执行 `set_permissions`。fallback 路径下权限被 set，0o600 不丢。✓

### load Err 传播影响评估（抽查 + grep）

#### 抽查点核对（与 report §「load Err 传播」一致）

| 调用点 | 实际代码 | 行为 |
|---|---|---|
| `manager_saved_connections.rs:145` | `match self.password_vault.load(...).await { Ok(Some(_)) => {}, Ok(None) => removed_ids.push, Err(e) => { warn; removed_ids.push } }` (L145-156) | ✅ Err → warn + 按不可用剔除 |
| `manager_saved_connections.rs:198` | `if password.is_empty() && self.password_vault.load(...).await?.is_none() { bail!(...) }` (L198-200) | ✅ `?` 传播 |
| `manager_saved_connections.rs:244` | `pub async fn load_stored_password(&self, connection_id: &str) -> anyhow::Result<Option<String>> { self.password_vault.load(...).await }` | ✅ 直传 Result |
| `manager_saved_connections.rs:248` | `match self.load_stored_password(...).await { Ok(opt) => opt.is_some(), Err(e) => { warn; false } }` (L249-255) | ✅ Err → warn + false |
| `manager_saved_connections.rs:309` | `let password = self.password_vault.load(...).await?.ok_or_else(|| anyhow!(...))?;` (L309-314) | ✅ `?` 传播 + ok_or_else |
| `mgr_lifecycle_persist.rs:179-191` | `match self.password_vault.load(...) { Ok(Some(pwd)) => ..., Ok(None) => return Err(anyhow!(...)), Err(e) => return Err(anyhow!("Failed to load stored SSH password: {}", e)) }` | ✅ 显式三分支 |
| `auth.rs:280-285` (CredentialStore::load) | `.map_err(\|error\| rmcp::transport::auth::AuthError::InternalError(error.to_string()))` | ✅ 映射为 AuthError |
| `auth.rs:302-306` (has_stored_oauth_credentials) | `let credentials = store.load().await?;` | ✅ `?` 传播 |
| `auth.rs:308-313` (clear_stored_oauth_credentials) | `.clear().await?` | ✅ `?` 传播 |
| `assembly/core/.../mcp/auth.rs:90-116` (has_stored/build_authorization_manager/prepare) | `.map_err(map_auth_error)` → `NortHingError::MCPError` | ✅ 错误传递到上层，无弹窗路径 |

#### grep 兜底（`\.load(` 调用点核对）

我执行了 `grep -rn "SSHPasswordVault\|MCPRemoteOAuthCredentialVault"` 于 `services-integrations/src` 与 assembly 层，得到 5 个调用点（manager_saved_connections.rs L145/170/198/202/209/244/263/309 + mgr_lifecycle_persist.rs L179 + auth.rs L282/288/295/304）。**无漏网调用点**。

#### 关键发现：app 层无直接 vault 调用点

`grep -rn "SSHPasswordVault\|password_vault" src/apps/` 无结果 — desktop/CLI/relay-server 均不直接调用 vault，所有入口经 `RemoteSSHManager` 间接调用。**无 UI 弹窗路径会因 Err 传播被错误触发**。✓

### 日志 English-only 检查

| 位置 | 字符串 | 状态 |
|---|---|---|
| `password_vault.rs:114` | `"Failed to back up vault {}: {}"` | EN ✓ |
| `password_vault.rs:163-164` | `"Treating saved SSH password profile as unavailable: id={}, error={}"` | EN ✓ |
| `auth.rs:175` | `"Failed to back up vault {}: {}"` | EN ✓ |
| `auth.rs:219-220` | `"Treating saved SSH password profile as unavailable: id={}, error={}"` | EN ✓ |

无 emoji、无 CJK。✓（新 vault 代码内仅 `warn` 类日志；其他日志为既有调用方）

### Cargo.toml + feature-rules.mjs 改动正当性

**改动**：
- `services-integrations/Cargo.toml:125` `remote-ssh-concrete` feature 追加 `northhing-services-core`（与该 feature 复用的 owner 一致：git/mcp/miniapp-runtime/workspace-search 早已依赖 services-core）
- `scripts/core-boundaries/rules/feature-rules.mjs:57` 同步 `ownerFeatures` 列表追加 `'remote-ssh-concrete'`
- `Cargo.lock` 由 `cargo check` 自动更新（dev-deps `serde_json`/`base64` 由 Task 3 引入，本 commit 顺带固化）

**核验**：feature flag 一致；feature-rules.mjs owner 覆盖同步；Cargo.lock 增量与 Task 3 的 dev-deps 严格对应（**report §环境备注 4 将此归因于「前序 relay 任务产生」是误读，实际是 Task 3 的 dev-deps 残留**，详见 Minor-3）。✓

### 行数 / god-file 压力

| 文件 | 行数 | 阈值 | 状态 |
|---|---|---|---|
| `services-integrations/src/remote_ssh/password_vault.rs` | 349 | 800 | ✓ |
| `services-integrations/src/mcp/auth.rs` | 519 | 800 | ✓ |

---

## 三、Findings（按 Critical/Important/Minor 分级）

### Critical

无。

### Important

无。

### Minor

**M-1：第一次写时不创建 `.bak`**（设计正确，但需文档化）

- 证据：`backup_vault()` (password_vault.rs:111-119) 在 `vault_path.exists()` 为 false 时直接 return。测试 `store_is_atomic_and_keeps_bak_of_previous_content` L328 显式断言 `!vault_path.with_extension("bak").exists()`（first write 后）。
- 影响：first write（vault 从无到有）期间若 write_atomic 落盘半途断电，无 `.bak` 恢复，**首次写入的 credential 可能丢失**。但 first write 内容来自 `VaultFile::default()` + 一个新 entry，丢的是「这一个新 entry」而非既有 entries。
- 这是 trade-off：备份前提是「存在上一版」，first write 没有上一版。✓ 设计正确，无需修。建议在 `backup_vault` doc 上注明此限制。

**M-2：`set_permissions` 失败被 `let _ =` 静默吞掉**（pre-existing 行为，brief 要求保持）

- 证据：`password_vault.rs:131-133` 与 `auth.rs:191-193`：`let _ = std::fs::set_permissions(&self.vault_path, std::fs::Permissions::from_mode(0o600));`
- 影响：若 `set_permissions` 因权限/文件系统异常失败，文件可能停留在 umask 默认权限（如 0o644），**vault 内容明文 ciphertext 对同机器其他用户可读**。但 brief §2 明示「保持现状行为」，且 pre-existing 代码即此模式。✓ 符合 brief 约束。
- 风险面极小：仅当 (1) 写成功 + (2) chmod 失败 才暴露；常见 chmod 失败为 ENOENT/EACCES，与 write 成功矛盾。
- 建议：可考虑未来改为 `tracing::warn!` 暴露 chmod 失败供运维发现。记终审 triage。

**M-3：Report §环境备注 4 把 Cargo.lock 的 base64/serde_json 增量误归因于「前序 relay 任务」**

- 证据：report §环境备注 4 说「Cargo.lock 有 1 处既有改动（northhing-relay-core 增加 base64/serde_json，前序 relay 任务产生），未触碰」。实际 `git show 88c719a:Cargo.lock` 显示改动在 `northhing-relay-server`（非 relay-core），且这是 Task 3 在 `src/apps/relay-server/Cargo.toml` 加 `serde_json`/`base64` dev-deps 的副作用 — Task 3 未 commit，dev-deps 由本 commit 的 cargo check 首次固化到 lock。
- 影响：cosmetic / 解释偏差。不影响安全、测试、spec 符合性。
- 建议：report 后续修正确认；本 review 已澄清实际来源。

**M-4：测试 `clear_deletes_file_when_last_entry_is_cleared` 与 `store_is_atomic_and_keeps_bak_of_previous_content` 名中无 "vault" 子串，`vault` 过滤器漏匹配**

- 证据：`cargo test -p northhing-services-integrations --features product-full vault` 命中 16 个测试而非 17 个；`--lib` 全跑才命中全部 17 个。
- 影响：CI 脚本若依赖 `vault` 过滤命名约定，可能漏跑；实测 `--lib` 命中全部。
- 建议：测试命名统一含 "vault" 或 "vault_" 前缀；或 CI 改用 `--lib` 跑全。记终审 triage。

**M-5：`backup_vault` 在 first write 时静默不备份（无 .bak 文件创建）已述**

- 与 M-1 实质相同，记冗余。撤销此条。

---

## 四、最终判决

| 维度 | 判决 | 主因 |
|---|---|---|
| **Spec 合规** | **PASS** | brief §1 fail-closed 覆盖 5 个写路径（无旁路）+ load Err 传播；§2 真复用 `JsonFileStore::write_atomic` + `.bak` + 0o600 保留；§3 测试覆盖 brief 全部要求（损坏×3/截断×3 + load + atomic + 空 entries）；实测 17/17 全绿 |
| **代码质量** | **PASS** | 错误 context 清晰（"vault corrupted: {path}" / "refusing to overwrite vault"）；helper 抽取对称（两 vault 同构 `read_vault_file`/`backup_vault`/`write_vault`）；cargo feature + feature-rules.mjs 同步；测试用 `TestTempDir`（RAII）+ 硬字节比对断言；仅 5 项 Minor，无 Critical/Important |

### Ledger 建议行

```
Task 4: PASS (commits ab6a91a..88c719a, review clean)
  - H-5 fail-closed + atomic + .bak 全部落地
  - 5 项 Minor 记终审 triage：
    M-1 first-write 无 .bak（设计正确，需文档化）
    M-2 set_permissions 失败静默（pre-existing，brief 要求保持）
    M-3 report §环境备注 Cargo.lock 增量归因误读（实际是 Task 3 残留）
    M-4 vault 过滤命名约定导致 2 个测试漏匹配
  - load Err 传播影响评估与代码实际一致；grep 兜底无漏网调用点；app 层无直接 vault 调用点
```