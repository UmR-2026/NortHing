//! Shared Deep Review manifest data types and JSON helper primitives.
//!
//! This file owns the cross-sibling vocabulary that the `manifest.rs`
//! sub-domain siblings (`scope_profile`, `evidence_pack`,
//! `run_manifest_gate`) all need:
//!
//! - `DeepReviewEvidencePackValidationError` — the typed error surface used by
//!   every evidence-pack validator. Sibling modules construct it via the small
//!   helper constructors (`new`, `missing_field`, `invalid_field`,
//!   `too_many_items`).
//! - Generic JSON access helpers (`value_for_any_key`,
//!   `normalized_non_empty_string`, `string_for_any_key`,
//!   `scope_dependency_hops_to_string`) — defensive manifest reads that
//!   accept either camelCase or snake_case keys and trim whitespace.
//! - Required-value validators (`ensure_object`, `required_value_for_any_key`,
//!   `required_string_for_any_key`, `required_u64_for_any_key`,
//!   `required_array_for_any_key`, `required_string_array_for_any_key`,
//!   `validate_budget_cap`) — used by `evidence_pack` to enforce budget caps
//!   and required field shapes against an incoming manifest.
//!
//! The name is `manifest_helpers` (rather than the R37c task-execution
//! `types.rs`) because this file is owned by the `manifest.rs` facade
//! sub-domain. Task-execution shared types live in `super::types`.
//!
//! Concrete scope profile, evidence pack, and run-manifest-gate logic live in
//! the sibling modules; this file holds the shared vocabulary only.

use serde_json::Value;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepReviewEvidencePackValidationError {
    detail: String,
}

impl DeepReviewEvidencePackValidationError {
    pub fn new(detail: impl Into<String>) -> Self {
        Self { detail: detail.into() }
    }

    pub fn missing_field(field: &'static str) -> Self {
        Self::new(format!("missing evidence pack field '{}'", field))
    }

    pub fn invalid_field(field: &'static str, reason: &'static str) -> Self {
        Self::new(format!("invalid evidence pack field '{}': {}", field, reason))
    }

    pub fn too_many_items(field: &'static str, max: usize, actual: usize) -> Self {
        Self::new(format!(
            "too many evidence pack items in '{}': max {}, got {}",
            field, max, actual
        ))
    }
}

impl fmt::Display for DeepReviewEvidencePackValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

pub(super) fn value_for_any_key<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key))
}

pub(super) fn normalized_non_empty_string(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

pub(super) fn string_for_any_key(value: &Value, keys: &[&str]) -> Option<String> {
    value_for_any_key(value, keys).and_then(normalized_non_empty_string)
}

pub(super) fn scope_dependency_hops_to_string(value: &Value) -> Option<String> {
    if let Some(hops) = value.as_u64() {
        return Some(hops.to_string());
    }
    normalized_non_empty_string(value)
}

pub(super) fn ensure_object(value: &Value, field: &'static str) -> Result<(), DeepReviewEvidencePackValidationError> {
    if value.is_object() {
        Ok(())
    } else {
        Err(DeepReviewEvidencePackValidationError::invalid_field(
            field,
            "expected object",
        ))
    }
}

pub(super) fn required_value_for_any_key<'a>(
    value: &'a Value,
    keys: &[&str],
    field: &'static str,
) -> Result<&'a Value, DeepReviewEvidencePackValidationError> {
    value_for_any_key(value, keys).ok_or_else(|| DeepReviewEvidencePackValidationError::missing_field(field))
}

pub(super) fn required_string_for_any_key(
    value: &Value,
    keys: &[&str],
    field: &'static str,
) -> Result<String, DeepReviewEvidencePackValidationError> {
    string_for_any_key(value, keys)
        .ok_or_else(|| DeepReviewEvidencePackValidationError::invalid_field(field, "expected non-empty string"))
}

pub(super) fn required_u64_for_any_key(
    value: &Value,
    keys: &[&str],
    field: &'static str,
) -> Result<u64, DeepReviewEvidencePackValidationError> {
    required_value_for_any_key(value, keys, field)?
        .as_u64()
        .ok_or_else(|| DeepReviewEvidencePackValidationError::invalid_field(field, "expected unsigned integer"))
}

pub(super) fn required_array_for_any_key<'a>(
    value: &'a Value,
    keys: &[&str],
    field: &'static str,
    max: usize,
) -> Result<&'a Vec<Value>, DeepReviewEvidencePackValidationError> {
    let array = required_value_for_any_key(value, keys, field)?
        .as_array()
        .ok_or_else(|| DeepReviewEvidencePackValidationError::invalid_field(field, "expected array"))?;
    if array.len() > max {
        return Err(DeepReviewEvidencePackValidationError::too_many_items(
            field,
            max,
            array.len(),
        ));
    }
    Ok(array)
}

pub(super) fn required_string_array_for_any_key(
    value: &Value,
    keys: &[&str],
    field: &'static str,
    max: usize,
) -> Result<Vec<String>, DeepReviewEvidencePackValidationError> {
    required_array_for_any_key(value, keys, field, max)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    DeepReviewEvidencePackValidationError::invalid_field(field, "expected non-empty string items")
                })
        })
        .collect()
}

pub(super) fn validate_budget_cap(
    budget: &Value,
    keys: &[&str],
    field: &'static str,
    max: usize,
) -> Result<(), DeepReviewEvidencePackValidationError> {
    let cap = required_u64_for_any_key(budget, keys, field)?;
    if cap as usize > max {
        return Err(DeepReviewEvidencePackValidationError::invalid_field(
            field,
            "exceeds supported manifest cap",
        ));
    }
    Ok(())
}
