# W8-3 Review（judge 验收）— Approved

- **commit**: `53e70dc`（单 commit，6 文件 +130/-64）
- **范围**: `src/apps/cli`（chat/model.rs、chat/session.rs、ui/model_selector.rs、ui/session_selector.rs、ui/startup/selectors.rs）+ `scripts/rot-budget.json`
- **仓库**: E:\agent-project\NortHing（main）

---

## 双判决（独立结论）

### SPEC 判决 — Pass

逐条核对 w8-3-selectors-dedup-brief.md + Global Constraints：

| 条目 | 期望 | 实测 | 结论 |
|---|---|---|---|
| ModelItem 映射提取 | `ModelItem::from_config(&AIModelConfig) -> ModelItem` 挂在定义模块 | `ui/model_selector.rs:26-35`，同模块导出 `From<&AIModelConfig>` | OK |
| ModelItem 两处切换 | selectors.rs + chat/model.rs 都用新 helper | diff `-8` 行 + `from_config(&m)` 两处调用 | OK |
| `.filter(\|m\| m.enabled)` 语义保持 | 调用前 filter 不变 | `selectors.rs:101` + `chat/model.rs:107` 均保留 | OK |
| time-ago 共享函数提取 | 唯一 owner + 两处切换 | `ui/session_selector.rs:27-43`（`format_elapsed` + `format_time_ago`），`selectors.rs:4` 与 `chat/session.rs:4` 双引用 | OK |
| time-ago 文案/阈值零变化 | 四档边界 + "just now"/"Xm ago"/"Xh ago"/"Xd ago" | `format_elapsed` 行 28-36 与原实现逐字符等价（60/3600/86400） | OK |
| custom_headers 文件内 helper | save/update 两臂等价 | `selectors.rs:852-860` `parse_custom_headers`，`save_new_model` + `update_existing_model` 各调用一次 | OK |
| ceiling 下调 875→861 | 同步 manifest | `rot-budget.json:74` 861，实测 861 行（`wc -l` 等价：read 末行 = 861） | OK |
| 单 commit、不含 `.superpowers/` | git show --stat 6 文件 | 6 文件均非 `.superpowers/`，单 commit | OK |
| rot-budget 只降不升 | 唯一变更条目 | 仅 `god_file:.../selectors.rs` ceiling 875→861，无其他改动 | OK |
| 不造无 owner 抽象 | 每个 helper 有 2 真实调用点 | from_config×2、format_time_ago×2、parse_custom_headers×2 | OK |
| 编译 | `cargo check -p northhing-cli` 0 error | 实跑 `Finished dev profile ... in 1.78s`（仅 pre-existing warning） | OK |
| 测试全绿 | `cargo test -p northhing-cli` | 实跑 41 passed;0 failed（含 3 个新测） | OK |
| rot-budget verify | `node scripts/verify-rot-budget.mjs` | 实跑 `Rot budget verification passed (7 god-file rules ...)` | OK |

### QUALITY 判决 — Pass

逐项打分：

- **复用核查（消三处复制）**：三处复制在 main 源码中已无副本。grep `ModelItem { id:` 无匹配（原复制点已切到 `from_config`），grep `elapsed().unwrap_or_default()` 在 selectors.rs 已消失（原 time-ago 块被 helper 替代）。`parse_custom_headers` 在 selectors.rs 中仅 1 处定义 + 2 处调用，无第三份。
- **owner 合理性**：
  - `ModelItem::from_config` 在 `ui/model_selector.rs`（ModelItem 定义处）— 类型 owner 即 helper owner，零跨文件跳转。
  - `format_elapsed`/`format_time_ago` 在 `ui/session_selector.rs`（SessionItem 定义处 + 显示文案是 SessionItem 的字段）— 同上。
  - `parse_custom_headers` 文件内私有 fn（仅 selectors.rs 用）— 正确限定作用域，避免泄漏。
  - 三处归属各有真实理由，未下沉到无关 util 模块（无新抽象层）。
- **行为零变化**：
  - `from_config` 字段顺序 / 字段名 / 类型与原 `{ id: m.id, name: m.name, provider: m.provider, model_name: m.model_name }` 等价（差异只是 `&m.id` → `m.id.clone()`，对 `&String` 调用 clone 仍得相同 String）。
  - `format_time_ago` 内部 `elapsed().unwrap_or_default()` 与旧版同位同语义；`format_elapsed` 四档阈值 60/3600/86400、字符串格式与旧实现逐字符一致。
  - `parse_custom_headers` 与旧实现的差异仅是 `headers_mode` 接收 `&str` 而非已 clone 的 String，输出 `Option<String>` 内容等价；并且去掉了一次无条件 clone（仅在需要时才 `to_string()`）— **非漂移，是微优化**。
- **测试有效性**（3 新测）：
  - `test_model_item_from_config`：覆盖 4 个字段各自赋值，并验证 `From<&AIModelConfig>` trait 实现。需字段名拼写错、字段缺失、trait 实现错误三类 bug 任一发生才挂。**非恒真**。
  - `test_format_elapsed_four_tiers`：14 个断言覆盖全部四档 + 边界（59→just now、60→1m ago、119→1m ago、120→2m ago、3599→59m ago、3600→1h ago、7199→1h ago、7200→2h ago、86399→23h ago、86400→1d ago）。需任意阈值错位或文案漂移才挂。**非恒真**。
  - `test_format_time_ago_recent`：验 `SystemTime::now()` 路径走 `unwrap_or_default()` → elapsed ≈ 0s → "just now"。覆盖了 time→elapsed→format_elapsed 的串联路径（防止某天有人把 `unwrap_or_default()` 改 `?` 而静默失败）。**轻量但有意义**。
- **预算闸**：仅 `selectors.rs` ceiling 下调 875→861；json 其他字段未触。
- **god-file 观测**：861 > 800（review pressure 区间），< 1000（不强制拆分）。rot-budget.json 中该条 note 保留 "R-14 god-file; live observation cohort (T2-6 superseded)"。本次 ceiling 下调同步观测值，符合 house rule 1。
- **logging**：零新增日志（diff 全文未触及 `tracing::`）。
- **i18n**：`"just now"`/`"Xm ago"` 等仍是硬英文，与 AGENTS i18n 规则表面冲突——但 AGENTS.md 注明 v0.1.0 桌面 CLI 硬编码中文+i18n 工程冻结中，且深审报告 §二特殊发现已记录此为"CLI 上下文下属合理例外"。非本任务范围。

---

## C/I/M 计数

- **Critical**: 0
- **Important**: 0
- **Minor**: 0

---

## 一句话理由

三处复制按 brief 要求正确归属到类型 owner 模块（ModelItem→model_selector、time-ago→session_selector、custom_headers→文件私有），行为零漂移（time-ago 四档阈值/文案逐字符等价、ModelItem 字段顺序/语义、custom_headers 输出 Option<String> 等价），ceiling 875→861 与实测行数同步、3 个新测覆盖全部档位边界且非恒真，编译 0 error / 41 测试全绿 / rot-budget verify 通过。