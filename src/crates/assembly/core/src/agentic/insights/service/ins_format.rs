//! Report synthesis and assembly for InsightsService.

use crate::agentic::insights::prompt_context::aggregate_stats_json_for_prompt;
use crate::agentic::insights::types::*;
use crate::infrastructure::ai::AIClient;
use crate::util::errors::{NortHingError, NortHingResult};
use std::sync::Arc;
use tracing::{info, warn};

use super::ins_types::*;
use super::InsightsService;

impl InsightsService {
    // ============ Stage 4b: Synthesis ============

    pub(crate) async fn generate_synthesis(
        ai_client: &Arc<AIClient>,
        aggregate: &InsightsAggregate,
        suggestions: &InsightsSuggestions,
        areas: &[ProjectArea],
        wins_friction: &WinsFrictionResult,
        interaction: &InteractionStyleResult,
        lang_instruction: &str,
    ) -> AtAGlance {
        let aggregate_json = aggregate_stats_json_for_prompt(aggregate);

        let areas_text = areas
            .iter()
            .map(|a| format!("- {}: {}", a.name, a.description))
            .collect::<Vec<_>>()
            .join("\n");
        let suggestions_text = serde_json::to_string_pretty(suggestions).unwrap_or_else(|_| "{}".to_string());
        let wins_friction_text = serde_json::to_string_pretty(wins_friction).unwrap_or_else(|_| "{}".to_string());
        let interaction_text = serde_json::to_string_pretty(interaction).unwrap_or_else(|_| "{}".to_string());

        match Self::generate_at_a_glance(
            ai_client,
            &aggregate_json,
            &areas_text,
            &suggestions_text,
            &wins_friction_text,
            &interaction_text,
            lang_instruction,
        )
        .await
        {
            Ok(val) => val,
            Err(e) => {
                warn!("At a Glance generation failed: {}, using defaults", e);
                AtAGlance::default()
            }
        }
    }

    // ============ Stage 5: Assembly ============

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn assemble_report(
        _base_stats: BaseStats,
        aggregate: InsightsAggregate,
        suggestions: InsightsSuggestions,
        areas: Vec<ProjectArea>,
        wins_friction: WinsFrictionResult,
        interaction: InteractionStyleResult,
        at_a_glance: AtAGlance,
        horizon: HorizonResult,
        fun_ending: Option<FunEnding>,
    ) -> InsightsReport {
        let days_covered = if !aggregate.date_range.start.is_empty() && !aggregate.date_range.end.is_empty() {
            let parse = |s: &str| -> Option<chrono::DateTime<chrono::Utc>> {
                chrono::DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|d| d.with_timezone(&chrono::Utc))
            };
            match (parse(&aggregate.date_range.start), parse(&aggregate.date_range.end)) {
                (Some(start), Some(end)) => end.signed_duration_since(start).num_days().unsigned_abs() as u32,
                _ => 1,
            }
            .max(1)
        } else {
            1
        };

        InsightsReport {
            generated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            date_range: aggregate.date_range.clone(),
            total_sessions: aggregate.sessions,
            analyzed_sessions: aggregate.analyzed,
            total_messages: aggregate.messages,
            days_covered,
            stats: InsightsStats {
                total_hours: aggregate.hours,
                msgs_per_day: aggregate.msgs_per_day,
                top_tools: aggregate.top_tools.clone(),
                top_goals: aggregate.top_goals.clone(),
                outcomes: aggregate.outcomes.clone(),
                satisfaction: aggregate.satisfaction.clone(),
                session_types: aggregate.session_types.clone(),
                languages: aggregate.languages.clone(),
                hour_counts: aggregate.hour_counts.clone(),
                agent_types: aggregate.agent_types.clone(),
                response_time_buckets: aggregate.response_time_buckets.clone(),
                median_response_time_secs: aggregate.median_response_time_secs,
                avg_response_time_secs: aggregate.avg_response_time_secs,
                friction: aggregate.friction.clone(),
                success: aggregate.success.clone(),
                tool_errors: aggregate.tool_errors.clone(),
                total_lines_added: aggregate.total_lines_added,
                total_lines_removed: aggregate.total_lines_removed,
                total_files_modified: aggregate.total_files_modified,
            },
            at_a_glance,
            interaction_style: InteractionStyle {
                narrative: interaction.narrative,
                key_patterns: interaction.key_patterns,
            },
            project_areas: areas,
            wins_intro: wins_friction.wins_intro,
            big_wins: wins_friction.big_wins,
            friction_intro: wins_friction.friction_intro,
            friction_categories: wins_friction.friction_categories,
            suggestions,
            horizon_intro: horizon.intro,
            on_the_horizon: horizon.opportunities,
            fun_ending,
            html_report_path: None,
        }
    }
}
