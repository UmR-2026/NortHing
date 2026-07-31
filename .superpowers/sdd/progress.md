# Backend Debug Progress Ledger

计划：`northing-full-bug-audit-2026-07-31.md` + 补充报告（后端方向 8 任务）
分支：`fix/backend-debug-0731`（worktree `E:\agent-project\.worktrees\northing-backend-debug`）
基线：c6096cb (main, 2026-07-31)

| Task | 状态 | Commits | 备注 |
|---|---|---|---|
| Task 1: complete (commits 1c41dc0..e3d0e53, review clean) | 24/24 tests; triage: Q-3 redundant guard, Q-4 mergeable loops, M-4 test gap, M-5 TOCTOU theoretical |
| Task 2: complete (commit 5971c54, review clean) | 37+7 tests; triage: 4 Minor (see task-02-review.md), capability token deferred |
| Task 3: complete (commit ab6a91a, review clean) | 49/49 tests; V-1 dynamic closure (judge independently confirmed); triage: SPA fallback 200 (product decision), axum version dependency |
| Task 4: complete (commit 88c719a, review clean) | 16+1 tests; triage: 5 Minor (see task-04-review.md) |
| Task 5: complete (commit a53711e, review clean) | 7+62 tests; triage: 6 Minor (see task-05-review.md) |
| Task 6: complete (commit 64c64dc, review CLEAN) | mcp 11+miniapp 29+sync 1 tests; triage: save_user_config same fail-open |
| Task 7: complete (commit 9be74ec, review clean) | 59 settings tests; triage: 3 Minor (dead wrapper / upsert UI regression / dedup narrow race) |
| Task 8: complete (commit 1a65fc1, review CLEAN) | 12 lsp tests + M-2 warning cleared; Minor: cleanup_temp_dirs dual-mode note |
| Final review: PASS (c6096cb..1a65fc1, judge-glm, CAN MERGE, 0 Critical/Important, 18 tech-debt / 16 accept) | high-priority tech-debt: T6 save_user_config fail-open, T8 M-8 stop_server path bug, T7 M-3 dedup unlocked write, T7 M-1 dead code |
| Regression sweep (2026-08-01): relay 49/49, integrations 172/172, desktop 98/98, core 1128/1134 (6 failed = pre-existing on main, confirmed by baseline run on E:\agent-project\northing: subagent_ports cancel/timeout x3 + auto_memory prompt_injection x3) | no regression introduced by branch |
| Task 9: complete (commit 6574b01, review APPROVED WITH NOTES by judge-m3) | target 14/14; full 1134/1134 x2 no flaky; cargo check -p northhing pass. B-1 (product bug): GlobalConfigManager::initialize -> INIT_MUTEX double-checked locking + fallible-work-first, removes OnceLock TOCTOU + irreversible half-init; B-2: subagent_ports ensure_global_config_for_tests no longer inits AIClientFactory (cancel true cause = execution_task ~0.84s LLM network roundtrip > 50ms cancel window, independent verification overturned brief TOCTOU attribution); A: default_memory_db_path cfg(test) thread-local seam + RAII MemoryDbPathGuard. Minor to triage: assert_secondary_fields_populated _expected_text dead-code (pre-existing), memory_db.rs 918 lines near 1000 god-file threshold. FYI: AIClientFactory::initialize_global same TOCTOU unfixed (follow-up). |
| Regression re-sweep (post-Task-9, 2026-08-01): core 1134/1134 x2 (was 1128 + 6 fixed); relay/integrations/desktop unaffected | DECISION: merge --no-ff to main; 5 high-priority tech-debt opened as follow-ups (see tech-debt-followups.md: FU-1 save_user_config fail-open / FU-2 LSP stop_server mapping / FU-3 dedup unlocked write / FU-4 dead wrapper / FU-5 AIClientFactory TOCTOU) |
