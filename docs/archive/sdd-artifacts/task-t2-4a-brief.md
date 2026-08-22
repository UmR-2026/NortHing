# Task T2-4a Brief — P2-16: `ConfigManager::save_config` 原子写

> 需求唯一来源。债项定义：`docs/status/tech-debt-ledger.md` P2-16（roadmap:184 T2-4 上半）。

## 背景

`save_config` 用裸 `fs::write` 整写全局配置 `app.json`，写入中断会留下截断文件。
债项处方：改走 `json_store::write_atomic` 模式（temp + rename），与 settings/vault 写路径一致。

## 现状（侦察事实，编排者亲验）

- 目标函数：`src/crates/assembly/core/src/service/config/mgr_load.rs:146-162`
  `pub(crate) async fn save_config(&self) -> NortHingResult<()>` —— `serde_json::to_string_pretty(&self.config)` →
  手动建父目录（`if !parent.exists()` 守卫）→ `fs::write(&self.config_file, content)`。
- 现成实现：`northhing_services_core::JsonFileStore`（unit struct），
  `src/crates/services/services-core/src/json_store.rs:136`
  `pub async fn write_atomic<T: Serialize>(&self, path: &Path, value: &T) -> Result<(), JsonFileStoreError>`。
  内部已含：父目录 `create_dir_all`、`to_string_pretty` 序列化、per-path 写锁、tmp+rename、Windows 重试。
  参考用法：`services-integrations/src/remote_ssh/password_vault.rs:10,120-124`（`use northhing_services_core::JsonFileStore`）。
- core 已依赖 services-core：`src/crates/assembly/core/Cargo.toml:97`（非 optional，直接用，勿动 Cargo.toml）。
- 相关测试：`mgr_load.rs` 与 `mgr_validate.rs` 内引用 save_config 的现有测试。

## 要求的改动

1. `save_config` 改为：序列化+父目录创建全部委托给 `JsonFileStore.write_atomic(&self.config_file, &self.config)`，
   错误经 `JsonFileStoreError` 映射到 `NortHingError::config(...)`（保持现有错误文案风格：
   `format!("Failed to write config file {:?}: {}", ...)` 或语义等价文案）。
   删除被委托接管的 `to_string_pretty` 与父目录守卫代码（write_atomic 已含，重复即冗余）。
2. **行为约束**：成功路径产物字节级等价（write_atomic 同为 to_string_pretty）；`create_backup` 不动；
   不新增对 Cargo.toml / 其它文件的改动；不引入备份逻辑（save_config 原本就没有）。
3. 新增 1 个测试（放在 mgr_load.rs 现有测试块）：save_config 后配置内容 round-trip 一致且
   配置目录无残留 tmp 文件（tmp 命名规则见 json_store.rs `build_temp_json_path`）。
   若现有测试已覆盖 round-trip，则新测试只断言无 tmp 残留，避免重复覆盖。

## 文档同步（家规 2，同一 commit）

- `docs/status/tech-debt-ledger.md` P2-16 条目：`**Status**: active` → `**Status**: resolved (2026-08-20, T2-4a)`
  （日期用实际 commit 日期）。

## 验证（最小集，MSVC wrapper）

cargo 一律：`& "$env:USERPROFILE\.cargo\bin\rustup.exe" run stable-msvc cargo ...`

1. `cargo check --workspace`（core 属共享 Rust，AGENTS.md 验证表）
2. focused：`cargo test -p northhing-core --features product-full --lib config`（或实际能命中 mgr_load/mgr_validate 测试的最小 filter，report 里说明）
3. `git diff --check`

## 纪律

- 日志/注释 English-only；顺手清配额仅限本函数紧邻的明显陈旧注释。
- 禁止顺手重构 ConfigManager 其它写路径（create_backup 等不在本任务）。
- 改动触及文件预计仅 mgr_load.rs + tech-debt-ledger.md；若发现必须动第三个文件，STOP 并 NEEDS_CONTEXT。

## Rust 工作约定（涉 Rust 任务必须遵守）

1. 仓库根 AGENTS.md / 就近 AGENTS.md 是规范唯一事实源（六层分层、骨干不变量、i18n、日志、平台边界），优先于任何通用 Rust 惯例；Cargo.toml 的 edition/lints 维持现状，不许套模板。
2. 遇编译错误（E0xxx）先用 skill 工具加载对应 skill（m01-ownership / m03-mutability / m04-zero-cost / m06-error-handling / m07-concurrency / unsafe-checker），trace 到设计层原因（谁该拥有这份数据？为什么跨线程？）再改代码——禁止无脑 .clone() / .unwrap() / Arc 包一切糊住编译器。
3. 设计取舍（错误分层、生命周期、并发模型）可查 m09-m15 与 domain-* skill；完整路由见 rust-router skill。
4. report 里写明：遇到的每个编译错误最终修在哪一层（机制层/设计层），一行一个。
