# Handoff — 2026-08-28：Slint 已物理删除，Dioxus 唯一壳；W5 终审 + W6 rot 清账 + W7 F7 全部收口

> 焦点：壳切换线全闭环；W7 交付 F7（设置页 provider 编辑）。**唯一残余 = 真机实测清单（W7 后构建，含新第 8 项 provider 编辑流）。**

## 状态一句话

2026-08-26 审计修复线（W1+W2+W3 共 16 任务）全部 CAN MERGE 收口；2026-08-27 晚用户拍板 Dioxus 为生产前端（`70bc4e8` 翻 flag）→ 同夜 W4-1 物理删除 Slint 壳（`707e414`+`0c95aa6`，-16,795 行，judge 两轮后 Approved）→ W4-2 壳级审计（1C/3I/3M，报告 `.superpowers/sdd/w4-2-dioxus-shell-review.md`） → W6 rot 清账 ×2（dead_code 128→106 + 检查器语义修正，check:rot 全绿）→ W7 F7 provider 编辑 ×2（终审 CAN MERGE 0C/0I/0M）全部收口。**main HEAD = `2cfd737`。**

## commit 链（今晚，倒序）

| 波 | 内容 | commits |
|---|---|---|
| W3 | 审计 Minor 残余 ×4（r2#5/r2#7+#8/F6/F10） | `d82a074` `94a786a` `79f36db` `c6f2924`；终审 CAN MERGE 0C/0I/3M |
| 翻转 | DIOXUS_SHELL=true（用户拍板） | `70bc4e8` |
| W4-1 | Slint 物理删除 | `707e414` + `0c95aa6`（审查修复轮） |
| W4-2 | Dioxus 壳审计 | 无代码（报告落盘） |
| W5 | 审计修复 ×4 | `de60a0b`(F1) `87cb1f4`(F2) `fafc1fa`+`21f9345`(F4+修复) `f680cf6`(F5+F6)；终审 CAN MERGE 0C/0I/12M |
| W6 | rot 清账 ×2 | `11a4e5e`(dead_code 128→106) `7d53621`(检查器语义修正，D1 仲裁)；check:rot 全绿 |
| W7 | F7 provider 编辑 ×2 | `2bb91ab`(API 层) `e8dbcfd`(弹窗 UI) `2cfd737`(终审注释补丁)；终审 CAN MERGE 0C/0I/0M |

## 下 session 第一件事（按序）

1. **真机实测（唯一残余人工项）**：清单 `.superpowers/sdd/manual-test-checklist-2026-08-27.md`，**必须用 W7 后构建**（HEAD ≥ `2cfd737`）。
   - 第 5 项（provider 编辑不抹 key）**作废**——Slint 侧已删；其语义由 W7 新第 8 项（Dioxus 编辑流）继承覆盖。
   - 第 6/7 项（进程残留）兜底 W5 终审 Cannot-verify 两项（LoopDestroyed 契约 / WindowDropGuard 复用）。
   - 第 8 项（新增）：provider 编辑/测试连接/删除默认拒删全流。
   - 旧实例是 W5 前构建，测了不算数——先重拉。
2. 真机实测其它项（折叠/抽屉=三窗跟随/防跳底/Z-order）在 Dioxus 壳上首测，发现即新 finding。

## 环境与工具教训（今夜新增）

- **`pnpm run desktop:dev` 在本机跑不起来**：PATH 上独立安装的 GNU cargo 1.95（`C:\Program Files\Rust stable GNU 1.95`）遮住 rustup shim，GNU ld 链接桌面端响应文件报错。正确拉起：`C:\Users\UmR\.cargo\bin\rustup.exe run stable-x86_64-pc-windows-msvc cargo run -p northhing`。**脚本是否改走 rustup run 待用户拍板。**
- 审查包生成：`cmd /c "( ... & git diff ... -- <paths>) > file"` 括号块会吞掉末位路径——**两步法**：先写头，再单独 `>>` 追加 diff。
- W4-1 实现者曾把测试 key 写进真 OS keyring（W5-3 修复时已清）——涉 keyring 的测试 brief 以后要显式写"必须 MockKeyring"。
- **W7-2 implementer 疑似 `git restore` 误伤**：commit 前把工作区未提交的 `progress.md` W7 段回滚丢失（mtime 23:19 vs commit 23:20 坐实；新文件不受影响，只有 tracked-modified 遭殃）。对策已立：**ledger 行每任务通过后立即 commit，不攒到波末**；brief 禁区条款已有但需盯实现者别用 `git restore .`/`git checkout .` 清工作区。
- **终审 reviewer 也会幻觉**：W7 终审初判"新文件 1101 行越线"实为把 diff 偏移量误读为文件行数（实测 501）。阻塞项一律磁盘复核后再打回。

## 队列（无 blocking）

- **治理新规（用户 2026-08-28）**：技术细则 = 编排者+子代理闭环；用户只拍板面向功能的产品决策。
- rot-budget.json 死登记 ×3 清理（W6 副产物：callbacks_lifecycle/refresh 已删、kernel_facade/tests.rs 被新语义排除；小机械单）。
- W7-2-M1：pages_settings.rs 776/800，下次 provider feature 先抽 provider_row.rs。
- F7 删除守卫升级版：会话引用完整性检查（产品决策，当前 ponytail 标注）。
- F3 几何跟随线程（搁置，等 dioxus 0.8 stable 事件钩子；审计自证当前 workaround 可接受）。
- r1 Minors / r2#4 等更早残余以 `audit-wave-final-review.md` triage 为准。
- T2-1 CI 补齐（老欠账，前置 i18n-contract 24 个预存失败）。

## 选派实证（今夜）

- `gemini-37-flash-agy`：W3 4/4 + W4-1（大删除，1 轮修复）+ W5 4/4——全天 implementer 零 BLOCKED。
- judge `minimax-m3` ×6 全合格（含揪出真 keyring 污染这种实锤）。
- 终审 `reviewer/step-explore_reviewer`：W3 终审 1 次空响应→SOP 续派成功；W4-2 壳审计一轮出活（质量高）；**W5 全波终审一次出活无空响应，CAN MERGE 判决含逐条 file:line 走查，质量保持**。
- `gemini-37-flash-agy` 补丁单（终审 triage 修一记一 ×2 注释）一轮 DONE，磁盘 diff 取证相符。
- 2026-08-27 用户拍板：3.7 全档位主推（3.6 停用）；vertex+agy 双渠道可并行；judge-ox-alpha 已从配置删除。
