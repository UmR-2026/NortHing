# W14-1c-5a Report — CI 双轨落到 windows-latest

## 变更摘要 (Diff Summary)

- **文件**: `.github/workflows/ci.yml`
  - **S1**: `rust-tests-serial` job `runs-on` 修改为 `windows-latest`；删除了 Linux 依赖安装步骤（Tauri apt 块）；保留了 checkout、rust-toolchain、rust-cache、i18n locale contract 生成（带 `shell: bash`）以及 serial 测试命令。
  - **S2**: `rust-build-check` job 中的 `Run workspace Rust tests` 步骤触发条件由 `if: matrix.os == 'ubuntu-latest'` 修改为 `if: matrix.os == 'windows-latest'`。
  - **S3**: `rust-tests-serial` job 名称由 `Rust Tests Serial (ubuntu-latest)` 修改为 `Rust Tests Serial (windows-latest)`。

## 验证结果 (Verification Evidence)

1. **YAML 语法验证**:
   - 命令: `python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml', encoding='utf-8')); print('YAML validation PASS')"`
   - 输出: `YAML validation PASS`
2. **Whitespace / Diff Check**:
   - 命令: `git diff --check`
   - 输出: 无输出（无任何 whitespace error）
3. **Diff 自审**:
   - 仅 `.github/workflows/ci.yml` 发生变更，精准覆盖 S1 / S2 / S3，无多余改动。
4. **Repo Hygiene Check**:
   - 命令: `pnpm run check:repo-hygiene`
   - 输出: `Repository hygiene check passed (2 content files scanned, 3740 filenames checked).`

## 状态

DONE

## Fix 轮 1

### 修复内容 (Fix Summary)
- 在 `rust-tests-serial` job 中，在 `actions/checkout@v4` 之后与 `dtolnay/rust-toolchain@stable` 之前，补充 `Setup OpenSSL (Windows, prebuilt)` 步骤（调用 `./scripts/ci/setup-openssl-windows.ps1`，`shell: pwsh`，`if: runner.os == 'Windows'`），形态与 `rust-build-check` 保持严格一致，确保 windows-latest runner 上 serial 测试链接所需 OpenSSL 环境变量与二进制就绪。

### 验证证据 (Verification Evidence)
1. **YAML 语法验证**:
   - 命令: `python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml', encoding='utf-8')); print('YAML validation PASS')"`
   - 输出: `YAML validation PASS`
2. **Whitespace / Diff Check**:
   - 命令: `git diff --check`
   - 输出: 无输出（无 whitespace 错误）
3. **Diff 自审**:
   - 仅在 `rust-tests-serial` 的 checkout 与 rust-toolchain 间插入 5 行 OpenSSL setup 步骤，无其它任何改动。

### 状态
DONE

