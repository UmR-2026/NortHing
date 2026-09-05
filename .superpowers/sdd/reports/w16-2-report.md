# W16-2 实现者报告：审查包组装脚本 manifest 化（含 Important-1 修复）

## 1. 改动摘要

- **清单驱动重构**：改造 `E:\agent-project\.opencode\external-review\2026-09-05\assemble-review-pkgs.ps1`，脚本顶部通过 `$packageAMaterials` 与 `$packageBMaterials` 清单数组声明各包材料，每项含 `title`、`path`、`required`（`$true`/`$false`），由 `Emit` 函数遍历清单顺序生成。
- **1/6 根因修复与清洗通用化**：
  - A 包清单显式补充材料 1/9：`<USERPROFILE>\.config\opencode\AGENTS.md`（编排者纪律，`required: $true`）；
  - B 包去重，仅保留一条 `anti-rot-system\SKILL.md` 材料；
  - 读取 brief 文件初始化包体 markdown 时，通过正则 `(?ms)^# 材料(?:\s+\d+/\d+)?[:：][^\r\n]*\s*$` 通用过滤 brief 尾部残留的空 `# 材料` 标题（解除对 `1/6` 的硬编码字面耦合），消除空标题位；材料编号由清单顺序 `$index/$total` 自动格式化（`1/9`、`1/7`）。
- **package-manifest.json 输出**：在同目录输出机器可读 manifest，包含脚本自身 `scriptSha256`、`generatedAt`（ISO 8601 UTC 时间）、各包的 `file`、`sha256`、`bytes` 以及每项材料的 `title`、`path`、`sha256`、`bytes`、`status`（`EMITTED` / `OMITTED`）与 `reason`。
- **Important-1 修复（先校验后写）**：
  - 在写任何目标包文件与 manifest 之前，完整遍历全部包的 brief 文件及所有 `required:$true` 材料，验证文件存在且可读；
  - 若任一缺失或不可读，脚本立即调用 `$host.SetShouldExit(1)` 并 `throw` 报错，以非零退出码终止，**此时磁盘上任何包文件与 manifest 均未被触碰/截断/修改**；
  - 控制台错误与警告提示纯英文化（Global Constraint 3），退出路径明确（解决同根 Minor-2 / Minor-4）。
- **代码提交**：
  - 首轮提交：`c827e02 feat(review): assemble script manifest-driven + OMITTED semantics (W16-2)`
  - 修复提交：`63a9289 fix(review): validate all required materials before writing any package (W16-2)`

## 2. 缺失语义与防破坏设计

材料清单中的 `required` 字段定义了审查材料的完整性约束：
- **先校验后写（Pre-flight Check）**：
  在任何包文件的创建、截断或写入之前，脚本执行全局预检。任何 `required:$true` 材料不存在或无法读取，立即抛出异常并设置非零退出码终止执行。由于截断重写逻辑位于预检之后，故障时磁盘既有包文件与 manifest 的 SHA-256、大小与 LastWriteTime 完全保持不变，杜绝留下半截包与陈旧 manifest 的失效形态。
- **降级显式标注（`required: $false`）**：
  当可选材料缺失或读取失败时，脚本在控制台打印 English 警告（`WARNING: Optional material not found at '<path>'. Marked as OMITTED.`），并在包体 markdown 对应材料编号处显式写入 `> **OMITTED**: <title> — <reason>` 节；同时在 `package-manifest.json` 中将该项材料的 `status` 标记为 `"OMITTED"`，`sha256` 与 `bytes` 置为 `null`，并记录具体的 `reason`（如 `File not found: <path>`），继续组装剩余材料。

## 3. 验证命令与输出原文

### 3.1 正向验证：两包组装与 manifest 生成

执行命令：
```text
pwsh -File .opencode\external-review\2026-09-05\assemble-review-pkgs.ps1
$ec = $LASTEXITCODE
Write-Host "LASTEXITCODE=$ec"
```

