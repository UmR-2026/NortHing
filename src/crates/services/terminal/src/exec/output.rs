// R22c split: Output handling (impl OutputState, 5 helper fn, CollectedOutput,
// impl HeadTailText, spawn_exec_process, spawn_pty_process, spawn_pipe_process)
// moved verbatim from exec.rs L725-1292.
//
// All method bodies preserved verbatim per R17 lesson. No behavior change.
// `use super::*;` brings in everything visible from exec.rs (pub items + the
// pub(crate) helpers added for cross-sibling access in this worktree).
// Post-merge (r22e), this import will be adjusted to `use super::types::*; use
// super::platform::*;` once the types and platform helpers are split out.

use super::platform::*;
use super::types::*;
use crate::{TerminalError, TerminalResult};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::VecDeque;
use std::io::ErrorKind;
use std::process::Stdio;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::{mpsc, Mutex, Notify};
use tracing::warn;

impl OutputState {
    pub(crate) fn new(output_capture_tx: Option<mpsc::UnboundedSender<String>>) -> Self {
        Self {
            inner: Mutex::new(OutputInner {
                chunks: VecDeque::new(),
                next_seq: 0,
                retained_bytes: 0,
                closed: false,
                exit_code: None,
            }),
            notify: Notify::new(),
            output_capture_tx,
        }
    }

    pub(crate) async fn push_chunk(&self, chunk: Vec<u8>) {
        if chunk.is_empty() {
            return;
        }
        let capture_text = self.output_capture_tx.as_ref().map(|_| bytes_to_string_smart(&chunk));
        {
            let mut inner = self.inner.lock().await;
            let seq = inner.next_seq;
            inner.next_seq = inner.next_seq.saturating_add(1);
            inner.retained_bytes = inner.retained_bytes.saturating_add(chunk.len());
            inner.chunks.push_back((seq, chunk));
            while inner.retained_bytes > MAX_RETAINED_OUTPUT_BYTES {
                if let Some((_, dropped)) = inner.chunks.pop_front() {
                    inner.retained_bytes = inner.retained_bytes.saturating_sub(dropped.len());
                } else {
                    break;
                }
            }
        }
        if let (Some(tx), Some(text)) = (&self.output_capture_tx, capture_text) {
            if let Err(e) = tx.send(text) {
                warn!("Failed to send output capture text: {e}");
            }
        }
        self.notify.notify_waiters();
    }

    pub(crate) async fn close(&self, exit_code: Option<i32>) {
        {
            let mut inner = self.inner.lock().await;
            inner.closed = true;
            inner.exit_code = exit_code;
        }
        self.notify.notify_waiters();
    }

    pub(crate) async fn is_closed(&self) -> bool {
        self.inner.lock().await.closed
    }

    pub(crate) async fn exit_code(&self) -> Option<i32> {
        self.inner.lock().await.exit_code
    }

    pub(crate) async fn wait_closed(&self) -> Option<i32> {
        loop {
            let notified = self.notify.notified();
            {
                let inner = self.inner.lock().await;
                if inner.closed {
                    return inner.exit_code;
                }
            }
            notified.await;
        }
    }

    async fn drain_since_with_output(
        &self,
        cursor: &mut OutputCursor,
        sink: &mut HeadTailText,
        output_tx: Option<&mpsc::Sender<String>>,
    ) -> bool {
        let inner = self.inner.lock().await;
        for (seq, chunk) in inner.chunks.iter() {
            if *seq >= cursor.next_seq {
                let text = bytes_to_string_smart(chunk);
                sink.push_str(&text);
                if let Some(tx) = output_tx {
                    if let Err(e) = tx.try_send(text) {
                        warn!("Failed to send output text to stream: {e}");
                    }
                }
            }
        }
        cursor.next_seq = inner.next_seq;
        inner.closed
    }

    pub(crate) async fn collect_until(
        &self,
        mut cursor: OutputCursor,
        deadline: tokio::time::Instant,
        max_output_chars: usize,
        output_tx: Option<&mpsc::Sender<String>>,
    ) -> CollectedOutput {
        let mut sink = HeadTailText::new(max_output_chars);

        loop {
            let closed = self.drain_since_with_output(&mut cursor, &mut sink, output_tx).await;
            if closed || tokio::time::Instant::now() >= deadline {
                break;
            }

            tokio::select! {
                _ = self.notify.notified() => {}
                _ = tokio::time::sleep_until(deadline) => break,
            }
        }

        let original_output_chars = sink.total_chars;
        CollectedOutput {
            output: sink.render(),
            original_output_chars,
            cursor,
        }
    }
}

