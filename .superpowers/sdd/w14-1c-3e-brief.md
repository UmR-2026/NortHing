# W14-1c-3e Brief — CI 双轨（新增串行测试 job）

> 来源：`w14-1b-arbitration.md` 步骤 11 + §5#4。BASE：`5f242fd`。
> 本单只加 CI 配置；「连续 5 轮全绿」是后续观测项（编排者负责），不在本单。

## 预检结论（已磁盘核实）

- `.github/workflows/ci.yml`：job `Rust Build Check (${{ matrix.os }})`（:26）内 :94-96 有 `Run workspace Rust tests` = `cargo test --locked --workspace`。无串行 job。

## Spec

- S1：ci.yml 新增独立 job `rust-tests-serial`（`runs-on: ubuntu-latest`，最小化：checkout + rust toolchain + 复用现有缓存策略若文件中已有），步骤 = `cargo test --locked --workspace -- --test-threads=1`。
- S2：不动现有 matrix job 与其它 job；workflow YAML 语法合法（用 `npx yaml-lint` 或 python -c yaml 任一可用手段验证，报告写验证方式）。

## Constraints

C1 只改 `.github/workflows/ci.yml`。C2 git 只点名 add。C3 **并行波**：工作树有其它 coder 在动 Rust 代码，你的 diff 不感知他们；若 push/commit 时发现别人 commit 进来，正常往前放，不 rebase 别人的。C4 以实际文件为准，偏离记 report。

## 验证

1. YAML 解析验证（方式自选，写进 report）
2. `git diff --check`（无 whitespace error）

## 报告

`.superpowers/sdd/w14-1c-3e-report.md`：清单 / 验证输出 / 状态词。完成后自行 commit（message 含 W14-1c-3e）。
