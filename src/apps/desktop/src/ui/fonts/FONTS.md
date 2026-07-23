# Fonts — northing desktop (Slint)

> Slint 1.17 custom font mechanism: `import "./fonts/Xxx.ttf"` in .slint file.
> Supported formats: `.ttf`, `.ttc`, `.otf` only. **woff2 NOT supported.**
> Variable font axes: TTF loads but NO `font-variation-settings` property exists;
> only `font-weight` (100–900) and `font-italic` are controllable.
> Therefore Fraunces is pre-instanced to static TTF; Noto Sans SC keeps wght axis
> (Slint `font-weight` selects along it).

## Families

| File | font-family | Weight | Style | Source | Version | License | Bytes |
|---|---|---|---|---|---|---|---|
| Fraunces-Regular.ttf | `Fraunces` | 400 | Regular | google/fonts (Fraunces variable) | 2024 | OFL-1.1 | 72,788 |
| Fraunces-Display.ttf | `Fraunces Display` | 600 | SemiBold | google/fonts (Fraunces variable) | 2024 | OFL-1.1 | 72,780 |
| Fraunces-Italic.ttf | `Fraunces` | 400 | Italic | google/fonts (Fraunces Italic variable) | 2024 | OFL-1.1 | 88,968 |
| NotoSansSC.ttf | `Noto Sans SC` | 100–900 (variable) | Regular | google/fonts (Noto Sans SC variable) | 2024 | OFL-1.1 | 1,777,952 |
| JetBrainsMono.ttf | `JetBrains Mono` | 400 | Regular | JetBrains (google/fonts mirror) | 2024 | OFL-1.1 | 300,144 |

## Instantiation axis values (Fraunces)

Design spec: `font-variation-settings: "WONK" 1, "SOFT" 60` (handoff §8).

| Instance | wght | SOFT | WONK | opsz | Rationale |
|---|---|---|---|---|---|
| Regular (text grade) | 400 | 60 | 1 | 14 | Body Latin / general UI text at 13–15px |
| Display (brand grade) | 600 | 60 | 1 | 144 | Brand "northing" / card name display at 16px+ |
| Italic | 400 | 60 | 1 | 14 | Emphasis / Latin italic |

Source variable font axes: opsz 9–144 (default 9), wght 100–900 (default 900), SOFT 0–100 (default 0), WONK 0–1 (default 1).

## Noto Sans SC subset

- Source: NotoSansSC.ttf variable (17.7 MB, wght 100–900)
- Charset: 通用规范 3500 字 + 珊 (U+745A) + ASCII printable (U+0020–007E) + CJK/fullwidth punctuation = 3,655 unique codepoints
- Output: TTF with wght axis preserved (Slint `font-weight` selects 400/500 per design)
- Design weights: body 400, names/titles 500 (handoff §7.1, §7.4)

## Usage in .slint (for FR-T3)

```slint
import "./fonts/Fraunces-Regular.ttf";
import "./fonts/Fraunces-Display.ttf";
import "./fonts/Fraunces-Italic.ttf";
import "./fonts/NotoSansSC.ttf";
import "./fonts/JetBrainsMono.ttf";
```

Then reference via `font-family`:
- Brand/display Latin: `font-family: "Fraunces Display";`
- Regular Latin: `font-family: "Fraunces";`
- CJK / agent name: `font-family: "Noto Sans SC";`
- Metadata mono: `font-family: "JetBrains Mono";`

## Licenses

All fonts are OFL-1.1. Full license texts:
- `OFL-Fraunces.txt`
- `OFL-JetBrainsMono.txt`
- `OFL-NotoSansSC.txt`
