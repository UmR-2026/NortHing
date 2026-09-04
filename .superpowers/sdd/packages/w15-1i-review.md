# Review Package — W15-1i（桌面启动挂死修复：core IO 超时降级 + F1 挪出 UI 执行器）

- 分支：`main`，BASE `9b41eac` → HEAD `f2f3819`（2 commits：`2472cff` services-core + `f2f3819` desktop）
- diff：`git diff 9b41eac..f2f3819`，补丁 = `.superpowers/sdd/packages/w15-1i-diff.patch`（14.8KB，4 文件）
- brief：`.superpowers/sdd/w15-1i-brief.md`（验收标准逐字在 §1，Global Constraints 在 §5）
- report：`.superpowers/sdd/reports/w15-1i-report.md`

## 任务一句话

桌面启动挂死根因 = F1 内核链在 dioxus UI 执行器内联跑，撞上 `json_store.rs:104` 一个永不完成的 tokio fs asyncify → Pending future 被混合循环 42k/s 重 poll → 主线程 busy-spin 饿死 tao 消息泵。修复双件套：core `read_optional`/`write_bytes_atomic` 单文件 IO 包 5s timeout（超时映射 `ErrorKind::TimedOut` 接入既有重试/降级）；桌面 F1 挪到 `turn_runtime` worker rt 执行、oneshot 回灌纯 DTO、Signal 写留 UI 侧。另修正 entry.rs 过时雷注释。

## 验收标准（逐条判 PASS/FAIL，对应 brief §1）

1. `read_optional` 的 `fs::metadata` + `fs::read_to_string` 包 timeout（5s 量级），超时转显式错误，调用方既有降级语义不变。
2. `write_bytes_atomic` 各 fs 操作包 timeout，超时接入既有重试与 PermissionDenied 降级。
3. 新增超时测试确定性触发（pending() 注入或极小 timeout），无早退绿；既有 json_store 测试保持绿。
4. F1（原 `app.rs:67-91`）内核链在 `turn_runtime()` 上执行，结果经 oneshot/JoinHandle 回灌；Signal 写全部在 UI 侧；缓存命中/settings warn/get_messages warn/entries 非空才 set 等语义逐一保留；`turn_runtime()` 为 None 时有 warn 日志。
5. `entry.rs` 注释与 trace 报告新结论一致（busy-wake + 冻结 timer；sleeping future 平反）。
6. 运行验证证据在 report：Responding=True、CPU 数值、截图路径。
7. diff 只触及允许文件集（json_store.rs / json_store_contracts.rs / app.rs / entry.rs）。
8. 界外零触碰：send_action / api_events / kernel_facade / ci.yml 等。

## Global Constraints（逐字）

- 禁止新增依赖。
- 并发/超时改动必带自动化测试（家规④）。
- 测试不得触真实用户配置；运行验证只读观察。
- 日志英文无 emoji。

## 重点质询（供 skeptical 校准，不是预判结论）

- report 声称新增 `read_optional_timeout` / `write_bytes_atomic_timeout` 公开变体「供自定义超时及测试驱动」——核查其是否构成无 owner 抽象（消费方是否真实）。
- 超时测试是否真的等到 Elapsed（10ms pending 注入是真等待，检查断言是否验证错误 kind/文案而非仅 is_err）。
- F1 挪窝后 `rx.await` 的失败路径（worker panic / channel 关闭）是否有兜底，还是静默无会话。
- 运行验证是单窗 70s 单次采样——修复前是 4/4 确定性必挂，单次 70s Responding=True + CPU 1.35s 已构成强证据，但 judge 可自行判断是否标注 Cannot verify。

## 背景（非判据）

- 根因三轮动检实证：`.superpowers/sdd/reports/startup-hang-trace-report.md`。
- 编排者已视觉验证截图 `screenshots/w15-1i-desktop-70s.png`：窗口正常渲染、无 Not Responding。
- 实现者报告遗留：`send_action`（输入发送路径）同型问题仍存——界外，已排后续单，不作为本单 finding。
