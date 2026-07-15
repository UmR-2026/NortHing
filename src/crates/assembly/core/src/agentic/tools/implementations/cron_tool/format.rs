use crate::service::cron::CronJob;

pub fn escape_markdown_table_cell(value: &str) -> String {
    value.replace('\\', "\\\\").replace('|', "\\|").replace('\n', "<br>")
}

pub fn build_list_result_for_assistant(workspace: &str, session_id: &str, jobs: &[CronJob]) -> String {
    if jobs.is_empty() {
        return format!(
            "No scheduled jobs found for session '{}' in workspace '{}'.",
            session_id, workspace
        );
    }

    let mut lines = vec![format!(
        "Found {} scheduled job(s) for session '{}' in workspace '{}'.",
        jobs.len(),
        session_id,
        workspace,
    )];
    lines.push(String::new());
    lines.push("| job_id | name | enabled | schedule |".to_string());
    lines.push("| --- | --- | --- | --- |".to_string());
    for job in jobs {
        lines.push(format!(
            "| {} | {} | {} | {} |",
            escape_markdown_table_cell(&job.id),
            escape_markdown_table_cell(&job.name),
            if job.enabled { "true" } else { "false" },
            escape_markdown_table_cell(&super::schedule::schedule_summary(&job.schedule)),
        ));
    }
    lines.join("\n")
}