pub(crate) fn emit_lifecycle(
    lifecycle_tx: Option<mpsc::UnboundedSender<ExecProcessLifecycleEvent>>,
    event: ExecProcessLifecycleEvent,
) {
    if let Some(tx) = lifecycle_tx {
        if let Err(e) = tx.send(event) {
            warn!("Failed to emit lifecycle event: {e}");
        }
    }
}

pub(crate) fn completion_status_for_control_action(action: ExecControlAction) -> ExecSessionCompletionStatus {
    match action {
        ExecControlAction::Interrupt => ExecSessionCompletionStatus::Interrupted,
        ExecControlAction::Kill => ExecSessionCompletionStatus::Killed,
    }
}

pub(crate) fn completion_for_closed_process(
    out_of_band_control_action: Option<ExecControlAction>,
) -> ExecSessionCompletion {
    if let Some(action) = out_of_band_control_action {
        return ExecSessionCompletion {
            status: completion_status_for_control_action(action),
            source: ExecSessionCompletionSource::OutOfBandControl,
        };
    }

    ExecSessionCompletion {
        status: ExecSessionCompletionStatus::Exited,
        source: ExecSessionCompletionSource::Process,
    }
}

pub(crate) fn lifecycle_status_for_completion(status: ExecSessionCompletionStatus) -> ExecProcessLifecycleStatus {
    match status {
        ExecSessionCompletionStatus::Exited => ExecProcessLifecycleStatus::Exited,
        ExecSessionCompletionStatus::Interrupted => ExecProcessLifecycleStatus::Interrupted,
        ExecSessionCompletionStatus::Killed => ExecProcessLifecycleStatus::Killed,
        ExecSessionCompletionStatus::Pruned => ExecProcessLifecycleStatus::Pruned,
    }
}

pub(crate) fn spawn_lifecycle_exit_watcher(
    session_id: i32,
    process: Arc<ExecProcess>,
    lifecycle_tx: Option<mpsc::UnboundedSender<ExecProcessLifecycleEvent>>,
) {
    if lifecycle_tx.is_none() {
        return;
    }

    tokio::spawn(async move {
        let exit_code = process.output.wait_closed().await;
        let completion = completion_for_closed_process(process.out_of_band_control_action());
        emit_lifecycle(
            lifecycle_tx,
            ExecProcessLifecycleEvent {
                session_id,
                status: lifecycle_status_for_completion(completion.status),
                exit_code,
            },
        );
    });
}

pub(crate) struct CollectedOutput {
    pub(crate) output: String,
    pub(crate) original_output_chars: usize,
    pub(crate) cursor: OutputCursor,
}

impl HeadTailText {
    pub(crate) fn new(max_chars: usize) -> Self {
        let head_budget = max_chars / 2;
        let tail_budget = max_chars.saturating_sub(head_budget);
        Self {
            max_chars,
            head_budget,
            tail_budget,
            head: String::new(),
            tail: VecDeque::new(),
            head_chars: 0,
            tail_chars: 0,
            omitted_chars: 0,
            total_chars: 0,
        }
    }

    pub(crate) fn push_str(&mut self, text: &str) {
        for ch in text.chars() {
            self.total_chars = self.total_chars.saturating_add(1);
            if self.max_chars == 0 {
                self.omitted_chars = self.omitted_chars.saturating_add(1);
                continue;
            }
            if self.head_chars < self.head_budget {
                self.head.push(ch);
                self.head_chars += 1;
                continue;
            }

            if self.tail_budget == 0 {
                self.omitted_chars = self.omitted_chars.saturating_add(1);
                continue;
            }

            self.tail.push_back(ch);
            self.tail_chars += 1;
            if self.tail_chars > self.tail_budget {
                self.tail.pop_front();
                self.tail_chars -= 1;
                self.omitted_chars = self.omitted_chars.saturating_add(1);
            }
        }
    }

    pub(crate) fn render(self) -> String {
        if self.omitted_chars == 0 {
            let mut output = self.head;
            output.extend(self.tail);
            return output;
        }

        let mut output = self.head;
        output.push_str("\n... [truncated, middle omitted] ...\n");
        output.extend(self.tail);
        output
    }
}

