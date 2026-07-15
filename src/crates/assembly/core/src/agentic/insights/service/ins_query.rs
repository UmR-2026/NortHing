//! Report save/load/query for InsightsService.

use crate::agentic::insights::session_paths::collect_effective_session_storage_roots;
use crate::agentic::insights::types::*;
use crate::agentic::persistence::PersistenceManager;
use crate::infrastructure::events::{emit_global_event, BackendEvent};
use crate::infrastructure::path_manager_arc;
use crate::util::errors::{NortHingError, NortHingResult};
use chrono::DateTime;
use serde_json::Value;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

use super::ins_types::*;
use super::InsightsService;

impl InsightsService {
    // ============ Save / Load / Utility ============

    pub(crate) async fn save_report(mut report: InsightsReport, locale: &str) -> NortHingResult<InsightsReport> {
        let path_manager = path_manager_arc();
        let usage_dir = path_manager.user_data_dir().join("usage-data");
        tokio::fs::create_dir_all(&usage_dir)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to create usage-data dir: {}", e)))?;

        let timestamp = report.generated_at;

        let html_content = crate::agentic::insights::html::generate_html(&report, locale);
        let html_path = usage_dir.join(format!("insights-{}.html", timestamp));
        tokio::fs::write(&html_path, &html_content)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to write HTML report: {}", e)))?;

        report.html_report_path = Some(html_path.to_string_lossy().to_string());

        let json_path = usage_dir.join(format!("insights-{}.json", timestamp));
        let json_str = serde_json::to_string_pretty(&report)
            .map_err(|e| NortHingError::serialization(format!("Failed to serialize report: {}", e)))?;
        tokio::fs::write(&json_path, &json_str)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to write report JSON: {}", e)))?;

        info!(
            "Report saved: json={}, html={}",
            json_path.display(),
            html_path.display()
        );

        Self::cleanup_old_reports(&usage_dir, 5).await;

        Ok(report)
    }

    async fn cleanup_old_reports(usage_dir: &Path, keep: usize) {
        let mut entries = match tokio::fs::read_dir(usage_dir).await {
            Ok(dir) => dir,
            Err(_) => return,
        };

        let mut json_files: Vec<std::path::PathBuf> = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("insights-") && name.ends_with(".json") {
                json_files.push(entry.path());
            }
        }

        json_files.sort();
        json_files.reverse();

        for old in json_files.into_iter().skip(keep) {
            let _ = tokio::fs::remove_file(&old).await;
            let html = old.with_extension("html");
            let _ = tokio::fs::remove_file(&html).await;
        }
    }

    pub async fn has_data(days: u32) -> NortHingResult<bool> {
        let path_manager = path_manager_arc();
        let pm = PersistenceManager::new(path_manager)?;
        let cutoff = SystemTime::now() - std::time::Duration::from_secs(days as u64 * 86400);

        for ws_path in collect_effective_session_storage_roots().await {
            if let Ok(sessions) = pm.list_sessions(&ws_path).await {
                if sessions.iter().any(|s| s.last_activity_at >= cutoff) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub async fn load_report(path: &str) -> NortHingResult<InsightsReport> {
        let json_str = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to read report file: {}", e)))?;
        let report: InsightsReport = serde_json::from_str(&json_str)
            .map_err(|e| NortHingError::Deserialization(format!("Failed to parse report: {}", e)))?;
        Ok(report)
    }

    pub async fn load_latest_reports() -> NortHingResult<Vec<InsightsReportMeta>> {
        let path_manager = path_manager_arc();
        let usage_dir = path_manager.user_data_dir().join("usage-data");

        if !usage_dir.exists() {
            return Ok(vec![]);
        }

        let mut entries = tokio::fs::read_dir(&usage_dir)
            .await
            .map_err(|e| NortHingError::io(format!("Failed to read usage-data dir: {}", e)))?;

        let mut json_files: Vec<std::path::PathBuf> = Vec::new();
        while let Ok(Some(entry)) = entries.next_entry().await {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("insights-") && name.ends_with(".json") {
                json_files.push(entry.path());
            }
        }

        json_files.sort();
        json_files.reverse();

        let mut reports = Vec::new();
        for json_path in json_files.iter().take(10) {
            match tokio::fs::read_to_string(json_path).await {
                Ok(json_str) => match serde_json::from_str::<InsightsReport>(&json_str) {
                    Ok(report) => {
                        let top_goals: Vec<String> = report
                            .stats
                            .top_goals
                            .iter()
                            .take(3)
                            .map(|(name, _)| name.clone())
                            .collect();
                        let mut lang_entries: Vec<_> = report.stats.languages.iter().collect();
                        lang_entries.sort_by(|(_, a), (_, b)| b.cmp(a));
                        let languages: Vec<String> =
                            lang_entries.iter().take(3).map(|(name, _)| name.to_string()).collect();

                        reports.push(InsightsReportMeta {
                            generated_at: report.generated_at,
                            total_sessions: report.total_sessions,
                            analyzed_sessions: report.analyzed_sessions,
                            date_range: report.date_range,
                            path: json_path.to_string_lossy().to_string(),
                            total_messages: report.total_messages,
                            days_covered: report.days_covered,
                            total_hours: report.stats.total_hours,
                            top_goals,
                            languages,
                        });
                    }
                    Err(e) => {
                        warn!("Failed to parse report {}: {}", json_path.display(), e);
                    }
                },
                Err(e) => {
                    warn!("Failed to read report {}: {}", json_path.display(), e);
                }
            }
        }

        Ok(reports)
    }

    pub(crate) async fn emit_progress(message: &str, stage: &str, current: usize, total: usize) {
        let payload = serde_json::json!({
            "message": message,
            "stage": stage,
            "current": current,
            "total": total,
        });
        if let Err(e) = emit_global_event(BackendEvent::Custom {
            event_name: "insights-progress".to_string(),
            payload,
        })
        .await
        {
            debug!("Failed to emit progress event: {}", e);
        }
    }
}
