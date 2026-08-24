# Task T2-2n Review — MiniApp 整删 M4：product-domains miniapp 整删 + 内置 6 件套资产

> **任务**：Task T2-2n
> **审查者**：独立 reviewer
> **工作目录**：`E:\agent-project\northing`（main）
> **实测 HEAD**：`62163f6 sdd: T2-2m ledger line + brief/report/review/diff artifacts`
> **审查时间**：2026-08-19
> **判定**：**PASS**（双判决：SPEC 合规 + 质量合规）

---

## 1. SPEC 合规判决：✅ PASS

### 1.1 删除清单完整性（约束 #1）

**重构删档对照表**（实测 `git status --short` D 列 76 文件 vs 授权清单）：

| 类别 | 授权（recon Q1-A） | 实测 | 一致性 |
|---|---|---|---|
| `src/miniapp/*.rs` | 16 | 16 | ✅ |
| `src/miniapp/builtin/assets/` 应用目录 | 6 | 6 | ✅ |
| `src/miniapp/builtin/assets/ppt-live/` | 27（含 vendored bundle） | 27 | ✅ |
| `tests/` miniapp 专测 | 6 | 6 | ✅ |
| **小计 D** | **76** | **76** | ✅ |

**16 个 .rs** 全部命中：bridge_builder / builtin / compiler / customization / draft / exporter / host_routing / lifecycle / mod / permission_policy / ports / runtime_facade / runtime / storage / types / worker。  
**6 个应用资产目录** 全部命中：coding-selfie / divination / gomoku / ppt-live / pr-review / regex-playground。  
**6 个专测文件** 全部命中：builtin_and_ports / common/mod / compiler_export_storage_and_runtime / host_routing_and_lifecycle_helpers / permissions_and_bridge / runtime_facade_and_customization。

**越界检查**：D 列 76 文件 100% 落在 `src/crates/contracts/product-domains/src/miniapp/**` 或 `src/crates/contracts/product-domains/tests/**` 内，**0 越界**。

### 1.2 不碰项（约束 #2）

| 项 | 状态 |
|---|---|
| `function-agents` 模块/feature | ✅ 存活，cargo test 通过 26 个用例 |
| `core-types/services-core` serde 变体 | ✅ 未触碰；`MiniApp` 死变体保留（归属 M5） |
| `tool_call_accumulator.rs` | ✅ 未触碰 |
| 根 AGENTS/docs | ✅ 未触碰（git diff HEAD 证实） |
| 顶层 `MiniApp/`（M5） | ✅ 未触碰 |
| `.opencode/`、`memory/`、`.handoffs/` | ✅ 仅含并行 session 既有改动，未被本批扩大 |
| function_agents 配套 anchor | ✅ self-test.mjs / required-rules.mjs 保留 `function_agents/*` 全部契约块 |

### 1.3 Cargo.toml / 独占性（约束 #3）

**重构 Cargo.toml 终态**：
```toml
[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true, optional = true }

[features]
default = []
function-agents = ["tracing"]
product-full = ["function-agents"]
```

- ✅ `miniapp` feature 行删除
- ✅ `product-full = ["function-agents"]`（摘掉 `"miniapp"`）
- ✅ `dirs` / `sha2` / `which` 三个 optional dep 行删除
- ✅ `tracing` 保留（仍被 `function-agents` 使用，未误删）

**B6 rg 独立复核**（reviewer 自跑）：
```
$ rg -n "dirs::|sha2::|which::" src/crates/contracts/product-domains/
（零命中）
```
report 的 B6 独占性结论得到独立验证。

### 1.4 Cargo.lock 收敛（约束 #4）

**实测 Cargo.lock diff**：
```diff
@@ -6075,12 +6075,9 @@ dependencies = [
 name = "northhing-product-domains"
 version = "0.2.10"
 dependencies = [
- "dirs",
  "serde",
  "serde_json",
- "sha2",
  "tracing",
- "which",
 ]
```

**逐包核验**（dirs/sha2/which 是否被 workspace 其它 crate 共享）：

| 包 | 共享消费者 | 锁条目保留？ | 行为 |
|---|---|---|---|
| `dirs` | `northhing` (L5763), `northhing-cli` (L5903), `northhing-core` (L5950) | 保留 | ✅ 正确 |
| `sha2` | `northhing` (L5833), `northhing-core` (L5999), `northhing-services-core` (L6134) | 保留 | ✅ 正确 |
| `which` | `northhing-core` (L6015) | 保留 | ✅ 正确 |

