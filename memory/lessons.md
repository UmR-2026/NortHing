---
name: northhing-lessons
type: knowledge
domain: T
tags: [analysis, review, security, ux]
---

# northhing 关键教训 (2026-07-16)

## 用户旅程 P0 阻断

1. installer 后端空文件 → 装包构建失败
2. 配置双写不同步 → 桌面的 key agent 不读
3. 引导流程死路 → pick-folder 无 handler
4. 事件桥缺失 → 发消息后 UI 永远不更新
5. 移动端入口不存在

## 安全高危

- agent 默认 skip_tool_confirmation=true 可删任意文件
- shell 拒绝名单可被绕过
- API key 明文存两处
- 配置非原子写

## AI 幻觉

- flashgrep 94.6%/36.1× 无 benchmark 数据
- 97%+ vibe coding 无依据
- "长期记忆"功能实际不存在
- "文档协作"功能不存在

## 代码质量

- 932/933 测试通过
- 0 god-files
- MSVC 构建成功
