# Redesign v2 — 全页面更新 Plan（最终态）

> **真值声明**：`northing-theme-system.html` 的 **CSS 是唯一视觉真值**。本 plan 是结构清单 + 逐页差异说明；plan 文字与范式文件冲突时，**一律以范式文件为准**，照抄其变量与规则，不要自创浓度/圆角/位置数值。
> 范式文件路径（OD 编辑沙盒）：`C:\Users\UmR\AppData\Roaming\Open Design\namespaces\release-stable-win\data\projects\northing-theme-system\northing-theme-system.html`
> **仓库内归档副本（git 追踪，离线可查，与 OD 于 2026-07-24 同步）**：`northing/docs/design/2026-07-22-frontend-redesign/prototypes/theme-system.html`（其余 8 页 + README 同目录）。**归档约定**：prototypes/ = 归档基线，OD = 迭代沙盒；OD 再迭代定稿后须同步回 prototypes/ 并 commit。索引见 `prototypes/README.md`。

## 0. v2 范式核心变化（vs v1）

| 维度 | v1 | v2 |
|---|---|---|
| 顶栏 | 横排名片 | **居中在场区**（头像64px+光环+名字+状态+编年史+心境语）；⚙移入操控台；品牌退**左下角**水印 |
| 整窗色相 | 仅对话区淡雾 | **整屋空气染色**：底色平铺 3.5% rep + 顶晕 7%（衰减到 100% 才透明，整窗都有色相）+ 头像体温 30% 光晕 + 底部 1.5% 冷雾 |
| agent 对话 | .msg 400 | 活跃轮 .msg **450** + 活跃轮 **4% rep 面** + 左 2.5px rep 竖线 |
| turn-meta | 显示模型/tok/耗时 | **删除整行**（不在对话流显示） |
| 设置⚙ | 顶栏内 | **操控台 deck-bar 内** |
| 品牌 logo | 顶栏右端 | **左下角**水印 opacity .25，left:44px |
| 窗口控制 | 无 | **右上角** − □ ×，right:44px（缩进把手渗光线内） |
| 把手 | padding-top 顶对齐 | **垂直居中** justify-content:center |
| 操控台通电 | 控制行全灰 | 模型名 ◇ **染 rep**（class `di rep`） |
| 微交互 | 无 | 沉积轮 hover .5→.7；输入聚焦整屋升档（JS `.speaking`：底色 4.5% + 顶晕 10%）；心境语延迟淡入+呼吸 |
| 思考块 | 提亮带 | **撤回**，纯 abyss-400 左缘 + 冷底 |
| 暗色 | 无 | **同骨架换皮肤**：`[data-theme="dark"]` 变量覆盖 + 辉光降档 |

## 1. 通用 CSS（所有页面照抄范式文件对应规则；下方为索引，数值以范式文件为准）

```css
/* 整屋空气染色 — 底色平铺，让最远角落也带色相（关键，别漏） */
#app{ background:color-mix(in srgb, var(--rep-500) 3.5%, var(--bg)); transition:background var(--grow); }
/* 顶晕 — 大椭圆，衰减到 100% 才透明，整窗浸色 */
#app::before{ background:radial-gradient(ellipse 160% 120% at 50% 4%, color-mix(in srgb,var(--rep-500) 7%,transparent) 0%, transparent 100%); }
/* 底部冷雾 */
#app::after{ background:radial-gradient(ellipse 80% 30% at 50% 100%, color-mix(in srgb,var(--abyss-500) 1.5%,transparent) 0%, transparent 60%); }
/* 输入聚焦整屋升档 */
#app.speaking{ background:color-mix(in srgb, var(--rep-500) 4.5%, var(--bg)); }
#app.speaking::before{ background:radial-gradient(ellipse 160% 120% at 50% 4%, color-mix(in srgb,var(--rep-500) 10%,transparent) 0%, transparent 100%); }
/* 头像体温光晕（30% 中心，向外晕开） */
.presence{ position:relative; }
.presence::before{ content:'';position:absolute;top:-30px;left:50%;transform:translateX(-50%);width:560px;height:360px;
  background:radial-gradient(ellipse 50% 50% at 50% 28%, color-mix(in srgb,var(--rep-500) 30%,transparent) 0%, color-mix(in srgb,var(--rep-500) 10%,transparent) 36%, transparent 70%);
  pointer-events:none;z-index:-1;transition:background var(--grow); }
/* 内容层 z-index（只点名内容区，绝不用 #app>* 通配——会压垮 absolute 的把手/水印/窗口控制） */
#app>.presence,#app>.stream,#app>.deck-wrap{position:relative;z-index:1}

/* 渗透补强 */
::selection{background:color-mix(in srgb,var(--rep-300) 40%,transparent);color:var(--fg)}
:focus-visible{outline:2px solid color-mix(in srgb,var(--rep-500) 55%,transparent);outline-offset:2px}
.turn.sediment{opacity:.5;transition:opacity .35s ease}
.turn.sediment:hover{opacity:.7}
.turn.active{padding:var(--s3) var(--s4);background:color-mix(in srgb,var(--rep-500) 4%,transparent);border-radius:var(--r-md);border-left:2.5px solid var(--rep-500);transition:background var(--grow),border-color var(--grow)}
.turn.active .msg{font-weight:450}
.db .di.rep{color:var(--rep-500);transition:color var(--grow)}
.think{border-left:2px solid var(--abyss-400);background:rgba(63,131,123,.05)}
/* 滚动条 */
.stream::-webkit-scrollbar{width:8px}
.stream::-webkit-scrollbar-thumb{background:color-mix(in srgb,var(--rep-400) 24%,var(--border));border-radius:var(--r-pill)}
.stream::-webkit-scrollbar-thumb:hover{background:color-mix(in srgb,var(--rep-400) 42%,var(--border))}
.stream{scrollbar-color:color-mix(in srgb,var(--rep-400) 24%,var(--border)) transparent;scrollbar-width:thin}
```

