# Task T1-4 Report — ComputerUse 接 shell guard（SW1-4）

## 1. 改动文件清单

- `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_actions/system_actions/app_control.rs`:
  - `handle_open_app`: 在执行 shell fallback 策略前及每个 fallback 命令 spawn 前接入 `banned_shell_command` 与 `guard_command_execution(cmd_str, "ComputerUse", true)`。
  - `handle_run_script`: 脚本正文与合成命令行均接入 `banned_shell_command` 与 `guard_command_execution(..., "ComputerUse", true)`，命中返回 `ErrorCode::GuardRejected` 结构化拒绝响应。
- `src/crates/assembly/core/src/agentic/tools/implementations/computer_use_tool/actions.rs`:
  - 抽取并实现 `guard_apple_script_execution`，在 `run_apple_script_impl` 的 macOS spawn 及跨平台入口处先过 `banned_shell_command` 与 `guard_command_execution`。
  - 添加 AppleScript guard 单元测试模块。
- `src/crates/assembly/core/src/agentic/tools/implementations/control_hub_tool_tests.rs`:
  - 针对 `run_script` 和 `open_app` 的 banned command 与 denylist 命中拒绝场景添加测试。

---

## 2. Spec 落实说明

1. **三条路径全部接 guard**：
   - `handle_run_script`: 在解释器分发前后均经过 `guard_command_execution`，Denied 时返回结构化 `GUARD_REJECTED` 响应，不触发 child process spawn。
   - `handle_open_app`: 维持 host-first 策略，在进入 shell fallback 时对 `app_name` 与合成的 fallback 命令串进行 `guard_command_execution` 判定，Denied 时返回 `NortHingError::tool` 拒绝，不触发 child process spawn。
   - `run_apple_script_impl`: 在 `spawn_blocking` 之前（且在 macOS 平台检查前）调用 `guard_apple_script_execution`，Denied 时返回 `NortHingError::tool` 拒绝。
2. **`banned_shell_command` 同过**：
   - 三条路径均在执行前比对 `banned_shell_command`，命中返回带被禁命令名的安全拦截错误信息。
3. **命令串构造与 AppleScript 判定选择依据**：
   - `handle_open_app` 使用 `shell_safety::program_args_to_command_string` 合成各平台的 fallback 命令串（如 `cmd /C start "" <app>` / `open -a <app>` / `gtk-launch <app>`）。
   - `handle_run_script` 使用 `shell_safety::program_args_to_command_string` 合成各 `script_type` 对应的真实执行命令串（如 `bash -c <script>` / `powershell ... <script>` / `cmd /U /C ... <script>` / `osascript -e <script>`）。
   - **AppleScript 判定选择依据**：AppleScript 既可能以 `/usr/bin/osascript -e <script>` 形态整串被执行，脚本正文也可能包含直接的危险命令（如 `do shell script "shutdown -h now"` 或 `rm -rf /`），或者首 token 为 `banned_shell_command`（如 `alias`）。若仅检合成串，`banned_shell_command` 会因首词为 `osascript` 而遗漏脚本内部的被禁命令；若仅检脚本正文，则缺少对完整 spawn 参数的校验。因此本实现采取**双重判定**：先检查原始脚本正文，再检查通过 `program_args_to_command_string` 合成的完整 osascript 命令串。任何一项命中均立即拒绝。
4. **审计**：
   - `guard_command_execution` 统一传入 `tool_name = "ComputerUse"`，内部通过 `log_audit_event` 正确记录审计决策（如 `deny-denylist` / `allow-skip`）。
5. **测试（最小集）**：
   - `system_run_script_denied_by_banned_command`: 验证 `alias` 触发 banned command 拒绝。
   - `system_run_script_denied_by_denylist`: 验证 `rm -rf /` 触发 denylist 拒绝。
   - `system_run_script_applescript_denied_by_denylist_before_os_check`: 验证 `script_type="applescript"` 危险命令在非 macOS 环境下同样被安全层提前拒绝。
   - `system_open_app_denied_by_banned_command`: 验证 `open_app` 传入 `alias` 被拒。
   - `system_open_app_denied_by_denylist`: 验证 `open_app` 传入 `rm -rf /` 被拒。
   - `apple_script_denied_by_banned_command`: 验证 `guard_apple_script_execution("alias ...")` 被拒。
   - `apple_script_denied_by_denylist`: 验证 `guard_apple_script_execution("rm -rf /")` 被拒。
   - `apple_script_synthesized_osascript_denied_by_denylist`: 验证 `do shell script "shutdown -h now"` 触发 denylist 被拒。
   - `apple_script_clean_passes_guard`: 验证正常 AppleScript 允许通过。
