# W7 计划：F7 设置页 provider 编辑功能（2026-08-28）

来源：W4-2 壳审计 F7 欠账；用户 2026-08-28 拍板做（选项 A）。侦察：`.superpowers/sdd/w7-f7-settings-edit-recon.md`（功能清单/缺口/复用/风险全在其中，任务书直接引用）。

## 功能范围（用户拍板 A = 编辑功能，含完整管理闭环）

设置页 Card 3 provider 区从只读列表升级为：每行**编辑**（弹窗表单：名称/类型下拉/Base URL/模型/API Key 留空=不变/启用开关）+ **测试连接** + **删除**（确认后删配置+清 keyring）。

编排者裁定（钉死）：
- **删除守卫**：删除默认 provider 一律拒绝（提示先切换默认）；删配置 + 删 keyring 两步都做；会话引用完整性检查**本波不做**（ponytail：记 ledger 后续项）。
- **key 语义（I1 教训重建）**：编辑时 key 留空 = 保留 keyring 现有 key；填新值 = 覆盖；keyring 读失败 = **fail-closed 拒绝保存**（绝不能当成"清空"）。不提供"清空 key"路径（要清就删 provider）。
- **类型显式选择**：编辑表单用 provider_type 下拉（anthropic/openai/gemini/custom-openai/custom-anthropic），禁用 URL 启发式推断（W5-3-M1 坑）；`provider_wire_format_from_str` 做映射，实现者先核 sync.rs 实际接受的字符串集。
- **i18n frozen**：desktop 现状硬编码中文文案，不加 ftl key。

## God-file 防线（实测行数，check 口径）

| 文件 | 实测 | 线 | 余量 | 本波规则 |
|---|---|---|---|---|
| app.rs | 959 | 962 | **3** | **禁止触碰** |
| pages_settings.rs | 731 | 800 | 69 | 只加行按钮+弹窗挂载点，增长 ≤60 |
| api.rs | 718 | 800 | 82 | 只加 mod 声明/re-export，增长 ≤10 |
| 新文件 | — | 800 | — | API 层新文件 + UI 层新文件，各 <800 起步 |

## Task 1 (W7-1)：API 层 — 编辑/删除 provider 的 Dioxus 包装 + keyring 语义

新文件 `src/apps/desktop/src/ui_dioxus/api_provider_edit.rs`（api.rs 零逻辑新增）：
- `edit_provider_with_keyring(keyring: &dyn KeyringBackend, ...) -> Result<(), KernelError>`：载入现有 config（不存在=Err）→ `validate_provider_input` 校验 → `resolve_edit_api_key` 解 key（留空继承/新值覆盖/读失败 fail-closed）→ `upsert_model_config(dto, key)`。
- `delete_provider_with_keyring(keyring: &dyn KeyringBackend, id) -> Result<(), KernelError>`：是默认 provider=Err 拒绝 → `delete_model_config` → `delete_api_key`（best-effort）。
- 复活 `resolve_edit_api_key`（sync.rs:16，去 `#[allow(dead_code)]`）；`resolve_effective_api_key`（sync.rs:5）若仍无调用方则一并删除（顺手清配额，dead_code 计数 −1）。
- 测试（MockKeyring）：留空继承 / 覆盖 / 读失败 fail-closed / 编辑不存在 id 报错 / 删除拒绝默认 / 删除清 keyring / 校验失败不写入。

## Task 2 (W7-2)：UI 层 — Card3 编辑入口 + 弹窗表单

新文件 `src/apps/desktop/src/ui_dioxus/pages_settings_provider_edit.rs`：弹窗组件（字段集见侦察 §4.2 mockup）+ 状态机（查看/编辑/测试中/保存中/删除确认）+ 三失败臂 UI 显式报错（对齐 W5-3 onboarding 语义）。pages_settings.rs 只加：每行编辑按钮 + 弹窗挂载 + 保存后刷新列表。改动前后截图进 report（编排者视觉验收）。

## Global Constraints（全波通用）

1. 分层边界：改动只在 `src/apps/desktop`；其它 crate 零改动。
2. 日志纪律：新增日志一律英文、无 emoji，带关键上下文字段。
3. SDD 禁区：implementer 禁止以任何 git 操作触碰 `.superpowers/`；禁止编辑 `progress.md`；report 用 write 工具写入 `.superpowers/sdd/`。
4. rot-budget：不上调任何 ceiling；god-file 防线按上表执行；新文件 <800 行；`node scripts/verify-rot-budget.mjs` 收口必须绿。
5. 验证最小集：`& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check -p northhing` + 聚焦测试 + rot 实测；命令与输出原文进 report。
6. commit 规则：每任务恰好一个 commit，消息对齐近期 git log；不含 `.superpowers/` 产物。
7. 不新建无 owner 抽象；复用侦察 §5 清单里点名的既有设施（upsert/delete_model_config、store/delete_api_key、resolve_edit_api_key、validate_provider_input、MockKeyring、ProviderFormDto）。
8. i18n frozen：硬编码中文文案，不动 ftl。
9. 家规 4 并发测试绑定：本波不碰 tokio 任务生命周期/取消/关闭顺序（UI 表单 + 同步 facade 调用），无强制并发测试义务；API 层逻辑测试按任务书清单执行。

## 终审

W7-2 完成后 review-package <w7-base>..HEAD 派终审。w7-base = W7-1 派发前 HEAD。