`[[package]]` 条目本身保留（其它 crate 仍依赖），仅 `northhing-product-domains` 的 dependencies 数组摘除三引用——**这正是约束 #4 期望的最小化改动**。  
✅ 仅 3 行删除，**无版本漂移、无无关包变动**。

### 1.5 i18n-audit.mjs（约束 #5）

**行数差**：pre `3838 行` → post `3833 行` = **-5 行** ✓

**diff 区段**（hunk `@@ -1820,11 +1820,6 @@`）：
```diff
-    {
-      surface: 'core-miniapp',
-      root: path.join(root, 'src', 'crates', 'contracts', 'product-domains', 'src', 'miniapp', 'builtin', 'assets'),
-      predicate: (file) => file.endsWith('.js'),
-    },
```

- ✅ 5 行删除，0 行插入
- ✅ 范围：原文件 L3566-L3570（report 行号一致）
- ✅ byte-preserved：mojibake 段（`scripts/i18n-audit.mjs:481` `è¿?,`）改前改后 **同一 SyntaxError**

```
改前 SyntaxError: scripts/i18n-audit.mjs:481   'è¿?,
改后 SyntaxError: scripts/i18n-audit.mjs:481   'è¿?,
```

### 1.6 boundary 归零（约束 #6）

```
$ rg -i miniapp scripts/core-boundaries | Measure-Object -Line
0
$ node scripts/check-core-boundaries.mjs
Core boundary check passed.
```

✅ **归零** + **PASS**。

### 1.7 门禁复跑（约束 #7）

reviewer 自跑（MSVC wrapper）：

| 命令 | 结果 |
|---|---|
| `cargo check --workspace` | ✅ Finished in 12.72s |
| `cargo test -p northhing-product-domains --no-default-features` | ✅ 0 unit / 0 integration / 0 doc；result ok |
| `cargo test -p northhing-product-domains --features function-agents` | ✅ 8 unit + 18 integration = 26 passed；0 failed |
| `cargo check -p northhing` | ✅ Finished in 11.55s |

### 1.8 无夹带/无关格式化（约束 #8）

`git diff --stat HEAD` 对 10 个批内修改文件：
```
scripts/core-boundaries/rules/feature-rules.mjs              | 4 +-
scripts/core-boundaries/rules/source/forbidden-rules.mjs     | 2 +-
scripts/core-boundaries/rules/source/required-rules.mjs     | 304 ----
scripts/core-boundaries/self-test.mjs                      | 96 ----
scripts/i18n-audit.mjs                                     | 5 -
src/crates/contracts/product-domains/AGENTS-CN.md          | 3 +-
src/crates/contracts/product-domains/AGENTS.md             | 2 +-
src/crates/contracts/product-domains/Cargo.toml            | 3 +-
src/crates/contracts/product-domains/src/lib.rs            | 3 -
Cargo.lock                                                 | 3 -
```

每个 hunk 均可映射 brief 条款；**0 夹带、0 无关格式化**。  
外加 2 个并行 session 文件（`.opencode/model-capability-notes.md` 148+/memory/northhing.md 6+）保持不变——非本批范围。

### 1.9 AGENTS.md / AGENTS-CN.md 同步（约束 #9）

**EN diff**：
```diff
-- Feature-gated additions must remain narrow. `miniapp`, `function-agents`, and
+- Feature-gated additions must remain narrow. `function-agents` and
-- `miniapp` may own MiniApp data shapes, pure lifecycle decisions, metadata and
-  import policies, built-in bundle identity, embedded source assets, seed-plan
-  facts, marker wire formats, host primitive call plans, and narrow ports.
```

**CN diff**：
```diff
-- 新增 feature-gated 内容必须保持窄边界。`miniapp`、`function-agents` 和 `product-full`...
+- 新增 feature-gated 内容必须保持窄边界。`function-agents` 和 `product-full`...
-- `miniapp` 可以拥有 MiniApp 数据形态...seed-plan facts...marker wire format...
```

✅ 中英文结构一致；mojibake 仅 PowerShell 控制台渲染问题（`Format-Hex` 验证字节是合法 UTF-8），实际字节未损。

---

## 2. QUALITY 判决：✅ PASS

### 2.1 实现整洁度

