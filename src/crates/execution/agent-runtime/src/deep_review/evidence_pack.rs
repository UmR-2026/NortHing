//! Deep Review evidence pack — typed view of `manifest.evidencePack`.
//!
//! The frontend builds the launch manifest, but Rust owns the final trust
//! boundary. `DeepReviewEvidencePack::from_manifest` enforces that:
//!
//! - `reviewMode == "deep"` (otherwise the pack is absent)
//! - the evidence pack shape is current (`version == 1`, `source ==
//!   "target_manifest"`)
//! - no forbidden content payload key (`sourceText`, `fullDiff`,
//!   `modelOutput`, `providerRawBody`, `fullFileContents`) appears anywhere
//!   in the pack — accepted in either camelCase or snake_case spelling
//! - per-array item caps (`EVIDENCE_PACK_CHANGED_FILE_LIMIT`,
//!   `EVIDENCE_PACK_HUNK_HINT_LIMIT`, `EVIDENCE_PACK_CONTRACT_HINT_LIMIT`,
//!   `EVIDENCE_PACK_PACKET_ID_LIMIT`, `EVIDENCE_PACK_TAG_LIMIT`) hold
//! - the manifest budget block is well-formed and within those caps
//! - `privacy.content == "metadata_only"` and every required privacy
//!   exclude is present in `privacy.excludes`
//!
//! All violations return `DeepReviewEvidencePackValidationError` (defined
//! in `super::types`) so the caller can surface a typed diagnostic.

use super::manifest_helpers::{
    ensure_object, required_array_for_any_key, required_string_array_for_any_key, required_string_for_any_key,
    required_u64_for_any_key, required_value_for_any_key, string_for_any_key, validate_budget_cap,
    DeepReviewEvidencePackValidationError,
};
use serde_json::Value;
use std::collections::HashSet;

pub(crate) const EVIDENCE_PACK_CHANGED_FILE_LIMIT: usize = 80;
pub(crate) const EVIDENCE_PACK_HUNK_HINT_LIMIT: usize = 80;
pub(crate) const EVIDENCE_PACK_CONTRACT_HINT_LIMIT: usize = 40;
pub(crate) const EVIDENCE_PACK_PACKET_ID_LIMIT: usize = 256;
pub(crate) const EVIDENCE_PACK_TAG_LIMIT: usize = 32;

pub(crate) const EVIDENCE_PACK_PRIVACY_EXCLUDES: &[&str] = &[
    "source_text",
    "full_diff",
    "model_output",
    "provider_raw_body",
    "full_file_contents",
];

