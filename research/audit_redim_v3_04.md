## 依赖与编译第三次审核报告

**审核日期**: 2026-06-27 
**审核员**: 依赖与编译审核员_v3 
**项目**: Northing 
**基准**: audit_redim04.md（上一轮未修复清单）

---

### 1. 修复状态总览

| 问题 | 上次状态 | 当前状态 | 是否修复 |
|------|----------|----------|----------|
| 边界泄露（CLI直接引用core内部模块） | 未修复 | 未修复 | ❌ 否 |
| installer依赖版本未统一 | 未修复 | 未修复 | ❌ 否 |
| workspace exclude拼写错误 | 未修复 | 未修复 | ❌ 否 |
| sccache未配置 | 未修复 | 未修复 | ❌ 否 |
| split-debuginfo未配置 | 未修复 | 未修复 | ❌ 否 |
| target-dir未重定向 | 未修复 | 未修复 | ❌ 否 |
| installer独立profile重复 | 未修复 | 未修复 | ❌ 否 |
| target/debug清理（180GB→970MB） | 已修复 | 回退至27GB | ⚠️ 回退 |
| northhing-core default feature `[]` | 已修复 | 保持修复 | ✅ 是 |
| release-fast profile | 已修复 | 保持修复 | ✅ 是 |

---

### 2. 边界泄露验证

**CLI是否仍直接引用core内部模块？** — **是，大量引用仍然存在。**

通过对 `src/apps/cli/src/` 全目录搜索，边界泄露数量如下：

| 泄露路径 | 引用次数 |
|----------|----------|
| `northhing_core::agentic::core/...` | 25 |
| `northhing_core::agentic::coordination::...` | 含于上述25 |
| `northhing_core::agentic::events::...` | 含于上述25 |
| `northhing_core::agentic::tools::...` | 含于上述25 |
| `northhing_core::infrastructure::...` | 9 |
| `northhing_core::service::config::...` | 63 |
| **合计** | **~97** |

**涉及文件与上次完全一致**（无减少）：
- `acp_cli.rs` — `service::config` 直接调用（6处）
- `chat_state.rs` — `agentic::core::message` 直接引用（2处）
- `main.rs` — `infrastructure::ai` + `service::config`（6处）
- `agentic_system.rs` — `infrastructure::ai` + `service::config`（2处）
- `agent/core_adapter.rs` — `agentic::coordination`、`agentic::core`、`agentic::events`、`agentic::tools`（4处）
- `management.rs` — `infrastructure` + `service::config`（5处）
- `root_handlers.rs` — `agentic::core::message`、`agentic::coordination`（6处）
- `modes/exec.rs` — `infrastructure`（1处）
- `modes/chat.rs` — `agentic::tools` + `service::config` + `infrastructure`（15处+）
- `ui/startup.rs` — `agentic::coordination` + `agentic::tools`（2处）

**是否已引入 runtime-ports 隔离层？** — **否。** 
在 `src/apps/cli/src/` 下搜索 `runtime_ports`、`runtime-ports`、`northhing_runtime_ports` 均 **零匹配**。`northhing-runtime-ports` crate 虽存在于 workspace 中，但 CLI 代码未通过该层进行任何间接访问。

> 与上一轮相比，边界泄露在数量和文件分布上 **无变化**，属于未动工状态。

---

### 3. installer依赖统一验证

**文件**: `northing-installer/src-tauri/Cargo.toml`（注意：实际路径为 `northing-installer`，workspace 中拼写为 `northhing-installer`）

**是否已使用 `workspace = true`？** — **否。** 
全局搜索 `workspace = true` 在该文件中 **零匹配**。所有依赖仍使用硬编码版本。

**版本— 突是否已解决？** — **否。** 典型— 突如下：

| 依赖 | installer 硬编码 | workspace 定义 | — 突级别 |
|------|------------------|----------------|----------|
| `dirs` | `5.0` | `6.0` | MAJOR |
| `zip` | `0.6` | `4.6` | MAJOR |
| `reqwest` | `0.12` | `0.13.4` | MINOR（0.x 语义等效 MAJOR） |
| `tauri` | `2` | `2.11` | 版本落后 |
| `tauri-build` | `2` | `2.6` | 版本落后 |
| `tauri-plugin-dialog` | `2` | `2.7` | 版本落后 |
| `tokio` | `1` | `1.52` | 版本落后 |
| `eventsource-stream` | `0.2` | `0.2.3` | 版本落后 |

> 说明：installer 作为独立 Tauri 应用，如果其 Cargo.toml 未被 workspace 识别（拼写错误导致实际未被 exclude 正确排除或纳入），版本漂移将持续导致重复编译和符号— 突。

**exclude 拼写是否修正？** — **否。** 
workspace 根 `Cargo.toml` 第34行：
```toml
exclude = [
 "northhing-installer/src-tauri", # 仍为 northhing（多一个 h），实际目录为 northing-installer
]
```

