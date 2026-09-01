# W8 计划：god-file 腐化修复波（2026-08-28/29）

来源：4 份代码层深审报告（`deep-rot-*.md`，2026-08-29 四路并行审查）。用户拍板顺序：input.rs → memory_db.rs → selectors.rs → app.rs + 硬编码路径。

## 总原则（全波钉死）

- **行为零变化**：本波全部是腐化清理，不是功能变更。移动代码不改逻辑；去重不改语义；任何"顺手改进" = 越界。
- **机械位移优先，架构重排禁止**：深审建议的架构级方案（如 popup dispatch 下沉 trait 化）本波不做，记 ledger 后续项。
- 每任务收口 ceiling 只降不升；文件拆分导致登记路径失效的，同 commit 清理 manifest（doc sync）。

## Task 1 (W8-1)：input.rs 拆分（最高风险，零测试文件）

- `input.rs` (802) → `input/` 目录模块：`mod.rs`（保留公共入口+类型）+ 按拦截层拆子模块；`handle_key_event` 543 行按拦截层（permission → question → global popup → info → command palette → specific popup → catch-all）拆 helper，**纯位移**
- 提取 `bridge` helper 消除 7 处 block_in_place 复制
- manifest：`god_file:...input.rs` 条目同 commit 处置（文件消失→清条目；新子模块 <800 无需登记）

## Task 2 (W8-2)：memory_db.rs 内部去重

- search_facts(183L)/get_facts(115L) 的三重内部复制（Some/None 分支、字符串→枚举 match、query_map 闭包）抽共享 helper；dead 变量线 ×2 + NaN/时钟 hack ×2 按深审报告处置
- 有既有 tests 模块做回归网；测试必须全绿且不许删改既有断言语义

## Task 3 (W8-3)：selectors.rs 消复制

- 三处复制（ModelItem 映射 ×chat/model.rs、time-ago 格式化 ×chat/session.rs、custom_headers 解析自重复）抽共享 helper 到合适归属（cli crate 内，就近原则）

## Task 4 (W8-4)：app.rs 抽离 + 硬编码路径修复

- 颜色工具三函数（parse_hex_rgb/mix_hex/chronicle_gradient + 其测试）→ 新模块（ui_dioxus/color.rs 或 css 邻近）；win_ops + close_* 系列 → 新模块（ui_dioxus/window_ops.rs）
- PopupType→hide 映射重复（close_all_popups vs navigate_back）抽单一定义处
- 收口后 app.rs 行数大降 → **manifest ceiling 962 同 commit 下调到实测值**（棘轮只降不升的正确方向）
- pages_onboarding.rs:133 硬编码开发者路径 → 产品正确默认值（空+占位符 或 系统标准目录；实现者核 AppSettings 的 workspace 默认解析逻辑后定，禁止发明新配置项）

## Global Constraints（全波通用）

1. 分层边界：W8-1/W8-3 只在 `src/apps/cli`；W8-2 只在 `src/crates/assembly/core`；W8-4 只在 `src/apps/desktop`。
2. 日志纪律：英文无 emoji。本波原则上零新增日志。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；**禁止 `git restore .`/`git checkout .`/`git stash` 等整树操作**（W7-2 事故教训），只许点名文件 add/commit。
4. rot-budget：ceiling 只降不升；manifest 变更只允许降 ceiling 或清死条目，且必须在同 commit 说明。
5. 验证最小集：MSVC `cargo check -p <crate>`（W8-2 用 `check -p northhing-core`）+ 该 crate 测试 + `node scripts/verify-rot-budget.mjs` 收口绿；命令+输出原文进 report。
6. commit 规则：每任务恰好一个 commit；不含 `.superpowers/`。
7. 不新建无 owner 抽象；去重提取的 helper 必须有 ≥2 个真实调用方。
8. 涉 keyring/真实 OS 资源：测试不得触生产存储（MockKeyring 纪律）。本波预期不涉。
9. 行为零变化铁律：judge 将逐臂核对位移 diff；发现逻辑漂移 = Critical。

## 终审

W8-4 完成后 review-package <w8-base>..HEAD。w8-base = W8-1 派发前 HEAD（`3ab2330`）。
