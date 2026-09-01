# Task Brief — W14-1e: 修复测试污染开发机真实记忆库

仓库：`E:\agent-project\NortHing`（main，BASE = `8c00962`）。

## 背景（编排者已磁盘复核，直接采信）

`default_memory_db_path()`（`src/crates/assembly/core/src/service/agent_memory/memory_db.rs:709`）默认指向**开发机真实用户配置目录**下的 memory SQLite 库。
`with_test_memory_db_path(path) -> MemoryDbPathGuard`（同文件 `:762`，文档见 `:749`）是 **RAII 守卫，在持有期间把 `default_memory_db_path()` 重定向到隔离路径**；`unique_test_memory_db_path()`（`:769`）生成唯一临时路径。

**已确认的缺陷**：`src/crates/assembly/core/src/service/agent_memory/auto_memory.rs:573` 的测试
`build_query_aware_facts_reminder_returns_some_with_matching_fact` **没有持有守卫**，却 import 并调用了
`default_memory_db_path()`（`:576` import、`:582` `let db_path = default_memory_db_path();`）→ 该测试会**打开并写入开发机真实的 memory.db 且不清理**。

同文件正确示范（`:431`、`:464`、`:483`、`:506`）：
```rust
let _db_guard = with_test_memory_db_path(unique_test_memory_db_path());
```
（守卫生效后，测试里继续调 `default_memory_db_path()` 是安全的——它已被重定向。）

## Spec

1. **修复主目标**：给 `auto_memory.rs:573` 那个测试补守卫（作为函数体第一行）。**不要**改它调用 `default_memory_db_path()` 的那行——守卫会重定向它。
2. **全仓核对（仲裁附带条件，必做）**：以下**测试侧** `default_memory_db_path()` 调用点，逐个确认是否处在守卫持有期内；漏了就补，补法与上面一致：
   - `src/crates/assembly/core/src/service/agent_memory/facts.rs:668`、`:704`（同一测试，`:661` 已 import 守卫）
   - `src/crates/assembly/core/src/service/agent_memory/facts.rs:731`（`:724` 已 import 守卫）
   - `src/crates/assembly/core/src/agentic/session/session_manager_lifecycle_tests/continuity_selfcheck.rs:187`、`:288`（`:22` 已 import 守卫）
   - `src/crates/assembly/core/src/kernel_facade/memory.rs:63`、`:102`（各自函数内 import）
   确认结果逐条写进 report（"已有守卫 / 已补 / 生产代码无需改"）。
3. **不许动生产代码路径**（这些是运行时真实行为，不是缺陷）：
   - `auto_memory.rs:246`、`:302`
   - `dream.rs:38`
   - `turn_persist.rs:457`
   若你判断其中某处其实也是测试代码，先 BLOCKED 上报，不要自作主张改。
4. **不许**改 `default_memory_db_path` / `with_test_memory_db_path` / `unique_test_memory_db_path` 本身的实现。
5. 若发现**新的**未受保护的测试侧调用点（不在上面清单里），一并补上并在 report 点名。

## 验证（命令 + 输出原文进 report）

**核心证据（本单成败判据）——证明真实库不再被触碰**：
```powershell
# 1) 先打印真实库路径并记下 mtime/size
#    路径由 default_memory_db_path() 决定（config_dir/northhing/memory/... 或 %APPDATA%\northhing\memory\...），report 里必须写明你解析到的绝对路径
$real = "<你解析到的真实 memory.db 路径>"
Get-Item $real | Select-Object FullName, LastWriteTime, Length | Format-List

# 2) 跑测试
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full agent_memory

# 3) 再取一次 mtime/size，必须与 (1) 完全一致
Get-Item $real | Select-Object FullName, LastWriteTime, Length | Format-List
```
若真实库不存在（本机还没建过），说明这一点，并改为断言"跑完测试后该路径仍不存在"。

**其余验证**：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo test -p northhing-core --features product-full memory
node scripts/verify-rot-budget.mjs
```

**环境硬事实**：
- PATH 上 GNU cargo 遮住 rustup shim → 必须用上面 `rustup.exe run stable-x86_64-pc-windows-msvc cargo ...` 完整前缀。
- 长构建走 PTY + 轮询；捕获输出用 `cmd /c "... > log 2>&1"`，**不要用 PowerShell 管道**（会因子进程继承句柄永久阻塞）。详见 skill `long-running-shell`。
- 跑完查残留进程：`Get-Process cmd,powershell` 里启动超 20 分钟的清掉。

## Constraints

1. 只碰上述测试代码文件；不新增依赖；不改生产行为。
2. rot-budget：不上调任何 ceiling；无新文件 >800 行（本单预期零新文件）。
3. **SDD 禁区**：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止 `git restore .` / `git clean` / `git add -A`；只许点名文件 add/commit。
4. 恰好一个 commit。
5. 日志英文无 emoji；注释英文。
6. 遇编译错误先加载对应 rust skill，禁止无脑 clone/unwrap。

## 报告

路径：`.superpowers/sdd/w14-1e-report.md`
必含：状态词（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）、commit SHA、`git show --stat`、**真实库 mtime/size 前后对比（核心证据）**、全仓核对逐条结论、三条验证命令输出、偏离清单、编译错误修在哪一层。

## 派发元信息

BASE = `8c00962`；禁区：生产路径（见 Spec 3）、`memory_db.rs` 的守卫实现、`progress.md`、`.superpowers/`（除报告）。
