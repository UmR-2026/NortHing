# Northing 代码腐化解决指南

**版本**: v0.1.0-rev1 
**日期**: 2026-06-28 
**更新说明**: 移除 `opt-level = 0` 阻塞项（B-1 修复），更新 stream handler 修复状态，添加编译配置警告 
**适用范围**: Northing 项目所有 Rust 代码贡献者（含 AI Agent 与人类） 

---

## 一、什么是代码腐化（Code Rot）

> 代码腐化不是 Bug，而是**系统性的结构衰败**。代码还存在，但已经"烂"了— 改任何地方都会引发连锁反应。

在 Northing 的语境下，代码腐化特指 AI 驱动开发中产生的结构性退化：

| 腐化类型 | 定义 | 典型症状 |
|---------|------|---------|
| **结构腐化** | 文件/模块/函数过度膨胀 | 单文件 >1000 行，单函数 >100 行，万能对象（God Object） |
| **技术债务** | 明知有问题但暂时未修的代码 | `TODO`/`FIXME` 堆积、`#[allow(dead_code)]` 泛滥、废弃路径残留 |
| **质量腐化** | 错误处理退化 | `unwrap()` 爆炸、`let _ = Result` 丢弃、生产代码 `panic!`/`unreachable!` |
| **依赖腐化** | 编译产物和依赖关系失控 | `target/` 膨胀、循环依赖、版本— 突、边界泄露 |
| **上下文腐化** | 代码超出模型理解阈值 | 单文件 >5000 行时，AI 理解精度下降 30-50%，错误放置新逻辑 |

---

## 二、腐化检测方法

### 2.1 快速扫描脚本

```bash
#!/bin/bash
# 保存为 scripts/code-rot-scan.sh

echo "=== 文件膨胀检测 ==="
find src/ -name "*.rs" -not -path "*/target/*" -not -path "*/tests/*" -exec wc -l {} + | sort -rn | head -20

echo "=== unwrap() 计数（生产代码） ==="
grep -rn 'unwrap(' src/ --include='*.rs' | grep -v '/tests/' | grep -v 'test_' | grep -v '#\[cfg(test)\]' | wc -l

echo "=== let _ = Result 丢弃计数 ==="
grep -rn 'let _ = ' src/ --include='*.rs' | grep -v '/tests/' | grep -v 'test_' | grep -v '#\[cfg(test)\]' | wc -l

echo "=== panic! 生产代码 ==="
grep -rn 'panic!' src/ --include='*.rs' | grep -v '/tests/' | grep -v 'test_' | grep -v '#\[cfg(test)\]' | wc -l

echo "=== unreachable! 生产代码 ==="
grep -rn 'unreachable!' src/ --include='*.rs' | grep -v '/tests/' | grep -v 'test_' | grep -v '#\[cfg(test)\]' | wc -l

echo "=== dead_code 允许 ==="
grep -rn '#\[allow(dead_code)\]' src/ --include='*.rs' | wc -l

echo "=== TODO/FIXME 遗留 ==="
grep -rn 'TODO\|FIXME\|HACK\|XXX' src/ --include='*.rs' | grep -v '/tests/' | wc -l

echo "=== target/ 大小 ==="
du -sh target/debug 2>/dev/null || echo "no target/debug"
du -sh target/release 2>/dev/null || echo "no target/release"
```

### 2.2 健康度阈值（红线）

| 指标 | 🟢 健康 | 🟡 警告 | 🔴 腐化 |
|------|--------|--------|--------|
| 单文件行数 | <500 | 500-1000 | **>1000** |
| 单函数行数 | <50 | 50-100 | **>100** |
| 文件 `unwrap()` | 0 | 1-5 | **>5** |
| `let _ = Result` | 0 | 1-3 | **>5** |
| 生产 `panic!` | 0 | 0 | **>0** |
| 生产 `unreachable!` | 0 | 0 | **>0** |
| `dead_code` 允许 | 0 | 1-5 | **>10** |
| `target/debug` 大小 | <1GB | 1-10GB | **>10GB** |

---

## 三、腐化修复策略（已验证）

### 3.1 本次修复清单（2026-06-28 并行 Agent 修复）

