# Task T1-6 Review — 安装器三修（SW1-6）

**Reviewer**: independent judge (read-only, no commits)
**Range**: `e5a484a..cdfd059` (4 files, +256/-16)
**Subject**: zip-slip + 卸载目录校验 + `launch_registered_uninstaller` 不信前端串
**Risk class**: 安全任务（installer 权限边界 + 注册表 + 文件系统递归删除）

---

## 双判决概览

| 判决 | 结果 |
|---|---|
| SPEC 判决 | ✅ PASS — Spec 1-6 全部满足 |
| QUALITY 判决 | ⚠️ PASS w/ residual risk — 0 Critical / 1 Important / 2 Minor |

总体：**APPROVED**（Important 项属于 brief 明确选择的规范化路径的固有残余风险，需在 README/ledger 留痕，但不阻塞合并；Minor 项同样不阻塞）。

---

## SPEC 判决（逐条）

| # | Spec 条款 | 判决 | 证据 |
|---|---|---|---|
| 1 | manifest 路径校验：拒绝绝对路径、`..`、空路径；Windows 盘符/UNC；extract + sha256 + manifest 解析三条路径全部生效 | ✅ | `extract.rs:27-76` `validate_manifest_relative_path` 实现；消费点：`extract.rs:84-87`（`load_payload_manifest`）、`extract.rs:94-97`（`validate_payload_sha256`，失败计入 failures Vec 不中断后续）、`extract.rs:146-147`（`extract_payload` 写入前最终拦截）。三层防御均到位。 |
| 2 | 卸载仅可删注册目录：read_uninstall_registration + canonicalize 比对 + 不一致/不存在返回明确错误；纯函数可测 | ✅ | `commands.rs:534-561` `verify_uninstall_path` + `commands.rs:484-532` `normalize_path_for_comparison`；`commands.rs:434-438` 在 `remove_dir_all` 之前调用；注册不存在 → "no valid installation is registered"；canonicalize 成功则 strip `\\?\UNC\` / `\\?\` 前缀、统一大小写、统一斜杠、trim 尾斜杠；测试 `test_verify_uninstall_path_no_registration_rejected` 覆盖 None/Some("")/Some("   ") 三态。 |
| 3 | `launch_registered_uninstaller` 不信前端串：后端读注册表 `uninstall_string` 执行；注册不存在 → 报错；DTO 字段处置 + 前端同步 | ✅ | `commands.rs:253-260` `launch_registered_uninstaller` 现在调用 `read_uninstall_registration()` 取 `reg.uninstall_string`，参数 `_request` 不被访问；`types.rs:144-153` `LaunchRegisteredUninstallerRequest.uninstall_command` 改为 `Option<String>` + `#[deprecated]` + `#[serde(default)]`，向后兼容旧前端；`useInstaller.ts:212-216` 同步移除 `uninstallCommand` 字段传递，error 路径交给后端 try/catch 处理（`useInstaller.ts:223-225`）。grep `uninstallCommand` 在 `src/` 全域为 0 匹配。 |
| 4 | 测试最小集：zip-slip（`..`、POSIX 绝对、Windows 绝对）+ 卸载比对（匹配放行/不等拒）；同文件 `#[cfg(test)]` | ✅ | `extract.rs:190-232` 4 组测试（合法路径 / 空 / `..` 7 例 / 绝对路径 9 例）；`commands.rs:632-685` 5 组测试（normalize / 匹配放行 / 不等拒 / 未注册拒）。共 9 组新测试，符合 brief 数量下限。断言形式 `is_ok()`/`is_err()`，真实校验拒绝行为而非仅调用。`cargo test` 输出 11 passed / 0 failed（实测）。 |
| 5 | README 卸载模式入口核对 | ✅ | `README.md:264-279` 描述：`uninstall.exe` 生成、注册表 uninstall_string 格式、`--uninstall` 启动参数、`uninstall.exe` 自动进入 uninstall mode。代码层面：`registry.rs:20-25` 仍按 `"{uninstall_exe}" --uninstall "{install_path}"` 生成注册串；`commands.rs:36-44` 仍解析 `--uninstall` 参数；`commands.rs:46-53` 仍按 exe 文件名含 "uninstall" 推断 mode；`useInstaller.ts:97-105` 仍根据 `LaunchContext.mode === 'uninstall'` 切 UI。入口点全部成立。 |
| 6 | 不顺手改 shortcut.rs / ai_config.rs / 前端其他部分 | ✅ | diff 仅触及 `commands.rs`、`extract.rs`、`types.rs`、`useInstaller.ts` 4 文件，与 brief 范围一致；`git diff --stat` 输出确认；`git status` 工作树中预存的脏文件（`.opencode/...`、`memory/...`、`.handoffs/...`）未被本任务 commit，符合 brief "不要碰" 要求。 |