6. **无顺手重构**：
   - host 优先、fallback 顺序、错误格式与现有逻辑完全兼容。

---

## 3. 验证命令及输出

### 1. `cargo test -p northhing-core --features product-full computer_use`
```text
running 28 tests
test agentic::tools::computer_use_host::tests::app_selector_constructors_populate_only_one_field ... ok
test agentic::tools::computer_use_host::tests::click_index_target_serializes_with_kind_tag ... ok
test agentic::tools::computer_use_host::tests::interactive_view_opts_apply_defaults_on_minimal_json ... ok
test agentic::tools::computer_use_host::tests::interactive_scroll_params_apply_defaults ... ok
test agentic::tools::implementations::computer_use_input::tests::screenshot_params_silently_ignore_implicit_center ... ok
test agentic::tools::implementations::computer_use_input::tests::screenshot_params_silently_ignore_crop_half_extent ... ok
test agentic::tools::computer_use_host::tests::interaction_state_serializes_expected_shape ... ok
test agentic::tools::implementations::computer_use_input::tests::screenshot_params_honor_window_flag ... ok
test agentic::tools::implementations::computer_use_input::tests::screenshot_params_silently_ignore_legacy_quadrant_and_crop_fields ... ok
test agentic::tools::implementations::computer_use_result::tests::append_interaction_state_includes_structured_block ... ok
test agentic::agents::definitions::subagents::computer_use::tests::computer_use_mode_basics ... ok
test agentic::tools::implementations::computer_use_result::tests::screenshot_body_keeps_existing_fields_and_adds_interaction_state ... ok
test agentic::tools::computer_use_host::tests::click_target_serializes_with_kind_tag ... ok
test agentic::tools::computer_use_host::tests::app_click_params_apply_defaults_on_deserialize ... ok
test agentic::tools::computer_use_host::tests::interactive_click_params_apply_defaults ... ok
test agentic::tools::computer_use_host::tests::interactive_type_text_params_round_trip ... ok
test agentic::tools::implementations::computer_use_tool::metadata::tests::multimodal_tool_output_format_whitelist ... ok
test agentic::tools::computer_use_host::tests::visual_mark_params_apply_defaults ... ok
test agentic::tools::computer_use_host::tests::app_wait_predicate_round_trips_each_variant ... ok
test agentic::tools::computer_use_host::tests::interactive_view_round_trips ... ok
test agentic::tools::implementations::computer_use_tool::actions::tests::apple_script_denied_by_banned_command ... ok
test agentic::agents::prompt_builder::tests::runtime_context_includes_computer_use_info_only_when_needed ... ok
test agentic::tools::implementations::control_hub_tool_tests::control_hub_tests::description_points_desktop_and_system_work_to_computer_use ... ok
test agentic::agents::registry::tests::computer_use_is_builtin_subagent_not_mode ... ok
test agentic::tools::registry::tests::registry_exposes_controlhub_and_computer_use ... ok
test agentic::tools::implementations::computer_use_tool::actions::tests::apple_script_denied_by_denylist ... ok
test agentic::tools::implementations::computer_use_tool::actions::tests::apple_script_synthesized_osascript_denied_by_denylist ... ok
test agentic::tools::implementations::computer_use_tool::actions::tests::apple_script_clean_passes_guard ... ok

test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 998 filtered out; finished in 0.05s
```

### 2. `cargo check --workspace`
```text
    Checking northhing-core v0.2.10 (E:\agent-project\northing\src\crates\assembly\core)
    Checking northhing v0.2.10 (E:\agent-project\northing\src\apps\desktop)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 50.77s
```

### 3. `pnpm run fmt:rs`
```text
> northhing@0.2.10 fmt:rs E:\agent-project\northing
> node scripts/format-changed-rust.mjs

[format-changed-rust] Formatting 3 Rust file(s).
```

---

## 4. 偏离 brief 之处

无偏离。所有路径均严格按照 brief 要求接入 `guard_command_execution` 与 `banned_shell_command`，并通过测试验证拒绝行为。

---

## 5. 派发与提交信息

- BASE commit: `0ac7e9a`
- HEAD commit: `0b656dd`
- Commit message: `feat(core): wire shell safety guard and banned command checks to ComputerUse actions (T1-4)`
