# W25-4 Brief — desktop 顺手清批（raw-window-handle 死依赖 + {win} 风格，单 judge）

## 1. 来源与验收标准（逐字）

W24 终审 defer 批（progress.md 台账 W24 波总结行）+ W24 修复轮 judge Minor：
- (b) m3 judge（233fe89 审查）：`raw-window-handle 0.6` 在 `src/apps/desktop/Cargo.toml:62` 疑似死依赖——rg `use raw_window_handle` / `raw_window_handle::` 在 `src/apps/desktop/` 全树零命中；「先 `cargo tree -i` 取剩余唯一入口后再删」。
- (c) m3 judge（1db4745 审查）：work.rs `use crate::ui_dioxus::windows::{win};` 单元素大括号形态与兄弟文件不一致，一行改齐。

### 验收标准（机械可核对）

- AC1: `src/apps/desktop/Cargo.toml` 删 `[dependencies.raw-window-handle]` 整块（两行：表头 + version）；`Cargo.lock` 由 cargo 自然更新（desktop 不再直持该 dep）。
- AC2: `src/apps/desktop/src/ui_dioxus/windows/work.rs` 的 `use crate::ui_dioxus::windows::{win};` 改为 `use crate::ui_dioxus::windows::win;`（与 facility.rs/self_app.rs 同形）。
- AC3: `cargo check -p northhing` 与 `cargo check --workspace` 在 Windows 全绿（0 error；warnings 数与 BASE 持平 60）。
- AC4: `cargo tree -i raw-window-handle@0.6.2` 输出中不再含 `northhing` 直接边（tao/wry/rfd 传递边保留为预期）。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `e57c6387de88110789ae600d90a4853da05aa90a`。

| 事实 | 证据 |
|---|---|
| `raw-window-handle` 在 desktop src 零引用 | m3 judge 独立 rg（233fe89 审查报告）+ 编排者复核 |
| `cargo tree -i raw-window-handle@0.6.2`：直接边只有 `northhing`（desktop）一条；tao/wry/rfd 传递边另有三条（删除后 crate 仍在树中，属预期——本单只清死直持） | 编排者 BASE 实跑 |
| W24 删除 `windows` 死依赖（233fe89）同模式先例：删后 workspace check 绿 +  sentinel 解锁 | git 历史 |
| work.rs:20 现为 `use crate::ui_dioxus::windows::{win};`（W24 修复轮后带 cfg 门控行在其上） | 现读 |
| 基线：`cargo check --workspace` 绿（W24 修复轮实证，60 warnings 存量） | CI run 35593336987 + 本地 |
| 本单非 meta-ratchet（Cargo.toml/Cargo.lock/work.rs 均不在 metaRatchetPaths）→ 单 judge | workflow-policy.json:20-31 |

## 3. 复用侦察（强制）

- 本单就是复用侦察的执行（删死依赖），report 里贴 rg 零命中与 cargo tree 证据原文。

## 4. Spec（必须全部满足）

- S1: 按 AC1/AC2 改，共 2 文件 + Cargo.lock。
- S2: 禁区：ci.yml、scripts/ 全部、workflow-policy.json、gate-registry.json、其它 desktop src 文件（除 work.rs 那一行）。
- S3: 若删 dep 后 `cargo check -p northhing` 红了（说明 rg 证据有漏网使用点）→ 停手 BLOCKED 上报，不得自行加回或改代码迁就。

## 5. Global Constraints（逐字遵守）

- 最小 diff：三个改动点，不多碰一行。
- 禁整树 git 操作；只点名 add/commit。
- 测试真实执行，report 贴输出原文（cargo check 尾部 + cargo tree 段 + hygiene）。
- 路径卫生（D-2）：report 里 cargo 输出含本机路径段一律 `<RUSTUP>`/`<HOME>` 占位。

## 6. 验证（命令 + 输出原文进 report）

BASE 全 sha = `e57c6387de88110789ae600d90a4853da05aa90a`（下文 `<BASE>`）。cargo 一律经 `rustup run stable-x86_64-pc-windows-msvc`（仓规：repo override 是 GNU）。

1. `cargo check -p northhing` → 0 error（warnings 60 持平）
2. `cargo check --workspace` → 0 error
3. `cargo tree -i raw-window-handle@0.6.2` → 输出无 `northhing` 直持边
4. `node scripts/check-repo-hygiene.mjs` → exit 0
5. `node scripts/verify-task-gate.mjs verify-attempt --base <BASE> --tip <本单最后 commit sha> --allowlist .superpowers/sdd/w25-4-allowlist.txt` → exit 0（自建 allowlist 含自身；**窗口内若出现 W25-2 文件（verify-rot-budget.mjs/workflow-policy.json 等），停手报编排者复跑，不得私扩 allowlist**）

## 7. 报告

路径 `.superpowers/sdd/w25-4-report.md`。章节：改动摘要 / 复用侦察 / 验证（命令+输出原文）/ 疑虑 / 状态（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）。

## 8. 派发元信息

- BASE: `e57c6387de88110789ae600d90a4853da05aa90a`
- 允许文件集（diff 越出即 judge Critical）：
  - `src/apps/desktop/Cargo.toml`
  - `Cargo.lock`
  - `src/apps/desktop/src/ui_dioxus/windows/work.rs`
  - `.superpowers/sdd/w25-4-brief.md` / `w25-4-report.md` / `w25-4-allowlist.txt`
- 禁区：§4-S2 所列 + 其它未列出文件。
- commit 规则：点名 git add；message 前缀 `chore(desktop): W25-4`；允许多 commit。
- 并行声明：W25-2 同波并行（scripts/ 侧），文件集不相交。注意同工作树 cargo 锁：若 cargo 命令撞锁等待属已知可容忍。
