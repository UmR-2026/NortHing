# W16-2 独立验收：assemble-review-pkgs.ps1 manifest 化

- 审查对象：`E:\agent-project` commit `c827e02`（单文件 `A .opencode/external-review/2026-09-05/assemble-review-pkgs.ps1`，143 行，纯新增）
- SPEC 源：`NortHing/.superpowers/sdd/w16-2-brief.md`
- 报告：`NortHing/.superpowers/sdd/reports/w16-2-report.md`
- 结论：**APPROVE_WITH_CONCERNS** — Critical 0 / Important 1 / Minor 4

---

## 1. SPEC 逐条判定

| # | 要求 | 判定 | 证据 |
|---|---|---|---|
| 1 | 清单驱动：数组含 title/path/required；Emit 遍历清单；无散落硬编码 Emit | PASS | `assemble-review-pkgs.ps1:6-16`（A 包 9 项）、`:18-26`（B 包 7 项），每项三字段齐全；`:28-31` 包表；唯一 Emit 调用点 `:116`（全文件通读，无第二处调用、无第二处 Emit 定义） |
| 2a | A 包含编排者纪律 AGENTS.md 且 required | PASS | `:7` `path = "<USERPROFILE>\.config\opencode\AGENTS.md"; required = $true` |
| 2b | B 包 SKILL.md 去重（只一条） | PASS | `:19` 唯一 `anti-rot-system\SKILL.md` 条目；实跑产物 `B-antirot-review.md` 材料标题 1/7..7/7 无重复（我方实跑 Select-String 校验） |
| 2c | 编号自动生成 | PASS | `:37` `$displayTitle = "$index/$total $title"`，`:112` `$total = $pkg.materials.Count`，`:116` 传 `$i+1`；产物实际为 `1/9..9/9` 与 `1/7..7/7` |
| 2d | 包体无空标题节 | PASS（并已核对不误删） | `:109` `-replace '(?ms)^# 材料 1/6[^\r\n]*\s*$',''`。我方核对：`A-brief.md:28` / `B-brief.md:29` 各恰有一处匹配行，且该行是 brief 末尾的无正文标题；`\s*` 贪婪只吞空白，遇非空白即回溯，**不会吞掉正常正文**。产物 `A-workflow-review.md:20-32` 显示 brief 正文完整保留、标题位已消失 |
| 3a | required 缺失 → 非零退出 | PASS | `:39-44`（`Write-Error` + `exit 1`；`$ErrorActionPreference="Stop"` 下 Write-Error 即终止）；report §3.2 实跑原文 `ERROR: Required material ... not found` + `LASTEXITCODE=1` |
| 3b | 非 required 缺失 → OMITTED 节 + manifest 标注 + warning 继续 | PASS | `:45-55` 三件齐全；report §3.3 实跑原文含 `WARNING: Optional material ... Marked as OMITTED.`、包体 `> **OMITTED**: ... — File not found: ...`、manifest `"status": "OMITTED"` / `sha256: null` / `reason`、`LASTEXITCODE=0` |
| 4 | manifest 结构字段 | PASS | `:132-140` + `:123-128` + `:48-55/69-76`。我方实读磁盘 `package-manifest.json`：`scriptSha256` = `fb6074a7...`，与**当前提交版脚本的实测 Get-FileHash 完全一致**（即该 manifest 由被审版本产出，非陈旧产物）；`generatedAt` 原文 `"2026-09-05T16:37:02Z"` 为合法 ISO（report §3.1 里显示的 `09/05/2026 16:35:52` 是 ConvertFrom-Json 反序列化后的本地化打印，非文件内容，不构成不符） |
| 5 | UTF8 编码 + 标题/来源/```text 围栏结构 | PASS（当前态） | `:63-67` header=`# 材料：` + `来源: \`path\`` + ```` ```text ````，footer 闭合围栏，全部 `-Encoding UTF8`；产物 `A-workflow-review.md:29-31` 实测符合。**"保持不变" 无法从 diff 判定**，见 §4 |

我方独立实跑校验（未重跑实现者已跑的用例，仅做产物一致性核对）：

