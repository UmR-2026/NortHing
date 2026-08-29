# W9-6 Review Brief（judge 验收单）

仓库：E:\agent-project\NortHing（main）。只读审查。

## 证据

- diff 包：`.superpowers/sdd/w9-6-review-package.diff`（单 commit `4a9818d`，13 文件 +1100/-78）
- 需求：`.superpowers/sdd/w9-6-file-tree-brief.md`（含路径围栏硬要求）
- 实现者报告：无独立文件（返回消息即报告；NOTE.md 在 `.superpowers/sdd/w9-6-shot-1-NOTE.md`）

## judge 重点核查项

1. **路径围栏（最高优先，信任边界）**：`resolve_within_workspace` 逐行审——`..` 逃逸、绝对路径、符号链接逃逸、Windows 盘符/UNC/大小写前缀陷阱；7 个测试是否真覆盖逃逸（断言拒绝而非仅仅不 panic）；canonical-prefix 二次校验逻辑。
2. **上限**：256KiB 默认/4MiB 上限/5 层深度/1024 entries——二进制检测方式（NUL 探测还是扩展名猜？）。
3. **偏离 5（工作区根错配）**：facade 用 `default_workspace_path()`（=进程 CWD），桌面配置的 current_workspace 未桥接——这意味着文件树显示的是 CWD 不是用户配置的工作区。**评估严重度**：当前桌面运行形态下 CWD 与工作区是否恰好一致（看 main.rs/init 流程的 cwd 设置）；若不一致 = Important 并记入队列（实现者已建议 bootstrap 加 set_current_workspace）。
4. **偏离 3**：`mod api_fs` 放 mod.rs 顶层而非 api.rs 子模块——核实 api.rs 零改动 + 接线等价。
5. **偏离 4**：windows.rs 恰好 800/800——`folded_files` 旁路 fold_all 的功能损失评估（跨窗折叠不再联动文件面板），注释在位与否。
6. **偏离 2**：rustfmt 重排了相邻 impl block——确认纯格式化无语义变化。
7. **截图缺失**（SVG mockup）→ Cannot verify，转实测清单。
8. Spec 5 节逐条 + Constraints 逐条。

## Judge 验收块（防腐校准，逐字遵循）

你是独立验收者，**被期望找茬，不是被期望放行**。一切以 diff 和实跑输出为准；实现者已跑过的测试不重跑，但验证章节命令与输出要对得上 diff。双判决缺一不算通过。防腐必查：复用核查 / 无 owner 抽象 / 预算闸 / god-file 观测点（windows.rs 800/800 贴线、css.rs 829/830 各一句）。**阻塞性数字断言磁盘实测**。Cannot verify 单独列出，禁止猜。

## 输出

判决书写入 `.superpowers/sdd/w9-6-review.md`；返回消息只给：判决 + SPEC/QUALITY + C/I/M + 一句话理由。
