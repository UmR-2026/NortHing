# Review Guide Template
# 
# 使用说明：
# 1. 每次完成编码任务后，复制此模板创建新的 review 指导文件
# 2. 文件命名：docs/reviews/YYYY-MM-DD-{task-id}-{brief-description}.md
# 3. 填入本次任务的所有关键信息
# 4. 将文件路径追加到 docs/PROJECT_STATE.md 的 review 记录中
# 5. 提交时包含此文件

---

# {Task Title} — Review Guide

> **Task ID**: {task-id}  
> **Date**: YYYY-MM-DD  
> **Branch**: `v3-restructure`  
> **HEAD**: `{commit-hash}`  
> **Author**: ZCode Agent  
> **Scope**: {一句话描述本次任务范围}

---

## 1. 任务概述 (What)

### 1.1 目标
{本次任务要解决的问题或实现的功能}

### 1.2 非目标
{明确排除在本次任务之外的内容}

---

## 2. 变更清单 (Changes)

### 2.1 修改文件

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `path/to/file.rs` | 新增/修改/删除 | 具体说明 |

### 2.2 关键代码片段

```rust
// 如果有特别重要的逻辑变更，在此展示
```

---

## 3. 设计决策 (Decisions)

### 3.1 决策 1：{决策标题}
- **选项 A**: {描述}
- **选项 B**: {描述}
- **选择**: {A/B}，理由：{为什么}

### 3.2 已知妥协
- {妥协 1}：{为什么接受}

---

## 4. 测试验证 (Verification)

### 4.1 编译状态
```bash
# 运行命令
cargo check -p northhing-core --lib
cargo check --manifest-path src/apps/desktop/Cargo.toml --lib --tests

# 结果
# 错误数：0
# 警告数：{N}（已有/新增）
```

### 4.2 测试状态
| 测试套件 | 结果 | 备注 |
|----------|------|------|
| northhing-core lib | ✅/❌ | {说明} |
| desktop lib | ✅/❌ | {说明} |
| agent-dispatch lib | ✅/❌ | {说明} |

### 4.3 手动验证步骤
1. {步骤 1}
2. {步骤 2}

---

## 5. 风险与问题 (Risks)

### 5.1 已知问题
| 问题 | 严重性 | 状态 | 说明 |
|------|--------|------|------|
| {问题描述} | 高/中/低 | 已修复/待修复/已接受 | {详情} |

### 5.2 后续债务
- {债务 1}：{为什么现在不解决，计划何时解决}

---

## 6. Review 检查清单 (Checklist)

### 6.1 代码审查重点
- [ ] {检查项 1}
- [ ] {检查项 2}

### 6.2 测试审查重点
- [ ] {检查项 1}
- [ ] {检查项 2}

### 6.3 文档审查重点
- [ ] {检查项 1}

---

## 7. 后续建议 (Next Steps)

### 7.1 立即跟进（下一 session）
- {建议 1}

### 7.2 未来迭代
- {建议 2}

---

> **End of Review Guide**
> 
> 此文件由 ZCode Agent 在任务完成后自动生成，供后续 review 参考。
> 如有疑问，请查看 git log 和对应 commit diff。
