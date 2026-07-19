//! W4 diagnostic repro: dual-runtime turn dispatch (D2i hypothesis).
//!
//! Mimics the desktop wiring and controlled variations. The core finding:
//! dispatching the turn on a PERSISTENT multi_thread runtime does NOT hang;
//! the desktop hangs because it dispatches on a THROWAWAY current_thread
//! runtime (runtime C, see `callbacks_lifecycle.rs:118-123`) that is DROPPED
//! as soon as `scheduler.submit()` returns — aborting the turn task that was
//! spawned (via `tokio::spawn` inside `dispatch_turn`, `sub_handle_out.rs:228`)
//! onto that same runtime.
//!
//! Modes:
//!   --mode=dual    runtime A (worker) inits+blocks; runtime B (persistent
//!                  multi_thread) dispatches. Does NOT hang (control).
//!   --mode=same    init + dispatch on one runtime. Does NOT hang (control).
//!   --mode=desktop runtime A inits+blocks; dispatch on a THROWAWAY
//!                  current_thread runtime C in a spawned thread, dropped
//!                  after submit() — exactly like callbacks_lifecycle.rs.
//!                  This is the mode that reproduces the hang.

use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use northhing_core::agentic::coordination::{
    global_coordinator, global_scheduler, set_global_scheduler, DialogScheduler,
    DialogSubmissionPolicy, DialogSubmitOutcome, DialogTriggerSource,
};
use northhing_core::agentic::core::{SessionConfig, SessionState};
use northhing_core::agentic::system::init_agentic_system;
use northhing_core::infrastructure::ai::AIClientFactory;
use northhing_core::service::config::initialize_global_config;

const TURN_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone, Copy)]
enum Mode {
    Dual,
    Same,
    Desktop,
}

fn parse_mode() -> Mode {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mode=desktop") {
        Mode::Desktop
    } else if args.iter().any(|a| a == "--mode=same") || args.iter().any(|a| a == "--same-runtime") {
        Mode::Same
    } else {
        Mode::Dual
    }
}

/// Replicates `init_agentic_system_for_desktop()` from
/// `src/apps/desktop/src/agent/agentic_system.rs` using only public
/// northhing-core APIs: global config service, global AIClientFactory,
/// agentic system (global coordinator), global dialog scheduler.
async fn init_core() -> Result<()> {
    initialize_global_config()
        .await
        .context("initialize_global_config failed")?;
    println!("W4-P: init_core: global config service initialized");

    AIClientFactory::initialize_global()
        .await
        .context("AIClientFactory::initialize_global failed")?;
    println!("W4-P: init_core: global AIClientFactory initialized");

    let system = init_agentic_system()
        .await
        .context("init_agentic_system failed")?;
    println!("W4-P: init_core: agentic system initialized");

    let coordinator = system.coordinator.clone();
    let session_manager = coordinator.session_manager().clone();
    let scheduler = DialogScheduler::new(coordinator.clone(), session_manager);
    let notifier_ok = coordinator.set_scheduler_notifier(scheduler.outcome_sender());
    let injection_ok =
        coordinator.set_round_injection_source(scheduler.round_injection_monitor());
    if !notifier_ok || !injection_ok {
        anyhow::bail!(
            "scheduler wiring conflict: notifier={notifier_ok}, injection={injection_ok}"
        );
    }
    set_global_scheduler(scheduler);
    println!("W4-P: init_core: global scheduler initialized");
    Ok(())
}

/// Create a session and submit a turn; return (session_id, submit outcome).
async fn create_session_and_submit() -> Result<(String, DialogSubmitOutcome)> {
    let scheduler = global_scheduler().ok_or_else(|| anyhow::anyhow!("no global scheduler"))?;
    let coordinator = global_coordinator().ok_or_else(|| anyhow::anyhow!("no global coordinator"))?;
    let workspace = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());
    let config = SessionConfig {
        workspace_path: Some(workspace),
        ..Default::default()
    };
    let session = coordinator
        .create_session("W4 repro session".to_string(), "agentic".to_string(), config)
        .await
        .context("create_session failed")?;
    let sid = session.session_id.clone();
    println!("W4-P: create_session_and_submit: created session sid={sid}");
    let outcome = scheduler
        .submit(
            sid.clone(),
            "hi".to_string(),
            None,
            None,
            "agentic".to_string(),
            None,
            DialogSubmissionPolicy::for_source(DialogTriggerSource::DesktopApi),
            None,
            None,
            None,
        )
        .await
        .map_err(|e| anyhow::anyhow!("scheduler.submit failed: {e}"))?;
    Ok((sid, outcome))
}

