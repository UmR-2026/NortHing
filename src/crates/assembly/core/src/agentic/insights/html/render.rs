use super::format::{format_duration_short, format_number, html_escape, markdown_inline};
use super::section::{
    render_big_wins, render_friction_categories, render_fun_ending, render_horizon, render_interaction_style,
    render_suggestions,
};
use super::theme::{HtmlLabels, CSS_STYLES, JS_SCRIPT};
use crate::agentic::insights::types::*;

pub fn generate_html(report: &InsightsReport, locale: &str) -> String {
    let l = HtmlLabels::for_locale(locale);

    let subtitle = l
        .subtitle_template
        .replace("{msgs}", &report.total_messages.to_string())
        .replace("{sessions}", &report.total_sessions.to_string())
        .replace("{analyzed}", &report.analyzed_sessions.to_string())
        .replace(
            "{start}",
            &report.date_range.start[..10.min(report.date_range.start.len())],
        )
        .replace("{end}", &report.date_range.end[..10.min(report.date_range.end.len())]);

    let at_a_glance = render_at_a_glance(&report.at_a_glance, &l);
    let nav_toc = render_nav_toc(&l);
    let stats_row = render_stats_row(report, &l);
    let project_areas = render_project_areas(&report.project_areas, &l);
    let basic_charts = render_basic_charts(&report.stats, &l);
    let interaction_style = render_interaction_style(&report.interaction_style, &l);
    let usage_charts = render_usage_charts(&report.stats, &l);
    let wins_intro_html = if report.wins_intro.is_empty() {
        String::new()
    } else {
        format!(
            r#"<p class="section-intro">{}</p>"#,
            markdown_inline(&report.wins_intro)
        )
    };
    let big_wins = render_big_wins(&report.big_wins, &l);
    let outcome_charts = render_outcome_charts(&report.stats, &l);
    let friction_intro_html = if report.friction_intro.is_empty() {
        String::new()
    } else {
        format!(
            r#"<p class="section-intro">{}</p>"#,
            markdown_inline(&report.friction_intro)
        )
    };
    let friction = render_friction_categories(&report.friction_categories, &l);
    let friction_charts = render_friction_charts(&report.stats, &l);
    let suggestions = render_suggestions(&report.suggestions, &l);
    let horizon = render_horizon(&report.horizon_intro, &report.on_the_horizon, &l);
    let fun_ending = render_fun_ending(&report.fun_ending);

    let js_with_labels = JS_SCRIPT
        .replace("__COPIED__", l.copied)
        .replace("__COPY_ALL_CHECKED__", l.copy_all_checked);

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>{page_title}</title>
  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
  <style>
{CSS}
  </style>
</head>
<body>
  <div class="container">
    <h1>{page_title}</h1>
    <p class="subtitle">{subtitle}</p>

    {at_a_glance}
    {nav_toc}
    {stats_row}

    <h2 id="section-work">{section_work}</h2>
    {project_areas}

    {basic_charts}

    <h2 id="section-usage">{section_usage}</h2>
    {interaction_style}

    {usage_charts}

    <h2 id="section-wins">{section_wins}</h2>
    {wins_intro}
    {big_wins}

    {outcome_charts}

    <h2 id="section-friction">{section_friction}</h2>
    {friction_intro}
    {friction}

    {friction_charts}

    <h2 id="section-suggestions">{section_suggestions}</h2>
    {suggestions}

    <h2 id="section-horizon">{section_horizon}</h2>
    {horizon}

    {fun_ending}
  </div>
  <script>
{JS}
  </script>
</body>
</html>"#,
        CSS = CSS_STYLES,
        JS = js_with_labels,
        page_title = html_escape(l.title),
        subtitle = html_escape(&subtitle),
        section_work = html_escape(l.section_work),
        section_usage = html_escape(l.section_usage),
        section_wins = html_escape(l.section_wins),
        section_friction = html_escape(l.section_friction),
        section_suggestions = html_escape(l.section_suggestions),
        section_horizon = html_escape(l.section_horizon),
        at_a_glance = at_a_glance,
        nav_toc = nav_toc,
        stats_row = stats_row,
        project_areas = project_areas,
        basic_charts = basic_charts,
        interaction_style = interaction_style,
        usage_charts = usage_charts,
        wins_intro = wins_intro_html,
        big_wins = big_wins,
        outcome_charts = outcome_charts,
        friction_intro = friction_intro_html,
        friction = friction,
        friction_charts = friction_charts,
        suggestions = suggestions,
        horizon = horizon,
        fun_ending = fun_ending,
    )
}

