# Meta-Plan Execution 文档 (Coding/Testing Agent 专用)

> **Reader**: LAEP Coding Model + LAEP Testing Model（4B lightweight model）
> **Companion**: [`2026-06-23-meta-plan.md`](./2026-06-23-meta-plan.md)（总目录）, [`2026-06-23-meta-plan-review.md`](./2026-06-23-meta-plan-review.md)（review 文档，**不要读**）
> **Canon**: [`../development/laep-execution-canon.md`](../development/laep-execution-canon.md)（4 准则）
> **Created:** 2026-06-23

---

## §0. 必读前置（每个 Task 开始前）

```bash
# 0.1 读这 3 个文件
Read .task/HANDOVER.md
Read .task/archive/prompt-cache-stats-api/review-guide.md   # 学 review 风格
Read docs/plans/2026-06-23-meta-plan.md   # 总目录（不读 execution 全文）
Read docs/development/laep-execution-canon.md   # 4 准则（理解 review 文档的拆分原因）

# 0.2 Windows 环境
export PATH="/c/msys64/mingw64/bin:$PATH"

# 0.3 输出文件位置
.task/Taskfile.toml       # Task 定义（cp 模板后填）
.task/change-log.json     # Coding 输出
.task/verification-report.json   # Testing 输出
.task/review-guide.md     # Review 输出（review 模型专用，**你不需要写**——LAEP Review Model 自动生成）

# 0.4 归档协议（每个 Task 完成后）
mkdir -p .task/archive/<task-name>/
mv .task/Taskfile.toml .task/change-log.json .task/verification-report.json .task/review-guide.md .task/archive/<task-name>/
```

---

## §1. Task 1 — `prompt-cache-stats-serialize`

### 1.1 意图

给"账本"加一个"导出按钮"——以后想看缓存效果时，能把账本里的所有数字打包成 JSON 文本，发给监控系统或者写日志。

### 1.2 代码内 doc comment 模板（直接复制到 `prompt_cache.rs` 的 `impl PromptCacheStats` 块末尾）

```rust
/// 把账本打包成 JSON 对象，方便发给监控系统。
///
/// 包含两组数字（系统提示 / 用户上下文），每组都有：
/// 命中次数、未命中次数、过期次数、命中率（0.0 到 1.0）。
///
/// 命中率在没有任何查询时返回 `0.0`（不是 `null`），
/// 这样下游直接 `format!("{:.1}%", rate * 100.0)` 不会出错。
pub fn as_json(&self) -> serde_json::Value {
    // ... 实现见 §1.3 ...
}
```

### 1.3 代码草稿

```rust
pub fn as_json(&self) -> serde_json::Value {
    serde_json::json!({
        "system_prompt": {
            "hits": self.system_prompt_hits,
            "misses": self.system_prompt_misses,
            "expired": self.system_prompt_expired,
            "hit_rate": self.system_prompt_hit_rate(),
        },
        "user_context": {
            "hits": self.user_context_hits,
            "misses": self.user_context_misses,
            "expired": self.user_context_expired,
            "hit_rate": self.user_context_hit_rate(),
        },
    })
}
```

### 1.4 测试步骤（操作化）

1. 在 `prompt_cache.rs` 的 `mod tests` 末尾追加 2 个测试函数（精确名字）：
   - `as_json_includes_all_six_fields`
   - `as_json_handles_zero_state`
2. 运行：`cargo test -p northhing-agent-runtime --lib prompt_cache::tests::as_json`
3. **成功标志**：`test result: ok. 2 passed; 0 failed`
4. **失败标志**：编译错误或任意 1 个 failed → 修复并重跑，最多 3 次
5. **全量回归**：`cargo test -p northhing-agent-runtime --lib prompt_cache` → 期望 15 passed (13 旧 + 2 新)

### 1.5 Taskfile 草稿

