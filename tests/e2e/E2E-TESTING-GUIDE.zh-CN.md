**中文** | [English](E2E-TESTING-GUIDE.md)

# northhing E2E 测试指南

使用 WebDriverIO + northhing 内嵌 WebDriver 进行 northhing 项目的端到端测试完整指南— ## 目录

- [测试理念](#测试理念)
- [测试级别](#测试级别)
- [快速开始](#快速开— [测试结构](#测试结构)
- [编写测试](#编写测试)
- [最佳实践](#最佳实— [问题排查](#问题排查)

## 测试理念

northhing E2E 测试专注— *用户旅程**— *关键路径**，确保桌面应用从用户角度正常工作。我们使用分层测试方法来平衡覆盖率和执行速度— ### 核心原则

1. **测试真实的用户工作流**，而不是实现细— 2. **使用 data-testid 属— *确保选择器稳— 3. **遵循 Page Object 模式**提高可维护— 4. **保持测试独立**和幂等— 5. **快速失— *并提供清晰的错误信息

## 测试级别

northhing 使用三级测试分类系统— ### L0 - 冒烟测试（关键路径）

**目的**：验证基本应用功能；必须在任何发布前通过— **特点**— 运行时间— 2 分钟
- 不需— AI 交互和工作区
- 可在 CI/CD 中运— 测试验证 UI 元素存在且可访问

**何时运行**：每次提交、合并前、发布前

**测试文件**— | 测试文件 | 验证内容 |
|----------|----------|
| `l0-smoke.spec.ts` | 应用启动、DOM结构、Header可见性、无严重JS错误 |
| `l0-open-workspace.spec.ts` | 工作区状态检测（启动— vs 工作区）、启动页交互 |
| `l0-open-settings.spec.ts` | 设置按钮可见性、设置面板打开/关闭 |
| `l0-navigation.spec.ts` | 工作区打开时侧边栏存在、导航项可见可点— |
| `l0-tabs.spec.ts` | 文件打开时标签栏存在、标签页正确显示 |
| `l0-theme.spec.ts` | 根元素主题属性、主题CSS变量、主题系统功— |
| `l0-i18n.spec.ts` | 语言配置、国际化系统功能、翻译内— |
| `l0-notification.spec.ts` | 通知服务可用、通知入口在header中可— |
| `l0-observe.spec.ts` | 手动观察测试 - 保持窗口打开60秒用于检— |

### L1 - 功能测试（特性验证）

**目的**：验证主要功能端到端工作，包含真实的UI交互— **特点**— 运行时间— 5 分钟
- 工作区已自动打开（测试在实际工作区上下文中运行）
- 不需— AI 模型（测— UI 行为，而非 AI 响应— 测试验证实际用户交互和状态变— **何时运行**：特性合并前、每晚构建、发布前

**测试文件**— | 测试文件 | 验证内容 |
|----------|----------|
| `l1-ui-navigation.spec.ts` | Header组件、窗口控制（最小化/最大化/关闭）、窗口状态切— |
| `l1-workspace.spec.ts` | 工作区状态检测、启动页 vs 工作区UI、窗口状态管— |
| `l1-chat-input.spec.ts` | 聊天输入、多行输入（Shift+Enter）、发送按钮状态、消息清— |
| `l1-navigation.spec.ts` | 导航面板结构、点击导航项切换视图、当前项高亮 |
| `l1-file-tree.spec.ts` | 文件树显示、文件夹展开/折叠、文件选择、在编辑器中打开文件 |
| `l1-editor.spec.ts` | Monaco编辑器显示、文件内容、标签栏、多标签切换/关闭、未保存标记 |
| `l1-terminal.spec.ts` | 终端容器、xterm.js显示、键盘输入、终端输— |
| `l1-git-panel.spec.ts` | Git面板显示、分支名、变更文件列表、提交输入、差异查— |
| `l1-settings.spec.ts` | 设置按钮、面板打开/关闭、设置标签、配置输— |
| `l1-session.spec.ts` | 会话场景、侧边栏会话列表、新建会话按钮、会话切— |
| `l1-dialog.spec.ts` | 模态遮罩、确认对话框、输入对话框、对话框关闭（ESC/背景— |
| `l1-chat.spec.ts` | 消息列表显示、消息发送、停止按钮、代码块渲染、流式指示器 |

### L2 - 集成测试（完整系统）

**目的**：验证完整工作流程与真实 AI 集成— **特点**— 运行时间— 5-60 分钟
- 需— AI 提供商配— **何时运行**：发布前、手动验— **当前状— *：L2 测试尚未实现

**计划测试文件**— | 测试文件 | 验证内容 | 状— |
|----------|----------|------|
| `l2-ai-conversation.spec.ts` | 完整AI对话流程 | 未实— |
| `l2-tool-execution.spec.ts` | 工具执行（Read、Write、Bash— | 未实— |
| `l2-multi-step.spec.ts` | 多步骤用户旅— | 未实— |

## 快速开— ### 1. 前置条件

安装必需的依赖：

```bash
# 安装 E2E 测试依赖
cd tests/e2e
pnpm install

# 构建应用（从项目根目录）
cd ../..
cargo build -p northhing-desktop
```

### 2. 验证安装

检查应用二进制文件是否存在— **Windows**: `target/debug/northhing-desktop.exe`
**Linux/macOS**: `target/debug/northhing-desktop`

### 3. 运行测试

```bash
# — tests/e2e 目录— # 运行 L0 冒烟测试（最快）
pnpm run test:l0

# 运行所— L0 测试
pnpm run test:l0:all

# 运行 L1 功能测试
pnpm run test:l1

# 运行特定测试文件
pnpm test -- --spec ./specs/l0-smoke.spec.ts
```

### 4. 测试运行模式（Release vs Dev— 测试框架统一运行— debug/dev 模式— #### Debug 模式（默认）
- **应用路径**: `target/debug/northhing-desktop.exe`
- **特点**: 包含调试符号、需— dev server（端— 1422— **使用场景**: 本地开发、快速迭— **如何识别当前使用的模— *— 运行测试时，查看输出的前几行— ```bash
# Debug 模式输出
application: <PROJECT_ROOT>\target\debug\northhing-desktop.exe
Debug build detected, checking dev server...
```

**核心原理**: 功能 E2E 默认使用 `target/debug/northhing-desktop.exe`。性能 E2E 需要显式设— `northhing_E2E_APP_MODE=release-fast`，并先执— `pnpm run desktop:build:release-fast`— 不要直接手工启动 `target/release-fast/northhing-desktop.exe` 做性能验证。直接启动会使用普通用— profile，除非手动提供隔离存储环境变量。E2E launcher 会自动设— `northhing_USER_ROOT`、`northhing_HOME` — `northhing_E2E_STORAGE_GUARD=1`，确保性能测试不会静默写入真实 northhing profile— ### 5. 启动和长 Session 性能 E2E

性能测试优先使用 `release-fast`，这样可以使用生— Web bundle 和类 release Rust profile，同时通过 `devtools` feature 保留嵌入— WebDriver— 先为目标工作区生成长 session 和长列表 fixture— ```bash
pnpm --dir tests/e2e run fixture:long-session -- --workspace <workspace-path> --session-count 80 --long-turns 80
```

运行— release 性能测试— ```bash
pnpm run desktop:build:release-fast
cross-env E2E_TEST_WORKSPACE=<workspace-path> northhing_E2E_PERF_SESSION_ID=perf-long-session-000 pnpm run e2e:test:perf:release-fast
```

运行 debug 对照测试— ```bash
cargo build -p northhing-desktop
cross-env E2E_TEST_WORKSPACE=<workspace-path> northhing_E2E_PERF_SESSION_ID=perf-long-session-000 pnpm run e2e:test:perf:debug
```

测试报告会写— `tests/e2e/reports/performance/`，包含启动阶段、Tauri API 聚合、首次打开— session、后— full hydrate 等时间片。可选阈值环境变量：`northhing_E2E_PERF_MAX_INTERACTIVE_MS`、`northhing_E2E_PERF_MAX_SESSION_FRAME_MS`— ## 测试结构

```
tests/e2e/
├── specs/ # 测试规范
├── l0-smoke.spec.ts # L0: 基本冒烟测试
├── l0-open-workspace.spec.ts # L0: 工作区检— ├── l0-open-settings.spec.ts # L0: 设置交互
├── l0-navigation.spec.ts # L0: 导航侧边— ├── l0-tabs.spec.ts # L0: 标签— ├── l0-theme.spec.ts # L0: 主题系统
├── l0-i18n.spec.ts # L0: 国际— ├── l0-notification.spec.ts # L0: 通知系统
├── l0-observe.spec.ts # L0: 手动观察
├── l1-ui-navigation.spec.ts # L1: 窗口控制
├── l1-workspace.spec.ts # L1: 工作区管— ├── l1-chat-input.spec.ts # L1: 聊天输入
├── l1-navigation.spec.ts # L1: 导航面板
├── l1-file-tree.spec.ts # L1: 文件树操— ├── l1-editor.spec.ts # L1: 编辑器功— ├── l1-terminal.spec.ts # L1: 终端
├── l1-git-panel.spec.ts # L1: Git面板
├── l1-settings.spec.ts # L1: 设置面板
├── l1-session.spec.ts # L1: 会话管理
├── l1-dialog.spec.ts # L1: 对话框组— └── l1-chat.spec.ts # L1: 聊天功能
├── page-objects/ # Page Object 模型
├── BasePage.ts # 包含通用方法的基— ├── ChatPage.ts # 聊天视图页面对象
├── StartupPage.ts # 启动屏幕页面对象
├── index.ts # 页面对象导出
└── components/ # 可复用组— ├── Header.ts # Header组件
└── ChatInput.ts # 聊天输入组件
├── helpers/ # 工具函数
├── index.ts # 工具导出
├── screenshot-utils.ts # 截图捕获
├── tauri-utils.ts # Tauri特定辅助函数
├── wait-utils.ts # 等待和重试逻辑
├── workspace-helper.ts # 工作区操— └── workspace-utils.ts # 工作区工— ├── fixtures/ # 测试数据
└── test-data.json
└── config/ # 配置
 ├── wdio.conf.ts # WebDriverIO基础配置
 ├── wdio.conf_l0.ts # L0测试配置
 ├── wdio.conf_l1.ts # L1测试配置
 └── capabilities.ts # 平台能力配置
```

## 编写测试

### 1. 测试文件命名

遵循此约定：

```
{级别}-{特性}.spec.ts

示例— l0-smoke.spec.ts
- l1-chat-input.spec.ts
- l2-ai-conversation.spec.ts
```

### 2. 使用 Page Objects

**不好**— ```typescript
it('should send message', async () => {
 const input = await $('[data-testid="chat-input-textarea"]');
 await input.setValue('Hello');
 const btn = await $('[data-testid="chat-input-send-btn"]');
 await btn.click();
});
```

**— *— ```typescript
import { ChatPage } from '../page-objects/ChatPage';

it('should send message', async () => {
 const chatPage = new ChatPage();
 await chatPage.sendMessage('Hello');
});
```

### 3. 测试结构模板

```typescript
/**
 * L1 特性名— spec: 此测试验证内容的描述— */

import { browser, expect } from '@wdio/globals';
import { SomePage } from '../page-objects/SomePage';

describe('特性名— , () => {
 const page = new SomePage();

 before(async () => {
 // 设置 - 在所有测试前运行一— await browser.pause(3000);
 await page.waitForLoad();
 });

 describe('子特— 1', () => {
 it('应该做某— , async () => {
 // 准备
 const initialState = await page.getState();
 
 // 执行
 await page.performAction();
 
 // 断言
 const newState = await page.getState();
 expect(newState).not.toEqual(initialState);
 });
 });

 afterEach(async function () {
 // 失败时捕获截图（由配置自动处理）
 });

 after(async () => {
 // 清理
 });
});
```

### 4. data-testid 命名约定

格式: `{模块}-{组件}-{元素}`

**示例**— ```html
<!-- 启动— >
<div data-testid="startup-container">
 <button data-testid="startup-open-folder-btn">打开文件— /button>
 <div data-testid="startup-recent-projects">...</div>
</div>

<!-- 聊天 -->
<div data-testid="chat-input-container">
 <textarea data-testid="chat-input-textarea"></textarea>
 <button data-testid="chat-input-send-btn">发— /button>
</div>

<!-- 顶栏 -->
<header data-testid="header-container">
 <button data-testid="header-minimize-btn">_</button>
 <button data-testid="header-maximize-btn">— /button>
 <button data-testid="header-close-btn">×</button>
</header>
```

### 5. 断言

使用清晰、具体的断言— ```typescript
// — 具体的期— expect(await header.isVisible()).toBe(true);
expect(messages.length).toBeGreaterThan(0);
expect(await input.getValue()).toBe('期望的文— );

// 避免: 模糊的断言
expect(true).toBe(true); // 无意— ```

### 6. 等待和重— 使用内置的等待工具：

```typescript
import { waitForElementStable, waitForStreamingComplete } from '../helpers/wait-utils';

// 等待元素变稳— await waitForElementStable('[data-testid="message-list"]', 500, 10000);

// 等待流式输出完成
await waitForStreamingComplete('[data-testid="model-response"]', 2000, 30000);
```

## 最佳实— ### 应该做的

1. **保持测试专注** - 一个测试，一个断言概念
2. **使用有意义的测试名称** - 描述预期行为
3. **测试用户行为** - 而不是实现细— 4. **正确处理异步** - 始终 await 异步操作
5. **测试后清— * - 需要时重置状— 6. **记录进度** - 使用 console.log 进行调试
7. **使用环境设置** - 集中管理超时和重— ### 不应该做— 1. **不要使用硬编码等— * - 使用 `waitForElement` 而不— `pause`
2. **不要在测试间共享状— * - 每个测试应该独立
3. **不要测试内部实现** - 专注于用户可见的行为
4. **不要忽略不稳定的测试** - 修复或标记为跳过并说明原— 5. **不要使用复杂的选择— * - 优先使用 data-testid
6. **不要测试第三方代— * - 只测— northhing 功能
7. **不要混合测试级别** - 保持 L0/L1/L2 分离

### 条件测试

```typescript
it('当工作区打开时应测试功能', async function () {
 const startupVisible = await startupPage.isVisible();
 
 if (startupVisible) {
 console.log('[测试] 跳过: 工作区未打开');
 this.skip();
 return;
 }
 
 // 测试继续...
});
```

## 问题排查

### 常见问题

#### 1. 内嵌 WebDriver 无法连接

**症状**: — `http://127.0.0.1:4445` — `/status` 或创— session 请求失败

**解决方案**:
```bash
# 构建 debug 桌面应用
cargo build -p northhing-desktop

# — debug 模式运行测试，northhing 会在进程内启— WebDriver
northhing_E2E_APP_MODE=debug pnpm --dir tests/e2e run test:l0:protocol

# 确认应用进程可以监听 127.0.0.1:4445
```

#### 2. 应用未构— **症状**: `Application not found at target/debug/northhing-desktop.exe`

**解决方案**:
```bash
# 构建应用（从项目根目录）
cargo build -p northhing-desktop

# 验证二进制文件存— # Windows
dir target\debug\northhing-desktop.exe
# Linux/macOS
ls -la target/debug/northhing-desktop
```

#### 3. 测试超时

**症状**: 测试失败并显— timeout"错误

**原因**:
- 应用启动慢（debug 构建更慢— 元素尚未可见
- 网络延迟

**解决方案**:
```typescript
// 增加特定操作的超时时— await page.waitForElement(selector, 30000);

// 添加策略性等— await browser.pause(1000); // 点击— ```

#### 4. 元素未找— **症状**: `Element with selector '[data-testid="..."]' not found`

**调试步骤**:
```typescript
// 1. 检查元素是否存— const exists = await page.isElementExist('[data-testid="my-element"]');
console.log('元素存在:', exists);

// 2. 捕获页面源码
const html = await browser.getPageSource();
console.log('页面 HTML:', html.substring(0, 1000));

// 3. 截图
await browser.saveScreenshot('./reports/screenshots/debug.png');

// 4. 在前端代码中验证 data-testid
// 检— src/web-ui/src/... 中的组件
```

#### 5. 不稳定的测试

**症状**: 测试有时通过，有时失— **常见原因**:
- 竞态条— 时序问题
- 测试间状态污— **解决方案**:
```typescript
// 使用 waitForElement 而不— pause
await page.waitForElement(selector);

// 确保测试独立— beforeEach(async () => {
 await page.resetState();
});
```

### 调试模式

启用调试运行测试— ```bash
# 启用 WebDriverIO 调试日志
pnpm test -- --spec ./specs/l0-smoke.spec.ts --log-level=debug
```

### 截图分析

测试失败时，截图会自动保存到 `tests/e2e/reports/screenshots/`— ## 添加新测— ### 分步指南

1. **确定测试级别** (L0/L1/L2)
2. **— `specs/` 目录创建测试文件**
3. **— UI 元素添加 data-testid** (如需— 4. **— `page-objects/` 创建或更— Page Objects**
5. **按照模板编写测试**
6. **本地运行测试**验证
7. **— `package.json` 添加 pnpm 脚本** (可— 8. **更新配置**以包含新— spec 文件

### 示例: 添加 L1 文件树测— 1. 创建 `tests/e2e/specs/l1-file-tree.spec.ts`
2. 向文件树组件添加 data-testid:
 ```tsx
 <div data-testid="file-tree-container">
 <div data-testid="file-tree-item" data-path={path}>
 ```
3. 创建 `page-objects/FileTreePage.ts`:
 ```typescript
 export class FileTreePage extends BasePage {
 async getFiles() { ... }
 async clickFile(name: string) { ... }
 }
 ```
4. 编写测试:
 ```typescript
 describe('L1 文件— , () => {
 it('应显示工作区文件', async () => {
 const files = await fileTree.getFiles();
 expect(files.length).toBeGreaterThan(0);
 });
 });
 ```
5. 运行: `pnpm test -- --spec ./specs/l1-file-tree.spec.ts`
6. 更新 `config/wdio.conf_l1.ts` 以包含新— spec

## CI/CD 集成

### 推荐测试策略

```yaml
# .github/workflows/e2e.yml (示例)
name: E2E Tests

on: [push, pull_request]

jobs:
 l0-tests:
 runs-on: windows-latest
 steps:
 - uses: actions/checkout@v3
 - name: Setup pnpm
 uses: pnpm/action-setup@v4
 with:
 version: 10.15.0
 - name: Setup Node.js
 uses: actions/setup-node@v3
 with:
 node-version: '20'
 cache: 'pnpm'
 - name: Setup Rust
 uses: dtolnay/rust-toolchain@stable
 - name: 构建应用
 run: cargo build -p northhing-desktop
 - name: 安装测试依赖
 run: cd tests/e2e && pnpm install
 - name: 运行 L0 测试
 run: cd tests/e2e && northhing_E2E_APP_MODE=debug pnpm run test:l0:all
 
 l1-tests:
 runs-on: windows-latest
 needs: l0-tests
 if: github.event_name == 'pull_request'
 steps:
 - uses: actions/checkout@v3
 - name: 构建应用
 run: cargo build -p northhing-desktop
 - name: 运行 L1 测试
 run: cd tests/e2e && northhing_E2E_APP_MODE=debug pnpm run test:l1
```

### 测试执行矩阵

| 事件 | L0 | L1 | L2 |
|------|----|----|-----|
| 每次提交 | — | — | — |
| Pull request | — | — | — |
| 每晚构建 | — | — | — |
| 发布— | — | — | — |

## 可用— pnpm 脚本

| 脚本 | 描述 |
|------|------|
| `pnpm run test` | 使用默认配置运行所有测— |
| `pnpm run test:l0` | 仅运— L0 冒烟测试 |
| `pnpm run test:l0:all` | 运行所— L0 测试 |
| `pnpm run test:l1` | 运行所— L1 测试 |
| `pnpm run test:l0:workspace` | 运行工作区测— |
| `pnpm run test:l0:settings` | 运行设置测试 |
| `pnpm run test:l0:navigation` | 运行导航测试 |
| `pnpm run test:l0:tabs` | 运行标签测试 |
| `pnpm run test:l0:theme` | 运行主题测试 |
| `pnpm run test:l0:i18n` | 运行国际化测— |
| `pnpm run test:l0:notification` | 运行通知测试 |
| `pnpm run test:l0:observe` | 运行观察测试 (60— |
| `pnpm run clean` | 清理 reports 目录 |

## 资源

- [WebDriverIO 文档](https://webdriver.io/)
- [Tauri 测试指南](https://tauri.app/v1/guides/testing/)
- [Page Object 模式](https://webdriver.io/docs/pageobjects/)
- [northhing 项目结构](../../AGENTS.md)

## 贡献

添加测试时：

1. 遵循现有结构和约— 2. 使用 Page Object 模式
3. 向新 UI 元素添加 data-testid
4. 保持测试在适当级别(L0/L1/L2)
5. 如引入新模式请更新本指南

## 支持

如有问题或疑问：

1. 查看[问题排查](#问题排查)部分
2. 查看现有测试文件以获取示— 3. 带着测试日志和截图提— issue
