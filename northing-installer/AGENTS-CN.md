[English](AGENTS.md) | **中文**

# AGENTS.md

## 范围

本文件适用于 `northhing-Installer`。仓库级规则请使用顶层 `AGENTS.md`。

## 此处重点

`northhing-Installer` 是一个独立的 Tauri + React 应用，不属于主 Cargo 工作区。

模块 README 中重点关注区域：

- `src-tauri/src/installer/commands.rs`：Tauri IPC 与卸载执行
- `src-tauri/src/installer/registry.rs`：Windows 注册表集成
- `src-tauri/src/installer/shortcut.rs`：快捷方式创建
- `src-tauri/src/installer/extract.rs`：压缩包解压
- `src/hooks/useInstaller.ts`：前端的安装器状态流转
- `src/i18n/`：仅供安装器使用的字符串；区域元数据由 `src/shared/i18n/contract/locales.json` 生成

安装流程：

```text
Language Select → Options → Progress → Model Setup → Theme Setup
```

## 命令

这里是命令参考，不是默认的预检列表。PR 范围请使用下方“验证”章节。

```bash
pnpm --dir northhing-Installer run installer:dev
pnpm --dir northhing-Installer run tauri:dev
pnpm --dir northhing-Installer run type-check
pnpm --dir northhing-Installer run build            # React 构建 / CI 复现
pnpm --dir northhing-Installer run installer:build  # 仅打包
```

## 验证

使用匹配的最小检查：

```bash
pnpm run i18n:audit                                                   # 仅资源类 i18n
pnpm run i18n:generate && pnpm run i18n:contract:test && pnpm run i18n:audit
pnpm --dir northhing-Installer run type-check                            # 前端 i18n / 运行时
cargo check --manifest-path northhing-Installer/src-tauri/Cargo.toml      # Tauri / Rust 变更
```

仅当涉及打包、载荷、原生打包、安装/卸载流程、注册表、快捷方式或解压的变更时，才运行完整的安装器构建：

```bash
pnpm --dir northhing-Installer run type-check && pnpm --dir northhing-Installer run installer:build
```

如果你修改了卸载流程，也请验证 `northhing-Installer/README.md` 中描述的卸载模式入口点。
