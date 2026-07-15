#![allow(clippy::too_many_arguments)]
pub mod fs;
pub mod pipeline;
pub mod search;
pub mod shell;
pub mod util;

// Re-export common stable types so cross-crate callers do not need deep
// module paths. Each group preserves the original module ownership; only
// the public surface is flattened here.
pub use fs::{
    build_remote_list_commands, delete_local_path, edit_local_file, write_local_file, DeleteLocalPathRequest,
    EditLocalFileRequest, FileSystem, LocalFileSystem, RemoteListCommandPlan, RemoteListEntry, WriteLocalFileRequest,
};
pub use pipeline::{
    count_tool_states, partition_tool_batches, should_retry_tool_attempt, ToolBatch, ToolExecutionErrorClass,
    ToolRetryAttemptFacts, ToolStateCounts, ToolTaskStateKind, ToolTurnCancellationSummary,
};
pub use search::{
    grep_search, GrepOptions, LocalGlobRequest, LocalGlobResult, OutputMode, ProgressCallback, RemoteGrepCommandRequest,
};
pub use shell::{
    banned_shell_command, bash_noninteractive_env, command_for_working_directory, render_local_shell_result,
    render_remote_shell_result, truncate_output_preserving_tail, LocalShellResultRenderRequest,
    RemoteShellResultRenderRequest,
};
pub use util::ansi_cleaner::{strip_ansi, strip_ansi_bytes, AnsiCleaner};
pub use util::read_line_prefix::{
    all_lines_have_read_prefix, read_tool_output_to_file_content, strip_read_line_number_prefix,
};
pub use util::string::{normalize_string, shell_single_quote, truncate_string_by_chars};
