use crate::agentic::insights::session_paths::collect_effective_session_storage_roots;
use crate::agentic::insights::types::*;
use crate::agentic::persistence::PersistenceManager;
use crate::infrastructure::path_manager_arc;
use crate::util::errors::NortHingResult;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::debug;

use super::coll_stats::{
    accumulate_code_stats_from_turns, accumulate_stats, bucket_response_times, compute_days_covered,
    compute_response_time_stats,
};
use super::coll_transcript::{build_transcript, load_session_messages_with_turns};

pub struct InsightsCollector;

impl InsightsCollector {
    /// Stage 1: Collect session data from PersistenceManager across all workspaces
    pub async fn collect(days: u32) -> NortHingResult<(BaseStats, Vec<SessionTranscript>)> {
        let path_manager = path_manager_arc();
        let pm = PersistenceManager::new(path_manager)?;
        let cutoff = SystemTime::now() - Duration::from_secs(days as u64 * 86400);

        let workspace_paths = collect_effective_session_storage_roots().await;

        let mut transcripts = Vec::new();
        let mut base_stats = BaseStats::default();
        let mut seen_session_ids = HashSet::new();

        for ws_path in &workspace_paths {
            let sessions = match pm.list_sessions(ws_path).await {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Skipping workspace {}: {}", ws_path.display(), e);
                    continue;
                }
            };

            for summary in &sessions {
                if summary.last_activity_at < cutoff {
                    continue;
                }

                if !seen_session_ids.insert(summary.session_id.clone()) {
                    continue;
                }

                let session = match pm.load_session(ws_path, &summary.session_id).await {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("Skipping session {}: load failed: {}", summary.session_id, e);
                        continue;
                    }
                };

                let turns = pm
                    .load_session_turns(ws_path, &summary.session_id)
                    .await
                    .unwrap_or_default();

                let messages = match load_session_messages_with_turns(&pm, ws_path, &summary.session_id, &turns).await {
                    Ok(m) if !m.is_empty() => m,
                    Ok(_) => {
                        debug!("Skipping session {}: no messages found", summary.session_id);
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!("Skipping session {}: load messages failed: {}", summary.session_id, e);
                        continue;
                    }
                };

                let mut transcript = build_transcript(&summary.session_id, &session, &messages);
                transcript.workspace_path = Some(ws_path.to_string_lossy().to_string());
                transcript.last_activity_unix_secs = summary
                    .last_activity_at
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                accumulate_stats(&mut base_stats, &session, &messages);
                accumulate_code_stats_from_turns(&mut base_stats, &turns);
                transcripts.push(transcript);
            }
        }

        base_stats.total_sessions = transcripts.len() as u32;

        if let Some(earliest) = transcripts.iter().min_by_key(|t| &t.created_at) {
            base_stats.first_session_at = Some(earliest.created_at.clone());
        }
        if let Some(latest) = transcripts.iter().max_by_key(|t| &t.created_at) {
            base_stats.last_session_at = Some(latest.created_at.clone());
        }

        // Compute response time buckets from raw intervals
        if !base_stats.response_times_raw.is_empty() {
            base_stats.response_time_buckets = bucket_response_times(&base_stats.response_times_raw);
            let (median, avg) = compute_response_time_stats(&base_stats.response_times_raw);
            base_stats.median_response_time_secs = Some(median);
            base_stats.avg_response_time_secs = Some(avg);
        }

        debug!(
            "Collected {} sessions with {} total messages",
            transcripts.len(),
            base_stats.total_messages
        );

        Ok((base_stats, transcripts))
    }

    /// Stage 3: Aggregate facets into InsightsAggregate
    pub fn aggregate(base_stats: &BaseStats, facets: &[SessionFacet]) -> InsightsAggregate {
        let mut goals: HashMap<String, u32> = HashMap::new();
        let mut outcomes: HashMap<String, u32> = HashMap::new();
        let mut satisfaction: HashMap<String, u32> = HashMap::new();
        let mut friction: HashMap<String, u32> = HashMap::new();
        let mut success: HashMap<String, u32> = HashMap::new();
        let mut session_types: HashMap<String, u32> = HashMap::new();
        let mut session_summaries = Vec::new();
        let mut friction_details = Vec::new();
        let mut user_instructions = Vec::new();

        for facet in facets {
            for (k, v) in &facet.goal_categories {
                *goals.entry(k.clone()).or_insert(0) += v;
            }
            *outcomes.entry(facet.outcome.clone()).or_insert(0) += 1;
            for (k, v) in &facet.user_satisfaction_counts {
                *satisfaction.entry(k.clone()).or_insert(0) += v;
            }
            for (k, v) in &facet.friction_counts {
                *friction.entry(k.clone()).or_insert(0) += v;
            }
            if !facet.primary_success.is_empty() && facet.primary_success != "none" {
                *success.entry(facet.primary_success.clone()).or_insert(0) += 1;
            }
            *session_types.entry(facet.session_type.clone()).or_insert(0) += 1;

            if !facet.brief_summary.is_empty() {
                session_summaries.push(facet.brief_summary.clone());
            }
            if !facet.friction_detail.is_empty() {
                friction_details.push(facet.friction_detail.clone());
            }
            for instr in &facet.user_instructions {
                if !user_instructions.contains(instr) {
                    user_instructions.push(instr.clone());
                }
            }
        }

        let mut top_tools: Vec<(String, u32)> = base_stats.tool_usage.iter().map(|(k, v)| (k.clone(), *v)).collect();
        top_tools.sort_by_key(|b| std::cmp::Reverse(b.1));
        top_tools.truncate(15);

        let mut top_goals: Vec<(String, u32)> = goals.iter().map(|(k, v)| (k.clone(), *v)).collect();
        top_goals.sort_by_key(|b| std::cmp::Reverse(b.1));
        top_goals.truncate(10);

        let hours = base_stats.total_duration_minutes as f32 / 60.0;
        let date_range = DateRange {
            start: base_stats.first_session_at.clone().unwrap_or_default(),
            end: base_stats.last_session_at.clone().unwrap_or_default(),
        };

        let days_covered = compute_days_covered(&date_range);
        let msgs_per_day = if days_covered > 0 {
            base_stats.total_messages as f32 / days_covered as f32
        } else {
            base_stats.total_messages as f32
        };

        let languages = base_stats.languages_by_files.clone();

        InsightsAggregate {
            sessions: base_stats.total_sessions,
            analyzed: facets.len() as u32,
            date_range,
            messages: base_stats.total_messages,
            hours,
            top_tools,
            top_goals,
            outcomes,
            satisfaction,
            friction,
            success,
            languages,
            session_summaries,
            friction_details,
            user_instructions,
            session_types,
            tool_errors: base_stats.tool_errors.clone(),
            hour_counts: base_stats.hour_counts.clone(),
            agent_types: base_stats.agent_types.clone(),
            msgs_per_day,
            response_time_buckets: base_stats.response_time_buckets.clone(),
            median_response_time_secs: base_stats.median_response_time_secs,
            avg_response_time_secs: base_stats.avg_response_time_secs,
            total_lines_added: base_stats.total_lines_added,
            total_lines_removed: base_stats.total_lines_removed,
            total_files_modified: base_stats.total_files_modified,
        }
    }
}
