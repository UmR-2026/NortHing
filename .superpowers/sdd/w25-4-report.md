# W25-4 任务报告 — desktop 顺手清批（raw-window-handle 死依赖 + {win} 风格，单 judge）

## 改动摘要

- **AC1 / S1 (删除 raw-window-handle 死直持依赖)**:
  - 在 `src/apps/desktop/Cargo.toml` 删除了 `[dependencies.raw-window-handle]` 块（两行：表头与 `version = "0.6"`）。
  - `Cargo.lock` 由 cargo 自然更新，移除了 `northhing` 对 `raw-window-handle 0.6.2` 的直接依赖项。
- **AC2 / S1 (统一 work.rs 导入风格)**:
  - 在 `src/apps/desktop/src/ui_dioxus/windows/work.rs` 将 `use crate::ui_dioxus::windows::{win};` 改为 `use crate::ui_dioxus::windows::win;`，与兄弟模块 `facility.rs` 与 `self_app.rs` 保持完全一致的单元素导入风格。
- **最小 diff 与约束遵守 (S1/S2/Global Constraints)**:
  - 全量改动仅 2 个代码文件加 1 个 lockfile，精准改动 3 处，未多触碰任何无关行。
  - 严守禁区：未修改 `scripts/` 下任何文件、未修改 `ci.yml`、`workflow-policy.json`、`gate-registry.json`，亦未改动任何其它 desktop 源文件。
  - 未触碰 W25-2 同波并行文件（`verify-rot-budget.mjs` / `workflow-policy.json` 等）。

## 复用侦察

### 1. rg 零引用证据
在修改前执行全树搜索，确认 `raw-window-handle` 在 `src/apps/desktop/` 下无任何代码引用：
```bash
rg "raw_window_handle" src/apps/desktop/
```
输出：
```
(0 matches)
```

### 2. cargo tree 依赖图分析
在 BASE (`e57c6387de88110789ae600d90a4853da05aa90a`) 下执行依赖反查：
```bash
rustup run stable-x86_64-pc-windows-msvc cargo tree -i raw-window-handle@0.6.2
```
BASE 输出：
```
raw-window-handle v0.6.2
├── northhing v0.2.10 (<RUSTUP>/src/apps/desktop)
├── rfd v0.17.2
│   └── dioxus-desktop v0.8.0-alpha.1
│       └── dioxus v0.8.0-alpha.1
│           └── northhing v0.2.10 (<RUSTUP>/src/apps/desktop)
├── tao v0.35.3
│   └── dioxus-desktop v0.8.0-alpha.1 (*)
└── wry v0.55.1
    └── dioxus-desktop v0.8.0-alpha.1 (*)
```
删除 `src/apps/desktop/Cargo.toml` 中的死直持依赖后执行相同反查：
```
raw-window-handle v0.6.2
├── rfd v0.17.2
│   └── dioxus-desktop v0.8.0-alpha.1
│       └── dioxus v0.8.0-alpha.1
│           └── northhing v0.2.10 (<RUSTUP>/src/apps/desktop)
├── tao v0.35.3
│   └── dioxus-desktop v0.8.0-alpha.1 (*)
└── wry v0.55.1
    └── dioxus-desktop v0.8.0-alpha.1 (*)
```
证据表明：`northhing` 直持边已彻底消失，剩余仅为 `dioxus-desktop` 下属 `rfd`/`tao`/`wry` 的合法传递依赖。

## 验证（命令 + 输出原文）

BASE: `e57c6387de88110789ae600d90a4853da05aa90a`

### 1. cargo check -p northhing
```bash
rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing
```
输出（尾部摘要）：
```
warning: `northhing-core` (lib) generated 16 warnings (run `cargo fix --lib -p northhing-core` to apply 15 suggestions)
    Checking northhing v0.2.10 (<RUSTUP>/src/apps/desktop)
warning: `northhing` (lib) generated 2 warnings (run `cargo fix --lib -p northhing` to apply 2 suggestions)
warning: `northhing` (bin "northhing") generated 60 warnings (2 duplicates) (run `cargo fix --bin "northhing" -p northhing` to apply 9 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 19.52s
```
结果：0 error，60 warnings（与 BASE 严格持平）。

### 2. cargo check --workspace
```bash
rustup run stable-x86_64-pc-windows-msvc cargo check --workspace
```
输出（尾部摘要）：
```
warning: `northhing-core` (lib) generated 16 warnings (run `cargo fix --lib -p northhing-core` to apply 15 suggestions)
    Checking northhing v0.2.10 (<RUSTUP>/src/apps/desktop)
warning: `northhing` (lib) generated 2 warnings (run `cargo fix --lib -p northhing` to apply 2 suggestions)
warning: `northhing` (bin "northhing") generated 60 warnings (2 duplicates) (run `cargo fix --bin "northhing" -p northhing` to apply 9 suggestions)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 18.75s
```
结果：0 error，全 workspace 编译通过。

### 3. cargo tree -i raw-window-handle@0.6.2
```bash
rustup run stable-x86_64-pc-windows-msvc cargo tree -i raw-window-handle@0.6.2
```
输出：
```
raw-window-handle v0.6.2
├── rfd v0.17.2
│   └── dioxus-desktop v0.8.0-alpha.1
│       └── dioxus v0.8.0-alpha.1
│           └── northhing v0.2.10 (<RUSTUP>/src/apps/desktop)
├── tao v0.35.3
│   └── dioxus-desktop v0.8.0-alpha.1 (*)
└── wry v0.55.1
    └── dioxus-desktop v0.8.0-alpha.1 (*)
```
结果：输出中不再包含 `northhing` 直接依赖边。

### 4. 仓库卫生检查
```bash
node scripts/check-repo-hygiene.mjs
```
输出：
```
Repository hygiene check passed (7 content files scanned, 3940 filenames checked).
```
退出码：0

### 5. Task Gate 验证 (verify-attempt)
- 报告初版提交 `b5b49aa` 校验：
```bash
node scripts/verify-task-gate.mjs verify-attempt --base e57c6387de88110789ae600d90a4853da05aa90a --tip b5b49aa --allowlist .superpowers/sdd/w25-4-allowlist.txt
```
输出：
```
Attempt verification passed: all modified files are within allowlist.
```
退出码：0

- 记录 Tip 校验的最终提交 `0f5a18b` 校验：
```bash
node scripts/verify-task-gate.mjs verify-attempt --base e57c6387de88110789ae600d90a4853da05aa90a --tip 0f5a18b --allowlist .superpowers/sdd/w25-4-allowlist.txt
```
输出：
```
Attempt verification passed: all modified files are within allowlist.
```
退出码：0

## 疑虑

无。

## 状态

DONE
