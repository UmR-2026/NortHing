# W24-2 执行卡 — 组装器收口（编排者侧，无仓内闸；agent-project 仓）

来源：plan §1 "W24-2 组装器收口（入 agent-project git + manifest 钉 allowlist/工具哈希 + maxBuffer + 流程改口）[编排者侧，无仓内闸]"；A-2〔Critical〕（independent-review-2026-09-09）：信任根未入版本控制 + 使用为零 + manifest 不钉 allowlist 内容 + execFileSync 默认 1MB maxBuffer。

- [ ] 1. 改 `E:\agent-project\.opencode\tools\assemble-review-package.mjs`：
  - execFileSync 全部调用加 `maxBuffer: 64 * 1024 * 1024`；
  - manifest 增 `allowlistSha256` + `allowlistBytes`（读文件实内容）；
  - manifest 增 `toolHashes`：assembler 自身 + `<repo>/scripts/verify-task-gate.mjs` 的 sha256。
- [ ] 2. 冒烟：负探针（错误 base → exit 1）+ 本波 W24-1/W24-3 审查包实战组装（采用证据）。
- [ ] 3. `git add .opencode/tools/assemble-review-package.mjs` 点名提交（agent-project 仓；message 注 A-2 收口）。
- [ ] 4. 流程改口：`<HOME>/.config/opencode/AGENTS.md`（编排者全局指令文件）工作流段 review-package 步骤改指 assembler；`memory/facts/conventions.md` 同步一句；两仓各自 commit。
- [ ] 5. 波级终审时把 agent-project diff 一并交 reviewer。

验收：assembler 在 W24 波内被真实使用 ≥1 次（W24-1 或 W24-3 的审查包），manifest 含 allowlist sha256 与 toolHashes。
