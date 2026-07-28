# tokens-srgb-table — OKLCH→sRGB 对照表

> 由 `oklch-to-srgb.py` 从 `tokens-draft.css` 自动生成（人读走查用，勿手改；重跑即刷新）。
> 转换口径：OKLCH → OKLab → 线性 sRGB → gamma 编码；越界通道在线性段截断 clamp[0,1]（逐通道，非色域映射）。

## 颜色 token（亮/暗双套）

| token | light OKLCH | light hex | dark OKLCH | dark hex | 备注（源注释） |
|---|---|---|---|---|---|
| `bg` | `oklch(0.9641 0.0041 91.45)` | `#F4F3F0` | `oklch(0.190 0.006 88)` | `#151411` | 主背景 <- #F4F3F0 |
| `surface` | `oklch(0.9853 0.0029 84.56)` | `#FBFAF8` | `oklch(0.220 0.006 88)` | `#1C1A17` | 卡片底 <- #FBFAF8 |
| `elevated` | `oklch(1.0000 0.0000 89.88)` | `#FFFFFF` | `oklch(0.250 0.005 88)` | `#23211F` | 浮起卡片 <- #FFFFFF |
| `raised` | `oklch(0.9463 0.0070 88.64)` | `#EFEDE8` | `oklch(0.280 0.007 88)` | `#2A2925` | 控件底 <- #EFEDE8 |
| `border` | `oklch(0.9165 0.0087 84.57)` | `#E6E3DD` | `oklch(0.340 0.008 88)` | `#3A3833` | 边框 <- #E6E3DD |
| `border-soft` | `oklch(0.9372 0.0070 88.64)` | `#ECEAE5` | `oklch(0.310 0.007 88)` | `#32302C` | 弱边框 <- #ECEAE5 |
| `fg` | `oklch(0.3297 0.0127 87.56)` | `#38352E` | `oklch(0.930 0.010 88)` | `#EBE8E0` | 正文 <- #38352E |
| `muted` | `oklch(0.5673 0.0162 84.59)` | `#7B766C` | `oklch(0.680 0.013 88)` | `#9C988F` | 次级 <- #7B766C |
| `faint` | `oklch(0.7165 0.0167 86.44)` | `#A8A398` | `oklch(0.550 0.012 88)` | `#74716A` | 弱化(≥4.0:1) <- #A8A398 |
| `rep-300` | `oklch(0.7748 0 48.96)` | `#B6B6B6` | `oklch(0.7748 0 48.96)` | `#B6B6B6` | 大面积 (灰; 珊瑚 fallback C=0.0892 <- #E5A583) |
| `rep-400` | `oklch(0.7030 0 47.60)` | `#9F9F9F` | `oklch(0.7030 0 47.60)` | `#9F9F9F` | 发光/竖线 (灰; 珊瑚 fallback C=0.1075 <- #D68A63) |
| `rep-500` | `oklch(0.6383 0 43.22)` | `#8B8B8B` | `oklch(0.6383 0 43.22)` | `#8B8B8B` | 强调≡渐变右端 (灰; 珊瑚 fallback C=0.1214 <- #C8714C) |
| `rep-600` | `oklch(0.5529 0 43.44)` | `#727272` | `oklch(0.5529 0 43.44)` | `#727272` | hover/深 (灰; 珊瑚 fallback C=0.1129 <- #A85A38) |
| `abyss-300` | `oklch(0.7044 0.0532 185.05)` | `#7AABA4` | `oklch(0.7044 0.0532 185.05)` | `#7AABA4` | <- #7AABA4 |
| `abyss-400` | `oklch(0.6436 0.0681 185.58)` | `#5A9B93` | `oklch(0.6436 0.0681 185.58)` | `#5A9B93` | <- #5A9B93 |
| `abyss-500` | `oklch(0.5633 0.0703 185.32)` | `#3F837B` | `oklch(0.5633 0.0703 185.32)` | `#3F837B` | <- #3F837B |
| `danger` | `oklch(0.550 0.100 28)` | `#A45950` | `oklch(0.660 0.090 28)` | `#C37D73` | 陶红 hue≈28°，低饱和 |
| `on-rep` | `oklch(0.985 0.008 60)` | `#FEF9F5` | `oklch(0.985 0.008 60)` | `#FEF9F5` | #FFF9F5 |
| `on-abyss` | `oklch(1 0 0)` | `#FFFFFF` | `oklch(1 0 0)` | `#FFFFFF` | — |
| `on-danger` | `oklch(1 0 0)` | `#FFFFFF` | `oklch(1 0 0)` | `#FFFFFF` | — |
| `air-rep` | `oklch(0.9527 0.0040 91.45)` | `#F0EFEC` | `oklch(0.2057 0.0058 88.00)` | `#181714` | — |
| `halo-rep` | `oklch(0.9413 0.0038 91.45)` | `#ECEBE9` | `oklch(0.2214 0.0056 88.00)` | `#1C1B18` | — |
| `air-rep-speaking` | `oklch(0.9504 0.0039 91.45)` | `#EFEEEC` | `oklch(0.2259 0.0055 88.00)` | `#1D1C19` | — |
| `halo-rep-speaking` | `oklch(0.9380 0.0038 91.45)` | `#EBEAE8` | `oklch(0.2348 0.0054 88.00)` | `#1F1E1B` | — |
| `air-rep-settings` | `oklch(0.9592 0.0040 91.45)` | `#F2F1EE` | `oklch(0.1967 0.0059 88.00)` | `#161512` | — |
| `halo-rep-settings` | `oklch(0.9576 0.0040 91.45)` | `#F2F1EE` | `oklch(0.2124 0.0057 88.00)` | `#1A1916` | — |
| `fog-abyss` | `oklch(0.9581 0.0041 106.30)` | `#F1F1EE` | `oklch(0.1956 0.0059 98.26)` | `#161512` | — |
| `halo-abyss` | `oklch(0.9521 0.0044 120.20)` | `#EFF0EC` | `oklch(0.2012 0.0059 108.65)` | `#161713` | — |
| `turn-active` | `oklch(0.9714 0.0028 84.56)` | `#F6F5F3` | `oklch(0.2367 0.0058 88.00)` | `#201E1B` | — |
| `presence-halo-rep` | `oklch(0.8989 0.0033 91.45)` | `#DEDDDB` | `oklch(0.2572 0.0051 88.00)` | `#242321` | — |
| `presence-halo-rep-soft` | `oklch(0.9413 0.0038 91.45)` | `#ECEBE9` | `oklch(0.2124 0.0057 88.00)` | `#1A1916` | — |
| `archive-cold-bg` | `oklch(0.9561 0.0042 111.13)` | `#F0F1EE` | `oklch(0.1975 0.0059 101.75)` | `#161613` | — |
| `archive-cold-border` | `oklch(0.9000 0.0115 168.00)` | `#D7E1DC` | `oklch(0.2497 0.0117 160.08)` | `#1D2320` | — |
| `archive-card-bg` | `oklch(0.9726 0.0032 125.14)` | `#F5F6F4` | `oklch(0.2303 0.0059 108.65)` | `#1D1D1A` | — |

## 结构 token（主题无关，亮暗共用）

| token | 值 | 备注（源注释） |
|---|---|---|
| `fs-sm` | `10px` | 时间戳、chip 细节、turn-meta |
| `fs-md` | `11.5px` | turn-head、状态行、ctx 标签 |
| `fs-lg` | `13px` | 名字、chip、模块标题 |
| `fs-body` | `15px` | 对话正文 |
| `fs-name` | `16px` | agent 名 |
| `s1` | `4px` | — |
| `s2` | `8px` | — |
| `s3` | `12px` | — |
| `s4` | `16px` | — |
| `s5` | `24px` | — |
| `s6` | `32px` | — |
| `r-sm` | `9px` | 控件、deck-bar 按钮 |
| `r-md` | `14px` | 左栏卡片、头像、用户气泡 |
| `r-lg` | `18px` | 大卡片 |
| `r-pill` | `999px` | chip、自我认知渐变条、呼吸点 |

## 亮色源 hex 回差校验（mockup 溯源）

| token | 计算 hex | mockup 源 hex | Δmax（单通道） |
|---|---|---|---|
| `bg` | `#F4F3F0` | `#F4F3F0` | 0 |
| `surface` | `#FBFAF8` | `#FBFAF8` | 0 |
| `elevated` | `#FFFFFF` | `#FFFFFF` | 0 |
| `raised` | `#EFEDE8` | `#EFEDE8` | 0 |
| `border` | `#E6E3DD` | `#E6E3DD` | 0 |
| `border-soft` | `#ECEAE5` | `#ECEAE5` | 0 |
| `fg` | `#38352E` | `#38352E` | 0 |
| `muted` | `#7B766C` | `#7B766C` | 0 |
| `faint` | `#A8A398` | `#A8A398` | 0 |
| `abyss-300` | `#7AABA4` | `#7AABA4` | 0 |
| `abyss-400` | `#5A9B93` | `#5A9B93` | 0 |
| `abyss-500` | `#3F837B` | `#3F837B` | 0 |
| `on-rep` | `#FEF9F5` | `#FFF9F5` | 1 |

Δmax ≤ 1 属 OKLCH 四位小数舍入的正常回差；rep-* 灰阶行的 fallback 珊瑚 hex 不参与校验。

## 色域截断清单

无（全部 token 落 sRGB 色域内，未发生截断）。