### Global Constraints（逐字核查）

| 约束 | 判定 |
|---|---|
| 日志 English-only、无 emoji | ✅ 所有错误信息为 English（"manifest path is empty", "Uninstall rejected: ..." 等），无 emoji |
| 只改本 brief 列出的点；不顺手重构、不扩张测试覆盖范围 | ✅ diff 与 brief 列出的 4 文件完全一致；测试仅覆盖漏洞点，未扩张 |
| Tauri command 命名 snake_case、TS 侧 camelCase 包装（若动 IPC 形状） | ✅ command 仍为 `launch_registered_uninstaller` / `uninstall`；DTO `rename_all = "camelCase"`；TS 侧 `installPath` / `installLocation` 一致 |
| 安全修复：所有"拒绝"路径必须有明确错误信息，不许静默放行 | ✅ 见 `validate_manifest_relative_path` 5 处 `anyhow::bail!` 全部携带 offending path；`verify_uninstall_path` 3 处拒绝全部带原因；`launch_registered_uninstaller` 注册缺失带原因；`uninstall` 注册不匹配带原因 |

---

## QUALITY 判决

### 安全深度分析（zip-slip / 卸载 / uninstaller 命令）

#### Zip-slip 校验完备性

`validate_manifest_relative_path`（`extract.rs:27-76`）覆盖：

| 攻击向量 | 拒绝方式 | 覆盖测试 |
|---|---|---|
| 空 / 全空白 | `trimmed.is_empty()` → bail | `test_reject_empty_manifest_path` |
| POSIX 绝对 `/etc/...` | `starts_with('/')` + `is_absolute()` | `test_reject_absolute_paths_posix_and_windows` |
| Windows 反斜杠绝对 `\Windows\...` | `starts_with('\\')` | `test_reject_absolute_paths_posix_and_windows` |
| Windows 盘符 `C:\...` | `contains(':')`（前置拦截）+ `Component::Prefix` 兜底 | `test_reject_absolute_paths_posix_and_windows` |
| Drive-relative `C:northhing.exe` | `contains(':')` | `test_reject_absolute_paths_posix_and_windows` |
| UNC `\\server\share\...` | `starts_with('\\')` + `starts_with(r"\\")` | `test_reject_absolute_paths_posix_and_windows` |
| UNC POSIX-flavored `//server/share/...` | `starts_with("//")` + `is_absolute()` 兜底 | `test_reject_absolute_paths_posix_and_windows` |
| NTFS ADS `file.txt:stream` | 整体 `contains(':')` 拒绝（粗粒度但有效） | `test_reject_absolute_paths_posix_and_windows` |
| 父目录遍历 `..` / `../foo` / `foo/../bar` | `Component::ParentDir` + `Component::Normal("..")` + 按 `/` 切分兜底 | `test_reject_zip_slip_traversal` |
| 反斜杠 traversal `..\foo` / `foo\..\bar` | `replace('\\','/')` 归一化后被 `Component` / `split('/')` 捕获 | `test_reject_zip_slip_traversal` |

