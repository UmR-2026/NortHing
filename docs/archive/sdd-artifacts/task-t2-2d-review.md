# Task T2-2d Review

## verdict
FAIL

## Findings

### Important — spec / scope: out-of-scope formatting edits
- `src/crates/assembly/core/src/agentic/coordination/dialog_turn/session.rs:216-220` — unrelated reformatting of the existing memory-reminder call. This violates Constraint 4: only delete task-listed code; do not opportunistically refactor hotspot files.
- `src/crates/assembly/core/src/agentic/coordination/dialog_turn/sub_handle_out.rs:405-410` — unrelated reformatting of `active_turn_tasks.insert(...)`, outside the requested deletion.
- `src/crates/assembly/core/src/agentic/execution/execution_engine.rs:150-156` — unrelated reformatting of existing `info!` calls, outside the requested deletion.

### Important — quality/spec evidence gap: `computer_link` downstream compatibility
- `src/crates/assembly/core/src/agentic/tools/implementations/create_plan_tool.rs:228-229` — the `computer_link` JSON field is removed while `user_link` remains. The supplied evidence does not establish that no frontend, desktop, or serialized-output consumer expects the removed field. The brief explicitly requires downstream/serialization/test synchronization. The focused test only proves the local core test.

## Spec judgment
FAIL.

S1–S6 are otherwise substantially aligned: the targeted symbols are absent from assembly, `remote_file_delivery.rs` and its module declaration are removed, deep-research links collapse to workspace-relative paths, and the supplied diff does not touch contracts, SSH semantics, the Bot fallback, services-integrations, or frontend files. However, the exact-scope constraint is violated by unrelated formatting churn, and the required downstream check for the removed JSON field is not evidenced.

## Quality judgment
FAIL.

The deletion is mechanically coherent and the reported MSVC checks, boundary check, and focused tests are green. The assembly targeted-symbol search is zero-hit and remaining `computer://` matches are confined to the allowed `services-integrations` C3 scope. Quality still fails because unrelated formatting increases review surface in hotspot files and the `create_plan_tool` output-contract change lacks evidence for all consumers.

## Cannot verify from diff — orchestrator must verify
1. No frontend/desktop/serialization consumer depends on `create_plan_tool`'s removed `computer_link` field; inspect consumers and run the relevant checks.
2. Runtime deep-research link behavior for local and remote workspace execution, including the intended post-C1 unreachable RemoteRelay producer path.
3. Feature-combination behavior beyond the reported `product-full` tests; verify no alternate feature retains callers of the removed builder method/module.
4. Verify actual working-tree diff/status for contracts, SSH fields, and `coordination/subagent_ports.rs:113` rather than relying only on the artifact.

## Evidence reviewed
- Spec: `.superpowers/sdd/task-t2-2d-brief.md`
- Implementer report: `.superpowers/sdd/task-t2-2d-report.md`
- Diff package: `.superpowers/sdd/task-t2-2d-diff.md` (BASE `9c14d22`)
- Targeted search: `src/crates/assembly` has zero hits for `remote_file_delivery|computer_link|computer://|TOOL_CONTEXT_REMOTE_FILE_DELIVERY|needs_computer_links`; remaining `computer://` matches are in `src/crates/services/services-integrations`.
