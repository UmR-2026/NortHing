//! Data collection and pipeline orchestration for InsightsService.

use crate::agentic::insights::cancellation;
use crate::agentic::insights::collector::InsightsCollector;
use crate::agentic::insights::facet_cache;
use crate::agentic::insights::prompt_context::{
    aggregate_stats_json_for_prompt, friction_block, summaries_block, user_instructions_block,
};
use crate::agentic::insights::session_paths::collect_effective_session_storage_roots;
use crate::agentic::insights::types::*;
use crate::infrastructure::ai::get_global_ai_client_factory;
use crate::infrastructure::ai::AIClient;
use crate::service::config::get_global_config_service;
use crate::service::config::AppConfig;
use crate::service::i18n::LocaleId;
use crate::util::errors::{NortHingError, NortHingResult};
use crate::util::types::Message;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

use super::ins_analyze::*;
use super::ins_format::*;
use super::ins_query::*;
use super::ins_types::*;
use super::InsightsService;

impl InsightsService {
    /// Main entry: run the full insights pipeline
    pub async fn generate(days: u32) -> NortHingResult<InsightsReport> {
        let token = cancellation::register().await;
        let result = Self::generate_inner(days, &token).await;
        cancellation::unregister().await;
        result
    }

    /// Cancel the current insights generation.
    pub async fn cancel() -> Result<(), String> {
        cancellation::cancel().await
    }

    async fn generate_inner(days: u32, token: &CancellationToken) -> NortHingResult<InsightsReport> {
        let user_lang = Self::get_user_language().await;
        let lang_instruction = Self::build_language_instruction(&user_lang);
        debug!("Insights generation using language: {}", user_lang);

        // Stage 1: Data Collection
        Self::emit_progress("Collecting session data...", "data_collection", 0, 0).await;
        let (base_stats, transcripts) = InsightsCollector::collect(days).await?;

        if transcripts.is_empty() {
            return Err(NortHingError::service("No sessions found in the specified time range"));
        }

        info!(
            "Collected {} sessions, {} messages",
            transcripts.len(),
            base_stats.total_messages
        );

        Self::check_cancelled(token)?;

        // Stage 2: Parallel Facet Extraction (fast model)
        let ai_factory = get_global_ai_client_factory()
            .await
            .map_err(|e| NortHingError::service(format!("Failed to get AI client factory: {}", e)))?;
        let ai_client_fast = ai_factory
            .get_client_resolved("fast")
            .await
            .map_err(|e| NortHingError::service(format!("Failed to resolve fast model: {}", e)))?;

        // Primary model for analysis stages — falls back to fast if not configured
        let ai_client_primary = match ai_factory.get_client_resolved("primary").await {
            Ok(client) => client,
            Err(_) => {
                warn!("Primary model not configured, falling back to fast model for analysis");
                ai_client_fast.clone()
            }
        };

        let facets = Self::extract_facets_adaptive(&ai_client_fast, &transcripts, &lang_instruction, token).await?;

        info!("Extracted facets for {} sessions", facets.len());

        Self::check_cancelled(token)?;

        // Stage 3: Aggregation (Rust-side, no AI)
        Self::emit_progress("Aggregating analysis...", "aggregation", 0, 0).await;
        let aggregate = InsightsCollector::aggregate(&base_stats, &facets);

        Self::check_cancelled(token)?;

        // Stage 4a: Parallel analysis (primary model) — 7 independent tasks
        Self::emit_progress("Analyzing patterns...", "analysis", 0, 0).await;

        let (suggestions, areas, wins_friction, interaction, horizon, fun_ending) =
            Self::generate_analysis_parallel(&ai_client_primary, &aggregate, &lang_instruction).await;

        Self::check_cancelled(token)?;

        // Stage 4b: Synthesis (primary model) — at_a_glance depends on 4a results
        Self::emit_progress("Writing summary...", "synthesis", 0, 0).await;

        let at_a_glance = Self::generate_synthesis(
            &ai_client_primary,
            &aggregate,
            &suggestions,
            &areas,
            &wins_friction,
            &interaction,
            &lang_instruction,
        )
        .await;

        Self::check_cancelled(token)?;

        // Stage 5: Assembly
        Self::emit_progress("Assembling report...", "assembly", 0, 0).await;
        let report = Self::assemble_report(
            base_stats,
            aggregate,
            suggestions,
            areas,
            wins_friction,
            interaction,
            at_a_glance,
            horizon,
            fun_ending,
        );

        let report = Self::save_report(report, &user_lang).await?;

        Self::emit_progress("Complete!", "complete", 0, 0).await;
        info!("Insights report generated successfully");

        Ok(report)
    }

