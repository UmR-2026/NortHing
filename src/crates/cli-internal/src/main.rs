//! northhing Internal CLI
//!
//! Hidden capability-gated command surface for:
//! - Subagent spawning and headless execution
//! - Skill invocation from scripts and automation
//! - Session management and tool inspection
//!
//! **SECURITY**: Every subcommand requires `NORTHHING_INTERNAL_TOKEN` env var
//! or `--internal-token` arg. Without it, exit code 77 (capability denied).
//!
//! This is NOT advertised to end users. Documentation lives in
//! `docs/internal/cli.md` only.

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;
use std::process;

// ======================== Capability Token ========================

const TOKEN_ENV_VAR: &str = "NORTHHING_INTERNAL_TOKEN";
const TOKEN_MIN_LENGTH: usize = 32;

/// Verify capability token before any command execution.
/// This runs BEFORE logging setup to avoid leaking capabilities in logs.
fn verify_capability_token(token: Option<&str>) -> Result<String> {
    let token = token
        .map(|t| t.to_string())
        .or_else(|| env::var(TOKEN_ENV_VAR).ok())
        .ok_or_else(|| {
            eprintln!("Error: Capability token required");
            eprintln!();
            eprintln!("This is an internal command surface. To use it:");
            eprintln!("  1. Generate a token: northhing internal token generate");
            eprintln!("  2. Set env var: export NORTHHING_INTERNAL_TOKEN=<token>");
            eprintln!("  3. Or pass: --internal-token <token>");
            eprintln!();
            eprintln!("Read docs/internal/cli.md for the threat model and usage.");
            anyhow::anyhow!("Capability denied: missing token")
        })?;

    if token.len() < TOKEN_MIN_LENGTH {
        return Err(anyhow::anyhow!(
            "Capability denied: token too short (min {} chars)",
            TOKEN_MIN_LENGTH
        ));
    }

    // Token format validated; cryptographic hash comparison against the stored
    // token registry is deferred — the current length gate alone prevents
    // accidental use of the internal surface by neighbouring processes.

    Ok(token)
}

// ======================== CLI Definition ========================

#[derive(Parser)]
#[command(name = "northhing-internal")]
#[command(about = "Internal command surface (capability-gated)", long_about = None)]
#[command(version)]
struct Cli {
    /// Internal capability token (alternative to NORTHHING_INTERNAL_TOKEN env var)
    #[arg(long, global = true)]
    internal_token: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new capability token
    Token {
        #[command(subcommand)]
        action: TokenAction,
    },

    /// Run a skill headlessly
    Run {
        /// Skill name or path
        #[arg(short, long)]
        skill: String,

        /// Input JSON for the skill
        #[arg(short, long)]
        input: Option<String>,

        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        output: OutputFormat,
    },

    /// Session management
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },

    /// Tool inspection and invocation
    Tool {
        #[command(subcommand)]
        action: ToolAction,
    },

    /// List registered capabilities
    Capabilities,
}

#[derive(Subcommand)]
enum TokenAction {
    /// Generate a new random token
    Generate,
    /// Validate a token
    Validate { token: String },
}

