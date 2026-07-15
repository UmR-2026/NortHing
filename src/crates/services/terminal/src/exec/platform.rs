// R22d: platform extraction - PTY Windows/Unix + encoding + utility free fn.
// 22+ top-level free fn moved verbatim from exec.rs L1300-1617.
// Visibility: all free fn retain private (internal helpers).
// cfg(unix) / cfg(windows) attributes preserved verbatim.

#[cfg(unix)]
use super::types::LocalPipeControlState;
use super::types::{ExecControlAction, ExecSessionEntry, OutputState, DEFAULT_YIELD_TIME_MS};
#[cfg(windows)]
use super::types::{WindowsPipeJob, WindowsPipeJobHandle};

// platform-related constants live in mod.rs (facade)
#[cfg(unix)]
use super::PIPE_INTERRUPT_GRACE_TIMEOUT_MS;
#[cfg(windows)]
use super::{CREATE_NO_WINDOW, PIPE_JOB_CLOSE_WAIT_MS};
use crate::{TerminalError, TerminalResult};
use chardetng::EncodingDetector;
use encoding_rs::{Encoding, IBM866, WINDOWS_1252};
use portable_pty::CommandBuilder;
use rand::Rng;
use std::collections::HashMap;
use std::io::ErrorKind;
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
#[cfg(unix)]
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::warn;
use uuid::Uuid;

#[cfg(unix)]
pub(super) fn configure_pipe_process_group(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                let err = std::io::Error::last_os_error();
                if err.raw_os_error() != Some(libc::EPERM) || libc::setpgid(0, 0) == -1 {
                    return Err(err);
                }
            }
            Ok(())
        });
    }
}

#[cfg(not(unix))]
pub(super) fn configure_pipe_process_group(_command: &mut Command) {}

#[cfg(windows)]
pub(super) fn configure_pipe_window_visibility(command: &mut Command) {
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub(super) fn configure_pipe_window_visibility(_command: &mut Command) {}

#[cfg(not(any(unix, windows)))]
pub(super) async fn control_pipe_child(child: &mut tokio::process::Child, action: ExecControlAction) -> Option<i32> {
    match action {
        ExecControlAction::Interrupt => interrupt_pipe_child(child).await,
        ExecControlAction::Kill => kill_pipe_child(child).await,
    }
}

#[cfg(windows)]
pub(super) async fn control_pipe_child(
    child: &mut tokio::process::Child,
    pipe_job: &WindowsPipeJobHandle,
    action: ExecControlAction,
) -> Option<i32> {
    match action {
        ExecControlAction::Interrupt => interrupt_pipe_child(child, pipe_job).await,
        ExecControlAction::Kill => kill_pipe_child(child, pipe_job).await,
    }
}

#[cfg(windows)]
pub(super) async fn interrupt_pipe_child(
    child: &mut tokio::process::Child,
    pipe_job: &WindowsPipeJobHandle,
) -> Option<i32> {
    kill_pipe_child(child, pipe_job).await
}

#[cfg(windows)]
pub(super) async fn kill_pipe_child(child: &mut tokio::process::Child, pipe_job: &WindowsPipeJobHandle) -> Option<i32> {
    let _ = close_windows_pipe_job_handle(pipe_job, "kill_pipe_child"); // intentionally ignored: best-effort cleanup
    if let Ok(wait_result) = tokio::time::timeout(Duration::from_millis(PIPE_JOB_CLOSE_WAIT_MS), child.wait()).await {
        return wait_result.ok().and_then(|status| status.code());
    }

    if let Some(pid) = child.id() {
        let pid = pid.to_string();
        let mut command = Command::new("taskkill");
        command.args(["/PID", &pid, "/T", "/F"]);
        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
        {
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let taskkill_result = command.status().await;
        if taskkill_result.is_ok_and(|status| status.success()) {
            return child.wait().await.ok().and_then(|status| status.code());
        }
    }

    if let Err(e) = child.kill().await {
        warn!("Failed to kill pipe child: {e}");
    }
    child.wait().await.ok().and_then(|status| status.code())
}

#[cfg(windows)]
pub(super) fn create_windows_pipe_job(child: &tokio::process::Child) -> TerminalResult<WindowsPipeJobHandle> {
    let pid = child.id().ok_or(TerminalError::ProcessNotRunning)?;
    let raw_handle = child.raw_handle().ok_or(TerminalError::ProcessNotRunning)?;
    let job = win32job::Job::create().map_err(|error| {
        TerminalError::Io(std::io::Error::other(format!(
            "failed to create pipe job for pid {pid}: {error}"
        )))
    })?;
    let mut info = win32job::ExtendedLimitInfo::new();
    info.limit_kill_on_job_close();
    job.set_extended_limit_info(&info).map_err(|error| {
        TerminalError::Io(std::io::Error::other(format!(
            "failed to configure pipe job for pid {pid}: {error}"
        )))
    })?;
    job.assign_process(raw_handle as isize).map_err(|error| {
        TerminalError::Io(std::io::Error::other(format!(
            "failed to assign pid {pid} to pipe job: {error}"
        )))
    })?;
    Ok(Arc::new(StdMutex::new(Some(WindowsPipeJob { _job: job, _pid: pid }))))
}

#[cfg(windows)]
pub(super) fn close_windows_pipe_job_handle(pipe_job: &WindowsPipeJobHandle, reason: &str) -> bool {
    let Ok(mut guard) = pipe_job.lock() else {
        return false;
    };
    let Some(job) = guard.take() else {
        return false;
    };
    let _ = reason; // intentionally ignored: suppresses unused warning on non-windows platforms
    drop(job);
    true
}

#[cfg(unix)]
pub(super) fn process_group_id(child: &tokio::process::Child) -> Option<libc::pid_t> {
    child.id().map(|pid| pid as libc::pid_t)
}

#[cfg(unix)]
pub(super) fn request_unix_pipe_control(pgid: libc::pid_t, action: ExecControlAction) -> Option<LocalPipeControlState> {
    match action {
        ExecControlAction::Interrupt => {
            signal_pipe_process_group_id(pgid, libc::SIGINT);
            Some(LocalPipeControlState::InterruptGrace {
                deadline: tokio::time::Instant::now() + Duration::from_millis(PIPE_INTERRUPT_GRACE_TIMEOUT_MS),
            })
        }
        ExecControlAction::Kill => {
            signal_pipe_process_group_id(pgid, libc::SIGKILL);
            None
        }
    }
}

#[cfg(unix)]
pub(super) fn signal_pipe_process_group_id(pgid: libc::pid_t, signal: libc::c_int) {
    unsafe {
        libc::killpg(pgid, signal);
    }
}

#[cfg(not(any(unix, windows)))]
pub(super) async fn interrupt_pipe_child(child: &mut tokio::process::Child) -> Option<i32> {
    kill_pipe_child(child).await
}

#[cfg(not(any(unix, windows)))]
pub(super) async fn kill_pipe_child(child: &mut tokio::process::Child) -> Option<i32> {
    if let Err(e) = child.kill().await {
        warn!("Failed to kill pipe child: {e}");
    }
    child.wait().await.ok().and_then(|status| status.code())
}

pub(super) fn spawn_pipe_reader<R>(mut reader: R, output: Arc<OutputState>) -> JoinHandle<()>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 8192];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => output.push_chunk(buffer[..n].to_vec()).await,
                Err(ref error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
    })
}