    fn check_cancelled(token: &CancellationToken) -> NortHingResult<()> {
        if token.is_cancelled() {
            Err(NortHingError::service("Insights generation cancelled"))
        } else {
            Ok(())
        }
    }

    // ============ Stage 2: Facet Extraction ============

    async fn extract_facets_adaptive(
        ai_client: &Arc<AIClient>,
        transcripts: &[SessionTranscript],
        lang_instruction: &str,
        token: &CancellationToken,
    ) -> NortHingResult<Vec<SessionFacet>> {
        let total = transcripts.len();
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_FACET_EXTRACTIONS));
        let counter = Arc::new(AtomicUsize::new(0));
        let rate_limited = Arc::new(AtomicBool::new(false));
        let cancelled = Arc::new(AtomicBool::new(false));

        let handles: Vec<_> = transcripts
            .iter()
            .enumerate()
            .map(|(idx, t)| {
                let client = ai_client.clone();
                let sem = semaphore.clone();
                let transcript = t.clone();
                let cnt = counter.clone();
                let rl = rate_limited.clone();
                let cl = cancelled.clone();
                let lang = lang_instruction.to_string();
                let child_token = token.clone();

                tokio::spawn(async move {
                    let _permit = sem
                        .acquire()
                        .await
                        .map_err(|e| NortHingError::service(format!("Semaphore error: {}", e)))?;

                    if cl.load(Ordering::Relaxed) || child_token.is_cancelled() {
                        return Err(NortHingError::service("Insights generation cancelled"));
                    }

                    if rl.load(Ordering::Relaxed) {
                        return Err(NortHingError::service("skipped_rate_limited"));
                    }

                    let n = cnt.fetch_add(1, Ordering::Relaxed) + 1;
                    Self::emit_progress(
                        &format!("Analyzing session {}/{}...", n, total),
                        "facet_extraction",
                        n,
                        total,
                    )
                    .await;

                    let result = Self::extract_single_facet(&client, &transcript, &lang).await;

                    if let Err(ref e) = result {
                        if is_rate_limit_error(e) {
                            rl.store(true, Ordering::Relaxed);
                        }
                    }

                    result.map(|facet| (idx, facet))
                })
            })
            .collect();

        let mut facets = Vec::new();
        let mut failed_indices: Vec<usize> = Vec::new();
        let mut hit_rate_limit = false;

        for (idx, handle) in handles.into_iter().enumerate() {
            if token.is_cancelled() {
                return Err(NortHingError::service("Insights generation cancelled"));
            }
            match handle.await {
                Ok(Ok((_orig_idx, facet))) => facets.push(facet),
                Ok(Err(e)) => {
                    let err_str = e.to_string();
                    if err_str.contains("cancelled") {
                        return Err(e);
                    }
                    if err_str.contains("skipped_rate_limited") || is_rate_limit_error(&e) {
                        hit_rate_limit = true;
                        failed_indices.push(idx);
                    } else {
                        warn!("Facet extraction failed for session {}: {}", idx, e);
                    }
                }
                Err(e) => warn!("Facet task panicked: {}", e),
            }
        }

        if hit_rate_limit && !failed_indices.is_empty() {
            let retry_count = failed_indices.len();
            warn!("Rate limit detected, retrying {} sessions sequentially", retry_count);
            Self::emit_progress(
                &format!("Rate limited. Retrying {} sessions sequentially...", retry_count),
                "facet_retry",
                0,
                retry_count,
            )
            .await;

            tokio::time::sleep(Duration::from_secs(3)).await;

            for (i, idx) in failed_indices.iter().enumerate() {
                Self::check_cancelled(token)?;

                Self::emit_progress(
                    &format!("Retrying session {}/{}...", i + 1, retry_count),
                    "facet_retry",
                    i + 1,
                    retry_count,
                )
                .await;

                match Self::extract_single_facet(ai_client, &transcripts[*idx], lang_instruction).await {
                    Ok(facet) => facets.push(facet),
                    Err(e) => warn!("Sequential retry also failed for session {}: {}", idx, e),
                }

                if i + 1 < retry_count {
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }

        Ok(facets)
    }

    async fn extract_single_facet(
        ai_client: &Arc<AIClient>,
        transcript: &SessionTranscript,
        lang_instruction: &str,
    ) -> NortHingResult<SessionFacet> {
        if let Ok(Some(cached)) = facet_cache::try_load_cached_facet(transcript).await {
            return Ok(cached);
        }

        let session_info = format!(
            "Session: {}\nAgent: {}\nName: {}\nDate: {}\nDuration: {} min\n\n{}",
            transcript.session_id,
            transcript.agent_type,
            transcript.session_name,
            transcript.created_at,
            transcript.duration_minutes,
            transcript.transcript
        );

        let prompt = format!(
            "{}{}",
            FACET_PROMPT_TEMPLATE.replace("{session_transcript}", &session_info),
            lang_instruction
        );
        let messages = vec![Message::user(prompt)];

        let response = ai_client
            .send_message(messages, None)
            .await
            .map_err(|e| NortHingError::service(format!("AI call failed: {}", e)))?;

        let json_str = extract_json_from_response(&response.text)?;
        let value: Value = serde_json::from_str(&json_str)
            .map_err(|e| NortHingError::Deserialization(format!("Failed to parse facet JSON: {}", e)))?;

        let facet = SessionFacet {
            session_id: transcript.session_id.clone(),
            underlying_goal: value["underlying_goal"].as_str().unwrap_or("").to_string(),
            goal_categories: parse_string_u32_map(&value["goal_categories"]),
            outcome: value["outcome"]
                .as_str()
                .unwrap_or("unclear_from_transcript")
                .to_string(),
            user_satisfaction_counts: parse_string_u32_map(&value["user_satisfaction_counts"]),
            claude_helpfulness: value["claude_helpfulness"]
                .as_str()
                .unwrap_or("moderately_helpful")
                .to_string(),
            session_type: value["session_type"].as_str().unwrap_or("single_task").to_string(),
            friction_counts: parse_string_u32_map(&value["friction_counts"]),
            friction_detail: value["friction_detail"].as_str().unwrap_or("").to_string(),
            primary_success: value["primary_success"].as_str().unwrap_or("").to_string(),
            brief_summary: value["brief_summary"].as_str().unwrap_or("").to_string(),
            languages_used: value["languages_used"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
            user_instructions: value["user_instructions"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default(),
        };

        let _ = facet_cache::save_cached_facet(transcript, &facet).await;

        Ok(facet)
    }

    // ============ Language helpers ============

    async fn get_user_language() -> String {
        match get_global_config_service().await {
            Ok(config_service) => match config_service.config::<AppConfig>(Some("app")).await {
                Ok(app_config) => app_config.language,
                Err(_) => "en-US".to_string(),
            },
            Err(_) => "en-US".to_string(),
        }
    }

    fn build_language_instruction(lang: &str) -> String {
        let json_rule = concat!(
            "\n\nCRITICAL JSON RULE: Inside JSON string values you MUST escape every literal double-quote as \\\".",
            " Do NOT place unescaped \" characters inside string values.",
            " For example, write \"he said \\\"hello\\\"\" instead of \"he said \"hello\"\".",
        );

        if lang.starts_with("en") {
            json_rule.to_string()
        } else {
            let lang_name = match lang {
                "ja" | "ja-JP" => "Japanese (日本語)",
                "ko" | "ko-KR" => "Korean (한국어)",
                "fr" | "fr-FR" => "French (Français)",
                "de" | "de-DE" => "German (Deutsch)",
                "es" | "es-ES" => "Spanish (Español)",
                "pt" | "pt-BR" => "Portuguese (Português)",
                "ru" | "ru-RU" => "Russian (Русский)",
                _ => LocaleId::from_str(lang)
                    .map(|locale| locale.model_language_name())
                    .unwrap_or(lang),
            };
            format!(
                "\n\nIMPORTANT: All descriptive text, summaries, suggestions, and narrative content in your response MUST be written in {}. Keep JSON keys and enum values in English.{}",
                lang_name, json_rule
            )
        }
    }
}
