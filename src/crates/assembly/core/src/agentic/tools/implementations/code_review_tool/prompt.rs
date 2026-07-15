//! Prompt / schema construction for `CodeReviewTool`.
//!
//! Owns input schema generation for all supported locales and both
//! standard/deep review modes.

use crate::agentic::core::CompressionContract;
use crate::agentic::deep_review::report as deep_review_report;
use crate::agentic::tools::framework::ToolUseContext;
use crate::service::i18n::code_review_copy_for_language;
use serde_json::{json, Value};

impl super::CodeReviewTool {
    /// Sync schema fallback (e.g. tests); prefers zh-CN wording. For model calls use [`input_schema_for_model`].
    pub fn input_schema_value() -> Value {
        Self::input_schema_value_for_language("zh-CN")
    }

    pub fn description_for_language(lang_code: &str) -> String {
        code_review_copy_for_language(lang_code).description.to_string()
    }

    pub fn input_schema_value_for_language(lang_code: &str) -> Value {
        Self::input_schema_value_for_language_with_mode(lang_code, false)
    }

    pub fn input_schema_value_for_language_with_mode(lang_code: &str, require_deep_fields: bool) -> Value {
        let copy = code_review_copy_for_language(lang_code);
        let (scope_desc, reviewer_summary_desc, source_reviewer_desc, validation_note_desc, plan_desc) = match lang_code
        {
            "en-US" => (
                "Human-readable review scope (optional, in English)",
                "Reviewer summary (in English)",
                "Reviewer source / role (optional, in English)",
                "Validation or triage note (optional, in English)",
                "Concrete remediation / follow-up plan items (in English)",
            ),
            "zh-TW" => (
                "Human-readable review scope (optional, in Traditional Chinese)",
                "Reviewer summary (in Traditional Chinese)",
                "Reviewer source / role (optional, in Traditional Chinese)",
                "Validation or triage note (optional, in Traditional Chinese)",
                "Concrete remediation / follow-up plan items (in Traditional Chinese)",
            ),
            _ => (
                "Human-readable review scope (optional, in Simplified Chinese)",
                "Reviewer summary (in Simplified Chinese)",
                "Reviewer source / role (optional, in Simplified Chinese)",
                "Validation or triage note (optional, in Simplified Chinese)",
                "Concrete remediation / follow-up plan items (in Simplified Chinese)",
            ),
        };
        let mut required = vec!["summary", "issues", "positive_points"];
        if require_deep_fields {
            required.extend(["review_mode", "review_scope", "reviewers", "remediation_plan"]);
        }

        json!({
            "type": "object",
            "properties": {
                "schema_version": {
                    "type": "integer",
                    "description": "Schema version for forward compatibility",
                    "default": 1
                },
                "summary": {
                    "type": "object",
                    "description": "Review summary",
                    "properties": {
                        "overall_assessment": {
                            "type": "string",
                            "description": copy.overall_assessment
                        },
                        "risk_level": {
                            "type": "string",
                            "enum": ["low", "medium", "high", "critical"],
                            "description": "Risk level"
                        },
                        "recommended_action": {
                            "type": "string",
                            "enum": ["approve", "approve_with_suggestions", "request_changes", "block"],
                            "description": "Recommended action"
                        },
                        "confidence_note": {
                            "type": "string",
                            "description": copy.confidence_note
                        }
                    },
                    "required": ["overall_assessment", "risk_level", "recommended_action"]
                },
                "issues": {
                    "type": "array",
                    "description": "List of issues found",
                    "items": {
                        "type": "object",
                        "properties": {
                            "severity": {
                                "type": "string",
                                "enum": ["critical", "high", "medium", "low", "info"],
                                "description": "Severity level"
                            },
                            "certainty": {
                                "type": "string",
                                "enum": ["confirmed", "likely", "possible"],
                                "description": "Certainty level"
                            },
                            "category": {
                                "type": "string",
                                "description": "Issue category (e.g., security, logic correctness, performance, etc.)"
                            },
                            "file": {
                                "type": "string",
                                "description": "File path"
                            },
                            "line": {
                                "type": ["integer", "null"],
                                "description": "Line number (null if uncertain)"
                            },
                            "title": {
                                "type": "string",
                                "description": copy.issue_title
                            },
                            "description": {
                                "type": "string",
                                "description": copy.issue_description
                            },
                            "suggestion": {
                                "type": ["string", "null"],
                                "description": copy.issue_suggestion
                            },
                            "source_reviewer": {
                                "type": "string",
                                "description": source_reviewer_desc
                            },
                            "validation_note": {
                                "type": "string",
                                "description": validation_note_desc
                            }
                        },
                        "required": ["severity", "certainty", "category", "file", "title", "description"]
                    }
                },
                "positive_points": {
                    "type": "array",
                    "description": copy.positive_points,
                    "items": {
                        "type": "string"
                    }
                },
                "review_mode": {
                    "type": "string",
                    "enum": ["standard", "deep"],
                    "description": "Review mode"
                },
                "review_scope": {
                    "type": "string",
                    "description": scope_desc
                },
                "reviewers": {
                    "type": "array",
                    "description": "Reviewer summaries",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Reviewer display name"
                            },
                            "specialty": {
                                "type": "string",
                                "description": "Reviewer specialty / role"
                            },
                            "status": {
                                "type": "string",
                                "description": "Reviewer result status"
                            },
                            "summary": {
                                "type": "string",
                                "description": reviewer_summary_desc
                            },
                            "partial_output": {
                                "type": "string",
                                "description": "Partial reviewer output captured before timeout or cancellation"
                            },
                            "packet_id": {
                                "type": "string",
                                "description": "Deep Review work packet id associated with this reviewer output"
                            },
                            "packet_status_source": {
                                "type": "string",
                                "enum": ["reported", "inferred", "missing"],
                                "description": "Whether packet_id/status was reported by the reviewer, inferred from scheduling metadata, or missing"
                            },
                            "issue_count": {
                                "type": "integer",
                                "description": "Validated issue count for this reviewer"
                            }
                        },
                        "required": ["name", "specialty", "status", "summary"],
                        "additionalProperties": false
                    }
                },
                "remediation_plan": {
                    "type": "array",
                    "description": plan_desc,
                    "items": {
                        "type": "string"
                    }
                },
                "report_sections": {
                    "type": "object",
                    "description": "Optional structured sections for richer review report presentation",
                    "properties": {
                        "executive_summary": {
                            "type": "array",
                            "description": "Short user-facing conclusion bullets",
                            "items": {
                                "type": "string"
                            }
                        },
                        "remediation_groups": {
                            "type": "object",
                            "description": "Grouped remediation and follow-up plan items",
                            "properties": {
                                "must_fix": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "should_improve": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "needs_decision": {
                                    "type": "array",
                                    "description": "Items needing user/product judgment. Each item should be an object with a 'question' and 'plan'.",
                                    "items": {
                                        "oneOf": [
                                            {
                                                "type": "object",
                                                "properties": {
                                                    "question": {
                                                        "type": "string",
                                                        "description": "The specific decision the user needs to make"
                                                    },
                                                    "plan": {
                                                        "type": "string",
                                                        "description": "The remediation plan text to execute if the user approves"
                                                    },
                                                    "options": {
                                                        "type": "array",
                                                        "description": "2-4 possible choices or approaches",
                                                        "items": { "type": "string" }
                                                    },
                                                    "tradeoffs": {
                                                        "type": "string",
                                                        "description": "Brief explanation of trade-offs between options"
                                                    },
                                                    "recommendation": {
                                                        "type": "integer",
                                                        "description": "Index of the recommended option (0-based), if any"
                                                    }
                                                },
                                                "required": ["question", "plan"]
                                            },
                                            {
                                                "type": "string"
                                            }
                                        ]
                                    }
                                },
                                "verification": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                }
                            },
                            "additionalProperties": false
                        },
                        "strength_groups": {
                            "type": "object",
                            "description": "Grouped positive observations",
                            "properties": {
                                "architecture": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "maintainability": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "tests": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "security": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "performance": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "user_experience": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                },
                                "other": {
                                    "type": "array",
                                    "items": { "type": "string" }
                                }
                            },
                            "additionalProperties": false
                        },
                        "coverage_notes": {
                            "type": "array",
                            "description": "Review coverage, confidence, timeout, cancellation, or manual follow-up notes",
                            "items": {
                                "type": "string"
                            }
                        }
                    },
                    "additionalProperties": false
                },
                "reliability_signals": {
                    "type": "array",
                    "description": "Structured reliability/status signals for Deep Review report UI and export",
                    "items": {
                        "type": "object",
                        "properties": {
                            "kind": {
                                "type": "string",
                                "enum": [
                                    "context_pressure",
                                    "compression_preserved",
                                    "cache_hit",
                                    "cache_miss",
                                    "concurrency_limited",
                                    "partial_reviewer",
                                    "reduced_scope",
                                    "retry_guidance",
                                    "skipped_reviewers",
                                    "token_budget_limited",
                                    "user_decision"
                                ],
                                "description": "Reliability signal category"
                            },
                            "severity": {
                                "type": "string",
                                "enum": ["info", "warning", "action"],
                                "description": "User-facing severity of this signal"
                            },
                            "count": {
                                "type": "integer",
                                "minimum": 0,
                                "description": "Optional affected item count"
                            },
                            "source": {
                                "type": "string",
                                "enum": ["runtime", "manifest", "report", "inferred"],
                                "description": "Where this reliability signal came from"
                            },
                            "detail": {
                                "type": "string",
                                "description": "Short user-facing detail for this signal"
                            }
                        },
                        "required": ["kind", "severity"],
                        "additionalProperties": false
                    }
                },
                "schema_version": {
                    "type": "integer",
                    "description": "Schema version for forward compatibility",
                    "minimum": 1
                }
            },
            "required": required,
            "additionalProperties": false
        })
    }

    /// Detect whether the current tool call is inside a DeepReview context.
    ///
    /// Delegates to `deep_review_report` so the policy lives in one place.
    pub fn is_deep_review_context(context: Option<&ToolUseContext>) -> bool {
        deep_review_report::is_deep_review_context(context)
    }

    /// Resolve the `CompressionContract` for the current tool context, if any.
    pub fn compression_contract_for_context(context: &ToolUseContext) -> Option<CompressionContract> {
        deep_review_report::compression_contract_for_context(context)
    }
}
