# R1 Shell-exec Sandbox — Complete Design

> **Status:** Design Complete — Awaiting User Review
> **Date:** 2026-06-23
> **Scope:** Phase 1 (Audit Pass) + Phase 2 (Denylist Extension) + Phase 3 (Mode-based Confirmation + Audit Log)
> **Related:** `docs/reviews/2026-06-23-r1-shell-exec-status.md` (status review)

---

## 1. Motivation

Per the R1 status review (commit `408c101`):

**现状缺口**：
- ⚠️ `skip_tool_confirmation` 默认是 `true`（`config/types.rs:1598`）— 默认无用户确认
- ❌ 审计日志缺失 — 无法追溯哪些命令被允许/拒绝/confirm 过
- ❌ 8+ 其他 shell-exec 路径未审计：
  - `computer_use_actions.rs`
  - `browser_launcher.rs`
  - `ngrok.rs`
  - `lsp/process.rs`
  - `mcp/server/connection.rs`
  - `miniapp/runtime.rs`
  - `process_manager.rs`
  - `glob_search.rs`
  - `port_adapters.rs`

**最大风险**：catastrophic commands 已经 denylist，但 confirmation 默认 skip 意味着 dangerous-but-not-catastrophic 命令（如 `rm -rf build/`、`git push --force`）直接执行。

---

## 2. Goal

Three-phase implementation:

### Phase 1 — Audit Pass (1-2 days, doc-only)
输出 `docs/security/r1-shell-exec-audit.md`，对每个 shell-exec 路径评估：
1. **访问控制**: 是否有 denylist 检查？是否有 confirmation？
2. **触发方式**: 是 LLM 触发还是 user 触发？frequency？
3. **测试覆盖**: 现有 unit tests 覆盖多少？
4. **风险等级**: 高/中/低 + 修复优先级
5. **修复建议**: 具体代码改动点 + 估计时间

### Phase 2 — Denylist Extension + Guard Function (1-2 days)
1. 提取 `guard_command_execution(cmd, context) -> Result<(), GuardError>` 统一函数：
   - 调用 `shell_safety::check_command_denied()`
   - 可选触发 confirmation（如果 context 允许）
   - 写 audit log
2. 所有未审计路径强制调用 `guard_command_execution()`
3. 不破坏现有 `bash_tool.validate_input` 的 denylist 检查

### Phase 3 — Mode-based Confirmation + Audit Log (0.5-1 day)
1. **Mode-based confirmation 默认值**:
   - `agentic` / `coding` / `plan` / `multitask` mode：默认 `skip_tool_confirmation: true`（当前行为不变）
   - `admin` / `dangerous` mode（新增）：默认 `skip_tool_confirmation: false`
   - Config schema 新增 `confirmation_mode: enum { Permissive, Strict }`
2. **Audit Log**:
   - 写到 `.northhing/audit.log`
   - 字段：timestamp, mode, command, decision (allow/deny/confirm), reason, session_id
   - 用 `tracing` layer 实现（不引入新依赖）

---

## 3. Non-Goals

- Not adding new denylist patterns (S-1 已有 11 条足够)
- Not changing the existing `BashTool` denylist check behavior
- Not removing the `skip_tool_confirmation` global config option
- Not changing other 25+ tools' `needs_permissions()` impls
- Not implementing distributed/remote shell-exec audit (out of scope)

---

## 4. Design

### 4.1 Phase 1: Audit Pass

**Output**: `docs/security/r1-shell-exec-audit.md`

**Per-path evaluation schema**:
```markdown
### <file path>

- **Access Control**:
  - Denylist check: ✅ / ❌ / ⚠️ partial
  - Confirmation: ✅ / ❌ / ⚠️ partial
  - LLM-triggered: yes / no / hybrid
- **Test Coverage**: <N> unit tests covering <X>%
- **Risk Level**: 🔴 high / 🟡 medium / 🟢 low
- **Trigger Frequency**: <estimated calls/day>
- **Recommended Fix**:
  - Code change: <description>
  - Estimated effort: <hours>
  - Priority: P0 / P1 / P2
- **Test Recommendations**:
  - Add unit test: <description>
  - Add integration test: <description>
```

### 4.2 Phase 2: Guard Function

**New function** in `src/crates/assembly/core/src/agentic/tools/implementations/shell_safety.rs`:

```rust
/// Outcome of a shell command guard check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardOutcome {
    Allowed,
    DeniedByDenylist { pattern: String },
    DeniedByConfirmation { reason: String },
    AwaitingConfirmation { timeout_secs: u64 },
}

/// Comprehensive guard for shell command execution.
///
/// Combines:
/// 1. Denylist check (fail-fast on catastrophic commands)
/// 2. Confirmation gate (if not skipped by context)
/// 3. Audit logging
///
/// Used by all shell-exec paths that aren't already covered by
/// `BashTool::validate_input`.
pub async fn guard_command_execution(
    cmd: &str,
    tool_name: &str,
    context: &ToolUseContext,
) -> Result<GuardOutcome, NortHingError> {
    // 1. Denylist check (synchronous, fail-fast)
    if let Some(pattern) = check_command_denied(cmd) {
        log_audit_event("deny", tool_name, cmd, pattern, &context);
        return Ok(GuardOutcome::DeniedByDenylist {
            pattern: pattern.to_string(),
        });
    }

    // 2. Confirmation check (async if needed)
    if context.skip_tool_confirmation() {
        log_audit_event("allow-skip", tool_name, cmd, "skip_tool_confirmation=true", &context);
        return Ok(GuardOutcome::Allowed);
    }

    // 3. Confirmation gate (async)
    let outcome = request_user_confirmation(cmd, tool_name, &context).await?;
    match outcome {
        Confirmed => {
            log_audit_event("confirm-allow", tool_name, cmd, "user confirmed", &context);
            Ok(GuardOutcome::Allowed)
        }
        Rejected(reason) => {
            log_audit_event("confirm-reject", tool_name, cmd, &reason, &context);
            Ok(GuardOutcome::DeniedByConfirmation { reason })
        }
        Timeout => {
            log_audit_event("confirm-timeout", tool_name, cmd, "user timeout", &context);
            Err(NortHingError::Timeout("Confirmation timed out".into()))
        }
        ChannelClosed => {
            log_audit_event("confirm-channel-closed", tool_name, cmd, "channel closed", &context);
            Err(NortHingError::Cancelled("Channel closed".into()))
        }
    }
}
```