三消费点覆盖：
1. `load_payload_manifest` (line 84-87)：解析后立即全量校验，提前 fail-fast（`anyhow::Result<PayloadManifest>`，上层 `start_installation` 在 `extract_payload` 之前已两次调用此函数）
2. `validate_payload_sha256` (line 94-97)：失败计入 `failures` Vec，不中断后续文件 hash 校验；最终由 `extract_payload:138-141` 的 `if !failures.is_empty() { bail! }` 一并拒绝
3. `extract_payload` (line 146-147)：在 `fs::copy` 写入前再次校验（最后一道闸门）

三层防御成立。即使 manifest 被绕过（例如代码层直接构造 `PayloadManifest`），extract 也会拒。

#### NTFS 冒号拒绝的合理性

`contains(':')` 是粗粒度拒绝：所有冒号一概拒绝。这同时覆盖：
- 盘符前缀 `C:` `D:` 等
- NTFS ADS `:` 分隔符
- POSIX 文件名中的合法 `:`（Linux 允许）

对 installer 目标（Windows 安装目录）而言，文件名包含 `:` 在 Windows 本来就非法（Windows 保留字符 `<>:"/\|?*`）。**对该产物无合法误伤**。在 Linux 上部署时若 manifest 真含 `:` 文件名会被过度拒绝——但 `northing-installer` 当前 build 目标仅 Windows，无现实误伤。✅

#### 卸载目录校验残余风险

`verify_uninstall_path` + `normalize_path_for_comparison` 覆盖：

| 绕过尝试 | 防御 |
|---|---|
| 大小写差异 `c:\program files\...` vs `C:\Program Files\...` | ✅ `to_lowercase()` |
| 正反斜杠差异 `C:\foo` vs `C:/foo` | ✅ `replace('\\', '/')` + `canonicalize()` |
| 尾斜杠差异 `C:\foo\` vs `C:\foo` | ✅ `trim_end_matches('/')` |
| `\\?\` 扩展长度前缀 | ✅ strip prefix |
| `\\?\UNC\` 前缀 | ✅ strip prefix（保留 `\\` 前缀） |
| 8.3 短名 `PROGRA~1` | ✅ `canonicalize()` 在 Windows 上解析为长名（前提：路径存在且 canonicalize 成功） |

**Important 残余风险**：junction / symlink path confusion（见 findings Q-1）

**注册表读取失败 fail-closed**：`commands.rs:434-438` 调用 `read_uninstall_registration()` 返回 `Option<UninstallRegistration>`；`verify_uninstall_path` 在 `None` / `Some("")` / `Some("   ")` 时全部 fail-closed 返回 "no valid installation is registered"。`registry.rs:67` `let install_location: String = key.get_value("InstallLocation").ok()?;` 用 `?` 传播 None（key 存在但 `InstallLocation` 缺失也会让整个 `read_uninstall_registration()` 返回 `None`）。**拒绝路径不会静默放行**。✅

#### `launch_registered_uninstaller` 向后兼容

- `types.rs:147-153`：`Option<String>` + `#[serde(default)]` 保证旧前端发送 `uninstallCommand: "..."` 也能反序列化为 `Some(...)`（被忽略），新前端不发送则默认 `None`。
- `commands.rs:253` `#[allow(deprecated)]`：参数命名 `_request`，不访问字段，理论上不需要此属性；但作为防御性标记无害。
- `useInstaller.ts:212-216`：移除 `uninstallCommand` 字段，`installPath` 沿用。TypeScript 类型层未引用 `uninstallCommand`，前端 grep 为 0 匹配。
- 后端 IPC 形状与前端调用一致（`{ request: { installPath: ... } }`）。✅

#### 测试质量

