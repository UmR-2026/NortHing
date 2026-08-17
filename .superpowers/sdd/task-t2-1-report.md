DONE

# Task T2-1 Report: CI 补齐（构建面 + 测试面）

## 1. Diff 摘要（逐文件）

### 1.1 `.github/workflows/ci.yml`
- **Line 98**: 将 `cargo check --workspace --exclude northhing-cli --exclude northhing` 改为 `cargo check --workspace`。使 `northhing`（Slint 桌面端）和 `northhing-cli`（CLI 端）全量纳入 3 OS matrix 构建检查（Windows, macOS, Ubuntu），硬化 merge 前 compile gate。
- **Line 100-102**: 将原单 crate 测试 `cargo test --locked -p northhing-core` 扩展为 `cargo test --locked --workspace`，并添加 `if: matrix.os == 'ubuntu-latest'` 条件控制，仅在 Ubuntu 单 OS 运行全工作区测试，控制 CI 运行成本与多 OS 漂移。

### 1.2 `docs/architecture/backend-roadmap.md`
- **Line 42**: 修正过期描述，`cargo tree -p northhing-kernel-api` 守卫入 CI 从"尚未入 CI，见 T2-1"更新为"已在 CI，kernel-api-clean job"。
- **Line 166**: T2-1 任务行中关于 kernel-api 守卫表述更新为"已在 CI（kernel-api-clean job）"。

### 1.3 `docs/status/tech-debt-ledger.md`
- **Line 194**: 将 P2-15 条目状态更新为 `resolved (2026-08-17, T2-1: cargo check --workspace in CI includes northhing and northhing-cli; code defect resolved in b0bfe43; process gate recorded in housekeeping rule 6)`。

---

## 2. 验证原始输出

### 2.1 Verification 1: 全 workspace cargo check
命令：
```powershell
& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo check --workspace
```
原始输出（冷编译末尾及增量重检输出）：
```text
warning: `northhing-core` (lib) generated 19 warnings (run `cargo fix --lib -p northhing-core` to apply 18 suggestions)
    Checking northhing-acp v0.2.10 (E:\agent-project\northing\src\crates\interfaces\acp)
    Checking northhing-cli-internal v0.2.10 (E:\agent-project\northing\src\crates\support\cli-internal)
    Checking northhing-cli v0.2.10 (E:\agent-project\northing\src\apps\cli)
warning: `northhing-cli` (bin "northhing-cli") generated 1 warning (run `cargo fix --bin "northhing-cli" -p northhing-cli` to apply 1 suggestion)
warning: `northhing` (bin "northhing") generated 5 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.55s
```
结果：PASS（0 errors，全 30 个 workspace member crate 均编译通过）。

### 2.2 Verification 2: Core 边界检查
命令：
```powershell
node scripts/check-core-boundaries.mjs
```
原始输出：
```text
Core boundary check passed.
```
结果：PASS。

### 2.3 Verification 3: ci.yml 语法与 diff 结构核验
命令：
```powershell
git diff --check
```
原始输出：
```text
(0 whitespace errors / 0 conflict markers)
```
改动段 YAML 结构人工复述：
```yaml
      - name: Generate i18n locale contract
        shell: bash
        run: node scripts/generate-i18n-contract.mjs
        # generated_locale_contract.rs is gitignored; northhing-core fails E0583 without it

      - name: Check compilation
        run: cargo check --workspace

      - name: Run workspace Rust tests
        if: matrix.os == 'ubuntu-latest'
        run: cargo test --locked --workspace
```
- `rust-build-check` 任务在 `ubuntu-latest`、`macos-15`、`windows-latest` 上均执行 `Check compilation`（全 workspace）。
- `Run workspace Rust tests` 仅在 `matrix.os == 'ubuntu-latest'` 时执行 `cargo test --locked --workspace`。
- YAML 缩进（6 空格 / 8 空格）完全对齐。

---

## 3. Workspace 内 Crate 测试盘点与 Ubuntu CI OS 敏感测试风险清单

### 3.1 Workspace Crate 测试分布盘点（30 个 member crates）

