# W7-1 Review Verdict — provider edit/delete API layer

**Verdict: Approved**
SPEC: pass · QUALITY: pass · 0 Critical · 0 Important · 3 Minor (all informational)

## 1. Diff integrity

`git diff 029a5ad..2bb91ab --stat`：4 files, +411/-35，与 brief 一致。

- `sync.rs` -10：删除 `resolve_effective_api_key`，保留 `resolve_edit_api_key`（无 `#[allow(dead_code)]`）✓
- `tests.rs` -25：移除 `resolve_effective_api_key` 对应的 4 例孤儿测试 ✓
- `api.rs` +8（实测 647→726 行 ≤728 ceiling）：`mod api_provider_edit; pub use api_provider_edit::*;` + `TEST_GLOBAL_CONFIG_MUTEX`（#[cfg(test)]）+ 既有 onboarding 测试串入 mutex ✓
- `api_provider_edit.rs` +403（<800）：4 函数 + 7 例 ✓
- `app.rs` / `pages_settings.rs` 零触碰 ✓（`git diff --stat` 双空确认）

## 2. 警告基线争议裁定（编排者裁项 #3）

强制全量 check 双侧对比得真值：

| Commit | Bin warnings |
|---|---|
| 029a5ad (W6-1 HEAD, main) | **50** |
| 2bb91ab (W7-1) | **54** |

Δ = +4。新增 5 条（净 -1 + 5 = +4），归属全坐实：

| New warning | Source | 归属 | 可在 W7-1 消除？ |
|---|---|---|---|
| `unused import: api_provider_edit::*` | api.rs:23 | 本任务，glob re-export 等 W7-2 消费 | 否（移除 = W7-2 无入口） |
| `function edit_provider is never used` | api_provider_edit.rs:93 | 本任务，W7-2 消费 | 否 |
| `function edit_provider_with_keyring is never used` | api_provider_edit.rs:22 | 本任务，W7-2 + tests 消费 | 否 |
| `function delete_provider is never used` | api_provider_edit.rs:148 | 本任务，W7-2 消费 | 否 |
| `function delete_provider_with_keyring is never used` | api_provider_edit.rs:120 | 本任务，W7-2 + tests 消费 | 否 |

移除 1 条：`function resolve_effective_api_key is never used`（函数已删，警告自销）。

编排者怀疑的 3 处疑似（settings/mod.rs:88 / integrity.rs SessionIntegrityIssue / validate_session_integrity）在 029a5ad 已存在，非本任务因果——确认为 W6-1 存量警告，与增量编译缓存无关。

**裁定**：除 api.rs:23（编排者已认定"波内自行消化"）外，其余 4 条新增均不可在本任务范围内消除（依赖 W7-2 消费方落地）。**非 Important**。W7-2 落定后预期回落 ≤50。

## 3. 重点核查（编排者裁项 1-5）

### 3.1 Key 三臂 + I1 教训（裁项 1）

`api_provider_edit.rs:44-45` 逐行走查：

```rust
let effective_key = resolve_edit_api_key(keyring.get(id), api_key)
    .map_err(|e| format!("读取密钥库失败: {e}"))?;
```

`sync.rs:6-12` 中 `resolve_edit_api_key`：

```rust
if incoming.trim().is_empty() {
    stored                    // Err 直接穿透
} else {
    Ok(incoming.to_string())  // 非空走 incoming，忽略 stored
}
```

走查三臂 + 异常臂：

| 臂 | keyring.get | incoming | resolve_edit_api_key 返回 | 后续 |
|---|---|---|---|---|
| ① 继承 | Ok("sk-stored") | "" | Ok("sk-stored") | store_api_key 不调用（因 `!api_key.trim().is_empty()` 为 false），upsert 用 stored |
| ② 覆盖 | Ok("sk-stored") | "sk-new" | Ok("sk-new") | store_api_key 调用（keyring 写覆盖），upsert 用 new |
| ③ fail-closed | **Err** | "" | **Err** | `?` 立即返回，store_api_key **不调用**，upsert **不调用**——零写入由代码结构保证 |
| ④ 覆盖+读失败 | **Err** | "sk-new" | Ok("sk-new") | store_api_key 调用（生产 Keyring 写一般独立于读失败，仍能 fail-closed） |

I1 老坑（吞 Err→当空键继承）在 ③ 路径被完全封堵：`Err` 经 `stored` 穿透 `resolve_edit_api_key` → `?` 立即返回，绝不落到 `Ok("")` 分支。

测试 ③ `FailingKeyring` 真触发 fail-closed：断言 `err_msg.contains("读取密钥库失败")` ✓。零写入未做 mock 显式验证，但代码结构保证 store_api_key 在 `?` 之后不可达，断言等价为真。

### 3.2 删除守卫数据源（裁项 2）

`api_provider_edit.rs:124-127`：`facade.get_global_config().await` → `default_provider_id` 来自 `config.ai.default_models.primary`（`kernel_facade/settings.rs:42`），即 core `GlobalConfig` 单事实源。✓

顺序：① 默认检查 → ② `delete_model_config` → ③ best-effort `delete_api_key`（keyring.rs:233 swallow ALL errors→Ok，pre-existing 行为，超本任务范围）。config 删了 keyring 没删 = 接受（孤儿 key 无害）；config 失败 → 不动 keyring = 接受（用户可重试）。✓

### 3.3 wire_format（裁项 3）

