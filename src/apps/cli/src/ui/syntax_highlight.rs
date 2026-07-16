/// Syntax highlighting module for TUI
///
/// Uses `syntect` for syntax analysis (when the `syntax-highlight` feature
/// is enabled) and converts highlighted output into ratatui `Span`s.
/// Falls back to plain text rendering when the feature is disabled.

/// Which syntect theme to use based on our app theme
// reason: HighlightTheme enum is kept for theme-aware syntax highlighting;
// the default theme path currently bypasses it
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum HighlightTheme {
    Dark,
    Light,
}

/// Extract file extension from a file path.
/// Returns "txt" if no extension is found.
pub fn ext_from_path(path: &str) -> &str {
    path.rsplit('.')
        .next()
        .filter(|ext| ext.len() <= 10 && !ext.contains('/') && !ext.contains('\\'))
        .unwrap_or("txt")
}

// ---------------------------------------------------------------------------
// Active implementation (syntect-based, feature = "syntax-highlight")
// ---------------------------------------------------------------------------

#[cfg(feature = "syntax-highlight")]
mod impl_syntax {
    use once_cell::sync::Lazy;
    use ratatui::{
        style::Style,
        text::{Line, Span},
    };
    use syntect::{
        easy::HighlightLines,
        highlighting::{Style as SyntectStyle, ThemeSet},
        parsing::SyntaxSet,
        util::LinesWithEndings,
    };

    use super::HighlightTheme;

    /// Global syntax set (loaded once, shared across all highlight calls)
    static SYNTAX_SET: Lazy<SyntaxSet> =
        Lazy::new(SyntaxSet::load_defaults_newlines);

    /// Global theme set (loaded once)
    static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

    impl HighlightTheme {
        fn syntect_theme_name(&self) -> &'static str {
            match self {
                HighlightTheme::Dark => "base16-ocean.dark",
                HighlightTheme::Light => "InspiredGitHub",
            }
        }
    }

    /// Highlight a block of code and return ratatui Lines.
    ///
    /// - `content`: the source code text
    /// - `file_ext`: file extension for language detection (e.g. "rs", "py")
    /// - `hl_theme`: dark or light theme
    ///
    /// Falls back to plain text if the language is not recognized.
    // reason: highlight_code() reserved for upcoming code-block rendering in
    // chat replies (today tool output bypasses it)
    #[allow(dead_code)]
    pub fn highlight_code<'a>(
        content: &str,
        file_ext: &str,
        hl_theme: HighlightTheme,
    ) -> Vec<Line<'a>> {
        let syntax = SYNTAX_SET
            .find_syntax_by_extension(file_ext)
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

        let theme =
            &THEME_SET.themes[hl_theme.syntect_theme_name()];
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut lines = Vec::new();

        for line_str in LinesWithEndings::from(content) {
            match highlighter.highlight_line(line_str, &SYNTAX_SET) {
                Ok(ranges) => {
                    let spans: Vec<Span<'a>> = ranges
                        .into_iter()
                        .map(|(style, text)| {
                            Span::styled(
                                text.to_string(),
                                syntect_to_ratatui_style(&style),
                            )
                        })
                        .collect();
                    lines.push(Line::from(spans));
                }
                Err(_) => {
                    lines.push(Line::from(Span::raw(
                        line_str
                            .trim_end_matches('\n')
                            .to_string(),
                    )));
                }
            }
        }

        lines
    }

    /// Highlight a single bash command line (e.g. "ls -la --color").
    /// Returns a Line with syntax-highlighted spans.
    pub fn highlight_bash_command<'a>(
        command: &str,
        hl_theme: HighlightTheme,
    ) -> Line<'a> {
        let syntax = SYNTAX_SET
            .find_syntax_by_extension("sh")
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

        let theme =
            &THEME_SET.themes[hl_theme.syntect_theme_name()];
        let mut highlighter = HighlightLines::new(syntax, theme);

        let input = if command.ends_with('\n') {
            command.to_string()
        } else {
            format!("{}\n", command)
        };

        match highlighter.highlight_line(&input, &SYNTAX_SET) {
            Ok(ranges) => {
                let spans: Vec<Span<'a>> = ranges
                    .into_iter()
                    .map(|(style, text)| {
                        let clean =
                            text.trim_end_matches('\n').to_string();
                        Span::styled(
                            clean,
                            syntect_to_ratatui_style(&style),
                        )
                    })
                    .filter(|s| !s.content.is_empty())
                    .collect();
                Line::from(spans)
            }
            Err(_) => {
                Line::from(Span::raw(command.to_string()))
            }
        }
    }

    /// Highlight bash output text (uses plain text / console syntax).
    /// Returns lines with minimal styling.
    // reason: highlight_bash_output() reserved for upcoming bash-output
    // syntax coloring (today shell output is rendered plain)
    #[allow(dead_code)]
    pub fn highlight_bash_output<'a>(
        output: &str,
        hl_theme: HighlightTheme,
    ) -> Vec<Line<'a>> {
        let _ = hl_theme;
        output
            .lines()
            .map(|line| {
                Line::from(Span::raw(line.to_string()))
            })
            .collect()
    }

    /// Highlight code with line numbers prepended.
    ///
    /// Returns lines in the format: `{line_number} | {highlighted_code}`
    pub fn highlight_code_with_line_numbers<'a>(
        content: &str,
        file_ext: &str,
        hl_theme: HighlightTheme,
        line_num_style: Style,
        separator_style: Style,
    ) -> Vec<Line<'a>> {
        let syntax = SYNTAX_SET
            .find_syntax_by_extension(file_ext)
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

        let theme =
            &THEME_SET.themes[hl_theme.syntect_theme_name()];
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut lines = Vec::new();

        let total_lines = content.lines().count();
        let num_width = total_lines.to_string().len().max(3);

        for (i, line_str) in
            LinesWithEndings::from(content).enumerate()
        {
            let line_num = i + 1;
            let mut spans = vec![
                Span::styled(
                    format!(
                        "{:>width$}",
                        line_num,
                        width = num_width
                    ),
                    line_num_style,
                ),
                Span::styled(" \u{2502} ", separator_style),
            ];

            match highlighter.highlight_line(line_str, &SYNTAX_SET) {
                Ok(ranges) => {
                    for (style, text) in ranges {
                        spans.push(Span::styled(
                            text.trim_end_matches('\n').to_string(),
                            syntect_to_ratatui_style(&style),
                        ));
                    }
                }
                Err(_) => {
                    spans.push(Span::raw(
                        line_str
                            .trim_end_matches('\n')
                            .to_string(),
                    ));
                }
            }

            lines.push(Line::from(spans));
        }

        lines
    }

    /// Convert a syntect Style to a ratatui Style.
    fn syntect_to_ratatui_style(style: &SyntectStyle) -> Style {
        let fg = style.foreground;
        Style::default().fg(ratatui::style::Color::Rgb(
            fg.r, fg.g, fg.b,
        ))
    }
}

