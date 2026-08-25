# Task P1a Brief — F4 编年史条（状态驱动渐变）

> 需求唯一来源：`.superpowers/sdd/consult-room/prescription-v3-20260825.md` §F4 + 真值 `docs/design/2026-07-22-frontend-redesign/consult-room/consult-room-main.html` L548-584。
> Base commit: `a2e5e5a`（P0c 已落）。

## 真值语义（已实现核对，必须满足）

- `BIRTH = '#DAD6CF'`（出生灰，恒定最左 stop 0%，**不褪色**）
- `MINDS = ['#C8714C','#3F837B','#8B5FBF','#D99B48','#4B8F6B']`（与 onboarding SWATCHES 同源）
- 历史 stop 位置均分 0..70%；当前色恒在 100%（右端）
- 历史色衰退：`mixHex(BIRTH, c, 0.18 + 0.82 * (i / (hist.len()-1)))`——越早越接近出生灰；`hist.len()==1` 时除零守卫
- 换色（dblclick 演示）：旧当前色 push 进 hist（落位 100%）→ nowC 轮到 MINDS 下一个 → 位置重算
- **事件驱动**：只在状态变化时重渲。真值的 rAF drift 是易位缓动 polish，**按迁移纪律不移植**（本轮 stop 位置直接跳到目标值，不做逐帧 easing）
- `mixHex` 在 **Rust 侧**实现（逐通道线性插值），不依赖 CSS `color-mix()`

## 范围：`src/apps/desktop/src/ui_dioxus/app.rs`（就地改）

1. 新 Signal：
   ```rust
   let mut mind_base = use_signal(|| "#C8714C".to_string());      // 当前色（真值初始 nowC）
   let mut mind_history = use_signal(|| vec!["#DAD6CF".to_string(), "#3F837B".to_string(), "#8B5FBF".to_string()]); // 真值初始 hist
   ```
   （真值初始值即 BIRTH + 两条历史 + 当前驱力橙；与真值 demo 初态一致。持久化归 F6，本轮 session-local。）

2. Rust 混色函数（可放 app.rs 底部 `#[cfg(test)]` 可测）：
   ```rust
   fn mix_hex(a: &str, b: &str, t: f64) -> String
   fn chronicle_gradient(history: &[String], current: &str) -> String
   // 产出 "linear-gradient(90deg, {BIRTH} 0.00%, {fade1} 35.00%, ..., {current} 100%)"
   // 位置：i/(n-1)*70 均分（n=history.len()），当前色 100%
   ```

3. `chronicle-bar` div（现有空壳）：
   ```rust
   div {
       class: "chronicle-bar",
       id: "chronicle-bar",
       style: format!("background: {}", chronicle_gradient(&mind_history.read(), &mind_base.read())),
       title: "它换代表色时：新色自右端进入，旧色慢慢沉向左（双击演示）",
       onmousedown: move |e| e.stop_propagation(),
       ondoubleclick: move |_| {
           let cur = mind_base();
           mind_history.write().push(cur.clone());
           let minds = ["#C8714C","#3F837B","#8B5FBF","#D99B48","#4B8F6B"];
           let next = minds[(minds.iter().position(|m| *m == cur).unwrap_or(0) + 1) % 5];
           mind_base.set(next.to_string());
       },
   }
   ```

4. **不动 `TRUTH_CSS`**。如需平滑过渡（可选，非必须）：在 `OVERLAY_CSS` 加 `@property --cpos-* { syntax: "<percentage>"; ... }` + transition。不做也行——本轮离散跳变即满足语义。

5. 单测（`app.rs` 或 api.rs 测试模块不合适，放 app.rs `#[cfg(test)]`）：
   - `mix_hex("#DAD6CF", "#3F837B", 1.0) == "#3F837B"`（t=1 全到目标）
   - `mix_hex("#DAD6CF", "#3F837B", 0.0) == "#DAD6CF"`（t=0 全在底色）
   - `chronicle_gradient` 单历史元素（len=1）不 panic 且含 "0.00%" 与 "100%"
   - 历史 3 条时位置为 0/35/70 + 100

## 禁区

- 不动 `TRUTH_CSS` / `consult-room-main.css` / truth HTML
- 不移植 rAF / JS easing
- 不接 settings 持久化（F5/F6 的事）
- 不动 P0b/P0c 的 send/approval 路径

## 验证（必跑并贴输出）

```powershell
$env:TEMP = "C:\Users\UmR\AppData\Local\Temp"; $env:TMP = $env:TEMP
cd E:\agent-project\northing
cargo check -p northhing --features ui-dioxus
cargo test -p northhing --features ui-dioxus --lib ui_dioxus
```

报告：`.superpowers/sdd/reports/task-p1a-chronicle-report.md`（status + files + 验证输出原文 + 偏离声明）。
