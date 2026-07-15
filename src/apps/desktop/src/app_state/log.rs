//! log module — see mod.rs for the wiring entry point.

use northhing_core::infrastructure::debug_log::log_event;
use std::sync::OnceLock;
use tokio::sync::mpsc;

/// Command sent to the background debug-log consumer.
struct LogCommand {
    component: &'static str,
    location: &'static str,
    mode_id: String,
    message: String,
    data: Option<[(String, String); 4]>,
}

/// Singleton sender + background thread handle.
///
/// Phase P4 (2026-06-22): replaces the per-call `std::thread::spawn` +
/// `tokio::runtime::Builder` anti-pattern with a single `OnceLock`-
/// initialised `mpsc::unbounded_channel` + one persistent consumer
/// thread.  This matches the `EventBus` pattern in
/// `northhing_transport::event_bus`.
static LOG_CHANNEL: OnceLock<mpsc::UnboundedSender<LogCommand>> = OnceLock::new();

/// Handle to the background consumer thread, stored for graceful
/// shutdown on process exit.
static LOG_HANDLE: OnceLock<std::thread::JoinHandle<()>> = OnceLock::new();

/// Ensure the background consumer is running.  Idempotent — the first
/// caller initialises the channel; subsequent calls are no-ops.
fn ensure_log_consumer() {
    if LOG_CHANNEL.get().is_some() {
        return;
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<LogCommand>();

    // Try to install the sender.  If another thread raced us and won,
    // drop our sender and use the one that was installed.
    if LOG_CHANNEL.set(tx).is_err() {
        return;
    }

    // Spawn the single background thread that owns the tokio runtime.
    let handle = std::thread::spawn(move || {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
            return;
        };
        rt.block_on(async move {
            while let Some(cmd) = rx.recv().await {
                log_event(cmd.component, &cmd.mode_id, cmd.location, &cmd.message, cmd.data).await;
            }
        });
    });

    // Store the handle for potential graceful shutdown.
    let _ = LOG_HANDLE.set(handle);
}

/// Phase H (2026-06-20): fire-and-forget debug-log helper.
///
/// Wraps `northhing_core::infrastructure::debug_log::log_event` via an
/// `mpsc::unbounded_channel` so the sync Slint callbacks can record
/// structured events without blocking.  Errors are swallowed (the
/// underlying `log_event` is also non-blocking and silent on failure)
/// — debug logging MUST NOT take down the UI.
///
/// Phase P4 (2026-06-22): optimised from per-call `std::thread::spawn` +
/// new tokio runtime to a `OnceLock`-initialised channel + single
/// background consumer thread.
///
/// Note: `location` is `'static` (matches `log_event`'s signature),
/// while `mode_id` and `message` are borrowed.  The channel owns the
/// cloned strings and passes them by reference inside the async block.
pub(super) fn log_debug_event(
    component: &'static str,
    location: &'static str,
    mode_id: &str,
    message: &str,
    data: Option<[(&str, String); 4]>,
) {
    // Ensure the background consumer is running (no-op after first call).
    ensure_log_consumer();

    // Clone the borrowed strings so the channel owns them.
    let mode_owned = mode_id.to_string();
    let message_owned = message.to_string();
    let owned_data: Option<[(String, String); 4]> = data.map(|pairs| {
        [
            (pairs[0].0.to_string(), pairs[0].1.clone()),
            (pairs[1].0.to_string(), pairs[1].1.clone()),
            (pairs[2].0.to_string(), pairs[2].1.clone()),
            (pairs[3].0.to_string(), pairs[3].1.clone()),
        ]
    });

    let cmd = LogCommand {
        component,
        location,
        mode_id: mode_owned,
        message: message_owned,
        data: owned_data,
    };

    // Fire-and-forget.  If the channel is closed (e.g. the consumer
    // thread panicked), we silently drop the log — debug logging must
    // never block or panic.
    if let Some(tx) = LOG_CHANNEL.get() {
        let _ = tx.send(cmd);
    }
}
