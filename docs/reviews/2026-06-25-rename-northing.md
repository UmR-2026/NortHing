
---

## 附录：第二阶段改名 — `Northing` → `NortHing`

> **触发**: 用户后续指示产品名大小写从 `Northing` 改为 `NortHing`（驼峰大小写风格）
> **执行时间**: 2026-06-25（同日）
> **commit**: 紧随 `667a47e` 之后

### 变更范围

| 类别 | 旧 | 新 |
| --- | --- | --- |
| 产品名 | `Northing` | `NortHing` |
| lower-case | `northing` | `northhing` |
| UPPER_CASE | `NORTHING_*` | `NORTHHING_*` |
| 第三方 brand | `opennorthing` | `opennorthhing` |

### 实施

1. 新脚本 `scripts/rename-to-northhing.py` 实现 case-sensitive 替换（862 files, 8770 replacements）
2. theme preset 文件名 `northing-*.json` → `northhing-*.json`（`git mv`）
3. `ui.bundle.js` byte-level rename（minified 资产）
4. `docs/northing-name.md` → `docs/northhing-name.md`（`git mv`）

### 验证

```
cargo check -p northhing-core -p northhing-server → Finished in 2m 30s, 0 errors
cargo test --workspace --lib --exclude northhing --exclude northhing-webdriver --exclude terminal-core
 → 1516 passed; 0 failed
```

### 注意

