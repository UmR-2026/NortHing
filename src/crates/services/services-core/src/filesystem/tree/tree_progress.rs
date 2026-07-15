//! File-search progress reporting trait + a batched implementation.
//!
//! [`FileSearchProgressSink`] decouples the search walker (in
//! [`super::tree_search`]) from any single delivery channel; the
//! [`BatchedFileSearchProgressSink`] collects results in memory and
//! flushes them in batches of either size or time interval.

use super::tree_types::FileSearchResultGroup;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::warn;

pub trait FileSearchProgressSink: Send + Sync {
    fn report(&self, result: FileSearchResultGroup);
    fn flush(&self);
}

pub struct BatchedFileSearchProgressSink {
    batch: Mutex<Vec<FileSearchResultGroup>>,
    batch_size: usize,
    flush_interval: Duration,
    last_flush_at: Mutex<Instant>,
    on_flush: Box<dyn Fn(Vec<FileSearchResultGroup>) + Send + Sync>,
}

impl BatchedFileSearchProgressSink {
    pub fn new<F>(batch_size: usize, flush_interval: Duration, on_flush: F) -> Self
    where
        F: Fn(Vec<FileSearchResultGroup>) + Send + Sync + 'static,
    {
        Self {
            batch: Mutex::new(Vec::new()),
            batch_size: batch_size.max(1),
            flush_interval,
            last_flush_at: Mutex::new(Instant::now() - flush_interval),
            on_flush: Box::new(on_flush),
        }
    }

    fn drain_batch(&self) -> Vec<FileSearchResultGroup> {
        match self.batch.lock() {
            Ok(mut guard) => std::mem::take(&mut *guard),
            Err(poisoned) => {
                warn!("File search progress batch mutex was poisoned, recovering lock");
                let mut guard = poisoned.into_inner();
                std::mem::take(&mut *guard)
            }
        }
    }

    fn elapsed_since_last_flush(&self) -> Duration {
        match self.last_flush_at.lock() {
            Ok(guard) => guard.elapsed(),
            Err(poisoned) => {
                warn!("File search progress flush timer mutex was poisoned, recovering lock");
                poisoned.into_inner().elapsed()
            }
        }
    }

    fn mark_flushed(&self) {
        match self.last_flush_at.lock() {
            Ok(mut guard) => {
                *guard = Instant::now();
            }
            Err(poisoned) => {
                warn!("File search progress flush timer mutex was poisoned, recovering lock");
                let mut guard = poisoned.into_inner();
                *guard = Instant::now();
            }
        }
    }

    fn flush_internal(&self, force: bool) {
        let should_flush = force || self.elapsed_since_last_flush() >= self.flush_interval;
        if !should_flush {
            return;
        }

        let batch = self.drain_batch();
        if batch.is_empty() {
            return;
        }

        (self.on_flush)(batch);
        self.mark_flushed();
    }
}

impl FileSearchProgressSink for BatchedFileSearchProgressSink {
    fn report(&self, result: FileSearchResultGroup) {
        let should_flush_now = match self.batch.lock() {
            Ok(mut guard) => {
                guard.push(result);
                guard.len() >= self.batch_size
            }
            Err(poisoned) => {
                warn!("File search progress batch mutex was poisoned, recovering lock");
                let mut guard = poisoned.into_inner();
                guard.push(result);
                guard.len() >= self.batch_size
            }
        };

        if should_flush_now {
            self.flush_internal(true);
            return;
        }

        self.flush_internal(false);
    }

    fn flush(&self) {
        self.flush_internal(true);
    }
}
