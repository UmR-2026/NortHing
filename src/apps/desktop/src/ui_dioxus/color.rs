// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Color utility functions extracted from app.rs (W8-4) — pure CSS gradient helpers.
// Use: `use crate::ui_dioxus::color::{parse_hex_rgb, mix_hex, chronicle_gradient};`

/// Parse a `#RRGGBB` hex string into `(r, g, b)` u8 components.
///
/// Returns `None` for invalid input (wrong length, non-hex chars).
pub fn parse_hex_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let s = hex.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Linearly interpolate between two hex colors at ratio `t` (clamped [0,1]).
///
/// Returns `b` if `a` is invalid, `a` if `b` is invalid.
pub fn mix_hex(a: &str, b: &str, t: f64) -> String {
    let (ar, ag, ab) = match parse_hex_rgb(a) {
        Some(c) => c,
        None => return b.to_string(),
    };
    let (br, bg, bb) = match parse_hex_rgb(b) {
        Some(c) => c,
        None => return a.to_string(),
    };
    let t = t.clamp(0.0, 1.0);
    let r = (ar as f64 + (br as f64 - ar as f64) * t).round().clamp(0.0, 255.0) as u8;
    let g = (ag as f64 + (bg as f64 - ag as f64) * t).round().clamp(0.0, 255.0) as u8;
    let b_val = (ab as f64 + (bb as f64 - ab as f64) * t).round().clamp(0.0, 255.0) as u8;
    format!("#{:02X}{:02X}{:02X}", r, g, b_val)
}

/// Build a CSS `linear-gradient(90deg, ...)` string from a history of hex colors
/// fading from `BIRTH` (#DAD6CF) into the history, ending with `current`.
pub fn chronicle_gradient(history: &[String], current: &str) -> String {
    const BIRTH: &str = "#DAD6CF";
    let n = history.len();
    let mut stops = Vec::with_capacity(n + 1);

    if n == 0 {
        stops.push(format!("{BIRTH} 0.00%"));
    } else {
        for (i, c) in history.iter().enumerate() {
            let (pos, col) = if n == 1 {
                (0.0, c.clone())
            } else {
                let frac = i as f64 / (n - 1) as f64;
                let pos = frac * 70.0;
                let col = if i == 0 {
                    c.clone()
                } else {
                    let t = 0.18 + 0.82 * frac;
                    mix_hex(BIRTH, c, t)
                };
                (pos, col)
            };
            stops.push(format!("{col} {pos:.2}%"));
        }
    }

    stops.push(format!("{current} 100%"));
    format!("linear-gradient(90deg, {})", stops.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Existing assertions preserved verbatim
    #[test]
    fn test_mix_hex() {
        assert_eq!(mix_hex("#DAD6CF", "#3F837B", 1.0), "#3F837B");
        assert_eq!(mix_hex("#DAD6CF", "#3F837B", 0.0), "#DAD6CF");
    }

    #[test]
    fn test_chronicle_gradient_single() {
        let grad = chronicle_gradient(&["#DAD6CF".to_string()], "#C8714C");
        assert!(grad.contains("0.00%"));
        assert!(grad.contains("100%"));
    }

    #[test]
    fn test_chronicle_gradient_three_history() {
        let history = vec!["#DAD6CF".to_string(), "#3F837B".to_string(), "#8B5FBF".to_string()];
        let grad = chronicle_gradient(&history, "#C8714C");
        assert!(grad.contains("0.00%"));
        assert!(grad.contains("35.00%"));
        assert!(grad.contains("70.00%"));
        assert!(grad.contains("100%"));
    }

    // W8-4: boundary tests added per deep-rot observation
    #[test]
    fn test_parse_hex_rgb_invalid() {
        assert_eq!(parse_hex_rgb("#GGGGGG"), None); // non-hex chars
        assert_eq!(parse_hex_rgb("#FFF"), None);    // 3-char shorthand rejected
        assert_eq!(parse_hex_rgb("#"), None);       // empty after #
        assert_eq!(parse_hex_rgb("not-a-color"), None); // no hash
    }

    #[test]
    fn test_parse_hex_rgb_pure_black_white() {
        assert_eq!(parse_hex_rgb("#000000"), Some((0, 0, 0)));
        assert_eq!(parse_hex_rgb("#FFFFFF"), Some((255, 255, 255)));
    }

    #[test]
    fn test_mix_hex_invalid_fallback() {
        // invalid first → returns second; invalid second → returns first
        assert_eq!(mix_hex("#GGGGGG", "#3F837B", 0.5), "#3F837B");
        assert_eq!(mix_hex("#DAD6CF", "#BADINPUT", 0.5), "#DAD6CF");
        // empty history → current = same as current
        let grad = chronicle_gradient(&[], "#000000");
        assert!(grad.contains("#000000 100%"));
        assert!(grad.contains("#DAD6CF 0.00%"));
    }

    #[test]
    fn test_chronicle_gradient_extremes() {
        // pure black current
        let grad = chronicle_gradient(&["#FFFFFF".to_string()], "#000000");
        assert!(grad.contains("#000000 100%"));
        // pure white current
        let grad = chronicle_gradient(&["#000000".to_string()], "#FFFFFF");
        assert!(grad.contains("#FFFFFF 100%"));
    }
}