```toml
[meta]
name = "prompt-cache-stats-serialize"
description = "Add as_json() to PromptCacheStats for telemetry export"
author = "agent"
created_at = "2026-06-23"
status = "IN_PROGRESS"

[env]
PATH_MSYS2 = "C:\msys64\mingw64\bin"

[context]
read = ["src/crates/execution/agent-runtime/src/prompt_cache.rs"]
references = [".task/archive/prompt-cache-stats-api/review-guide.md"]

[boundaries]
can_modify = ["src/crates/execution/agent-runtime/src/prompt_cache.rs"]
no_touch = [
    "src/crates/execution/agent-runtime/src/lib.rs",
    "Cargo.toml",
    "Cargo.lock",
]

[verification]
commands = [
    "cargo check -p northhing-agent-runtime --lib",
    "cargo test -p northhing-agent-runtime --lib prompt_cache",
]

[output]
change_log = ".task/change-log.json"
verification_report = ".task/verification-report.json"
```

### 1.6 解耦约束

- **不修改**任何 `pub fn` 签名（仅在 `impl PromptCacheStats` 块末尾追加新方法）
- **不修改**任何现有测试函数
- **不引入**新 crate 依赖（`serde_json::json!` 宏已在 `serde_json` workspace dep 中）
- **不修改** `mod tests { use super::... }` 的 import 列表

---

## §2. Task 2 — `prompt-cache-stats-combined`

### 2.1 意图

给账本加一个"算总账"功能——不分子账本（系统提示/用户上下文），只看一个大数字。运营仪表盘上只需展示一行。

### 2.2 代码内 doc comment 模板

```rust
/// 返回"算总账"用：所有查询的总数。
/// 等于 `system_prompt_total() + user_context_total()`。
pub fn combined_total(&self) -> u64 { /* ... */ }

/// 返回"算总账"用：所有查询的总命中率（0.0 到 1.0）。
/// 没有任何查询时返回 `0.0`。
pub fn combined_hit_rate(&self) -> f64 { /* ... */ }
```

### 2.3 代码草稿

```rust
pub fn combined_total(&self) -> u64 {
    self.system_prompt_total() + self.user_context_total()
}

pub fn combined_hit_rate(&self) -> f64 {
    let total = self.combined_total();
    if total == 0 {
        0.0
    } else {
        (self.system_prompt_hits + self.user_context_hits) as f64 / total as f64
    }
}
```

### 2.4 测试步骤

1. 追加 2 个测试：
   - `combined_hit_rate_is_zero_when_no_lookups`（默认 `PromptCacheStats::default()` → 期望 0.0）
   - `combined_hit_rate_averages_two_caches`（构造 `system: 2 hits / 3 total, user: 1 hit / 1 total` → 期望 `(2+1)/(3+1) = 0.75`）
2. 跑：`cargo test -p northhing-agent-runtime --lib prompt_cache::tests::combined`
3. **成功**：`2 passed; 0 failed`；全量期望 17 passed (15 + 2)

### 2.5 Taskfile 草稿

```toml
[meta]
name = "prompt-cache-stats-combined"
description = "Add combined_total() and combined_hit_rate() to PromptCacheStats"
author = "agent"
created_at = "2026-06-23"
status = "IN_PROGRESS"

[env]
PATH_MSYS2 = "C:\msys64\mingw64\bin"

[context]
read = ["src/crates/execution/agent-runtime/src/prompt_cache.rs"]
references = [".task/archive/prompt-cache-stats-api/review-guide.md"]

[boundaries]
can_modify = ["src/crates/execution/agent-runtime/src/prompt_cache.rs"]
no_touch = [
    "src/crates/execution/agent-runtime/src/lib.rs",
    "Cargo.toml",
    "Cargo.lock",
]

[verification]
commands = [
    "cargo check -p northhing-agent-runtime --lib",
    "cargo test -p northhing-agent-runtime --lib prompt_cache",
]

[output]
change_log = ".task/change-log.json"
verification_report = ".task/verification-report.json"
```

