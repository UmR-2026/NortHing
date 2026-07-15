---
name: documentation-and-adrs
description: "Use when making architectural decisions, recording the rationale for v3 changes, or upgrading PROJECT_STATE.md. Provides ADR (Architecture Decision Record) templates and documentation standards. Trigger this when introducing a new const flag pattern, changing module boundaries, or completing a v3 phase."
---

# Documentation & ADRs (northhing v3 适配— > 来源：addyosmani/agent-skills documentation-and-adrs

## When to trigger

- 引入新的 const flag 模式（需要记— 为什么用 const flag 而非 cfg/运行时配— - 完成 v3 phase 后更新项目文— - 做出影响— crate 的架构决— - PROJECT_STATE.md / HANDOFF.md 需要重大更— ## ADR 体系

### 目录结构

```
northhing-v3/
 docs/
 decisions/— ADR 存放— adr-001-const-flag-pattern.md
 adr-002-regression-test-per-flag.md
 adr-003-skill-pointer-mode.md
 ...
 PROJECT_STATE.md
 CODE_REVIEW.md
 HANDOFF.md
```

### ADR 模板

```markdown
# ADR-NNN: <决策标题>

**Status**: Accepted | Superseded by ADR-MMM | Deprecated
**Date**: YYYY-MM-DD
**Phase**: v3.x

## Context

<为什么要做这个决策？面临什么问题？>

## Decision

<做了什么决策？具体方案是什么？>

## Alternatives Considered

### Alternative A: <名称>
- 优点: <...>
- 缺点: <...>
- 为什么没— <...>

### Alternative B: <名称>
- 优点: <...>
- 缺点: <...>
- 为什么没— <...>

## Consequences

- 正面: <...>
- 负面: <...>
- 缓解措施: <...>

## Rollback

<如何回滚这个决策？const flag 改为 false？git revert— ```

### 已有 ADR 建议（从 v3 已完成工作中提取— | ADR | 标题 | 对应 Phase |
|-----|------|-----------|
| ADR-001 | 采用 const flag 模式— prompt loading 重构 | v3.0 |
| ADR-002 | 每个 const flag 配套 regression test | v3.0 |
| ADR-003 | const flag 默认 false（safe default— | v3.0 |
| ADR-004 | Collapse gstack skills into bundle entry | v3.1 |
| ADR-005 |— Skill pointer 替代 inline memory block | v3.2 |
| ADR-006 | Exclude ProjectLayout from default context | v3.3 |

## PROJECT_STATE.md 升级建议

从扁平状态文件升级为包含 ADR 引用的结构化文档— ```markdown
## 当前状— ### 已完成的 const flags
- [x] `DISABLE_COLLAPSED_TOOL_LISTING_REMINDER` (v3.0-C)— See ADR-001
- [x] `DROP_AGENT_DEFAULT_TOOLS_IN_LISTING` (v3.0-B)— See ADR-001
- [x] `COLLAPSE_GSTACK_SKILLS_IN_LISTING` (v3.1-E)— See ADR-004
- [x] `USE_MEMORY_SKILL_POINTER` (v3.2-A)— See ADR-005
- [x] `INCLUDE_PROJECT_LAYOUT_BY_DEFAULT` (v3.3-D)— See ADR-006

### 总计 token 节省
~6,500-9,500 tokens/turn

### 进行— (— ```

## HANDOFF.md 改进建议

基于 mattpocock/skills handoff 的理念：

1. **增加 "Suggested Skills" 字段**— 告诉接手 agent 应该加载哪些 skill
2. **引用而非重复**— 不重— commit message— PROJECT_STATE 内容，只引用路径
3. **保持精简**— HANDOFF.md 应该— 5 分钟速读的入口，不是百科全书

## Documentation for Agents— AGENTS.md— PROJECT_STATE.md 中维护以下项目约定：

| 约定 | 内容 |
|------|------|
| 构建命令 | `cargo build/test -p <crate>` |
| 测试基线 | 821+ tests, must pass |
| 禁止事项 | northhing-desktop build, v1 docs, push |
| const flag 规范 | 默认 false, 有注— regression test |
| commit 格式 | `<type>(<scope>): v3.x <letter> <desc>` |
