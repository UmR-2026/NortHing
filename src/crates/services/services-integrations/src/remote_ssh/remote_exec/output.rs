//! Output buffering, truncation, and collection for remote exec sessions.

use super::manager::RemoteExecSessionEntry;
use super::process::RemoteExecProcess;
use super::types::{
    RemoteExecControlAction, RemoteExecError, RemoteExecProcessLifecycleEvent, RemoteExecProcessLifecycleStatus,
    RemoteExecSessionCompletion, RemoteExecSessionCompletionSource, RemoteExecSessionCompletionStatus,
    DEFAULT_YIELD_TIME_MS, MAX_RETAINED_OUTPUT_BYTES,
};
use rand::Rng;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex, Notify};
use tokio::time::Instant;
use tracing::warn;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct OutputCursor {
    pub(crate) next_seq: u64,
}

pub(crate) struct CollectedOutput {
    pub(crate) output: String,
    pub(crate) original_output_chars: usize,
    pub(crate) cursor: OutputCursor,
}

pub(crate) struct HeadTailText {
    pub(crate) head_budget: usize,
    pub(crate) tail_budget: usize,
    pub(crate) head: String,
    pub(crate) tail: VecDeque<char>,
    pub(crate) head_chars: usize,
    pub(crate) tail_chars: usize,
    pub(crate) omitted_chars: usize,
    pub(crate) total_chars: usize,
}

pub(crate) struct OutputState {
    pub(crate) inner: Mutex<OutputInner>,
    pub(crate) notify: Notify,
    pub(crate) output_capture_tx: Option<mpsc::UnboundedSender<String>>,
}

pub(crate) struct OutputInner {
    pub(crate) chunks: VecDeque<(u64, Vec<u8>)>,
    pub(crate) next_seq: u64,
    pub(crate) retained_bytes: usize,
    pub(crate) closed: bool,
    pub(crate) exit_code: Option<i32>,
}
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
        let capture_text = self
            .output_capture_tx
            .as_ref()
            .map(|_| String::from_utf8_lossy(&chunk).to_string());
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
                let text = String::from_utf8_lossy(chunk).to_string();
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
        deadline: Instant,
        max_output_chars: usize,
        output_tx: Option<&mpsc::Sender<String>>,
    ) -> CollectedOutput {
        let mut sink = HeadTailText::new(max_output_chars);

        loop {
            let closed = self.drain_since_with_output(&mut cursor, &mut sink, output_tx).await;
            if closed || Instant::now() >= deadline {
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
    lifecycle_tx: Option<mpsc::UnboundedSender<RemoteExecProcessLifecycleEvent>>,
    event: RemoteExecProcessLifecycleEvent,
) {
    if let Some(tx) = lifecycle_tx {
        if let Err(e) = tx.send(event) {
            warn!("Failed to emit lifecycle event: {e}");
        }
    }
}

pub(crate) fn completion_status_for_control_action(action: RemoteExecControlAction) -> RemoteExecSessionCompletionStatus {
    match action {
        RemoteExecControlAction::Interrupt => RemoteExecSessionCompletionStatus::Interrupted,
        RemoteExecControlAction::Kill => RemoteExecSessionCompletionStatus::Killed,
    }
}

pub(crate) fn completion_for_closed_remote_process(
    out_of_band_control_action: Option<RemoteExecControlAction>,
) -> RemoteExecSessionCompletion {
    if let Some(action) = out_of_band_control_action {
        return RemoteExecSessionCompletion {
            status: completion_status_for_control_action(action),
            source: RemoteExecSessionCompletionSource::OutOfBandControl,
        };
    }

    RemoteExecSessionCompletion {
        status: RemoteExecSessionCompletionStatus::Exited,
        source: RemoteExecSessionCompletionSource::Process,
    }
}

pub(crate) fn lifecycle_status_for_completion(status: RemoteExecSessionCompletionStatus) -> RemoteExecProcessLifecycleStatus {
    match status {
        RemoteExecSessionCompletionStatus::Exited => RemoteExecProcessLifecycleStatus::Exited,
        RemoteExecSessionCompletionStatus::Interrupted => RemoteExecProcessLifecycleStatus::Interrupted,
        RemoteExecSessionCompletionStatus::Killed => RemoteExecProcessLifecycleStatus::Killed,
        RemoteExecSessionCompletionStatus::Pruned => RemoteExecProcessLifecycleStatus::Pruned,
    }
}

pub(crate) fn spawn_lifecycle_exit_watcher(
    session_id: i32,
    process: Arc<RemoteExecProcess>,
    lifecycle_tx: Option<mpsc::UnboundedSender<RemoteExecProcessLifecycleEvent>>,
) {
    if lifecycle_tx.is_none() {
        return;
    }

    tokio::spawn(async move {
        let exit_code = process.output.wait_closed().await;
        let completion = completion_for_closed_remote_process(process.out_of_band_control_action());
        emit_lifecycle(
            lifecycle_tx,
            RemoteExecProcessLifecycleEvent {
                session_id,
                status: lifecycle_status_for_completion(completion.status),
                exit_code,
            },
        );
    });
}

impl HeadTailText {
    fn new(max_chars: usize) -> Self {
        let head_budget = max_chars / 2;
        let tail_budget = max_chars.saturating_sub(head_budget);
        Self {
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

    fn push_str(&mut self, text: &str) {
        for ch in text.chars() {
            self.total_chars += 1;
            if self.head_chars < self.head_budget {
                self.head.push(ch);
                self.head_chars += 1;
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

    fn render(self) -> String {
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

pub(crate) fn deadline_from_now(yield_time_ms: Option<u64>) -> Instant {
    Instant::now() + Duration::from_millis(yield_time_ms.unwrap_or(DEFAULT_YIELD_TIME_MS))
}

pub(crate) fn input_bytes_for_write(chars: &str, append_enter: bool) -> Vec<u8> {
    let mut bytes = chars.as_bytes().to_vec();
    if append_enter {
        bytes.push(b'\n');
    }
    bytes
}

pub(crate) fn new_session_id(sessions: &HashMap<i32, RemoteExecSessionEntry>) -> i32 {
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

pub(crate) fn new_chunk_id() -> String {
    Uuid::new_v4().to_string()[..8].to_string()
}

#[cfg(test)]
mod tests {
    use super::{new_session_id, HeadTailText};
    use std::collections::HashMap;

    #[test]
    fn remote_exec_session_ids_match_local_test_baseline() {
        let sessions = HashMap::new();

        assert_eq!(new_session_id(&sessions), 1000);
    }

    #[test]
    fn head_tail_text_keeps_full_output_when_unbounded() {
        let mut buffer = HeadTailText::new(usize::MAX);
        buffer.push_str("abcdefghijklmnop");

        assert_eq!(buffer.total_chars, 16);
        assert_eq!(buffer.render(), "abcdefghijklmnop");
    }
}