### 2.6 解耦约束

- 同 Task 1（纯加法，不改签名、不改测试、不引入依赖）

---

## §3. Task 3 — `prompt-cache-stats-effectiveness-report`

### 3.1 意图

把"账本 + 三个命中率 + 当前时间戳"打包成一个"快照"——以后调试/汇报时一次性拿走所有信息，不用一个一个查。

### 3.2 代码内 doc comment 模板

```rust
use std::time::SystemTime;

/// 一张"缓存效果快照"——某个时间点所有数字的合集。
/// 序列化后可发给监控/写日志/保存到磁盘。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheEffectivenessReport {
    /// 当前账本（hits / misses / expired 原始计数）
    pub stats: PromptCacheStats,
    /// 系统提示命中率（0.0 到 1.0）
    pub system_prompt_hit_rate: f64,
    /// 用户上下文命中率（0.0 到 1.0）
    pub user_context_hit_rate: f64,
    /// 总命中率（0.0 到 1.0）
    pub combined_hit_rate: f64,
    /// 抓取这张快照的时间（毫秒，自 1970-01-01 起）
    pub captured_at_ms: u64,
}

impl SessionPromptCacheStore {
    /// 抓取当前缓存效果快照。线程安全（与 `get_stats` 共享同一把锁）。
    pub fn get_effectiveness_report(&self) -> CacheEffectivenessReport { /* ... */ }
}
```

### 3.3 代码草稿

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheEffectivenessReport {
    pub stats: PromptCacheStats,
    pub system_prompt_hit_rate: f64,
    pub user_context_hit_rate: f64,
    pub combined_hit_rate: f64,
    pub captured_at_ms: u64,
}

impl SessionPromptCacheStore {
    pub fn get_effectiveness_report(&self) -> CacheEffectivenessReport {
        let stats = self.get_stats();
        let system_prompt_hit_rate = stats.system_prompt_hit_rate();
        let user_context_hit_rate = stats.user_context_hit_rate();
        let combined_hit_rate = stats.combined_hit_rate();
        CacheEffectivenessReport {
            stats,
            system_prompt_hit_rate,
            user_context_hit_rate,
            combined_hit_rate,
            captured_at_ms: current_time_ms(),
        }
    }
}
```

### 3.4 测试步骤

1. 追加 3 个测试：
   - `effectiveness_report_reflects_current_stats`（设几个 lookup，验证报告里的数字与 `get_stats()` 一致）
   - `effectiveness_report_serializes_to_json`（`serde_json::to_string(&report)` 成功且包含 `"system_prompt_hit_rate"` 字段）
   - `effectiveness_report_zero_state`（默认 store，期望所有 hit_rate = 0.0，stats == default()）
2. 跑：`cargo test -p northhing-agent-runtime --lib prompt_cache::tests::effectiveness`
3. **成功**：`3 passed`；全量期望 20 passed (17 + 3)

### 3.5 Taskfile 草稿

```toml
[meta]
name = "prompt-cache-stats-effectiveness-report"
description = "Add CacheEffectivenessReport + get_effectiveness_report() for one-shot snapshot export"
author = "agent"
created_at = "2026-06-23"
status = "IN_PROGRESS"

[env]
PATH_MSYS2 = "C:\msys64\mingw64\bin"

[context]
read = ["src/crates/execution/agent-runtime/src/prompt_cache.rs"]
references = [".task/archive/prompt-cache-stats-api/review-guide.md"]

[boundaries]
can_modify = ["src/crates/execution/agent-runtime/src/prompt_cache.rs"]
no_touch = [
    "src/crates/execution/agent-runtime/src/lib.rs",
    "Cargo.toml",
    "Cargo.lock",
]

[verification]
commands = [
    "cargo check -p northhing-agent-runtime --lib",
    "cargo test -p northhing-agent-runtime --lib prompt_cache",
]

