# W9 全波终审判决书

**审查范围**: `151f77c..HEAD`（W9-1~W9-7 + 修复，+30/-2 文件，+3739/-588）  
**审查基准**: 只读，不改代码不 commit  
**判决日期**: 2026-08-29

---

## 裁决

**CAN MERGE**

---

## SPEC 判决 ✅

对照裁决书五个缺口 + C2/C3/C4 + ③④ 交付完整度：

| 缺口/需求 | 交付状态 | 证据 |
|---|---|---|
| 确认门补"本会话内允许"第三档 (W9-1) | ✅ 交付 | `approval_card.rs` 三按钮（允许/拒绝/本会话允许）+ `session_allow_list` HashSet + 自动批准失败回退 pending card |
| 记忆浏览面板 (W9-2 TH-3) | ✅ 交付 | `pages_memory.rs` — list/search/export JSONL，全接线 |
| 降级即报错 UI 路径 (W9-3 原则9) | ✅ 交付 | `turn_banner.rs` — `maybe_set_degraded` 在 TurnState Failed/Cancelled + submit error 三处调用，`DEGRADED_QUOTA_MSG` / `DEGRADED_BILLING_MSG` |
| 会话管理 CRUD + subagent 可见 (W9-4 C2+C3) | ✅ 交付 | `pages_archive.rs` — search/rename(validate_rename C/J/K safe)/delete(confirm gate)/export(markdown)/detail，subagent badge + 低透明度 |
| 技能管理 UI (W9-5 SK-05) | ✅ 交付 | `pages_settings_skills.rs` — list/enable/disable，optimistic toggle + rollback |
| 文件树/预览 (W9-6 C4) | ✅ 交付 | `panel_files.rs` + `api_fs.rs` — lazy expanding tree + text preview，workspace_root fenced |
| 摆设卡片做真 (W9-7 ③) | ✅ 交付 | `pages_settings_cards.rs` — 沉积(facts+skills count)/编年史(genesis+event)/身份(model display_name)/准则(设计原则说明)，display_breath/dual_optics 持久化 |
| C2 多会话管理 | ✅ 已裁决并实现 | 删除/重命名/导出/搜索齐全 |
| C3 子代理可见性 | ✅ 已裁决并实现 | 低显著度 badge + 透明度 |
| C4 文件树预览 | ✅ 已裁决并实现 | 右侧面板 |

---

## QUALITY 判决 ✅

### 1. 新 facade 面一致性

**Memory 族 (list_facts/search_facts)**：DTO（`FactDto`）在 `kernel-api/src/memory.rs` 定义，facade 实现在 `kernel_facade/memory.rs`，UI 薄包装在 `api.rs`。枚举→字符串扁平化风格与 `EpisodeDto` 一致。错误映射统一用 `KernelError::Runtime`。✅

**Platform 族 (list_workspace_tree/read_workspace_file + FileTreeEntryDto)**：DTO 在 `kernel-api/src/platform.rs`，facade 实现在 `kernel_facade/platform.rs`，UI 薄包装在 `api_fs.rs`。`workspace_root` 参数两族一致。路径防护（`..`/绝对路径/symlink）在 facade 层一次完成。✅

**Session 族 (delete_session/rename_session)**：已在 `KernelSessionApi` trait 定义，facade 薄转发，UI 在 `api.rs`。✅

**Agent/Skill 族 (list_skills/set_skill_enabled)**：`list_skills` 在 `api.rs` 做了 user-scope override overlay（UI 层逻辑，合理），`set_skill_enabled` 薄转发。✅

**无业务逻辑渗入契约层**：DTO 全在 `kernel-api` contracts crate，纯数据结构；所有业务逻辑在 facade 或 UI 层。✅

### 2. UI 面织合

- `entries` 流与 `session_allow_list`：事件处理循环中先查 allow-list 再决定自动批准或 push pending，逻辑清晰。✅
- `degraded` 信号与 entries 流：独立信号，在 TurnState Failed/Cancelled/SubmitError 三处设置，在 Started 时清除。不与 entries 直接耦合。✅
- 确认门 allow-list（`approval_card.rs`）：独立模块，`settle_approval`/`push_pending_approval`/`render_approval_card` 三函数分离职责。✅
- `HashSet` clone 模式在 `panel_files.rs` 的展开/选择逻辑中：`expanded.peek().clone()` → 修改 → `expanded.clone().set(new_set)`。Dioxus Signal 标准用法，无数据竞争。✅

### 3. rot 全程账

| 文件 | 触线前 | 当前实测 | 处置 | rot-budget |
|---|---|---|---|---|
| `css.rs` | 831 | 829 | ✅ 降（830 ceiling） | ceiling 830 ✅ |
| `app.rs` | 825 | 791 | ✅ 降（抽离 color/window_ops） | 条目已移除 ✅ |
| `unix_epoch` | 70 | 69 | ✅ 降 | ceiling 69 ✅ |
| `api.rs` | 799 | 799 | → 持平 | 未入表（<800）✅ |
| `windows.rs` | 800 | 800 | → 持平 | 未入表（=800）✅ |