// ---------------------------------------------------------------------------
// Fallback implementation (plain text, feature disabled)
// ---------------------------------------------------------------------------

#[cfg(not(feature = "syntax-highlight"))]
mod impl_plain {
    use ratatui::{
        style::Style,
        text::{Line, Span},
    };

    use super::HighlightTheme;

    /// Plain-text fallback for code highlighting.
    #[allow(dead_code)]
    pub fn highlight_code<'a>(
        content: &str,
        _file_ext: &str,
        _hl_theme: HighlightTheme,
    ) -> Vec<Line<'a>> {
        content
            .lines()
            .map(|line| {
                Line::from(Span::raw(line.to_string()))
            })
            .collect()
    }

    /// Plain-text fallback for bash command highlighting.
    pub fn highlight_bash_command<'a>(
        command: &str,
        _hl_theme: HighlightTheme,
    ) -> Line<'a> {
        Line::from(Span::raw(command.to_string()))
    }

    /// Plain-text fallback for bash output highlighting.
    #[allow(dead_code)]
    pub fn highlight_bash_output<'a>(
        output: &str,
        _hl_theme: HighlightTheme,
    ) -> Vec<Line<'a>> {
        output
            .lines()
            .map(|line| {
                Line::from(Span::raw(line.to_string()))
            })
            .collect()
    }

    /// Plain-text fallback: code with line numbers, no colour.
    pub fn highlight_code_with_line_numbers<'a>(
        content: &str,
        _file_ext: &str,
        _hl_theme: HighlightTheme,
        line_num_style: Style,
        separator_style: Style,
    ) -> Vec<Line<'a>> {
        let total_lines = content.lines().count();
        let num_width = total_lines.to_string().len().max(3);

        content
            .lines()
            .enumerate()
            .map(|(i, line_str)| {
                let line_num = i + 1;
                Line::from(vec![
                    Span::styled(
                        format!(
                            "{:>width$}",
                            line_num,
                            width = num_width
                        ),
                        line_num_style,
                    ),
                    Span::styled(" \u{2502} ", separator_style),
                    Span::raw(line_str.to_string()),
                ])
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Re-export the active implementation under the module namespace
// ---------------------------------------------------------------------------

#[cfg(feature = "syntax-highlight")]
pub use impl_syntax::*;

#[cfg(not(feature = "syntax-highlight"))]
pub use impl_plain::*;