#[cfg(unix)]
pub(super) fn spawn_pipe_reader_with_done<R>(
    mut reader: R,
    output: Arc<OutputState>,
    done_tx: mpsc::Sender<()>,
) -> JoinHandle<()>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tokio::spawn(async move {
        let mut buffer = vec![0u8; 8192];
        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break,
                Ok(n) => output.push_chunk(buffer[..n].to_vec()).await,
                Err(ref error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        if let Err(e) = done_tx.send(()).await {
            warn!("Failed to notify pipe reader completion: {e}");
        }
    })
}

pub(super) fn apply_sanitized_environment_to_pty(command: &mut CommandBuilder, overlay: &HashMap<String, String>) {
    command.env_clear();
    for (key, value) in sanitized_environment(overlay) {
        command.env(key, value);
    }
}

pub(super) fn sanitized_environment(overlay: &HashMap<String, String>) -> HashMap<String, String> {
    let mut env = HashMap::new();
    for (key, value) in std::env::vars() {
        if !is_tauri_host_env(&key) {
            env.insert(key, value);
        }
    }
    for (key, value) in overlay {
        env.insert(key.clone(), value.clone());
    }
    env
}

pub(super) fn is_tauri_host_env(key: &str) -> bool {
    let key = key.to_ascii_uppercase();
    key == "TAURI_CONFIG" || key.starts_with("TAURI_ENV_") || key.starts_with("TAURI_ANDROID_PACKAGE_NAME_")
}

pub(super) fn deadline_from_now(yield_time_ms: Option<u64>) -> tokio::time::Instant {
    tokio::time::Instant::now() + Duration::from_millis(yield_time_ms.unwrap_or(DEFAULT_YIELD_TIME_MS))
}

pub(super) fn new_session_id(sessions: &HashMap<i32, ExecSessionEntry>) -> i32 {
    loop {
        let session_id = if cfg!(test) {
            sessions
                .keys()
                .copied()
                .max()
                .map(|max| std::cmp::max(max, 999) + 1)
                .unwrap_or(1000)
        } else {
            rand::thread_rng().gen_range(1_000..100_000)
        };

        if !sessions.contains_key(&session_id) {
            return session_id;
        }
    }
}

pub(super) fn new_chunk_id() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}

pub(super) fn input_bytes_for_write(chars: &str, append_enter: bool) -> Vec<u8> {
    let mut bytes = chars.as_bytes().to_vec();
    if append_enter {
        #[cfg(windows)]
        bytes.push(b'\r');
        #[cfg(not(windows))]
        bytes.push(b'\n');
    }
    bytes
}

pub(super) fn bytes_to_string_smart(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }

    decode_bytes(bytes, detect_encoding(bytes))
}

pub(super) fn detect_encoding(bytes: &[u8]) -> &'static Encoding {
    let mut detector = EncodingDetector::new();
    detector.feed(bytes, true);
    let (encoding, _is_confident) = detector.guess_assess(None, true);

    if encoding == IBM866 && looks_like_windows_1252_punctuation(bytes) {
        return WINDOWS_1252;
    }

    encoding
}

pub(super) fn decode_bytes(bytes: &[u8], encoding: &'static Encoding) -> String {
    let (decoded, _, had_errors) = encoding.decode(bytes);
    if had_errors {
        String::from_utf8_lossy(bytes).into_owned()
    } else {
        decoded.into_owned()
    }
}

pub(super) const WINDOWS_1252_PUNCT_BYTES: [u8; 8] = [0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97, 0x99];

pub(super) fn looks_like_windows_1252_punctuation(bytes: &[u8]) -> bool {
    let mut saw_extended_punctuation = false;
    let mut saw_ascii_word = false;

    for &byte in bytes {
        if byte >= 0xA0 {
            return false;
        }
        if (0x80..=0x9F).contains(&byte) {
            if !WINDOWS_1252_PUNCT_BYTES.contains(&byte) {
                return false;
            }
            saw_extended_punctuation = true;
        }
        if byte.is_ascii_alphabetic() {
            saw_ascii_word = true;
        }
    }

    saw_extended_punctuation && saw_ascii_word
}