设置页（无 #app，用 .settings-page 容器）雾更淡：底色平铺 1.5% + 顶晕 2%（衰减 100%）。
档案馆：底色平铺与顶晕**用 abyss 不用 rep**（冷雾 3% + 底色 abyss 1.5%）。

## 2. 通用 JS（有操控台的页面）

```js
const deck=document.querySelector('.deck');
const app=document.getElementById('app')||document.querySelector('.app')||document.body;
if(deck){deck.addEventListener('focusin',()=>app.classList.add('speaking'));deck.addEventListener('focusout',()=>app.classList.remove('speaking'));}
```

## 2b. 暗色皮肤 + 演示切换钮（所有页面都要带）

变量覆盖块（照抄范式文件 `[data-theme="dark"]`，含 ::before/::after/.presence::before/.think/.avatar::after/.user-bubble 的暗色降档；数值以范式文件为准）。body 背景用 `var(--page-out)`（亮 #1a1917 / 暗 #0c0b09）。

每页右下角加一个**演示用**亮/暗切换钮（跟随系统默认），便于评审直接看暗色：
```html
<button class="theme-toggle" id="themeToggle" title="切换亮/暗（演示）">◐</button>
```
```css
.theme-toggle{position:fixed;right:16px;bottom:16px;z-index:60;width:34px;height:34px;border-radius:50%;border:1px solid var(--border);background:var(--elevated);color:var(--muted);cursor:pointer;font-size:15px;display:flex;align-items:center;justify-content:center;box-shadow:0 4px 12px rgba(0,0,0,.12);transition:.2s}
.theme-toggle:hover{color:var(--fg)}
```
```js
(function(){
  const t=document.getElementById('themeToggle');if(!t)return;
  const set=m=>{m==='dark'?document.documentElement.setAttribute('data-theme','dark'):document.documentElement.removeAttribute('data-theme');t.textContent=m==='dark'?'◑':'';};
  let cur=window.matchMedia&&window.matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light';set(cur);
  t.addEventListener('click',()=>{cur=cur==='dark'?'light':'dark';set(cur);});
})();
```
（若该页已有演示抽屉承载亮/暗切换，如主题系统页，则不重复加此钮。）

## 3. 逐页改动清单

### 3.1 onboarding（自我认知首次启动）— 文件 `northing-self-cognition-onboarding`/`onboarding.html`
- 加整屋空气染色（底色平铺+顶晕+底部冷雾，§1）
- 出生态头像加 .avatar-wrap 光环（auraBreath；灰白态 `radial-gradient(circle, rgba(181,176,168,.15), transparent 70%)`，选色后跟 rep）
- 加 .presence::before 体温光晕 + 心境语 .p-mood（出生态"我还不知道我是谁。"，选色后 JS 更新为关键词）
- 加窗口控制 .win-ctrl（右上 right:44px）+ 水印 .watermark（左下 left:44px）
- 加暗色块 + 演示切换钮（§2b）；加 ::selection/:focus-visible；硬编码珊瑚→var(--rep-*)；统一圆角变量

### 3.2 空态首次进入 — 文件 `northing-empty-state`/`northing-empty-state.html`
- **重构横排 topbar 为居中 .presence**（照范式结构：avatar-wrap 64px+光环 + p-name + p-state + p-chrono + p-mood）
- ⚙ 移入操控台 deck-bar；品牌 logo→左下水印；加窗口控制右上
- 加整屋空气染色 + 体温光晕 + 心境语 + speaking JS + 暗色块 + 演示切换钮
- 开场白保留在对话区；硬编码珊瑚→var(--rep-*)；统一圆角

