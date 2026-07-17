---
name: northhing
aliases: [NortHing, northhing]
type: project
domain: T
tags: [rust, agent, desktop, ai, workspace-search]
---

# northhing

**creature**: 面向非技术用户的 AI协作桌面应用
**tag**: v0.1.0-human-usable (2026-07-16)

## Architecture

6 层 modular 设计：Interfaces → Assembly → Adapters → Services → Execution → Contracts

- 编译通过: cargo check --workspace (excl. northhing-acp 10 pre-existing errors)
- 测试: 932/933 pass (--workspace --exclude northhing)
- 桌面: MSVC 构建成功
- CLI: northhing-cli (binary target)

## Key Invariants

- USE_LIGHTWEIGHT_ACTOR = true (A2 activation)
- 0 god-files (>750 lines)
- 0 hand-written unsafe in app_state/
- Cargo.lock tracked (binary workspace)

## Status

- v0.1.0-human-usable tag @ 9ac3757
- GitHub: UmR-2026/NortHing
