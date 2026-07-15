//! Parallel analysis orchestration for InsightsService.

mod analyze_aggregate;
mod analyze_facet;
mod analyze_suggestions;
mod analyze_wins;

#[allow(unused_imports)]
pub use analyze_aggregate::*;
#[allow(unused_imports)]
pub use analyze_facet::*;
#[allow(unused_imports)]
pub use analyze_suggestions::*;
#[allow(unused_imports)]
pub use analyze_wins::*;

use crate::agentic::insights::prompt_context::{aggregate_stats_json_for_prompt, friction_block, summaries_block};
use crate::agentic::insights::types::*;
use crate::infrastructure::ai::AIClient;
use crate::util::errors::{NortHingError, NortHingResult};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::warn;

use super::ins_types::*;
use super::InsightsService;

impl InsightsService {
    // ============ Stage 4a: Parallel Analysis ============

    pub(crate) async fn generate_analysis_parallel(
        ai_client: &Arc<AIClient>,
        aggregate: &InsightsAggregate,
        lang_instruction: &str,
    ) -> (
        InsightsSuggestions,
        Vec<ProjectArea>,
        WinsFrictionResult,
        InteractionStyleResult,
        HorizonResult,
        Option<FunEnding>,
    ) {
        let aggregate_json = aggregate_stats_json_for_prompt(aggregate);
        let summaries_text = summaries_block(aggregate);
        let friction_text = friction_block(aggregate);

        let semaphore = Arc::new(Semaphore::new(3));

        // Task 1: Suggestions
        let client_1 = ai_client.clone();
        let agg_1 = aggregate.clone();
        let lang_1 = lang_instruction.to_string();
        let sem_1 = semaphore.clone();
        let suggestions_handle = tokio::spawn(async move {
            let _permit = sem_1
                .acquire()
                .await
                .map_err(|e| NortHingError::service(format!("Semaphore error: {}", e)))?;
            Self::generate_suggestions(&client_1, &agg_1, &lang_1).await
        });

        // Task 2: Areas
        let client_2 = ai_client.clone();
        let agg_2 = aggregate.clone();
        let lang_2 = lang_instruction.to_string();
        let sem_2 = semaphore.clone();
        let areas_handle = tokio::spawn(async move {
            let _permit = sem_2
                .acquire()
                .await
                .map_err(|e| NortHingError::service(format!("Semaphore error: {}", e)))?;
            Self::identify_areas(&client_2, &agg_2, &lang_2).await
        });

        // Task 3a: Wins
        let client_3a = ai_client.clone();
        let agg_json_3a = aggregate_json.clone();
        let summaries_3a = summaries_text.clone();
        let lang_3a = lang_instruction.to_string();
        let sem_3a = semaphore.clone();
        let wins_handle = tokio::spawn(async move {
            let _permit = sem_3a
                .acquire()
                .await
                .map_err(|e| NortHingError::service(format!("Semaphore error: {}", e)))?;
            Self::analyze_wins(&client_3a, &agg_json_3a, &summaries_3a, &lang_3a).await
        });

        // Task 3b: Friction
        let client_3b = ai_client.clone();
        let agg_json_3b = aggregate_json.clone();
        let summaries_3b = summaries_text.clone();
        let friction_3b = friction_text.clone();
        let lang_3b = lang_instruction.to_string();
        let sem_3b = semaphore.clone();
        let friction_handle = tokio::spawn(async move {
            let _permit = sem_3b
                .acquire()
                .await
                .map_err(|e| NortHingError::service(format!("Semaphore error: {}", e)))?;
            Self::analyze_friction(&client_3b, &agg_json_3b, &summaries_3b, &friction_3b, &lang_3b).await
        });

        // Task 4: Interaction Style
        let client_4 = ai_client.clone();
        let agg_json_4 = aggregate_json.clone();
        let summaries_4 = summaries_text.clone();
        let lang_4 = lang_instruction.to_string();
        let sem_4 = semaphore.clone();
        let interaction_handle = tokio::spawn(async move {
            let _permit = sem_4
                .acquire()
                .await
                .map_err(|e| NortHingError::service(format!("Semaphore error: {}", e)))?;
            Self::analyze_interaction_style(&client_4, &agg_json_4, &summaries_4, &lang_4).await
        });

        // Task 5: Horizon
        let client_5 = ai_client.clone();
        let agg_json_5 = aggregate_json.clone();
        let summaries_5 = summaries_text.clone();
        let friction_5 = friction_text.clone();
        let lang_5 = lang_instruction.to_string();
        let sem_5 = semaphore.clone();
        let horizon_handle = tokio::spawn(async move {
            let _permit = sem_5
                .acquire()
                .await
                .map_err(|e| NortHingError::service(format!("Semaphore error: {}", e)))?;
            Self::generate_horizon(&client_5, &agg_json_5, &summaries_5, &friction_5, &lang_5).await
        });

        // Task 6: Fun Ending
        let client_6 = ai_client.clone();
        let agg_json_6 = aggregate_json.clone();
        let summaries_6 = summaries_text.clone();
        let lang_6 = lang_instruction.to_string();
        let sem_6 = semaphore.clone();
        let fun_ending_handle = tokio::spawn(async move {
            let _permit = sem_6
                .acquire()
                .await
                .map_err(|e| NortHingError::service(format!("Semaphore error: {}", e)))?;
            Self::generate_fun_ending(&client_6, &agg_json_6, &summaries_6, &lang_6).await
        });

        // Collect results with retry on transient failures
        let suggestions = Self::resolve_with_retry(
            suggestions_handle,
            "Suggestions",
            || async { Self::generate_suggestions(ai_client, aggregate, lang_instruction).await },
            default_suggestions,
        )
        .await;

        let areas = Self::resolve_with_retry(
            areas_handle,
            "Areas",
            || async { Self::identify_areas(ai_client, aggregate, lang_instruction).await },
            Vec::new,
        )
        .await;

        let wins_result = Self::resolve_with_retry(
            wins_handle,
            "Wins",
            || async {
                Self::analyze_wins(
                    ai_client,
                    &aggregate_stats_json_for_prompt(aggregate),
                    &summaries_block(aggregate),
                    lang_instruction,
                )
                .await
            },
            WinsResult::default,
        )
        .await;

        let friction_result = Self::resolve_with_retry(
            friction_handle,
            "Friction",
            || async {
                Self::analyze_friction(
                    ai_client,
                    &aggregate_stats_json_for_prompt(aggregate),
                    &summaries_block(aggregate),
                    &friction_block(aggregate),
                    lang_instruction,
                )
                .await
            },
            FrictionResult::default,
        )
        .await;

        let wins_friction = WinsFrictionResult {
            wins_intro: wins_result.intro,
            big_wins: wins_result.big_wins,
            friction_intro: friction_result.intro,
            friction_categories: friction_result.friction_categories,
        };

        let interaction = Self::resolve_with_retry(
            interaction_handle,
            "Interaction Style",
            || async {
                Self::analyze_interaction_style(
                    ai_client,
                    &aggregate_stats_json_for_prompt(aggregate),
                    &summaries_block(aggregate),
                    lang_instruction,
                )
                .await
            },
            InteractionStyleResult::default,
        )
        .await;

        let horizon = Self::resolve_with_retry(
            horizon_handle,
            "Horizon",
            || async {
                Self::generate_horizon(
                    ai_client,
                    &aggregate_stats_json_for_prompt(aggregate),
                    &summaries_block(aggregate),
                    &friction_block(aggregate),
                    lang_instruction,
                )
                .await
            },
            HorizonResult::default,
        )
        .await;

        let fun_ending = Self::resolve_with_retry(
            fun_ending_handle,
            "Fun Ending",
            || async {
                Self::generate_fun_ending(
                    ai_client,
                    &aggregate_stats_json_for_prompt(aggregate),
                    &summaries_block(aggregate),
                    lang_instruction,
                )
                .await
            },
            || None,
        )
        .await;

        (suggestions, areas, wins_friction, interaction, horizon, fun_ending)
    }

