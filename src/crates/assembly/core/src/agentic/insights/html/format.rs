pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Convert simple markdown inline formatting to HTML.
/// Handles **bold** and *italic* after html_escape.
pub fn markdown_inline(s: &str) -> String {
    let escaped = html_escape(s);
    let mut result = String::with_capacity(escaped.len() + 64);
    let chars: Vec<char> = escaped.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
            if let Some(end) = find_closing_double_star(&chars, i + 2) {
                result.push_str("<strong>");
                for &c in &chars[i + 2..end] {
                    result.push(c);
                }
                result.push_str("</strong>");
                i = end + 2;
                continue;
            }
        }
        if chars[i] == '*' && (i + 1 < len && chars[i + 1] != '*') {
            if let Some(end) = find_closing_single_star(&chars, i + 1) {
                result.push_str("<em>");
                for &c in &chars[i + 1..end] {
                    result.push(c);
                }
                result.push_str("</em>");
                i = end + 1;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

fn find_closing_double_star(chars: &[char], start: usize) -> Option<usize> {
    let len = chars.len();
    let mut i = start;
    while i + 1 < len {
        if chars[i] == '*' && chars[i + 1] == '*' && i > start {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_closing_single_star(chars: &[char], start: usize) -> Option<usize> {
    let len = chars.len();
    let mut i = start;
    while i < len {
        if chars[i] == '*' && (i + 1 >= len || chars[i + 1] != '*') && i > start {
            return Some(i);
        }
        i += 1;
    }
    None
}

pub fn js_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "")
}

pub fn format_duration_short(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else if secs < 3600.0 {
        format!("{:.1}m", secs / 60.0)
    } else {
        format!("{:.1}h", secs / 3600.0)
    }
}

pub fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
