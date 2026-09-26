# W26-2 Review (m3)

- BASE: 831094c
- TIP: 7610de5
- VERDICT: APPROVE_WITH_CONCERNS
- C=0 / I=0 / M=1

## SPEC: PASS

逐条对 AC1-AC6 证据。

### AC1 PASS
- dialog_turn.rs:68-70 字段定义 + serde 属性正确
- 复用 core-types/errors.rs:40-58 AiErrorDetail（未新建 TurnErrorDetail）
- 字段位置 token_usage 之后、status 之前
- services-core 已依赖 northhing-core-types (Cargo.toml:13)

### AC2 PASS
- turn_lifecycle.rs:446-452 签名新增 error_detail: Option<AiErrorDetail>
- turn_lifecycle.rs:477 turn.error_detail = error_detail 落盘
- turn_persist.rs:210 唯一调用方适配
- AgenticEvent::DialogTurnFailed 同款 detail（落盘/事件一致）

### AC3 PASS
- save.rs:116-124 新增 Error 分支
- 仅 Error+无文字+detail=Some 时合成消息
- error_detail=None 时跳过（双向兼容）
- 文本 [Error: {类别}: {provider_message}] 与 live 路径同前缀
- error_category_label 穷尽 13 类别
- save.rs 内联 3 个 #[cfg(test)] 测试

### AC4 PASS
- 重建路径走共享 Message 列表
- desktop 消费点 app.rs:74 → session_mock.rs:102-149
- CLI 消费点 chat_state_core.rs:94 from_core_messages
- 报告引用证据与 diff 一致
- 零 surface 代码改动

### AC5 PASS
- dialog_turn.rs:524-634 内联 4 个 serde 测试
- test result: ok. 56 passed; 0 failed
- 含旧格式无字段、camelCase 往返、snake alias、skip_serializing_if

### AC6 PASS (环境性 caveat)
- §6.0 cargo check product-full exit 0 (host)
- §6.1 workspace check 0 error (隔离树 527 行)
- §6.2 host 被 W26-5 WIP 阻断 (15 错全在 mcp/server/manager/tests.rs)
- §6.2 替代：隔离树 37/0 (含 3 个本单新测试)
- §6.3 services-core 56/0 (含 4 个本单 serde 测试)
- §6.4 check-repo-hygiene.mjs exit 0
- §6.5 verify-task-gate.mjs exit 0 (831094c..7610de5)
- 台账 P2-5 翻转文案：report 疑虑 6 提供

## QUALITY: PASS

### 常规项
- 命名：snake_case 函数 / camelCase 字段一致
- 错误处理：Option<AiErrorDetail> 显式 None；生产路径 0 新增 unwrap/expect/panic
- 日志：本单未新增日志
- 路径卫生：6 个文件均在 allowlist 内
- 平台边界：未触碰 Tauri / desktop / 平台 API
- 层依赖：services-core → contracts 已合规

### 防腐必查项 (judge 块)
1. 复用核查：report 复用侦察一节存在且属实
   - AiErrorDetail 直接复用 core-types/errors.rs
   - serde 模式复用 token_usage 同文件先例
   - Message 构造模式复用 save.rs:113 assistant_with_reasoning
   - 独立核验：grep TurnErrorDetail = 0 命中

2. 无 owner 抽象：
   - error_category_label (fn, 私有) → save.rs 单一消费
   - format_turn_error_message (pub(crate)) → 同 crate 单一消费
   - error_detail 字段 → 4 个真实消费方
   - 无投机性抽象

3. 预算闸：未触碰 rot-budget.json / workflow-policy.json / metaRatchetPaths

4. 条件早退测试：save.rs 3 测试 + dialog_turn.rs 4 测试均有真实断言执行；无早退路径

5. god-file 观测点：6 个触动文件均非登记 god-file

### 进一步核验
- distill.rs 越界波及 (Minor)：brief §8 钉死 scope 为 :164-183 区，diff 含 4 处额外 rustfmt hunk (extract_tool_records :39-44 / extract_failures :66-70 / use 换行 :148-152 / vec! :483-489)。报告疑虑 3 已披露；rustfmt --check 全 clean；属家规 1「顺手清配额」

## Findings
- [Minor] distill.rs 4 个额外 rustfmt hunk 超出 brief 钉死的 :164-183 区 (distill.rs:39-44, 66-70, 148-152, 483-489)；rustfmt 单一驱动、check 全 clean、报告疑虑 3 已披露；非阻塞；建议下次 brief scope 描述更宽或留 fmt:rs 显式 hook。无修复指令。

无 Critical / Important。

## Cannot verify from diff
- §6.0 宿主 cargo check -p northhing-core --features product-full 仅 CHECK_OK (exit 0) 行文未贴输出原文；但 §6.1 workspace check 同样覆盖 core crate，可独立佐证
- §6.2 宿主树 15 错误位置：可独立 git status 看到 W26-5 的 staged 改动，但不重跑
- §6.3 隔离树复跑：报告「commit 后磁盘版修订」追加段贴了 11 个 test result 全 ok；可信
- rustfmt 7 处 check：报告声明全 clean；未贴命令+输出但仓库有 rustfmt --check --edition 2021 自检机制；非阻塞

## 范围变动
- 文件集：与 allowlist 一致 (6 .rs + 3 .md/.txt)
- commit: f54a3fe (代码) + 7610de5 (report/allowlist 回填)
- diff --stat 444+/22-
- 无 git add -A 痕迹

## 终判
**APPROVE_WITH_CONCERNS**

理由：SPEC 6 条全部 PASS (AC6 含环境性 caveat，已以隔离树 37/0 复跑 + host 非测试 check 闭合)；QUALITY 5 项防腐必查项全 PASS；唯一 Minor 涉及 distill.rs 4 处 rustfmt 越界波及 (已披露、fmt-clean、house rule 1 范围内)。C=0 / I=0 / M=1。