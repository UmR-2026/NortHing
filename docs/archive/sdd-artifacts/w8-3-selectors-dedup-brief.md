# Task Brief — W8-3: selectors.rs 消三处复制

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/cli` 仅 CLI crate。
深审报告（先读，§二 全部病灶带 file:line）：`.superpowers/sdd/deep-rot-onboarding-selectors.md`。

## Spec（验收标准）

### 消三处复制（行为零变化）

1. **ModelItem 映射**（`selectors.rs:113-121` × `chat/model.rs:108-113`）：提取 `ModelItem::from_config(&AIModelConfig) -> ModelItem`（或等效构造函数），挂在 ModelItem 定义处的同一模块；两处调用点都切换。`.filter(|m| m.enabled)` 语义保持原样。
2. **time-ago 四档格式化**（`selectors.rs:49-59` × `chat/session.rs:147-157`，零行差异）：提取单个共享函数，归属就近（如 `chat/` 下公共 util 或 session.rs 持有、selectors 引用——实现者选定唯一 owner 并在 report 说明选择）；两处调用点切换。
3. **custom_headers 解析**（`selectors.rs:205-209` × `:344-348`，文件内自重复）：提取文件内私有 helper，save/update 两处切换。

### 防线与纪律

- selectors.rs ceiling 875（当前 875 触顶）：去重后行数下降 → **同 commit 下调 manifest 条目到实测值**
- ModelItem/time-ago 的展示格式逐字符不许变（含 `"just now"` 等文案、四档阈值）
- `provider_display_name` 竞速解析、UNIX_EPOCH unwrap_or_default 为深审观察项——**本波不动**，report 观察项带过即可
- 测试：cli crate 现有测试全绿；若提取的 helper 可纯函数化（time-ago 应该可以），附 1-2 个聚焦单测（含边界档位）

### 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing-cli`：0 error
2. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc test -p northhing-cli`：全绿
3. `node scripts/verify-rot-budget.mjs`：绿

## Global Constraints（逐字，源自 plan-2026-08-28-w8-godfile-rotfix.md）

1. 分层边界：改动只在 `src/apps/cli`（+ manifest 条目下调）。
2. 日志纪律：英文无 emoji；本任务零新增日志。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；**禁止 `git restore .`/`git checkout .`/`git stash` 等整树操作**，只许点名文件 add/commit。
4. rot-budget：ceiling 只降不升；manifest 仅允许本任务指定的下调。
5. 验证最小集：上述 3 条；report 写入 `.superpowers/sdd/w8-3-selectors-dedup-report.md`（write 工具）。
6. commit 规则：恰好一个 commit；不含 `.superpowers/`。
7. 不新建无 owner 抽象；每个提取的 helper 有 2 处真实调用点。
8. 行为零变化铁律：judge 逐块核对；逻辑漂移 = Critical。
9. 遇编译错误先加载对应 rust skill，禁止无脑 clone/unwrap 糊编译器。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因；需要澄清 = NEEDS_CONTEXT。
- 返回消息含：状态 / commit SHA / git show --stat / 验证输出尾部 / selectors.rs 新行数 / 偏离清单。
- 假汇报 = 停用：编排者将用磁盘 diff 逐条核对。
