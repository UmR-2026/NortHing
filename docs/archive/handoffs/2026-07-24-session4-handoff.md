# Session 4 Handoff — 2026-07-24 (Memory P0 启动 + 构建环境修复)

> HEAD（northing）未推送 11 commit。本 session 在 opencode 里做 Memory P0 + 自我认知 UI 编排。用户并行 session 做前端探索/OD 设计。
> 触发：用户"开了新 session 试试 opencode" → "这两天优先使用 step 把 2、3 做了，现在 OD 里做自我认知前端 UI"。

## 1. 本 session 做了什么

### Open Design MCP 修复
- 病灶：`~/.config/opencode/opencode.jsonc` 里 open-design MCP 用了 `"env"` 字段，opencode 要求 `"environment"` → `ELECTRON_RUN_AS_NODE=1` 没传入，Electron 起 GUI 不起 stdio。
- 修：`env`→`environment` + 加 `"timeout": 30000`。重启后 MCP 工具注入成功（list_projects 等可用）。

### gcc 构建环境修复（关键，否则 rusqlite bundled 编不过）
- 表象：rusqlite bundled 编 SQLite 时 cc1.exe 退 `0xC0000139`（STATUS_ENTRYPOINT_NOT_FOUND）。
- 根因（两层）：① MSYS2 gcc 16 部分更新，mingw64 运行时 DLL 旧 → `pacman -Syu` 两遍修复（更新 msys2-runtime + mingw-w64 crt/libwinpthread/headers）。② **DLL 劫持**：PATH 上 `C:\Program Files\Rust stable GNU 1.95\bin\` 的旧 `libgcc_s_seh-1.dll`/`libwinpthread-1.dll` 排在 msys64 前，gcc16 cc1 加载到旧 libgcc 缺入口点。
- **修法（每条 cargo 命令必带）**：`$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`。实测这样 GNU 工具链下 rusqlite bundled 编译通过。

### CJK FTS 方案验证（探针实测，临时项目 fts_probe）
- 默认 `unicode61` 查"以后" = **0**（不分中文词，"以后都用pnpm"当一个 token）。
- `trigram` 查 4 字 = 1，查 **2 字"以后" = 0**（trigram 要求 ≥3 字，中文双字词全废）。
- **定案**：预切 bigram 存独立 `text_fts` 列 + unicode61。实测 2 字词可搜（"以后"→1、"喜欢"→1、"依赖"→1、"docker"→0）。

### memory_db.rs 返修历程（Step 两连败 → qw 收尾）
- s37：写 v1（932 行）但**从未编译**（被取消）。
- srouter：返修，设计全对（CJK 预切 + 三因子 BM25×keyword_weight×recency_boost + token 边界 keyword 匹配 + weight 上限 5.0），但留 **5 处 bug**：① 缺 `use super::facts::{Fact,...}` 导入 ② search_facts SQL 参数错位（workspace `LIMIT ?4` 应 ?3、非 workspace `LIMIT ?3` 应 ?2）③ line116 SQL 调 Rust 函数 `segment_for_fts(text)` ④ 末尾 `compile_error!` 调试绊线 ⑤ 假 `sanity_check` 测试占位、真测试 `memory_db_tests.rs` 没经 `#[path]` 接上。**空返回**。
- **coder-qw：5 处 bug 全修**（导入 line6 / LIMIT ?3+?2 / Rust 侧 backfill / 删 compile_error / `#[path]` 接测试 line649）。

### 关键发现：feature gate
- northhing-core **默认 features 为空**，memory_db 模块/测试要 `--features product-full` 才编译/注册。这解释了之前"0 个 memory_db 测试"和 compile_error 不炸（模块被门控不编）。
- 正确验证命令：`cargo test -p northhing-core --features product-full memory_db`（带 PATH 前缀）。

### 外部评审核对（用户并行探索喂来的两份 review）
- 代码层面基本属实：CJK 崩 / 三因子只实现 BM25 / recency 缺失 / contains 误匹配 / weight 无上限。
- **误报**：handoff "GBK 乱码"是假（合法 UTF-8，评审用 GBK 编辑器看的）；git 166→11 正常（origin/main 被推过）。
- 评审没看到的：rusqlite 构建阻塞（gcc 坏）、feature gate。

