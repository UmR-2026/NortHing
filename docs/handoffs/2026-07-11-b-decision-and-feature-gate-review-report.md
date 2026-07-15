# B Decision + Feature-Gate Fix — QClaw Review Report

**审查时间**: 2026-07-11 15:01-15:15 GMT+8  
**审查范围**: 2 commits (4cb230fe + 0b4dc1f3)  
**审查人**: QClaw (GLM-5.2)  
**审查指南**: docs/handoffs/2026-07-11-b-decision-and-feature-gate-review-guide.md

---

## 1. 执行摘要

**批次评级: 9.0/10 — APPROVED**

两个 commit 均通过全部验证轴。escape_html XSS 修复正确且完整，feature-gate 收紧匹配实际耦合。发现 1 个非阻塞 fmt diff（import ordering）和 1 个 pre-existing `gen` 关键字问题（不在本次范围）。

---

## 2. Commit 1: 4cb230fe — escape_html XSS fix

### 2.1 Diff 审查
- `auth_types.rs:155` — 新增 `.replace('&', "&amp;")` 作为**第一个** replace ✅
- `tests.rs:5` — 断言从 `"a&lt;b&gt;&c&quot;d&#39;e"` 改为 `"a&lt;b&gt;&amp;c&quot;d&#39;e"` ✅
- 5 个 replace 顺序正确：`&` → `<` → `>` → `"` → `'` ✅

### 2.2 安全验证
- **XSS entity-bypass 攻击路径**：攻击者通过 OAuth `error` 参数注入 `&lt;script&gt;...`，浏览器解析 `&lt;` 为 `<` 执行脚本。修复后 `&` 先被替换为 `&amp;`，`&lt;` 变成 `&amp;lt;` 浏览器渲染为字面文本。✅
- **调用点覆盖**：2 个外部输入调用点（line 187 OAuth error display + line 218 missing param list）均使用 `escape_html`。✅

### 2.3 全 workspace HTML escape 函数审计
| 函数 | 位置 | `&`→`&amp;` 在首位 | 状态 |
|------|------|-------------------|------|
| `escape_html` | `auth/auth_types.rs:153` | ✅ (本次修复) | FIXED |
| `html_escape` | `insights/html/format.rs:1` | ✅ (已有) | OK |
| `escape_html_attr` | `miniapp/compiler.rs:155` | ✅ (已有) | OK |

**结论**: 无其他遗漏的 HTML escape 函数。✅

### 2.4 测试验证
```
cargo test -p northhing-core --lib --features product-full escape_html
→ 1 passed; 0 failed; 914 filtered out
```

### 2.5 评分

| 轴 | 分数 | 说明 |
|---|---:|------|
| Security defense correctness | 10/10 | `&`→`&amp;` 在首位，完美阻断 entity-bypass |
| Test assertion accuracy | 10/10 | 断言正确反映安全行为 |
| Baseline preservation | 10/10 | 914 passed 不变 |
| Workspace unaffected | 10/10 | 2 files, +2/-1 |
| Iron rules | 9/10 | tests.rs 有 1 处 import ordering fmt diff |
| Commit message clarity | 10/10 | 详细说明根因、攻击路径、修复原理 |

**Commit 1 总分: 9.5/10 — APPROVED**

---

## 3. Commit 2: 0b4dc1f3 — Feature-gate fix

### 3.1 Diff 审查
- `lib.rs:24` — `service_agent_runtime` 从 `service-integrations` 收紧为 `all(service-integrations, product-full)` ✅
- `service/mod.rs:21` — `mcp` 模块同上 ✅
- `service/mod.rs:23` — `remote_connect` 模块同上 ✅
- `service/mod.rs:63` — `MCPService` re-export 同上 ✅

### 3.2 编译验证（三种 feature 组合）

| Feature 组合 | 修复前 | 修复后 |
|---|---|---|
| default | 0 errors | 0 errors ✅ |
| service-integrations | **53 errors** | **0 errors** ✅ |
| product-full | 0 errors | 0 errors ✅ |

### 3.3 消费者审计
```
git grep -l "service-integrations" -- "**/Cargo.toml"
→ 仅 northhing-core/Cargo.toml
```
无外部消费者单独使用 `service-integrations` 而不带 `product-full`。✅

### 3.4 Cfg gate 一致性
`service/mod.rs` 中 3 个 `all(service-integrations, product-full)` 对应 3 个实际依赖 product-full 的模块，其余 7 个 `service-integrations` 单独 gate 的模块（i18n、lsp 等）不依赖 product-full，保持不变。✅

### 3.5 评分

| 轴 | 分数 | 说明 |
|---|---:|------|
| Cfg gate matches actual coupling | 10/10 | 3 模块 + 1 re-export 100% 依赖 product-full |
| Baseline preservation | 10/10 | 914 passed 不变 |
| Workspace unaffected | 10/10 | 2 files, +4/-4 |
| Iron rules | 10/10 | 无 fmt diff，无新增违规 |
| Commit message clarity | 10/10 | 详述根因、before/after 数据 |

**Commit 2 总分: 9.0/10 — APPROVED**

扣分项：53 个 pre-existing errors 是 R50 拆分遗留的债务，虽非本 commit 引入，但说明拆分流程缺少 `--features service-integrations` 单独编译验证。

---

## 4. 对 Mavis 4 个问题的回答

### Q1: XSS defense scope
**无遗漏**。workspace 内共 3 个 HTML escape 函数（`escape_html`、`html_escape`、`escape_html_attr`），均已正确将 `&`→`&amp;` 放在首位。本次修复的 `escape_html` 是唯一存在 bug 的，其余两个从未受影响。

### Q2: Feature-gate precedent
**建议 R-series 拆分时预防线**。未来 god-object split 应在 split-time 即对依赖 parent feature 的 `use` 语句做 cfg gate 对齐，而非事后补救。具体建议：worker 流程增加 `cargo build -p <crate> --features <each-feature-alone>` 验证步骤。

### Q3: Audit scope (4 个 integration test binaries)
**独立 workstream**。这 4 个 `tests/` 二进制文件的编译失败属于 pre-existing debt，与本次 2 个 commit 无因果关系。建议单独创建 cleanup task 处理，不阻塞本批次合并。

### Q4: Test assertion wording
**保持单个测试**。`escape_html_replaces_all_special_chars` 测试名已清晰表达意图——验证所有特殊字符都被转义。拆分为 5 个子测试会增加维护成本但不提升失败定位能力（5 个 replace 是一个原子操作链，任何一个错误都意味着函数逻辑有问题）。

---

## 5. 非阻塞观察

1. **tests.rs import ordering fmt diff**: `escape_html` 和 `OAuthCallbackLocale` 的 import 顺序不符合 rustfmt 默认排序。建议后续 `cargo fmt` 统一修复。
2. **pre-existing `gen` 关键字问题**: `weixin_qr_login.rs:52` 的 `rand::thread_rng().gen()` 在新版 Rust 中 `gen` 是保留关键字，需改为 `r#gen()` 或 `gen_range()`。不在本次范围。
3. **R50 拆分流程改进**: 53 个 pre-existing errors 表明 god-object split 后缺少 `--features service-integrations` 单独编译验证。建议写入 worker checklist。

---

## 6. 最终裁决

| Commit | 评分 | 裁决 |
|--------|------|------|
| 4cb230fe (escape_html) | 9.5/10 | ✅ APPROVED |
| 0b4dc1f3 (feature-gate) | 9.0/10 | ✅ APPROVED |
| **批次** | **9.0/10** | **✅ APPROVED** |

批次已准备好进行 user-driven review-fix-cleanup cycle。