[output]
change_log = ".task/change-log.json"
verification_report = ".task/verification-report.json"
```

### 3.6 解耦约束

- `CacheEffectivenessReport` 是**全新 `pub struct`**，不修改任何现有 struct
- `get_effectiveness_report()` 是**全新 `pub fn`**，不动现有 `get_stats()` / `clear_stats()` 等
- `use std::time::SystemTime;` 可能与现有 import 重复 → **LAEP Coding Model 自检**：若提示 unused import，删之
- 依赖：使用 `serde::{Serialize, Deserialize}`（`PromptCacheStats` 已有 derive，新 struct 复用同一机制）

---

## §4. Task 4 — `partitioned-loader-extra-tests`（**dev-dep 自动体检**）

### 4.1 意图

"提示拼装器"是关键代码（每次你和 AI 对话都跑一遍），但只考了 5 道题，3 条重要路径没考。补 3 道题，**不改任何逻辑**。

### 4.2 步骤 0：dev-dep 自动体检（人类零干预）

```bash
# 4B 模型必须先跑此体检，决定走哪条路
grep -E '^tokio\s*=|^tokio-test\s*=' src/crates/assembly/core/Cargo.toml
```

- **情形 A**（有 `tokio` 或 `tokio-test`）→ 走路径 1（`#[tokio::test]`）
- **情形 B**（没有）→ 走路径 2（同步测试，把 `can_modify` 改回 `prompt_builder_impl.rs`）

### 4.3 代码内 doc comment 模板（追加在 `mod tests` 末尾的注释）

```rust
#[cfg(test)]
mod extra_tests {
    //! 补强 PartitionedLoader 的关键路径覆盖。
    //!
    //! 这些测试在 release 构建中**完全消失**（`#[cfg(test)]` 隔离），
    //! 不影响生产二进制大小或性能。

    use super::*;

    /// 同一身份连续两次构建 agent prompt，第二次应直接返回缓存（不重新算）
    #[tokio::test]
    async fn agent_prompt_cache_hit_skips_rebuild() { /* ... */ }

    /// 改了工具定义后 system prompt 缓存应失效（重新构建）
    #[tokio::test]
    async fn system_prompt_cache_miss_after_tool_defs_change() { /* ... */ }

    /// 同一组字段（即使 String 在不同内存地址）应产生相同 hash
    #[test]
    fn cache_identity_hash_is_stable_for_equivalent_inputs() { /* ... */ }
}
```

### 4.4 代码草稿

```rust
#[tokio::test]
async fn agent_prompt_cache_hit_skips_rebuild() {
    let mut loader = PartitionedLoader::new("agentic_mode");
    let ctx = PromptBuilderContext::default();

    // 第一次：cache miss
    let first = loader.build_agent_prompt(&ctx).await.expect("build");

    // 第二次：身份未变 → cache hit，HashMap 命中走 fast path
    let second = loader.build_agent_prompt(&ctx).await.expect("build");

    // 缓存命中时返回的是 clone，引用同一字符串
    assert_eq!(first, second);
    // 验证 loader 内部确实缓存了
    assert!(loader.agent_prompt.is_some());
    assert_eq!(
        loader.agent_prompt_identity.as_ref().map(|i| i.template_name.clone()),
        Some("agentic_mode".to_string())
    );
}

#[tokio::test]
async fn system_prompt_cache_miss_after_tool_defs_change() {
    let mut loader = PartitionedLoader::new("agentic_mode");
    let mut ctx = PromptBuilderContext::default();

    let _ = loader.build_agent_prompt(&ctx).await.expect("agent");
    let _ = loader.build_system_prompt(&ctx, "hash-A").await.expect("sys A");

    // 改 tool_defs hash → 缓存应失效
    let _ = loader.build_system_prompt(&ctx, "hash-B").await.expect("sys B");

    // 验证：缓存被新值覆盖
    let identity = loader.system_prompt_identity.as_ref().expect("identity");
    assert_eq!(identity.tool_defs_hash, hash_string("hash-B"));
}

