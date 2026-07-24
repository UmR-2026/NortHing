# Marvis (腾讯元宝桌面端) vs northing 架构对比 Review

> 日期: 2026-07-25
> 来源: 逆向工程 Marvis `E:\Program Files\Tencent\Marvis\Application\1.60.1900.122\`
> 目的: 为 northing 提供参考实现分析，供 coder 后续迭代参考

---

## 1. Marvis 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                      Qt5 Application Layer                      │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │              Marvis.exe (6.6MB) - 主 GUI                  │ │
│  │  ┌───────────────────────────────────────────���─────────┐ │ │
│  │  │           QCefView (Qt + CEF)                       │ │ │
│  │  │  ┌───────────────────────────────────────────────┐  │ │ │
│  │  │  │      CefRendererProcess.exe (852KB)          │  │ │ │
│  │  │  │  ┌─────────────────────────────────────────┐  │  │ │ │
│  │  │  │  │   Frontend (Vite-built SPA)            │  │  │ │ │
│  │  │  │  │  - index.html (workbench)              │  │  │ │ │
│  │  │  │  │  - offline-page (assets/cards)         │  │  │ │ │
│  │  │  │  │  - basis/ktx transcoders (textures)    │  │  │ │ │
│  │  │  │  └─────────────────────────────────────────┘  │  │ │ │
│  │  │  └───────────────────────────────────────────────┘  │  │ │ │
│  │  └─────────────────────────────────────────────────────┘  │ │
│  └───────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                      Backend Services                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ MarvisHost   │  │ MarvisSvr   │  │ LocalModelService    │ │
│  │ (21MB!)      │  │ (1.7MB)     │  │ (626KB)              │ │
│  │ 核心宿主     │  │ 服务进程    │  │ 本地模型推理        │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────��──────────┐ │
│  │ MarvisMCP    │  │ AndrowsMCP   │  │ GameInfoMCP          │ │
│  │ (332KB)      │  │ (15MB!)      │  │ (325KB)              │ │
│  │ 核心MCP      │  │ Android MCP  │  │ 游戏信息MCP          │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐ │
│  │ MarvisNode   │  │ MarvisLaun.  │  │ MarvisAssistant      │ │
│  │ (91MB Node)  │  │ (2.3MB)      │  │ (3.0MB)              │ │
│  │ Node.js运行时│  │ 启动器       │  │ 助手进程             │ │
│  └──────────────┘  └──────────────┘  └──────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                      Computer Control Layer                     │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │              tool/app_pilot/                              │ │
│  │  - app_pilot.exe          (MAA 游戏自动化框架)           │ │
│  │  - MaaFramework.dll       (MAA 核心)                     │ │
│  │  - MaaToolkit.dll         (MAA 工具包)                   │ │
│  │  - MaaWin32ControlUnit.dll (Windows 控制单元)            │ │
│  │  - onnxruntime.dll        (ONNX 推理引擎)                │ │
│  │  - opencv_world4_maa.dll  (OpenCV 图像处理)              │ │
│  │  - fastdeploy_ppocr_maa.dll (PaddleOCR)                  │ │
│  │  - ocr/det.onnx, rec.onnx (OCR 模型)                     │ │
│  └───────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────────┤
│                      Local AI / OCR Models                      │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ models/                                                   │ │
│  │  - angle_net.onnx      (文本方向检测, 22KB)               │ │
│  │  - crnn_lite_lstm.onnx (文字识别, 5.3MB)                  │ │
│  │  - dbnet.onnx          (文本检测, 3.6MB)                  │ │
│  │  - keys.txt            (字符集)                           │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. 七大核心发现

### 2.1 Qt5 + CEF 架构（非 Electron）
- 用 **Qt5** 做原生窗口 + **QCefView** 嵌入 Chromium
- 前端是 **Vite 构建的 SPA**（build-time: 202607232122，非常新）
- 比 Electron 更轻量，但集成复杂度更高

### 2.2 多进程微服务架构
**���个 GUI 应用拆成了 12+ 个独立进程**：

| 进程 | 大小 | 职责 |
|------|------|------|
| **MarvisHost.exe** | 21MB | **核心宿主**，最大的进程 |
| **AndrowsMCP.exe** | 15MB | Android 设备控制 MCP 服务器 |
| **MarvisNode.exe** | 91MB | **Node.js 运行时**（内置完整 npm） |
| **Marvis.exe** | 6.6MB | 主 GUI 进程 |
| **MarvisAssistant.exe** | 3.0MB | 助手逻辑进程 |
| **MarvisDlSvr.exe** | 5.4MB | 下载服务 |
| **MarvisLauncher.exe** | 2.3MB | 启动器 |
| **LocalModelService.exe** | 626KB | **本地模型推理**（ONNX） |
| **MarvisMCP.exe** | 332KB | 核心 MCP 服务器 |
| **GameInfoMCP.exe** | 325KB | 游戏信息 MCP |

### 2.3 MCP 协议是第一公民
**三个独立的 MCP 服务器**，都是独立 exe：
- `MarvisMCP.exe` — 核心能力
- `AndrowsMCP.exe` — Android 设备控制（15MB，最大说明逻辑最复杂）
- `GameInfoMCP.exe` — 游戏信息

**设计哲学：所有能力都通过 MCP 协议暴露**，包括本地模型、屏幕控制、游戏信息。

### 2.4 本地 AI 推理栈
- **ONNX Runtime**：`models/` 目录下有 OCR 模型（DBNet + CRNN + 方向检测）
- **LocalModelService.exe**：本地模型服务（可能是 Qwen/DeepSeek 量化模型）
- **app_pilot** 也用 ONNX：`tool/app_pilot/` 里有 `onnxruntime.dll` + OCR 模型

### 2.5 屏幕自动化能力（app_pilot）
**完整的计算机视觉 + 控制栈**：
- **MAA Framework**：`MaaFramework.dll` — 游戏自动化框架
- **OpenCV**：`opencv_world4_maa.dll`
- **PaddleOCR**：`fastdeploy_ppocr_maa.dll`
- **Windows 控制**：`MaaWin32ControlUnit.dll`

**能力：看屏幕 → OCR 识别 → 自动点击操作。**

### 2.6 Node.js 扩展运行时
- `marvisnode/MarvisNode.exe` = 91MB 的完整 Node.js 运行时
- 内置 `npm` / `npx`
- `node_modules/` 目前是空的（说明是运行时动态安装或从远程加载）

### 2.7 认证与安全
- `bearer/qgenericbearer.dll` — OAuth 2.0 Bearer token 认证
- `diff_config.json` + `file_config.json` — 配置管理
- `ai_sdk_internal.dll` — AI SDK 内部接口

---

## 3. northing 当前状态（2026-07-25）

| 维度 | 状态 |
|------|------|
| GUI 框架 | Tauri（计划中），当前纯 CLI |
| 前端 | 无（Slint 实验性） |
| 进程模型 | 单进程 |
| MCP 协议 | ❌ 无实现 |
| 本地模型 | ❌ 无 |
| 屏幕控制 | ❌ 无 |
| 文件系统 | 无挂载同步（SQLite FTS5 存储层开发中） |
| Agent 架构 | 设计文档已定（C4 + judge-subagent loop），代码未实现 |
| 记忆系统 | ✅ hm.db 2061条 + 日志 + 向量（bge-m3 1024d） |
| 成长/自改进 | 规划中（Phase 0-3），当前无实现 |
| 代码可读性 | ✅ 全 Rust 源码，1924 .rs 文件 |
| 版本 | 开发中，236 commits |
| 登录态 | 无用户系统 |
| 更新机制 | 无自动更新 |

---

## 4. 对比分析

| 维度 | Marvis | northing |
|------|--------|----------|
| **GUI 框架** | Qt5 + CEF（成熟） | Tauri + Slint（实验性） |
| **前端** | Vite SPA（本地 workbench） | 无 |
| **进程模型** | 12+ 独立进程 | 单进程 |
| **MCP** | ✅ 3 个 MCP 服务器 | ❌ 无 |
| **本地模型** | ✅ ONNX + LocalModelService | ❌ 无 |
| **屏幕控制** | ✅ MAA + OCR + OpenCV | ❌ 无 |
| **文件系统** | 无本地挂载同步 | ❌ 无 |
| **Agent 架构** | 多进程微服务 | 设计文档已定，未实现 |
| **记忆系统** | 黑盒（DLL） | ✅ 完整实现 |
| **代码可读性** | ❌ 全二进制 | ✅ 全源码 |
| **版本** | 1.60.1900.122（生产就绪） | 开发中 |
| **登录态** | Bearer token | 无用户系统 |
| **更新机制** | 完整 installer + 差分更新 | 无 |

---

## 5. northing 独特优势

1. **记忆系统已落地** — hm-hybrid-memory 完整实现，Marvis 的记忆是黑盒
2. **代码完全透明** — 全 Rust 源码，可审计、可修改
3. **架构更干净** — 设计文档明确分层（core / kernel-api / desktop / tools），边界清晰

---

## 6. northing 核心差距

1. **没有 MCP 协议层** — 最大缺失，Marvis 把所有能力封装成 MCP 服务器
2. **没有本地推理** — 完全依赖外部 API
3. **没有屏幕控制** — 缺少 agent 桌面应用的杀手锏能力
4. **没有 GUI** — 纯 CLI/API，无桌面界面

---

## 7. 借鉴优先级（给 coder）

### P0 — 必须做
1. **MCP 作为第一公民** → 把 kernel-api 改成 MCP 服务器，所有能力走 MCP 协议
2. **多进程宿主模式** → MarvisHost 模式，kernel-api 演化成宿主进程管理 subagent

### P1 — 强烈建议
3. **本地模型服务独立进程** → LocalModelService.exe 模式，northing 做本地 embedding / 小模型推理
4. **屏幕自动化栈** → MAA + ONNX + OpenCV，集成到 tool layer

### P2 — 未来考虑
5. **前端方案升级** → Vite + React/Vue 比 Slint 更成熟，如果 desktop-tauri 要换方案
6. **自动更新机制** → electron-updater 或自建差分更新

---

## 8. 关键代码路径（Marvis 参考）

```
Marvis 核心文件:
- src/main/index.js          → 应用生命周期 + IPC 路由（Electron 模式）
- src/main/mount/index.js    → 文件夹挂载同步（账号隔离模型）
- src/main/auth-guard.js     → 强制登录守卫
- src/main/window.js         → 多视图窗口管理

Marvis 进程启动:
- MarvisHost.exe (21MB)      → 核心宿主
- MarvisMCP.exe (332KB)      → MCP 服务器
- LocalModelService.exe      → 本地推理
- tool/app_pilot/            → 屏幕自动化

Marvis 前端:
- marvis-offline-page/index.html → Vite SPA 入口
- marvis-offline-page/workbench/ → 工作台资源
```

---

## 9. 结论

**Marvis 证明了"agent 桌面应用"长什么样**，northing 需要把这些能力用 Rust 重新实现一遍，但架构可以更干净。

核心差距不是"有没有"，而是"架构思维"：
- Marvis 把每个能力拆成独立进程 + MCP 协议
- northing 还在单进程思维里
- **northing 的机会：用 Rust 的内存安全 + 类型安全，重新实现 Marvis 的多进程 MCP 架构，但更轻量、更安全、更可维护**