- **A 项**：整删目录用 `git rm -r`，lib.rs 删 2 行精确（`#[cfg(feature = "miniapp")] pub mod miniapp;`），不留残骸
- **B 项**：Cargo.toml 改动精炼（4 行减少 + 1 行修改），tracing 正确保留
- **C 项**：i18n-audit.mjs 严格遵守 5 行授权，无 byte 损失
- **D 项**：boundary 修改策略合理：
  - `feature-rules.mjs`：product-domains 块 `dependencies: []`（空数组替代 `[{...}]`）
  - `forbidden-rules.mjs`：Command::new 例外由 `allowPaths: [...runtime.rs...]` 简化为无 allowPaths（全域禁用）
  - `required-rules.mjs`：12 个 miniapp 块整块删除（11 模块 + builtin.rs），保留 function_agents 块
  - `self-test.mjs`：移除 dirs/sha2 owner 校验 + Command::new allowPaths 校验 + 12 个 manifestContractChecks 项
- **E 项**：AGENTS.md / AGENTS-CN.md 中英文各 3 行精确改动，结构对齐

### 2.2 Cargo.lock 自动收敛证据

implementer 没有手工编辑 lock（report §B 5 注明），由 `cargo check -p northhing-product-domains` 自动收敛；diff 仅 3 行（仅 product-domains dependencies 数组去名），无意外漂移。

### 2.3 编译错误分层（report §4）

report 自报"遇到的编译错误：0 个"——这与 M1-M3 已清上层引用、本批是末梢整删一致；无 .clone()/.unwrap() 滥用迹象（因为没有需要编译的代码改动，零触发了不变量破缺）。

### 2.4 验证命令覆盖

report §5 八组验证命令 + 输出原文齐全；reviewer 独立复跑 4 组核心门禁（cargo check × 2 + cargo test × 2）全部 PASS。

---

## 3. Findings

### Critical
**无**。

### Important
**无**。

### Minor

| # | 位置 | 描述 | 建议 |
|---|---|---|---|
| M1 | `Cargo.lock` | `dirs` / `sha2` / `which` 的 `[[package]]` 条目仍存在（其它 crate 共享使用），属正确行为但 report 未明确说明"为何保留"；下批 M5 决策点若其它 crate 也用不上，仍可能产生孤儿 | 终审时若发现这些包仅在本批相关 feature 下被引用，可顺手清；当前不动是正确选择 |
| M2 | `scripts/core-boundaries/self-test.mjs` L580 | `productDomainRuntimeRule.path === 'src/crates/contracts/product-domains/src'` 仍存在并被 self-test 用于验证"Command::new 全域禁用" | 行为正确；M5 收口时可考虑是否仍需 |
| M3 | `scripts/core-boundaries/self-test.mjs` L2306-2332 | 保留 function_agents 的 4 个 manifestContractChecks 条目 | 与"function-agents 存活"一致，无需改 |

### Cannot Verify（reviewer 自行核验项）

| 项 | 处理方式 |
|---|---|
| 是否所有 27 个 ppt-live 文件全删（含 27,805 行 vendored bundle） | ✅ `git status --short \| grep ppt-live \| wc -l` = 27，与 recon 一致 |
| ppt-live/dist/ui.bundle.js 等非文本资产是否也被 git 跟踪并删除 | ✅ `git diff --stat HEAD` 列出 27 个 ppt-live 文件全 D，含 `.gitignore` / `.mjs` / `.json` / `.js` / `.css` / `.html` 各类 |
| 越界删除检测 | ✅ D 列 76 文件 100% 在 `product-domains/src/miniapp/**` 或 `product-domains/tests/**`；0 越界 |
| `tests/function_agent_contracts.rs` 是否存活 | ✅ Test-Path True，含 18 个 integration 测试运行通过 |
| `core-types/src/surface.rs` 的 `MiniApp` 死变体 | ✅ 按 recon Q7 + report §6 说明，留 M5 处理，本批不碰 |
| 并行 session 文件未被扩大 | ✅ `.opencode/model-capability-notes.md`（148+/0-）+ `memory/northhing.md`（6+/0-）均为单一加法 hunk，不涉及 miniapp 相关内容 |

---

## 4. 一句话总结论

> **M4 批通过**：76 删除 + 10 修改文件精确匹配 brief 授权清单，零越界、零夹带；Cargo.toml / Cargo.lock / i18n-audit.mjs / boundary / AGENTS 双语均与 brief 条款逐项对齐；MSVC 门禁（cargo check --workspace、--no-default-features、--features function-agents、cargo check -p northhing）全部 PASS，`rg -i miniapp scripts/core-boundaries` 归零。无 Critical/Important finding，可签收进入 M5（顶层 MiniApp/ + 契约死变体 + 文档收口）。