#[test]
fn cache_identity_hash_is_stable_for_equivalent_inputs() {
    let h1 = hash_string("agentic_mode");
    let h2 = hash_string("agentic_mode");
    let h3 = hash_string("agentic_mode_other");

    assert_eq!(h1, h2, "same input must produce same hash");
    assert_ne!(h1, h3, "different input must produce different hash");
}
```

> **注意**：`hash_string` 是 `partitioned_loader.rs` 内的 `fn hash_string(s: &str) -> u64`（line 160），已有 `#[test] fn hash_string_is_deterministic()` 验证，新测试可复用。

### 4.5 测试步骤

1. 跑体检命令（§4.2 步骤 0）确定走路径 1 还是 2
2. **路径 1**：在 `partitioned_loader.rs` 的 `mod tests` 末尾追加上述 3 个测试
3. **路径 2**：在 `prompt_builder_impl.rs` 的 `mod tests`（若有）或新建 `mod tests` 中写同步版本（用 `tokio::runtime::Runtime` 或直接调用同步 helper）
4. 跑：`cargo test -p northhing-assembly-core --lib prompt_builder`
5. **成功**：`3 passed`（路径 1 新增），或 `2 passed`（路径 2 同步版 + 已有的 `hash_string_is_deterministic`）；0 failed

### 4.6 Taskfile 草稿（路径 1：有 tokio）

```toml
[meta]
name = "partitioned-loader-extra-tests"
description = "Add 3 tests covering cache-hit, cache-miss-after-change, and identity-hash stability in PartitionedLoader"
author = "agent"
created_at = "2026-06-23"
status = "IN_PROGRESS"

[env]
PATH_MSYS2 = "C:\msys64\mingw64\bin"

[precheck]
# 体检门控：若失败则走路径 2 并修改 can_modify
command = "grep -E '^tokio\\s*=|^tokio-test\\s*=' src/crates/assembly/core/Cargo.toml"
expect_match = true

[context]
read = [
    "src/crates/assembly/core/src/agentic/agents/prompt_builder/partitioned_loader.rs",
    "src/crates/assembly/core/Cargo.toml",
]
references = [".task/archive/prompt-cache-stats-api/review-guide.md"]

[boundaries]
can_modify = [
    "src/crates/assembly/core/src/agentic/agents/prompt_builder/partitioned_loader.rs",
]
no_touch = [
    "src/crates/assembly/core/src/agentic/agents/prompt_builder/prompt_builder_impl.rs",
    "src/crates/assembly/core/Cargo.toml",
    "Cargo.lock",
]

[verification]
commands = [
    "cargo check -p northhing-assembly-core --lib",
    "cargo test -p northhing-assembly-core --lib prompt_builder::partitioned_loader",
]

[output]
change_log = ".task/change-log.json"
verification_report = ".task/verification-report.json"
```

### 4.7 Taskfile 草稿（路径 2：无 tokio，**降级**）

```toml
[boundaries]
can_modify = [
    "src/crates/assembly/core/src/agentic/agents/prompt_builder/partitioned_loader.rs",
    "src/crates/assembly/core/src/agentic/agents/prompt_builder/prompt_builder_impl.rs",
]
```

> 其余字段同路径 1。

### 4.8 解耦约束

- **仅修改 `mod tests`**（已有 `#[cfg(test)]`），release 二进制完全不变
- **不修改**任何 `pub fn` 签名或实现
- **不引入**新依赖（tokio 已有 / 不需要）
- 测试与 `PartitionedLoader` 内部状态直接交互（`loader.agent_prompt` / `loader.system_prompt_identity`）——这是**已存在的 pattern**（看 `invalidate_*` 现有测试），无需新接口

---

## §5. Task 5 — `command-runner-mock`（**下游 mock 自动体检**）

### 5.1 意图