fn render_at_a_glance(aag: &AtAGlance, l: &HtmlLabels) -> String {
    format!(
        r##"<div class="at-a-glance">
  <div class="glance-title">{title}</div>
  <div class="glance-sections">
    <div class="glance-section"><strong>{working}</strong> {working_text} <a href="#section-wins" class="see-more">{nav_wins} &rarr;</a></div>
    <div class="glance-section"><strong>{hindering}</strong> {hindering_text} <a href="#section-friction" class="see-more">{nav_friction} &rarr;</a></div>
    <div class="glance-section"><strong>{quick}</strong> {quick_text} <a href="#section-suggestions" class="see-more">{nav_suggestions} &rarr;</a></div>
    <div class="glance-section"><strong>{ahead}</strong> {ahead_text} <a href="#section-horizon" class="see-more">{nav_horizon} &rarr;</a></div>
  </div>
</div>"##,
        title = html_escape(l.at_a_glance),
        working = html_escape(l.whats_working),
        working_text = markdown_inline(&aag.whats_working),
        hindering = html_escape(l.whats_hindering),
        hindering_text = markdown_inline(&aag.whats_hindering),
        quick = html_escape(l.quick_wins),
        quick_text = markdown_inline(&aag.quick_wins),
        ahead = html_escape(l.looking_ahead),
        ahead_text = markdown_inline(&aag.looking_ahead),
        nav_wins = html_escape(l.section_wins),
        nav_friction = html_escape(l.section_friction),
        nav_suggestions = html_escape(l.section_suggestions),
        nav_horizon = html_escape(l.section_horizon),
    )
}

fn render_nav_toc(l: &HtmlLabels) -> String {
    format!(
        r##"<nav class="nav-toc">
  <a href="#section-work">{}</a>
  <a href="#section-usage">{}</a>
  <a href="#section-wins">{}</a>
  <a href="#section-friction">{}</a>
  <a href="#section-suggestions">{}</a>
  <a href="#section-horizon">{}</a>
</nav>"##,
        html_escape(l.nav_work),
        html_escape(l.nav_usage),
        html_escape(l.nav_wins),
        html_escape(l.nav_friction),
        html_escape(l.nav_suggestions),
        html_escape(l.nav_horizon),
    )
}

