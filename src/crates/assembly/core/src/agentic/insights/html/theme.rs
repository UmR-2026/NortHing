pub struct HtmlLabels {
    pub title: &'static str,
    pub subtitle_template: &'static str, // "{msgs} messages across {sessions} sessions ({analyzed} analyzed) | {start} to {end}"
    pub at_a_glance: &'static str,
    pub whats_working: &'static str,
    pub whats_hindering: &'static str,
    pub quick_wins: &'static str,
    pub looking_ahead: &'static str,
    pub nav_work: &'static str,
    pub nav_usage: &'static str,
    pub nav_wins: &'static str,
    pub nav_friction: &'static str,
    pub nav_suggestions: &'static str,
    pub nav_horizon: &'static str,
    pub stat_sessions: &'static str,
    pub stat_messages: &'static str,
    pub stat_hours: &'static str,
    pub stat_days: &'static str,
    pub stat_msgs_per_day: &'static str,
    pub stat_median_response: &'static str,
    pub stat_avg_response: &'static str,
    pub section_work: &'static str,
    pub section_usage: &'static str,
    pub section_wins: &'static str,
    pub section_friction: &'static str,
    pub section_suggestions: &'static str,
    pub section_horizon: &'static str,
    pub chart_goals: &'static str,
    pub chart_tools: &'static str,
    pub chart_languages: &'static str,
    pub chart_session_types: &'static str,
    pub chart_tool_errors: &'static str,
    pub chart_agent_types: &'static str,
    pub chart_response_time: &'static str,
    pub chart_time_of_day: &'static str,
    pub chart_what_helped: &'static str,
    pub chart_outcomes: &'static str,
    pub chart_friction_types: &'static str,
    pub chart_satisfaction: &'static str,
    pub time_morning: &'static str,
    pub time_afternoon: &'static str,
    pub time_evening: &'static str,
    pub time_night: &'static str,
    pub sessions_suffix: &'static str,
    pub no_data: &'static str,
    pub no_project_areas: &'static str,
    pub no_interaction_style: &'static str,
    pub no_big_wins: &'static str,
    pub no_friction: &'static str,
    pub no_horizon: &'static str,
    pub md_additions: &'static str,
    pub copy_all_checked: &'static str,
    pub features_to_try: &'static str,
    pub usage_patterns: &'static str,
    pub try_this_prompt: &'static str,
    pub copied: &'static str,
    pub median_label: &'static str,
    pub average_label: &'static str,
    pub stat_lines: &'static str,
    pub stat_files: &'static str,
}

impl HtmlLabels {
    pub fn for_locale(locale: &str) -> Self {
        if locale.starts_with("zh") {
            Self::zh()
        } else {
            Self::en()
        }
    }

    pub fn en() -> Self {
        HtmlLabels {
            title: "northhing Insights",
            subtitle_template: "{msgs} messages across {sessions} sessions ({analyzed} analyzed) | {start} to {end}",
            at_a_glance: "At a Glance",
            whats_working: "What's working:",
            whats_hindering: "What's hindering you:",
            quick_wins: "Quick wins to try:",
            looking_ahead: "Looking ahead:",
            nav_work: "What You Work On",
            nav_usage: "How You Use northhing",
            nav_wins: "Impressive Things",
            nav_friction: "Where Things Go Wrong",
            nav_suggestions: "Suggestions",
            nav_horizon: "On the Horizon",
            stat_sessions: "Sessions",
            stat_messages: "Messages",
            stat_hours: "Hours",
            stat_days: "Days",
            stat_msgs_per_day: "Msgs/Day",
            stat_median_response: "Median Response",
            stat_avg_response: "Avg Response",
            section_work: "What You Work On",
            section_usage: "How You Use northhing",
            section_wins: "Impressive Things You Did",
            section_friction: "Where Things Go Wrong",
            section_suggestions: "Suggestions",
            section_horizon: "On the Horizon",
            chart_goals: "What You Wanted",
            chart_tools: "Top Tools Used",
            chart_languages: "Languages",
            chart_session_types: "Session Types",
            chart_tool_errors: "Tool Errors Encountered",
            chart_agent_types: "Agent Types",
            chart_response_time: "User Response Time Distribution",
            chart_time_of_day: "Messages by Time of Day",
            chart_what_helped: "What Helped Most",
            chart_outcomes: "Outcomes",
            chart_friction_types: "Primary Friction Types",
            chart_satisfaction: "Satisfaction (Inferred)",
            time_morning: "Morning (6-12)",
            time_afternoon: "Afternoon (12-18)",
            time_evening: "Evening (18-24)",
            time_night: "Night (0-6)",
            sessions_suffix: "sessions",
            no_data: "No data",
            no_project_areas: "No project areas identified.",
            no_interaction_style: "No interaction style data available.",
            no_big_wins: "No big wins identified yet.",
            no_friction: "No significant friction points found.",
            no_horizon: "No horizon workflows identified.",
            md_additions: "northhing.md Additions",
            copy_all_checked: "Copy All Checked",
            features_to_try: "Features to Try",
            usage_patterns: "Usage Patterns",
            try_this_prompt: "Try this prompt:",
            copied: "Copied!",
            median_label: "Median",
            average_label: "Average",
            stat_lines: "Lines",
            stat_files: "Files",
        }
    }