| # | 修复项 | 文件数 | 修改量 | 方法 | 状态 |
|---|--------|--------|--------|------|------|
| 1 | **信号量 unwrap** | 1 | 7 处 | `sem.acquire().await.map_err(...)— ` | ✅ 已修复 |
| 2 | **unreachable! 清理** | 4 | 4 处 | `warn!` + 合理 fallback / `Err` | ✅ 已修复 |
| 3 | **theme.rs expect** | 1 | 1 处 | `unwrap_or_else` + `Default` fallback | ✅ 已修复 |
| 4 | **`let _ = Result` 清理** | 5 | 75 处 | `if let Err(e) = ... { warn!(...); }` | ✅ 已修复 |
| 5 | **编译配置** | 3 | 追加 | `sccache` + `split-debuginfo` + `target-dir` + `exclude` 修正 | ✅ 已修复 |
| 6 | **编译阻塞 B-1** | 1 | 删除 | 移除 `.cargo/config.toml` 中的 `opt-level = 0` + `[profile.dev]` 重复定义 | ✅ 已修复 |
| 7 | **stream handler let _ =** | 1 | 13 处 | `responses.rs` 13 处 `tx_event.send` 改为 warn logging | ✅ 已修复 |

### 3.2 按腐化类型的修复方法

#### A. `unwrap()` 修复流程

**决策树**：

```
遇到 unwrap()
 ├── 是否绝对不可能失败？（如编译期 include_str! 解析）
 │ ├── 是 → 保留 unwrap()，但添加注释 // Invariant: ...
 │ └── 否 → 继续
 ├── 函数是否返回 Result？
 │ ├── 是 → 改为 `.map_err(|e| Error::...)— `
 │ └── 否 → 继续
 ├── 是否有合理的 Default 值？
 │ ├── 是 → `.unwrap_or_else(|| { warn!(...); Default::default() })`
 │ └── 否 → 继续
 └── 是否可以忽略失败？
 ├── 是 → `if let Err(e) = ... { warn!(...); }`
 └── 否 → 修改函数签名返回 Result
```

**参考修复**（insights/service.rs）：
```rust
// 修复前
let _permit = sem_1.acquire().await.unwrap();

// 修复后
let _permit = sem_1
 .acquire()
 .await
 .map_err(|e| NortHingError::service(format!("Semaphore error: {}", e)))— ;
```

#### B. `let _ = Result` 修复流程

**分类处理**：

| 类型 | 示例 | 修复方式 |
|------|------|---------|
| 事件发送 | `let _ = event_bus.send(...);` | `if let Err(e) = event_bus.send(...) { warn!("Event send failed: {}", e); }` |
| 日志/IO 写入 | `let _ = file.write_all(...);` | `if let Err(e) = file.write_all(...) { warn!("Write failed: {}", e); }` |
| 信号量释放 | `let _ = sem.release();` | 保留 `let _ =` + 注释 `// intentionally ignored: best-effort cleanup` |
| 平台初始化 | `let _ = CoInitializeEx(...);` | 保留 `let _ =` + 注释 `// intentionally ignored: may already be initialized` |
| 未使用变量抑制 | `let _ = reason;` | 保留 `let _ =` + 注释 `// intentionally ignored: suppresses unused warning on non-windows` |

#### C. `unreachable!()` 修复流程

**原则**：生产代码中 `unreachable!()` 是**傲慢的断言**。改为 `warn!` + 降级处理。

**参考修复**（write_file.rs）：
```rust
// 修复前
WriteLocalFileStatus::AlreadyExistsSameContent => unreachable!(),

// 修复后
WriteLocalFileStatus::AlreadyExistsSameContent => {
 return Ok(WriteLocalFileOutcome {
 status,
 bytes_written: 0,
 lines_written: 0,
 // ... 其他字段
 });
}
```

#### D. God Object 拆分流程

以 `session_manager.rs`（6532 行）为例：

```
session_manager.rs (6532 行)
 ├── 职责1: 会话生命周期管理 → session_lifecycle.rs
 ├── 职责2: 会话恢复 → session_restore.rs (已存在，整合)
 ├── 职责3: 持久化 → session_persistence.rs (已存在，整合)
 ├── 职责4: 证据管理 → session_evidence.rs (已存在)
 ├── 职责5: 压缩 → compression/compressor.rs (已存在)
 ├── 职责6: 配置管理 → session_config.rs (新增)
 ├── 职责7: 清理/过期 → session_cleanup.rs (新增)
 └── 职责8: 统计/限额 → session_usage.rs (已存在)
 
保留: session_manager.rs (facade, ~200 行)
 └── 使用 pub use 重新导出，保持 API 兼容
```

**拆分步骤**：
1. 识别 `SessionManager` 的 `impl` 块，按职责分组
2. 新建 `src/agentic/session/session_lifecycle.rs` 等文件
3. 将对应的 `fn` 迁移到新文件
4. 在新文件顶部 `use crate::agentic::session::SessionManager;`
5. 在 `mod.rs` 中添加 `pub mod session_lifecycle;`
6. 在 `SessionManager` 的 `impl` 中保留调用新模块的 facade 方法
7. 编译验证 → 运行测试 → 提交

