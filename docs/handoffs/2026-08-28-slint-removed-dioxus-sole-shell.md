# Handoff — 2026-08-28 凌晨：Slint 已物理删除，Dioxus 唯一壳，W5 修复完毕待终审

> 焦点：今夜主线 = 壳切换（flip → 物理删除 → 壳审计 → 审计修复）。**W5 全波终审未跑，是下 session 第一件事。** 真机实测清单需按 Dioxus 壳重排后再测。

## 状态一句话

2026-08-26 审计修复线（W1+W2+W3 共 16 任务）全部 CAN MERGE 收口；2026-08-27 晚用户拍板 Dioxus 为生产前端（`70bc4e8` 翻 flag）→ 同夜 W4-1 物理删除 Slint 壳（`707e414`+`0c95aa6`，-16,795 行，judge 两轮后 Approved）→ W4-2 壳级审计（1C/3I/3M，报告 `.superpowers/sdd/w4-2-dioxus-shell-review.md`）→ W5 修复 4/4 完成且 per-task review 全过。**main HEAD = `2ebc8c3`（台账）/ 最新代码 = `f680cf6`（W5-4）。**

## commit 链（今晚，倒序）

| 波 | 内容 | commits |
|---|---|---|
| W3 | 审计 Minor 残余 ×4（r2#5/r2#7+#8/F6/F10） | `d82a074` `94a786a` `79f36db` `c6f2924`；终审 CAN MERGE 0C/0I/3M |
| 翻转 | DIOXUS_SHELL=true（用户拍板） | `70bc4e8` |
| W4-1 | Slint 物理删除 | `707e414` + `0c95aa6`（审查修复轮） |
| W4-2 | Dioxus 壳审计 | 无代码（报告落盘） |
| W5 | 审计修复 ×4 | `de60a0b`(F1) `87cb1f4`(F2) `fafc1fa`+`21f9345`(F4+修复) `f680cf6`(F5+F6) |

## 下 session 第一件事（按序）

1. **W5 全波终审**：`review-package 86ab479..f680cf6`（两步生成法，见下运维教训）派 `reviewer/step-explore_reviewer`；Minors 12 条在 W5 各任务行内待 triage；另带 W4-1 的 M-4（hygiene 扫描基数 13→8）。
2. **实测清单 Dioxus 版重排**：
   - 第 5 项（provider 编辑不抹 key）**作废**——Dioxus 壳无编辑 UI（审计 F7，L 量级，产品决策欠账）；I1 修复随 Slint 回调层已删，sync.rs 两个 helper 现为 dead code。
   - 第 6/7 项（进程残留）**必须用 W5 后构建测**——F1 修复后 ✕ 关窗才走优雅退出链（LoopDestroyed → perform_shutdown → MCP 清理）；库契约与 WindowDropGuard 两项 judge ⚠️ 由该实测兜底。
   - 当前运行中的实例（1:08 启动）是 **W5 之前的构建**，测了不算数——先重拉。
3. 真机实测其它项（折叠/抽屉=三窗跟随/防跳底/Z-order）在 Dioxus 壳上首测，发现即新 finding。

## 环境与工具教训（今夜新增）

- **`pnpm run desktop:dev` 在本机跑不起来**：PATH 上独立安装的 GNU cargo 1.95（`C:\Program Files\Rust stable GNU 1.95`）遮住 rustup shim，GNU ld 链接桌面端响应文件报错。正确拉起：`C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo run -p northhing`。**脚本是否改走 rustup run 待用户拍板。**
- 审查包生成：`cmd /c "( ... & git diff ... -- <paths>) > file"` 括号块会吞掉末位路径——**两步法**：先写头，再单独 `>>` 追加 diff。
- W4-1 实现者曾把测试 key 写进真 OS keyring（W5-3 修复时已清）——涉 keyring 的测试 brief 以后要显式写"必须 MockKeyring"。

## 队列（无 blocking）

- F7 provider 编辑 UI（L，产品决策：要不要在 Dioxus 设置页做编辑表单）。
- F3 几何跟随线程（搁置，等 dioxus 0.8 stable 事件钩子；审计自证当前 workaround 可接受）。
- r1 Minors / r2#4 等更早残余以 `audit-wave-final-review.md` triage 为准。
- T2-1 CI 补齐（老欠账，前置 i18n-contract 24 个预存失败）。

## 选派实证（今夜）

- `gemini-37-flash-agy`：W3 4/4 + W4-1（大删除，1 轮修复）+ W5 4/4——全天 implementer 零 BLOCKED。
- judge `minimax-m3` ×6 全合格（含揪出真 keyring 污染这种实锤）。
- 终审 `reviewer/step-explore_reviewer`：W3 终审 1 次空响应→SOP 续派成功；W4-2 壳审计一轮出活（质量高）。
- 2026-08-27 用户拍板：3.7 全档位主推（3.6 停用）；vertex+agy 双渠道可并行；judge-ox-alpha 已从配置删除。