9 组测试均通过 `is_ok()` / `is_err()` 断言拒绝行为，而非仅调用函数：
- `test_valid_manifest_relative_paths`：5 例正向（覆盖 Windows / POSIX 分隔符、`./` 前缀）
- `test_reject_empty_manifest_path`：空 / 全空白
- `test_reject_zip_slip_traversal`：7 例 traversal（含混合反斜杠、多级 `..\..\..\secret.txt`）
- `test_reject_absolute_paths_posix_and_windows`：9 例绝对路径（含 POSIX、Windows 盘符、UNC POSIX、NTFS ADS）
- `test_normalize_path_for_comparison`：4 例 normalize 等价/不等价
- `test_verify_uninstall_path_matches`：3 例匹配（大小写、斜杠方向、尾斜杠）
- `test_verify_uninstall_path_mismatch_rejected`：3 例不匹配（完全不等、前缀关系、子串关系）
- `test_verify_uninstall_path_no_registration_rejected`：3 例未注册（None / Some("") / Some("   ")）
- `test_verify_uninstall_path_empty_request_rejected`：2 例空请求

**测试真实反映拒绝行为，非仅烟雾调用**。✅

#### README 入口核对

report §"README 入口核对结果" 已逐条声明（uninstall.exe 生成位置、注册表 uninstall_string 格式、`--uninstall` arg、useInstaller 切换）；与 `registry.rs:20-25` / `commands.rs:36-44` / `useInstaller.ts:97-105` 一致。入口点未改变。✅

#### 验证缺口核查

brief 第 53 条要求尝试 `pnpm run installer:build`。report §4 如实标注：
> 尝试执行 `pnpm run installer:build`：该构建脚本依赖完整打包 northhing 主应用...由于子进程在当前环境未指定 MSVC 工具链走默认 GNU 导致调用 `dlltool.exe` 失败，且 `pnpm run build` 中的 `sync:model-i18n` 依赖 snapshot 中缺失的 `src/web-ui` 资源（已在 AGENTS.md 说明），因此打包命令在此受限环境下无法完整端到端跑完。单元测试与类型检查已全量通过。

如实标注尝试了、跑了哪些、为什么未跑完。✅

---

## Findings

### Critical（0）

无。

### Important（1）

#### Q-1 junction / symlink path confusion via canonicalize()（残余风险，需 ledger 留痕）

**位置**：`commands.rs:484-498` `normalize_path_for_comparison` 优先 `p.canonicalize()`

**场景**：如果 install 时用户/管理员主动把 `install_path` 设为 junction（例如 `C:\MyJunction` → `C:\Windows`），installer 把 junction 路径写入注册 `InstallLocation`。之后 uninstall 时如果有人传 `C:\Windows` 作为 `request.install_path`：
1. `verify_uninstall_path` 调用 `normalize_path_for_comparison` 两端
2. `C:\MyJunction.canonicalize()` 在 Windows 上解析为 `C:\Windows`（跟随后端目标）
3. `C:\Windows.canonicalize()` 已是 canonical
4. 字符串相等，校验放行
5. `fs::remove_dir_all("C:\Windows")` 删 Windows 目录

**前置条件**：攻击者需要 admin 权限在 install 时建 junction + 让 installer 把 junction 当 install 路径。攻击门槛高，但路径混淆本身不防御。

**为什么这是 brief 选择下的固有残余**：brief 第 32 条明确写 "与 `request.install_path` 规范化（canonicalize/反斜杠/尾斜杠差异）后比对"，canonicalize 是指定方案。junction 解析是 canonicalize 的固有行为。

**建议（不阻塞合并）**：
- 选项 A：在 `verify_uninstall_path` 通过之后、`remove_dir_all` 之前，加一道 `fs::symlink_metadata(install_path)`，若 `file_type().is_symlink()` 则拒绝（防止请求侧是 symlink）。
- 选项 B：在 `normalize_path_for_comparison` 中，**先**做字符串前缀等粗糙比对（保留原始形态），不一致直接拒绝；只有原始形态一致时才 canonicalize 比对。这样 `C:\MyJunction` 和 `C:\Windows` 原始字符串不等，直接 fail。
- 选项 C：在 `build_uninstall_registration` 写入注册表前，若 `install_path` 是 junction，canonicalize 并存长名进注册 `InstallLocation`。

**评估**：本任务下不修复。**仅记 ledger**，由后续 hardening 任务处理。

### Minor（2）

#### Q-2 NUL byte 未显式拒绝（防御层可选）

**位置**：`extract.rs:27-76` `validate_manifest_relative_path`

