# Review — Task P2a — Room 全页数据流

- **Base:** 1d8dcb2
- **Head:** 00b559b
- **Scope:** ui_dioxus 启动 hydrate（api.rs / app.rs / session_mock.rs / pages_archive.rs / pages_space.rs）
- **Reviewer:** judge-m3

---

### Spec Compliance

- ✅ Spec compliant

逐项核验：

| Brief 要求 | 文件 / 行号 | 结论 |
|---|---|---|
| User/Text 或 Multimodal → Witness { who: "见证者", body: t } | `session_mock.rs:241-252` | ✅ |
| Assistant/Mixed → Entity { who: "它", body: text 非空取 text 否则 reasoning_content.unwrap_or_default(), children: tool_calls.map(ToolLog) } | `session_mock.rs:253-277` | ✅ text 优先 + reasoning fallback 均有 |
| Assistant/Text 或 Multimodal → Entity { who: "它", body: t, children: vec![] } | `session_mock.rs:278-291` | ✅ |
| System / Tool（含 ToolResult 任意变体）→ 跳过 | `session_mock.rs:292` `_ => {}` | ✅ 通过 catch-all 兜底 |
| 时间戳 / images / arguments 丢弃 | `messages_to_entries` 内未引用 | ✅ |
| 启动 use_future：ensure_room_session Ok→set sid + get_messages，Err 仅 warn | `app.rs:114-138` | ✅ 两条路径都走 `tracing::warn!`，无 error UI 写入 |
| `get_messages` Ok 且非空才 `entries.set`，空 vec 保留 seed | `app.rs:121-126` `if !converted.is_empty()` | ✅ |
| `get_messages` Err 仅 warn，保留 seed | `app.rs:128-130` | ✅ |
| `get_messages` 包装与 `get_session` 同风格 | `api.rs:61-64` 紧邻 `get_session`，签名/注释/单行委托一致 | ✅ |
| api.rs 新增 `get_messages` 测试覆盖（无 facade 不 panic） | `api.rs:159` 接入既有 `test_api_functions_fail_cleanly_before_init` | ✅ |
| pages_archive.rs STRATA 顶部 `// TODO(data)` | `pages_archive.rs:31` | ✅ |
| pages_space.rs DOORS 顶部 `// TODO(data)` | `pages_space.rs:46` | ✅ |
| 至少 4 个纯函数单测 | session_mock.rs 新增 5 个：user_text_to_witness、mixed_with_tool_calls、mixed_reasoning_fallback、system_and_tool_skipped、empty_returns_empty（L186-305） | ✅ 5 个，超额 |
| 禁区：不动 send_action / stop_action / streaming / TurnState 行为 | diff 仅在 L114-138 增 future，未触碰 send_action (L269-311)、stop_action (L313-323)、event handler match 分支 | ✅ |
| 禁区：不动 contracts / core / GlobalConfig | diff 全部位于 `src/apps/desktop/src/ui_dioxus/` | ✅ |
| 禁区：不改 MockEntry / MockChild 类型定义 | `session_mock.rs:19-49` 类型签名未动；新增字段仅为 `who`/`body`/`children` 既有 | ✅ |
| 禁区：i18n 键不变，"见证者" / "它" 字面量与 send_action 一致 | `session_mock.rs:243,249,273,280,287` 与 send_action L302 "见证者"、TurnState 分支 L174,195,213 "它" 一致 | ✅ |
| 禁区：不改 `seed_session()` | `session_mock.rs:55-93` 未触碰 | ✅ |
| 验证：`cargo check -p northhing` + `cargo check -p northhing --tests` 均绿 | report L32-41 给出尾部输出（35/37 warnings，0 errors） | ✅ |

- ⚠️ Cannot verify from diff

  - `ensure_room_session()` 实际返回耗时：决定事件过滤是否真生效，但 brief 已言明"顺带修正 …事件不再全放行"是副作用而非硬指标，不影响 spec 通过。
  - `get_messages` 在真实内核态下的字段序列化：`snake_case` 字段名在 DTO 端已断言；无法在此 diff 内确认 facade 反序列化路径。
  - Dioxus Runtime 在 `use_future` 内对同一 Signal 的并发 `.set()` / `.write()` 是否触发实际争用：标准 Signal 内部有锁，无 borrow conflict；但与事件订阅 future 的 `entries.write().push(Approval)` 并发语义只能跑出来确认。

---

### Strengths