fn render_stats_row(report: &InsightsReport, l: &HtmlLabels) -> String {
    let response_time_stats = match (
        report.stats.median_response_time_secs,
        report.stats.avg_response_time_secs,
    ) {
        (Some(median), Some(avg)) => format!(
            r#"  <div class="stat"><div class="stat-value">{}</div><div class="stat-label">{}</div></div>
  <div class="stat"><div class="stat-value">{}</div><div class="stat-label">{}</div></div>"#,
            format_duration_short(median),
            html_escape(l.stat_median_response),
            format_duration_short(avg),
            html_escape(l.stat_avg_response),
        ),
        _ => String::new(),
    };

    let code_stats = if report.stats.total_lines_added > 0 || report.stats.total_lines_removed > 0 {
        format!(
            r#"  <div class="stat"><div class="stat-value">+{}/-{}</div><div class="stat-label">{}</div></div>
  <div class="stat"><div class="stat-value">{}</div><div class="stat-label">{}</div></div>"#,
            format_number(report.stats.total_lines_added),
            format_number(report.stats.total_lines_removed),
            html_escape(l.stat_lines),
            format_number(report.stats.total_files_modified),
            html_escape(l.stat_files),
        )
    } else {
        String::new()
    };

    format!(
        r#"<div class="stats-row">
{code_stats}
  <div class="stat"><div class="stat-value">{sessions}</div><div class="stat-label">{l_sessions}</div></div>
  <div class="stat"><div class="stat-value">{messages}</div><div class="stat-label">{l_messages}</div></div>
  <div class="stat"><div class="stat-value">{hours:.1}</div><div class="stat-label">{l_hours}</div></div>
  <div class="stat"><div class="stat-value">{days}</div><div class="stat-label">{l_days}</div></div>
  <div class="stat"><div class="stat-value">{mpd:.1}</div><div class="stat-label">{l_mpd}</div></div>
{response_time_stats}
</div>"#,
        sessions = report.total_sessions,
        messages = report.total_messages,
        hours = report.stats.total_hours,
        days = report.days_covered,
        mpd = report.stats.msgs_per_day,
        l_sessions = html_escape(l.stat_sessions),
        l_messages = html_escape(l.stat_messages),
        l_hours = html_escape(l.stat_hours),
        l_days = html_escape(l.stat_days),
        l_mpd = html_escape(l.stat_msgs_per_day),
    )
}