```text
PS> (Get-FileHash assemble-review-pkgs.ps1 -Algorithm SHA256).Hash.ToLowerInvariant()
fb6074a7bbef36a67c4957b8949fdf117d9f1c73caa4022bba2ffa640c0af05e

PS> $m = Get-Content -Raw package-manifest.json | ConvertFrom-Json
scriptSha256: fb6074a7bbef36a67c4957b8949fdf117d9f1c73caa4022bba2ffa640c0af05e   # 与上行一致
generatedAt(raw json): "2026-09-05T16:37:02Z"
A-workflow-review.md bytes=56313 mats=9   -> 9 项全 EMITTED
B-antirot-review.md  bytes=92117 mats=7   -> 7 项全 EMITTED

PS> git show c827e02 --name-status
A .opencode/external-review/2026-09-05/assemble-review-pkgs.ps1   # 单文件，符合 Global Constraint 4
```

磁盘产物时间戳 `2026/9/6 0:37:02` 晚于 commit 时间 `00:36:46`，且全 EMITTED —— 证实负向测试后确已改回并重跑，工作树无残留污染。

## 2. Global Constraints

1. pwsh 7 兼容 / 零新依赖 — PASS（`[ordered]`、`Get-FileHash`、`ConvertTo-Json -Depth`、`-LiteralPath` 均为内置；manifest 实际产出即为兼容性证据）。
2. 验证输出原文进 report — PASS（正向 + 两条负向均有原文；缺一条见 Minor-1）。
3. 脚本输出 English-only — 部分违反，见 Minor-2。
4. 只 commit 该 ps1 — PASS（`--name-status` 单文件）。工作树另有 `M .opencode/tools/shot-window.ps1`，未进本 commit，属他任务，不计入本次。

## 3. Findings

### Important

**I-1 · required 缺失时留下"截断包 + 陈旧 manifest"，故障态本身缺显式标记（`:110` / `:39-44`）**
`:110` `Set-Content` 先把目标包截断重写，`:116` 循环中途 `Write-Error/exit 1` 直接终止 —— 此时磁盘上留下的是一个**半截 A-workflow-review.md**，而 `package-manifest.json`（`:138-140` 在循环之后才写）仍是上一次成功运行的内容。report §3.2 实跑正是这个状态。这恰好落回本任务要根除的失效模式：目录里躺着一个看起来正常、实则缺材料的包。
缓解：manifest 记录了 package 级 sha256，理论上可被哈希比对识破 —— 但需要有人主动比对，不是自证的。
最小修法（任选其一，一行级）：失败分支前 `Remove-Item -LiteralPath $target -Force -EA SilentlyContinue`；或组装写临时文件、全部成功后再 `Move-Item` 覆盖；或失败时把 manifest 覆写为 `{"status":"FAILED", ...}`。

### Minor

**M-1 · report 缺"改回后最终正向重跑"的输出原文（report:149）**
§3.3 结尾只声明"已将脚本改回"，未贴改回后的重跑命令与输出；报告里唯一的正向输出（§3.1）产生于负向测试之前。我方通过 scriptSha256 与磁盘产物全 EMITTED 自行补证通过，故不升档；但按"验证章节命令与输出要与 diff 对得上"，这条本应由实现者给出。

**M-2 · 控制台错误/警告内插中文材料标题，与 Global Constraint 3（English-only）字面冲突（`:42`/`:45`/`:80`/`:83`）**
消息框架是英文，但 `$title` 为中文，实跑输出即 `ERROR: Required material '编排者纪律（每 session 常驻注入的 AGENTS.md）' not found ...`。属"数据 vs 文案"的灰区，故仅 Minor。若要严格合规：错误信息用 `path` 定位（本就唯一），标题留给 manifest。

**M-3 · 空标题清洗正则硬编码 `1/6`（`:109`）**
`^# 材料 1/6...` 是对当前两份 brief 的字面耦合。brief 若改版（例如遗留的是 `# 材料 2/8`），清洗静默失效、悬空标题重新混入包体，且**无任何告警**。建议改为匹配"`# 材料 <任意编号>` 且其后直到下一标题无正文"的通用形态，或至少在未命中时 `Write-Warning`。注：brief 文件不在允许文件集内，脚本内清洗是当时唯一合规做法，方向无误。