    /// Generic helper to resolve a spawned task with retry on transient failures.
    ///
    /// Retries on rate-limit errors, empty AI responses, and JSON extraction failures.
    async fn resolve_with_retry<T, RetryFut, RetryFn, DefaultFn>(
        handle: tokio::task::JoinHandle<NortHingResult<T>>,
        label: &str,
        retry_fn: RetryFn,
        default_fn: DefaultFn,
    ) -> T
    where
        RetryFut: std::future::Future<Output = NortHingResult<T>>,
        RetryFn: FnOnce() -> RetryFut,
        DefaultFn: FnOnce() -> T,
    {
        let result = handle
            .await
            .map_err(|e| NortHingError::service(format!("{} task panicked: {}", label, e)));

        match result {
            Ok(Ok(val)) => val,
            Ok(Err(e)) if is_retryable_error(&e) => {
                warn!("{} failed (retryable): {}, retrying after delay", label, e);
                Self::emit_progress(&format!("Retrying {}...", label.to_lowercase()), "analysis_retry", 0, 0).await;
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                retry_fn().await.unwrap_or_else(|e| {
                    warn!("{} retry failed: {}, using defaults", label, e);
                    default_fn()
                })
            }
            Ok(Err(e)) => {
                warn!("{} failed: {}, using defaults", label, e);
                default_fn()
            }
            Err(e) => {
                warn!("{} task error: {}, using defaults", label, e);
                default_fn()
            }
        }
    }
}
