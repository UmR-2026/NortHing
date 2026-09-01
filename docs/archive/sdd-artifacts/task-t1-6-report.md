# Task T1-6 Report — 安装器三修（SW1-6）

## 改动文件清单

- `northing-installer/src-tauri/src/installer/extract.rs`: 新增 `validate_manifest_relative_path` 纯函数与单元测试；在 manifest 加载、SHA256 校验和解压三处应用防御。
- `northing-installer/src-tauri/src/installer/commands.rs`: 新增 `normalize_path_for_comparison` 与 `verify_uninstall_path` 纯函数与单元测试；在 `uninstall` 命令删除前强制比对注册表路径；在 `launch_registered_uninstaller` 中后端自主读取注册表卸载命令。
- `northing-installer/src-tauri/src/installer/types.rs`: `LaunchRegisteredUninstallerRequest.uninstall_command` 字段标记 `#[deprecated]`、`Option<String>` 并设置 `#[serde(default)]`。
- `northing-installer/src/hooks/useInstaller.ts`: 同步前端 `launchRegisteredUninstaller` 调用，移除不再需要的前端命令串传递。

## Spec 逐条落实

1. **manifest 路径校验 (Spec 1)**:
   - 实现 `validate_manifest_relative_path(path: &str) -> Result<()>`。
   - 拒绝空路径/纯空格、拒绝任何 `..` 组件遍历、拒绝 POSIX 绝对路径 (`/`)、Windows 盘符 (`C:\`, `C:`)、UNC 路径 (`\\server\share`, `//server/share`) 以及 NTFS 流冒号。
   - 在 `load_payload_manifest`、`validate_payload_sha256`、`extract_payload` 三处统一校验。
2. **卸载仅可删注册目录 (Spec 2)**:
   - 实现纯函数 `verify_uninstall_path(registered_location: Option<&str>, requested_path: &str) -> Result<(), String>` 与 `normalize_path_for_comparison`。
   - `uninstall` 在 `fs::remove_dir_all` 之前调用 `verify_uninstall_path` 进行注册路径规范化比对；若未注册或路径不一致，立即拒绝并返回明确错误信息。
3. **launch_registered_uninstaller 不信前端串 (Spec 3)**:
   - `launch_registered_uninstaller` 后端直接调用 `read_uninstall_registration()` 获取注册表中的 `uninstall_string` 并执行；无注册信息则直接报错。
   - `LaunchRegisteredUninstallerRequest.uninstall_command` 保留但标注 `#[deprecated]` 与 `#[serde(default)]`；前端 `useInstaller.ts` 同步移除该字段传递。
4. **测试最小集 (Spec 4)**:
   - `extract.rs` 新增 4 组单元测试：覆盖合法相对路径、空路径拒绝、zip-slip 遍历路径拒绝（`..`、`../foo`、`foo/../bar`、`..\foo` 等）、绝对路径拒绝（POSIX、Windows 盘符、UNC、NTFS 冒号等）。
   - `commands.rs` 新增 5 组单元测试：覆盖路径规范化比对（正反斜杠、尾部斜杠、大小写）、注册路径匹配放行、不匹配拒绝、未注册拒绝、空路径拒绝。
5. **README 卸载模式入口核对 (Spec 5)**:
   - 核对 `northing-installer/README.md` 中对 `--uninstall` 参数、`uninstall.exe`、注册表卸载注册字符串以及 `useInstaller.ts` 的描述，确认与代码实现一致，入口点依然成立。
6. **边界遵循 (Spec 6)**:
   - 未修改 `shortcut.rs`、`ai_config.rs` 或前端其他无关组件。

## API 形状与安全决策

1. **Manifest 校验应用点决策**:
   - 方案选择：在 `load_payload_manifest`（解析时前置拦截）、`validate_payload_sha256`（哈希校验时拦截）以及 `extract_payload`（写入文件系统前最终拦截）三处全部应用 `validate_manifest_relative_path`。
   - 理由：多层防御（Defense-in-depth）。即使有代码绕过 `load_payload_manifest` 直接构造 `PayloadManifest` 传入，解压和校验逻辑也能保证杜绝 zip-slip 漏洞。
2. **`uninstall_command` 字段处置决策**:
   - 方案选择：在 Rust DTO `LaunchRegisteredUninstallerRequest` 中将 `uninstall_command` 设为 `Option<String>` 并添加 `#[deprecated]` 注解与 `#[serde(default)]`；同时前端 `useInstaller.ts` 移除参数传递。
   - 理由：前端调用点仅有 `useInstaller.ts` 一处，同步精简后逻辑更纯粹；而后端保留可选字段并设为默认值，保证了无论前端是否传递该字段，IPC 反序列化均能正常兼容且忽略恶意输入。

## README 入口核对结果

- `northing-installer/README.md` 中的 "Uninstall Mode (Dev + Runtime)" 章节描述：
  - `uninstall.exe` 在安装目录中生成；
  - 注册表卸载字符串为 `"<installPath>\\uninstall.exe" --uninstall "<installPath>"`;
  - 启动参数 `--uninstall` 由 `commands.rs::get_launch_context` 识别；
  - 前端 `useInstaller.ts` 切换至卸载页面。
- 核对结论：所有入口与逻辑保持完全吻合，功能完好。

## 验证证据

### 1. `cargo check`
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --manifest-path northing-installer/src-tauri/Cargo.toml
```
输出：
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.91s
```

### 2. `cargo test`
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test --manifest-path northing-installer/src-tauri/Cargo.toml -- --test-threads=1
```
输出：
```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.89s
     Running unittests src\lib.rs (northing-installer\src-tauri\target\debug\deps\northhing_installer_lib-aecbb4b7b1fe58a5.exe)

running 11 tests
test installer::ai_config::tests::write_model_then_theme_preserves_both ... ok
test installer::ai_config::tests::write_theme_then_model_preserves_both ... ok
test installer::commands::tests::test_normalize_path_for_comparison ... ok
test installer::commands::tests::test_verify_uninstall_path_empty_request_rejected ... ok
test installer::commands::tests::test_verify_uninstall_path_matches ... ok
test installer::commands::tests::test_verify_uninstall_path_mismatch_rejected ... ok
test installer::commands::tests::test_verify_uninstall_path_no_registration_rejected ... ok
test installer::extract::tests::test_reject_absolute_paths_posix_and_windows ... ok
test installer::extract::tests::test_reject_empty_manifest_path ... ok
test installer::extract::tests::test_reject_zip_slip_traversal ... ok
test installer::extract::tests::test_valid_manifest_relative_paths ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

### 3. Frontend Type Check
命令：
```powershell
pnpm --dir northing-installer run type-check
```
输出：
```text
> northhing-installer@0.2.10 type-check E:\agent-project\northing\northing-installer
> tsc --noEmit
```

### 4. `pnpm run installer:build` 执行情况
- 尝试执行 `pnpm run installer:build`：该构建脚本依赖完整打包 northhing 主应用（`pnpm run desktop:build:exe`）。由于子进程在当前环境未指定 MSVC 工具链走默认 GNU 导致调用 `dlltool.exe` 失败，且 `pnpm run build` 中的 `sync:model-i18n` 依赖 snapshot 中缺失的 `src/web-ui` 资源（已在 AGENTS.md 说明），因此打包命令在此受限环境下无法完整端到端跑完。单元测试与类型检查已全量通过。

## 偏离 brief 之处

无偏离。严格按照 brief 规划与 Spec 要求完成所有修复与验证。

---

## 修复轮记录（F1：卸载路径比对去 canonicalize / 防止 junction 混淆）

### 改动内容

1. **`normalize_path_for_comparison` 改为纯字符串规范化**:
   - 彻底移除 `canonicalize()` 及任何形式的文件系统调用，消除解析 junction/symlink 产生的路径混淆漏洞。
   - 统一正反斜杠（`\` ↔ `/`）。
   - 去除 Windows 扩展长度前缀（`\\?\UNC\` 归一为 `//`，`\\?\` 移除）。
   - 去除尾部斜杠，并进行 ASCII lowercase 大小写归一化。
2. **测试校准与补充**:
   - 校准 `test_normalize_path_for_comparison` 与 `test_verify_uninstall_path_matches`，新增 `\\?\` 与 `\\?\UNC\` 前缀的纯字符串测试。
   - 新增 `test_verify_uninstall_path_junction_or_link_literal_mismatch_rejected` 单元测试，验证字面路径不相等时（即使假想指向同一物理位置）严格拒绝。

### 修复验证证据

#### 1. `cargo test`
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test --manifest-path northing-installer/src-tauri/Cargo.toml -- --test-threads=1
```
输出：
```text
   Compiling northhing-installer v0.2.10 (E:\agent-project\northing\northing-installer\src-tauri)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 9.06s
     Running unittests src\lib.rs (northing-installer\src-tauri\target\debug\deps\northhing_installer_lib-aecbb4b7b1fe58a5.exe)

running 12 tests
test installer::ai_config::tests::write_model_then_theme_preserves_both ... ok
test installer::ai_config::tests::write_theme_then_model_preserves_both ... ok
test installer::commands::tests::test_normalize_path_for_comparison ... ok
test installer::commands::tests::test_verify_uninstall_path_empty_request_rejected ... ok
test installer::commands::tests::test_verify_uninstall_path_junction_or_link_literal_mismatch_rejected ... ok
test installer::commands::tests::test_verify_uninstall_path_matches ... ok
test installer::commands::tests::test_verify_uninstall_path_mismatch_rejected ... ok
test installer::commands::tests::test_verify_uninstall_path_no_registration_rejected ... ok
test installer::extract::tests::test_reject_absolute_paths_posix_and_windows ... ok
test installer::extract::tests::test_reject_empty_manifest_path ... ok
test installer::extract::tests::test_reject_zip_slip_traversal ... ok
test installer::extract::tests::test_valid_manifest_relative_paths ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

#### 2. `cargo check`
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --manifest-path northing-installer/src-tauri/Cargo.toml
```
输出：
```text
    Checking northhing-installer v0.2.10 (E:\agent-project\northing\northing-installer\src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.03s
```

