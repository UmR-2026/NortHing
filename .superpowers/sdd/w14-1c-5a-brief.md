# W14-1c-5a Brief — CI 双轨落到 windows（切片 5 前置解锁）

> 来源：`w14-1b-arbitration.md` 步骤 11 + §5#4；切片 5 阻塞处置。BASE：`c603688`。
> 决策记录：用户超时未响应（decide_pick_one 600s timeout），按仓内 W2 先例执行推荐项 A（serial 移 windows），用户可推翻。
> 背景：ubuntu/macos `cargo check --workspace` 自 2026-07-17 起 CI 慢性全红（编排者逐 run 核实），windows 一直绿。rust-tests-serial 首轮（655e96a）死在 ubuntu 编译墙（exit 101，同 commit ubuntu build check 死在 Check compilation 步）。仲裁 §5#4 只钉「双 job 连续 5 轮全绿」，未钉 OS。

## Spec

- S1：`rust-tests-serial` job（ci.yml:99-152）`runs-on` 改 `windows-latest`；删除该 job 的 Linux 系统依赖安装步骤（:105-133，Tauri apt 块）；保留 checkout / rust toolchain / rust-cache / i18n locale contract 生成 / `cargo test --locked --workspace -- --test-threads=1` 步骤。i18n 生成步骤内 `test ! -d northhing-Installer && test -f ...` 的 bash 断言语句在 windows runner 默认 shell 下可跑（GitHub windows runner 有 bash），但需实查该步骤 `shell: bash` 是否已显式声明——保持现状不删。
- S2：`rust-build-check` job 内 `Run workspace Rust tests` 步骤（:94-96）的 `if: matrix.os == 'ubuntu-latest'` 改为 `if: matrix.os == 'windows-latest'`（并行轨从 ubuntu 挪到唯一编译绿的 windows）。ubuntu/macos 仍跑 cargo check（红的预存账照原样，不修不在本单范围）。
- S3：job 名 `Rust Tests Serial (ubuntu-latest)` 同步改名 `(windows-latest)`。
- S4：YAML 语法验证（npx yaml-lint 或 python -c yaml 任一，报告写明方式）。

## Constraints

C1 只改 `.github/workflows/ci.yml`。C2 git 只点名 add 该文件。C3 不尝试修 ubuntu/macos 编译红（独立账，出本单范围）。C4 不改任何测试命令本体（`--locked`、`-–test-threads=1` 保持原样）。C5 以磁盘实际文件为准，偏差记 report。

## 验证

1. YAML 解析验证（方式自选，写进 report）
2. `git diff --check` 无 whitespace error
3. diff 自审：除 S1/S2/S3 三处外零改动

## 报告

写 `.superpowers/sdd/w14-1c-5a-report.md`：diff 摘要 / 验证输出 / 状态词。完成后自行 commit（message 含 W14-1c-5a）。