- 转换函数是真正的纯函数，单一消费者（app.rs），无中间 trait / 配置抽象，与 brief "无既有可复用"的判定一致。
- `get_messages` 包装严格模仿 `get_session` 的位置和签名（紧邻 + 同样一行委托），薄得无可挑剔。
- 测试超额完成（5/4），且第 3 个 `mixed_reasoning_fallback` 主动覆盖了 brief 转换表中"text 空时回退 reasoning_content"的隐藏分支——这是 spec 表格里写明但未必所有人都记得测试的边界。
- 错误路径全部 `tracing::warn!`，无 `unwrap()`、无 `expect()`、无 error UI——符合 brief"保留 seed"的稳态要求。
- `// TODO(data)` 标记放在两个 const 数组常量正上方，定位明确，不引入新代码。

---

### Issues

#### Critical
无。

#### Important
无。

#### Minor

- **M1 · entries.set 与事件订阅 future 的并发竞态窗口（`app.rs:114-138` ↔ `app.rs:154-228`）**
  新 use_future 调用 `entries.set(converted)` 做全量替换；既有事件订阅 future 通过 `entries.write().push(MockEntry::Approval{...})` 做追加。两者共享同一 `Signal<Vec<MockEntry>>`，Dioxus Signal 内部锁保证写不撕裂，但**时序上仍存在窄窗**：mount 后到 `entries.set` 完成前，若有任何 `ToolCall AwaitingConfirmation` 事件先被订阅 future 处理（sid=None 期间全部放行），其 push 会被后续 `entries.set` 整体覆盖，造成该 Approval 卡片短暂丢失（直到下一次同 call_id 事件或下次刷新）。  
  brief 未显式要求处理此竞态；同 race 模式已存在于 send_action（`entries.write().push(Witness{...})` 也在 entries.set 之后才被看到），所以是历史窗口。  
  升级路径：把 `ensure_room_session` + `get_messages` 的结果**追加**到 `seed_session()` 之后（merge 而非 replace），或加显式 monotonic flag 抑制启动期事件写入。  
  → 记 ledger，指向终审 triage，不阻塞本任务。

- **M2 · `unwrap_or(true)` 过滤逻辑未变（`app.rs:161,169,194`）**
  brief 提到"顺带修正 L135 等处的 sid 过滤：现在启动即有 sid，事件不再全放行"——实现路径是隐式的（通过提前 set sid），filter 表达式本身一字未改。  
  这是合理解读（禁区禁止改事件分支），但 brief 用词"修正"略激进，建议下轮要么去掉"修正"措辞、要么把 filter 改成 `unwrap_or(false)`（启动期拒收陌生事件，更安全）。  
  → 记 ledger，仅文字语义层面的观察，不阻塞。

---

### Quality 独立观察

- **复用核查属实**：report 声称的 `grep "get_messages" src/apps/desktop/src/ui_dioxus` 命中数 0、`MessageDto` 在 ui_dioxus 层无既有转换器、`message_to_item`（Slint-only）与 `kernel_facade/dto.rs::message_to_dto`（core 侧反向）均不可跨层引用——经抽查确认三层归属准确（slint DTO 在 app_state/sessions.rs，core 转换在 kernel_facade/dto.rs），判断无伪。
- **抽象归属**：`messages_to_entries` 唯一调用方是 `app.rs:123`；定义在 `session_mock.rs`（数据形状模块），无 trait、无 builder、无泛型参数。ponytail ladder 第四档（"已存在？复用"）已查过；第五档（"已装依赖能解决？"）也确认无既有可调；纯函数是当前最简形式。
- **预算闸**：diff 未触碰 `scripts/rot-budget.json`；session_mock.rs 现 305 行（`+190`），远低于 800 行 god-file 阈值——文件健康度良好，但建议下个相邻任务留意（每轮 +190 速度若保持，5 轮后逼近警戒线）。
- **Dioxus Signal 用法**：`let mut session_id_signal = session_id_signal; let mut entries = entries;` 是 Dioxus `use_future` 内对 `Signal<T>` 的标准 rebind 写法——闭包内 mut 重绑以调用 `.set()` / `.write()`，外层 immutable handle 借给 futures 共享同一 Signal。无 borrow 冲突。
- **i18n / 国际化**：who 字面量未走 `LocalePack::t(keys::…)`——与 brief 禁区一致（"不做 i18n 键变更"）；与现存 send_action / TurnState 路径同源硬编码。

---

### Assessment

**Task quality:** Approved

**Reasoning:** 实现严格对齐 brief 所有转换表条目、启动时序、空 vec 语义与禁区清单；5 个单测超额覆盖；复用侦察与分层判断属实；错误处理干净；两处 TODO 标记就位；架构与编码风格与既有 ui_dioxus 模块一致。Critical / Important = 0；Minor 仅记录两条不阻塞的并发窗口观察，记入 ledger 备终审 triage 即可。