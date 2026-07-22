//! Evidence pack definition and validation.

use super::types::{
    EvidencePack, EvidenceRejection, FsDiffEvidence, HumanFeedbackSlot, RateSample,
    ToolTraceEvidence,
};

const MAX_ENTRIES_PER_SLOT: usize = 16;
const MAX_EXCERPT_LEN: usize = 400;
const MAX_TOTAL_BUDGET: usize = 12_000;

impl EvidencePack {
    /// Validates the evidence pack according to the protocol rules.
    ///
    /// Returns Ok(()) if valid, Err(EvidenceRejection) if invalid.
    pub fn validate(&self) -> Result<(), EvidenceRejection> {
        // Check that traces and fs_diffs are not both empty
        if self.traces.is_empty() && self.fs_diffs.is_empty() {
            return Err(EvidenceRejection::TracesAndFsDiffsBothEmpty);
        }

        // Check slot counts
        Self::check_slot_count("traces", &self.traces.len())?;
        Self::check_slot_count("fs_diffs", &self.fs_diffs.len())?;

        // Check human_feedback slot count if Present
        if let HumanFeedbackSlot::Present(ref feedbacks) = self.human_feedback {
            Self::check_slot_count("human_feedback", &feedbacks.len())?;
        }

        // success_rate is a mandatory slot; RateSample::NoBaselineYet is a valid explicit value
        // so no additional validation is needed here beyond the struct field being present

        // Validate traces
        for (i, trace) in self.traces.iter().enumerate() {
            Self::check_whitespace_field("turn_id", &trace.turn_id)?;
            Self::check_excerpt_len("traces", i, trace.error_excerpt.len())?;
            if let Some(ref repair) = trace.repair_excerpt {
                Self::check_excerpt_len("traces", i, repair.len())?;
            }
            if Self::is_episode_source(&trace.turn_id) {
                return Err(EvidenceRejection::EpisodeSourceBlacklisted {
                    path: trace.turn_id.clone(),
                });
            }
        }

        // Validate fs_diffs
        for (i, diff) in self.fs_diffs.iter().enumerate() {
            Self::check_whitespace_field("path", &diff.path)?;
            Self::check_excerpt_len("fs_diffs", i, diff.path.len())?;
            if Self::is_episode_source(&diff.path) {
                return Err(EvidenceRejection::EpisodeSourceBlacklisted {
                    path: diff.path.clone(),
                });
            }
        }

        // Validate human_feedback
        if let HumanFeedbackSlot::Present(ref feedbacks) = self.human_feedback {
            for (i, fb) in feedbacks.iter().enumerate() {
                Self::check_excerpt_len("human_feedback", i, fb.excerpt.len())?;
                if Self::is_episode_source(&fb.origin) {
                    return Err(EvidenceRejection::EpisodeSourceBlacklisted {
                        path: fb.origin.clone(),
                    });
                }
            }
        }

        // Check total budget
        let total = self.total_character_budget();
        if total > MAX_TOTAL_BUDGET {
            return Err(EvidenceRejection::TotalBudgetExceeded {
                max: MAX_TOTAL_BUDGET,
                actual: total,
            });
        }

        Ok(())
    }

    fn check_slot_count(slot: &str, actual: &usize) -> Result<(), EvidenceRejection> {
        if *actual > MAX_ENTRIES_PER_SLOT {
            return Err(EvidenceRejection::SlotCountExceeded {
                slot: slot.to_string(),
                max: MAX_ENTRIES_PER_SLOT,
                actual: *actual,
            });
        }
        Ok(())
    }

    fn check_excerpt_len(
        slot: &str,
        index: usize,
        actual: usize,
    ) -> Result<(), EvidenceRejection> {
        if actual > MAX_EXCERPT_LEN {
            return Err(EvidenceRejection::ExcerptTooLong {
                slot: slot.to_string(),
                index,
                max: MAX_EXCERPT_LEN,
                actual,
            });
        }
        Ok(())
    }

    fn check_whitespace_field(field: &str, value: &str) -> Result<(), EvidenceRejection> {
        if value.trim().is_empty() {
            return Err(EvidenceRejection::WhitespaceField {
                field: field.to_string(),
            });
        }
        Ok(())
    }

    /// Check if a path or origin comes from the episode/diary blacklist.
    /// "northhing" here is the runtime data directory name (matching the literal
    /// used in episodes store.rs), NOT the repository name.
    /// Paths are normalized to forward slashes before matching to handle
    /// Windows backslash separators.
    fn is_episode_source(value: &str) -> bool {
        let normalized = value.replace('\\', "/");
        normalized.contains("northhing/episodes") || normalized.contains("episodes.jsonl")
    }