**观察**：validator 未检查 `\0` 字符。下游 `fs::copy` / `fs::create_dir_all` 会因 NUL byte 失败（OS syscall 拒绝），所以**实际**不会被写入。但显式 bail 更早、更可观察。

**建议（不阻塞）**：在 `trimmed.is_empty()` 后加 `if trimmed.contains('\0') { bail }`。

#### Q-3 `#[allow(deprecated)]` 在 `launch_registered_uninstaller` 不必要

**位置**：`commands.rs:253`

**观察**：参数命名为 `_request`，未被访问，Rust 不会对"函数参数类型含 deprecated 字段"触发 warning。该属性无效果（无害但冗余）。

**建议（不阻塞）**：可移除；保留亦可（防御性）。

---

## 双判决结论

| 判决 | 通过 | 备注 |
|---|---|---|
| SPEC | ✅ PASS | Spec 1-6 全部满足，Global Constraints 全部遵守 |
| QUALITY | ✅ PASS w/ 1 Important residual | Q-1 junction 路径混淆由 brief 选择的 canonicalize 方案固有引入，记 ledger 不阻塞；Q-2/Q-3 留待后续 hardening |

**最终**：APPROVED（0 Critical / 1 Important / 2 Minor）

---

## Round 2 — F1 修复复核（`cdfd059..3891080`）

**Reviewer**: independent judge (read-only, no commits)
**Range**: `cdfd059..3891080` (1 file, +33/-42)
**Subject**: F1 闭环 — `normalize_path_for_comparison` 去 `canonicalize()`，纯字符串规范化
**Risk class**: 安全任务（残余风险面收口）

---

### F1 闭环判定

**整改方向 = 纯字符串规范化，禁 canonicalize，禁一切文件系统访问**