**M-4 · `exit 1` 在 `Stop` 语义下不是实际退出路径（`:43`/`:81`）**
`$ErrorActionPreference="Stop"` 使 `Write-Error` 立即抛终止性错误，`exit 1` 不会被执行；非零退出码来自未捕获错误而非显式 exit。当前行为正确（report §3.2 实测 `LASTEXITCODE=1`），保留 `exit 1` 也算 EAP 被改时的兜底，故不要求改。仅记录：退出码是间接得来的，不要在此基础上假设"可自定义退出码"。

### 已检查但判定为无问题（回应 QUALITY 提问）

- **哈希计算正确性 / 大文件内存**：`Get-FileHash`（`:60`/`:99`/`:121`）是流式，不占内存；材料 sha256 与 bytes 均取自**源文件**（非重新编码后的包体字节），与 brief §4 的 `materials:[{path, sha256, bytes}]` 语义一致。`Get-Content -Raw`（`:59`）确为全量读入，但材料最大 92KB，无实际风险，且要拼进包体本就必须读全文 —— 不构成 finding。
- **错误路径覆盖**：`Test-Path` 缺失分支（`:39`）+ 读取异常 `try/catch` 分支（`:77`）双覆盖，两分支都按 `required` 分流，无静默 `continue`。
- **投机性抽象**：无。无单实现接口、无多余参数化、无为"将来第三个包"预留的配置层；`$packages` 表就是需求本身。符合 YAGNI。

## 4. Cannot verify from diff（未猜测，逐条列出）

1. **SPEC-1 的"改造"与 SPEC-5 的"保持现有 UTF8 编码行为与包体结构不变"** —— 本 commit 是 `A`（新增）而非 `M`，git 中不存在旧版脚本，无法做前后对比，因此"是否真的删除了原有散落硬编码 Emit""编码/结构是否与旧版一致"两点**无法由 diff 判定**。我方能确认的只有当前态：全文件唯一 Emit 调用点、当前结构与编码符合 SPEC 文字描述、产物实测正确。若编排者需要该项闭环，需另行提供改造前脚本快照。
2. **`<USERPROFILE>\.config\opencode\AGENTS.md` 的内容正确性** —— 该路径在仓库外，其内容是否即 report §3.1 所贴片段，我方仅能确认包体 `A-workflow-review.md:29-31` 的来源标注与围栏结构正确、material status=EMITTED 且 sha256/bytes 非空；未逐字比对外部文件。
3. **负向测试期间的中间态**（截断包大小 49245 等）—— 现场已被最终重跑覆盖，只能采信 report 原文，无法复核。此为 I-1 的判定依据来源，不影响 I-1 成立（代码路径 `:110`→`:116` 静态可证）。

---

**结论：APPROVE_WITH_CONCERNS**（Critical 0 / Important 1 / Minor 4）。SPEC 五条全部满足，两条负向实测证据齐备，单文件提交合规。放行条件建议：I-1 在下一次触碰该脚本时一并修（一行级），M-1 由编排者以本报告 §1 的实跑输出补齐台账。

---

# 修复轮重审（63a9289）

- 审查对象：`E:\agent-project` commit `63a9289`（`fix(review): validate all required materials before writing any package (W16-2)`，单文件 `M .opencode/external-review/2026-09-05/assemble-review-pkgs.ps1`，+31/-7）
- 结论：**APPROVE** — Critical 0 / Important 0 / Minor 2（均为新记录的观察项，非阻塞）

## R1. Important-1：已消除（我方独立负向实跑复核通过）

代码结构（`:101-123`）：预检循环在**所有写盘语句之前**——第一处写盘是 `:134 Set-Content`，预检整体位于 `:102-123`，两个包的 brief 文件 + 全部 `required:$true` 材料在此逐一 `Test-Path` + `[System.IO.File]::OpenRead(...).Dispose()`（存在性 + 可读性双检），任一失败即 `$host.SetShouldExit(1)` + `throw`。静态可证：required 缺失路径下不存在任何 `Set-Content`/`Add-Content` 可达点。

我方**未采信 report 的前后快照**，独立重跑负向场景（刻意选 **B 包第 2 项**材料破坏路径，以同时验证「A 包全部合格、B 包不合格」的跨包绕过面）：

