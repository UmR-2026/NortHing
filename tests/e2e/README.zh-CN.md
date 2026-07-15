**中文** | [English](README.md)

# northhing E2E 测试

使用 WebDriverIO + northhing 内置 WebDriver— E2E 测试框架— > 完整文档请参— [E2E-TESTING-GUIDE.zh-CN.md](E2E-TESTING-GUIDE.zh-CN.md)

## 快速开— ### 1. 安装依赖

```bash
# 构建 debug 应用
cargo build -p northhing-desktop

# 安装测试依赖
pnpm --dir tests/e2e install
```

### 2. 运行测试

```bash
# L0 冒烟测试 (最— pnpm --dir tests/e2e run test:l0
pnpm --dir tests/e2e run test:l0:all

# L1 功能测试
pnpm --dir tests/e2e run test:l1

# 运行所有测— pnpm --dir tests/e2e test
```

## 测试级别

| 级别 | 目的 | 运行时间 | AI需— |
|------|------|----------|--------|
| L0 | 冒烟测试 - 验证基本功能 | < 1分钟 | 不需— |
| L1 | 功能测试 - 验证功能特— | 5-15分钟 | 不需— mock) |
| L2 | 规划中，暂未实现 | N/A | N/A |

## 目录结构

```
tests/e2e/
├── specs/ # 测试用例
├── page-objects/ # Page Object 模型
├── helpers/ # 辅助工具
├── fixtures/ # 测试数据
└── config/ # 配置文件
```

## 常见问题

### 内置 WebDriver 未就— 测试启动器会直接拉起 northhing，并等待 `127.0.0.1:4445` 上的内置 WebDriver 服务就绪— ### 应用未构— ```bash
cargo build -p northhing-desktop
```

### 测试超时

Debug 构建启动较慢，可在配置中调整超时时间— ## 更多信息

- [完整测试指南](E2E-TESTING-GUIDE.zh-CN.md) - 测试编写规范、最佳实践、测试计— - [northhing 项目结构](../../AGENTS.md)
