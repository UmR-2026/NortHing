//! Strip JSON-schema fields that Gemini's tool/response schema does not
//! accept, and normalize type/nullable into Gemini's expectations.
//!
//! The main entry point is [`sanitize_schema`]. Helpers in this file are
//! module-private — they only support schema sanitization.

use serde_json::{Map, Value};

/// Strip JSON-schema fields that Gemini does not accept from a tool or
/// response schema.
pub fn sanitize_schema(value: Value) -> Value {
    strip_unsupported_schema_fields(value)
}

pub(super) fn strip_unsupported_schema_fields(value: Value) -> Value {
    match value {
        Value::Object(mut map) => {
            let all_of = map.remove("allOf");
            let any_of = map.remove("anyOf");
            let one_of = map.remove("oneOf");
            let (normalized_type, nullable_from_type) = normalize_schema_type(map.remove("type"));

            let mut sanitized = Map::new();
            for (key, value) in map {
                if key == "properties" {
                    if let Value::Object(properties) = value {
                        sanitized.insert(
                            key,
                            Value::Object(
                                properties
                                    .into_iter()
                                    .map(|(name, schema)| (name, strip_unsupported_schema_fields(schema)))
                                    .collect(),
                            ),
                        );
                    }
                    continue;
                }

                if is_supported_schema_key(&key) {
                    sanitized.insert(key, strip_unsupported_schema_fields(value));
                }
            }

            if let Some(all_of) = all_of {
                merge_schema_variants(&mut sanitized, all_of, true);
            }

            let mut nullable = nullable_from_type;
            if let Some(any_of) = any_of {
                nullable |= merge_union_variants(&mut sanitized, any_of);
            }
            if let Some(one_of) = one_of {
                nullable |= merge_union_variants(&mut sanitized, one_of);
            }

            if let Some(schema_type) = normalized_type {
                sanitized.insert("type".to_string(), Value::String(schema_type));
            }
            if nullable {
                sanitized.insert("nullable".to_string(), Value::Bool(true));
            }

            Value::Object(sanitized)
        }
        Value::Array(items) => Value::Array(items.into_iter().map(strip_unsupported_schema_fields).collect()),
        other => other,
    }
}

fn is_supported_schema_key(key: &str) -> bool {
    matches!(
        key,
        "type"
            | "format"
            | "description"
            | "nullable"
            | "enum"
            | "items"
            | "properties"
            | "required"
            | "minItems"
            | "maxItems"
            | "minimum"
            | "maximum"
            | "minLength"
            | "maxLength"
            | "pattern"
    )
}

fn normalize_schema_type(type_value: Option<Value>) -> (Option<String>, bool) {
    match type_value {
        Some(Value::String(value)) if value != "null" => (Some(value), false),
        Some(Value::String(_)) => (None, true),
        Some(Value::Array(values)) => {
            let mut types = values
                .into_iter()
                .filter_map(|value| value.as_str().map(str::to_string));
            let mut nullable = false;
            let mut selected = None;

            for value in types.by_ref() {
                if value == "null" {
                    nullable = true;
                } else if selected.is_none() {
                    selected = Some(value);
                }
            }

            (selected, nullable)
        }
        _ => (None, false),
    }
}

fn merge_union_variants(target: &mut Map<String, Value>, variants: Value) -> bool {
    let mut nullable = false;

    if let Value::Array(variants) = variants {
        for variant in variants {
            let sanitized = strip_unsupported_schema_fields(variant);
            match sanitized {
                Value::Object(map) => {
                    let is_null_only = map
                        .get("type")
                        .and_then(Value::as_str)
                        .map(|value| value == "null")
                        .unwrap_or(false)
                        && map.len() == 1;

                    if is_null_only {
                        nullable = true;
                        continue;
                    }

                    merge_schema_map(target, map, false);
                }
                Value::String(value) if value == "null" => nullable = true,
                _ => {}
            }
        }
    }

    nullable
}

fn merge_schema_variants(target: &mut Map<String, Value>, variants: Value, preserve_required: bool) {
    if let Value::Array(variants) = variants {
        for variant in variants {
            if let Value::Object(map) = strip_unsupported_schema_fields(variant) {
                merge_schema_map(target, map, preserve_required);
            }
        }
    }
}

fn merge_schema_map(target: &mut Map<String, Value>, source: Map<String, Value>, preserve_required: bool) {
    for (key, value) in source {
        match key.as_str() {
            "properties" => {
                if let Value::Object(source_props) = value {
                    let target_props = target.entry(key).or_insert_with(|| Value::Object(Map::new()));
                    if let Value::Object(target_props) = target_props {
                        for (prop_key, prop_value) in source_props {
                            target_props.entry(prop_key).or_insert(prop_value);
                        }
                    }
                }
            }
            "required" if preserve_required => {
                if let Value::Array(source_required) = value {
                    let target_required = target.entry(key).or_insert_with(|| Value::Array(Vec::new()));
                    if let Value::Array(target_required) = target_required {
                        for item in source_required {
                            if !target_required.contains(&item) {
                                target_required.push(item);
                            }
                        }
                    }
                }
            }
            "nullable" => {
                if value.as_bool().unwrap_or(false) {
                    target.insert(key, Value::Bool(true));
                }
            }
            "type" => {
                target.entry(key).or_insert(value);
            }
            _ => {
                target.entry(key).or_insert(value);
            }
        }
    }
}