控制台输出：
```text
Name                  Length
----                  ------
package-manifest.json   6078
A-workflow-review.md   56313
B-antirot-review.md    92117

LASTEXITCODE=0
```

检查 manifest 反序列化状态（两包全部材料 status=EMITTED）：
```powershell
$manifest = Get-Content -Raw "E:\agent-project\.opencode\external-review\2026-09-05\package-manifest.json" | ConvertFrom-Json
Write-Host "scriptSha256: $($manifest.scriptSha256)"
Write-Host "packages count: $($manifest.packages.Count)"

foreach ($pkg in $manifest.packages) {
    Write-Host "Package: $($pkg.file), bytes=$($pkg.bytes), sha256=$($pkg.sha256), materials=$($pkg.materials.Count)"
    $nonEmitted = $pkg.materials | Where-Object { $_.status -ne "EMITTED" }
    if ($nonEmitted) {
        Write-Host "  Found non-emitted materials!"
    } else {
        Write-Host "  All $($pkg.materials.Count) materials status=EMITTED"
    }
}
```

输出：
```text
scriptSha256: 64496592c9c59555ab2d046e9ae4b3631c3c02873908021b63ec0cabcfd3e194
packages count: 2
Package: A-workflow-review.md, bytes=56313, sha256=b168905f6541eae3c06541d71151580d9bcf8e4d5171bd983dd1168b0bd5bd58, materials=9
  All 9 materials status=EMITTED
Package: B-antirot-review.md, bytes=92117, sha256=6c2074a03f22f26817c7eaebf3b1b8560d74819c20553db6afa2633442000f12, materials=7
  All 7 materials status=EMITTED
```

### 3.2 负向验证 1（Important-1 核心验证）：required 缺失时非零退出且产物未被触碰

测试过程：
1. 记录运行前 `package-manifest.json`、`A-workflow-review.md`、`B-antirot-review.md` 的 SHA-256、LastWriteTimeUtc 与文件长度。
2. 临时修改 A 包第一项材料路径为不存在的 `<USERPROFILE>\.config\opencode\NONEXISTENT_AGENTS.md`（`required: $true`）。
3. 运行脚本，捕获错误输出与 `$LASTEXITCODE`。
4. 重新比对三份产物的 SHA-256、LastWriteTimeUtc 与长度。

运行前产物快照：
```text
Name                  SHA256                                                           LastWriteTime                Length
----                  ------                                                           -------------                ------
package-manifest.json 18f1650f431737b336e67e485af27b401c36ceeeac45a0b5b40515848d94cb8b 2026-09-05T16:48:56.7336672Z   6078
A-workflow-review.md  b168905f6541eae3c06541d71151580d9bcf8e4d5171bd983dd1168b0bd5bd58 2026-09-05T16:48:56.6006918Z  56313
B-antirot-review.md   6c2074a03f22f26817c7eaebf3b1b8560d74819c20553db6afa2633442000f12 2026-09-05T16:48:56.7091950Z  92117
```

执行命令与失败输出：
```text
pwsh -File .opencode\external-review\2026-09-05\assemble-review-pkgs.ps1
$ec = $LASTEXITCODE
Write-Host "LASTEXITCODE=$ec"
```

输出：
```text
Exception: E:\agent-project\.opencode\external-review\2026-09-05\assemble-review-pkgs.ps1:113
Line |
 113 |                  throw "ERROR: Required material not found at '$path'"
     |                  ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
     | ERROR: Required material not found at '<USERPROFILE>\.config\opencode\NONEXISTENT_AGENTS.md'
LASTEXITCODE=1
```

