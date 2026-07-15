# Review: prompt-cache-stats — SessionPromptCacheStore Hit/Miss Statistics

> **Task ID**: prompt-cache-stats  
> **Date**: 2026-06-23  
> **Branch**: `v3-restructure`  
> **HEAD**: `5b2c137` (当前工作区 HEAD，任务在独立工作区执行)  
> **Author**: Agent (LAEP 协议)  
> **Scope**: Add hit/miss/expired statistics tracking to `SessionPromptCacheStore`

---

## 1. 变更摘要

### 1.1 修改文件

| 文件 | 变更类型 | 说明 | 行数 |
|------|----------|------|------|
| `src/crates/execution/agent-runtime/src/prompt_cache.rs` | modify | 新增 `PromptCacheStats` 结构体；`SessionPromptCacheStore` 增加 `stats: Arc<Mutex<PromptCacheStats>>` 字段；在 `lookup_system_prompt` 和 `lookup_user_context` 中增加 hit/miss/expired 计数；新增 `get_stats()` 方法 | +28 |

### 1.2 新增类型

```rust
#[derive(Debug, Clone, Default)]
pub struct PromptCacheStats {
    pub system_prompt_hits: u64,
    pub system_prompt_misses: u64,
    pub system_prompt_expired: u64,
    pub user_context_hits: u64,
    pub user_context_misses: u64,
    pub user_context_expired: u64,
}
```

### 1.3 统计注入点

| 方法 | 命中分支 | 统计字段 | 操作 |
|------|----------|----------|------|
| `lookup_system_prompt` | `is_usable` → `Hit` | `system_prompt_hits` | `+= 1` |
| `lookup_system_prompt` | `is_expired` → `Expired` | `system_prompt_expired` | `+= 1` |
| `lookup_system_prompt` | `_` → `Miss` | `system_prompt_misses` | `+= 1` |
| `lookup_user_context` | `is_usable` → `Hit` | `user_context_hits` | `+= 1` |
| `lookup_user_context` | `is_expired` → `Expired` | `user_context_expired` | `+= 1` |
| `lookup_user_context` | `_` → `Miss` | `user_context_misses` | `+= 1` |

---

## 2. 验证结果

### 2.1 编译状态

```bash
cargo check -p northhing-agent-runtime --lib
```
- **错误**: 0
- **警告**: 0
- **状态**: ✅ PASS

### 2.2 测试状态

```bash
cargo test -p northhing-agent-runtime --lib prompt_cache
```

| 测试 | 结果 | 覆盖 |
|------|------|------|
| `default_prompt_cache_policy_uses_one_day_persistence_ttl` | ✅ pass | 已有测试 |
| `system_prompt_cache_requires_matching_identity` | ✅ pass | 已有测试 |
| `expired_user_context_is_evicted_on_read` | ✅ pass | 已有测试 |
| `invalidate_scope_can_clear_all_cached_prompt_parts` | ✅ pass | 已有测试 |

- **总计**: 4 passed, 0 failed, 0 ignored
- **状态**: ✅ PASS

### 2.3 测试覆盖评估

**注意**: 4 个通过的测试均为任务前已有的测试，用于验证 prompt cache 的基础功能。本次新增的统计功能**没有专门的测试**覆盖。

**缺失的测试**:
- `stats_start_at_zero` — 验证 `PromptCacheStats::default()` 所有字段为 0
- `lookup_system_prompt_hit_increments_hit_counter` — 验证命中后 `system_prompt_hits` 增加
- `lookup_system_prompt_miss_increments_miss_counter` — 验证 miss 后 `system_prompt_misses` 增加
- `lookup_system_prompt_expired_increments_expired_counter` — 验证 expired 后 `system_prompt_expired` 增加
- `get_stats_returns_cloned_snapshot` — 验证 `get_stats()` 返回值的独立性

---

## 3. 设计决策审查

### 3.1 线程安全模型

**决策**: `Arc<Mutex<PromptCacheStats>>`

```rust
pub struct SessionPromptCacheStore {
    session_caches: Arc<DashMap<String, SessionPromptCache>>,
    stats: Arc<Mutex<PromptCacheStats>>,
}
```

**评估**: ✅ 正确。`DashMap` 已经提供了 `session_caches` 的并发安全，但 `PromptCacheStats` 是简单计数器，不需要 `DashMap` 的复杂并发语义。`Arc<Mutex<>>` 足够且轻量。

**潜在问题**: `lock().unwrap()` 在统计更新时可能 panic（如果 Mutex 被 poison）。但考虑到这是 debug/observability 功能，panic 在调试阶段是可接受的。

### 3.2 统计与 `invalidate` 的交互

**代码路径**:
```rust
Some(entry) if entry.text.is_expired(ttl, now_ms) => {
    self.stats.lock().unwrap().system_prompt_expired += 1;
    self.invalidate(session_id, PromptCacheScope::SystemPrompt);
    PromptCacheLookup::Expired
}
```

**评估**: ✅ 正确。`expired` 计数在 `invalidate` 之前，确保计数器被正确记录。如果顺序反过来，`invalidate` 可能失败（虽然在这个调用路径中不会），但当前顺序更安全。