pub(crate) async fn spawn_exec_process(request: &ExecCommandRequest) -> TerminalResult<ExecProcess> {
    if request.argv.is_empty() || request.argv[0].is_empty() {
        return Err(TerminalError::InvalidConfig("missing command executable".to_string()));
    }
    if !request.cwd.is_dir() {
        return Err(TerminalError::InvalidConfig(format!(
            "working directory does not exist: {}",
            request.cwd.display()
        )));
    }

    if request.tty {
        spawn_pty_process(request).await
    } else {
        spawn_pipe_process(request).await
    }
}

async fn spawn_pty_process(request: &ExecCommandRequest) -> TerminalResult<ExecProcess> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut command = CommandBuilder::new(&request.argv[0]);
    command.cwd(&request.cwd);
    apply_sanitized_environment_to_pty(&mut command, &request.env);
    for arg in request.argv.iter().skip(1) {
        command.arg(arg);
    }

    let mut child = pair.slave.spawn_command(command)?;
    let killer = child.clone_killer();
    let output = Arc::new(OutputState::new(request.output_capture_tx.clone()));
    let mut reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;
    let writer = Arc::new(StdMutex::new(writer));
    let (writer_tx, mut writer_rx) = mpsc::channel::<Vec<u8>>(128);
    let (output_tx, mut output_rx) = mpsc::channel::<Vec<u8>>(128);

    let reader_task = tokio::task::spawn_blocking(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = buffer[..n].to_vec();
                    if output_tx.blocking_send(chunk).is_err() {
                        break;
                    }
                }
                Err(ref error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(ref error) if error.kind() == ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(_) => break,
            }
        }
    });

    let output_task = tokio::spawn({
        let output = Arc::clone(&output);
        async move {
            while let Some(chunk) = output_rx.recv().await {
                output.push_chunk(chunk).await;
            }
        }
    });

    let writer_task = tokio::spawn({
        let writer = Arc::clone(&writer);
        async move {
            while let Some(bytes) = writer_rx.recv().await {
                if let Ok(mut guard) = writer.lock() {
                    if let Err(e) = guard.write_all(&bytes) {
                        warn!("Failed to write stdin bytes: {e}");
                    }
                    if let Err(e) = guard.flush() {
                        warn!("Failed to flush stdin writer: {e}");
                    }
                }
            }
        }
    });

    let wait_blocking = tokio::task::spawn_blocking(move || child.wait().ok().map(|status| status.exit_code() as i32));
    let wait_output = Arc::clone(&output);
    let pty_handles = Arc::new(StdMutex::new(Some(PtyKeepAlive {
        _master: pair.master,
        _slave: if cfg!(windows) { Some(pair.slave) } else { None },
    })));
    let close_pty_handles = Arc::clone(&pty_handles);
    let close_task = tokio::spawn(async move {
        let code = wait_blocking.await.ok().flatten();
        writer_task.abort();
        if let Ok(mut handles) = close_pty_handles.lock() {
            handles.take();
        }

        let mut reader_task = reader_task;
        if tokio::time::timeout(Duration::from_millis(PTY_EXIT_DRAIN_TIMEOUT_MS), &mut reader_task)
            .await
            .is_err()
        {
            reader_task.abort();
        }

        let mut output_task = output_task;
        if tokio::time::timeout(Duration::from_millis(PTY_EXIT_DRAIN_TIMEOUT_MS), &mut output_task)
            .await
            .is_err()
        {
            output_task.abort();
        }
        wait_output.close(code).await;
    });

    Ok(ExecProcess {
        output,
        writer: Some(writer_tx),
        terminator: StdMutex::new(Some(Terminator::Pty(killer))),
        out_of_band_control_action: StdMutex::new(None),
        helper_tasks: StdMutex::new(vec![close_task]),
        pty_handles,
        #[cfg(windows)]
        pipe_job: None,
    })
}

