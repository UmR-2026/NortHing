use std::fs;

use crate::util::string::normalize_string;

use super::edit_types::{ApplyEditResult, EditLocalFileOutcome, EditResult};
use super::edit_validate::{build_not_found_diagnostics, count_lines_before, count_newlines, match_contexts};

/// Core match-and-replace logic.  `normalized_content` and `uses_crlf` are
/// pre-computed by the caller so they are not re-derived per candidate.
fn apply_match_and_replace(
    normalized_content: &str,
    uses_crlf: bool,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<ApplyEditResult, String> {
    let normalized_old = normalize_string(old_string);
    let normalized_new = normalize_string(new_string);

    if normalized_old.is_empty() {
        return Err("old_string cannot be empty.".to_string());
    }

    let matches: Vec<_> = normalized_content.match_indices(&normalized_old).collect();

    if matches.is_empty() {
        return Err("old_string not found in file.".to_string());
    }

    if matches.len() > 1 && !replace_all {
        return Err(format!(
            "`old_string` appears {} times in file, either provide a larger string with more surrounding context to make it unique or use `replace_all` to change every instance of `old_string`.\n{}",
            matches.len(),
            match_contexts(normalized_content, &normalized_old, &matches)
        ));
    }

    let first_match_pos = matches[0].0;
    let start_line = count_lines_before(normalized_content, first_match_pos);
    let old_end_line = start_line + count_newlines(&normalized_old);
    let new_end_line = start_line + count_newlines(&normalized_new);

    let new_normalized_content = if replace_all {
        normalized_content.replace(&normalized_old, &normalized_new)
    } else {
        normalized_content.replacen(&normalized_old, &normalized_new, 1)
    };

    let new_content = if uses_crlf {
        new_normalized_content.replace("\n", "\r\n")
    } else {
        new_normalized_content
    };

    Ok(ApplyEditResult {
        new_content,
        match_count: matches.len(),
        edit_result: EditResult {
            start_line,
            old_end_line,
            new_end_line,
        },
    })
}

pub fn apply_edit_to_content(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<ApplyEditResult, String> {
    let mut last_error = String::from("old_string not found in file.");

    // Pre-compute so every candidate iteration reuses the same normalized form.
    let uses_crlf = content.contains("\r\n");
    let normalized_content = normalize_string(content);

    for (candidate_old, candidate_new) in super::edit_preview::edit_string_candidates(content, old_string, new_string) {
        match apply_match_and_replace(
            &normalized_content,
            uses_crlf,
            &candidate_old,
            &candidate_new,
            replace_all,
        ) {
            Ok(result) => return Ok(result),
            Err(error) if error == "old_string not found in file." => {
                last_error = error;
            }
            Err(error) => return Err(error),
        }
    }

    Err(format!(
        "{}\n{}",
        last_error,
        build_not_found_diagnostics(content, old_string)
    ))
}

pub fn edit_file(file_path: &str, old_string: &str, new_string: &str, replace_all: bool) -> Result<EditResult, String> {
    let content = fs::read_to_string(file_path).map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;
    let result = apply_edit_to_content(&content, old_string, new_string, replace_all)?;

    fs::write(file_path, &result.new_content).map_err(|e| format!("Failed to write file {}: {}", file_path, e))?;

    Ok(result.edit_result)
}

pub fn edit_local_file(request: super::edit_types::EditLocalFileRequest) -> Result<EditLocalFileOutcome, String> {
    let content = fs::read_to_string(&request.resolved_path)
        .map_err(|error| format!("Failed to read file {}: {}", request.logical_path, error))?;
    edit_local_file_with_content(super::edit_types::EditLocalFileWithContentRequest {
        logical_path: request.logical_path,
        resolved_path: request.resolved_path,
        current_content: content,
        old_string: request.old_string,
        new_string: request.new_string,
        replace_all: request.replace_all,
    })
}

pub fn edit_local_file_with_content(
    request: super::edit_types::EditLocalFileWithContentRequest,
) -> Result<EditLocalFileOutcome, String> {
    let result = apply_edit_to_content(
        &request.current_content,
        &request.old_string,
        &request.new_string,
        request.replace_all,
    )?;

    fs::write(&request.resolved_path, result.new_content.as_bytes())
        .map_err(|error| format!("Failed to read file {}: {}", request.logical_path, error))?;

    Ok(EditLocalFileOutcome {
        new_content: result.new_content,
        match_count: result.match_count,
        edit_result: result.edit_result,
    })
}