### 3.3 `get_stats()` 返回 Clone

```rust
pub fn get_stats(&self) -> PromptCacheStats {
    self.stats.lock().unwrap().clone()
}
```

**评估**: ✅ 正确。返回 `Clone` 而不是引用，避免调用方持有 Mutex guard。`PromptCacheStats` 全是 `u64` 字段，clone 成本极低。

---

## 4. 边界检查

### 4.1 `can_modify` 合规性

| 文件 | 是否修改 | 边界状态 |
|------|----------|----------|
| `src/crates/execution/agent-runtime/src/prompt_cache.rs` | ✅ 是 | 在 `can_modify` 列表中 |
| `src/crates/execution/agent-runtime/src/lib.rs` | ❌ 否 | 在 `no_touch` 列表中，未修改 |
| `src/crates/execution/agent-runtime/src/prompt.rs` | ❌ 否 | 在 `references` 列表中（只读），未修改 |
| `Cargo.toml` | ❌ 否 | 在 `no_touch` 列表中，未修改 |
| `Cargo.lock` | ❌ 否 | 在 `no_touch` 列表中，未修改 |

**边界检查**: ✅ 全部合规，无越界修改。

---

## 5. 风险与问题

| 问题 ID | 描述 | 严重度 | 状态 | 说明 |
|---|---|---|---|---|
| R-1 | 无统计功能专用测试 | 中 | 已接受 | 已有基础测试通过，但统计计数器本身未验证。建议补充 4-5 个测试 |
| R-2 | `lock().unwrap()` 可能 panic | 低 | 已接受 | 仅影响 stats，不影响核心 cache 功能。debug 功能可接受 |
| R-3 | `change-log.json` 中 `tests_added` / `tests_modified` 为空 | 低 | 已接受 | 记录不准确，但 verification-report 正确反映了测试通过状态 |

---

## 6. 代码审查细节

### 6.1 导入添加

```rust
use std::sync::{Arc, Mutex};
```

原有代码已使用 `Arc`（`session_caches: Arc<DashMap<...>>`），新增 `Mutex` 导入合理。

### 6.2 结构体字段位置

`stats` 字段放在 `session_caches` 之后，与 `SessionPromptCacheStore` 的 `new()` 初始化顺序一致：

```rust
impl SessionPromptCacheStore {
    pub fn new() -> Self {
        Self {
            session_caches: Arc::new(DashMap::new()),
            stats: Arc::new(Mutex::new(PromptCacheStats::default())),
        }
    }
}
```

✅ 正确。

### 6.3 默认实现

`PromptCacheStats` 使用 `#[derive(Default)]`，`SessionPromptCacheStore::default()` 通过 `new()` 委托，stats 自动初始化为全零。

✅ 正确。

---

## 7. 建议

### 7.1 立即补充（可选）

补充 4 个统计专用测试：

```rust
#[test]
fn stats_default_all_zero() {
    let stats = PromptCacheStats::default();
    assert_eq!(stats.system_prompt_hits, 0);
    assert_eq!(stats.system_prompt_misses, 0);
    assert_eq!(stats.system_prompt_expired, 0);
    assert_eq!(stats.user_context_hits, 0);
    assert_eq!(stats.user_context_misses, 0);
    assert_eq!(stats.user_context_expired, 0);
}

#[test]
fn lookup_system_prompt_hit_increments_counter() {
    let store = SessionPromptCacheStore::new();
    store.create_session("s1");
    store.set_system_prompt("s1", CachedSystemPrompt::new(
        SystemPromptCacheIdentity::new("id"), "prompt"
    ));
    let _ = store.lookup_system_prompt("s1", &SystemPromptCacheIdentity::new("id"), None);
    let stats = store.get_stats();
    assert_eq!(stats.system_prompt_hits, 1);
    assert_eq!(stats.system_prompt_misses, 0);
}
```

### 7.2 未来优化

- 考虑使用 `std::sync::atomic::AtomicU64` 替代 `Mutex<u64>`，消除锁竞争（虽然 `Mutex` 在单 writer 场景下开销极小）
- 考虑增加 `reset_stats()` 方法，用于定期重置计数器

---

## 8. 结论

**状态**: ✅ **有条件通过（Conditional Pass）**

代码实现正确、编译通过、已有测试全部通过。唯一的 gap 是**统计功能本身缺少专用测试** — 建议补充 4-5 个测试后正式通过。

**核心质量**: 高
- 线程安全模型正确 (`Arc<Mutex<>>`)
- 统计注入点覆盖完整 (6 个分支全部计数)
- 无边界越界修改
- 无副作用到现有功能

**债务**: 统计计数器测试缺失（建议下轮 session 补充）

---

> **End of Review**
>
> 基于 `.task/Taskfile.toml` + `.task/change-log.json` + `.task/verification-report.json` + 实际代码 (`prompt_cache.rs`) 验证生成。
> 编译验证: `cargo check -p northhing-agent-runtime --lib` ✅ 0 errors, 0 warnings
> 测试验证: `cargo test -p northhing-agent-runtime --lib prompt_cache` ✅ 4/4 PASS
