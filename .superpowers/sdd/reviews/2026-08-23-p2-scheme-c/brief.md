# Review Brief — P2 契约去秘密化（Scheme C 只写 key 通道）

## 审查对象

- Diff: `E:\agent-project\NortHing\.superpowers\sdd\reviews\2026-08-23-p2-scheme-c\diff.patch`（8 文件，+126/-100）
- BASE = `cbedffa`；范围 = 全部 staged 改动。
- 仓库：`E:\agent-project\NortHing`。需要完整上下文读仓库文件；工作区有无关未 staged 改动（kernel-api/memory.rs、turn.rs、progress.md、theme.rs 等），**不在审查范围**。

## 被审方案自述（外部 agent 交付声明，逐条核真伪）

1. `kernel-api/settings.rs`：`AIModelConfigDto`/`ProviderConfigDto` 删除 `api_key` 字段，DTO doc 写入"任何 kernel 返回的 DTO 形状都不能携带秘密值"不变量；`upsert_model_config(config, api_key: Option<String>)` 为 key 唯一入口（Some 设置、None 保留现值，merge 语义与旧版逐字段对齐）；`ProviderFormDto` 纯入向保留。
2. `core/kernel_facade/settings.rs`：`list_model_configs`/`get_global_config` 不再产 key；upsert 两分支改用参数。
3. desktop：推送循环从"整 DTO 读-改-写"简化为"读 id 列表 + set key"；upsert-provider 传 `Some(effective_key)`；删 dead 的 `provider_to_ai_model_config`/`provider_wire_format` 连测试；push 测试断言改走 core 内部 `get_ai_models()`。
4. `kernel-api/lib.rs`：新增 `contract_shape_tests` 源级扫描全 crate pub 字段，命中 {api_key, access_key, private_key, secret, password, credential, token} 精确分词即 fail；声称 `SkillOverrideEntry.key` 与 `*_tokens` 不命中、fn 参数无 pub 天然豁免。
5. 验证声称：三编译门绿、contract_shape 1/1、desktop app_state 90/90（净 −1）、rot-budget 绿且 expect 1092→1089 / dead_code 111→109 双下调。

## Constraints（验收硬标准，逐条核）

1. **Scheme C 不变量**：core 不得把 api_key 持久化到磁盘（load scrub + skip_serializing + set_config 快照恢复路径不得被本批削弱）；DTO 出参不得携带秘密值。
2. **upsert merge 语义**：`None` 更新必须保留现 key，与旧版逐字段 merge 等价——这是静默丢 key 的高危点，重点核 `core/kernel_facade/settings.rs` 两分支与 core `update_ai_model` 的交互（F1 失效逻辑是否仍触发）。
3. **desktop provider-test 流**：去 key 化后测连接必须仍可用——`ProviderFormDto` 入向携带 key 的路径是否完整，推送循环"读 id + set key"是否语义等价于旧整 DTO 读-改-写。
4. **六层分层**：contracts 不得新增向上依赖；`contract_shape_tests` 作为测试代码位置是否合规。
5. **远程兼容**：进程外 kernel 场景下 key 不出进程边界的设计是否成立（DTO 面无 key 后此条应自然满足，确认无残留泄漏面如 `ProviderConfigDto` 其他字段）。
6. **rot-budget 只降不升**：json diff 仅 4 行，核实为 expect/dead_code 双下调、无上调。
7. **治理测试有效性**：`contract_shape_tests` 的分词逻辑是否真会命中 `pub api_key: Option<String>` 形状而放过 `SkillOverrideEntry.key`/`*_tokens`/fn 参数——读实现核真假阳性边界，不接受自述。
8. **日志英文-only、无 emoji**；家规 god-file/测试绑定照旧。

## 输出要求

写到 `E:\agent-project\NortHing\.superpowers\sdd\reviews\2026-08-23-p2-scheme-c\report.md`：
- 双判决：spec 合规（自述 1–5 逐条 PASS/FAIL 带行号证据）+ 代码质量。
- findings 分级 Critical/Important/Minor，文件:行号 + 证据。
- Cannot-verify-from-diff 单独列清单。
- 最后一行 `APPROVE` / `REQUEST_CHANGES`。