| 检查项 | 证据 | 判决 |
|---|---|---|
| `normalize_path_for_comparison`（commands.rs:484-505）移除 canonicalize | 全文 0 处 `canonicalize` / 0 处 `Path::` / 0 处 `fs::` 调用；只使用 `replace('\\','/')`、`starts_with`、`trim_end_matches`、`to_ascii_lowercase` | ✅ 闭环 |
| `verify_uninstall_path`（commands.rs:507-534）fail-closed 不变 | 注册缺失 → "no valid installation is registered"；空请求 → "Install path is empty"；不等 → "Uninstall rejected: requested path '...' does not match registered install location '...'"；所有拒绝路径都带明确错误信息 | ✅ 不变 |
| `\\?\UNC\` / `\\?\` 前缀剥离 | 用 `lower_prefix.starts_with("//?/unc/")` / `"//?/"` 归一化为 `//` 或裸盘符路径；mixed-case 也正确处理（probe 验证 `\\?\UNC\Server\Share\Foo` ↔ `\\server\share\foo` 等价） | ✅ 正确 |
| 大小写 / 正反斜杠 / 尾斜杠归一 | 现有 5 组测试（已扩 `\\?\` 用例）全部覆盖；probe 覆盖 17 例 | ✅ 完备 |
| 其他位置无残留 canonicalize | grep `canonicalize` 在 `commands.rs` 仍命中 1 处（line 537，在 `normalize_path` 函数中，用于 `validate_install_path`），与 F1 范围无关；F1 明确只针对 `verify_uninstall_path` 路径 | ✅ 不在 F1 范围，不视为残留 |

**链接解析攻击面消除**：原代码 `p.canonicalize()` 会在 Windows 上跟随 junction/symlink，把 `C:\MyJunction` 解析为 `C:\Program Files\northhing` 后放行。修复后函数不触碰文件系统，纯字面字符串比对——攻击者即使建 junction 指向 install dir，传入的 `C:\MyJunction` 字面与注册表 `C:\Program Files\northhing` 字面直接判不等，**拒绝**。probe 实测 `C:\Program Files\northhing` vs `C:\MyJunction` → `not equal`。

---

### 规范化完备性反向核查（误放行风险）

跑独立 probe（17 例）实测字面 → 规范化映射：

| 类别 | 输入 | 规范化输出 | 期望 | 实际 |
|---|---|---|---|---|
| 同义放行 | `C:\Program Files\northhing` ↔ `C:/Program Files/northhing/` | `c:/program files/northhing` | equal | ✅ equal |
| 同义放行 | `c:\program files\northhing\` ↔ `C:\Program Files\northhing` | `c:/program files/northhing` | equal | ✅ equal |
| `\\?\` 前缀剥离 | `\\?\C:\Program Files\northhing` ↔ `C:\Program Files\northhing` | `c:/program files/northhing` | equal | ✅ equal |
| `\\?\UNC\` 归一 | `\\?\UNC\server\share\foo` ↔ `\\server\share\foo` | `//server/share/foo` | equal | ✅ equal |
| `\\?\UNC\` 大小写 + 路径大小写 | `\\?\UNC\Server\Share\Foo` ↔ `\\server\share\foo` | `//server/share/foo` | equal | ✅ equal |
| `\\?\UNC\` 尾斜杠 | `\\?\unc\server\share\foo\` ↔ `\\server\share\foo` | `//server/share/foo` | equal | ✅ equal |
| 不同盘符 | `C:\Program Files\northhing` vs `D:\Program Files\northhing` | `c:/program files/northhing` vs `d:/program files/northhing` | not equal | ✅ not equal |
| 不同子目录 | `C:\foo` vs `C:\foo\bar` | `c:/foo` vs `c:/foo/bar` | not equal | ✅ not equal |
| 短横线后缀 | `C:\foo` vs `C:\foo-bar` | `c:/foo` vs `c:/foo-bar` | not equal | ✅ not equal |
| `..` 父遍历 | `C:\foo` vs `C:\foo\..\bar` | `c:/foo` vs `c:/foo/../bar` | not equal | ✅ not equal（旧 `canonicalize` 会折叠为 `c:/bar` 误放行；现在不折叠，**fail-closed**） |
| `.` 当前目录 | `C:\foo` vs `C:\foo\.` | `c:/foo` vs `c:/foo/.` | not equal | ✅ not equal |
| **Junction 攻击** | `C:\Program Files\northhing` vs `C:\MyJunction` | `c:/program files/northhing` vs `c:/myjunction` | not equal | ✅ **not equal**（攻击放行面已封死） |
| Drive-relative | `C:\foo` vs `C:foo` | `c:/foo` vs `c:foo` | not equal | ✅ not equal |
| 多重分隔符 | `C:\foo` vs `C:\\foo` | `c:/foo` vs `c://foo` | not equal | ✅ not equal（保守 fail-closed，合法但病态输入被拒，可接受） |

**误放行（false-accept）= 0 例**。所有不同字面路径判不等，所有合理同义形式判相等。`..` / `.` 不折叠是有意的——brief 明确"纯字符串比对、不跟随任何链接"，且原 canonicalize 反而是攻击面入口。

**非 ASCII 字符大小写折叠**（如 `Café` vs `café`）：`to_ascii_lowercase` 仅处理 ASCII。Windows installer 路径实际均为 ASCII，理论缺口不构成现实风险。**不视为新引入的 regression**（原 canonicalize 方案在 Windows 上也不做 Unicode casefold）。如未来需要支持，可换 `to_lowercase`（Unicode-aware 但速度稍慢）—— 留待后续 hardening 任务。

---

### 新测试断言真实拒绝行为

`test_verify_uninstall_path_junction_or_link_literal_mismatch_rejected`（commands.rs:655-661）：

```rust
let reg = Some(r"C:\Program Files\northhing");
assert!(verify_uninstall_path(reg, r"C:\AppJunction").is_err());
assert!(verify_uninstall_path(reg, r"C:\SymlinkTarget").is_err());
```

✅ 用 `is_err()` 断言拒绝（不是 `is_ok()`、不是仅调用），且传入的路径与注册路径在文件系统上**没有任何关联**——纯字面差异即拒。即使攻击者真的建立了指向 install 目录的 junction 或 symlink，本函数也不再跟随，行为是确定的、纯字面的。test 真实反映拒绝行为。