```text
=== BEFORE ===
package-manifest.json 333a67cd2734d27e0abdda0a5440ac72d82750ae44b4db94450aa0c65bfbe679  6078   2026-09-05T16:53:00.7400704Z
A-workflow-review.md  b168905f6541eae3c06541d71151580d9bcf8e4d5171bd983dd1168b0bd5bd58  56313  2026-09-05T16:53:00.6332505Z
B-antirot-review.md   6c2074a03f22f26817c7eaebf3b1b8560d74819c20553db6afa2633442000f12  92117  2026-09-05T16:53:00.7204214Z

PS> pwsh -File .\neg-test.ps1        # 脚本副本，仅把 rot-budget.json 改为 rot-budget.NONEXISTENT.json
Exception: ...\neg-test.ps1:113
 113 |                  throw "ERROR: Required material not found at '$path'"
     | ERROR: Required material not found at 'E:\agent-project\NortHing\scripts\rot-budget.NONEXISTENT.json'
LASTEXITCODE=1

=== AFTER ===
package-manifest.json 333a67cd2734d27e0abdda0a5440ac72d82750ae44b4db94450aa0c65bfbe679  6078   2026-09-05T16:53:00.7400704Z
A-workflow-review.md  b168905f6541eae3c06541d71151580d9bcf8e4d5171bd983dd1168b0bd5bd58  56313  2026-09-05T16:53:00.6332505Z
B-antirot-review.md   6c2074a03f22f26817c7eaebf3b1b8560d74819c20553db6afa2633442000f12  92117  2026-09-05T16:53:00.7204214Z
```

三份产物 sha256 / 长度 / LastWriteTimeUtc **纳秒级完全一致**，磁盘零触碰；退出码 1。**I-1 关闭**。

**§4 复核要点 4（部分包过、部分未过的绕过面）：不存在。** 预检是「先全部包校验完，再进入写盘循环」的两段式结构（两个独立 `foreach`），而非「边校验边写」；上述实跑正是 A 包全合格而 B 包不合格的场景，A 包同样零触碰。若把预检写成单循环内校验+写，才会有该绕过面——当前实现没有。

（测试卫生：负向用脚本副本 `neg-test.ps1` 同目录运行，`$PSScriptRoot` 解析一致，用完 `Remove-Item`；复核后 `git status --short` 仅剩 `M .opencode/tools/shot-window.ps1`，属他任务遗留，与首轮审查记录一致，本轮无新增残留。）

## R2. 同根 Minor 复核

| 项 | 判定 | 证据 |
|---|---|---|
| M-4 退出码路径 | **已理顺** | `:42/80/105/112/118` 统一 `$host.SetShouldExit(1)` + `throw`，`exit 1` 全部移除；退出码不再是「EAP 副作用间接得来」而是显式设置。实测 `LASTEXITCODE=1`（上文）。`if ($host -and $host.SetShouldExit)` 的存在性守卫在无 host 宿主下也不会二次异常 |
| M-2 控制台英文化 | **已闭环** | `:43/45/81/83/106/113/119` 全部消息只含英文框架 + `$path`/`$($_.Exception.Message)`，中文 `$title` 已从错误/警告消息移除（`$title` 只保留在包体 markdown 与 manifest 数据里，符合首轮建议的「用 path 定位、标题留给 manifest」）。实跑错误原文即纯英文（上文），Global Constraint 3 满足 |
| M-3 空标题正则通用化 | **已修且不误删**（实测） | `:133` `'(?ms)^# 材料(?:\s+\d+/\d+)?[:：][^\r\n]*\s*$'`，`1/6` 字面耦合解除 |

M-3 的「不误删」我方逐样本实跑（避免通用化过度）：

```text
IN : "# 标题\n正文\n"          -> OUT: "# 标题\n正文\n"        # 普通标题不受影响
IN : "# 材料：X\n\n正文保留\n"  -> OUT: "\n正文保留\n"          # 只删标题行，正文保留
IN : "# 材料 2/8：空标题\n"     -> OUT: ""                     # 通用编号命中（原 1/6 硬编码做不到）
IN : "# 材料清单\n内容\n"       -> OUT: "# 材料清单\n内容\n"    # 无冒号不误命中
IN : "## 材料：二级\n内容\n"    -> OUT: "## 材料：二级\n内容\n"  # 二级标题不误命中
```