async fn spawn_pipe_process(request: &ExecCommandRequest) -> TerminalResult<ExecProcess> {
    let mut command = Command::new(&request.argv[0]);
    command.args(request.argv.iter().skip(1));
    command.current_dir(&request.cwd);
    command.env_clear();
    for (key, value) in sanitized_environment(&request.env) {
        command.env(key, value);
    }
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    configure_pipe_process_group(&mut command);
    configure_pipe_window_visibility(&mut command);
    command.kill_on_drop(true);

    let mut child = command.spawn()?;
    #[cfg(windows)]
    let pipe_job = create_windows_pipe_job(&child)?;
    #[cfg(windows)]
    let wait_task_pipe_job = Arc::clone(&pipe_job);
    #[cfg(unix)]
    let pipe_pgid = process_group_id(&child).ok_or(TerminalError::ProcessNotRunning)?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let output = Arc::new(OutputState::new(request.output_capture_tx.clone()));

    let mut reader_tasks = Vec::new();
    #[cfg(unix)]
    let (reader_done_tx, mut reader_done_rx) = mpsc::channel::<()>(2);
    if let Some(stdout) = stdout {
        #[cfg(unix)]
        reader_tasks.push(spawn_pipe_reader_with_done(
            stdout,
            Arc::clone(&output),
            reader_done_tx.clone(),
        ));
        #[cfg(not(unix))]
        reader_tasks.push(spawn_pipe_reader(stdout, Arc::clone(&output)));
    }
    if let Some(stderr) = stderr {
        #[cfg(unix)]
        reader_tasks.push(spawn_pipe_reader_with_done(
            stderr,
            Arc::clone(&output),
            reader_done_tx.clone(),
        ));
        #[cfg(not(unix))]
        reader_tasks.push(spawn_pipe_reader(stderr, Arc::clone(&output)));
    }

    let (control_tx, mut control_rx) = mpsc::channel::<ExecControlAction>(1);
    let wait_output = Arc::clone(&output);

    #[cfg(unix)]
    let (child_exit_tx, mut child_exit_rx) = mpsc::channel::<Option<i32>>(1);
    #[cfg(unix)]
    tokio::spawn(async move {
        let code = child.wait().await.ok().and_then(|status| status.code());
        if let Err(e) = child_exit_tx.send(code).await {
            warn!("Failed to send child exit code: {e}");
        }
    });
    #[cfg(unix)]
    let wait_task = tokio::spawn(async move {
        let mut exit_code = None;
        let mut child_exited = false;
        let mut remaining_readers = reader_tasks.len();
        let mut control_state: Option<LocalPipeControlState> = None;

        loop {
            if let Some(state) = control_state {
                if tokio::time::Instant::now() >= state.deadline() {
                    signal_pipe_process_group_id(pipe_pgid, libc::SIGKILL);
                    control_state = None;
                }
            }

            if child_exited && remaining_readers == 0 {
                break;
            }

            let wait_budget = control_state
                .map(LocalPipeControlState::deadline)
                .map(|deadline| deadline.saturating_duration_since(tokio::time::Instant::now()))
                .filter(|duration| !duration.is_zero())
                .unwrap_or_else(|| Duration::from_millis(100));

            tokio::select! {
                biased;

                status = child_exit_rx.recv(), if !child_exited => {
                    exit_code = status.flatten();
                    child_exited = true;
                }

                action = control_rx.recv() => {
                    control_state = request_unix_pipe_control(
                        pipe_pgid,
                        action.unwrap_or(ExecControlAction::Kill),
                    );
                }

                done = reader_done_rx.recv(), if remaining_readers > 0 => {
                    if done.is_some() {
                        remaining_readers = remaining_readers.saturating_sub(1);
                    } else {
                        remaining_readers = 0;
                    }
                }

                _ = tokio::time::sleep(wait_budget), if control_state.is_some() => {}
            }
        }

        for task in reader_tasks {
            if let Err(e) = task.await {
                warn!("Reader task panicked: {e}");
            }
        }
        wait_output.close(exit_code).await;
    });
    #[cfg(not(unix))]
    let wait_task = tokio::spawn(async move {
        let code = tokio::select! {
            status = child.wait() => status.ok().and_then(|status| status.code()),
            action = control_rx.recv() => {
                #[cfg(windows)]
                {
                    control_pipe_child(
                        &mut child,
                        &wait_task_pipe_job,
                        action.unwrap_or(ExecControlAction::Kill),
                    ).await
                }
                #[cfg(not(windows))]
                {
                    control_pipe_child(&mut child, action.unwrap_or(ExecControlAction::Kill)).await
                }
            }
        };
        #[cfg(windows)]
        let _ = close_windows_pipe_job_handle(&wait_task_pipe_job, "wait_task_complete"); // intentionally ignored: best-effort cleanup
        for task in reader_tasks {
            if let Err(e) = task.await {
                warn!("Reader task panicked: {e}");
            }
        }
        wait_output.close(code).await;
    });

    Ok(ExecProcess {
        output,
        writer: None,
        terminator: StdMutex::new(Some(Terminator::Pipe(control_tx))),
        out_of_band_control_action: StdMutex::new(None),
        helper_tasks: StdMutex::new(vec![wait_task]),
        pty_handles: Arc::new(StdMutex::new(None)),
        #[cfg(windows)]
        pipe_job: Some(Arc::clone(&pipe_job)),
    })
}