给"执行 shell 命令"功能加一个"假执行"模式——以后测试时不用真跑命令，省时且不会误删文件。**只有别人真需要时才做**（体检门控自动判断）。

### 5.2 步骤 0：下游 mock 自动体检

```bash
# 检查 tool-execution 是否有自己的命令 mock
grep -rn "mock.*Command\|MockCommand\|test.*command" src/crates/execution/tool-execution/ 2>/dev/null | head -5
grep -rn "fn run_command\|run_command(" src/crates/execution/tool-execution/ 2>/dev/null | head -5
```

- **情形 A**（tool-execution 已有 mock 或不调用 services-core 的 `run_command`）→ **SKIP** Task 5，change-log 记录 "SKIPPED: precheck passed (no work needed)"
- **情形 B**（tool-execution 调 `run_command` 且无 mock）→ 继续

### 5.3 代码内 doc comment 模板

```rust
//! 测试用假执行模块。
//!
//! `#[cfg(test)]` 隔离——release 构建中**完全消失**，
//! 不影响生产二进制大小或性能。
//!
//! 使用方法（仅测试代码可见）：
//! 1. `set_mock_output(Ok(CommandOutput {...}))` 预设假输出
//! 2. 调用 `mock_run_command(...)` 拿到预设的假输出
//! 3. `clear_mock_output()` 清除预设（或用 `None` 触发 panic 防止静默走真路径）
#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::sync::Mutex;

    static MOCK_OUTPUT: Mutex<Option<Result<CommandOutput, SystemError>>> = Mutex::new(None);

    pub fn set_mock_output(out: Result<CommandOutput, SystemError>) {
        *MOCK_OUTPUT.lock().unwrap() = Some(out);
    }

    pub fn clear_mock_output() {
        *MOCK_OUTPUT.lock().unwrap() = None;
    }

    pub async fn mock_run_command(
        _cmd: &str,
        _args: &[String],
        _cwd: Option<&str>,
        _env: Option<&[(String, String)]>,
    ) -> Result<CommandOutput, SystemError> {
        MOCK_OUTPUT
            .lock()
            .unwrap()
            .take()
            .expect("mock output not set; call set_mock_output first")
    }
}
```

### 5.4 代码草稿（追加在 `command.rs` 末尾的 `mod tests` 之前）

```rust
#[cfg(test)]
pub mod test_support {
    use super::*;
    use std::sync::Mutex;

    static MOCK_OUTPUT: Mutex<Option<Result<CommandOutput, SystemError>>> = Mutex::new(None);

    pub fn set_mock_output(out: Result<CommandOutput, SystemError>) {
        *MOCK_OUTPUT.lock().unwrap() = Some(out);
    }

    pub fn clear_mock_output() {
        *MOCK_OUTPUT.lock().unwrap() = None;
    }

    pub async fn mock_run_command(
        _cmd: &str,
        _args: &[String],
        _cwd: Option<&str>,
        _env: Option<&[(String, String)]>,
    ) -> Result<CommandOutput, SystemError> {
        MOCK_OUTPUT
            .lock()
            .unwrap()
            .take()
            .expect("mock output not set; call set_mock_output first")
    }
}
```

### 5.5 测试步骤

1. 跑体检（§5.2 步骤 0）→ 情形 A 直接 SKIP
2. 情形 B：在 `command.rs` 末尾的 `mod tests` 内追加 4 个测试：
   - `mock_run_command_returns_configured_output`（`set_mock_output(Ok(...))` → `mock_run_command(...)` 返回相同）
   - `mock_run_command_returns_configured_error`（`set_mock_output(Err(...))` → `mock_run_command` 返回相同 Err）
   - `clear_mock_output_resets_to_panic_mode`（`clear_mock_output()` 后调用 `mock_run_command` 期望 panic）
   - `run_command_propagates_exit_code`（真路径 smoke test：`run_command("cmd", &["/C", "exit", "0"], ...)` 期望 success=true）
3. 跑：`cargo test -p northhing-services-core --lib system::command`
4. **成功**：4 passed

### 5.6 Taskfile 草稿

```toml
[meta]
name = "command-runner-mock"
description = "Add cfg(test) mock injection point to run_command for downstream tool tests; SKIP if precheck finds existing mock"
author = "agent"
created_at = "2026-06-23"
status = "IN_PROGRESS"

