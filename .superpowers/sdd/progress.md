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

- Task B2: complete (commits 4f45f14..7a4bdca, review 一轮双 PASS 0C/0I/3M) — FU-2: uninstall_plugin 先经 registry 解析 plugin_id→全部 languages（先于 unregister）再逐个 stop_server（先于 loader 删文件）；shutdown() plugin_ids→languages 改名；manager +2 tests（方案 A 端到端：即退 dummy 进程走真实 spawn，规避 shutdown 60s 硬超时；cfg(windows)/cfg(not(windows)) 双分支）。core lib 1139/1139。观察项: uninstall_plugin 全仓暂无生产调用方。终审 triage: Minor-1 stop_server 恒 Ok 使新 warn 分支不可达（pre-existing）；Minor-2 commit body 未记改名；Minor-3 测试两 dummy 共用 id。

- Task B3: complete (commits 7a4bdca..755a503 = b0bfe43 + 755a503, review 一轮 4/4 判决 PASS 0C/0I/1M) — C-A build fix: keyring v1 feature 使能 + keyring.rs 3 行 API/Lazy 编译修复 + provider_test.rs 导入路径（**重大发现：P1-C3 合入后 desktop 从未编译过**，judge 逐行核定零行为变化、无历史凭据故 UTF-16 编码差异无兼容影响、Cargo.lock +4 包全为 v1 feature 依赖）。C-B FU-3: 公共 load_app_settings 全程持 SETTINGS_WRITE_LOCK（dedup 写 + keyring 迁移写整窗在锁内）、`_at` 保持无锁避免 tokio Mutex 重入死锁，用户拍板 (a) 锁住公共 load（计划字面"load 纯读"写于 C3 前，偏离已在 commit message + brief §0 声明）；FU-4: dead save_app_settings wrapper 删除（全仓无调用方，warning 归零）。desktop lib settings 79 passed，新基线 118/118。终审 triage: Minor-1 callbacks_settings/mod.rs:29 注释仍引用已删的 save_app_settings；观察项: keyring.rs 5 个 C3 前 test-only dead-code warning、Windows keyring UTF-16 编码细节建议记台账。

- Task B4: complete (commits 6868377..50b0f44, review 一轮双 PASS 0C/0I/3M) — FU-5: initialize_global 套用 6574b01 的 double-checked locking（新 static AI_CLIENT_FACTORY_INIT_MUTEX = std::sync::OnceLock<tokio::sync::Mutex<()>>，fast path 免锁 → 取锁 → 锁内 double-check → fallible work 全部在唯一 OnceLock::set 之前），消除并发后到者拿到伪 Err("Failed to initialize global AIClientFactory") 的 TOCTOU；P0-E 五条计时日志逐字保留。测试取方案 B：双检锁骨架抽为 module-private helper `init_once_with`，两个新测试覆盖 8 并发 build 恰一次（multi_thread flavor）+ build 失败后 cell 保持空且重试成功；方案 A（并发跑 initialize_global 本体）因进程级 OnceLock 与 lib 测试二进制共享、初始化后会让 subagent_ports spawned task 在有真实凭据机器上发起真实 LLM 调用而不 hermetic（judge 独立取证确认，参照 6574b01 B-2 组决策）。core lib 1141 总（基线 1139 + 2），编排者独立实跑 focused 测试复核一致。implementer=coder-dv4f（首次实证，汇报与磁盘一致无造假）/ judge=judge-m3。终审 triage: Minor-1 report 自述行数 592 实为 589；Minor-2 并发测试 cell.get() 断言冗余（无害）；Minor-3 init_once_with 未来若 global.rs 复用可上抽 util 模块。

- Wave1 分支终审: PASS | PASS (merge-base 41695f5..e6be249, judge-m3 终审视角, 0 Critical / 0 Important / 0 新增 Minor) — 独立盘点 6 把锁的拓扑与调用链，确认 B1 write_lock / B3 SETTINGS_WRITE_LOCK / B4 AI_CLIENT_FACTORY_INIT_MUTEX 与 core 侧 ConfigService::manager RwLock、GLOBAL_CONFIG_SERVICE、GlobalConfigManager::INIT_MUTEX 之间无锁序反转、无嵌套持锁 await、无死锁环（desktop settings 锁内不含 sync_providers_to_core，MCP 与 settings 路径完全隔离）；B3 偏离三处声明一致且 C3 fail-closed 语义零漂移；b0bfe43 build fix 零行为变化 + Cargo.lock +4 全为 v1 链式依赖；接受 B4 测 helper 的等价替代。副作用：MCP write_lock 轻微 lock convoy（低频写，可接受）。triage 裁定：B1-M2 + B3-M1 合并前修（→ commit 8f921cc，coder-ling 机械单，cargo check -p northhing 通过）；B1-M1（save_config 非原子写）+ B4-M3（init_once_with 上抽）+ uninstall_plugin 无生产调用方 → 登记独立债项；B2-M1/M2/M3 + B4-M1/M2 忽略（pre-existing 或 cosmetic）；keyring test-only warning + Windows UTF-16LE 编码细节 → 记台账。**待用户拍板**：P1-C3 过程性缺陷（desktop 自 C3 合入后从未编译，b0bfe43 事后修复）是否登记独立债项 / 流程改进项（如 merge to main 前 desktop cargo check 必须通过）。证据：wave1-final-review-brief.md / wave1-final-review.md / wave1-final-review.diff。