#### E. 编译产物清理

```bash
# 单次清理
cargo clean

# 长期配置（.cargo/config.toml）
[build]
rustc-wrapper = "sccache" # 需要 cargo install sccache
target-dir = "target-shared" # 避免多仓库重复编译

# profile.dev 统一在 Cargo.toml 中定义，不要在 .cargo/config.toml 中重复
# .cargo/config.toml 只保留 [build]、[target.*]、[alias] 等全局配置
```

**根 `Cargo.toml` 中的 `[profile.dev]`**（唯一定义处）：
```toml
[profile.dev]
split-debuginfo = "packed" # 将 .pdb 打包，减少增量文件
incremental = true
```

**⚠️ 警告**：`opt-level = 0` 在 dev profile 中会导致 `ring` 和 `aws-lc-sys` 的 C 代码编译失败。不要设置 `opt-level = 0`。默认 `opt-level = 0`（未声明时）在 Cargo 中等于 0，但 Cargo 的默认实现与显式声明 `-O0` 不同— 显式声明会传递给 cc 的 CFLAGS，破坏 C 依赖编译。如果需要降低编译时间，使用 `incremental = true` + `split-debuginfo = "packed"` 即可。

---

## 四、腐化预防机制

### 4.1 提交前检查（Pre-commit Hooks）

```bash
#!/bin/bash
# 保存为 .git/hooks/pre-commit

# 1. 文件大小检查
MAX_LINES=1000
for file in $(git diff --cached --name-only | grep '\.rs$'); do
 lines=$(wc -l < "$file")
 if [ "$lines" -gt "$MAX_LINES" ]; then
 echo "ERROR: $file has $lines lines (max $MAX_LINES)"
 echo "Split before committing. See docs/code-rot-prevention-guide.md"
 exit 1
 fi
done

# 2. unwrap 增量检查（不阻止，但警告）
NEW_UNWRAPS=$(git diff --cached | grep -c '^+.*unwrap(' || true)
if [ "$NEW_UNWRAPS" -gt 3 ]; then
 echo "WARNING: $NEW_UNWRAPS new unwrap() added. Review before commit."
fi

# 3. panic/unreachable 零容忍
NEW_PANICS=$(git diff --cached | grep -c '^+.*panic!\|^+.*unreachable!' || true)
if [ "$NEW_PANICS" -gt 0 ]; then
 echo "ERROR: $NEW_PANICS panic! or unreachable! in production code."
 echo "Replace with error propagation or warn + fallback."
 exit 1
fi
```

### 4.2 模块代谢周期（Metabolic Cycle）

```markdown
每次提交时执行：
 1. 如果新增文件 >500 行 → 触发拆分审查
 2. 如果新增 unwrap() >3 处 → 触发错误处理审查
 3. 如果新增 `#[allow(dead_code)]` → 必须附带说明注释

每月执行：
 1. `cargo clean` → 释放编译产物
 2. `scripts/code-rot-scan.sh` → 生成健康度报告
 3. 审查 Top 5 最大文件 → 评估拆分优先级

每季度执行：
 1. 审查 `#[allow(dead_code)]` 标记 → 清理超过 3 个月未触发的代码
 2. 审查 `TODO`/`FIXME` → 转化为 Issue 或删除
 3. 审查旧 Agent 路径 → 确认是否可删除（如 Phase1/2/3）
```

### 4.3 AI Agent 工作约束

当 AI Agent 生成代码时，强制遵循以下规则：

```markdown
## AI 生成代码约束（写入 .agents/instructions/code-style.md）

1. 文件大小
 - 新增文件不得超过 500 行
 - 如果超过，必须拆分为 2+ 个文件，并说明拆分理由

2. 函数大小
 - 单函数不得超过 50 行
 - 如果超过，必须提取子函数，并命名子函数职责

3. 错误处理（零容忍）
 - 禁止使用 `unwrap()` 除非在 `#[cfg(test)]` 或编译期不变量（附注释）
 - 禁止 `let _ = result;` 除非附 `// intentionally ignored: ...` 注释
 - 禁止生产代码 `panic!`/`unreachable!`，使用 `Result` 或 `warn!` + fallback

4. 死代码（零容忍）
 - 禁止新增 `#[allow(dead_code)]` 无说明
 - 如果必须保留，注释说明：保留原因、预计何时移除、谁负责

5. 工具暴露
 - 新增工具必须考虑是否纳入 `ToolExposure::Collapsed`
 - 新增子 Agent 必须评估是否纳入 `BackgroundSubagentStartResult`

