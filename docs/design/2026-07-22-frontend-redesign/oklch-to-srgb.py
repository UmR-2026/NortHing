#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""OKLCH -> sRGB token 生成器（FR-T1，可重跑）。

读取 tokens-draft.css（OKLCH 单一事实源），产出：
  1. tokens-srgb-table.md            —— token 名 × 亮/暗 × hex 对照表（人读走查）
  2. src/apps/desktop/src/ui/redesign_palette.slint —— Slint 调色板（struct 双色
     token 集 + RedesignTheme.dark 三元翻转 + 动效时长常量）

转换口径：OKLCH -> OKLab -> 线性 sRGB -> gamma 编码；越界通道在线性段截断
clamp[0,1]（逐通道，非色域映射）。数学实现参照 Bjorn Ottosson 的 OKLab 公开
推导（零依赖，仅标准库；P1.1 定稿改 css 后直接重跑本脚本即可再生成）。
"""

import math
import re
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
CSS_PATH = SCRIPT_DIR / "tokens-draft.css"
TABLE_PATH = SCRIPT_DIR / "tokens-srgb-table.md"
REPO_ROOT = SCRIPT_DIR.parents[2]
SLINT_PATH = REPO_ROOT / "src" / "apps" / "desktop" / "src" / "ui" / "redesign_palette.slint"

DECL_RE = re.compile(r"--([a-zA-Z0-9-]+)\s*:\s*([^;]+);")
OKLCH_RE = re.compile(
    r"^oklch\(\s*([0-9]*\.?[0-9]+)\s+([0-9]*\.?[0-9]+)\s+([0-9]*\.?[0-9]+)\s*\)$"
)
COLOR_MIX_RE = re.compile(
    r"^color-mix\(in oklch,\s*var\(--([a-zA-Z0-9-]+)\)\s+"
    r"([0-9]*\.?[0-9]+)%,\s*var\(--([a-zA-Z0-9-]+)\)\s*\)$"
)
PX_RE = re.compile(r"^([0-9]*\.?[0-9]+)px$")
SOURCE_HEX_RE = re.compile(r"#([0-9A-Fa-f]{6})\b")

BASE_NAMES = {
    "bg", "surface", "elevated", "raised",
    "border", "border-soft", "fg", "muted", "faint",
}

# 分组注释（忠实 tokens-draft.css 的语义注释）
GROUP_COMMENTS = {
    "base": "基底（亮=咨询室白灰 / 暗=灰黑锚；plan §5：明暗是房间的时间，中性安全恒在）",
    "rep": (
        "代表色色阶（v1 出生态 S=C=0 灰阶；§4 S=自我浮现度，出生 C=0 灰白，\n"
        "    // 成长=C 上升、颜色从基底浮现。等亮度两模式同值，编年史读法一致。\n"
        "    // 回退预案（plan §5）：Phase 3 延期则切默认珊瑚——把各 rep-* 行注释中的\n"
        "    // fallback C 恢复即可（L+H 与灰阶逐档对齐，属单变量级切换：整组 rep-* 一次换 C）"
    ),
    "abyss": "深渊青色阶（哲学常量：思考/深处，非 agent 色；等亮度两模式同值）",
    "danger": "异常态危险色（plan §12：低饱和陶红，双色；[T3] P2.1 定稿）",
    "on": "文字色（rep/abyss/danger 表面上的前景白）",
    "mix": "语义混合色（OKLCH 混色，由生成器解析）",
    "archive": "档案馆冷色（abyss 冷系，禁 rep；archive.html L34-39）",
    "fs": "字号三档 + 正文/名字（handoff §8：小字端严格收敛，层级靠颜色+字体承担）",
    "spacing": "间距 4 基数阶梯（4×1/2/3/4/6/8）",
    "radius": "圆角阶梯",
    "other": "未分组 token（css 新增，生成器自动收录）",
    "mind": (
        "mind 色维度（consult-room v2 spike 2026-08-02：5 mind × 5 角色 × 双主题；"
        "派生语义=CSS color-mix(in srgb) 即 gamma sRGB 逐通道插值，透明端出 8 位 alpha hex）"
    ),
}

# ----------------------------------------------------------------------------
# mind 色维度（consult-room v2，spike 2026-08-02）
# 源：northing-consult-room.html 的 --mind-base 候选与 color-mix 派生参数。
# 角色：glow=光晕15% / intense=40%(暗)·12%on白(亮) / line=70%on白(暗)·76%on#101416(亮)
#       / frame=55%alpha(暗)·=line(亮) / accent=base(暗)·84%on#241108(亮)
# ----------------------------------------------------------------------------
MIND_BASE = (
    ("drive", (0xC8, 0x71, 0x4C)),  # 驱力 #C8714C
    ("abyss", (0x3F, 0x83, 0x7B)),  # 深渊 #3F837B
    ("leap", (0x8B, 0x5F, 0xBF)),   # 跃迁 #8B5FBF
    ("gaze", (0xD9, 0x9B, 0x48)),   # 凝视 #D99B48
    ("calm", (0x4B, 0x8F, 0x6B)),   # 镇静 #4B8F6B
)
_WHITE = (255, 255, 255)
_LIGHT_LINE_BG = (0x10, 0x14, 0x16)     # #101416
_LIGHT_ACCENT_BG = (0x24, 0x11, 0x08)   # #241108
MIND_ROLES = ("glow", "intense", "line", "frame", "accent")


def _mix(c1, c2, w):
    """gamma sRGB 逐通道插值，w 为 c1 权重（= color-mix(in srgb, c1 w%, c2)）。"""
    return tuple(round(c1[i] * w + c2[i] * (1 - w)) for i in range(3))


def _hex(rgb):
    return "#%02X%02X%02X" % rgb


def _hex8(rgb, pct):
    return "#%02X%02X%02X%02X" % (rgb[0], rgb[1], rgb[2], round(pct * 255 / 100))


def _muddy_flag(rgb):
    """设计裁决辅助：标出发闷/过暗的亮色派生（供人读表走查，不阻断生成）。"""
    spread = max(rgb) - min(rgb)
    lum = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
    if spread < 40:
        return "发闷(彩度低)"
    if lum < 90:
        return "过暗"
    return ""


def mind_tokens():
    """返回 (fields, light_by, dark_by, rows)；rows=(name, light_hex, dark_hex, note)。"""
    fields, light_by, dark_by, rows = [], {}, {}, []
    for mind, base in MIND_BASE:
        dark = {
            "glow": _hex8(base, 15),
            "intense": _hex8(base, 40),
            "line": _hex(_mix(base, _WHITE, 0.70)),
            "frame": _hex8(base, 55),
            "accent": _hex(base),
        }
        light = {
            "glow": _hex8(base, 15),  # 亮色光晕禁用，值保留备查
            "intense": _hex(_mix(base, _WHITE, 0.12)),
            "line": _hex(_mix(base, _LIGHT_LINE_BG, 0.76)),
            "frame": None,
            "accent": _hex(_mix(base, _LIGHT_ACCENT_BG, 0.84)),
        }
        light["frame"] = light["line"]
        for role in MIND_ROLES:
            name = f"mind-{mind}-{role}"
            fields.append((name, "color"))
            light_by[name] = light[role]
            dark_by[name] = dark[role]
            note = ""
            if role in ("accent", "line"):
                note = _muddy_flag(tuple(int(light[role][i:i + 2], 16) for i in (1, 3, 5)))
            rows.append((name, light[role], dark[role], note))
    return fields, light_by, dark_by, rows


def fail(msg):
    print(f"[oklch-to-srgb] 错误: {msg}", file=sys.stderr)
    sys.exit(1)


# ----------------------------------------------------------------------------
# 颜色数学：OKLCH -> OKLab -> 线性 sRGB -> gamma（Ottosson 公开推导）
# ----------------------------------------------------------------------------

def oklch_to_hex(l_ok, c_ok, h_ok):
    """返回 (r, g, b, clamped)；clamped 为被截断的 (通道名, 截断前线性值) 列表。"""
    h = math.radians(h_ok)
    a = c_ok * math.cos(h)
    b = c_ok * math.sin(h)
    # OKLab -> LMS（立方根域）
    l_ = l_ok + 0.3963377774 * a + 0.2158037573 * b
    m_ = l_ok - 0.1055613458 * a - 0.0638541728 * b
    s_ = l_ok - 0.0894841775 * a - 1.2914855480 * b
    lms_l = l_ ** 3
    lms_m = m_ ** 3
    lms_s = s_ ** 3
    # LMS -> 线性 sRGB
    lin = (
        +4.0767416621 * lms_l - 3.3077115913 * lms_m + 0.2309699292 * lms_s,
        -1.2684380046 * lms_l + 2.6097574011 * lms_m - 0.3413193965 * lms_s,
        -0.0041960863 * lms_l - 0.7034186147 * lms_m + 1.7076147010 * lms_s,
    )
    clamped = []
    out = []
    for name, u in zip(("r", "g", "b"), lin):
        if u < 0.0 or u > 1.0:
            clamped.append((name, u))
        u = min(1.0, max(0.0, u))
        u = 12.92 * u if u <= 0.0031308 else 1.055 * (u ** (1.0 / 2.4)) - 0.055
        out.append(int(round(u * 255)))
    return out[0], out[1], out[2], clamped


def oklch_to_oklab(l, c, h):
    h_rad = math.radians(h)
    return l, c * math.cos(h_rad), c * math.sin(h_rad)


def oklab_to_oklch(l, a, b):
    c = math.sqrt(a * a + b * b)
    h = math.degrees(math.atan2(b, a))
    if h < 0:
        h += 360
    return l, c, h


def resolve_color_mix_list(tokens):
    by_name = {t["name"]: t for t in tokens}
    resolved = {}

    def _resolve(name):
        if name in resolved:
            return resolved[name]
        tok = by_name[name]
        if not tok.get("color_mix"):
            resolved[name] = tok
            return tok
        c1 = _resolve(tok["color1"])
        c2 = _resolve(tok["color2"])
        w = tok["percentage"] / 100.0
        l1, a1, b1 = oklch_to_oklab(c1["L"], c1["C"], c1["H"])
        l2, a2, b2 = oklch_to_oklab(c2["L"], c2["C"], c2["H"])
        lm = l1 * w + l2 * (1 - w)
        am = a1 * w + a2 * (1 - w)
        bm = b1 * w + b2 * (1 - w)
        l, c, h = oklab_to_oklch(lm, am, bm)
        result = {
            "name": name, "L": l, "C": c, "H": h,
            "raw": (f"{l:.4f}", f"{c:.4f}", f"{h:.2f}"),
            "comment": tok["comment"],
        }
        resolved[name] = result
        return result

    for tok in tokens:
        _resolve(tok["name"])
    return [resolved[t["name"]] for t in tokens]


def fmt_oklch(tok):
    rl, rc, rh = tok["raw"]
    return f"oklch({rl} {rc} {rh})"


# ----------------------------------------------------------------------------
# CSS 解析（行式，跟踪块注释状态；注释内容捕获作溯源）
# ----------------------------------------------------------------------------

def strip_comments(line, in_comment):
    """返回 (代码部分, 注释部分, 行末是否仍在注释中)。"""
    code_parts, note_parts, i = [], [], 0
    while i < len(line):
        if in_comment:
            end = line.find("*/", i)
            if end == -1:
                note_parts.append(line[i:])
                break
            note_parts.append(line[i:end])
            i = end + 2
            in_comment = False
        else:
            start = line.find("/*", i)
            if start == -1:
                code_parts.append(line[i:])
                break
            code_parts.append(line[i:start])
            i = start + 2
            in_comment = True
    return "".join(code_parts), "".join(note_parts), in_comment


def parse_css(text):
    root_tokens = []  # (name, value_str, note) —— 结构 token（主题无关）
    light, dark = [], []
    cur = None
    in_comment = False
    for raw_line in text.splitlines():
        code, note, in_comment = strip_comments(raw_line, in_comment)
        code_s = code.strip()
        if '[data-theme="light"]' in code_s:
            cur = "light"
            continue
        if '[data-theme="dark"]' in code_s:
            cur = "dark"
            continue
        if re.match(r"^:root\s*\{", code_s):
            cur = "root"
            continue
        if code_s.startswith("}"):
            cur = None
            continue
        m = DECL_RE.search(code_s)
        if not m or cur is None:
            continue
        name = m.group(1)
        value = m.group(2).strip()
        note = " ".join(note.split())
        if cur == "root":
            if not PX_RE.match(value):
                fail(f":root 块 token --{name} 值不是 px 尺寸: {value!r}")
            root_tokens.append((name, value, note))
        else:
            om = OKLCH_RE.match(value)
            if om:
                (light if cur == "light" else dark).append({
                    "name": name,
                    "L": float(om.group(1)),
                    "C": float(om.group(2)),
                    "H": float(om.group(3)),
                    "raw": (om.group(1), om.group(2), om.group(3)),
                    "comment": note,
                })
            else:
                mm = COLOR_MIX_RE.match(value)
                if mm:
                    (light if cur == "light" else dark).append({
                        "name": name,
                        "color_mix": True,
                        "color1": mm.group(1),
                        "percentage": float(mm.group(2)),
                        "color2": mm.group(3),
                        "comment": note,
                    })
                else:
                    fail(f'{cur} 块 token --{name} 值不是 oklch() 或 color-mix(): {value!r}')
    if not root_tokens:
        fail("未解析到 :root 结构 token 块")
    if not light:
        fail('未解析到 [data-theme="light"] 块')
    if not dark:
        fail('未解析到 [data-theme="dark"] 块')
    return root_tokens, light, dark


def group_of(name):
    if name in BASE_NAMES:
        return "base"
    if name.startswith("rep-"):
        return "rep"
    if name.startswith("abyss-"):
        return "abyss"
    if name == "danger":
        return "danger"
    if name.startswith("on-"):
        return "on"
    if name.startswith(("air-", "halo-", "fog-", "presence-")) or name == "turn-active":
        return "mix"
    if name.startswith("archive-"):
        return "archive"
    if name.startswith("fs-"):
        return "fs"
    if re.match(r"^s\d+$", name):
        return "spacing"
    if name.startswith("r-"):
        return "radius"
    if name.startswith("mind-"):
        return "mind"
    return "other"


# ----------------------------------------------------------------------------
# 产物一：tokens-srgb-table.md
# ----------------------------------------------------------------------------

def md_escape(s):
    return s.replace("|", "\\|") if s else "—"


def gen_md(root_tokens, light, dark_by, checks, clamped_all, mind_rows=()):
    out = []
    out.append("# tokens-srgb-table — OKLCH→sRGB 对照表")
    out.append("")
    out.append("> 由 `oklch-to-srgb.py` 从 `tokens-draft.css` 自动生成（人读走查用，勿手改；重跑即刷新）。")
    out.append("> 转换口径：OKLCH → OKLab → 线性 sRGB → gamma 编码；越界通道在线性段截断 clamp[0,1]（逐通道，非色域映射）。")
    out.append("")
    out.append("## 颜色 token（亮/暗双套）")
    out.append("")
    out.append("| token | light OKLCH | light hex | dark OKLCH | dark hex | 备注（源注释） |")
    out.append("|---|---|---|---|---|---|")
    for tok in light:
        d = dark_by[tok["name"]]
        out.append(
            f"| `{tok['name']}` | `{fmt_oklch(tok)}` | `{tok['hex']}` "
            f"| `{fmt_oklch(d)}` | `{d['hex']}` | {md_escape(tok['comment'])} |"
        )
    out.append("")
    out.append("## 结构 token（主题无关，亮暗共用）")
    out.append("")
    out.append("| token | 值 | 备注（源注释） |")
    out.append("|---|---|---|")
    for name, value, note in root_tokens:
        out.append(f"| `{name}` | `{value}` | {md_escape(note)} |")
    out.append("")
    out.append("## 亮色源 hex 回差校验（mockup 溯源）")
    out.append("")
    out.append("| token | 计算 hex | mockup 源 hex | Δmax（单通道） |")
    out.append("|---|---|---|---|")
    for name, calc, src, delta in checks:
        out.append(f"| `{name}` | `{calc}` | `{src}` | {delta} |")
    out.append("")
    out.append("Δmax ≤ 1 属 OKLCH 四位小数舍入的正常回差；rep-* 灰阶行的 fallback 珊瑚 hex 不参与校验。")
    out.append("")
    out.append("## 色域截断清单")
    out.append("")
    if clamped_all:
        out.append("| 模式 | token | 通道 | 截断前线性值 |")
        out.append("|---|---|---|---|")
        for mode, name, ch, val in clamped_all:
            out.append(f"| {mode} | `{name}` | `{ch}` | {val:.6f} |")
    else:
        out.append("无（全部 token 落 sRGB 色域内，未发生截断）。")
    out.append("")
    if mind_rows:
        out.append("## mind 色维度（consult-room v2 spike，color-mix(in srgb) 预计算）")
        out.append("")
        out.append("| token | light hex | dark hex | 设计裁决注 |")
        out.append("|---|---|---|---|")
        for name, lh, dh, note in mind_rows:
            out.append(f"| `{name}` | `{lh}` | `{dh}` | {md_escape(note)} |")
        out.append("")
        out.append("裁决注为生成器启发式（彩度 spread<40 标发闷、亮度<90 标过暗），最终取舍由设计走查定。")
        out.append("")
    return "\n".join(out)


# ----------------------------------------------------------------------------
# 产物二：redesign_palette.slint
# ----------------------------------------------------------------------------

def gen_slint(root_tokens, light, dark_by, clamped_all, mind=None):
    color_order = [t["name"] for t in light]
    light_by = {t["name"]: t for t in light}
    struct_fields = [(n, "color") for n in color_order]
    if mind:
        struct_fields += mind[0]
    struct_fields += [(n, "length") for (n, _v, _c) in root_tokens]

    clamp_note = (
        "无（全部 token 落 sRGB 色域内）"
        if not clamped_all
        else "、".join(f"{mode}/{name}.{ch}" for mode, name, ch, _v in clamped_all)
    )

    lines = []
    lines.append("// ============================================================================")
    lines.append("// redesign_palette.slint — northing 重设计调色板（FR-T1，生成器产出）")
    lines.append("// ----------------------------------------------------------------------------")
    lines.append("// 本文件由生成器自动产出——勿手改数值；改动请改源头后重跑：")
    lines.append("//   源：docs/design/2026-07-22-frontend-redesign/tokens-draft.css（OKLCH 单一事实源）")
    lines.append("//   生成器：docs/design/2026-07-22-frontend-redesign/oklch-to-srgb.py（零依赖标准库）")
    lines.append("//   重跑：python oklch-to-srgb.py")
    lines.append("//")
    lines.append("// 转换口径：OKLCH -> OKLab -> 线性 sRGB -> gamma 编码；越界通道在线性段截断")
    lines.append(f"// clamp[0,1]（逐通道，非色域映射）。本次生成色域截断：{clamp_note}。")
    lines.append("//")
    lines.append("// 与 theme.slint（MaterialTheme）并存、互不干扰；FR-T3 起组件逐步换绑到本调色板，")
    lines.append("// 组件侧统一读 RedesignTheme.t.<token>。")
    lines.append("// ============================================================================")
    lines.append("")
    lines.append("// 完整 token 集（忠实 tokens-draft.css 命名，kebab-case 沿用 slint 惯例）")
    lines.append("export struct RedesignTokens {")
    last_group = None
    for name, ftype in struct_fields:
        g = group_of(name)
        if g != last_group:
            lines.append(f"    // {GROUP_COMMENTS[g]}")
            last_group = g
        lines.append(f"    {name}: {ftype},")
    lines.append("}")
    lines.append("")

    def instance_block(prop_name, banner, tokens_by_name, mind_by=None):
        # 结构 token 亮暗同值，两套实例均携带（struct 字面量要求全字段）
        fields = []
        for name in color_order:
            tok = tokens_by_name[name]
            tail = f" // {fmt_oklch(tok)}"
            if prop_name == "LIGHT" and tok["comment"]:
                tail += f" · {tok['comment']}"
            fields.append((f"{name}: {tok['hex']}", tail))
        if mind_by is not None and mind:
            for name, _ftype in mind[0]:
                fields.append((f"{name}: {mind_by[name]}", ""))
        for name, value, _note in root_tokens:
            fields.append((f"{name}: {value}", ""))
        block = []
        block.append(f"    // {banner}")
        block.append(f"    out property <RedesignTokens> {prop_name}: {{")
        for i, (code, tail) in enumerate(fields):
            # struct 字面量保守写法：末字段不带尾逗号
            comma = "," if i < len(fields) - 1 else ""
            block.append(f"        {code}{comma}{tail}")
        block.append("    };")
        return block

    lines.append("// 亮/暗两个常量实例 + dark 三元翻转（参 theme.slint getter 精神，")
    lines.append("// 以 struct 三元代替逐 token getter；FR-T5 接「跟随系统/亮/暗」显示模式）。")
    lines.append("export global RedesignTheme {")
    lines.append("    // 暗色开关。默认 true，与现网 MaterialTheme.dark-mode 默认一致；")
    lines.append("    // tokens-draft.css 以 light 兼作 :root 默认，翻转仅改变本全局取值。")
    lines.append("    in-out property <bool> dark: true;")
    lines.append("")
    lines.extend(instance_block(
        "LIGHT",
        "LIGHT —— 心理咨询室白灰（tokens-draft.css :root / [data-theme=\"light\"]）",
        light_by,
        mind[1] if mind else None,
    ))
    lines.append("")
    lines.extend(instance_block(
        "DARK",
        "DARK —— 灰黑锚（tokens-draft.css [data-theme=\"dark\"]；[T1][T4] 亮度台阶为推导值，P1.1 走查定稿）",
        dark_by,
        mind[2] if mind else None,
    ))
    lines.append("")
    lines.append("    // 当前生效 token 集：dark ? DARK : LIGHT")
    lines.append("    out property <RedesignTokens> t: dark ? DARK : LIGHT;")
    lines.append("")
    lines.append("    // 动效时长常量（设计人格「慢重向下一次性」；retarget-notes：CSS 动效翻译为")
    lines.append("    // slint property animation，保守动效恰是 Slint 主场）")
    lines.append("    out property <duration> dur-normal: 350ms; // 常规过渡（如工具 chip 暖->冷）")
    lines.append("    out property <duration> dur-once: 1200ms; // 一次性入场动效")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


# ----------------------------------------------------------------------------
# 主流程
# ----------------------------------------------------------------------------

def main():
    if not CSS_PATH.exists():
        fail(f"找不到单一事实源: {CSS_PATH}")
    root_tokens, light, dark = parse_css(CSS_PATH.read_text(encoding="utf-8"))

    light_names = [t["name"] for t in light]
    dark_names = [t["name"] for t in dark]
    if len(set(light_names)) != len(light_names):
        fail(f"light 块存在重名 token: {light_names}")
    if set(light_names) != set(dark_names):
        fail(
            "light/dark 颜色 token 集合不一致: "
            f"仅 light={sorted(set(light_names) - set(dark_names))}, "
            f"仅 dark={sorted(set(dark_names) - set(light_names))}"
        )
    dark_by = {t["name"]: t for t in dark}

    # 解析 color-mix 引用（OKLCH 混色：两色 OKLCH→OKLab 按权重线性插值→走既有管线）
    light = resolve_color_mix_list(light)
    dark = resolve_color_mix_list(dark)
    dark_by = {t["name"]: t for t in dark}

    # 转换
    clamped_all = []
    for mode, tokens in (("light", light), ("dark", dark)):
        for tok in tokens:
            r, g, b, clamped = oklch_to_hex(tok["L"], tok["C"], tok["H"])
            tok["hex"] = "#%02X%02X%02X" % (r, g, b)
            tok["rgb"] = (r, g, b)
            for ch, val in clamped:
                clamped_all.append((mode, tok["name"], ch, val))

    # 亮色源 hex 回差校验（rep-* 注释中的 fallback 珊瑚 hex 不参与）
    checks = []
    for tok in light:
        if "fallback" in tok["comment"]:
            continue
        m = SOURCE_HEX_RE.search(tok["comment"])
        if not m:
            continue
        src = m.group(1).upper()
        sr, sg, sb = (int(src[i:i + 2], 16) for i in (0, 2, 4))
        cr, cg, cb = tok["rgb"]
        delta = max(abs(cr - sr), abs(cg - sg), abs(cb - sb))
        checks.append((tok["name"], tok["hex"], f"#{src}", delta))
        if delta > 4:
            fail(
                f"亮色 --{tok['name']} 换算 {tok['hex']} 与源 #{src} 回差 Δ={delta} 超限，"
                "请核对转换数学或 css 数值"
            )

    # mind 色维度（consult-room v2 spike；独立于 css 源，参数内联于本生成器）
    mind = mind_tokens()

    # 写产物
    TABLE_PATH.write_text(
        gen_md(root_tokens, light, dark_by, checks, clamped_all, mind_rows=mind[3]),
        encoding="utf-8",
    )
    SLINT_PATH.parent.mkdir(parents=True, exist_ok=True)
    SLINT_PATH.write_text(
        gen_slint(root_tokens, light, dark_by, clamped_all, mind=mind),
        encoding="utf-8",
    )

    # 摘要
    print(
        f"[oklch-to-srgb] 解析完成: 结构 token {len(root_tokens)} 个, "
        f"light {len(light)} 色, dark {len(dark)} 色"
    )
    for name, calc, src, delta in checks:
        print(f"[oklch-to-srgb]   校验 {name}: 计算 {calc} vs 源 {src} (Δmax={delta})")
    if clamped_all:
        for mode, name, ch, val in clamped_all:
            print(f"[oklch-to-srgb]   截断 {mode}/{name}.{ch}: 线性值 {val:.6f} -> clamp")
    else:
        print("[oklch-to-srgb]   色域截断: 无")
    print(f"[oklch-to-srgb] 写出 {TABLE_PATH}")
    print(f"[oklch-to-srgb] 写出 {SLINT_PATH}")


if __name__ == "__main__":
    main()
