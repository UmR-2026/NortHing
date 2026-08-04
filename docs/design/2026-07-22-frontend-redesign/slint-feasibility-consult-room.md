# Slint 落地可行性结论 — consult-room 方向（spike 2026-08-02）

> 分支：spike/consult-room-slint（一次性）。探针：`src/apps/desktop/src/ui/poc_consult_probe.slint`。
> 验证：`CARGO_TARGET_DIR=<主worktree>/target` + `CARGO_PROFILE_DEV_SPLIT_DEBUGINFO=off` +
> `rustup run stable-x86_64-pc-windows-msvc cargo check -p northhing` → **Finished（3m59s，含探针）**。
> 本文是旧 PoC（slint-feasibility-poc.md）的增量：只写新方向新验的项与对旧处方的修正。

## 总判

consult-room v2 的翻译墙仍是"矮墙"，但旧 PoC 有一项处方**错的**：breathe 绑 scale-x/scale-y
在本 Slint 版本不存在（探针实测 Unknown property）。正确姿势=绑 opacity——而 v2 的 aura 呼吸语义
本就是 opacity（1→0.65），无损失。其余三项（阴影替身 / mind 派生表 / 收容框双线）全部机制可行。

## 探针结论

| 探针 | 机制 | 编译 | 折扣 | 决策建议 |
|---|---|---|---|---|
| 1 阴影 | A=偏移 Rectangle 假阴影；B=无阴影靠 border+底色阶 | 双过 | 0 | **采 B 为默认**。暗色阴影不可见、亮色无菌室靠线不靠影；A 留作中央 room 卡浮不起时的后备 |
| 2 mind 派生表 | 生成器扩 mind 维度：5 色 × 5 角色 × 双主题=25 token，color-mix(in srgb)=gamma 逐通道插值，透明端 8 位 alpha hex | 过 | 0 | 入产品。亮色派生（混 #241108/#101416）五 hue 均不发闷（hue 保持好）；`mind-leap-line` 的"过暗"旗标是误报——亮底上线本就该深，真正判据是对比度，全部达标 |
| 3 收容框/亮双线 | 外圈 Rectangle border（暗=frame 发光色/亮=line）+ 内圈 1px Rectangle `visible: !dark`；尺寸 100%/parent 表达式 | 过 | 0 | 入产品。resize 贴窗口，禁硬编码 px |
| 4 breathe | `opacity: 0.85 + 0.15*Math.sin(animation-tick()/6000ms*360deg)` 真绑 | 过 | 0 | **修正旧 PoC**：scale-x/scale-y 不存在，改 opacity。`desktop:dev` 肉眼确认仍待 FR-T3 走查 |

## 对旧 PoC 速查表的增量/修正

- ❌ 旧：`@keyframes breathe{scale:1.015}` → `animation-tick()` 绑 **scale-x/scale-y**。
  ✅ 新：Rectangle 无 scale-x/scale-y；绑 **opacity**（或 width/height，但会重排，不推荐）。
- ❌ 旧未记：`border-radius: 50%` 不编译（百分比→长度仅限 width/height 系）。✅ 圆=半径定值 px。
- ✅ 新：双 `slint_build::compile_with_config` 共存时，`SLINT_INCLUDE_GENERATED` 后者覆盖前者——
  **探针放 main 之前编译**，rust 层 include 仍是 main；探针编译错误照常在 build.rs 暴露。
- ✅ 新：新 worktree 缺 gitignore 的生成文件，先跑 `node scripts/generate-i18n-contract.mjs`
  （否则 northhing-core E0583 generated_locale_contract 缺失）。
- ✅ 新：color-mix 与透明端的正式入产品路径已实证=生成器扩维（勿手改 redesign_palette.slint），
  25 个 mind token 已随本次重跑进 palette 与 tokens-srgb-table.md。

## 红线（沿用旧 PoC，复述）

品牌不染 rep；思考块底不染 rep；沉积轮不染当前 rep；**用户侧（见证者）不染 rep 边/底**
（v2 现稿 witness 右缘 2px accent 须在 FR-T3 前改掉）；正文不染 rep；叠层用显式 Rectangle 顺序。

## 遗留（不阻塞，交 FR-T3）

1. breathe opacity 肉眼走查（desktop:dev）。
2. mind 表设计走查：凝视/镇静 的亮色 accent 偏深是刻意的对比取舍，需设计师确认"深一档"而非"发闷"。
3. 探针文件与 build.rs 探针段在本分支留档不合并；合入 FR-T3 时删除。