fn render_project_areas(areas: &[ProjectArea], l: &HtmlLabels) -> String {
    if areas.is_empty() {
        return format!(r#"<div class="empty">{}</div>"#, html_escape(l.no_project_areas));
    }

    let items: Vec<String> = areas
        .iter()
        .map(|a| {
            format!(
                r#"<div class="project-area">
  <div class="area-header">
    <span class="area-name">{name}</span>
    <span class="area-count">~{count} {suffix}</span>
  </div>
  <div class="area-desc">{desc}</div>
</div>"#,
                name = html_escape(&a.name),
                count = a.session_count,
                suffix = html_escape(l.sessions_suffix),
                desc = markdown_inline(&a.description),
            )
        })
        .collect();

    format!(r#"<div class="project-areas">{}</div>"#, items.join("\n"))
}

// ============ Charts split by section ============

fn render_basic_charts(stats: &InsightsStats, l: &HtmlLabels) -> String {
    let goals_chart = render_bar_chart(l.chart_goals, &stats.top_goals, "#2563eb", 6);
    let tools_chart = render_bar_chart(l.chart_tools, &stats.top_tools, "#0891b2", 6);

    let mut lang_items: Vec<(String, u32)> = stats.languages.iter().map(|(k, v)| (k.clone(), *v)).collect();
    lang_items.sort_by_key(|b| std::cmp::Reverse(b.1));
    lang_items.truncate(6);
    let lang_chart = render_bar_chart(l.chart_languages, &lang_items, "#10b981", 6);

    let mut type_items: Vec<(String, u32)> = stats.session_types.iter().map(|(k, v)| (k.clone(), *v)).collect();
    type_items.sort_by_key(|b| std::cmp::Reverse(b.1));
    type_items.truncate(6);
    let types_chart = render_bar_chart(l.chart_session_types, &type_items, "#8b5cf6", 6);

    let row1 = wrap_charts_row(&goals_chart, &tools_chart);
    let row2 = wrap_charts_row(&lang_chart, &types_chart);
    format!("{}{}", row1, row2)
}

fn render_usage_charts(stats: &InsightsStats, l: &HtmlLabels) -> String {
    let mut html = String::new();

    if !stats.response_time_buckets.is_empty() {
        let response_time_chart = render_response_time_chart(&stats.response_time_buckets, stats, l);
        html.push_str(&response_time_chart);
    }

    let time_of_day_chart = render_time_of_day_chart(&stats.hour_counts, l);

    let mut tool_error_items: Vec<(String, u32)> = stats.tool_errors.iter().map(|(k, v)| (k.clone(), *v)).collect();
    tool_error_items.sort_by_key(|b| std::cmp::Reverse(b.1));
    tool_error_items.truncate(6);
    let tool_errors_chart = render_bar_chart(l.chart_tool_errors, &tool_error_items, "#dc2626", 6);

    let mut agent_types_chart = String::new();
    if !stats.agent_types.is_empty() {
        let mut agent_type_items: Vec<(String, u32)> = stats.agent_types.iter().map(|(k, v)| (k.clone(), *v)).collect();
        agent_type_items.sort_by_key(|b| std::cmp::Reverse(b.1));
        agent_type_items.truncate(6);
        agent_types_chart = render_bar_chart(l.chart_agent_types, &agent_type_items, "#f97316", 6);
    }

    html.push_str(&wrap_charts_row(&time_of_day_chart, &tool_errors_chart));
    if !agent_types_chart.is_empty() {
        html.push_str(&wrap_charts_row(&agent_types_chart, ""));
    }

    html
}

fn render_outcome_charts(stats: &InsightsStats, l: &HtmlLabels) -> String {
    let has_success = !stats.success.is_empty();
    let has_outcomes = !stats.outcomes.is_empty();

    if !has_success && !has_outcomes {
        return String::new();
    }

    let mut success_items: Vec<(String, u32)> = stats.success.iter().map(|(k, v)| (k.clone(), *v)).collect();
    success_items.sort_by_key(|b| std::cmp::Reverse(b.1));
    success_items.truncate(6);
    let success_chart = render_bar_chart(l.chart_what_helped, &success_items, "#16a34a", 6);

    let mut outcome_items: Vec<(String, u32)> = stats.outcomes.iter().map(|(k, v)| (k.clone(), *v)).collect();
    outcome_items.sort_by_key(|b| std::cmp::Reverse(b.1));
    outcome_items.truncate(6);
    let outcomes_chart = render_bar_chart(l.chart_outcomes, &outcome_items, "#8b5cf6", 6);

    wrap_charts_row(&success_chart, &outcomes_chart)
}

fn render_friction_charts(stats: &InsightsStats, l: &HtmlLabels) -> String {
    let has_friction = !stats.friction.is_empty();
    let has_satisfaction = !stats.satisfaction.is_empty();

    if !has_friction && !has_satisfaction {
        return String::new();
    }

    let mut friction_items: Vec<(String, u32)> = stats.friction.iter().map(|(k, v)| (k.clone(), *v)).collect();
    friction_items.sort_by_key(|b| std::cmp::Reverse(b.1));
    friction_items.truncate(6);
    let friction_chart = render_bar_chart(l.chart_friction_types, &friction_items, "#dc2626", 6);

    let mut satisfaction_items: Vec<(String, u32)> = stats.satisfaction.iter().map(|(k, v)| (k.clone(), *v)).collect();
    satisfaction_items.sort_by_key(|b| std::cmp::Reverse(b.1));
    satisfaction_items.truncate(6);
    let satisfaction_chart = render_bar_chart(l.chart_satisfaction, &satisfaction_items, "#eab308", 6);

    wrap_charts_row(&friction_chart, &satisfaction_chart)
}

/// Wraps one or two chart cards into a layout row.
/// - Two non-empty cards → 2-column grid `.charts-row`.
/// - One non-empty card  → standalone full-width (no grid wrapper, just margin).
/// - Both empty           → empty string.
fn wrap_charts_row(card_a: &str, card_b: &str) -> String {
    match (card_a.is_empty(), card_b.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!(r#"<div class="charts-row charts-row-single">{}</div>"#, card_a),
        (true, false) => format!(r#"<div class="charts-row charts-row-single">{}</div>"#, card_b),
        (false, false) => format!(r#"<div class="charts-row">{}{}</div>"#, card_a, card_b),
    }
}

// ============ Chart helpers ============

fn render_response_time_chart(
    buckets: &std::collections::HashMap<String, u32>,
    stats: &InsightsStats,
    l: &HtmlLabels,
) -> String {
    let bucket_order = ["2-10s", "10-30s", "30s-1m", "1-2m", "2-5m", "5-15m", ">15m"];
    let ordered_items: Vec<(String, u32)> = bucket_order
        .iter()
        .filter_map(|&label| {
            buckets
                .get(label)
                .and_then(|&v| if v > 0 { Some((label.to_string(), v)) } else { None })
        })
        .collect();

    if ordered_items.is_empty() {
        return String::new();
    }

    let max_val = ordered_items.iter().map(|(_, v)| *v).max().unwrap_or(1) as f64;
    let bars: String = ordered_items.iter().map(|(label, value)| {
        let pct = (*value as f64 / max_val) * 100.0;
        format!(
            r#"<div class="bar-row"><div class="bar-label">{}</div><div class="bar-track"><div class="bar-fill" style="width:{:.1}%;background:#6366f1"></div></div><div class="bar-value">{}</div></div>"#,
            html_escape(label), pct, value,
        )
    }).collect();

    let footer = match (stats.median_response_time_secs, stats.avg_response_time_secs) {
        (Some(median), Some(avg)) => format!(
            r#"<div style="font-size:12px;color:#64748b;margin-top:8px">{}: {:.1}s &bull; {}: {:.1}s</div>"#,
            html_escape(l.median_label),
            median,
            html_escape(l.average_label),
            avg,
        ),
        _ => String::new(),
    };

    format!(
        r#"<div class="chart-card" style="margin:24px 0"><div class="chart-title">{}</div>{}{}</div>"#,
        html_escape(l.chart_response_time),
        bars,
        footer,
    )
}

fn render_time_of_day_chart(hour_counts: &std::collections::HashMap<u32, u32>, l: &HtmlLabels) -> String {
    if hour_counts.is_empty() {
        return format!(
            r#"<div class="chart-card"><div class="chart-title">{}</div><div class="empty">{}</div></div>"#,
            html_escape(l.chart_time_of_day),
            html_escape(l.no_data),
        );
    }

    let hour_json: Vec<String> = (0..24)
        .map(|h| format!("\"{}\":{}", h, hour_counts.get(&h).copied().unwrap_or(0)))
        .collect();

    format!(
        r#"<div class="chart-card" id="time-of-day-chart">
  <div class="chart-title" style="display:flex;justify-content:space-between;align-items:center">
    <span>{title}</span>
    <select id="tz-selector" class="tz-select" onchange="updateTimeChart()">
    </select>
  </div>
  <div id="time-bars"></div>
  <script>
    window.__hourCountsUTC = {{{hour_data}}};
    window.__timeLabels = {{morning:"{lm}",afternoon:"{la}",evening:"{le}",night:"{ln}"}};
  </script>
</div>"#,
        title = html_escape(l.chart_time_of_day),
        hour_data = hour_json.join(","),
        lm = l.time_morning,
        la = l.time_afternoon,
        le = l.time_evening,
        ln = l.time_night,
    )
}

fn render_bar_chart(title: &str, items: &[(String, u32)], color: &str, max_items: usize) -> String {
    let non_zero: Vec<&(String, u32)> = items.iter().filter(|(_, v)| *v > 0).collect();

    if non_zero.is_empty() {
        return String::new();
    }

    let max_val = non_zero.iter().map(|(_, v)| *v).max().unwrap_or(1) as f64;
    let bars: Vec<String> = non_zero
        .iter()
        .take(max_items)
        .map(|(label, value)| {
            let pct = (*value as f64 / max_val) * 100.0;
            let display_label = label
                .replace('_', " ")
                .split_whitespace()
                .map(|w| {
                    let mut c = w.chars();
                    match c.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().to_string() + c.as_str(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                r#"<div class="bar-row">
  <div class="bar-label">{}</div>
  <div class="bar-track"><div class="bar-fill" style="width:{:.1}%;background:{}"></div></div>
  <div class="bar-value">{}</div>
</div>"#,
                html_escape(&display_label),
                pct,
                color,
                value,
            )
        })
        .collect();

    format!(
        r#"<div class="chart-card"><div class="chart-title">{}</div>{}</div>"#,
        html_escape(title),
        bars.join("\n"),
    )
}