/// Dispatch on a persistent runtime and poll until the turn leaves Processing.
async fn dispatch_turn(input: &str) -> Result<String> {
    let (sid, outcome) = create_session_and_submit().await?;
    let turn_id = match &outcome {
        DialogSubmitOutcome::Started { turn_id, .. } => turn_id.clone(),
        DialogSubmitOutcome::Queued { turn_id, .. } => turn_id.clone(),
    };
    println!("W4-P: dispatch_turn: submitted turn turn_id={turn_id}, polling for completion");

    let coordinator = global_coordinator().ok_or_else(|| anyhow::anyhow!("no global coordinator"))?;
    let sm = coordinator.session_manager().clone();
    let poll = async {
        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let state = sm.get_session(&sid).map(|s| s.state.clone());
            match state {
                Some(SessionState::Idle) => {
                    println!("W4-P: dispatch_turn: session returned to Idle (turn done)");
                    break;
                }
                Some(SessionState::Error { error, .. }) => {
                    println!("W4-P: dispatch_turn: session entered Error: {error}");
                    break;
                }
                Some(SessionState::Processing { phase, .. }) => {
                    println!("W4-P: dispatch_turn: still Processing phase={:?}", phase);
                }
                None => {
                    println!("W4-P: dispatch_turn: session not found");
                    break;
                }
            }
        }
    };

    tokio::time::timeout(TURN_TIMEOUT, poll)
        .await
        .map_err(|_| anyhow::anyhow!("TURN HUNG: did not complete within {:?}", TURN_TIMEOUT))?;

    Ok(turn_id)
}

/// Dual-runtime path: runtime A (worker) inits+blocks; runtime B (persistent
/// multi_thread) dispatches. CONTROL: does NOT hang.
async fn run_dual_runtime() -> Result<()> {
    println!("W4-P: run_dual_runtime: starting (mimics desktop D2i wiring)");
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    let worker = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build worker runtime");
            if let Err(e) = rt.block_on(init_core()) {
                eprintln!("W4-P: run_dual_runtime: init_core failed on runtime A: {e}");
                std::process::exit(1);
            }
            println!("W4-P: run_dual_runtime: runtime A init done, blocking on shutdown_rx");
            let _ = shutdown_rx.recv();
        })
        .expect("failed to spawn worker");

    tokio::time::sleep(Duration::from_secs(2)).await;
    println!("W4-P: run_dual_runtime: runtime B dispatching turn");
    let result = dispatch_turn("hi").await;
    match &result {
        Ok(tid) => println!("W4-P: run_dual_runtime: turn completed turn_id={tid}"),
        Err(e) => println!("W4-P: run_dual_runtime: {e}"),
    }
    let _ = shutdown_tx.send(());
    worker.join().expect("worker thread panicked");
    result.map(|_| ())
}

/// Same-runtime control: init + dispatch on one runtime. Does NOT hang.
async fn run_same_runtime() -> Result<()> {
    println!("W4-P: run_same_runtime: starting (init + dispatch on one runtime)");
    init_core().await?;
    println!("W4-P: run_same_runtime: init done on runtime B, dispatching turn");
    dispatch_turn("hi").await.map(|_| ())
}

