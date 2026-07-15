#[path = "edit_apply.rs"]
mod edit_apply;
#[path = "edit_preview.rs"]
mod edit_preview;
#[path = "edit_types.rs"]
mod edit_types;
#[path = "edit_validate.rs"]
mod edit_validate;

// Public API re-exported for cross-crate consumers.
pub use edit_apply::{apply_edit_to_content, edit_file, edit_local_file, edit_local_file_with_content};
pub use edit_preview::sanitize_read_tool_copied_text;
pub use edit_types::{
    ApplyEditResult, EditLocalFileOutcome, EditLocalFileRequest, EditLocalFileWithContentRequest, EditResult,
};

#[cfg(test)]
mod tests {
    use super::apply_edit_to_content;
    use super::edit_file;
    use super::sanitize_read_tool_copied_text;
    use super::EditResult;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp_file(contents: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("northhing-edit-file-test-{unique}.txt"));
        fs::write(&path, contents).expect("temp file should be written");
        path
    }

    #[test]
    fn sanitize_read_tool_copied_text_strips_cat_n_prefixes() {
        let sanitized =
            sanitize_read_tool_copied_text("     1\talpha\n     2\tbeta").expect("read prefixes should be stripped");

        assert_eq!(sanitized, "alpha\nbeta");
    }

    #[test]
    fn sanitize_read_tool_copied_text_allows_mixed_lines() {
        let sanitized =
            sanitize_read_tool_copied_text("     1\talpha\nplain").expect("partial prefixes should still be stripped");

        assert_eq!(sanitized, "alpha\nplain");
    }

    #[test]
    fn apply_edit_to_content_matches_curly_quotes() {
        let content = "msg := \u{201c}hello\u{201d}\n";
        let result = apply_edit_to_content(content, "msg := \"hello\"", "msg := \"hi\"", false)
            .expect("quote-normalized edit should succeed");

        assert_eq!(result.new_content, "msg := \"hi\"\n");
    }

    #[test]
    fn apply_edit_to_content_matches_multiline_lf_input_against_crlf_file() {
        let content = "header\r\nalpha\r\nbeta\r\nfooter\r\n";
        let result = apply_edit_to_content(content, "alpha\nbeta", "alpha\nBETA", false).expect("edit should succeed");

        assert_eq!(result.match_count, 1);
        assert_eq!(
            result.edit_result,
            EditResult {
                start_line: 2,
                old_end_line: 3,
                new_end_line: 3,
            }
        );
        assert_eq!(result.new_content, "header\r\nalpha\r\nBETA\r\nfooter\r\n");
    }

    #[test]
    fn apply_edit_to_content_accepts_read_tool_line_prefixes() {
        let content = "alpha\nbeta\n";
        let result = apply_edit_to_content(content, "     1\talpha\n     2\tbeta", "alpha\nBETA", false)
            .expect("edit should succeed with read prefixes");

        assert_eq!(result.new_content, "alpha\nBETA\n");
    }

    #[test]
    fn apply_edit_to_content_replace_all_reports_match_count() {
        let result =
            apply_edit_to_content("one\r\ntwo\r\none\r\n", "one", "ONE", true).expect("replace_all should succeed");

        assert_eq!(result.match_count, 2);
        assert_eq!(result.new_content, "ONE\r\ntwo\r\nONE\r\n");
        assert_eq!(result.edit_result.start_line, 1);
    }

    #[test]
    fn apply_edit_to_content_rejects_empty_old_string() {
        let error = apply_edit_to_content("alpha\n", "", "beta", false).expect_err("empty old_string should fail");

        assert_eq!(error, "old_string cannot be empty.");
    }

    #[test]
    fn apply_edit_to_content_multiple_match_error_includes_contexts() {
        let error = apply_edit_to_content(
            "first block\n  same();\nend first\n\nsecond block\n  same();\nend second\n",
            "  same();",
            "  changed();",
            false,
        )
        .expect_err("ambiguous edit should fail");

        assert!(error.contains("`old_string` appears 2 times in file"));
        assert!(error.contains("[match 1 starts at line 2]"));
        assert!(error.contains("first block"));
        assert!(error.contains("[match 2 starts at line 6]"));
        assert!(error.contains("second block"));
    }

    #[test]
    fn apply_edit_to_content_not_found_includes_nearby_diagnostics() {
        let error = apply_edit_to_content(
            "fn main() {\n    println!(\"hello\");\n}\n",
            "println!(\"goodbye\");",
            "println!(\"hi\");",
            false,
        )
        .expect_err("missing text should fail");

        assert!(error.contains("old_string not found in file."));
        assert!(error.contains("[nearby content around line 2]"));
        assert!(error.contains("println!(\"hello\");"));
    }

    #[test]
    fn apply_edit_to_content_not_found_calls_out_read_prefixes() {
        let error = apply_edit_to_content("alpha\nbeta\n", "     1\talpha\n     2\tgamma", "alpha\nBETA", false)
            .expect_err("missing text should fail");

        assert!(error.contains("Read-tool line-number prefixes"));
    }

    #[test]
    fn edit_file_preserves_crlf_when_editing_with_lf_old_string() {
        let path = write_temp_file("first\r\nalpha\r\nbeta\r\n");

        let result = edit_file(path.to_str().expect("utf-8 path"), "alpha\nbeta", "alpha\nBETA", false)
            .expect("edit should succeed");
        let content = fs::read_to_string(&path).expect("edited file should be readable");

        fs::remove_file(&path).expect("temp file should be deleted");

        assert_eq!(
            result,
            EditResult {
                start_line: 2,
                old_end_line: 3,
                new_end_line: 3,
            }
        );
        assert_eq!(content, "first\r\nalpha\r\nBETA\r\n");
    }

    // -- whitespace-normalization candidate tests ----------------------------------

    #[test]
    fn apply_edit_tabs_old_matches_spaces_file_2w() {
        // Model copies with tabs; file uses 2-space indentation.
        let content = "fn main() {\n  let x = 1;\n  let y = 2;\n}\n";
        let result = apply_edit_to_content(
            content,
            "fn main() {\n\tlet x = 1;\n\tlet y = 2;\n}",
            "fn main() {\n\tlet x = 0;\n\tlet y = 0;\n}",
            false,
        )
        .expect("tabs→2-space edit should succeed");
        assert_eq!(result.new_content, "fn main() {\n  let x = 0;\n  let y = 0;\n}\n");
    }

    #[test]
    fn apply_edit_tabs_old_matches_spaces_file_4w() {
        // Model copies with tabs; file uses 4-space indentation.
        let content = "fn main() {\n    let x = 1;\n}\n";
        let result = apply_edit_to_content(
            content,
            "fn main() {\n\tlet x = 1;\n}",
            "fn main() {\n\tlet x = 0;\n}",
            false,
        )
        .expect("tabs→4-space edit should succeed");
        assert_eq!(result.new_content, "fn main() {\n    let x = 0;\n}\n");
    }

    #[test]
    fn apply_edit_spaces_old_matches_tabs_file_4w() {
        // Model copies with 4-space indentation; file uses tabs.
        let content = "fn main() {\n\tlet x = 1;\n}\n";
        let result = apply_edit_to_content(
            content,
            "fn main() {\n    let x = 1;\n}",
            "fn main() {\n    let x = 0;\n}",
            false,
        )
        .expect("4-space→tabs edit should succeed");
        assert_eq!(result.new_content, "fn main() {\n\tlet x = 0;\n}\n");
    }

    #[test]
    fn apply_edit_spaces_old_matches_tabs_file_2w() {
        // Model copies with 2-space indentation; file uses tabs.
        let content = "fn main() {\n\tlet x = 1;\n}\n";
        let result = apply_edit_to_content(
            content,
            "fn main() {\n  let x = 1;\n}",
            "fn main() {\n  let x = 0;\n}",
            false,
        )
        .expect("2-space→tabs edit should succeed");
        assert_eq!(result.new_content, "fn main() {\n\tlet x = 0;\n}\n");
    }

    #[test]
    fn apply_edit_whitespace_candidate_does_not_match_when_content_differs() {
        // Tabs→spaces conversion should NOT produce a false match when the
        // non-whitespace portion of the content differs.
        let content = "fn main() {\n    let x = 1;\n}\n";
        let error = apply_edit_to_content(
            content,
            "fn main() {\n\tlet x = 999;\n}",
            "fn main() {\n\tlet x = 0;\n}",
            false,
        )
        .expect_err("different content should fail");
        assert!(error.contains("old_string not found in file."));
    }

    #[test]
    fn apply_edit_whitespace_candidate_preserves_crlf() {
        // Whitespace-normalized edit on a CRLF file must preserve CRLF.
        let content = "fn main() {\r\n    let x = 1;\r\n}\r\n";
        let result = apply_edit_to_content(
            content,
            "fn main() {\n\tlet x = 1;\n}",
            "fn main() {\n\tlet x = 0;\n}",
            false,
        )
        .expect("whitespace-normalized CRLF edit should succeed");
        assert_eq!(result.new_content, "fn main() {\r\n    let x = 0;\r\n}\r\n");
        assert_eq!(result.match_count, 1);
    }

    #[test]
    fn apply_edit_curly_quotes_with_crlf_file() {
        // find_actual_string must work on CRLF files after the fix.
        let content = "msg := \u{201c}hello\u{201d}\r\n";
        let result = apply_edit_to_content(content, "msg := \"hello\"", "msg := \"hi\"", false)
            .expect("curly-quote edit on CRLF file should succeed");
        assert_eq!(result.new_content, "msg := \"hi\"\r\n");
    }

    #[test]
    fn apply_edit_curly_quotes_with_crlf_file_and_whitespace_mismatch() {
        // Combined: CRLF file, curly quotes, AND tab↔space mismatch.
        let content = "fn main() {\r\n    msg := \u{201c}hello\u{201d}\r\n}\r\n";
        let result = apply_edit_to_content(
            content,
            "fn main() {\n\tmsg := \"hello\"\n}",
            "fn main() {\n\tmsg := \"hi\"\n}",
            false,
        )
        .expect("combined CRLF + curly + tab→space edit should succeed");
        assert_eq!(result.new_content, "fn main() {\r\n    msg := \"hi\"\r\n}\r\n");
    }
}
