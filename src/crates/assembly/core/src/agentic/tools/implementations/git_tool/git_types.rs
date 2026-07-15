use serde_json::{json, Map, Value};

const GIT_DIFF_FILE_SEPARATOR: &str = " -- ";
const RANGE_TWO_DOT: &str = "..";
const RANGE_THREE_DOT: &str = "...";
const DIFF_FLAGS: &[&str] = &["--staged", "--cached", "--stat"];
const SHORT_FLAG_PREFIX: &str = "-";
pub(crate) const ALLOWED_OPERATIONS: &[&str] = &[
    "status",
    "diff",
    "log",
    "add",
    "commit",
    "branch",
    "checkout",
    "switch",
    "pull",
    "push",
    "fetch",
    "merge",
    "rebase",
    "stash",
    "reset",
    "restore",
    "show",
    "tag",
    "remote",
    "clone",
    "init",
    "blame",
    "cherry-pick",
    "rev-parse",
    "describe",
    "shortlog",
    "clean",
];
const DANGEROUS_OPERATIONS: &[&str] = &["push --force", "reset --hard", "clean -fd", "rebase"];

#[derive(Debug, PartialEq, Default)]
pub(crate) struct ParsedDiffArgs {
    pub(crate) staged: bool,
    pub(crate) stat: bool,
    pub(crate) source: Option<String>,
    pub(crate) target: Option<String>,
    pub(crate) files: Option<Vec<String>>,
}

fn strip_command_wrapping(raw: &str) -> &str {
    let trimmed = raw.trim();
    let Some(stripped) = trimmed.strip_prefix("```").and_then(|value| value.strip_suffix("```")) else {
        return trimmed.trim_matches('`').trim();
    };

    let stripped = stripped.trim();
    if let Some((first_line, rest)) = stripped.split_once('\n') {
        if first_line
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        {
            return rest.trim();
        }
    }

    stripped
}

fn parse_git_command_text(text: &str) -> Option<Value> {
    let trimmed = strip_command_wrapping(text);
    let command = trimmed.strip_prefix("git ").map(str::trim).unwrap_or(trimmed);
    let mut parts = command.splitn(2, char::is_whitespace);
    let operation = parts.next()?.trim();
    if operation.is_empty()
        || !operation
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return None;
    }

    let args = parts.next().map(str::trim).filter(|args| !args.is_empty());
    let mut value = json!({ "operation": operation });
    if let Some(args) = args {
        value["args"] = json!(args);
    }
    Some(value)
}

fn split_leading_operation(args: &str) -> Option<(String, String)> {
    let args = args.trim().strip_prefix("git ").map(str::trim).unwrap_or(args.trim());
    let mut parts = args.splitn(2, char::is_whitespace);
    let operation = parts.next()?.trim();
    if !ALLOWED_OPERATIONS.contains(&operation) {
        return None;
    }

    let rest = parts.next().unwrap_or("").trim().to_string();
    Some((operation.to_string(), rest))
}

fn infer_operation_from_flag_args(args: &str) -> Option<&'static str> {
    let tokens: Vec<&str> = args.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    let has_log_flag = tokens.iter().any(|token| {
        matches!(
            *token,
            "--since" | "--until" | "--oneline" | "--grep" | "--author" | "--decorate" | "--walk-reflogs",
        ) || token.starts_with("--since=")
            || token.starts_with("--until=")
    });
    if has_log_flag {
        return Some("log");
    }

    let has_diff_flag = tokens.iter().any(|token| {
        matches!(
            *token,
            "--staged" | "--cached" | "--stat" | "--numstat" | "--name-only" | "--name-status"
        )
    });
    if has_diff_flag {
        return Some("diff");
    }

    None
}

fn preserve_git_input_metadata(parsed: &mut Value, source: &Map<String, Value>) {
    let Some(parsed_obj) = parsed.as_object_mut() else {
        return;
    };
    for key in ["working_directory", "timeout"] {
        if let Some(value) = source.get(key) {
            parsed_obj.entry(key.to_string()).or_insert_with(|| value.clone());
        }
    }
}