- 用户在第一阶段 commit (`667a47e`) 后中断并提出大小写修改；commit `667a47e` 保留了 `northing` 大小写，需要二次 commit
- 第二阶段 commit SHA 待记录（在 `git commit` 后回填）
- 大小写修改是 cosmetic 决策，技术债 0（不影响外部接口）
embedded` 等 | `northhing:embedded` 等 | 同步改 |
| Skill slot key | `agent-app-system` | `northhing-system` | 同步改 |
| 第三方品牌 "openagent-app" | `openagent-app` | `opennorthhing` | user 决定"也改"（含外部— 突风险） |
| CSS variables (`--agent-app-*`) | `--agent-app-*` | `--northhing-*` | 同步改 |
| Tauri bundle id | `com.agent-app.installer` | `com.northhing.installer` | 同步改 |
| Installer 目录 | `agent-app-Installer/` | `northhing-installer/` | 同步改 |
| GitHub repo URL refs | `GCWing/agent-app` | `GCWing/northhing` | 占位改（实际 GitHub repo 未改，需后续 `gh repo rename`） |
| 历史 plan docs (`docs/superpowers/plans/*.md`) | 含 `agent-app` 字样 | **保留** | user 决定"保留历史计划" + 加 LEGACY 注释头 |

## 不在范围内

- 真实 GitHub repo 重命名（需要 `gh repo rename` + git remote 更新）— 待办
- 域名 `openagent-app.com` 重命名（DNS 外部）— 待办
- Homebrew tap `GCWing/homebrew-tap` 更新 — 待办
- Tauri 签名证书重生成 — 待办（bundle id 改了以后跟旧签名断开）
- v0.1.0 tag 移动 — 未动；下一个 release 是 v0.2.0-alpha

## 实施流程

### 1. 重命名脚本 (Python)

`scripts/rename-to-northhing.py` 实现 11 条替换规则（kebab / snake / PascalCase / SCREAMING_SNAKE / 各种变体）+ `scripts/legacy-prefix.py` 给 14 个历史 plan 加 `<!-- LEGACY: -->` 注释。

**经验教训**:
- 第一版脚本只检测 `.rs/.toml/.md/.json` 等"标准文本扩展名"，错过 `.slint/.py/Caddyfile/Dockerfile/.json/.cjs/.ts/.tsx` 等
- 第一版脚本用 `read_text(encoding='utf-8')`，遇到 UTF-8-with-bad-bytes 文件会 UnicodeDecodeError 静默跳过。改成 `read_bytes` + 字节级替换（ASCII 模式不受编码影响）
- 第一版脚本还会被自己的替换规则命中（脚本里有 `agent-app` 字样），变成 `northhing → northhing` 自映射。需要在脚本里保护 self-references

### 2. 目录重命名

- `agent-app-Installer/` → `northhing-installer/`（`git mv`，成功）
- `src/apps/cli/themes/presets/agent-app-*.json` → `northhing-*.json`（文件名含 product name 但内部为空，git mv）
- **仓库根目录重命名**：遇到 Windows "Device or resource busy" 错误（`target/debug/` 有大量 .exe/.pdb 仍被 rust-analyzer / Windows Defender 占用）。计划用以下策略：
 1. 先 commit 当前 922 文件修改
 2. 让用户手动 `mv agent-app northhing`（在没有 cargo / IDE 占用的状态下）
 3. 在新位置 `git reset HEAD` 同步 index，git 会基于内容相似度识别 rename

### 3. 验证

- ✅ `cargo check --workspace` 通过 (43.93s, 0 errors, 28 个 northhing crate)
- ✅ `cargo test --workspace --lib --exclude northhing --exclude northhing-webdriver --exclude terminal-core` → 1516 passed, 0 failed
- ✅ `grep -rln "agent-app"` 在工作区只剩 `docs/superpowers/plans/*.md`（14 个 LEGACY plan）

### 4. 剩余 `agent-app` 引用（设计内）

| 位置 | 状态 |
| --- | --- |
| `docs/superpowers/plans/*.md` (14 个) | 加 LEGACY 注释，保留原文 |
| GitHub repo URL refs (`GCWing/agent-app`) | 代码改了，但实际 GitHub repo 没改 — 待办 |
| Tauri 签名证书（旧 `com.agent-app.installer` 签名） | bundle id 改了之后断开 — 待办 |
| Homebrew tap `GCWing/homebrew-tap` | 引用旧 release event `agent-app-release-published` — 待办 |
| 域名 `openagent-app.com` | 已改为 `opennorthhing.com` 字样 — 待办 |

## 风险与回滚

### 风险

1. **wire-format 破坏性变更**：`agent-app://runtime/` 改 `northhing://runtime/` 意味着所有 session state / persisted file references 失效。v0.1.0 还没真实用户，所以 OK
2. **OpenAI 风格 model provider id 改名为 `opennorthhing`**：与现实供应商 brand — 突（如有），需评估
3. **CSS contract 改名**：`--agent-app-bg` → `--northhing-bg`，如果 host CSS 没同步 export，MiniApp Demo 视觉会断
4. **Tauri bundle id 改了** → macOS / Windows 签名证书需重新生成
5. **i18n 字符串改了**：用户如果已经下载了 v0.1.0，再次启动会看到英文 `agent-app` 字符串（因为生成的 locale contract 已经替换了）

### 回滚路径

```bash
# 切回备份分支
git checkout backup/pre-rename-agent-app

# 如果已经 commit rename，revert 它
git revert <rename-commit-sha>
```

备份分支 `backup/pre-rename-agent-app` 在 commit `fb2f17c`（v0.1.0 merge commit）保存。

## 决策点记录（用于将来类似任务）

1. **Massive rename 必须先用脚本扫** 取代手动 sed — 跨 1500+ 文件的改动不可能手工
2. **字节级替换** 比 decode-encode-replace 更安全（不破坏多字节字符）
3. **必须处理脚本自引用** — 脚本里出现的字符串会被自身规则改写
4. **Windows 文件锁** 让 `target/debug/` 目录很难原地 rename — 用 git commit + 用户手动 mv 配合
5. **历史计划** 加 LEGACY 注释是 better than 全删 — 历史决策信息保留



---

## 附录:第二阶段改名 — Northing → NortHing

> **触发**: 用户后续指示产品名大小写从 Northing 改为 NortHing(驼峰大小写风格)
> **执行时间**: 2026-06-25(同日)
> **commit**: 紧随 667a47e 之后

### 变更范围

| 类别 | 旧 | 新 |
| --- | --- | --- |
| 产品名 | Northing | NortHing |
| lower-case | northhing | northhing |
| UPPER_CASE | NORTHHING_* | NORTHHING_* |
| 第三方 brand | opennorthhing | opennorthhing |

### 实施

1. 新脚本 scripts/rename-to-northhing.py 实现 case-sensitive 替换(862 files, 8770 replacements)
2. theme preset 文件名 northhing-*.json → northhing-*.json(git mv)
3. ui.bundle.js byte-level rename(minified 资产)
4. docs/northing-name.md → docs/northhing-name.md(git mv)

### 验证

```
cargo check -p northhing-core -p northhing-server → Finished in 2m 30s, 0 errors
cargo test --workspace --lib --exclude northhing --exclude northhing-webdriver --exclude terminal-core
 → 1516 passed; 0 failed
```

### 注意

- 用户在第一阶段 commit (667a47e) 后中断并提出大小写修改;commit 667a47e 保留了 northhing 大小写,需要二次 commit
- 第二阶段 commit SHA 待记录(在 git commit 后回填)
- 大小写修改是 cosmetic 决策,技术债 0(不影响外部接口)