    pub fn zh() -> Self {
        HtmlLabels {
            title: "northhing 洞察",
            subtitle_template: "{msgs} 条消息，{sessions} 个会话（{analyzed} 个已分析）| {start} 至 {end}",
            at_a_glance: "概览",
            whats_working: "做得好的：",
            whats_hindering: "遇到的阻碍：",
            quick_wins: "快速提升：",
            looking_ahead: "展望未来：",
            nav_work: "工作领域",
            nav_usage: "使用方式",
            nav_wins: "亮眼成果",
            nav_friction: "问题所在",
            nav_suggestions: "建议",
            nav_horizon: "未来展望",
            stat_sessions: "会话",
            stat_messages: "消息",
            stat_hours: "小时",
            stat_days: "天",
            stat_msgs_per_day: "消息/天",
            stat_median_response: "中位响应",
            stat_avg_response: "平均响应",
            section_work: "工作领域",
            section_usage: "你如何使用 northhing",
            section_wins: "亮眼成果",
            section_friction: "问题所在",
            section_suggestions: "建议",
            section_horizon: "未来展望",
            chart_goals: "你的需求",
            chart_tools: "常用工具",
            chart_languages: "编程语言",
            chart_session_types: "会话类型",
            chart_tool_errors: "工具错误统计",
            chart_agent_types: "智能体类型",
            chart_response_time: "用户响应时间分布",
            chart_time_of_day: "按时段分布",
            chart_what_helped: "最有帮助的方面",
            chart_outcomes: "结果分布",
            chart_friction_types: "主要摩擦类型",
            chart_satisfaction: "满意度（推断）",
            time_morning: "上午 (6-12)",
            time_afternoon: "下午 (12-18)",
            time_evening: "晚上 (18-24)",
            time_night: "凌晨 (0-6)",
            sessions_suffix: "个会话",
            no_data: "暂无数据",
            no_project_areas: "未识别到项目领域。",
            no_interaction_style: "暂无交互风格数据。",
            no_big_wins: "暂未识别到亮眼成果。",
            no_friction: "未发现明显摩擦点。",
            no_horizon: "暂未识别到未来工作流。",
            md_additions: "northhing.md 补充",
            copy_all_checked: "复制选中项",
            features_to_try: "推荐功能",
            usage_patterns: "使用模式",
            try_this_prompt: "试试这个提示：",
            copied: "已复制！",
            median_label: "中位数",
            average_label: "平均值",
            stat_lines: "行",
            stat_files: "文件",
        }
    }
}

