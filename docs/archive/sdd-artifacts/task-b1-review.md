SPEC: FAIL
QUALITY: PASS

# Task B1 审查报告 — FU-1 save_user_config / delete_server_config fail-closed

审查对象：commit d4b11b5（BASE 41695f5，单 commit，4 文件 +210/-29）。
独立取证：全 diff 复核 + 计划 §Task B1 / FU-1 BASE 原文 / 层 A、层 B 生产文件全文 / ConfigService 读链（service.rs、mgr_validate.rs、mgr_load.rs、app_shell.rs、errors.rs）/ 测试文件全文与基座。implementer 报告仅作线索，其结论已逐条对照源码复核。

## 一、Spec 逐条核对

1. **错误分类按 ErrorKind** — PASS。`classify_config_read`（assembly/core/src/service/mcp/config/service.rs:24-35）：Ok→Some；`NortHingError::NotFound`→Ok(None)；其它→Err(MCPRuntimeError::configuration)。已独立核实 ConfigService 实际语义：`config::<Value>(Some(key))`（service/config/service.rs:74-87）→ `ConfigManager::get`（mgr_validate.rs:8-16）→ `get_value_by_path_from_config`（mgr_validate.rs:115-129），缺 key → `NotFound`；`GlobalConfig.mcp_servers` 为 Option+skip_serializing_if（app_shell.rs:56-57），未写入时键不存在 → NotFound=合法空态成立。其它错误（含未来可能的 Io 变体，errors.rs:37）一律中止写——分类正确且防御性。报告 §2 结论与独立核对一致。
2. **delete_server_config 纳入范围** — PASS（services-integrations/src/mcp/config/service.rs:257-295）：None→not_found 保留；未识别格式→Configuration fail-closed，不再与 not_found 混同。
3. **镜像 load_project_configs_strict** — PASS：语义/措辞镜像 :128-148（user-level 措辞），未发明新模式。
4. **读取注入 IO 错误 → fail-closed 且既有配置不丢（测试）** — PASS：integrations +4（config_and_server_lifecycle.rs:153-249）+ core lib +2（assembly core service.rs:185-239）。断言强度：错误 kind + 文案 + set_config_value 未被调用 + 既有值 json!(42) 原样保留。测试有效性见 Important-1 之外的 Minor-2。
5. **并发写不丢条目（测试）** — FAIL。见 Important-1。
6. **验证命令** — PASS。报告 §4.1-4.3 含原文输出（integrations mcp 过滤 44 passed/0 failed 含新增 4；core lib mcp 13 passed/0 failed 含新增 2；cargo check northhing-core 0 error）。按纪律未重跑。
7. **范围外路径未动** — PASS：diff 仅 4 文件；project 级 save_project_config/load_project_configs_strict（:241-255/:128-148）与 load_user_configs/load_all_configs（:57-121）逐行未变。读侧宽容兜底在层 A 收紧后仍成立：真实读错误经 load_user_configs 的 `?`（:87）上抛、被 load_all_configs warn+empty 捕获（:59-65）；既有用例 keeps_load_failures_as_empty_baseline 未改动仍绿（报告 §4.1）。
8. **同 commit 翻转台账** — PASS：`git grep -c resolved` BASE=0 → HEAD=2（全局状态行 + FU-1 段落说明）；FU-2..FU-5 仍 open。
9. **纪律** — PASS：仅提交范围内文件（git status 仅未追踪审查工件）；diff 无格式化噪声（全部 hunk 为逻辑改动）；日志/错误文案 English-only、无 emoji；生产文件 240/296 行 <800；commit message conventional 前缀 + 中文正文，风格符合。

## 二、Findings

### Critical
无。

