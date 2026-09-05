# Handoff — 2026-09-04（凌晨收口）：CI 双红修复实证 + 桌面启动挂死根因定位

> 上一份：`2026-09-03-w15-1-done-screenshot-pending.md`。本份覆盖：渠道战况、W15-1f/1g 双修复、CI 日志权限打通、桌面挂死三轮定位。

## 0. 一句话状态

HEAD `bb50b07`（已推 main）。CI @bb50b07：**serial 测试 / core boundary / rot / i18n / kernel-api 全绿**；仍红：windows parallel 测试（新暴露 MemoryDb 迁移竞态）、ubuntu/macos 编译（预存）、repo hygiene（预存）。桌面 app 启动挂死**根因已定位**（非 W15-1 引入），修复任务未派。

## 1. 本段 commit（倒序）

| commit | 内容 | 验收 |
|---|---|---|
| `bb50b07` | merge W15-1g | CI serial ✅ 实证 |
| `ea2882c` | W15-1g 符号链接检查顺序对调（platform.rs） | minimax-m3 APPROVE 0C/0I/0M |
| `6cbebbb` | W15-1f dev-deps anyhow 删行 + file-watch feature 挂接 | minimax-m3 APPROVE 0C/0I/0M，CI core boundary ✅ 实证 |

## 2. 渠道战况（选派口径更新）

- **agy 双渠道（3.8/3.7）：派发即取消** → 用户拍板封存。
- **vertex gemini-3.8-flash：当日故障**，新形态报错 `Requests ending with a model turn are not supported`（三次全挂，含全新 session）。
- **coder-qwf = 当前主力 coder**：本波 5 单全交付（含挂死排查三连：插桩→排雷→core 探针）。
- minimax-m3 judge 两场 APPROVE + 一场静态追踪，稳定。

## 3. 桌面启动挂死全案（三轮定位，根因已钉）

**症状**：debug exe 启动后窗口「未响应」，主线程 ~100% 单核自转 ≥7min，debug.log 零写入，骨架渲染一次后冻结。4/4 确定性复现。pre-W15-1 回滚二进制同样挂 → 预存。

**根因链**（qwf 三轮插桩实证，细节在 `.superpowers/sdd/reports/startup-hang-trace-report.md`）：
1. 挂点 = F1 `ensure_room_session → list_sessions_all_workspaces`（api.rs:145 → kernel_facade/session.rs:92）。
2. 风暴源 = `services-core/src/json_store.rs:104` 的 `tokio::fs::read_to_string(state.json).await`——ws#2 第 53 个会话的该 await **永不完成**（8.4M+ polls 恒 Pending，spawn_blocking 完成信号丢失竞态；同 Handle 新发 spawn_blocking 却 <300ms 正常）。
3. dioxus 0.8.0-alpha.1 混合循环对该 Pending future 高频自唤醒（42k poll/s）→ 主线程 busy-poll → 窗口 ghost。**勘误了 entry.rs 旧结论**「任何 sleeping use_future 引自转」——实验证明睡死无害，真正毒的是「Pending + 自 wake」。
4. 02:12 健康 → 23:02 必挂的翻转 = 宿主时序竞态窗口，非数据形态变化（index.json 08-27 未变）。

**修复方向（报告里有双侧详案）**：
- core 根治：`read_optional` 加超时 + 降级（对环境竞态通用免疫）
- 桌面止血：F1 挪 `turn_runtime` spawn + watch 回灌（实验证明该 park 模式无害）
- 最后一环（可选）：`[patch]` dioxus-desktop 打 enter/exit guard 确认 wake 生产者

## 4. CI 现状与下一单（W15-1h）

- ✅ serial 测试绿 = W15-1g 实证（serial 5 轮连绿观测可从本轮起计）。
- ❌ **windows parallel 测试新红**：`kernel_facade::tests::test_search_facts_returns_ok` @ tests.rs:645 —— `MemoryDb open failed: duplicate column name: status`。serial 绿 parallel 红 + 本地全绿 = **迁移竞态/并行干扰**（两个并发测试开同一全局 MemoryDb 各跑一遍 ALTER TABLE）。修法候选：迁移幂等（IF NOT EXISTS / 错误容忍）或测试 DB 隔离。**这是下一单 W15-1h**，brief 未写。
- ❌ ubuntu/macos 编译红：预存陈账，未动。
- ❌ repo hygiene 红：SDD 工件绝对路径触发，**等用户拍板**（checker 加 .superpowers 豁免 vs 工件脱敏）。
- gh CLI 已装已登录（`C:\Program Files\GitHub CLI\gh.exe`，UmR-2026，repo+workflow scope）——编排者 PATH 是 session 启动快照，**调用用全路径**。

## 5. 本机环境变更（下 session 须知）

- `~\AppData\Roaming\northhing\config\app.json`：smoke-echo MCP 残留条目已删（备份 `app.json.bak-20260903`）。
- `~\AppData\Roaming\northhing\episodes\`：164 个 testslug-* 测试污染目录（~200MB）隔离到 `episodes-quarantine-20260903\`（可逆，确认无价值后可删）。
- `shot-window.ps1` 的 Add-Type 故障根因 = csc 临时目录不可写；修法 = TMP/TEMP 指到 `C:\WINDOWS\TEMP\opencode`。可用替代：`C:\WINDOWS\TEMP\opencode\win-shot.ps1`（窗口截图，已实证）+ `fullscreen-shot.ps1`。
- `run_detached` 插件本 session 静默死两次（0 日志 0 进程）→ 长命令一律 PTY + cmd 重定向。
- 截图产出：`screenshots/w15-1-0{1,2,3}-*.png`（未提交；w15-1-03 拍到了挂死窗口=无效验收图，重拍需挂死修好后）。

## 6. 队列

1. **W15-1h**：MemoryDb 迁移竞态修复（§4，brief 未写；证据 = CI run 33789958328 日志）。
2. **桌面挂死修复单**（core 超时降级 + 桌面 F1 挪窝，报告里有方案；修好后重拍 §7#11 三张截图，W15-1 才完整闭环）。
3. repo hygiene 治理（等拍板）、ubuntu/macos 编译陈账。
4. follow-up：core 裸 feature 编译缺口（qwf 在 W15-1g 再次踩到：`-p northhing-core` 无 feature 基线 3 个 E0433）；turn_persist.rs:546 泄漏候选；css_files.rs 孤儿；**测试污染真实配置/数据目录的 E 类问题升级**（smoke-echo + testslug 实证污染源存在）。
5. W15-2 输入框多行+拖入 → W15-3 → W16。

## 7. 环境/雷区（增量）

- 宵禁 03:00，本 handoff 写于 02:50。
- 派发返回异常先 `git status`/`git log` 磁盘核查（再次实证：被取消任务留插桩 WIP，续派收尾成功）。
- qwf 派发正文里工作树状态要写实（"src 干净"这类断言 coder 会采信）。

## Suggested skills

- `long-running-shell`、`subagent-driven-development`、`verification-before-completion`、`handoff`