pub const CSS_STYLES: &str = r#"
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif; background: #f8fafc; color: #334155; line-height: 1.65; padding: 48px 24px; }
    .container { max-width: 800px; margin: 0 auto; }
    h1 { font-size: 32px; font-weight: 700; color: #0f172a; margin-bottom: 8px; }
    h2 { font-size: 20px; font-weight: 600; color: #0f172a; margin-top: 48px; margin-bottom: 16px; }
    h3 { font-size: 16px; font-weight: 600; color: #0f172a; margin-top: 24px; margin-bottom: 12px; }
    .subtitle { color: #64748b; font-size: 15px; margin-bottom: 32px; }
    .nav-toc { display: flex; flex-wrap: wrap; gap: 8px; margin: 24px 0 32px 0; padding: 16px; background: white; border-radius: 8px; border: 1px solid #e2e8f0; }
    .nav-toc a { font-size: 12px; color: #64748b; text-decoration: none; padding: 6px 12px; border-radius: 6px; background: #f1f5f9; transition: all 0.15s; }
    .nav-toc a:hover { background: #e2e8f0; color: #334155; }
    .stats-row { display: flex; gap: 24px; margin-bottom: 40px; padding: 20px 0; border-top: 1px solid #e2e8f0; border-bottom: 1px solid #e2e8f0; flex-wrap: wrap; }
    .stat { text-align: center; }
    .stat-value { font-size: 24px; font-weight: 700; color: #0f172a; }
    .stat-label { font-size: 11px; color: #64748b; text-transform: uppercase; }
    .at-a-glance { background: linear-gradient(135deg, #fef3c7 0%, #fde68a 100%); border: 1px solid #f59e0b; border-radius: 12px; padding: 20px 24px; margin-bottom: 32px; }
    .glance-title { font-size: 16px; font-weight: 700; color: #92400e; margin-bottom: 16px; }
    .glance-sections { display: flex; flex-direction: column; gap: 12px; }
    .glance-section { font-size: 14px; color: #78350f; line-height: 1.6; }
    .glance-section strong { color: #92400e; }
    .see-more { color: #b45309; text-decoration: none; font-size: 13px; white-space: nowrap; }
    .see-more:hover { text-decoration: underline; }
    .project-areas { display: flex; flex-direction: column; gap: 12px; margin-bottom: 32px; }
    .project-area { background: white; border: 1px solid #e2e8f0; border-radius: 8px; padding: 16px; }
    .area-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
    .area-name { font-weight: 600; font-size: 15px; color: #0f172a; }
    .area-count { font-size: 12px; color: #64748b; background: #f1f5f9; padding: 2px 8px; border-radius: 4px; }
    .area-desc { font-size: 14px; color: #475569; line-height: 1.5; }
    .narrative { background: white; border: 1px solid #e2e8f0; border-radius: 8px; padding: 20px; margin-bottom: 24px; }
    .narrative p { margin-bottom: 12px; font-size: 14px; color: #475569; line-height: 1.7; }
    .key-insight { background: #f0fdf4; border: 1px solid #bbf7d0; border-radius: 8px; padding: 12px 16px; margin-top: 12px; font-size: 14px; color: #166534; }
    .section-intro { font-size: 14px; color: #475569; line-height: 1.6; margin-bottom: 16px; }
    .big-wins { display: flex; flex-direction: column; gap: 12px; margin-bottom: 24px; }
    .big-win { background: #f0fdf4; border: 1px solid #bbf7d0; border-radius: 8px; padding: 16px; }
    .big-win-title { font-weight: 600; font-size: 15px; color: #166534; margin-bottom: 8px; }
    .big-win-desc { font-size: 14px; color: #15803d; line-height: 1.5; }
    .big-win-impact { font-size: 12px; color: #166534; opacity: 0.8; font-style: italic; margin-top: 6px; }
    .friction-categories { display: flex; flex-direction: column; gap: 16px; margin-bottom: 24px; }
    .friction-category { background: #fef2f2; border: 1px solid #fca5a5; border-radius: 8px; padding: 16px; }
    .friction-title { font-weight: 600; font-size: 15px; color: #991b1b; margin-bottom: 6px; }
    .friction-desc { font-size: 13px; color: #7f1d1d; margin-bottom: 10px; }
    .friction-examples { margin: 0 0 0 20px; font-size: 13px; color: #334155; }
    .friction-examples li { margin-bottom: 4px; }
    .claude-md-section { background: #eff6ff; border: 1px solid #bfdbfe; border-radius: 8px; padding: 16px; margin-bottom: 20px; }
    .claude-md-section h3 { font-size: 14px; font-weight: 600; color: #1e40af; margin: 0 0 12px 0; }
    .claude-md-actions { margin-bottom: 12px; padding-bottom: 12px; border-bottom: 1px solid #dbeafe; }
    .copy-all-btn { background: #2563eb; color: white; border: none; border-radius: 4px; padding: 6px 12px; font-size: 12px; cursor: pointer; font-weight: 500; transition: all 0.2s; }
    .copy-all-btn:hover { background: #1d4ed8; }
    .copy-all-btn.copied { background: #16a34a; }
    .claude-md-item { display: flex; flex-wrap: wrap; align-items: flex-start; gap: 8px; padding: 10px 0; border-bottom: 1px solid #dbeafe; }
    .claude-md-item:last-child { border-bottom: none; }
    .cmd-checkbox { margin-top: 2px; }
    .cmd-code { background: white; padding: 8px 12px; border-radius: 4px; font-size: 12px; color: #1e40af; border: 1px solid #bfdbfe; font-family: monospace; display: block; white-space: pre-wrap; word-break: break-word; flex: 1; }
    .cmd-why { font-size: 12px; color: #64748b; width: 100%; padding-left: 24px; margin-top: 4px; }
    .features-section, .patterns-section { display: flex; flex-direction: column; gap: 12px; margin: 16px 0; }
    .feature-card { background: #f0fdf4; border: 1px solid #86efac; border-radius: 8px; padding: 16px; }
    .pattern-card { background: #f0f9ff; border: 1px solid #7dd3fc; border-radius: 8px; padding: 16px; }
    .feature-title, .pattern-title { font-weight: 600; font-size: 15px; color: #0f172a; margin-bottom: 6px; }
    .feature-oneliner, .pattern-summary { font-size: 14px; color: #475569; margin-bottom: 8px; }
    .feature-why { font-size: 13px; color: #334155; line-height: 1.5; }
    .feature-code { background: #f8fafc; padding: 12px; border-radius: 6px; margin-top: 12px; border: 1px solid #e2e8f0; display: flex; align-items: flex-start; gap: 8px; }
    .feature-code code { flex: 1; font-family: monospace; font-size: 12px; color: #334155; white-space: pre-wrap; }
    .pattern-prompt { background: #f8fafc; padding: 12px; border-radius: 6px; margin-top: 12px; border: 1px solid #e2e8f0; }
    .pattern-prompt code { font-family: monospace; font-size: 12px; color: #334155; display: block; white-space: pre-wrap; margin-bottom: 8px; }
    .prompt-label { font-size: 11px; font-weight: 600; text-transform: uppercase; color: #64748b; margin-bottom: 6px; }
    .copy-btn { background: #e2e8f0; border: none; border-radius: 4px; padding: 4px 8px; font-size: 11px; cursor: pointer; color: #475569; flex-shrink: 0; }
    .copy-btn:hover { background: #cbd5e1; }
    .charts-row { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; margin: 24px 0; }
    .charts-row-single { grid-template-columns: 1fr; }
    .chart-card { background: white; border: 1px solid #e2e8f0; border-radius: 8px; padding: 16px; }
    .chart-title { font-size: 12px; font-weight: 600; color: #64748b; text-transform: uppercase; margin-bottom: 12px; }
    .bar-row { display: flex; align-items: center; margin-bottom: 6px; }
    .bar-label { width: 100px; font-size: 11px; color: #475569; flex-shrink: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .bar-track { flex: 1; height: 6px; background: #f1f5f9; border-radius: 3px; margin: 0 8px; }
    .bar-fill { height: 100%; border-radius: 3px; }
    .bar-value { width: 28px; font-size: 11px; font-weight: 500; color: #64748b; text-align: right; }
    .empty { color: #94a3b8; font-size: 13px; padding: 12px 0; }
    .tz-select { font-size: 11px; padding: 2px 6px; border: 1px solid #e2e8f0; border-radius: 4px; background: #f8fafc; color: #475569; cursor: pointer; }
    .horizon-section { display: flex; flex-direction: column; gap: 16px; }
    .horizon-card { background: linear-gradient(135deg, #faf5ff 0%, #f5f3ff 100%); border: 1px solid #c4b5fd; border-radius: 8px; padding: 16px; }
    .horizon-title { font-weight: 600; font-size: 15px; color: #5b21b6; margin-bottom: 8px; }
    .horizon-possible { font-size: 14px; color: #334155; margin-bottom: 10px; line-height: 1.5; }
    .horizon-steps { margin: 0 0 0 20px; font-size: 13px; color: #6b21a8; }
    .horizon-steps li { margin-bottom: 4px; }
    .horizon-tip { font-size: 13px; color: #5b21b6; background: #ede9fe; border-radius: 6px; padding: 8px 12px; margin-top: 10px; line-height: 1.5; }
    .horizon-prompt { margin-top: 10px; }
    .fun-ending { background: linear-gradient(135deg, #fef3c7 0%, #fde68a 100%); border: 1px solid #fbbf24; border-radius: 12px; padding: 24px; margin-top: 40px; text-align: center; }
    .fun-headline { font-size: 18px; font-weight: 600; color: #78350f; margin-bottom: 8px; }
    .fun-detail { font-size: 14px; color: #92400e; }
    @media (max-width: 640px) { .charts-row { grid-template-columns: 1fr; } .stats-row { justify-content: center; } }
"#;

pub const JS_SCRIPT: &str = r#"
    function copyText(btn, text) {
      navigator.clipboard.writeText(text).then(function() {
        var orig = btn.textContent;
        btn.textContent = ' __COPIED__ ';
        btn.style.background = '#16a34a';
        btn.style.color = 'white';
        setTimeout(function() {
          btn.textContent = orig;
          btn.style.background = '';
          btn.style.color = '';
        }, 2000);
      });
    }

    function copyAllChecked(btn) {
      var section = btn.closest('.claude-md-section');
      var items = section.querySelectorAll('.claude-md-item');
      var texts = [];
      items.forEach(function(item) {
        var cb = item.querySelector('.cmd-checkbox');
        if (cb && cb.checked) {
          var code = item.querySelector('.cmd-code');
          if (code) texts.push(code.textContent.trim());
        }
      });
      if (texts.length === 0) return;
      navigator.clipboard.writeText(texts.join('\n\n')).then(function() {
        btn.textContent = '__COPIED__';
        btn.classList.add('copied');
        setTimeout(function() {
          btn.textContent = '__COPY_ALL_CHECKED__';
          btn.classList.remove('copied');
        }, 2000);
      });
    }

    (function initTimezoneSelector() {
      var sel = document.getElementById('tz-selector');
      if (!sel || !window.__hourCountsUTC) return;
      var common = [
        'UTC',
        'America/New_York','America/Chicago','America/Denver','America/Los_Angeles',
        'Europe/London','Europe/Paris','Europe/Berlin',
        'Asia/Tokyo','Asia/Shanghai','Asia/Kolkata','Asia/Singapore',
        'Australia/Sydney','Pacific/Auckland'
      ];
      var localTz = Intl.DateTimeFormat().resolvedOptions().timeZone;
      if (common.indexOf(localTz) === -1) common.unshift(localTz);
      common.forEach(function(tz) {
        var opt = document.createElement('option');
        opt.value = tz;
        opt.textContent = tz.replace(/_/g,' ');
        if (tz === localTz) opt.selected = true;
        sel.appendChild(opt);
      });
      updateTimeChart();
    })();

    function updateTimeChart() {
      var sel = document.getElementById('tz-selector');
      var container = document.getElementById('time-bars');
      if (!sel || !container || !window.__hourCountsUTC) return;
      var tz = sel.value;
      var shifted = {};
      for (var h = 0; h < 24; h++) {
        var utcCount = window.__hourCountsUTC[h] || 0;
        if (utcCount === 0) continue;
        var d = new Date(Date.UTC(2024,0,1,h,0,0));
        var localH = parseInt(d.toLocaleString('en-US',{hour:'numeric',hour12:false,timeZone:tz}));
        shifted[localH] = (shifted[localH]||0) + utcCount;
      }
      var labels = window.__timeLabels;
      var periods = [
        {label:labels.morning, hours:[6,7,8,9,10,11]},
        {label:labels.afternoon, hours:[12,13,14,15,16,17]},
        {label:labels.evening, hours:[18,19,20,21,22,23]},
        {label:labels.night, hours:[0,1,2,3,4,5]}
      ];
      var maxVal = 0;
      var data = periods.map(function(p) {
        var count = 0;
        p.hours.forEach(function(h){count += shifted[h]||0;});
        if (count > maxVal) maxVal = count;
        return {label:p.label, count:count};
      });
      var html = '';
      data.forEach(function(d) {
        var pct = maxVal > 0 ? (d.count/maxVal*100) : 0;
        html += '<div class="bar-row"><span class="bar-label">'+d.label+'</span>'
          +'<div class="bar-track"><div class="bar-fill" style="width:'+pct+'%;background:#8b5cf6"></div></div>'
          +'<span class="bar-value">'+d.count+'</span></div>';
      });
      container.innerHTML = html;
    }
"#;
