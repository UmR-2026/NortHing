# W15-1l 独立验收报告

审查范围：`05bbd40..0ea30b3`，对照 `.superpowers/sdd/w15-1l-brief.md` 与 review package。

## SPEC 判定

1. **PASS** — 五个包装模块的内核 await 均位于统一 `kernel_dispatch` future 内：`api.rs:28-51`、`api_fs.rs:45-53`、`api_memory.rs:11-16`、`api_settings.rs:18-75`、`api_provider_edit.rs:30-38`。多 await 链 `list_skills` 在同一 future：`api_settings.rs:58-73`。
2. **PASS** — `turn_runtime()` 缺失时记录英文 warn 并执行 `fut.await`：`api.rs:209-217`。
3. **PASS** — 调用点脚手架已移除：`app.rs:65-91`、`app.rs:269-305`、`app.rs:311-321`；`approval_card.rs:20-34`。`SendOutcome` 已删除。
4. **PASS** — `kernel_dispatch` 将后台通道关闭映射为 `KernelError::Runtime`，无 panic：`api.rs:242-254`；包装层公开返回签名保持 `Result<..., KernelError>`，例如 `api.rs:28`。
5. **PASS** — 同一 helper 被五个模块消费：`api.rs:242`、`api_fs.rs:47`、`api_memory.rs:13`、`api_settings.rs:19`、`api_provider_edit.rs:30`。
6. **PASS** — 允许集核对：diff 仅 8 个文件，五个 api 模块、`app.rs`、`approval_card.rs`，以及续单授权的 `pages_archive.rs`；未修改 `api_events.rs` 或其它 pages。
7. **PASS** — 原 helper 测试已由仅检查 None 错误改为真实值断言：`api.rs:362-365`；新增错误透传测试：`api.rs:367-378`。

## QUALITY 判定

- **复用侦察：PASS。** report 明确记录既有 `spawn_on_turn_runtime` 的复用与语义演进（report:35-41），且 diff 显示 `kernel_dispatch` 委托该 helper（`api.rs:242-247`）。
- **无 owner 抽象：PASS。** `kernel_dispatch` 直接服务于五个现有 UI API 包装模块，存在真实调用方，不是孤立抽象。
- **预算闸：PASS。** `scripts/rot-budget.json` 未在 diff 中触及；当前 god-file `pages_onboarding.rs` 登记项虽被 report 审计发现，但未修改，且报告未声称修改该文件。
- **条件早退测试：PASS。** `test_spawn_on_turn_runtime_behavior` 无条件早退，直接断言 `Ok(42)`；`test_kernel_dispatch_behavior` 直接断言成功与错误结果（`api.rs:362-378`）。
- **god-file 观测：PASS。** 续单触及 `pages_archive.rs`，diff 仅局部加载块重构；report 已说明该改动是组件体裸 spawn 的根因修复，并提供运行验证。登记的超 800 行文件 `pages_onboarding.rs` 未触及。
- **直调绕过审计：PASS（按约束处理）。** report 列出 `pages_onboarding.rs:188` 的直调并说明因 pages 零改动约束未改；列出 `api_events.rs:101` 为订阅链路并保留。该审计结果与 brief 的界外约束一致。

## 验证证据核对

- report 提供 `cargo check -p northhing` 绿输出（report:51-57）。
- report 提供指定 focused tests 的真实输出，166 + 1 + 1 通过（report:59-72）。
- report 提供 `cargo build -p northhing` 绿输出（report:74-79）。
- report 提供运行验证原文：60 秒双窗均 `Responding=True`、`hung=False`，CDP `strataCount=70`、`hasLoading=false`，并有主窗 ping 回归与截图路径（report:116-151）。
- `git diff --check 05bbd40..0ea30b3` 无输出，未见 whitespace 错误。

## Cannot verify from diff

- 无法仅凭 diff 独立确认截图文件内容及视觉结论；但 report 提供了明确截图路径，并记录编排者视觉复核。该项作为运行证据采信，不构成代码缺口。
- 无法仅凭 diff 独立确认 60 秒运行脚本的环境真实性；report 提供完整原始输出，未发现与代码 diff 矛盾之处。

## 结论

**APPROVE**

Findings：
- Critical：无。
- Important：无。
- Minor：无阻塞项。`pages_onboarding.rs:188` 仍存在绕过包装层的直调，但这是 brief 明确要求上报、且 pages 零改动的已授权边界冲突，不作为本任务缺陷。
