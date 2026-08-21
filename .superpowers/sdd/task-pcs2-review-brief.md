# Review Brief — PCS-2（skills 数据目录化 + fs watch 热加载）

## 审查对象

- 仓库：`E:\agent-project\.worktrees\northing-pcs2`（分支 feat/pcs2-skills-watch-0821）
- 范围：`fad68f7..3c6e2f5`（单 commit，40 文件 +960/-258）
- diff 包：`.superpowers/sdd/review-package-pcs2.diff`
- 实现 brief / report：`.superpowers/sdd/task-pcs2-brief.md` / `task-pcs2-report.md`

## 约束（本任务 spec 的精确要求）

- **trait 签名不许改**：`load_project_skills(&self)` 无参形态必须保持（用户历史约束，改了 = SPEC FAIL）。
- watcher：单 RecommendedWatcher；`user_skills_dir()` Recursive + 项目槽 NonRecursive；远程槽不 watch；350ms 去抖（校准依据须在 report）。
- DisposableList 持 watcher 句柄 + abort 闭包；工作区切换整体重建；停机 dispose。家规 4 并发测试。
- desktop UI 写必须经 `slint::invoke_from_event_loop`。
- catalog：BuiltinSkillId 枚举/BUILTIN_SKILL_SPECS 静态表删除；24 个内置 skill 的 group 元数据落 SKILL.md frontmatter（或 manifest）；catalog.rs:217 一致性测试数据源同步改；**include_dir!/build.rs/安装机制不许动**。
- 明确不做清单（越界即 FAIL）：统一面板/权限框架、per-skill 动态工具注册、远程 watch、include_dir! 移除、installer 载荷、prompt 热路径优化。
- desktop workaround 移除后，skills 面板 workspace 覆盖列应真实激活（on_set_skill_global/on_set_skill_workspace 回调）。

## 独立验证（你必须实跑）

1. `cargo check --workspace` + `cargo check -p northhing`（MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`）
2. `cargo test -p northhing-core --features product-full --lib skill_watch` + `--lib catalog` + `--lib agentic::tools::implementations::skills`
3. `cargo test -p northhing --lib`
4. `node scripts/check-core-boundaries.mjs` + `pnpm run check:rot`
5. **语义深挖（本轮重点）**：
   a. **去抖正确性**：连续事件风暴（模拟 .system staging rename）会不会导致 refresh 递归触发（refresh→install check→rename→事件→refresh 循环）？读实现判断终止性；
   b. **DisposableList 重建竞态**：`sync_watched_paths` 重建期间，旧 watcher 的 in-flight 事件会不会落到新 registry 状态上（脏刷新）？dispose 后事件回调是否可能再触发 refresh；
   c. **load_project_skills 工作区解析**：`global_workspace_service()` 解析失败/无当前工作区时返回什么（Err/空/default）——desktop 探测逻辑对这些分支的行为是否都安全；
   d. **catalog 动态解析的失败面**：某个 SKILL.md frontmatter 缺 group 或写错时的行为（warn+跳过/报错/崩溃），24 个内置 skill 的 frontmatter 标注逐一抽查至少 5 个确认格式一致。

## 你的角色定位

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

## 双判决（缺一不算通过）

1. **SPEC**：对照 brief 验收逐条 PASS/FAIL + file:line 证据。
2. **QUALITY**：常规项 + 三必查（复用核查 / 无 owner 抽象 / 预算闸）。god-file 观测点：触及登记文件则附健康度一句，未触及跳过。

## Cannot verify from diff

无法判定的单独列出，禁止猜。

## 档位

Critical / Important / Minor。plan-mandated 冲突交编排者。

## 报告

`.superpowers/sdd/task-pcs2-review.md`：双判决、证据、独立验证、语义深挖四点、findings。最终消息以 APPROVED / REJECTED 开头。