[env]
PATH_MSYS2 = "C:\msys64\mingw64\bin"

[precheck]
# 下游 mock 体检：若 tool-execution 已有 mock 或不调用 services-core run_command，则 SKIP
command = "sh -c '(grep -rn \"mock.*Command\\|MockCommand\" src/crates/execution/tool-execution/ 2>/dev/null | head -5; grep -rn \"services_core::system::command\\|services-core.*run_command\" src/crates/execution/tool-execution/ 2>/dev/null | head -5) | grep -q .'"
expect_match = true
on_mismatch = "SKIP_TASK"

[context]
read = [
    "src/crates/services/services-core/src/system/command.rs",
    "src/crates/services/services-core/src/process_manager.rs",
]
references = [".task/archive/prompt-cache-stats-api/review-guide.md"]

[boundaries]
can_modify = ["src/crates/services/services-core/src/system/command.rs"]
no_touch = [
    "src/crates/services/services-core/src/process_manager.rs",
    "src/crates/services/services-core/src/lib.rs",
    "Cargo.toml",
    "Cargo.lock",
]

[verification]
commands = [
    "cargo check -p northhing-services-core --lib",
    "cargo test -p northhing-services-core --lib system::command",
]

[output]
change_log = ".task/change-log.json"
verification_report = ".task/verification-report.json"
```

### 5.7 解耦约束

- `pub mod test_support` 在 `#[cfg(test)]` 内——release 构建**完全消失**
- 不修改 `run_command` / `run_command_simple` / `check_command` 任何已有 `pub fn`
- 不修改 `process_manager.rs`（已是 `no_touch`）
- 不引入新依赖
- 与下游 `tool-execution` 的可能 mock **完全独立**（不同模块、不同 crate），可单独装/拆

---

## §6. 阶段验收（跑完 5 Task 后必跑）

- [ ] `cargo test --workspace` 全绿（无 regression）
- [ ] 5 个 Task 全部归档到 `.task/archive/<task-name>/`
- [ ] `docs/plans/2026-06-23-next-tasks.md` 标记 v3-P1 = COMPLETED（不强制，可选）
- [ ] `.task/HANDOVER.md` 更新到新 session 状态
- [ ] 每个 Task 的 `change-log.json` 的 `confidence` 字段 ≥ medium
- [ ] 任何 `unsafe` / `unwrap()` / `panic!()` 在 change-log `notes` 字段注明

---

## §7. 紧急刹车条件（**STOP 报告人类**）

按 LAEP 协议 SKILL.md `Pause Conditions` 表：

| 触发条件 | 动作 |
|---------|------|
| 任何 Task 触碰 `no_touch` 列表 | **STOP** 报告 boundary violation |
| 编译错误 3 次未解决 | **STOP** 报告 BLOCKED |
| 测试 FAIL 3 次未解决 | **STOP** 报告 BLOCKED（**不是**无限重试） |
| Task 4 体检发现无 tokio → 降级失败 | **STOP** 报告 NEEDS_HUMAN |
| Task 5 体检结果与预期不符 | **STOP** 报告 NEEDS_HUMAN |
| 任何 `unsafe` 块需要新增 | **STOP** 报告 NEEDS_HUMAN |

> 紧急情况下**不要**继续猜测——4B 模型的最大价值是"快速失败 + 准确报告"，不是"硬撑到对"。

---

> **END of execution document.** Coding/Testing Agent **不应**阅读 `meta-plan-review.md`——那是 review 模型的关注领域。