运行后产物快照：
```text
Name                  SHA256                                                           LastWriteTime                Length
----                  ------                                                           -------------                ------
package-manifest.json 18f1650f431737b336e67e485af27b401c36ceeeac45a0b5b40515848d94cb8b 2026-09-05T16:48:56.7336672Z   6078
A-workflow-review.md  b168905f6541eae3c06541d71151580d9bcf8e4d5171bd983dd1168b0bd5bd58 2026-09-05T16:48:56.6006918Z  56313
B-antirot-review.md   6c2074a03f22f26817c7eaebf3b1b8560d74819c20553db6afa2633442000f12 2026-09-05T16:48:56.7091950Z  92117
```
**证明**：退出码为 1，且包文件与 manifest 的哈希与修改时间 100% 一致，未被任何截断或修改。

### 3.3 负向验证 2：required: $false 材料缺失时标注 OMITTED 并成功组装

临时将 A 包第一项材料调整为 `required: $false`，路径为不存在的 `<USERPROFILE>\.config\opencode\NONEXISTENT_AGENTS.md`。

执行命令：
```text
pwsh -File .opencode\external-review\2026-09-05\assemble-review-pkgs.ps1
$ec = $LASTEXITCODE
Write-Host "LASTEXITCODE=$ec"
```

输出：
```text
WARNING: Optional material not found at '<USERPROFILE>\.config\opencode\NONEXISTENT_AGENTS.md'. Marked as OMITTED.

Name                  Length
----                  ------
package-manifest.json   6098
A-workflow-review.md   49245
B-antirot-review.md    92117

LASTEXITCODE=0
```

检查包体 markdown 内容：
```text
A-workflow-review.md L29-L31:
# 材料：1/9 编排者纪律（每 session 常驻注入的 AGENTS.md）

> **OMITTED**: 编排者纪律（每 session 常驻注入的 AGENTS.md） — File not found: <USERPROFILE>\.config\opencode\NONEXISTENT_AGENTS.md
```

检查 manifest.json 记录：
```json
{
  "title": "编排者纪律（每 session 常驻注入的 AGENTS.md）",
  "path": "C:\\Users\\UmR\\.config\\opencode\\NONEXISTENT_AGENTS.md",
  "sha256": null,
  "bytes": null,
  "status": "OMITTED",
  "reason": "File not found: C:\\Users\\UmR\\.config\\opencode\\NONEXISTENT_AGENTS.md"
}
```

### 3.4 改回后最终正向重跑（解决 Minor-1）

将脚本配置改回全 `required: $true` 正常路径，并提交 `fix(review): validate all required materials before writing any package (W16-2)`。

执行最终组装重跑：
```text
pwsh -File .opencode\external-review\2026-09-05\assemble-review-pkgs.ps1
$ec = $LASTEXITCODE
Write-Host "LASTEXITCODE=$ec"
```

输出：
```text
Name                  Length
----                  ------
package-manifest.json   6078
A-workflow-review.md   56313
B-antirot-review.md    92117

LASTEXITCODE=0
```

复核脚本 hash 与 manifest 中 scriptSha256 的一致性：
```powershell
$scriptHash = (Get-FileHash -LiteralPath ".opencode\external-review\2026-09-05\assemble-review-pkgs.ps1" -Algorithm SHA256).Hash.ToLowerInvariant()
$manifestHash = (Get-Content -Raw ".opencode\external-review\2026-09-05\package-manifest.json" | ConvertFrom-Json).scriptSha256
Write-Host "script file hash:    $scriptHash"
Write-Host "manifest scriptHash: $manifestHash"
Write-Host "Match: $($scriptHash -eq $manifestHash)"
```

输出：
```text
script file hash:    64496592c9c59555ab2d046e9ae4b3631c3c02873908021b63ec0cabcfd3e194
manifest scriptHash: 64496592c9c59555ab2d046e9ae4b3631c3c02873908021b63ec0cabcfd3e194
Match: True
```

## 4. 遗留与 Caveat

无遗留问题。Important-1（先校验后写，防止留下截断包与陈旧 manifest）与 Minor-1~Minor-4 均已完全闭环。工作区仅提交目标 ps1 文件，无其它杂质变更。

DONE
