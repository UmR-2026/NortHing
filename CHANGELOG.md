# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.10] - 2026-07-16

### Changed
- **Version bump** from 0.2.0 → 0.2.10 to align workspace metadata with the current release train
- `workspace.package.version` and `workspace.metadata.version` both pinned to `0.2.10`

### Notes
- No functional code changes; release-cosmetics-only bump
- Prepares the crate for crates.io publication under the new version scheme

## [0.1.0-human-usable] - 2026-07-16

### Added
- **AGENTS.md restoration** — contributor onboarding document re-committed to repository root
- **Syntect optionalized** behind a `syntax-highlight` crate feature, reducing compile-time and binary size for users who do not need syntax highlighting
- **Roadmap and documentation audit** docs committed (`docs/handoffs/2026-07-16-graph-review.md`, `docs/handoffs/2026-07-16-handoff.md`)

### Changed
- **v0.1.0-human-usable** tagged as the first end-user-release baseline

### Fixed
- **Test pass rate**: 932 / 933 tests pass (pre-existing `northhing-acp` error remains out-of-scope)

### Notes
- CLI ✅ compiles; god-file split (0 god-files with lib + tests >750 lines) still holds
- `cargo fmt` clean across the workspace
- Release-prep sweep removes ad-hoc developer scripts (`.py`, `.cjs`, audit scratch files, phantom `{}`) from version control and hardens `.gitignore` to prevent re-introduction

## [0.2.0] - 2026-07-10

### Added
- **R44-R59 god-object split chain** (assembly/core 大规模重构)
  - R44 (R49 完成): session/turn/round/scheduler/lsp/process/grep/skills-registry/workspace/dispatch/task_execution/search/exec_command/command_mod 共 13 个核心 god-file 拆分，19 commits
  - R50-R59: 45 commits，完成 agent_runtime/compression/insights/edit_file/config_manager/transcript_export/session_restore/message/mcp_tools/session_evidence/computer_use_host/subagent_orchestrator/ports/insights_service/feishu/bash_tool/tool_pipeline/git_tool/scheduler/openai_stream/deep_review_budget/mcp_auth/tools_registry/browser_launcher/scheduler_turn/tools_registry/bash_execute/insights_analyze/pipeline_exec/so_lifecycle/system_actions/acp_requirements/deep_review_report/file_write_tool/code_review_tool/tool_context_runtime/insights_html/lsp_workspace_manager/remote_connect/review_platform_tool/browser_control_actions/cron_tool/snapshot_core/session_persistence/coordination_ports/session_usage_service/prompt_builder_impl/workspace_runtime_service/session_manager_metadata_tests/session_manager_lifecycle_tests 等 ~25 个 god-file 拆分
  - 总计: 64 commits, 511 files, +45,723/-41,423 行, ~300+ sibling files

### Changed
- **QClaw 9/10 APPROVED**: R44-R59 comprehensive review (8 axes 全过)，2 fix commits 已合 main (c5c09ac6 + 180e2813)
- **Kimi 6.5/10 review**: 4 Critical 问题（架构债，v0.2.0 scope），P0/P1/P2 行动清单已记录

### Fixed
- **R60 cleanup**: 70 worktrees 删除, 82 branches 删除, working tree clean
- **测试状态**: `cargo check -p northhing-core --tests` 0 errors（R58 修复 R47-R57 遗留 125 errors）

### Notes
- main HEAD: `8827ed9b`
- 0 god-files (lib + tests >750 行)
- `cargo check --workspace`: 10 pre-existing `northhing-acp` errors (out of scope)
- 详细状态: `docs/handoffs/2026-07-10-r60-closeout-handoff.md`
- Kimi review: `E:\agent-project\review-summary.md` + 8 dimension reports

## [0.1.0] - 2026-06-24

### Added
- **R3**: `SessionStoragePathResolution` enum 类型安全重构
  - `Local { workspace_path }` — 本地工作区存储
  - `Remote { requested_workspace_path, effective_storage_path, remote_connection_id, remote_ssh_host }` — 远程 SSH 工作区
  - `UnresolvedRemote { requested_workspace_path, effective_storage_path, remote_connection_id }` — 未解析的远程工作区
  - 自定义 serde 序列化保持与原来 struct 完全相同的 JSON 格式
  - 新增访问器方法：`effective_storage_path()`, `storage_kind()`, `remote_connection_id()`, `remote_ssh_host()`

### Changed
- **A1**: 修复 149 个 clippy warnings → 降至 15 个
  - 84 个 auto-fixable warnings 自动修复
  - 65 个手动修复（sort_by, field_reassign, redundant_locals 等）
- **B2**: 全局 tracing 迁移（178 文件）
  - `log::` → `tracing::` 统一日志门面
  - 13 个 Cargo.toml 添加 tracing 依赖
- **C2**: 死代码清理（29 warnings）
  - 移除 24 个未使用 import
  - 修复 3 个 unused_variables
  - 修复 1 个 unnecessary_mut

### Fixed
- **A2**: P0 问题调查完成
  - `extract.rs` timeout 问题：文件路径已变化，问题不再存在于当前代码
  - `coordinator.rs` prune_context panic：代码已重构，问题已不存在

### Deprecated
- `SessionStoragePathResolution::new()` 构造函数（已移除，使用 `local()`, `remote()`, `unresolved_remote()` 替代）

### Notes
- 这是一个 MVP 发布版本，包含核心功能稳定
- 测试覆盖：1456+ passed, 0 failed, 2 ignored（v3-restructure 验证）
- 构建状态：CLI ✅ / GUI ✅（desktop 编译通过，测试因 Windows DLL 缺失需排除）
- 环境要求：Windows 上需 `rustup component add clippy`

[0.1.0]: https://github.com/northhing/northhing/releases/tag/v0.1.0