**Usage**: 各 shell-exec 路径在 `Command::new()` 之前调用 `guard_command_execution()`。

### 4.3 Phase 3: Mode-based Confirmation

**New config schema** in `src/crates/assembly/core/src/service/config/types.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfirmationMode {
    /// Skip confirmation for all LLM-triggered commands (current default for coding modes)
    Permissive,
    /// Require user confirmation for all LLM-triggered shell commands
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellSecurityConfig {
    #[serde(default = "default_confirmation_mode")]
    pub confirmation_mode: ConfirmationMode,

    /// Per-mode overrides
    #[serde(default)]
    pub mode_overrides: HashMap<String, ConfirmationMode>,
}

fn default_confirmation_mode() -> ConfirmationMode {
    ConfirmationMode::Permissive
}
```

**Default mapping** (initial implementation):
- `agentic` / `coding` / `plan` / `multitask` / `debug` / `team` / `cowork` / `claw` → `Permissive`
- `admin` / `dangerous` (future) → `Strict`

**Migration**:
- `skip_tool_confirmation: bool` field **deprecated** but kept for backward compat
- New precedence: `ShellSecurityConfig.mode_overrides[mode]` > `ShellSecurityConfig.confirmation_mode` > `skip_tool_confirmation`

### 4.4 Phase 3: Audit Log

**New module**: `src/crates/assembly/core/src/service/audit_log.rs`

**Format**: NDJSON (one JSON object per line)
```json
{"timestamp":"2026-06-23T17:30:00Z","mode":"agentic","tool_name":"Bash","command":"rm -rf build/","decision":"allow-skip","reason":"skip_tool_confirmation=true","session_id":"...","dialog_turn_id":"..."}
```

**Path**: `.northhing/audit.log` (alongside `debug.log`)

**Implementation**: Use existing `tracing` layer (`tracing-subscriber`'s `fmt::layer()` with custom writer).

---

## 5. Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `guard_command_execution` async signature breaks existing sync paths | Medium | Medium | Provide sync wrapper that returns Pending variant |
| Audit log grows unbounded | Low | Medium | Add rotation: keep last 10MB or 7 days |
| Mode override config schema invalid for existing user configs | Low | High | New field with sensible default; old `skip_tool_confirmation` still works |
| Phase 1 audit finds more dangerous paths than listed | Medium | Medium | Audit pass is exploratory; spec may be updated mid-flight |
| Confirmation UX 实际不工作 (UI 端没接) | High | High | **Phase 3 includes explicit UX verification** as part of acceptance |

---

## 6. Acceptance Criteria

### Phase 1
- [ ] `docs/security/r1-shell-exec-audit.md` exists
- [ ] All 9+ shell-exec paths documented with full schema
- [ ] Each entry has: access control, test coverage, risk level, fix recommendation

### Phase 2
- [ ] `guard_command_execution()` exists with unit tests
- [ ] All 8+ un-audited paths call `guard_command_execution()` before `Command::new()`
- [ ] Existing `BashTool::validate_input` denylist behavior unchanged
- [ ] All existing tests still pass

### Phase 3
- [ ] `ConfirmationMode` enum + `ShellSecurityConfig` defined
- [ ] `mode_overrides` map works for coding/admin modes
- [ ] `.northhing/audit.log` is created on first shell-exec
- [ ] Audit log entries have: timestamp, mode, tool_name, command, decision, reason, session_id
- [ ] All existing tests pass
- [ ] `cargo test --workspace --lib` returns 0 failed

---

## 7. Acceptance Commands

```bash
cd E:/agent-project/northhing

# Phase 1: doc-only
ls docs/security/r1-shell-exec-audit.md

# Phase 2
cargo test -p northhing-core --lib shell_safety 2>&1 | tail -3
# Expect: all passed

cargo build -p northhing 2>&1 | tail -3
# Expect: 0 errors

# Phase 3
cargo test -p northhing-core --lib service::config 2>&1 | tail -3
# Expect: all passed

# Audit log verification
ls -la .northhing/audit.log 2>&1
# Expect: file exists after first shell-exec test
```

---

## 8. Out of Scope

- Adding new denylist patterns (S-1 already covers catastrophic commands)
- Changing `BashTool` denylist behavior
- Removing `skip_tool_confirmation` global config
- Distributed/remote shell-exec audit
- Per-user confirmation policies
- Confirmation UI redesign (current UI is assumed working)

---

## 9. Plan Reference

Implementation via LAEP protocol. Tasks to be detailed in impl plan after spec approval.

Estimated total: 3-5 days.

---

**Last updated:** 2026-06-23
**Status:** Awaiting user review before execution.