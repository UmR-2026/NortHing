use super::format::{html_escape, js_escape, markdown_inline};
use super::theme::HtmlLabels;
use crate::agentic::insights::types::*;

pub(crate) fn render_interaction_style(style: &InteractionStyle, l: &HtmlLabels) -> String {
    if style.narrative.is_empty() && style.key_patterns.is_empty() {
        return format!(r#"<div class="empty">{}</div>"#, html_escape(l.no_interaction_style));
    }

    let patterns_html = if style.key_patterns.is_empty() {
        String::new()
    } else {
        let items: Vec<String> = style
            .key_patterns
            .iter()
            .map(|p| format!(r#"<div class="key-insight">{}</div>"#, markdown_inline(p)))
            .collect();
        items.join("\n")
    };

    format!(
        r#"<div class="narrative">
  <p>{}</p>
  {}
</div>"#,
        markdown_inline(&style.narrative),
        patterns_html,
    )
}

pub(crate) fn render_big_wins(wins: &[BigWin], l: &HtmlLabels) -> String {
    if wins.is_empty() {
        return format!(r#"<div class="empty">{}</div>"#, html_escape(l.no_big_wins));
    }

    let items: Vec<String> = wins
        .iter()
        .map(|w| {
            let impact_html = if w.impact.is_empty() {
                String::new()
            } else {
                format!(r#"<div class="big-win-impact">{}</div>"#, markdown_inline(&w.impact))
            };
            format!(
                r#"<div class="big-win">
  <div class="big-win-title">{}</div>
  <div class="big-win-desc">{}</div>
  {}
</div>"#,
                html_escape(&w.title),
                markdown_inline(&w.description),
                impact_html,
            )
        })
        .collect();

    format!(r#"<div class="big-wins">{}</div>"#, items.join("\n"))
}

pub(crate) fn render_friction_categories(categories: &[FrictionCategory], l: &HtmlLabels) -> String {
    if categories.is_empty() {
        return format!(r#"<div class="empty">{}</div>"#, html_escape(l.no_friction));
    }

    let items: Vec<String> = categories
        .iter()
        .map(|f| {
            let examples_html = if f.examples.is_empty() {
                String::new()
            } else {
                let lis: Vec<String> = f
                    .examples
                    .iter()
                    .map(|e| format!("<li>{}</li>", markdown_inline(e)))
                    .collect();
                format!(
                    r#"<ul class="friction-examples">{}</ul>"#,
                    lis.join("\n")
                )
            };

            let suggestion_html = if f.suggestion.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<div class="key-insight" style="background:#fef2f2;border-color:#fca5a5;color:#991b1b;margin-top:10px">{}</div>"#,
                    markdown_inline(&f.suggestion)
                )
            };

            format!(
                r#"<div class="friction-category">
  <div class="friction-title">{}</div>
  <div class="friction-desc">{}</div>
  {}
  {}
</div>"#,
                html_escape(&f.category),
                markdown_inline(&f.description),
                examples_html,
                suggestion_html,
            )
        })
        .collect();

    format!(r#"<div class="friction-categories">{}</div>"#, items.join("\n"))
}

pub(crate) fn render_suggestions(suggestions: &InsightsSuggestions, l: &HtmlLabels) -> String {
    let mut sections = Vec::new();

    if !suggestions.northhing_md_additions.is_empty() {
        let items: Vec<String> = suggestions
            .northhing_md_additions
            .iter()
            .enumerate()
            .map(|(i, md)| {
                format!(
                    r#"<div class="claude-md-item">
  <input type="checkbox" class="cmd-checkbox" id="md-{i}" checked>
  <div class="cmd-code">{}</div>
  <button class="copy-btn" onclick="copyText(this, '{}')">&nbsp;Copy&nbsp;</button>
  <div class="cmd-why">{}</div>
</div>"#,
                    html_escape(&md.content),
                    js_escape(&md.content),
                    html_escape(&md.rationale),
                    i = i,
                )
            })
            .collect();

        sections.push(format!(
            r#"<div class="claude-md-section">
  <h3>{md_title}</h3>
  <div class="claude-md-actions">
    <button class="copy-all-btn" onclick="copyAllChecked(this)">{copy_all}</button>
  </div>
  {items}
</div>"#,
            md_title = html_escape(l.md_additions),
            copy_all = html_escape(l.copy_all_checked),
            items = items.join("\n"),
        ));
    }

    if !suggestions.features_to_try.is_empty() {
        let items: Vec<String> = suggestions
            .features_to_try
            .iter()
            .map(|f| {
                let code_html = if f.example_usage.is_empty() {
                    String::new()
                } else {
                    format!(
                        r#"<div class="feature-code">
  <code>{}</code>
  <button class="copy-btn" onclick="copyText(this, '{}')">&nbsp;Copy&nbsp;</button>
</div>"#,
                        html_escape(&f.example_usage),
                        js_escape(&f.example_usage),
                    )
                };

                format!(
                    r#"<div class="feature-card">
  <div class="feature-title">{}</div>
  <div class="feature-oneliner">{}</div>
  <div class="feature-why">{}</div>
  {}
</div>"#,
                    html_escape(&f.feature),
                    markdown_inline(&f.description),
                    markdown_inline(&f.benefit),
                    code_html,
                )
            })
            .collect();

        sections.push(format!(
            r#"<h3 id="section-features">{}</h3>
<div class="features-section">{}</div>"#,
            html_escape(l.features_to_try),
            items.join("\n")
        ));
    }

    if !suggestions.usage_patterns.is_empty() {
        let items: Vec<String> = suggestions
            .usage_patterns
            .iter()
            .map(|p| {
                let detail_html = if p.detail.is_empty() {
                    String::new()
                } else {
                    format!(r#"<div class="pattern-detail">{}</div>"#, markdown_inline(&p.detail))
                };

                let prompt_html = if p.suggested_prompt.is_empty() {
                    String::new()
                } else {
                    format!(
                        r#"<div class="pattern-prompt">
  <div class="prompt-label">{}</div>
  <code>{}</code>
  <button class="copy-btn" onclick="copyText(this, '{}')">&nbsp;Copy&nbsp;</button>
</div>"#,
                        html_escape(l.try_this_prompt),
                        html_escape(&p.suggested_prompt),
                        js_escape(&p.suggested_prompt),
                    )
                };

                format!(
                    r#"<div class="pattern-card">
  <div class="pattern-title">{}</div>
  <div class="pattern-summary">{}</div>
  {}
  {}
</div>"#,
                    html_escape(&p.pattern),
                    markdown_inline(&p.description),
                    detail_html,
                    prompt_html,
                )
            })
            .collect();

        sections.push(format!(
            r#"<h3 id="section-patterns">{}</h3>
<div class="patterns-section">{}</div>"#,
            html_escape(l.usage_patterns),
            items.join("\n")
        ));
    }

    sections.join("\n")
}

pub(crate) fn render_horizon(intro: &str, workflows: &[HorizonWorkflow], l: &HtmlLabels) -> String {
    if workflows.is_empty() {
        return format!(r#"<div class="empty">{}</div>"#, html_escape(l.no_horizon));
    }

    let intro_html = if intro.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="section-intro">{}</p>"#, markdown_inline(intro))
    };

    let items: Vec<String> = workflows
        .iter()
        .map(|h| {
            let how_to_try_html = if h.how_to_try.is_empty() {
                String::new()
            } else {
                format!(r#"<div class="horizon-tip">{}</div>"#, markdown_inline(&h.how_to_try))
            };

            let prompt_html = if h.copyable_prompt.is_empty() {
                String::new()
            } else {
                let escaped = html_escape(&h.copyable_prompt);
                let js_escaped = h
                    .copyable_prompt
                    .replace('\\', "\\\\")
                    .replace('\'', "\\'")
                    .replace('\n', "\\n");
                format!(
                    r#"<div class="horizon-prompt">
  <div class="prompt-label">{try_prompt}</div>
  <div class="feature-code">
    <code>{code}</code>
    <button class="copy-btn" onclick="copyText(this, '{js_code}')">Copy</button>
  </div>
</div>"#,
                    try_prompt = html_escape(l.try_this_prompt),
                    code = escaped,
                    js_code = js_escaped,
                )
            };

            format!(
                r#"<div class="horizon-card">
  <div class="horizon-title">{}</div>
  <div class="horizon-possible">{}</div>
  {}
  {}
</div>"#,
                html_escape(&h.title),
                markdown_inline(&h.whats_possible),
                how_to_try_html,
                prompt_html,
            )
        })
        .collect();

    format!(
        r#"{}<div class="horizon-section">{}</div>"#,
        intro_html,
        items.join("\n")
    )
}

pub(crate) fn render_fun_ending(ending: &Option<FunEnding>) -> String {
    match ending {
        Some(fe) => format!(
            r#"<div class="fun-ending">
  <div class="fun-headline">{}</div>
  <div class="fun-detail">{}</div>
</div>"#,
            html_escape(&fe.headline),
            markdown_inline(&fe.detail),
        ),
        None => String::new(),
    }
}
