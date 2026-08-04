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

---

# P1 Security Round Ledger (2026-08-04)

计划：`.superpowers/sdd/plan-2026-08-04-p1-security.md`
分支：`fix/p1-security-0804`（worktree `.worktrees/p1-security-0804`，基线 ae44334）

- Task C1: complete (commits ae44334..3404060, review 双 PASS；fix 1 轮：report 确认门结论捏造被打回重写) — trash crate 默认回收站 + fail-closed + permanent 开关；88 tests；副产物：P1-6 新债入库（DeleteFileTool needs_permissions=false 绕过确认门，remote rm -rf 无确认）

- Task C2: complete (commits 3404060..7fa7d62, review 双 PASS 0C/0I, 6 Minor) — standalone relay loopback 默认 + 自动 key 生成（~/.northhing/relay/api_key, 0600 + 原子写）+ RELAY_BIND env + 非 loopback 无 key 启动 fail-closed + CORS 收紧（移除了 relay-core lib.rs 硬编码 CorsLayer::permissive()，cors_allow_origins 字段原本未接线已补接）+ embedded relay 启动 warn + ledger P1-5 resolved / P1-7 active。61 tests。

- Task C3: complete (commits 7fa7d62..f42451d, review 双 PASS, 1I/10M；fix 1 轮: I-1 环境约束显式化 + M-1/M-3/M-4 计数与措辞修正 + M-6 新增 P1-8 MCPServerConfig.env 明文 concern + M-8 并发测试 final-state 加固) — ProviderConfig.api_key 迁移 OS keyring (v4.1.6, windows-native-keyring-store)；KeyringBackend trait + Production/Mock 双实现；load 路径迁移 + sentinel + idempotent + fail-closed (ring/aws-lc-sys gcc 缺失环境约束已显式记录, CI 覆盖)。副产物：P1-7 (embedded relay key threading, C2) + P1-8 (MCPServerConfig.env plaintext, C3 fix 轮发现)。

---

# Backend Follow-ups Round Ledger (2026-08-05)

计划：`.superpowers/sdd/plan-2026-08-04-backend-followups.md`
分支：`fix/backend-followups-0804`（worktree `.worktrees/backend-followups-0804`，基线 41695f5）
选派：implementer coder-qw / 任务 judge judge-qw（用户 2026-08-05 指定 qwen 线；探针通过）

- Task B1: complete (commits 41695f5..808ed65, review r2 双 PASS；fix 1 轮: r1 SPEC FAIL 唯一阻塞项=计划要求的"并发写不丢条目"测试缺失，用户拍板 (a) 加锁+并发测试 → 808ed65 补 tokio Mutex 串行化 user/project 读-改-写 + 3 并发用例，judge 对照无锁 BASE 实证用例能抓 lost-update) — FU-1: 层 A CoreMCPConfigStore.get_config_value 错误分类（NotFound=空态/其它=Err 中止写）+ 层 B save_user_config/delete_server_config 未识别格式 fail-closed（镜像 load_project_configs_strict）。integrations +7 / core +2 tests。终审 triage: Minor-1 save_config 非原子写待登记独立债项；Minor-2 台账 FU-1 注记 "+4" 应为累计 "+7"。