6. 上下文管理
 - 新增 `SessionRelationship` 类型必须考虑持久化策略
 - 新增背景任务必须评估结果投递可靠性（fire-and-forget vs queue）
```

---

## 五、本次修复后遗留问题

| # | 遗留问题 | 严重程度 | 预计修复版本 | 状态 |
|---|---------|---------|-------------|------|
| 1 | `session_manager.rs` 6532 行拆分 | 🔴 P0 | v0.1.1 | 待分配 |
| 2 | `review_platform/mod.rs` 4866 行拆分 | 🔴 P0 | v0.1.1 | 待分配 |
| 3 | 旧 Phase 路径删除（3 结构体 + phase1/2/3） | 🔴 P0 | v0.1.1 | 待分配 |
| 4 | `map_subagent_result_to_lightweight` 删除 | 🟠 P1 | v0.1.1 | 待分配 |
| 5 | `computer_use_input.rs` + `browser_launcher.rs` 删除 | 🟠 P1 | v0.1.1 | 待分配 |
| 6 | 边界泄露（97 处 → 62 处 CLI 引用） | 🟠 P1 | v0.2.0 | 待分配 |
| 7 | `installer` 依赖版本统一 | 🟠 P1 | v0.1.1 | 待分配 |
| 8 | stream handler `let _ =` 不一致（anthropic.rs 12 / openai.rs 11 / gemini.rs 8） | 🟠 P1 | v0.1.1 | 本轮已修复 responses.rs，其余 3 文件待修复 |
| 9 | `unwrap()` 总数从 518 降至 <100 | 🟡 P2 | v0.2.0 | 持续 |
| 10 | `let _ = Result` 从 526 降至 <100 | 🟡 P2 | v0.2.0 | 持续 |
| 11 | 测试覆盖率从 27% 提升至 50% | 🟡 P2 | v0.2.0 | 持续 |

### 新增已知问题（本轮审查发现）

- **B-1 `opt-level = 0` 编译阻塞**（✅ 已修复）：`.cargo/config.toml` 中 `opt-level = 0` 导致 `ring`/`aws-lc-sys` C 代码编译失败。修复方式：从 `.cargo/config.toml` 删除 `[profile.dev]` 节，统一在根 `Cargo.toml` 中定义。详见 `docs/handoffs/2026-06-28-working-tree-review.md`。
- **stream handler 修复不一致**：`responses.rs` 已修复 13 处 `let _ = tx_event.send(...)`，但 `anthropic.rs`(12)、`openai.rs`(11)、`gemini.rs`(8) 仍使用静默丢弃。应在同轮 commit 中修复或注明 "Part 1 of 4"。
- **临时文件未清理**：`_rot_scan.py`、`_rot_scan.txt` 应删除或加入 `.gitignore`；`target-shared/` 应加入 `.gitignore`。

---

## 六、附录：审核工具

### 6.1 已完成的审核报告

| 报告 | 日期 | 维度 | 状态 |
|------|------|------|------|
| `research/audit_dim01.md` | 2026-06-27 | 结构腐化 | 基线 |
| `research/audit_dim02.md` | 2026-06-27 | 技术债务 | 基线 |
| `research/audit_dim03.md` | 2026-06-27 | 质量与测试 | 基线 |
| `research/audit_dim04.md` | 2026-06-27 | 依赖与编译 | 基线 |
| `research/audit_redim01.md` | 2026-06-27 | 结构腐化重审 | 重审 v1 |
| `research/audit_redim02.md` | 2026-06-27 | 技术债务重审 | 重审 v1 |
| `research/audit_redim03.md` | 2026-06-27 | 质量与测试重审 | 重审 v1 |
| `research/audit_redim04.md` | 2026-06-27 | 依赖与编译重审 | 重审 v1 |
| `research/audit_redim_v3_01.md` | 2026-06-28 | 结构腐化第三次 | 重审 v2 |
| `research/audit_redim_v3_02.md` | 2026-06-28 | 技术债务第三次 | 重审 v2 |
| `research/audit_redim_v3_03.md` | 2026-06-28 | 质量与测试第三次 | 重审 v2 |
| `research/audit_redim_v3_04.md` | 2026-06-28 | 依赖与编译第三次 | 重审 v2 |

### 6.2 快速自检命令

```bash
# 每天执行一次
bash scripts/code-rot-scan.sh | tee research/health-$(date +%Y%m%d).log

# 对比上次
bash scripts/code-rot-scan.sh | diff research/health-$(date -d yesterday +%Y%m%d).log -
```

---

**指南维护者**: Northing 维护团队 
**更新策略**: 每轮代码腐化修复后更新 
**审核触发**: 文件行数 >1000、新增 unwrap() >3、新增 panic/unreachable >0
