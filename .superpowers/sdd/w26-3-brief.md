# W26-3 Brief — P2-2 单实例拒启动（desktop win32 mutex，单 judge）

## 1. 来源与验收标准（逐字）

Plan §1：
> "W26-3 P2-2 单实例拒启动（desktop main.rs，win32 mutex；v1 不做参数转发）[单 judge]"

用户拍板（2026-09-09）："P2-2 选 (a) 拒启动"。**v1 不做参数转发**（第二实例不带任何消息给首实例，直接拒）。

ledger P2-2（tech-debt-ledger.md:98-103）症状：两实例共享 app.json 配置，last-write-wins，会话状态互踩。

### 验收标准（机械可核对）

- AC1: desktop 入口在 `main.rs` 日志初始化之后、worker 线程创建之前做单实例检查（现 :63 与 :67 之间）；已是第二实例 → 打印一行英文信息到 stderr + `std::process::exit(1)`（沿用现有退出码惯例）。
- AC2: 实现 = 新文件 `src/apps/desktop/src/single_instance.rs`：`SingleInstanceGuard` RAII（Drop 时 CloseHandle）+ `try_acquire() -> Result<SingleInstanceGuard, ...>`；win32 手写 `unsafe extern "system"` 声明（CreateMutexW / GetLastError / CloseHandle——复用 ui_dioxus/windows/mod.rs:30-69 的先例模式，零新 crate 依赖）；`#[cfg(not(target_os = "windows"))]` 下 stub 恒 Ok。
- AC3: mutex 名钉死 `"Local\\NorthHingDesktopSingleton"`（Local 会话级命名空间）。
- AC4: cfg(test) 单测：同名二次获取必败；drop 后恢复可获取；**每个测试各用彼此不同的独立 mutex 名**（cargo test 多线程并行，防互踩；均不碰生产名）。
- AC5: §6 验证全绿，输出原文进 report。

## 2. 编排者预检结论（逐项钉死，直接采信）

BASE = `9222bbc42c2b0a13961dd8e63686c3c804abe698`。

| 事实 | 证据 |
|---|---|
| main.rs:58-133 main 函数结构；:60-63 tracing 初始化；:67-88 worker 线程；检查点 = :64 前后 | 侦察实证 |
| main.rs 现零 cfg(windows) 代码；退出码惯例 `std::process::exit(1)` + eprintln | 侦察实证 |
| 手写 extern "system" 先例：ui_dioxus/windows/mod.rs:30-69（SetWindowPos/IsWindow 等 user32 系函数无 #[link] 直绑，靠默认导入库解析）；本单目标函数（CreateMutexW/GetLastError/CloseHandle）是 kernel32 系，MSVC 工具链同样默认链接——零新依赖 | 侦察实证 + 复审订正 |
| W24 已删 desktop 的 windows crate 死依赖——**本单不得重新引入 windows/windows-sys 依赖**（手写 extern 正是为了零依赖） | 233fe89 在案 |
| ERROR_ALREADY_EXISTS = 183（win32 常量） | win32 ABI 常识，实现者自查核实 |
| cfg(test) 先例：app_state/settings/keyring.rs:295-430 平台模块就地单测 | 侦察实证 |
| 非 meta-ratchet → 单 judge | workflow-policy.json |
| desktop Cargo.toml 现 63 行无 windows 系依赖 | 现读 |

## 3. 复用侦察（强制）

- extern 声明模式复用 ui_dioxus/windows 先例；不引新 crate。
- report 必须有「复用侦察」节。

## 4. Spec（必须全部满足）

- S1: single_instance.rs 按 AC2/AC3/AC4。main.rs 接线按 AC1：获取 guard 失败 → 打印 `NortHing desktop is already running in this session; refusing to start a second instance.` → exit(1)；成功则 guard 持有到 main 结束（let _guard = ...）。
- S2: 非 Windows stub 恒 Ok（CI ubuntu 哨兵腿要编过；stub 臂的编译正确性由 CI 实证，report 里注明该缺口）。
- S2b: **台账 P2-2 翻转由编排者在波收口执行**（ledger 不进场——W26-2/W26-4 同翻同文件，并行冲突规避）；本单 report 附建议翻转文案。
- S3: 禁区：Cargo.toml/Cargo.lock（零依赖变化——若发现必须加依赖，BLOCKED 上报）、ui_dioxus/ 全部、cli、scripts/、ci.yml、其它 W26 领地。
- S4: 不做参数转发/焦点激活/单实例插件（v1 范围外，越出即 FAIL）。

## 5. Global Constraints（逐字遵守）

- unsafe 代码须带 `// SAFETY:` 注释（仓规 unsafe 纪律）。
- 日志/stderr 英文无 emoji。
- 禁整树 git 操作；只点名 add/commit。
- 测试真实执行（Windows 本地可真实跑 mutex 用例），输出原文进 report。
- 路径卫生（D-2）；cargo 带 rustup 前缀。

## 6. 验证（命令 + 输出原文进 report）

1. `rustup run stable-x86_64-pc-windows-msvc cargo test -p northhing single_instance` → 绿（真实 mutex 用例）
2. `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing` → 0 error
3. `node scripts/check-repo-hygiene.mjs` → exit 0
4. `node scripts/verify-task-gate.mjs verify-attempt --base 9222bbc42c2b0a13961dd8e63686c3c804abe698 --tip <本单最后 commit sha> --allowlist .superpowers/sdd/w26-3-allowlist.txt` → exit 0（自建 allowlist 含自身；**窗口内若出现其它 W26 单文件，停手报编排者复跑**）

## 7. 报告

路径 `.superpowers/sdd/w26-3-report.md`。章节：改动摘要 / 复用侦察 / 验证 / 疑虑 / 状态。

## 8. 派发元信息

- BASE: `9222bbc42c2b0a13961dd8e63686c3c804abe698`
- 允许文件集：
  - `src/apps/desktop/src/main.rs`
  - `src/apps/desktop/src/single_instance.rs`（新建）
  - `.superpowers/sdd/w26-3-brief.md` / `w26-3-report.md` / `w26-3-allowlist.txt`
- 禁区：§4-S3 + 其它未列出文件。
- commit 规则：点名 git add；前缀 `feat(desktop): W26-3`，body 注「用户 2026-09-09 拍板 P2-2 (a) 拒启动」；允许多 commit。
- 并行声明：W26-1/2/4/5 同波全并行，文件集不相交。