### 3.3 设置 A（壳+通用+自我认知）— 文件 `northing-set-a-general`/`settings-master.html`
- 设置页淡雾（底色 1.5% + 顶晕 2%）；自我认知区头像加光环
- 加窗口控制 + 左下水印 + 暗色块 + 演示切换钮 + ::selection/:focus-visible
- 硬编码珊瑚→var(--rep-*)；确保 :root 有 --rep-300..600/--abyss-300..500/--grow/--r-* ；统一圆角

### 3.4 设置 B（模型 Providers）— 文件 `northing-set-b-models`/`settings-models.html`
- 设置页淡雾；加窗口控制 + 左下水印 + 暗色块 + 演示切换钮 + ::selection/:focus-visible
- 硬编码珊瑚→var(--rep-*)；验证 ✓ 用 abyss-500、⚠️ 用 rep-500；统一圆角 + 完整 :root

### 3.5 设置 C（工作区+技能）— 文件 `northing-set-c-ws-skills`/`settings.html`
- 设置页淡雾；加窗口控制 + 左下水印 + 暗色块 + 演示切换钮 + ::selection/:focus-visible
- 硬编码珊瑚→var(--rep-*)；统一圆角 + 完整 :root

### 3.6 设置 D（MCP 服务器）— 文件 `northing-set-d-mcp`/`settings-mcp-servers.html`
- 设置页淡雾；加窗口控制 + 左下水印 + 暗色块 + 演示切换钮 + ::selection/:focus-visible
- 硬编码珊瑚→var(--rep-*)；连接 ✓ 用 abyss-500、⚠️ 用 rep-500；统一圆角 + 完整 :root

### 3.7 设置 E（访问权限）— 文件 `northing-set-e-access`/`access-permissions.html`
- 设置页淡雾；加窗口控制 + 左下水印 + 暗色块 + 演示切换钮 + ::selection/:focus-visible
- 硬编码珊瑚→var(--rep-*)；自治档选中态用 rep 高亮；统一圆角 + 完整 :root

### 3.8 档案馆 v1 — 文件 `northing-archive`/`archive-v1.html`
- **冷雾**：底色平铺 abyss 1.5% + 顶晕 abyss 3%（不用 rep）
- 沉积淡化 opacity 梯度**保留不动**；hover 回升保留
- ::selection 用 abyss-300；加窗口控制 + 左下水印 + 暗色块 + 演示切换钮
- 硬编码珊瑚→如有改 var(--rep-*)；统一圆角 + 完整 :root

## 4. 统一窗口 chrome（所有页面一致，数值以范式文件为准）

```css
.handle{position:absolute;top:0;bottom:0;width:34px;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:10px;z-index:20;cursor:pointer}
.win-ctrl{position:absolute;top:16px;right:44px;display:flex;align-items:center;gap:2px;z-index:30}
.win-btn{width:28px;height:28px;border-radius:7px;display:flex;align-items:center;justify-content:center;color:var(--faint);cursor:pointer;transition:.15s;border:none;background:transparent}
.win-btn:hover{background:var(--raised);color:var(--fg)}
.win-btn.close:hover{background:rgba(220,53,69,.1);color:#dc3545}
.watermark{position:absolute;bottom:22px;left:44px;display:flex;align-items:center;gap:6px;opacity:.25;z-index:2}
```
窗口控制 HTML（#app 内最前，照抄范式文件 .win-ctrl 三按钮 SVG）。

## 5. 统一圆角（所有页面用变量，禁硬编码）

| 层级 | 圆角 | 用途 |
|---|---|---|
| 窗口容器 | 20px | #app/.app |
| 卡片 | 14px | 设置卡片/气泡/操控台/方头像 |
| 小控件 | 9px | 按钮/输入/segment |
| pill | 999px | chip/徽章/滚动条/编年史条 |
| 窗口控制钮 | 7px | .win-btn |

变量：--r-sm 9px / --r-md 14px / --r-lg 18px / --r-pill 999px。

## 6. 红线（不要做）

- 品牌 logo 不染 rep
- 思考块底不染 rep（保持 abyss 冷色）
- 沉积轮不染当前 rep（保持褪色灰）
- 用户气泡不染 rep 边/底
- 正文 .msg 不染 rep（保持 --fg）
- 设置页雾：底色≤1.5% / 顶晕≤2%
- 档案馆雾用 abyss 不用 rep
- 绝不用 `#app>*` 通配 z-index（会压垮 absolute 子元素）
- 暗色不要用纯黑 #000 或霓虹亮色（用深暖黑 + 辉光逻辑）