## 2. 未完成 / 卡点

- **memory_db 测试未确认全绿**：qw 修完 5 bug 后跑 `--features product-full` 测试，shell 卡在 product-full 首次重编译（拉全量 crate，慢），任务被取消。**下午第一件事：跑 `cargo test -p northhing-core --features product-full memory_db`（带 PATH 前缀）确认全绿。**
- 未过 judge，未正式 commit（本 handoff 后做 WIP commit 保存进度）。

## 3. 用户决策记录

- 前后端分开做；这两天优先 Step 做任务 2、3（Memory P0 + judge-mom）；自我认知 UI 在 OD 做。
- gcc 修复方式：用户选"你修 MSYS2 gcc"（我执行 pacman -Syu + PATH 修复）。
- 返修 coder：用户选 coder-srouter（守 Step 偏好）→ 失败 → 我升级到 coder-qw（实证）。
- 自我认知 UI：用户开 OD 并行 session 自己做，让我先别管 OD。
- 用户要"todo list + 设计模板"自己扔给 OD（我已给：P0 完整 brief + 复用模板 + P1/P2 短 brief）。

## 4. 后续队列

| 序 | 单 | 备注 |
|---|---|---|
| 1 | M-P0-1 验证全绿 | `--features product-full`，带 PATH 前缀 |
| 2 | M-P0-1 judge 验收 + 正式 commit | WIP commit 已存，judge PASS 后转正 |
| 3 | M-P0-2 | query-aware 接线（build_workspace_agent_memory_prompt + turn_persist）+ 反馈循环（注入+0.2/未引用-0.1）+ JSONL→DB 迁移 |
| 4 | judge-mom + dream | 依赖 P0；设计稿建议 P0 用固定参数冷启动，别上时机自学习 |
| 5 | C6/C7 设计稿 | 待 C4 后 |

## 5. 雷区补充（本 session 新增）

- **cargo 命令必带 `$env:PATH = "C:\msys64\mingw64\bin;" + $env:PATH`**（否则 Rust GNU 旧 libgcc 劫持 gcc16，rusqlite bundled 编不过）。
- **northhing-core 测试要 `--features product-full`**（默认 features 空，memory_db 不编不注册）。
- **cargo 缓存会骗验收**：源码有 `compile_error!` 时 `cargo check` 仍可能 exit 0（吃旧缓存）。验证前 `cargo clean -p northhing-core` 强制真编译。
- **Step 系（s37/srouter）接不住中型返修**：s37 循环吐字、srouter 留 bug+调试绊线+假测试+空返回。中型/需迭代调试的单用 qw/lc。（s35 仅极小观察单。）
- subagent 空返回 ≠ 没干活：srouter 空返回但改动已落盘，必须 git status/diff 独立验证。

## 6. 自我认知 UI（OD，用户并行）

- 设计依据：`docs/design/2026-07-23-self-cognition/first-entry-design.md`（4 字段 + 5 色板 + 生成流程）。
- 已给用户 OD 设计包：P0 完整 brief（四句宣言内嵌填空 + 五色性格板 + 生成动画）+ 复用模板 + P1（Episodes 侧边栏）/P2（成长时刻、Memory 可视化）短 brief。
- OD 项目"northing 自我认知"曾建后被用户删，用户改在并行 session 自己开。

## 7. 用户并行探索发现的治理项（非本轨，待 triage）

- ledger P2-9/P2-10 状态写 active 与实际不符——**但 P2-9 ledger 详文写着"37 violations 剩余 + stage3 接 CI 待做"，与 handoff "0 violations resolved" 矛盾**，不是简单翻状态，需先厘清。
- boundary checker 未接 CI（0 violations 可能回归，结构性风险）。
- 探索报告在 northing 根目录 `exploration-{governance-debt,self-cognition,frontend-product}_20260724.md`（未跟踪，用户产物）。

## 8. 一句话状态

Open Design MCP 通了、gcc 修好了、CJK 方案验证了、memory_db 五 bug 由 qw 修完——就差 `--features product-full` 跑测试确认全绿，然后 judge + 转正 commit；Step 系中型单两连败，已升级 qw。
