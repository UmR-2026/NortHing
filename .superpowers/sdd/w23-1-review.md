# W23-1 Review — 文档/注释诚实化批

## SPEC: PASS
## QUALITY: PASS

## Findings

无

## Cannot verify from diff

- 无（report §1 两条命令 exit 0 + 文本一致；报告本身含本地绝对路径属报告作者卫生范畴，不在 commit `bd271a6` 的 4 文件 diff 内）。

## 范围变动

- 无。`git diff --name-only c8f1023..bd271a6` = `AGENTS-CN.md` / `AGENTS.md` / `docs/status/tech-debt-ledger.md` / `src/crates/support/cli-internal/src/main.rs`，与 brief 允许文件集 1:1 一致，与 allowlist 1:1 一致。

## 证据要点（备查）

- 分层表第 7 行 support 已补（Owns = 测试支撑与内部工具（豁免生产分层约束的辅助 crate），Modules = `test-support`, `cli-internal`，Layer doc = none/无）；contracts 行 Modules 追加 `kernel-api`, `disposable`；services 行 Modules 追加 `debug-log`；interfaces 行 `acp` 已存在未动。表下追加机械对照句「以根 `Cargo.toml` workspace members + `scripts/core-boundaries/rules/crate-layout.mjs` 为准」。AGENTS-CN.md 同步镜像同两处。
- 验证表「Shared Rust logic in `core`, adapters, or services」行：原文 `cargo check --workspace`, plus the nearest focused `cargo test` when behavior changed 整段保留（"when behavior changed" 指引未删），末尾追加「（`northhing-core` 定向测试须带 `--features product-full`，裸跑 E0433）」。CN 镜像同改。
- `docs/status/tech-debt-ledger.md:260-266` 新增 P2-25 五字段条目：Symptom / Mitigation / Evidence / Proposed fix / Status 顺序与既有 P2-19~P2-24 既有四字段顺序兼容（Mitigation 插在 Symptom 之后，符合 brief 字段序）。Evidence 行精确行号经实地核对：
  - 函数本体 `:225-248`：`pub async fn guard_command_execution`(:225) → 函数尾 `}`(:248) ✓
  - `"allow-stub"` 审计 `:244`：`log_audit_event(tool_name, cmd, "allow-stub", ...)` ✓
  - Phase 3 未接线自曝注释 `:238-239`：`// 2. Confirmation gate (Phase 2 stub: always skipped)` + `// Phase 3 wires the actual confirmation flow via request_user_confirmation` ✓
- Mitigation 引用 `process_result.rs:225-234` 实为 AND 语义注释段（行 225-234 内容为「AND semantics: skip confirmation only when BOTH shell_security AND legacy flag agree」），与 stub 缓解事实自洽 ✓
- Change Protocol 段未改（brief 要求不改该段，diff 仅 +8 行 ledger，Change Protocol 区域无 - 行）✓
- `src/crates/support/cli-internal/src/main.rs` 改动严格限于模块文档 SECURITY 段（diff 唯一新增两行 `//! Capability token gate is format-only (length ≥ 32); cryptographic validation deferred — see verify_capability_token`），零代码行变更 ✓
- commit message：`docs: document and comment honesty updates (W23-1)` + body 注明 ZCode 审查 A1/D3/Unguided#1/Unguided#2 + 用户拍板 2026-09-09 ✓
- 报告三节齐全（改动摘要 / 验证 / 状态），状态词 `DONE` 明确 ✓
- Cargo.toml `workspace.members` 25 条目逐一对账（report §3 25 行表）与 brief 要求的「25 members」一致 ✓
