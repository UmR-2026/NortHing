# W14-1c-3a Brief — AgentRegistry unregister seam + prompt_stability 测试清理

> 来源：`w14-1b-arbitration.md` §2.2 B-2 第 1 行 + §5#6 + 2026-09-02 补遗（可见性规则）。BASE：`5f242fd`。

## 预检结论（已磁盘核实）

- `AgentRegistry` 在 `src/crates/assembly/core/src/agentic/agents/registry/mod.rs:96`，内部 `agents: Arc<RwLock<HashMap<String, AgentEntry>>>`（:98），写锁 helper `write_agents()`（:120）。**无 unregister/remove 方法**（真 API 缺口）。
- 目标测试：`src/crates/assembly/core/src/agentic/tools/implementations/task_tool/task_tool_agents.rs:228` `prompt_stability_description_with_context_renders_available_agents_in_stable_order`——经 `register_prompt_order_test_subagent` 注入 4 个测试 agent（`AAAPromptOrderBuiltin`/`ZZZPromptOrderBuiltin`/`AAAPromptOrderUser`/`ZZZPromptOrderUser`，:243-260 区域），**测完不清理**，污染同进程其它测试。
- 同 crate（core）内 seam → `#[cfg(test)] pub` 有效（补遗规则②）。

## Spec

- S1：`registry/mod.rs` 加 `#[cfg(test)] pub fn unregister_for_test(&self, name: &str)`（从 `agents` map 移除；注释「测试专用 seam，release 构建不存在」）。**偏离仲裁 §5#6 记录**：仲裁"永久生产 API + cfg(test)"表述自相矛盾，按 cfg(test) seam 落地（同 crate 测试够用）。
- S2：重构 :228 测试，末尾（含 panic 路径尽量覆盖——用 scopeguard 式或显式调用均可，选最简单）注销 4 个注入 agent。
- S3：`cargo test -p northhing-core --features product-full task_tool_agents` 全绿；测试数不降。

## Constraints

C1 同 crate seam 只许 `#[cfg(test)] pub`，禁裸 `pub(crate)→pub`；C2 不动 FACADE/global_scheduler/六层；C3 `let _ =` 闸 371/388，src 非测试路径零新增；C4 git 只点名 add、禁 add -A/restore/checkout；C5 与代码冲突以代码为准、偏离记 report；C6 **并行波**：同工作树有其它 coder 在跑，cargo 锁等待是正常的，**禁杀任何进程、禁动非本单文件**；若编译错来自别人未完成的文件（如 deep_review / remote_ssh / auto_memory / keyring / ci.yml），等待重试，不修别人的活。

## 验证

MSVC：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-x86_64-pc-windows-msvc cargo ...`；`cmd /c` 重定向。
1. `cargo check -p northhing-core --features product-full`（0 error）
2. `cargo test -p northhing-core --features product-full task_tool_agents`（绿）
3. `cargo test -p northhing-core --features product-full agents`（registry 相关全绿）

## 报告

`.superpowers/sdd/w14-1c-3a-report.md`：改动清单 / 验证输出原文 / 复用侦察 / 偏离节 / 状态词。完成后自行 commit（message 含 W14-1c-3a，只 add 点名文件）。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
