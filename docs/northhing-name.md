# NortHing / 纳森 产品名称约定

> **生效日期**: 2026-06-25（v0.1.0 后续大改名）
> **取代文档**: `docs/northing-name.md`（前次改名产物，本次又改名）
> **更前身**: `docs/agent-app-name.md`（v0.1.0 阶段）
> **前身**: `northing` / `Northing`（v0.1.0 后第一版改名产物）
> **适用范围**: 所有代码、文档、配置、CLI 输出、日志文件名

## 选定名称

- **产品名（中/英）**: `NortHing` / `纳森`
- **CLI 二进制名**: `northhing-cli`
- **桌面二进制名**: `northhing`（Slint 壳）
- **Server 二进制名**: `northhing-server`
- **Relay 二进制名**: `northhing-relay-server`
- **Internal CLI**: `northhing-internal`（独立 capability-gated CLI）
- **仓库路径**: `E:\agent-project\northhing`（A0.x 重命名）
- **Cargo workspace 名**: `northhing`
- **Crate 名前缀**: `northhing-*`（共 27 个 crate）
- **日志文件名前缀**: `northhing*.log`
- **内部命名空间**（snake_case）: `northhing`
- **用户配置目录**: `~/.config/northhing/` 或 `%APPDATA%\northhing\`
- **Sandbox 目录**: `.northhing/`
- **环境变量前缀**: `NORTHHING_*`
- **CLI 启动横幅**: `NortHing vX.Y.Z`
- **CLI 命令名**: `northhing`
- **Tauri bundle id**: `com.northhing.installer`
- **GitHub repo**: `UmR-2026/northhing`

## 替换规则

| 旧名称 | 新名称 | 说明 |
|--------|--------|------|
| `northhing` | `northhing` | 产品名、用户可见字符串、kebab-case 通用 |
| `NortHing` | `NortHing` | prose 中的英文产品名 |
| `northhing's` | `northhing's` | 所有格 |
| `northhing` | `northhing` | snake_case Rust crate import / 内部命名空间 |
| `NortHing` | `NortHing` | PascalCase Rust 类型名 |
| `NORTHHING_*` | `NORTHHING_*` | 全部大写环境变量 |
| `northhing-*` | `northhing-*` | 27 个 crate 名前缀 |
| `northhing_*` | `northhing_*` | Rust snake_case crate 名（极少见） |
| `opennorthhing` | `opennorthhing` | model provider id（用户决定重命名） |
| `northhing-Installer/` | `northhing-installer/` | installer 目录名 |
| `northhing-Installer/src-tauri` | `northhing-installer/src-tauri` | Cargo workspace exclude 路径 |
| `northhing://runtime/` | `northhing://runtime/` | tool runtime URI scheme |
| `northhing:embedded` 等 | `northhing:embedded` 等 | WebDriver capability 名 |
| `NORTHHING_WEBDRIVER_*` | `NORTHHING_WEBDRIVER_*` | webdriver 相关 env var |
| `--northhing-*` | `--northhing-*` | CSS custom properties |
| `com.northhing.installer` | `com.northhing.installer` | Tauri bundle identifier |

## 保留旧名称的地方

以下情况**不替换**，保留原始名称 + 加 LEGACY 注释：

- `docs/superpowers/plans/*.md` 中的历史 `northhing` 字样（用户决策：保留作历史参考）
- `LICENSE` 文件中的第三方版权声明
- 上游 fork 关系的 git remote URL（已删 commit history）

## 决策记录

详见 `docs/reviews/2026-06-25-rename-northhing.md`。

## 查询方法

```bash
# 查找所有残留的 "northhing"（应在 rename 后仅命中 docs/superpowers/plans/）
git grep -in "northhing" -- ':!docs/superpowers/plans/*'

# 查找所有 "northhing"（应为 0 命中）
git grep -in "northhing"

# 查找所有 "NORTHHING_"（应为 0 命中）
git grep -in "NORTHHING_"

# 查找所有 "northhing-"（除历史 plans 外应为 0 命中）
git grep -in "northhing-"

# 验证 Cargo workspace 名
grep "^\[workspace.metadata\]" -A 2 Cargo.toml | grep "name = "
```

## 注释规范

在代码中引用产品名时，使用统一注释格式：

```rust
// northhing: <描述>
// 例如：
// northhing: CLI entry point for the desktop shell
```

对于历史代码中保留的 `northhing` / `BitFun` 引用，添加注释说明：

```rust
// LEGACY(northhing): 保留原始名称，迁移期兼容性
// LEGACY(BitFun): v0.1.0 之前前身
```