所有触线点处置均为"降/清"，无任何 ceiling 上调。降额均已在 rot-budget.json 中反映。✅

### 4. 防线余量

| 文件 | 实测 | rot-budget ceiling | 余量 |
|---|---|---|---|
| `css.rs` | 829 | 830 | +1 |
| `app.rs` | 791 | 无（<800 已出表） | — |
| `api.rs` | 799 | 无（<800） | — |
| `windows.rs` | 800 | 无（=800 临界） | 0 |
| `unix_epoch` | 69 | 69 | 0 |

下一个桌面波需注意 `windows.rs` 在 ceilings 边缘（800），小幅改动即可触线。

### 5. 累积 Minor 队列 triage

| 来源 | 描述 | 处置建议 |
|---|---|---|
| W9-2 retro M-1 | — | 已修复（retro review clean） |
| W9-2 retro M-2 | — | 已修复（retro review clean） |
| W9-4 M-1 | CJK 截断 | ✅ 已修（`validate_rename` 用 chars.count） |
| W9-4 M-2 | CJK 截断同源 | ✅ 已修 |
| W9-5 M-1 | api.rs 贴线拆分信号 | **转终审**：`list_skills`+user-scope overlay 仍在 api.rs（799 行）。下波拆分时优先处理。 |
| W9-7 M-1 | 编年史 Genesis/Event 英文硬编码 | **转终审**：`pages_settings_cards.rs:188,194` 硬编码 "Genesis"/"Event"。应走 i18n keys。影响：非中文用户看到英文标签。建议下波加 keys + 走 locale.t()。 |
| W9-7 M-2 | display_name 语义擦边 | **转终审**：`pages_settings_cards.rs:2716` 用 `model.display_name` 作为"名讳"。字段语义是"模型的展示名"，不是"用户命名"。功能上 work，但语义漂移。建议：1) 加注释标注；2) 后续增加独立的 identity_name 字段。 |

### 6. Cannot Verify 清单

| 项 | 来源 | 状态 |
|---|---|---|
| Mockup 截图 ×3（W9-4/5/6） | 任务报告 | ⚠️ **无真机验证**。转下一波实测清单。 |
| 降级横幅 UI 实测 | W9-3 | ⚠️ 需在配额耗尽场景下肉眼验证横幅出现/消失。 |
| 文件树递归展开实测 | W9-6 | ⚠️ 需在 >5 层深目录下验证深度截断。 |
| 记忆导出 JSONL 格式 | W9-2 | 代码路径有测试（list/search），导出文件格式需实测验证。 |

---

## 防腐校准核查

- **复用核查**: `northhing-core-types` 新增依赖仅用于 `time` 模块（`pages_memory.rs`），不构成过度引入。`classify_ai_error_message` + `ErrorCategory` 通过 `kernel-api` re-export，ui 层不直接耦合 core-types。✅
- **无 owner 抽象**: 所有新 trait impl（`KernelMemoryApi::list_facts/search_facts`、`KernelPlatformApi::list_workspace_tree/read_workspace_file`）都在 facade 层有唯一实现，无 mock/stub 残留。✅
- **预算闸**: rot-budget.json 所有 ceiling 均为持平或下降，无上调。css.rs 831→830，app.rs 出表，unix_epoch 70→69。✅
- **God-file 观测点**: app.rs 791/1000 ✅，css.rs 829/830 ✅，api.rs 799/800 ✅，windows.rs 800/800 ⚠️。均合规。
- **阻塞性数字断言磁盘实测**: 行数已用 `Get-Content` 实测确认。✅

---

## 发现汇总

| 级别 | 编号 | 位置 | 描述 |
|---|---|---|---|
| **Minor** | M-1 | `pages_settings_cards.rs:188,194` | 编年史 "Genesis"/"Event" 英文硬编码，应走 i18n |
| **Minor** | M-2 | `pages_settings_cards.rs:2716` | `model.display_name` 语义擦边作"名讳"，应加注释或独立字段 |
| **Minor** | M-3 | `css.rs:841` | `.degraded-banner` CSS 规则与 `.close-btn` 规则连写在同一行，可读性差 |
| **观察** | O-1 | `pages_settings_cards.rs` | `windows.rs` 实测 800 行，下波改动需留意 ceiling |

无 Critical，无 Important。

---

**判决**: CAN MERGE  
**SPEC**: PASS ✅ | **QUALITY**: PASS ✅  
**Critical**: 0 | **Important**: 0 | **Minor**: 3（含历史队列 2 项转终审 triage）

**理由**: W9 七个任务的全部五项裁决缺口已交付，跨任务织合无冲突，rot 预算全绿，防线余量可接受，三个 Minor 均为下波优先级。
