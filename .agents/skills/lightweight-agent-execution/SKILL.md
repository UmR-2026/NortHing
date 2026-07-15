---
name: lightweight-agent-execution
description: Use when dispatching a lightweight agent (4B-level model) to execute a well-bounded coding task with minimal human intervention. The task must have clear boundaries (can_modify/no_touch), verification commands, and produce structured outputs for automated review.
---

# Lightweight Agent Execution Protocol (LAEP)

Execute a coding task using three lightweight models in sequence: Coding → Testing → Review. Each model produces a structured output consumed by the next.

## When to Use

- Task has clear file boundaries (can_modify / no_touch defined)
- No architectural decisions needed
- Can be verified by `cargo check` / `cargo test`
- Human review is a quick confirmation, not deep analysis

## When NOT to Use

- Task requires architecture design
- Task needs cross-module coordination
- Task boundaries are unclear
- Task requires user interaction during execution

## Prerequisites

1. **Taskfile.toml exists** at `.task/Taskfile.toml`
2. **Schemas exist** at `.task/schemas/`
3. **Git workspace is clean** (or changes are isolated)

## The Process

```
Phase 1: Coding Model
  Input: Taskfile.toml
  Output: .task/change-log.json
  
Phase 2: Testing Model
  Input: Taskfile.toml + change-log.json
  Output: .task/verification-report.json
  
Phase 3: Review Model
  Input: Taskfile.toml + change-log.json + verification-report.json
  Output: docs/reviews/YYYY-MM-DD-<task>-review.md
```

## Phase 1: Coding Model

**Prompt:** Use `.agents/skills/lightweight-agent-execution/coding-prompt.md`

**Key rules:**
- Only modify files in `can_modify` list
- Never touch files in `no_touch` list
- Generate `change-log.json` at the end
- Report DONE, DONE_WITH_CONCERNS, or BLOCKED

## Phase 2: Testing Model

**Prompt:** Use `.agents/skills/lightweight-agent-execution/testing-prompt.md`

**Key rules:**
- Do not modify code
- Execute all commands in `Taskfile.verification.commands`
- Generate `verification-report.json`
- Report PASS, FAIL, or BLOCKED

## Phase 3: Review Model

**Prompt:** Use `.agents/skills/lightweight-agent-execution/review-prompt.md`

**Key rules:**
- Do not run any commands
- Do not re-verify
- Read structured outputs, format for human readability
- Generate `review-guide.md`

## Pause Conditions

| Phase | Pause Condition | Action |
|-------|-----------------|--------|
| Coding | Touches `no_touch` file | STOP, report violation |
| Coding | Compile errors after 3 attempts | STOP, report BLOCKED |
| Testing | Compile FAIL | Return to Coding Model for fix |
| Testing | Test FAIL | Return to Coding Model for fix |
| Testing | Boundary violation | STOP, report NEEDS_HUMAN |
| Review | Never pauses | Always completes |

## Output Files

| File | Phase | Description |
|------|-------|-------------|
| `.task/change-log.json` | Coding | Structured change record |
| `.task/verification-report.json` | Testing | Verification results |
| `docs/reviews/YYYY-MM-DD-<task>-review.md` | Review | Human-readable review guide |

## Integration with Existing Skills

- **Before LAEP:** Use `superpowers:writing-plans` to create the plan
- **After LAEP:** Use `superpowers:finishing-a-development-branch` to merge
- **Alternative:** Use `superpowers:subagent-driven-development` for complex tasks requiring heavy models

## Quick Reference

```bash
# Create a new task
cp .task/templates/Taskfile.toml .task/Taskfile.toml
# Edit Taskfile.toml for your task

# Run LAEP (three phases)
# Phase 1: Coding Model (dispatch with coding-prompt.md)
# Phase 2: Testing Model (dispatch with testing-prompt.md)
# Phase 3: Review Model (dispatch with review-prompt.md)

# Clean up after task
rm .task/change-log.json .task/verification-report.json
```

## Schema References

- `change-log.json`: `.task/schemas/change-log-v1.schema.json`
- `verification-report.json`: `.task/schemas/verification-report-v1.schema.json`

## Version

LAEP v1.0 (2026-06-23)
