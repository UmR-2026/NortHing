# Task Brief — W8-2: memory_db.rs 内部去重 + 死变量/回退 hack 处置

仓库：E:\agent-project\NortHing（main）。范围：`src/crates/assembly/core` 仅该 crate。
深审报告（先读，逐条病灶带 file:line）：`.superpowers/sdd/deep-rot-memorydb-lsp.md` §文件1。

## Spec（验收标准）

### 1. 消三重复制（主目标，约 −70 行）

对 `get_facts`（l:231-346）与 `search_facts`（l:377-560）之间的三块复制，提取 `MemoryDb` 私有 helper：
- **stmt 构造 Some/None 分支**（l:236-252 / l:404-430，仅 params 不同）→ 单函数 + 参数化
- **query_map 行映射闭包**（l:254-287 / l:434-469，34/36 行逐字段相同）→ 单闭包/函数复用
- **字符串→枚举转换三块 match**（scope/confidence/fact_type，l:294-328 / l:481-515 逐字相同）→ 三个 parse helper（如 `parse_scope` / `parse_confidence` / `parse_fact_type`）

**语义钉死**：未知枚举字符串的 fallback 行为必须与现状逐字一致（现状是 warn + 默认臂还是静默默认，照抄），去重不许顺带"修正"。

### 2. 死变量处置（2 处）

- l:542 `let bm25_pos = -rank;` 计算后丢弃 → 删该行；若 `ScoredFact.bm25` 存负值 rank 的意图可疑，report 记观察项，不改存储语义
- l:291 `last_mentioned_at` 解构后从未赋值进 `Fact` → 删除该字段解构（或若 `Fact` 结构体确有该字段而重构遗漏 → STOP，BLOCKED 上报，这可能是真 bug 不是死代码）

### 3. 回退 hack 处置（2 处，本波允许的行为微调，逐处钉死语义）

- l:556 `.unwrap_or(Ordering::Equal)`（partial_cmp NaN 臂）→ NaN score 沉底（降序语境用 `Ordering::Greater`）+ 一行注释说明；附一个 NaN 排序单测
- l:475 `.unwrap_or(0)`（SystemTime 时钟回拨臂）→ 时钟异常时 **跳过 recency boost**（不制造极端排序值）+ `tracing::warn!` 一条（英文）+ 一行注释；附单测（注入异常时钟路径若可测，不可测则 report 说明）

### 4. 防线

- `memory_db.rs` ceiling 918：去重后行数大降 → **同 commit 下调 manifest 条目到实测值**（棘轮只降不升的正确方向；commit message 注明）
- 不改 `dream` 子模块、不改 judge_mom KV 区（深审观察项，本波不动）
- 既有测试语义零改动；全绿

### 5. 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing-core`：0 error
2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-core memory_db`：全绿（含新 2 例）
3. `node scripts/verify-rot-budget.mjs`：绿

## Global Constraints（逐字，源自 plan-2026-08-28-w8-godfile-rotfix.md）

1. 分层边界：改动只在 `src/crates/assembly/core`（+ manifest 条目下调）。
2. 日志纪律：英文无 emoji，带关键上下文字段（本任务仅 §3 新增一条 warn）。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；**禁止 `git restore .`/`git checkout .`/`git stash` 等整树操作**，只许点名文件 add/commit。
4. rot-budget：ceiling 只降不升；manifest 仅允许本任务指定的下调。
5. 验证最小集：上述 3 条；report 写入 `.superpowers/sdd/w8-2-memorydb-dedup-report.md`（write 工具）。
6. commit 规则：恰好一个 commit；不含 `.superpowers/`。
7. 不新建无 owner 抽象；提取的 helper 消费方 = get_facts/search_facts 两处既有调用点。
8. 除 §3 两处点名微调外行为零变化；judge 逐块核对去重等价性。
9. 遇编译错误先加载对应 rust skill（m01/m03/m04 等）trace 设计层，禁止无脑 clone/unwrap 糊编译器。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；需要澄清 = NEEDS_CONTEXT。
- 返回消息含：状态 / commit SHA / git show --stat / 验证输出尾部 / memory_db.rs 新行数 / 偏离清单。
- 假汇报 = 停用：编排者将用磁盘 diff 逐条核对。