pub(crate) fn normalize_git_input(input: Value) -> Value {
    if let Some(text) = input.as_str() {
        return parse_git_command_text(text).unwrap_or(input);
    }

    let Some(source) = input.as_object() else {
        return input;
    };

    if source
        .get("operation")
        .and_then(|value| value.as_str())
        .is_some_and(|operation| !operation.is_empty())
    {
        return input;
    }

    for key in ["command", "cmd"] {
        if let Some(text) = source.get(key).and_then(|value| value.as_str()) {
            if let Some(mut parsed) = parse_git_command_text(text) {
                preserve_git_input_metadata(&mut parsed, source);
                return parsed;
            }
        }
    }

    if let Some(args) = source.get("args").and_then(|value| value.as_str()) {
        if let Some((operation, rest)) = split_leading_operation(args) {
            let mut parsed = json!({ "operation": operation });
            if !rest.is_empty() {
                parsed["args"] = json!(rest);
            }
            preserve_git_input_metadata(&mut parsed, source);
            return parsed;
        }

        if let Some(operation) = infer_operation_from_flag_args(args) {
            let mut parsed = json!({
                "operation": operation,
                "args": args.trim(),
            });
            preserve_git_input_metadata(&mut parsed, source);
            return parsed;
        }
    }

    input
}

pub(crate) fn is_dangerous_operation(operation: &str, args: &str) -> bool {
    let full_cmd = format!("{} {}", operation, args);
    DANGEROUS_OPERATIONS.iter().any(|&danger| full_cmd.contains(danger))
}

pub(crate) fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub(crate) fn parse_diff_args(args_str: &str) -> ParsedDiffArgs {
    let mut result = ParsedDiffArgs {
        staged: args_str.contains("--staged") || args_str.contains("--cached"),
        stat: args_str.contains("--stat"),
        ..Default::default()
    };

    let (refs_part, files_part) = if let Some(sep_pos) = args_str.find(GIT_DIFF_FILE_SEPARATOR) {
        let refs = args_str[..sep_pos].trim();
        let files = args_str[sep_pos + GIT_DIFF_FILE_SEPARATOR.len()..].trim();
        (refs, Some(files))
    } else if let Some(stripped) = args_str.strip_prefix("-- ") {
        ("", Some(stripped.trim()))
    } else {
        (args_str.trim(), None)
    };

    let ref_tokens: Vec<&str> = refs_part
        .split_whitespace()
        .filter(|token| !DIFF_FLAGS.iter().any(|flag| token == flag) && !token.starts_with(SHORT_FLAG_PREFIX))
        .collect();

    let refs_text = if ref_tokens.len() == 1 {
        ref_tokens[0]
    } else if ref_tokens.len() >= 2 {
        &ref_tokens.join(" ")
    } else {
        ""
    };

    if !refs_text.is_empty() {
        let (src, tgt) = split_range(refs_text);
        result.source = src;
        result.target = tgt;
    }

    result.files = if result.source.is_none() || result.target.is_some() {
        files_part.map(|fp| fp.split_whitespace().map(|s| s.to_string()).collect::<Vec<String>>())
    } else {
        None
    };

    result
}

fn split_range(text: &str) -> (Option<String>, Option<String>) {
    let (sep_len, pos) = if let Some(p) = text.find(RANGE_THREE_DOT) {
        (RANGE_THREE_DOT.len(), p)
    } else if let Some(p) = text.find(RANGE_TWO_DOT) {
        (RANGE_TWO_DOT.len(), p)
    } else {
        return (Some(text.to_string()), None);
    };

    let src = text[..pos].trim();
    let tgt = text[pos + sep_len..].trim();

    match (src.is_empty(), tgt.is_empty()) {
        (false, false) => (Some(src.to_string()), Some(tgt.to_string())),
        (false, true) => (Some(src.to_string()), None),
        (true, false) => (None, Some(tgt.to_string())),
        (true, true) => (None, None),
    }
}

pub(crate) fn git_operation_needs_light_checkpoint(operation: &str, args: Option<&str>) -> bool {
    match operation {
        "add" | "commit" | "pull" | "checkout" | "switch" | "merge" | "rebase" | "stash" | "reset" | "restore"
        | "clean" | "cherry-pick" => true,
        "branch" => args.is_some_and(|value| !value.trim().is_empty()),
        _ => false,
    }
}
