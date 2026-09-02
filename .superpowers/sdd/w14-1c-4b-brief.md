# W14-1c-4b Brief — C/D 类锁纪律全仓扫描（仲裁步骤 8）

> 来源：`.superpowers/sdd/w14-1b-arbitration.md` 步骤 8（§4 表 #8）+ §5 附带条件。BASE：`b7675d1`。
> 本单是 W14-1c 切片 4 的下半，与另一路 init gate 重写并行（文件集不相交，见 C1）。

## 目标（一句话）

全仓扫描测试代码中的锁纪律：**每一个改环境变量（`std::env::set_var` / `remove_var`）或改 CWD（`set_current_dir`）的测试入口，都必须持有对应的进程级守卫锁**（`ENV_LOCK` / `CWD_LOCK` / `REMOTE_SEARCH_TEST_LOCK` / `TEST_GLOBAL_CONFIG_MUTEX`）。产出 = grep 清单 + 违规点 patch。

## 背景（已磁盘核实的类别定义，来自 w14-1a 清单）

- **C 类**（依赖同步原语，24 条）：`TEST_GLOBAL_CONFIG_MUTEX`（8）、`CWD_LOCK`（11）、`REMOTE_SEARCH_TEST_LOCK`（3）、`ENV_LOCK`（2）。
- **D 类**（改环境变量，4 条）：`northing-installer/src-tauri`（2）、`core/path_manager.rs`（2）。
- W14-1c-1/2/3 已处理过其中一部分（`INIT_GUARD`、`CWD_LOCK` 归位等），本单是**全仓兜底扫描**，不是重复已完成的切片。

## Spec

- S1 **清单**：用 ripgrep 全仓扫描（`src/`、`northing-installer/`、`tests/`），列出：
  1. 所有测试代码（`#[cfg(test)]` 模块、`tests/*.rs`）中调用 `std::env::set_var` / `remove_var` / `set_current_dir` 的位点；
  2. 每个位点所在测试是否持有对应守卫锁（在同一测试函数或其 helper 内获取 `ENV_LOCK`/`CWD_LOCK` 等，锁的获取必须先于状态修改、守卫存活到断言结束）。
  清单写进 report，格式：文件:行 | 测试名 | 改什么 | 持什么锁 | 判定（合规/违规）。
- S2 **patch**：违规点全部就地补锁（沿用该文件/该 crate 已有的守卫锁形态；若该 crate 无现成守卫锁，新增一个 `static ENV_LOCK: std::sync::Mutex<()>` 级别的最小守卫并加 file-header 注释）。若违规点 >10 个，patch 前 10 个并在 report 列出剩余为 follow-up。
- S3 不删测试、不改断言语义；只加锁/加守卫。
- S4 顺手清配额范围内：发现 mojibake 注释可就地修复。

## Constraints

C1 **禁碰文件**：`src/crates/assembly/core/src/kernel_facade/tests.rs` 和 `src/crates/assembly/core/src/kernel_facade/lifecycle.rs` 归并行的 W14-1c-4a 所有——扫描发现这两文件内有违规也只在 report 里列出，不 patch。
C2 并行波警示：同工作树有另一路 coder 在动 `kernel_facade/`。你的 diff 不感知他们；commit 时若发现别人 commit 进来，正常 commit 自己的，不 rebase 别人的。git add 只点名你改过的文件。
C3 以磁盘实际代码为准，brief 与磁盘冲突以磁盘为准并记 report。
C4 日志英文无 emoji。
C5 不引入新依赖；不加 `pub` 可见性放宽。

## 验证（report 必须含命令+输出摘录）

1. 扫描命令本身（rg 命令行 + 命中计数）
2. `rustup run stable-x86_64-pc-windows-msvc cargo check --workspace`（`cmd /c` 重定向输出；参考 skill `long-running-shell`）
3. 对每个被 patch 的 crate：`rustup run stable-x86_64-pc-windows-msvc cargo test -p <crate> --lib` 全绿
4. `git diff --check` 无 whitespace error
5. rot 闸自查：不得新增 `let _ =` 位点（371/388 基线不许涨）

## 报告

写 `.superpowers/sdd/w14-1c-4b-report.md`：全量清单表 / patch 清单 / 验证输出 / 状态词。完成后自行 commit，message 含 `W14-1c-4b`。