---

### Fix brief 纪律核查

| 约束 | 判定 |
|---|---|
| 独立 commit，message 后缀 `(T1-6 fix)` | ✅ `3891080 fix(installer): use pure string normalization for uninstall path comparison to prevent junction confusion (T1-6 fix)` |
| 只改 1 个目标文件 | ✅ `git show --stat 3891080` 输出 `1 file changed, 33 insertions(+), 42 deletions(-)`，仅 `commands.rs`；`git diff 3891080^..3891080 -- <other 3 files>` 为空 |
| 未顺手做上轮 Minors（Q-2 NUL byte、Q-3 `#[allow(deprecated)]`） | ✅ `commands.rs:253` 仍保留 `#[allow(deprecated)]`；`extract.rs:27-76` `validate_manifest_relative_path` 全文无 `\0` 检查（grep 验证 0 匹配）；Q-2/Q-3 完整留待终审 triage |
| 不碰工作树预存脏文件（.opencode/、memory/、.handoffs/） | ✅ `git status` 显示这些文件与 fix 前一致，仅 brief 产物（task-t1-6-*）新增为 untracked（SDD 流程文件，非代码） |
| 日志 English-only、无 emoji | ✅ 测试注释"Even if C:\AppJunction pointed to C:\Program Files\northhing, differing literals are rejected"是 English，无 emoji |
| 所有拒绝路径必须明确错误信息 | ✅ `verify_uninstall_path` 三处拒绝（空请求 / 未注册 / 不等）全部带原因字符串 |

---

### 第一轮通过项无回归核查

| 第一轮通过项 | 修复后状态 | 证据 |
|---|---|---|
| Spec 1 manifest 路径校验（3 消费点） | ✅ 不变 | `extract.rs:84-87 / 94-97 / 146-147` 3 处 `validate_manifest_relative_path` 调用未变；4 组测试仍通过 |
| Spec 2 卸载注册路径校验语义 | ✅ 收紧 | 仍 fail-closed；拒绝原因字符串未变；只是"如何判等"由 canonicalize 改为字面，攻击面缩小 |
| Spec 3 `launch_registered_uninstaller` 不信前端串 | ✅ 不变 | `commands.rs:253-260` 未动；`#[allow(deprecated)]` 仍在；`useInstaller.ts` 调用点未动 |
| 9 组原始测试 | ✅ 仍全过 | `test_normalize_path_for_comparison`、`test_verify_uninstall_path_matches` 扩 `\\?\` 用例（additive，不破坏原断言）；其余 7 组字面零变化 |
| README 入口核对 | ✅ 入口未变 | 本次未改任何注册表写入/读取路径、未改 mode 判定、未改前端调用；README 仍准确 |

---

### 验证证据

命令（编排者抽查，read-only）：

```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo test --manifest-path northing-installer/src-tauri/Cargo.toml -- --test-threads=1
```

输出：

```text
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

独立 probe（17 例字面 → 规范化映射）在 judge 临时目录构造并删除，未污染仓库；输出见上文"规范化完备性反向核查"表。所有 MUST-EQUAL 等价、MUST-DIFFERENT 不等。

---

### 双判决结论

| 判决 | 通过 | 备注 |
|---|---|---|
| SPEC | ✅ PASS | F1 brief 全部 4 条要求（纯字符串、禁 canonicalize、禁 FS、仅字面）逐条满足；独立 commit + 1 文件 + 后缀正确；不顺手做 Minor 守住 |
| QUALITY | ✅ PASS | 0 Critical / 0 Important / 0 Minor。F1 Important 残余风险已消除；上一轮 Q-2/Q-3 仍按预期留待终审 triage，无新增问题 |

**Round 2 终结**：F1 闭环彻底。junction/symlink 攻击面从设计上消除（不再触碰 FS），无 false-accept，无 regression。

**最终**：APPROVED（0 Critical / 0 Important / 0 Minor）