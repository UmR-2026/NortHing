# northhing-onboarding — 项目级入职技能

> 新 agent 接手 northhing 项目时的必读指南。

## 触发条件

- 新 session 启动，用户说"继续 northhing 工作"
- 用户说"修 northhing 的 X bug"
- 用户说"给 northhing 加 X 功能"

## 必读文件 (按顺序)

1. `README.md` — 项目简介、Quick Start
2. `AGENTS.md` — 架构、命令、验证表
3. `docs/handoffs/2026-07-16-handoff.md` — 最近一次 session 的详细产出
4. `HANDOFF.md` — 跨 session 状态追踪 (§0 有 commit log)
5. `memory/northhing.md` + `memory/lessons.md` — 项目记忆 (FTS5 可查)

## 环境准备

```powershell
# GCC 路径 (必须)
$env:Path = "C:\msys64\mingw64\bin;" + $env:Path

# 桌面构建 (MSVC)
rustup override set stable-x86_64-pc-windows-msvc  # 在 northhing 目录下
cargo run -p northhing

# workspace 测试
cargo test --workspace --exclude northhing

# 代码地图
cd .graph
py query.py "搜索词" --mode hybrid --limit 5
py backup.py create  # 备份 FTS5 + 向量 DB
```

## 项目结构速查

```
northhing/
├── .graph/              # FTS5 + MiniLM 记忆搜索 (Python)
│   ├── build.py         # 从 memory/*.md 建 FTS5 索引
│   ├── embed.py         # 生成 384d 向量
│   ├── query.py         # hybrid 搜索
│   ├── embed_server.py  # HTTP embedding server (port 7788)
│   └── backup.py        # 备份还原
├── memory/              # 项目记忆 (被 .graph 索引)
├── src/
│   ├── apps/
│   │   ├── desktop/     # Slint 桌面 (主产品)
│   │   ├── cli/         # TUI 终端
│   │   ├── server/      # Web RPC 服务
│   │   └── relay-server/# 远程配对中继
│   └── crates/
│       ├── assembly/core/   # 核心运行时
│       ├── services/services-integrations/  # MCP/Git/搜索/远程
│       ├── execution/      # Agent/dispatch/harness
│       ├── adapters/        # AI 协议适配
│       └── contracts/       # DTO/事件/端口
├── docs/
│   ├── handoffs/        # 每 session 交接文档
│   ├── plans/           # 路线图/执行计划
│   ├── specs/           # 设计规格
│   └── releases/        # Release notes
└── .agents/skills/      # 项目级技能 (本目录)
```

## 已知 P0 阻断 (按优先级)

1. **配置双写** — 桌面写 `~/.northhing/config/app.json`，agent 读 `%APPDATA%/northhing/config/app.json`
2. **事件桥** — UI 不订阅 AgenticEvent，发消息后永远不更新
3. **引导回调** — pick-folder / test-provider 无 Rust handler
4. **installer 后端** — 8 个空 .rs 文件
5. **移动端入口** — RemoteConnectService 无 desktop 调用

## 已知 P1 安全

- `skip_tool_confirmation=true` 默认
- API key 明文存两处
- 配置非原子写
- shell 拒绝名单可绕过

## 验证铁律

- Status report 必带 evidence (commit hash / file path / test result)
- Subagent 报告的 lines/counts/sizes 必独立 verify (`wc -l`)
- Spec 数字必 re-verify; long-line ≤5 per file, cap 120
- `Measure-Object -Line` 永远不要用

## 代码地图使用

```bash
# 搜索记忆
cd .graph
py query.py "northhing architecture" --mode hybrid --mode T --limit 5

# 备份 (不会进 git)
py backup.py create
py backup.py verify --name backup_20260717_030000.tar.gz
py backup.py restore --name backup_xxx.tar.gz

# 重建索引 (memory/ 变更后)
py build.py && py embed.py
```

## 不要做的事

- 不要直接 `git push` 到 main，先确认 remote 是 `UmR-2026/NortHing`
- 不要修改 `.rs` 文件中的 crate 名 (northhing-* 是正确的)
- 不要删除 `.graph/*.db` 文件 (运行期生成)
- 不要在 `memory/` 存真实 API key (会被 FTS5 索引)
- 不要绕过 `cargo check` 直接 `cargo run`

## 关联外部文件

- Kimi K3 全量 review: `C:\Users\UmR\WorkBuddy\2026-07-17-02-25-47\northing-deep-review.md`
- QClaw code review: `.handoffs/review-commit-997e14e_20260717.md`
- BitFun 旧 workspace: `C:\Users\UmR\.bitfun\personal_assistant\workspace\`