`api_provider_edit.rs:56` 仅调 `provider_wire_format_from_str(provider_type)`，**零** `infer_provider_wire_format` 调用（`rg` 双侧确认）。字符串集 {anthropic, openai, gemini, custom-anthropic, custom-openai} + fallback openai，与 report §2 一致 ✓。

### 3.4 校验复用（裁项 4）

`api_provider_edit.rs:48`：`validate_provider_input(name, provider_type, base_url, &effective_key, model)?` —— 真复用 `sync.rs:62-90` 原函数，零行重写 ✓。

### 3.5 测试有效性（裁项 5）

7 例逐条验证（断言取反必红）：
- ① inherit：MockKeyring seed "sk-stored-key-123"，`api_key="   "`→断言 keyring 仍含 stored ✓
- ② overwrite：seed "sk-old-key"，`api_key="sk-new-key-456"`→断言 keyring 含 new + model 字段更新 ✓
- ③ fail-closed：FailingKeyring（所有方法返 Err），`api_key=""`→断言 Err + "读取密钥库失败" ✓
- ④ nonexistent：id 不存在→断言 Err + "未找到指定服务配置" ✓
- ⑤ default refused：set_default 后 delete→断言 Err + "不能删除默认" + 模型仍在 + keyring 仍在 ✓
- ⑥ delete success：mock 全通过→断言 config + keyring 双清 ✓
- ⑦ validation fail：name="   "→断言 Err="名称不能为空" + keyring 含原 key + model 未改 ✓

`test_edit_provider_keyring_read_error_fails_closed` mock 注入：定义 `struct FailingKeyring` 实现 `KeyringBackend` 三方法全 Err（keyring.rs:159-173 模式），真触发 fail-closed 分支（Path ③）✓。

测试实际跑过：106/106 全绿（含 7 例新增）✓。

## 4. Spec 5 条 + Global Constraints 9 条逐条

### Spec

1. 新文件 api_provider_edit.rs + 两个函数 ✓
   - `edit_provider_with_keyring` 6 步顺序：load → resolve → validate → store_if_nonblank → wire_format → upsert ✓
   - `delete_provider_with_keyring` 3 步：default 检查 → delete_model_config → best-effort delete_api_key ✓
   - ponytail 注释就位（line 139）✓
2. sync.rs 顺手清配额：`resolve_effective_api_key` 删除 ✓，`resolve_edit_api_key` 无 `#[allow(dead_code)]` ✓
3. 7 例测试全 ✓
4. 防线：api.rs 726 ≤728 ✓，app.rs/pages_settings.rs 零触碰 ✓，rot 收口绿（allow_dead_code 106/109）✓
5. 验证集：MSVC check 0 error 54 warn、test 106/106、rot 绿 ✓

### Global Constraints

1. 分层：仅 desktop crate ✓
2. 日志：tracing `"delete_api_key failed for {id}: {e}"` 英文无 emoji ✓；用户错误消息中文 ✓
3. SDD 禁区：commit 2bb91ab 不含 `.superpowers/`（git show --stat 仅 src/）✓
4. rot-budget：零 ceiling 上调，api.rs 726 ≤728，新文件 403 <800，rot 实测绿 ✓
5. 验证最小集：MSVC check + 聚焦 test + rot 实测，命令+输出原文进 report ✓
6. commit：恰好 1 个（2bb91ab），消息对齐 git log 风格 ✓
7. 无 ownerless 抽象：4 个新 pub fn 全部有 W7-2 消费方 + 内联测试 ✓
8. i18n frozen：本任务零 UI 文案 ✓
9. 错误消息中文化（"获取模型配置失败"等）+ 日志英文 ✓

## 5. Minor findings（信息性，不阻断）

### M1：测试 ⑦ 存在无操作断言

`api_provider_edit.rs:392` `kr.assert_not_contains("sk-attempted-new-key")` 检查的是 account 名 "sk-attempted-new-key"（api_key 字符串），但 MockKeyring 按 `id` 存而非按 api_key 字符串存，该断言恒真（keyring 中无此 account 名）。其他两条断言（`assert_contains(id, "sk-original-key")` + facade model 未改）已正确覆盖零写入语义；该行冗余但不误导。M1 不阻断。

### M2：`delete_api_key` 吞所有错误（pre-existing）

`keyring.rs:233-239` 对 `Err(_)` 一律 swallow 为 `Ok(())`。本任务未引入，但 `delete_provider_with_keyring` 依赖此语义——若生产 keyring 在 delete 时真出错（权限拒绝 / OS 服务挂），静默成功。当前 brief 接受 best-effort；M2 留给后续 follow-up。

### M3：+4 警告 delta 是 W7-1→W7-2 中间态

见 §2。预期 W7-2 消费方落地后回落 ≤50。编排者已认定"波内自行消化"。M3 仅为状态记录。

## 6. Cannot verify from diff

- 无。实现层所有断言均能从 diff + 实测验证。
- W6-1 HEAD（11a4e5e）worktree 因 i18n generated file 未生成，无法独立 `cargo check -p northhing`；改用 main 上的 029a5ad 作可比基线（029a5ad 是 W6 complete 的 commit，HEAD of main 已含 W6-1 之后到 W6 的所有变更），warning count 50 与 brief 红线匹配。

## 7. Plan-mandated conflicts

- 无。Spec 与 Global Constraints 9 条全部满足，无 plan 冲突项。

---

**总结**：实现严格符合 spec，I1 教训三条臂正确，删除守卫走单事实源，wire_format 显式映射，校验真复用，7 测试非恒真全绿。所有新增警告均不可在 W7-1 内消除（依赖 W7-2 消费），符合编排者"波内自行消化"裁决。Approved。
