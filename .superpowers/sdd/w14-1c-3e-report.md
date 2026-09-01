# W14-1c-3e 实施报告 — CI 双轨（新增串行测试 job）

## 1. 变更清单

- `.github/workflows/ci.yml`：
  - 新增独立 job `rust-tests-serial`（`runs-on: ubuntu-latest`）；
  - 步骤包含：`actions/checkout@v4`、Linux 依赖安装（Tauri）、`dtolnay/rust-toolchain@stable`、`swatinem/rust-cache@v2`（复用现有 cache-key 策略）、i18n 合约生成（`node scripts/generate-i18n-contract.mjs`）、串行测试执行 `cargo test --locked --workspace -- --test-threads=1`；
  - 现有 matrix job 与其它 jobs 保持原样不动。

## 2. 验证证据

### 2.1 YAML 语法校验
命令：
```pwsh
python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml', 'r', encoding='utf-8')); print('YAML parsing SUCCESS')"
```
输出：
```
YAML parsing SUCCESS
```

### 2.2 Git Diff 格式校验
命令：
```pwsh
git diff --check .github/workflows/ci.yml
```
输出：
```
(无 whitespace 错误输出，exit code 0)
```

### 2.3 仓库清洁度门禁
命令：
```pwsh
pnpm run check:repo-hygiene
```
输出：
```
Repository hygiene check passed (10 content files scanned, 3728 filenames checked).
```

## 3. 遗留与 Caveat

- 处于五路并行波中，本次 commit 仅点名 stage `.github/workflows/ci.yml` 与 `.superpowers/sdd/w14-1c-3e-report.md`，未触碰其它 coder 正在修改的 Rust 源码与其它 brief。
- CI 真实多轮运行验证属后续观测项（由编排者负责）。

DONE
