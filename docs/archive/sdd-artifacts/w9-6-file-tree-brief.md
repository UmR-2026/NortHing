# Task Brief — W9-6: 文件树/预览（右侧面板模块）

仓库：E:\agent-project\NortHing（main）。跨层：contracts/kernel-api + assembly/core facade + desktop。
来源：校准裁决 C4——文件树/预览**做，作为右侧面板的模块之一**（不是左栏主结构）。

## 现状（编排者已核实）

- facade 平台层（contracts/kernel-api/src/platform.rs）**没有**工作区文件浏览 API（现有：terminal/image/health/panels/onboarding/inspector/artifacts）——需新增。
- 工作区根解析：facade helpers 有 `default_workspace_path`；settings 侧 AppSettings.current_workspace 语义（P22 波）。
- services 层有文件系统原语（WorkspaceFileSystem trait 等，skills registry 在用）——实现者找最短复用路径。
- 右侧抽屉设施：ui_dioxus 有抽屉/面板机制（W1-W3 有抽屉跟随实测项）——先读现有抽屉/面板组件再定挂载。
- 防线：app.rs 749/800（净增 ≤20）；api.rs 799/800 **贴线——新 wrapper 必须先按 api_provider_edit.rs 模式建子模块**（如 api_fs.rs）；css.rs 743/830 有余量。

## Spec（验收标准）

### 1. 契约层（contracts/kernel-api）

- 新增 DTO：`FileTreeEntryDto { path, name, is_dir, size_bytes: Option<u64> }`（serde snake_case）。
- `KernelPlatformApi`（或语义最贴的既有 trait）新增：
  - `list_workspace_tree(dir: &str, max_depth: Option<u32>) -> Result<Vec<FileTreeEntryDto>, KernelError>`（相对 workspace 根的路径；空 dir = 根）
  - `read_workspace_file(path: &str, max_bytes: Option<u64>) -> Result<String, KernelError>`（默认上限 ≤256KB；二进制/超限返回结构化 Err）

### 2. facade 实现（纯 passthrough 到 services 层）

- **路径围栏（信任边界，硬要求）**：所有输入路径规范化后必须前缀匹配 workspace 根；`..` 逃逸/绝对路径/符号链接逃逸 → Err。附逃逸测试。
- 大小写/Windows 路径分隔符归一。
- 不存在路径 → KernelError::NotFound。

### 3. desktop 接线 + UI

- 新 api 子模块（如 `api_fs.rs`，参照 api_provider_edit.rs 模式）包装两方法。
- 右侧抽屉加「文件」模块：树形列表（目录可展开/折叠，懒加载子层）+ 点文件 → 预览面板（文本渲染，大文件截断提示）。
- 空态/错误态/非文本文件态中文显式。
- 行数纪律：树组件进新文件（如 `panel_files.rs`）。

### 4. 测试

- 契约/facade：路径围栏逃逸 ×2（`..`、绝对路径）、正常列目录、读文件超限 Err、二进制 Err。
- desktop：纯逻辑（树排序/格式化）若可抽则测。

### 5. 验证集（命令+输出原文进 report）

1. `& "$env:USERPROFILE\.cargo\bin\cargo.exe" +stable-msvc check --workspace`（contracts 动了）
2. `+stable-msvc test -p northhing --lib` + `test -p northhing-core`（facade 新测试）
3. `node scripts/verify-rot-budget.mjs`：绿
4. 截图：右侧抽屉文件模块（树+预览）`.superpowers/sdd/w9-6-shot-1.png`（不 commit）

## Global Constraints

1. 分层边界：contracts 只加 DTO+trait 方法；facade 纯 passthrough；UI 只 desktop。
2. 日志英文无 emoji。
3. SDD 禁区：禁止 git 操作 `.superpowers/`；禁止编辑 `progress.md`；禁止整树 git 操作；开工先 `git status`。
4. rot-budget：不上调；新文件 <800；收口绿。
5. commit：恰好一个；不含 `.superpowers/`。
6. 不新建无 owner 抽象；i18n 走 locale.t 既有模式。
7. **安全**：路径围栏是硬边界，测试必须覆盖逃逸。
8. 遇编译错误先加载对应 rust skill。

## 派发元信息

- 完成标准 = DONE；受阻 = BLOCKED + 原因。禁止报 Done 留 next steps。
- 返回消息含：状态 / commit SHA / git show --stat / 验证输出尾部 / 截图路径 / 偏离清单。
