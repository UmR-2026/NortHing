# W9-6 Judge 验收单（第二轮，fixer commit f7df521）

**原 verdict commit**: 4a9818d  
**fixer commit**: f7df521  
**轮次**: 2  
**验收状态**: ✅ 通过（4 项已修复；2 项偏差可接受）

---

## 复查项

### I-1（RESOLVED）: `read_workspace_file` 符号链接逃逸

**修复机制**：`resolve_within_workspace` 对 workspace root 和 joined target 双双 canonicalize（follows symlinks），前缀比较在 canonical namespace 下进行；同时 `symlink_metadata` 拒绝任何用户路径的 leaf 为符号链接。

**Windows `\\?\` 前缀核查**：`pick_workspace_root` 在传入 `resolve_within_workspace` 前先 canonicalize workspace root 为 `\\?\C:\...` 形式，joined 路径同理 canonicalize 后同前缀空间比较。fixer commit message 明确记载修复了 bare-drive 混用 bug（absolute-only 一侧产 `\\?\` 另一侧产 bare `C:\...`，造成误拒）。当前代码：`canonicalize(root)` + `canonicalize(joined)` + `or_else(absolute)` fallback，双边 canonical 无混用。✅

**代码路径**：
```
pick_workspace_root(Some(raw)) → canonicalize(root) → PathBuf
resolve_within_workspace(canonical_root, user_path)
  → joined = canonical_root.join(relative)  [same \\?\ prefix]
  → canonical_root = canonicalize(workspace_root)  [redundant-but-safe]
  → canonical_lex = canonicalize(&joined)  [follows symlinks]
  → if !canonical_lex.starts_with(&canonical_root) → Validation
  → symlink_metadata(&joined) → matches!(is_symlink) → Validation
```

**结论**: 双重防御层（canonicalize 前缀 + symlink_metadata 拒绝），Windows `\\?\` 前缀一致，I-1 ✅ CLOSED。

---

### I-2（RESOLVED）: 工作区根 CWD 与配置不匹配

**修复机制**：`KernelPlatformApi` 两方法新增 `workspace_root: Option<&str>` 首参。Desktop `api_fs` 每调用从 `AppSettings.current_workspace` 读取并传 `Some(...)`。`None` 回落至 `default_workspace_path()`（CWD），兼容测试/CLI。

**桥接链**：
```
Desktop: desktop_workspace_root() → load_app_settings().await → current_workspace.map(|p| p.to_string())
  → workspace_root.as_deref() → Option<&str>
Facade: pick_workspace_root(Some(raw)) → canonicalize(raw) → If non-absolute → Validation
        → resolve_within_workspace(canonical_root, dir/path)
```

**风险评估**: `desktop_workspace_root()` 每调用读 settings（非缓存），在快速连续 UI 调用下增加 IO 路径。但 fail-open 到 CWD 从不崩溃，且 settings 文件通常 OS-cached。未来可优化为 per-component 缓存，但非正确性问题。

**结论**: I-2 ✅ CLOSED。

---

### M-1（RESOLVED）: windows.rs rot-budget 超线

`windows.rs` 精确 800 行（threshold `> 800`，800 = NOT over）。rot-budget 脚本绿灯。✅

### M-2（RESOLVED）: fold_all 不联动 files 面板

`.folded_files opts out of fold_all by design (see panel_files::render_files_section).` 一行注释在位。文档化有意行为。✅

---

## 偏差评估

### 偏离 1（Symlink 测试 Windows 降级）

**性质**: 两个 symlink 测试（`read_file_rejects_symlink_to_outside_target`, `list_tree_skips_symlink_to_outside_target`）在无 `SeCreateSymbolicLinkPrivilege` 主机上 skip-on-runtime-denied（eprintln warn 而非 hard panic）。

**可接受性**: ✅ Minor。  
理由：这不是测试静默跳过，而是**平台能力缺失**（Windows 非开发者模式不允许创建符号链接）。代码层面的 symlink 防御（`symlink_metadata` 拒绝 + `canonicalize` 前缀校验）在生产环境始终执行，与测试能否创建 link 无关。eprintln warning 确保 CI 日志中可见。建议后续在 Windows Developer Mode CI 上补跑。

### 偏离 2（read_file_rejects_too_large 断言放宽）

**性质**: 原只接受 `NotFound`，现接受 `NotFound | Validation`。  
**可接受性**: ✅ None。  
理由：`"".into()` 路径现在被路径围栏以 `Validation` 拒绝（`""` → resolve_within_workspace → `relative.is_empty()` 返回 `true` → workspace_root），在到达 `is_file()` 之前拦截。这是**更强的 gate**（更高层级拦截），放宽断言恰好匹配语义进展。

---

## SPEC / QUALITY 判决

| 维度 | 结论 |
|------|------|
| **SPEC** | ✅ 通过 — 路径围栏两层防御闭合；工作区根由桌面配置驱动；所有上限/二进制检测/i18n/约束满足 |
| **QUALITY** | ✅ 通过 — I-1/I-2 安全缺口已关闭；M-1/M-2 合规；偏差 Minor/None |

| 分类 | Created | Resolved | 遗留 |
|------|---------|----------|------|
| Critical | 0 | 0 | 0 |
| Important | 2 | 2 | 0 |
| Minor | 2 | 2 | 0 |
| Cannot Verify | 1 | 0 | 1（截图仍为 SVG mockup，合理保留） |

**最终判决：PASS**

一句话理由：**I-1 的 canonicalize+prefix 双防御已在 Windows `\\?\` 命名空间正确闭合（fixer 记录了 bare-drive 混用修复），I-2 的桌面 settings-to-facade 桥接链完整且 fail-open 安全，两项 Minor 已落线 + 文档化，偏差均可在平台限制下接受；12 围栏测试全绿与 rot-budget 绿灯确证。**