| Crate | 路径 | tests/ 目录 | `#[test]` / `#[tokio::test]` 数量 |
|---|---|---|---|
| `cli` | `src/apps/cli` | 否 | 34 |
| `desktop` | `src/apps/desktop` | 否 | 101 |
| `relay-server` | `src/apps/relay-server` | 是 | 26 |
| `server` | `src/apps/server` | 否 | 1 |
| `ai-adapters` | `src/crates/adapters/ai-adapters` | 是 | 127 |
| `webdriver` | `src/crates/adapters/webdriver` | 否 | 0 |
| `core` | `src/crates/assembly/core` | 是 | 1154 |
| `product-capabilities` | `src/crates/assembly/product-capabilities` | 是 | 8 |
| `core-types` | `src/crates/contracts/core-types` | 是 | 7 |
| `events` | `src/crates/contracts/events` | 否 | 8 |
| `kernel-api` | `src/crates/contracts/kernel-api` | 否 | 0 |
| `product-domains` | `src/crates/contracts/product-domains` | 是 | 88 |
| `runtime-ports` | `src/crates/contracts/runtime-ports` | 是 | 46 |
| `agent-dispatch` | `src/crates/execution/agent-dispatch` | 是 | 24 |
| `agent-runtime` | `src/crates/execution/agent-runtime` | 是 | 261 |
| `agent-stream` | `src/crates/execution/agent-stream` | 否 | 48 |
| `harness` | `src/crates/execution/harness` | 是 | 5 |
| `runtime-services` | `src/crates/execution/runtime-services` | 是 | 7 |
| `tool-contracts` | `src/crates/execution/tool-contracts` | 是 | 88 |
| `tool-execution` | `src/crates/execution/tool-execution` | 是 | 87 |
| `tool-provider-groups` | `src/crates/execution/tool-provider-groups` | 否 | 8 |
| `acp` | `src/crates/interfaces/acp` | 否 | 51 |
| `debug-log` | `src/crates/services/debug-log` | 否 | 2 |
| `relay-core` | `src/crates/services/relay-core` | 否 | 38 |
| `services-core` | `src/crates/services/services-core` | 是 | 82 |
| `services-integrations` | `src/crates/services/services-integrations` | 是 | 200 |
| `terminal` | `src/crates/services/terminal` | 否 | 22 |
| `cli-internal` | `src/crates/support/cli-internal` | 否 | 0 |
| `test-support` | `src/crates/support/test-support` | 是 | 15 |
| `plan-compliance-checker` | `tools/plan-compliance-checker` | 是 | 19 |
| **总计** | — | **16 个含 tests/** | **约 2507 个标注测试** |

### 3.2 Ubuntu (Linux CI) 环境 OS 敏感套件风险清单

1. **Keyring / Secret Service 守护进程缺失风险**：
   - **涉及 Crate**：`src/apps/desktop` (`keyring` 4.1.6)
   - **风险等级**：低（已解耦）
   - **现状评估**：`desktop` 中的 settings/keyring 单元测试与 IO 迁移测试均已采用 `MockKeyring`（基于内存 HashMap 注入），未直接调用 Linux Secret Service API。
2. **PTY / 终端会话分配与权限风险**：
   - **涉及 Crate**：`src/crates/services/terminal` (`portable-pty`), `services-core`
   - **风险等级**：低
   - **现状评估**：`terminal` crate 的 22 个测试主要覆盖 ANSI 序列解析、缓冲区序列化与环境变量过滤逻辑；在标准 GitHub Actions Ubuntu runner 中 `/dev/pts` 默认可用。
3. **无头显示环境（Headless DISPLAY / X11 / Wayland）与图形/剪贴板依赖风险**：
   - **涉及 Crate**：`src/apps/desktop` (`slint`), `src/apps/cli` (`arboard`)
   - **风险等级**：中低
   - **现状评估**：`desktop` 单元测试主要聚焦状态机、event_bridge、settings、streaming lifecycle 等数据流；未拉起真实 GUI 窗口事件循环。`cli` 测试集中在参数解析与聊天状态分词；未在测试中直接调用 `arboard::Clipboard::new()`。
4. **路径分隔符与规范化（`dunce` vs `std::fs::canonicalize`）**：
   - **涉及 Crate**：`northhing-core`, `services-core`, `product-domains`
   - **风险等级**：低
   - **现状评估**：大部分代码路径使用 `PathBuf::join` 与 `dunce::canonicalize`；Linux 上不存在 UNC 前缀（`\\?\`），路径处理比 Windows 更为标准。
5. **本地网络端口绑定与并发冲突（`127.0.0.1`）**：
   - **涉及 Crate**：`relay-server`, `relay-core`, `services-integrations`
   - **风险等级**：低
   - **现状评估**：集成测试主要绑定 ephemeral port（port 0）或使用 tokio duplex stream，未硬编码固定对外端口。