实际 brief 中待清洗行为 `A-brief.md:28` / `B-brief.md:29` 的 `# 材料 1/6：…`（全角冒号），新正则命中；产物 `A-workflow-review.md` 前 32 行标题序列为 `# 外部审查包 A…` / 4 个 `##` / `# 材料：1/9 …`，无悬空空标题、brief 正文完整。

## R3. 正向回归（我方实跑）

```text
PS> pwsh -File .\assemble-review-pkgs.ps1
package-manifest.json   6078
A-workflow-review.md   56313
B-antirot-review.md    92117
LASTEXITCODE=0

script=64496592c9c59555ab2d046e9ae4b3631c3c02873908021b63ec0cabcfd3e194
manifest.scriptSha256=64496592c9c59555ab2d046e9ae4b3631c3c02873908021b63ec0cabcfd3e194
match=True
A-workflow-review.md bytes=56313 mats=9 nonEmitted=0 diskSha=True
B-antirot-review.md  bytes=92117 mats=7 nonEmitted=0 diskSha=True
generatedAt raw: "generatedAt": "2026-09-05T16:57:29Z"
```

两包 16 项材料全 `EMITTED`；manifest 记录的 package sha256 与磁盘重算值一致；`scriptSha256` 与当前提交版脚本文件哈希一致（即 manifest 由被审版本产出）；`generatedAt` 为合法 ISO-8601 UTC。首轮 M-1（缺改回后正向重跑原文）由 report §3.4 补齐，且与我方本轮实跑数值一致（bytes 三值 6078/56313/92117 完全吻合）。**M-1 关闭。**

## R4. 新改动引入的问题

未发现 Critical/Important。新增预检为纯只读校验（`Test-Path` + `OpenRead().Dispose()`），无副作用、无新依赖、pwsh7 内置；`Emit` 内原有的 required 分支保留为纵深防御，未与预检冲突。两处新增 Minor 观察项：

**M-5 · 预检与写盘之间存在 TOCTOU 窗口（`:102-123` → `:127-153`）**
required 材料若在预检通过之后、`Emit` 读取之前被删除或锁定（外部进程/并发运行），`Emit` 的 `:43`/`:81` 分支仍会在包文件已被 `:134` 截断之后抛出，退回首轮 I-1 描述的「半截包 + 陈旧 manifest」形态。本脚本是人工单次运行的组装工具，窗口毫秒级且无并发写者，风险实际为零，故记 Minor 不要求改。若将来纳入自动化/并发调度，最小修法仍是首轮给过的「写临时文件 + 全部成功后 `Move-Item` 覆盖」。

**M-6 · 正则未命中时无告警（`:133`）**
首轮 M-3 给的是「通用化 **或** 未命中时 `Write-Warning`」二选一，实现者选了通用化，字面已满足。但 brief 若改用完全不同的标题形态（例如去掉冒号写成 `# 材料 1/6`），清洗仍会静默失效且无任何提示。一行 `if (-not ($briefContent -match $re)) { Write-Warning ... }` 可闭合，非阻塞。

## R5. Cannot verify from diff（逐条列出，未猜测）

1. 首轮 §4 的第 1 条（旧版脚本不存在于 git，无法比对「改造前是否真有散落硬编码 Emit」）在本轮仍然成立——但本轮的审查对象是 `c827e02 → 63a9289` 的增量，该增量为纯 `M` 且完整可读，不受影响。
2. 外部路径 `<USERPROFILE>\.config\opencode\AGENTS.md` 的内容正确性：我方仅确认其 status=EMITTED、sha256/bytes 非空、包体来源标注与围栏结构正确，未逐字比对该仓外文件（与首轮口径一致）。

---

**结论：APPROVE**（Critical 0 / Important 0 / Minor 2）。首轮 Important-1 与 Minor-1~4 全部闭环，且 I-1 由我方独立负向实跑（跨包场景）复核而非采信实现者快照；正向回归两包 16 项全 EMITTED、哈希自洽。新记 M-5（TOCTOU，当前使用形态下风险为零）/ M-6（正则未命中无告警）两项非阻塞观察，交终审 triage。
