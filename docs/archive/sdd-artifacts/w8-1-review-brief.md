# W8-1 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w8-1-review-package.diff`（= `3ab2330..3337c73`，7 文件 +894/-812；input.rs 802 → input/ 目录 5 文件：mod 46 + bridge 11 + non_key 169 + key_actions 235 + key_popups 428）
- 需求：`.superpowers/sdd/w8-1-input-split-brief.md`（Spec 6 条；**最高纪律 = 行为零变化**）
- 深审病灶：`.superpowers/sdd/deep-rot-app-input.md` §2
- 实现者报告（含逐臂位移自查表）：`.superpowers/sdd/w8-1-input-split-report.md`

## judge 重点核查项（按风险排序）

1. **逐臂纯位移核实（最高优先）**：原 `handle_key_event` 543 行的 30+ match 臂，逐臂对照新旧位置——顺序、条件、臂体必须逐字符等价（允许的差异：路径前缀调整、缩进、`.await` 包装不变性）。实现者报告里有逐臂自查表——**不可全信，抽查 ≥8 臂亲自 diff 核对**，尤其含 `apply_exit_reason` 8 参数调用与递归 `handle_command` 的臂。
2. **bridge helper 等价性**：7 处 `block_in_place(|| rt.block_on(async move {...}))` 的闭包捕获各不相同——新 helper 泛型签名是否真正等价（move 语义、捕获集、返回值类型）；抽查 3 处调用点。
3. **公共 API 面不变**：`chat::input::` 的对外可见项（原 input.rs 的 pub 项）在 mod.rs re-export 后，对 `mod.rs:157` 及调用方完全透明；rg 调用方确认零适配改动（diff 里除 mod.rs:157 路径适配外不应有其它文件的调用点改动）。
4. **manifest 处置**：`god_file:...input.rs` 条目删除；diff 中 rot-budget.json 仅此项变更，无 ceiling 数字改动。
5. **非 Windows 构建**：input.rs 原有平台 cfg 门（如有）在拆分后保持等效。
6. **隐含行为面**：`handle_non_key_event`（119 行）的鼠标分支位移同样纯位移；KeyEvent 构造/状态读取顺序不变。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。实现者的报告是待证伪的声明，不是证据；一切以 diff 和实跑输出为准。实现者已跑过的测试不重跑，但其"验证"章节的命令与输出要与 diff 内容对得上（缺输出 = 打回）。

双判决缺一不算通过（SPEC 对照 brief §Spec 6 条；QUALITY 独立判断）。防腐必查：复用核查 / 无 owner 抽象 / 预算闸（manifest 只允许清死条目）/ god-file 观测点（本 diff 使 input.rs 条目消亡，确认无 >800 新文件）。**Cannot verify from diff** 单独列出，禁止猜。plan-mandated 冲突交编排者。

## 输出

判决书写入 `.superpowers/sdd/w8-1-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M 计数 + 一句话理由。
