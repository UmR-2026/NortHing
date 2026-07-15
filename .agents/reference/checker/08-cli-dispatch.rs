// REFERENCE — copied from tools/plan-compliance-checker/src/main.rs
// Last synced: ec1902e (v3-restructure)
// Mirror only — NOT compiled. Original file lives in src/.
// If you change the source, re-run: node scripts/copy_reference.js

use clap::Parser;

pub mod plan;
pub mod path_resolver;
pub mod git_inspector;
pub mod command_runner;
pub mod task;
pub mod report;

#[derive(Parser)]
#[command(name = "plan-compliance-checker", about = "Verify workspace state against a plan markdown document", version)]
struct Cli {
    /// Path to the plan markdown file
    plan: std::path::PathBuf,

    /// Check only one task (e.g., "1.3")
    #[arg(long)]
    task: Option<String>,

    /// Skip long-running verify commands (cargo build, cargo test)
    #[arg(long)]
    skip_slow: bool,

    /// Force re-running verify commands even for slow ones
    #[arg(long)]
    force_verify: bool,

    /// Override the plan-start SHA (default: HEAD~1, meaning compare against the commit before HEAD)
    #[arg(long)]
    start_sha: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value_t = Format::Human)]
    format: Format,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Format { Human, Json }

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let plan_text = std::fs::read_to_string(&cli.plan)?;
    let mut plan = plan::parse_plan(&plan_text)?;
    plan.path = cli.plan.clone();

    let start_sha = cli.start_sha.unwrap_or_else(|| "HEAD~1".to_string());
    plan.plan_start_sha = start_sha.clone();

    let cwd = std::env::current_dir()?;
    let results = task::check_plan(&plan, &cwd)?;

    match cli.format {
        Format::Human => report::format_human(&plan, &results),
        Format::Json => println!("{}", report::format_json(&plan, &results)),
    }

    Ok(())
}