### Important
1. **spec 要求的「并发写不丢条目」测试缺失，且未声明偏离。**
   - spec 依据：计划 §Task B1 测试行「读取注入 IO 错误 → fail-closed 且既有配置不丢；**并发写不丢条目**」；债清单 FU-1 验证行「新增**并发写** + 读取注入 IO 错误的测试，断言 fail-closed 且不丢既有配置」。
   - 证据：commit 无任何并发测试（tests/config_and_server_lifecycle.rs 全文 564 行无 concurrent 用例；services-integrations tests 目录 grep join_all/spawn/concurrent 无匹配）；报告 §7 自称「无其它偏离」，此项遗漏在报告与 brief 中均未提及。
   - 静态分析（说明缺失不等于"忘记写"）：save_user_config 的读-改-写跨 get（service.rs:213）→ set（:229-236）两次独立 await；MCPConfigService 无锁（struct 仅 config_store，:24-26）；生产路径 ConfigService 的 RwLock 按调用持有（config 取读锁 service/config/service.rs:78；set_config 取写锁 :99），读-改-写窗口不持锁 → 两个并发 save 可丢条目（lost update）。spec 要求的"N 并发 save 断言 N 条目"测试在当前实现下大概率失败。仓库先例：remote_connect bot persistence_tests.rs:40 `concurrent_updates_do_not_lose_entries`、desktop settings io_tests.rs:43 `concurrent_updates_preserve_all_writes`。
   - 性质：plan-mandated 用户决策项——计划"修复方向"只给了 ErrorKind 分类 + 原子落盘核查，未提供保证并发不丢条目的机制，但"测试"行要求该测试。
   - 修复建议（二选一，需用户/编排者定夺）：
     a) 补串行化 + 测试：在 services-integrations MCPConfigService 内用 tokio Mutex 包住 user 级（对称含 project 级）读-改-写，或在 store 层提供原子更新接口；新增并发写测试断言条目不丢（镜像 `concurrent_updates_do_not_lose_entries`）。
     b) 显式 descope：修订计划/债文档，从 FU-1 验证要求移除「并发写不丢条目」并记录理由，同时在 tech-debt-followups.md 新登记"mcp_servers 读-改-写 lost update"独立并发债项。

### Minor
1. **写入非原子观察项未登记债台账。** `save_config`（mgr_load.rs:146-162）直接 `fs::write`（:158），非 temp+rename。报告 §5.1 的暂缓理由成立（唯一落盘点被全 key 共享、触及"Config single source of truth"骨干、改造面大；brief §1 明示出口允许记观察项），但结论仅存于报告与 FU-1 翻转注记，tech-debt-followups.md 未登记独立债项，存在丢失风险。建议：新登记 FU 项"GlobalConfig 文档原子落盘（参照 services-core json_store::write_atomic 模式）"。
2. **两个 trait 层读错误测试在 BASE 上也会通过。** save/delete user 读错误用例（config_and_server_lifecycle.rs:153-178 / 208-227）依赖 store 直接返回 Err，而 BASE 代码 get_config_value 后本就有 `?`（已对 BASE 源码核实）——它们是防未来层 B 引入宽容的回归护栏（镜像 project 级用例），非本单修复的证明。本单修复的真实证明在 classify_config_read 三分支用例（直击旧 `Err(_)→Ok(None)` 吞错行为）与两个未识别格式拒写用例（BASE 上旧 save 从空 map 重建、旧 delete 返回 not_found kind，均已对照 BASE 源码确认新测试在 BASE 会失败）。无需改动，记录在案。

## 三、Quality 判决依据（PASS）

- 错误语义一致：save/delete 对未识别格式用同一拒写文案；delete 严格区分 None→not_found、未识别→Configuration、条目缺失→not_found；classify 错误消息含 key 名便于定位。
- 未引入新 fail-open；读侧宽容面（load_all_configs 兜底）按 spec 保持。
- 命名清晰；classify_config_read 提为纯函数可单测；注释解释"why"。
- 无新 unwrap/panic 入生产路径（expect/unwrap 仅在测试）；无明文密钥；分层合规（assembly→services 依赖方向既有，services-integrations 未反向依赖 core）。
- god-file 线：240/296 行 <800。

## 四、实际运行的复核命令（只读）

- `git status` / `git log --oneline -5` / `git log -1 --format=...` / `git diff --stat 41695f5..d4b11b5` → 单 commit（parent=41695f5），4 文件 +210/-29；工作区仅未追踪审查工件。
- `git grep -c resolved <BASE|HEAD> -- .superpowers/sdd/tech-debt-followups.md` → 0 → 2（翻转核实）。
- 提供的 task-b1-review.diff（UTF-16LE）解码后与 `git diff 41695f5..d4b11b5` 输出逐字比对 → IDENTICAL（14095 字符）。
- `git show 41695f5:.../mcp/config/service.rs`（层 B BASE 源码）→ 核实旧行为与新测试有效性。
- 全仓 `git grep "fn .*concurren"` 与 tests 目录 spawn/join 检索 → 无 MCP 配置并发写用例；确认仓库既有并发测试先例位置。
- 按纪律未重跑 implementer 已跑且报告含原文输出的三条 cargo 命令。

## 五、Cannot verify from diff

1. 「并发写测试在当前实现下会失败」为静态分析预测（依据：读-改-写窗口无锁，见 Important-1 证据链）；确证需实际编写并运行该测试，超出 judge 只读范围。若走修复路径 (a)，以 fixer 提交后的测试实跑输出为准。
2. 三条 cargo 验证命令的运行时通过状态依赖报告 §4 原文输出（按审查纪律采信，未重跑）。