# Task T1-5 Fix Brief — F1：Write/Edit 确认门补齐（用户拍板选项 b）

## 背景

T1-5 审查 F1（Important，plan-mandated）：SW1-5 验收要求"全新配置下 Bash/Write/Edit/Delete 弹确认"，但 Write/Edit 的 `needs_permissions` 硬编码 false（pre-existing，2026-07-15 起，与 P1-6 同款反模式）。**用户已拍板选项 (b)：删覆写恢复默认 true，与 P1-6 修复方式对齐。**

## 改动点（编排者已实证定位，直接执行）

1. `src/crates/assembly/core/src/agentic/tools/implementations/file_write_tool/mod.rs:68-70` — 删除 `fn needs_permissions(&self, _input) -> bool { false }` 覆写，恢复 trait 默认 `!is_readonly()`（该工具 is_readonly=false → true）。
2. `src/crates/assembly/core/src/agentic/tools/implementations/file_edit_tool.rs:157-159` — 同上，删覆写。

## Spec

1. 两处覆写删除后，Write/Edit `needs_permissions()=true`；全新配置（T1-5 已翻转 combined_skip=false）下四工具全部弹确认。
2. 新测试（最小集）：Write 与 Edit 各一条 `needs_permissions()=true` 断言（经工具实例或 GetToolSpec，与 T1-5 已加的 Delete 断言同风格）。
3. 既有行为变化仅限：全新/未显式配置 skip_tool_confirmation 的用户，Write/Edit 开始弹确认；显式 `skip_tool_confirmation: true` 用户不变。report 里复述这句。
4. 不顺手做 M1（Bash 对称测试）/ M2（内部路径注释）——已挂账终审 triage，不归你。
5. 更新 `.superpowers/sdd/task-t1-5-report.md`：修正 §2.4 静默省略 Write/Edit 的表述与 §6 "无偏离" 声明，如实记录本轮补齐（judge 指出的报告缺陷，一并修）。

## Global Constraints（逐字遵守）

- 日志 English-only、无 emoji。
- 只改本 brief 列出的点；不扩张测试覆盖范围。
- 与 T1-5 主改动**分开的独立 commit**，message 后缀 `(T1-5 fix)`。

## 验证（命令 + 输出进 report；追加进 report 的验证节）

Windows MSVC wrapper：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. 能命中两个新断言的 focused `cargo test -p northhing-core --features product-full ...`（你跑什么写什么）
2. `cargo check --workspace`
3. `pnpm run fmt:rs`

## 派发元信息

- 这是 T1-5 的修复轮，叠在 `bec0ae7` 之上。
- 工作树无关脏文件（.opencode/model-capability-notes.md、memory/northhing.md、.handoffs/）不碰。
- 完成后最后一条消息以 DONE / BLOCKED 开头，附新 commit hash。
