# Task Brief — W11-2: selectors 克隆集群（helper 级消除 + 页面级合并地图）

仓库：E:\agent-project\NortHing（main）。范围：`src/apps/cli` 仅 CLI crate。
病灶：`blind-review-selectors-2026-08-29.md`（行号可能漂移，执行前 rg 核实存在性）。

## 本波范围裁定（钉死）

克隆集群分两层，本波只做 A 层 + 产出 B 层地图：

**A 层 = helper 级消除（全部做掉）**：
1. `provider_display_name`/`model_display_name` 嵌套 fn 逐字双份（selectors.rs:815-837 ↔ modes/chat/model.rs:31-53）→ 提取到唯一 owner（建议 `ui/model_selector.rs` 或现有共享处，与 W8-3 的 `ModelItem::from_config` 归同邻域），两处改调。
2. `block_in_place + block_on` 模板（selectors.rs 15 处 + 可能其它文件）→ **先查 W8-1 建的 `input/bridge.rs` 是否可复用**；可复用就统一调它，不可复用说明理由再建共享 helper。
3. `parse_custom_headers` 不对称（selectors 用 helper vs model_config.rs 内联）→ 统一到 helper。
4. 魔数 `128000` / `8192`（selectors.rs:295-296）→ 命名常量 + 注释。
5. `"primary"` 字符串哨兵（selectors.rs:813,839）→ 命名常量或 Option 化（选语义更诚实的，行为零变化）。

**B 层 = 页面级合并地图（只产出文档，不改代码）**：
selectors.rs ↔ modes/chat/{session,model,model_config,skill,subagent,theme,agent}.rs 的逐段克隆对账表（哪段对应哪段、是否逐字相同、合并难点），写入 report 的「B 层地图」节——供下波页面级合并决策。**B 层本波禁动代码。**

## 防线

- selectors.rs ceiling 861：A 层消除后行数下降 → **同 commit 下调到实测值**。
- 行为零变化铁律：所有替换必须语义逐字等价（judge 会逐块对照）。
- cli 既有测试全绿；A1/A2 提取的纯函数附 ≥2 聚焦测试（含 CJK/边界）。

## 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing-cli`：0 error
2. `+stable-msvc test -p northhing-cli`：全绿
3. `node scripts/verify-rot-budget.mjs`：绿

## Global Constraints

1. 分层边界：只动 `src/apps/cli` + manifest。
2. 日志英文无 emoji；零新增日志。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止整树 git 操作；开工先 `git status`。
4. rot-budget：ceiling 只降不升。
5. commit：恰好一个；不含 `.superpowers/`。
6. 不新建无 owner 抽象；每个提取 helper ≥2 真实调用点。
7. 遇编译错误先加载对应 rust skill。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因。
- 返回消息含：状态 / commit SHA / git show --stat / A 层逐项处置+证据 / B 层地图摘要 / 验证输出尾部 / selectors.rs 新行数 / 偏离清单。