const EVIDENCE_PACK_FORBIDDEN_KEYS: &[&str] = &[
    "sourceText",
    "source_text",
    "fullDiff",
    "full_diff",
    "modelOutput",
    "model_output",
    "providerRawBody",
    "provider_raw_body",
    "fullFileContents",
    "full_file_contents",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepReviewEvidencePack {
    version: u64,
    source: String,
    changed_files: Vec<String>,
    packet_ids: Vec<String>,
    hunk_hint_count: usize,
    contract_hint_count: usize,
    content_boundary: String,
}

impl DeepReviewEvidencePack {
    pub fn from_manifest(raw: &Value) -> Result<Option<Self>, DeepReviewEvidencePackValidationError> {
        if string_for_any_key(raw, &["reviewMode", "review_mode"]).as_deref() != Some("deep") {
            return Ok(None);
        }

        let Some(pack) = raw
            .as_object()
            .and_then(|m| m.get("evidencePack").or_else(|| m.get("evidence_pack")))
        else {
            return Ok(None);
        };
        ensure_object(pack, "evidencePack")?;
        if let Some(key) = forbidden_evidence_pack_key(pack) {
            return Err(DeepReviewEvidencePackValidationError::new(format!(
                "forbidden evidence pack field '{}'",
                key
            )));
        }

        let version = required_u64_for_any_key(pack, &["version"], "version")?;
        if version != 1 {
            return Err(DeepReviewEvidencePackValidationError::invalid_field(
                "version",
                "expected 1",
            ));
        }

        let source = required_string_for_any_key(pack, &["source"], "source")?;
        if source != "target_manifest" {
            return Err(DeepReviewEvidencePackValidationError::invalid_field(
                "source",
                "expected target_manifest",
            ));
        }

        let changed_files = required_string_array_for_any_key(
            pack,
            &["changedFiles", "changed_files"],
            "changedFiles",
            EVIDENCE_PACK_CHANGED_FILE_LIMIT,
        )?;
        let domain_tags = required_string_array_for_any_key(
            pack,
            &["domainTags", "domain_tags"],
            "domainTags",
            EVIDENCE_PACK_TAG_LIMIT,
        )?;
        let risk_focus_tags = required_string_array_for_any_key(
            pack,
            &["riskFocusTags", "risk_focus_tags"],
            "riskFocusTags",
            EVIDENCE_PACK_TAG_LIMIT,
        )?;
        let packet_ids = required_string_array_for_any_key(
            pack,
            &["packetIds", "packet_ids"],
            "packetIds",
            EVIDENCE_PACK_PACKET_ID_LIMIT,
        )?;

        let diff_stat = required_value_for_any_key(pack, &["diffStat", "diff_stat"], "diffStat")?;
        ensure_object(diff_stat, "diffStat")?;
        required_u64_for_any_key(diff_stat, &["fileCount", "file_count"], "diffStat.fileCount")?;
        required_string_for_any_key(
            diff_stat,
            &["lineCountSource", "line_count_source"],
            "diffStat.lineCountSource",
        )?;

        let hunk_hints = required_array_for_any_key(
            pack,
            &["hunkHints", "hunk_hints"],
            "hunkHints",
            EVIDENCE_PACK_HUNK_HINT_LIMIT,
        )?;
        for hint in hunk_hints {
            ensure_object(hint, "hunkHints[]")?;
            required_string_for_any_key(hint, &["filePath", "file_path"], "hunkHints[].filePath")?;
            required_u64_for_any_key(
                hint,
                &["changedLineCount", "changed_line_count"],
                "hunkHints[].changedLineCount",
            )?;
            required_string_for_any_key(
                hint,
                &["lineCountSource", "line_count_source"],
                "hunkHints[].lineCountSource",
            )?;
        }

        let contract_hints = required_array_for_any_key(
            pack,
            &["contractHints", "contract_hints"],
            "contractHints",
            EVIDENCE_PACK_CONTRACT_HINT_LIMIT,
        )?;
        for hint in contract_hints {
            ensure_object(hint, "contractHints[]")?;
            let kind = required_string_for_any_key(hint, &["kind"], "contractHints[].kind")?;
            if !matches!(
                kind.as_str(),
                "i18n_key" | "tauri_command" | "api_contract" | "config_key"
            ) {
                return Err(DeepReviewEvidencePackValidationError::invalid_field(
                    "contractHints[].kind",
                    "unknown contract hint kind",
                ));
            }
            required_string_for_any_key(hint, &["filePath", "file_path"], "contractHints[].filePath")?;
            let hint_source = required_string_for_any_key(hint, &["source"], "contractHints[].source")?;
            if hint_source != "path_classifier" {
                return Err(DeepReviewEvidencePackValidationError::invalid_field(
                    "contractHints[].source",
                    "expected path_classifier",
                ));
            }
        }

        validate_evidence_pack_budget(pack)?;
        let content_boundary = validate_evidence_pack_privacy(pack)?;

        let _ = (domain_tags, risk_focus_tags);

        Ok(Some(Self {
            version,
            source,
            changed_files,
            packet_ids,
            hunk_hint_count: hunk_hints.len(),
            contract_hint_count: contract_hints.len(),
            content_boundary,
        }))
    }
}

#[cfg(test)]
impl DeepReviewEvidencePack {
    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn changed_files(&self) -> &[String] {
        &self.changed_files
    }

    pub fn packet_ids(&self) -> &[String] {
        &self.packet_ids
    }

    pub fn hunk_hint_count(&self) -> usize {
        self.hunk_hint_count
    }

    pub fn contract_hint_count(&self) -> usize {
        self.contract_hint_count
    }

    pub fn content_boundary(&self) -> &str {
        &self.content_boundary
    }

    pub fn requires_tool_confirmation(&self) -> bool {
        true
    }
}

fn validate_evidence_pack_budget(pack: &Value) -> Result<(), DeepReviewEvidencePackValidationError> {
    let budget = required_value_for_any_key(pack, &["budget"], "budget")?;
    ensure_object(budget, "budget")?;
    validate_budget_cap(
        budget,
        &["maxChangedFiles", "max_changed_files"],
        "budget.maxChangedFiles",
        EVIDENCE_PACK_CHANGED_FILE_LIMIT,
    )?;
    validate_budget_cap(
        budget,
        &["maxHunkHints", "max_hunk_hints"],
        "budget.maxHunkHints",
        EVIDENCE_PACK_HUNK_HINT_LIMIT,
    )?;
    validate_budget_cap(
        budget,
        &["maxContractHints", "max_contract_hints"],
        "budget.maxContractHints",
        EVIDENCE_PACK_CONTRACT_HINT_LIMIT,
    )?;
    required_u64_for_any_key(
        budget,
        &["omittedChangedFileCount", "omitted_changed_file_count"],
        "budget.omittedChangedFileCount",
    )?;
    required_u64_for_any_key(
        budget,
        &["omittedHunkHintCount", "omitted_hunk_hint_count"],
        "budget.omittedHunkHintCount",
    )?;
    required_u64_for_any_key(
        budget,
        &["omittedContractHintCount", "omitted_contract_hint_count"],
        "budget.omittedContractHintCount",
    )?;
    Ok(())
}

fn validate_evidence_pack_privacy(pack: &Value) -> Result<String, DeepReviewEvidencePackValidationError> {
    let privacy = required_value_for_any_key(pack, &["privacy"], "privacy")?;
    ensure_object(privacy, "privacy")?;
    let content = required_string_for_any_key(privacy, &["content"], "privacy.content")?;
    if content != "metadata_only" {
        return Err(DeepReviewEvidencePackValidationError::invalid_field(
            "privacy.content",
            "expected metadata_only",
        ));
    }
    let excludes = required_string_array_for_any_key(
        privacy,
        &["excludes"],
        "privacy.excludes",
        EVIDENCE_PACK_PRIVACY_EXCLUDES.len(),
    )?;
    let excludes = excludes.into_iter().collect::<HashSet<_>>();
    for required in EVIDENCE_PACK_PRIVACY_EXCLUDES {
        if !excludes.contains(*required) {
            return Err(DeepReviewEvidencePackValidationError::invalid_field(
                "privacy.excludes",
                "missing required excluded content type",
            ));
        }
    }
    Ok(content)
}

fn forbidden_evidence_pack_key(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if EVIDENCE_PACK_FORBIDDEN_KEYS.contains(&key.as_str()) {
                    return Some(key.clone());
                }
                if let Some(nested) = forbidden_evidence_pack_key(child) {
                    return Some(nested);
                }
            }
            None
        }
        Value::Array(items) => items.iter().find_map(forbidden_evidence_pack_key),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{DeepReviewEvidencePack, EVIDENCE_PACK_HUNK_HINT_LIMIT};
    use serde_json::{json, Value};

    fn valid_evidence_pack_manifest() -> Value {
        json!({
            "reviewMode": "deep",
            "evidencePack": {
                "version": 1,
                "source": "target_manifest",
                "changedFiles": ["src/crates/adapters/api-layer/src/review.rs"],
                "diffStat": {
                    "fileCount": 1,
                    "totalChangedLines": 4,
                    "lineCountSource": "diff_stat"
                },
                "domainTags": ["api_layer"],
                "riskFocusTags": ["cross_boundary_api_contracts"],
                "packetIds": ["reviewer:ReviewArchitecture", "judge:ReviewJudge"],
                "hunkHints": [
                    {
                        "filePath": "src/crates/adapters/api-layer/src/review.rs",
                        "changedLineCount": 4,
                        "lineCountSource": "diff_stat"
                    }
                ],
                "contractHints": [
                    {
                        "kind": "api_contract",
                        "filePath": "src/crates/adapters/api-layer/src/review.rs",
                        "source": "path_classifier"
                    }
                ],
                "budget": {
                    "maxChangedFiles": 80,
                    "maxHunkHints": 80,
                    "maxContractHints": 40,
                    "omittedChangedFileCount": 0,
                    "omittedHunkHintCount": 0,
                    "omittedContractHintCount": 0
                },
                "privacy": {
                    "content": "metadata_only",
                    "excludes": [
                        "source_text",
                        "full_diff",
                        "model_output",
                        "provider_raw_body",
                        "full_file_contents"
                    ]
                }
            }
        })
    }

    #[test]
    fn evidence_pack_parses_metadata_only_manifest() {
        let manifest = valid_evidence_pack_manifest();

        let pack = DeepReviewEvidencePack::from_manifest(&manifest)
            .expect("evidence pack should validate")
            .expect("evidence pack should be present");

        assert_eq!(pack.version(), 1);
        assert_eq!(pack.source(), "target_manifest");
        assert_eq!(pack.content_boundary(), "metadata_only");
        assert_eq!(
            pack.changed_files().iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["src/crates/adapters/api-layer/src/review.rs"]
        );
        assert_eq!(
            pack.packet_ids().iter().map(String::as_str).collect::<Vec<_>>(),
            vec!["reviewer:ReviewArchitecture", "judge:ReviewJudge"]
        );
        assert_eq!(pack.hunk_hint_count(), 1);
        assert_eq!(pack.contract_hint_count(), 1);
        assert!(pack.requires_tool_confirmation());
    }

    #[test]
    fn evidence_pack_parses_snake_case_manifest() {
        let manifest = json!({
            "review_mode": "deep",
            "evidence_pack": {
                "version": 1,
                "source": "target_manifest",
                "changed_files": ["src/web-ui/src/locales/en-US/flow-chat.json"],
                "diff_stat": {
                    "file_count": 1,
                    "total_changed_lines": 2,
                    "line_count_source": "diff_stat"
                },
                "domain_tags": ["frontend_i18n"],
                "risk_focus_tags": ["configuration_changes"],
                "packet_ids": ["reviewer:ReviewFrontend"],
                "hunk_hints": [
                    {
                        "file_path": "src/web-ui/src/locales/en-US/flow-chat.json",
                        "changed_line_count": 2,
                        "line_count_source": "diff_stat"
                    }
                ],
                "contract_hints": [
                    {
                        "kind": "i18n_key",
                        "file_path": "src/web-ui/src/locales/en-US/flow-chat.json",
                        "source": "path_classifier"
                    }
                ],
                "budget": {
                    "max_changed_files": 80,
                    "max_hunk_hints": 80,
                    "max_contract_hints": 40,
                    "omitted_changed_file_count": 0,
                    "omitted_hunk_hint_count": 0,
                    "omitted_contract_hint_count": 0
                },
                "privacy": {
                    "content": "metadata_only",
                    "excludes": [
                        "source_text",
                        "full_diff",
                        "model_output",
                        "provider_raw_body",
                        "full_file_contents"
                    ]
                }
            }
        });

        let pack = DeepReviewEvidencePack::from_manifest(&manifest)
            .expect("snake-case evidence pack should validate")
            .expect("evidence pack should be present");

        assert_eq!(pack.changed_files()[0], "src/web-ui/src/locales/en-US/flow-chat.json");
        assert_eq!(pack.contract_hint_count(), 1);
    }

    #[test]
    fn evidence_pack_missing_stays_compatible_with_legacy_manifest() {
        let manifest = json!({
            "reviewMode": "deep",
            "workPackets": []
        });

        assert_eq!(
            DeepReviewEvidencePack::from_manifest(&manifest).expect("legacy manifest should parse"),
            None
        );
    }

    #[test]
    fn evidence_pack_rejects_forbidden_source_or_diff_payload_keys() {
        let mut manifest = valid_evidence_pack_manifest();
        manifest["evidencePack"]["sourceText"] = json!("fn main() {}");

        let error = DeepReviewEvidencePack::from_manifest(&manifest).expect_err("source text must not be accepted");

        assert!(error.to_string().contains("forbidden evidence pack field"));
        assert!(error.to_string().contains("sourceText"));
    }

    #[test]
    fn evidence_pack_rejects_non_metadata_privacy_boundary() {
        let mut manifest = valid_evidence_pack_manifest();
        manifest["evidencePack"]["privacy"]["content"] = json!("full_diff");

        let error =
            DeepReviewEvidencePack::from_manifest(&manifest).expect_err("full diff content must not be accepted");

        assert!(error.to_string().contains("privacy.content"));
        assert!(error.to_string().contains("metadata_only"));
    }

    #[test]
    fn evidence_pack_rejects_oversized_hunk_hint_arrays() {
        let mut manifest = valid_evidence_pack_manifest();
        let hunk_hints = (0..=EVIDENCE_PACK_HUNK_HINT_LIMIT)
            .map(|index| {
                json!({
                    "filePath": format!("src/lib_{index}.rs"),
                    "changedLineCount": 1,
                    "lineCountSource": "diff_stat"
                })
            })
            .collect::<Vec<_>>();
        manifest["evidencePack"]["hunkHints"] = json!(hunk_hints);

        let error =
            DeepReviewEvidencePack::from_manifest(&manifest).expect_err("oversized hunk hints must be rejected");

        assert!(error.to_string().contains("hunkHints"));
        assert!(error.to_string().contains("max 80"));
    }
}
