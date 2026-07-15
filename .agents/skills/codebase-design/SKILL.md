---
name: codebase-design
description: "Use when designing Rust module boundaries, crate interfaces, or trait hierarchies. Provides a shared vocabulary for architecture: depth, seam, adapter, leverage, locality. Trigger this when planning new v3 phases, designing const flag placement, or reviewing module structure."
---

# Codebase Design (northhing v3 适配— > 来源：mattpocock/skills codebase-design

## When to trigger

- 设计新的 const flag 应该放在哪个 crate/module 层级
- 规划 v3 后续 phase 的模块改— - 审查 trait 设计— crate 边界
- 判断某个模块是否"足够— ## 核心词汇— | 术语 | 含义 | Rust 对应 |
|------|------|-----------|
| **Module** | 一个功能单— | Rust module / crate |
| **Interface** | 外部使用— API— | `pub fn` / `pub trait` / `pub struct` |
| **Implementation** | 接口背后的逻辑 | `impl` block / private items |
| **Depth** | 接口— + 实现— = 好模— | pub items— + 内部逻辑复杂 |
| **Seam** | 可以切换实现的地— | trait boundary / const flag if/else |
| **Adapter** |— seam 的转换层 | `impl TraitA for StructB` |
| **Leverage** | 小改— 大效— | 改一— const flag— 影响整个 prompt loading |
| **Locality** | 相关代码放在一— | 同一 crate / 同一 module |

## 设计原则

### 1. 追求深模— (Depth)

**好设— *：小接口 + 大实— ```rust
// GOOD: 深模— 1— pub fn，内部逻辑复杂
pub fn load_prompt() -> String {
 if USE_MEMORY_SKILL_POINTER {
 load_skill_pointer() // 内部有复杂逻辑
 } else {
 load_inline_block() // 内部有复杂逻辑
 }
}

// BAD: 浅模— 接口暴露了太多内部细— pub fn load_skill_pointer() -> String { ... }
pub fn load_inline_block() -> String { ... }
pub fn decide_which_path(flag: bool) -> Path { ... }
pub fn read_skill_file(path: Path) -> String { ... }
// 调用者需要理解全— 4 个函数的关系
```

### 2. const flag— Seam

const flag— if/else 是一— **seam**— 一个可以切换实现的点— 好的 seam 位置— -— 函数入口处（调用者无感知— -— 模块边界处（上层不需要知道底层用哪条路径— -— 函数中间（代码可读性差— -— 散布在多处（难以追踪所有分支）

### 3. 接口即测试面

一个模块的 pub items 就是它的测试面。如— pub items 太多，测试需要覆盖的路径就太多— **规则**：如果删除一— pub fn 后测试不受影响，说明它可能不需— pub— ### 4. 一— Adapter = 一个假— Seam

当你— `impl TraitA for StructB`，你在说 "这里有一— seam，TraitA 的实现可以替— v3— const flag— if/else 也是同一个概念— 一个手工实现的 adapter— ### 5. Locality：相关代码放一— const flag 的定义应该放在：
-— 使用它的函数旁边（同文件顶部— -— 集中放在 config 文件中（违反 locality— ## DESIGN-IT-TWICE 模式

当设计一— const flag 的接口时，先设计两个方案再比较：

**方案 A**— ```rust
const USE_X: bool = true;
fn load() -> String {
 if USE_X { short_path() } else { long_path() }
}
```

**方案 B**— ```rust
const USE_X: bool = true;
fn load() -> String {
 let data = if USE_X { get_pointer() } else { get_inline() };
 format_output(data) // 共享后处— }
```

比较：方— B— seam 提前了，共享— format_output。如— format_output 的逻辑在两条路径中相同，方— B— DRY— ## DEEPENING 检— 对现有模块做 deepening 检查：

1. **找浅模块**：pub items / impl lines 比— > 0.3 的模块可能太— 2. **— friction**：调用者需要理解模块内部才能正确使用的— 接口不够— 3. **找重— *：两个模块的 impl 有大段相似代— 可以抽取共享的深模块

##— v3 工作流的整合— `northhing-v3-workflow`— Step 1（设— const flag）时— 1.— DESIGN-IT-TWICE 设计至少两个放置方案
2. 选择 deepening 更好的方— 3.— code-review 时用 depth/seam 词汇评估

## 与其— skill 的关— - **northhing-v3-workflow**: 提供 const flag 放置的架构指— - **code-review**: 审查时用 depth/seam 词汇评估
- **brainstorming**: 设计探索阶段使用这些概念讨论方案
