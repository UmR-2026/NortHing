# Project Audit Summary — 2026-08-26

范围：全仓（1489 .rs / 273k 行，main @ 8bc015d）。方法：机械闸门 + MSVC 全量测试链 + 三区域并行独立审查（R1 desktop=minimax-m3 / R2 core=qwen38-max / R3 services+execution+adapters=minimax-m3）。

## 总判定

**needs attention** — 架构分层、锁纪律、密钥 fail-closed 主干、SSE/JSON 容错、LSP 事务化卸载等结构不变量全部成立；但存在 **1 个 Critical 产品缺陷**（桌面事件泵）与一批 Important 级热点，外加 rot 预算 4 文件超限需用户裁定。

## 一、机械闸门

| 闸门 | 结果 | 处置 |
|---|---|---|
| repo-hygiene | ✅ pass | — |
| core boundaries | ❌→✅ | russh-keys 规则陈旧（0.62 已吸收回主 crate，Cargo.lock 零引用），删规则修复（8bc015d） |
| check:rot | ❌ 4 违规 | `ui_dioxus/app.rs` 962>800、`callbacks_settings/refresh.rs` 834>800、`ui_dioxus/css.rs` 830>800（均未登记）、`callbacks_lifecycle.rs` 1011>1009。**处置待用户**：拆文件 or 抬天花板（家规 7 需签字） |
| 测试 desktop (MSVC) | ✅ 152/152 | — |
| 测试 integrations (MSVC, product-full) | ✅ 47/47 | — |
| 测试 core (MSVC, product-full) | 1048✅/1❌→✅ | 唯一失败 = i18n 陈旧测试（T0-3 品牌统一后期望值没同步），已修（8bc015d），聚焦复跑 9/9 |

密度普查：allow(dead_code) ×137；crates 内 unwrap() ×708（多数在测试/低风险路径，R1-R3 已按生产可达性筛过）；unsafe 涉及 9 文件（R3 判定全部 sound FFI）。

## 二、审查发现（合并排序）

### Critical

- **C1 · EventQueue 堆在桌面端永不消费 → 万条事件后静默丢 UI 事件**（R2，r2-core.md#1）
  `enqueue` 同时入堆（上限 10000）+ broadcast；桌面只消费 broadcast（system.rs:87-103），堆只有 CLI 在 pop。堆满后所有非 Critical 事件（TextChunk/完成事件/工具事件）`Err(EventQueueFull)` 被调用点 `let _ =` 吞掉——**长会话中 UI 冻结而 turn 继续跑**。与 P2b 修的"满队丢事件"同域但不同根：P2b 把丢失从静默变可观测，本条是堆根本不该在桌面侧计容量。
  修复方向：容量闸与广播投递解耦（无堆消费者时不占堆预算），或桌面 bootstrap 起常驻 drain 任务。M 工作量，需小设计确认。

### Important

- **I1 · 编辑 provider 时 keyring 读失败会静默抹掉用户 API key**（R1）`app_state/callbacks_settings/provider.rs:121-125` `.ok()` 吞错 → `Some("")` 进 upsert。破坏 P1-2 fail-closed 承诺的编辑路径。
- **I2 · 一个损坏的 session_state.json 毒化整个会话列表**（R2）`session_subhandlers.rs:303-307` 解析错误直接传播；`list_sessions_all_workspaces` 任一工作区失败整体 Err——连带打断 P22 的 room 会话解析。修法：跳过+默认 Idle（S）。
- **I3 · growth 蒸馏 LLM 调用挡在 DialogTurnCompleted 之前**（R2）`sub_handle_out.rs:352→365-368`，UI 最多晚 15-30s 才停"生成中"。修法：先发完成事件 / growth hook 出临界路径（S/M）。
- **I4 · LSP 子进程孤儿**（R3 F1）：spawn 缺 kill_on_drop+process_group，Drop 只打日志。
- **I5 · MCP stop 杀不到子进程树**（R3 F2）：child.kill 不递归，spawn 无 process_group。
- **I6 · vault 密钥文件非原子写**（R3 F3）`password_vault.rs:57`/`auth.rs:114`：崩溃在写入中途 → 密钥长度失配 → 全部历史密码永久不可解。内容本体已是原子写，钥匙文件漏了同款处理。
- **I7 · SSE 日志收集器默认无上限**（R3 F4）：`SseLogConfig::default() max_output None`，10 万 token 流 = 内存里挂 10 万条。
- **I8 · 抽屉窗 HWND_TOPMOST**（R1）`block_registry.rs:153`：压过所有其它应用而非仅主窗之上。
- **I9 · 8 处 expect() 建 runtime 在 spawn 线程内**（R1）`callbacks_lifecycle.rs` 多点：失败即线程 panic，用户操作静默无效；同文件 :866 已有 match+banner 正确范式可抄。

### Minor（代表性，全量见各分报告）

broadcast 缓冲 1024 满即 warn-drop 且订阅者串行慢者拖全队（R2）；error_banners 每错误起一个 sleep(5s) 线程（R1）；37 处回调内建临时 runtime（W4 只修了 turn-dispatch 半边）（R1）；block_registry 60Hz Timer 永不停止（R1）；FileWatchService 每次 watch/unwatch 重生 watcher（R3 F6）；SSE 错误子串匹配脆弱（R3 F8）；`to_string_lossy` 非 UTF-8 路径 U+FFFD 错配（P22 judge）。

### 各区健康面（值得说的好话）

原子设置写 + SETTINGS_WRITE_LOCK 全程持锁、keyring provider-key 主路径 fail-closed、Slint 线程纪律守恒、turn-generation guard、cancel/watchdog 防护、vault 内容原子写+fail-closed、SSE/JSON 修复对畸形输出鲁棒、LSP 卸载事务化+回滚、unsafe 面小且 sound。

## 三、建议队列（待用户拍板）

| # | 项 | 量级 | 备注 |
|---|---|---|---|
| 1 | C1 EventQueue 桌面堆解耦 | M | 先 30 分钟设计定方向再派 |
| 2 | I1 provider 编辑抹 key | S | fail-closed 回归，安全相邻，优先 |
| 3 | I2 state.json 毒化列表 | S | 连带保护 P22 成果 |
| 4 | services 进程批：I4+I5 (+F5/F9 顺带) | M | 统一 process_group/kill_on_drop 范式 |
| 5 | I6 vault 钥匙文件原子写 | S | 数据永久丢失级 |
| 6 | I7 SSE 缓冲上限 | S | 一行 default 改动+测试 |
| 7 | I9 expect×8 换 match 范式 | S-M | 机械转录，同文件有现成范式 |
| 8 | I8 TOPMOST | S | UX |
| 9 | I3 growth 出临界路径 | M | 动 turn 收尾顺序，需小心 |
| 10 | rot ×4 | M-L each | 用户选：拆 or 抬（抬需签字） |
| — | 真机手动走查 | — | 仍欠：onboarding 全流程 + room 流式/approval |

分报告：r1-desktop.md / r2-core.md / r3-services.md（同目录）。