/// Desktop-faithful path. Mimics `callbacks_lifecycle.rs:118-123` exactly:
/// a spawned thread creates a throwaway current_thread runtime (runtime C),
/// runs `scheduler.submit()` on it, and the runtime is DROPPED when the
/// closure ends. The turn task spawned inside `dispatch_turn`
/// (`sub_handle_out.rs:228`, `tokio::spawn`) lands on runtime C, so dropping
/// runtime C aborts it — leaving the session stuck in Processing forever.
fn run_desktop_faithful() -> Result<Option<String>> {
    println!("W4-P: run_desktop_faithful: starting (exact callbacks_lifecycle.rs wiring)");
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    let worker = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("failed to build worker runtime");
            if let Err(e) = rt.block_on(init_core()) {
                eprintln!("W4-P: run_desktop_faithful: init_core failed on runtime A: {e}");
                std::process::exit(1);
            }
            println!("W4-P: run_desktop_faithful: runtime A init done, blocking");
            let _ = shutdown_rx.recv();
        })
        .expect("failed to spawn worker");

    std::thread::sleep(Duration::from_secs(2));

    // Exact pattern from callbacks_lifecycle.rs:118-123.
    // Capture the session ID so we can poll that specific session afterward.
    let (done_tx, done_rx) = mpsc::channel::<Option<String>>();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build current_thread runtime for dispatch");
        let res = rt.block_on(create_session_and_submit());
        let sid = match res {
            Ok((sid, outcome)) => {
                println!("W4-P: run_desktop_faithful: dispatch thread submit ok sid={sid} outcome={outcome:?}");
                Some(sid)
            }
            Err(e) => {
                println!("W4-P: run_desktop_faithful: dispatch thread submit err: {e}");
                None
            }
        };
        let _ = done_tx.send(sid);
        // runtime C dropped here when closure ends
    });

    // Wait for the dispatch thread to finish submit() (fast).
    let session_id = match done_rx.recv_timeout(Duration::from_secs(30)) {
        Ok(sid) => sid,
        Err(_) => {
            println!("W4-P: run_desktop_faithful: dispatch thread did NOT finish within 30s");
            None
        }
    };

    let _ = shutdown_tx.send(());
    worker.join().expect("worker thread panicked");
    Ok(session_id)
}

/// Poll the session state from within an existing async context (runtime B).
/// Returns true if the session is stuck in Processing.
async fn poll_session_stuck(sid: String) -> bool {
    let coordinator = match global_coordinator() {
        Some(c) => c,
        None => return true,
    };
    let sm = coordinator.session_manager().clone();
    let mut stuck = false;
    for i in 0..10 {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let state = sm.get_session(&sid).map(|s| s.state.clone());
        match &state {
            Some(SessionState::Processing { phase, .. }) => {
                println!("W4-P: run_desktop_faithful: session {sid} STUCK in Processing phase={:?} (round {i})", phase);
                stuck = true;
            }
            Some(SessionState::Idle) => {
                println!("W4-P: run_desktop_faithful: session {sid} returned to Idle (round {i})");
                break;
            }
            Some(SessionState::Error { error, .. }) => {
                println!("W4-P: run_desktop_faithful: session {sid} Error: {error} (round {i})");
                break;
            }
            None => {
                println!("W4-P: run_desktop_faithful: session {sid} not found (round {i})");
                break;
            }
        }
    }
    stuck
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_names(true)
        .with_line_number(true)
        .init();

    let mode = parse_mode();
    match mode {
        Mode::Dual => {
            let result = run_dual_runtime().await;
            match result {
                Ok(()) => println!("W4-P: main: repro finished successfully"),
                Err(e) => {
                    println!("W4-P: main: repro finished with error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Mode::Same => {
            let result = run_same_runtime().await;
            match result {
                Ok(()) => println!("W4-P: main: repro finished successfully"),
                Err(e) => {
                    println!("W4-P: main: repro finished with error: {e}");
                    std::process::exit(1);
                }
            }
        }
        Mode::Desktop => {
            // run_desktop_faithful spawns runtime A + runtime C and returns
            // the session id; we then poll from within runtime B (this context).
            let session_id = run_desktop_faithful();
            match session_id {
                Ok(Some(sid)) => {
                    let stuck = poll_session_stuck(sid).await;
                    if stuck {
                        println!("W4-P: main: CONFIRMED session stuck in Processing (hang reproduced)");
                    } else {
                        println!("W4-P: main: session NOT stuck (turn completed or aborted cleanly)");
                    }
                }
                Ok(None) => println!("W4-P: main: no session created"),
                Err(e) => {
                    println!("W4-P: main: repro error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