---

### 4. 编译配置验证

**sccache 配置？** — **无。** 
全局搜索 `.cargo/config.toml` 和全仓库 `sccache` 均 **零匹配**。未配置缓存加速。

**split-debuginfo？** — **无。** 
workspace 根 `Cargo.toml` 和 `.cargo/config.toml` 中均无 `split-debuginfo` 字段。Debug 构建产物未拆分，直接贡献 target/debug 体积膨胀。

**target-dir？** — **未重定向。** 
workspace 根 `Cargo.toml` 中无 `target-dir` 配置；`.cargo/config.toml` 中无 `[build]` 节的 `target-dir` 键。编译产物仍默认落在 `northing/target/debug`，当前已达 **27GB**。

**dev profile opt-level？** — **未配置。** 
workspace 根 `Cargo.toml`：
```toml
[profile.dev]
incremental = true
# opt-level 未声明，默认为 0
```
上一轮提到的 `dev profile opt-level` 仍未设置。

**installer 独立 profile 重复？** — **仍然存在。** 
`northing-installer/src-tauri/Cargo.toml` 中定义：
```toml
[profile.release]
opt-level = "z" # ← 与 workspace opt-level = 3 不一致
lto = true
codegen-units = 1
strip = true

[profile.release-fast]
inherits = "release"
lto = false
codegen-units = 16
strip = false
incremental = true
```
与 workspace 根 `Cargo.toml` 的 `release` 相比：
- workspace: `opt-level = 3`
- installer: `opt-level = "z"`（最小体积，与 workspace 优化方向不一致）

`release-fast` 的命名和结构虽与 workspace 同名，但由于 installer 的 `release` 基准是 `"z"`，实际产物与 workspace 的 `release-fast` 不完全一致。

---

### 5. 已修复项回退检查

| 已修复项 | 上次状态 | 当前状态 | 回退？ |
|----------|----------|----------|--------|
| `target/debug` 大小 | 970MB | **27GB** | ⚠️ **严重回退** |
| `northhing-core` default feature `[]` | `default = []` | `default = []` | ✅ 未回退 |
| `release-fast` profile | workspace + installer 均有 | workspace + installer 均有 | ✅ 未回退 |

**target/debug 回退说明**：
- 从 970MB 膨胀至 **27GB**，虽未达到历史峰值 180GB，但已膨胀约 **27×**。
- 可能原因：未配置 `split-debuginfo`、未清理旧产物、未重定向 `target-dir` 到独立磁盘/分区。

---

### 6. 综合评分（更新）

| 维度 | 得分 | 说明 |
|------|------|------|
| 边界隔离 | 0/10 | 零进展，~97 处直接引用全部保留 |
| 依赖统一 | 1/10 | 仅 `northhing-core` default feature 保持正确；installer 版本— 突与拼写错误均未修复 |
| 编译缓存/产物管理 | 2/10 | sccache、split-debuginfo、target-dir 均未配置；target/debug 回退至 27GB |
| Profile 一致性 | 3/10 | workspace 的 release-fast 存在；installer 的 profile 与 workspace 不一致 |
| 已修复项稳定性 | 6/10 | core default feature 和 release-fast 未回退；但 target/debug 已严重回退 |
| **总分** | **2.4/10** | **较上轮无实质提升，target/debug 体积反而显著恶化。** |

---

### 7. 行动建议（优先级排序）

1. **P0 — 紧急清理 target/debug**：`cargo clean` 或删除 `target/debug` 以释放 27GB 磁盘；随后配置 `split-debuginfo = "packed"` 防止再次膨胀。
2. **P0 — 修正 workspace exclude 拼写**：将 `northhing-installer` 改为 `northing-installer`，确保 workspace 正确识别该 crate。
3. **P1 — 统一 installer 依赖**：将 `northing-installer/src-tauri/Cargo.toml` 中所有依赖改为 `workspace = true`，消除版本— 突。
4. **P1 — 统一 installer profile**：删除 installer 的独立 `[profile.release]` / `[profile.release-fast]`，继承 workspace 定义；若必须保留 `"z"`，则命名改为 `profile.release-small` 避免语义— 突。
5. **P2 — 引入 runtime-ports 隔离层**：将 CLI 中 97 处 `northhing_core::agentic::...` / `infrastructure::...` / `service::config::...` 引用逐步迁移到 `northhing-runtime-ports` 或新增 facade crate。
6. **P2 — 配置 sccache**：在 `.cargo/config.toml` 中添加 `[build] rustc-wrapper = "sccache"`。
7. **P2 — 重定向 target-dir**：在 `.cargo/config.toml` 中设置 `target-dir = "<独立路径>"` 以隔离编译产物。

