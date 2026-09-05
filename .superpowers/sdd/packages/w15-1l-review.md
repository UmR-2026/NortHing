# Review Package — W15-1l（包装层统一派发 + 档案馆挂载风暴修复）

- 分支：`main`，BASE `05bbd40` → HEAD `0ea30b3`（2 commits：`3c28c0a` 包装层派发 + `0ea30b3` 档案馆 spawn→use_future）
- diff：`git diff 05bbd40..0ea30b3`，补丁 = `.superpowers/sdd/packages/w15-1l-diff.patch`（8 文件）
- brief：`.superpowers/sdd/w15-1l-brief.md`；续单修正：发现 `pages_archive.rs:126` 组件体裸 spawn 是第二重根因（60FPS 重渲染自激），编排者审计后授权修复该文件（pages 零改动约束对该点解除，其余 pages 仍零改动）
- report：`.superpowers/sdd/reports/w15-1l-report.md`（含续单节）

## 任务一句话

用户实测档案馆点击即（未响应）。修复双件套：①包装层——api.rs/api_fs.rs/api_memory.rs/api_settings.rs/api_provider_edit.rs 所有内核薄包装统一经 `spawn_on_turn_runtime`（或演进形态 kernel_dispatch）派发到 worker rt，None 时 warn+内联回退；W15-1j 调用点脚手架（app.rs F1/send/stop、approval_card settle）去冗余回直调。②档案馆挂载加载从组件体裸 spawn 改 use_future。

## 验收标准（逐条判 PASS/FAIL）

1. 五个 api*.rs 模块中每个体内 await kernel_facade 的 pub async 包装都经统一 helper 派发到 worker rt；多 await 链（如 list_skills）整条作为一个 future 派发。
2. turn_runtime None → warn 日志 + 内联回退（desktop_uninit 测试不变红）。
3. W15-1j 调用点脚手架已去（app.rs / approval_card.rs 回直调形态；SendOutcome 等不再需要的类型已删）。
4. `cargo check -p northhing` 绿；`cargo test -p northhing --lib --test desktop_uninit_a --test desktop_uninit_b` 绿。
5. 运行验证：档案馆加载出列表（CDP strataCount=70）且双窗 hung=False，60s CPU 增量 ~0；主窗 send 回归正常。截图 w15-1l-archive.png / w15-1l-main.png（编排者已视觉复核 archive 图：列表完整、无加载中、无未响应）。
6. diff 只触及允许文件集（5 个 api*.rs + app.rs + approval_card.rs + pages_archive.rs）；api_events.rs / 其它 pages_*.rs 零改动。
7. `spawn_on_turn_runtime` 的原 None 早退单测随语义演进而更新（现在能断言真实值），不许删测试装绿。

## Global Constraints（逐字）

- 禁止新增依赖；错误语义不变（Result<T, KernelError> 签名不动）；通道失败映射 KernelError::Runtime 不 panic。
- 日志英文无 emoji；禁整树 git 操作。
- 包装层外不许有遗漏：ui_dioxus 内若有绕过包装直调 kernel_facade 的点，report 须列出审计结果。

## 重点质询（供 skeptical 校准，不是预判结论）

- helper 的 None 回退语义与 W15-1j 原语义（None→Err(())）不同——检查所有依赖旧语义的地方是否已同步改。
- kernel_facade() 调用搬进 spawn 后，参数捕获必须 own（&str→String 等），逐一核编译期生命周期处理是否合理（禁止无脑 clone 大对象）。
- `subscribe_events`（api_events.rs）未被包——确认它确实不在被派发清单内（它是订阅，包了就错）。
- pages_archive.rs 的 use_future 改造：依赖捕获是否会在重渲染时产生过期闭包（use_future 依赖数组为空 = 只跑一次，符合挂载加载语义）。
- rot 闸：app.rs 应变小了（去脚手架），memory_db.rs 未触碰——`pnpm run check:rot` 应绿。