    /// Calculate total character budget used by all evidence.
    fn total_character_budget(&self) -> usize {
        let mut total = 0;

        for trace in &self.traces {
            total += trace.turn_id.len();
            total += trace.tool.len();
            total += trace.error_excerpt.len();
            if let Some(ref repair) = trace.repair_excerpt {
                total += repair.len();
            }
        }

        for diff in &self.fs_diffs {
            total += diff.path.len();
            total += diff.before_digest.len();
            total += diff.after_digest.len();
        }

        if let HumanFeedbackSlot::Present(ref feedbacks) = self.human_feedback {
            for fb in feedbacks {
                total += fb.origin.len();
                total += fb.excerpt.len();
            }
        }

        total
    }

    /// Returns a list of evidence IDs in the format described below.
    ///
    /// Format:
    /// - ToolTraceEvidence: T1, T2, ... Tn
    /// - FsDiffEvidence: F1, F2, ... Fn
    /// - SuccessRateComparison: S1
    /// - HumanFeedbackSlot Present: H1, H2, ... Hn
    /// - HumanFeedbackSlot Absent: no H IDs
    pub fn evidence_ids(&self) -> Vec<String> {
        let mut ids = Vec::new();

        for (i, _) in self.traces.iter().enumerate() {
            ids.push(format!("T{}", i + 1));
        }

        for (i, _) in self.fs_diffs.iter().enumerate() {
            ids.push(format!("F{}", i + 1));
        }

        // SuccessRateComparison always has S1
        ids.push("S1".to_string());

        // Human feedback IDs only if Present
        if let HumanFeedbackSlot::Present(ref feedbacks) = self.human_feedback {
            for (i, _) in feedbacks.iter().enumerate() {
                ids.push(format!("H{}", i + 1));
            }
        }

        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{AbsentReason, HumanFeedback, RateSample, SuccessRateComparison};

    fn make_valid_pack() -> EvidencePack {
        EvidencePack {
            traces: vec![ToolTraceEvidence {
                turn_id: "turn-1".to_string(),
                tool: "tool-a".to_string(),
                error_excerpt: "error".to_string(),
                repair_excerpt: None,
            }],
            fs_diffs: vec![],
            success_rate: SuccessRateComparison {
                baseline: RateSample::NoBaselineYet,
                candidate: RateSample::Present {
                    successes: 5,
                    attempts: 10,
                },
            },
            human_feedback: HumanFeedbackSlot::Absent(AbsentReason::NoHumanExposureYet),
        }
    }

    #[test]
    fn valid_pack_passes_validation() {
        let pack = make_valid_pack();
        assert!(pack.validate().is_ok());
    }

    #[test]
    fn both_traces_and_fs_diffs_empty_rejected() {
        let pack = EvidencePack {
            traces: vec![],
            fs_diffs: vec![],
            success_rate: SuccessRateComparison {
                baseline: RateSample::NoBaselineYet,
                candidate: RateSample::Present {
                    successes: 5,
                    attempts: 10,
                },
            },
            human_feedback: HumanFeedbackSlot::Absent(AbsentReason::NoHumanExposureYet),
        };
        let err = pack.validate().unwrap_err();
        assert!(matches!(err, EvidenceRejection::TracesAndFsDiffsBothEmpty));
    }

    #[test]
    fn traces_over_16_rejected() {
        let mut pack = make_valid_pack();
        pack.traces = (0..17)
            .map(|i| ToolTraceEvidence {
                turn_id: format!("turn-{}", i),
                tool: "tool".to_string(),
                error_excerpt: "e".to_string(),
                repair_excerpt: None,
            })
            .collect();
        let err = pack.validate().unwrap_err();
        assert!(matches!(err, EvidenceRejection::SlotCountExceeded { slot, .. } if slot == "traces"));
    }

    #[test]
    fn excerpt_401_chars_rejected() {
        let mut pack = make_valid_pack();
        pack.traces[0].error_excerpt = "a".repeat(401);
        let err = pack.validate().unwrap_err();
        assert!(matches!(
            err,
            EvidenceRejection::ExcerptTooLong { max: 400, actual: 401, .. }
        ));
    }

    #[test]
    fn total_budget_over_12k_rejected() {
        // Need many traces with valid-length excerpts to exceed 12k budget
        // Each trace: turn_id(~10) + tool(~10) + error_excerpt(400) + repair(400) ~= 820
        // 15 traces * 820 = 12300 > 12000
        let traces: Vec<ToolTraceEvidence> = (0..15)
            .map(|i| ToolTraceEvidence {
                turn_id: format!("turn-{}", i),
                tool: format!("tool-{}", i),
                error_excerpt: "a".repeat(400),
                repair_excerpt: Some("b".repeat(400)),
            })
            .collect();
        let pack = EvidencePack {
            traces,
            fs_diffs: vec![],
            success_rate: SuccessRateComparison {
                baseline: RateSample::NoBaselineYet,
                candidate: RateSample::Present {
                    successes: 5,
                    attempts: 10,
                },
            },
            human_feedback: HumanFeedbackSlot::Absent(AbsentReason::NoHumanExposureYet),
        };
        let err = pack.validate().unwrap_err();
        assert!(matches!(
            err,
            EvidenceRejection::TotalBudgetExceeded { max: 12_000, actual, .. } if actual > 12_000
        ));
    }

    #[test]
    fn path_with_northhing_episodes_rejected() {
        let mut pack = make_valid_pack();
        // Path contains "northhing/episodes" as per design spec blacklist
        pack.fs_diffs = vec![FsDiffEvidence {
            path: "E:/agent-project/northhing/episodes/turn-1.jsonl".to_string(),
            before_digest: "abc".to_string(),
            after_digest: "def".to_string(),
            added: 1,
            removed: 0,
        }];
        let err = pack.validate().unwrap_err();
        assert!(matches!(
            err,
            EvidenceRejection::EpisodeSourceBlacklisted { path }
            if path.contains("northhing/episodes")
        ));
    }

    #[test]
    fn path_with_northhing_backslash_episodes_rejected() {
        // Windows backslash path must also be rejected after normalization
        let mut pack = make_valid_pack();
        pack.fs_diffs = vec![FsDiffEvidence {
            path: r"C:\Users\X\AppData\Roaming\northhing\episodes\slug.jsonl".to_string(),
            before_digest: "abc".to_string(),
            after_digest: "def".to_string(),
            added: 1,
            removed: 0,
        }];
        let err = pack.validate().unwrap_err();
        assert!(matches!(
            err,
            EvidenceRejection::EpisodeSourceBlacklisted { path }
            if path.contains("northhing\\episodes") || path.contains("northhing/episodes")
        ));
    }

    #[test]
    fn turn_id_whitespace_rejected() {
        let mut pack = make_valid_pack();
        pack.traces[0].turn_id = "   ".to_string();
        let err = pack.validate().unwrap_err();
        assert!(matches!(
            err,
            EvidenceRejection::WhitespaceField { field }
            if field == "turn_id"
        ));
    }

    #[test]
    fn evidence_ids_correct_format() {
        let pack = make_valid_pack();
        let ids = pack.evidence_ids();
        assert_eq!(ids, vec!["T1", "S1"]);
    }

    #[test]
    fn evidence_ids_with_fs_diffs_and_human_feedback() {
        let pack = EvidencePack {
            traces: vec![
                ToolTraceEvidence {
                    turn_id: "turn-1".to_string(),
                    tool: "tool-a".to_string(),
                    error_excerpt: "error1".to_string(),
                    repair_excerpt: None,
                },
                ToolTraceEvidence {
                    turn_id: "turn-2".to_string(),
                    tool: "tool-b".to_string(),
                    error_excerpt: "error2".to_string(),
                    repair_excerpt: None,
                },
            ],
            fs_diffs: vec![FsDiffEvidence {
                path: "src/main.rs".to_string(),
                before_digest: "abc".to_string(),
                after_digest: "def".to_string(),
                added: 5,
                removed: 2,
            }],
            success_rate: SuccessRateComparison {
                baseline: RateSample::Present {
                    successes: 8,
                    attempts: 10,
                },
                candidate: RateSample::Present {
                    successes: 9,
                    attempts: 10,
                },
            },
            human_feedback: HumanFeedbackSlot::Present(vec![
                HumanFeedback {
                    origin: "user@example.com".to_string(),
                    excerpt: "Looks good".to_string(),
                },
            ]),
        };
        let ids = pack.evidence_ids();
        assert_eq!(ids, vec!["T1", "T2", "F1", "S1", "H1"]);
    }

    #[test]
    fn evidence_ids_human_absent_no_h_ids() {
        let pack = make_valid_pack();
        let ids = pack.evidence_ids();
        assert!(!ids.iter().any(|id| id.starts_with('H')));
    }
}
