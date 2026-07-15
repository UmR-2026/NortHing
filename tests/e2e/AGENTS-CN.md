[English](AGENTS.md) | **中文**

# AGENTS.md

## 范围

本文件适用于 `tests/e2e`。仓库级规则请使用顶层 `AGENTS.md`。

## 此处重点

桌面端 E2E 测试使用 WebDriverIO 加上 northhing 内置的 WebDriver 实现。

`E2E-TESTING-GUIDE.md` 中定义的层级：

- L0：冒烟测试
- L1：功能测试
- L2：计划中，尚未实现

核心规则：

1. 测试真实的用户工作流
2. 使用 `data-testid` 作为稳定的选择器
3. 采用 Page Object Model
4. 保持测试独立且幂等

## 命令

```bash
cargo build -p northhing-desktop
pnpm --dir tests/e2e install
pnpm --dir tests/e2e run test:l0
pnpm --dir tests/e2e run test:l0:all
pnpm --dir tests/e2e run test:l1
pnpm --dir tests/e2e exec wdio run ./config/wdio.conf.ts --spec "./specs/<file>.spec.ts"
```

## 验证

优先使用范围最窄的相关 spec；只在必要时再扩大范围。