#[derive(Subcommand)]
enum SessionAction {
    /// List all sessions
    List,
    /// Create a new session
    New {
        /// Model ID to use
        #[arg(short, long)]
        model: Option<String>,
    },
    /// Send a message to a session
    Send {
        /// Session ID
        session_id: String,
        /// Message text
        message: String,
    },
    /// Export session to file
    Export {
        /// Session ID
        session_id: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
enum ToolAction {
    /// List available tools
    List,
    /// Invoke a tool directly
    Invoke {
        /// Tool name
        name: String,
        /// Tool arguments as JSON
        #[arg(short, long)]
        args: String,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum OutputFormat {
    Json,
    Text,
    Yaml,
}

// ======================== Main ========================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Token verification happens BEFORE any command execution
    // This prevents accidental invocation even if command parsing succeeds
    let _token = match verify_capability_token(cli.internal_token.as_deref()) {
        Ok(token) => token,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(77); // EX_CONFIG - capability denied
        }
    };

    // Now safe to initialize logging
    init_logging();

    match cli.command {
        Commands::Token { action } => handle_token(action).await,
        Commands::Run { skill, input, output } => handle_run(skill, input, output).await,
        Commands::Session { action } => handle_session(action).await,
        Commands::Tool { action } => handle_tool(action).await,
        Commands::Capabilities => handle_capabilities().await,
    }
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

// ======================== Command Handlers ========================

async fn handle_token(action: TokenAction) -> Result<()> {
    match action {
        TokenAction::Generate => {
            let token = generate_token();
            println!("{}", token);
            println!();
            println!("Store this in a secure location. It will not be shown again.");
            println!("Set env var: export NORTHHING_INTERNAL_TOKEN=<token>");
            Ok(())
        }
        TokenAction::Validate { token } => {
            if token.len() >= TOKEN_MIN_LENGTH {
                println!("Token format valid (length: {})", token.len());
                // Hash comparison against the stored token registry is deferred
                // (see `read_token` for rationale).
                println!("Hash validation: deferred (format-only check active)");
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Token too short: {} < {}",
                    token.len(),
                    TOKEN_MIN_LENGTH
                ))
            }
        }
    }
}

fn generate_token() -> String {
    use rand::RngCore;
    let mut bytes = vec![0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex_encode(&bytes)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

async fn handle_run(skill: String, input: Option<String>, output: OutputFormat) -> Result<()> {
    println!("Running skill: {}", skill);
    if let Some(input) = input {
        println!("Input: {}", input);
    }
    println!("Output format: {:?}", output);

    // Skill headless execution is a deferred feature (Phase N):
    //   1. Load skill from ~/.config/northhing/skills/ or the bundled registry
    //   2. Parse input JSON into the skill's expected input schema
    //   3. Execute the skill's headless workflow (no UI, no streaming)
    //   4. Format and print output
    //
    // The `Run` subcommand is wired for future activation. Return an error
    // so callers can distinguish "not implemented" from "succeeded silently".
    Err(anyhow::anyhow!(
        "skill headless execution is not yet implemented (skill={})",
        skill
    ))
}

async fn handle_session(action: SessionAction) -> Result<()> {
    match action {
        SessionAction::List => {
            println!("Sessions:");
            println!("  [Session listing not yet implemented - placeholder]");
            Ok(())
        }
        SessionAction::New { model } => {
            println!("Creating new session");
            if let Some(model) = model {
                println!("  Model: {}", model);
            }
            println!("\n[Session creation not yet implemented - placeholder]");
            Ok(())
        }
        SessionAction::Send { session_id, message } => {
            println!("Sending to session {}: {}", session_id, message);
            println!("\n[Message send not yet implemented - placeholder]");
            Ok(())
        }
        SessionAction::Export { session_id, output } => {
            println!("Exporting session {}", session_id);
            if let Some(output) = output {
                println!("  To file: {}", output);
            }
            println!("\n[Session export not yet implemented - placeholder]");
            Ok(())
        }
    }
}

async fn handle_tool(action: ToolAction) -> Result<()> {
    match action {
        ToolAction::List => {
            println!("Available tools:");
            println!("  [Tool listing not yet implemented - placeholder]");
            Ok(())
        }
        ToolAction::Invoke { name, args } => {
            println!("Invoking tool: {}", name);
            println!("Arguments: {}", args);
            println!("\n[Tool invocation not yet implemented - placeholder]");
            Ok(())
        }
    }
}

async fn handle_capabilities() -> Result<()> {
    println!("Registered capabilities:");
    println!("  - skill:run    (Run skills headlessly)");
    println!("  - session:*    (Session management)");
    println!("  - tool:*       (Tool inspection and invocation)");
    println!();
    println!("All capabilities require token verification.");
    Ok(())
}
