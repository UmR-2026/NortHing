//! SSE log collector.
//!
//! Buffers raw SSE data while a stream is in-flight and only flushes the
//! captured entries to the log on error. Output respects `SseLogConfig`'s
//! `max_output` budget by emitting a head + tail window when the buffer
//! overflows so long histories remain debuggable.

use crate::types::SseLogConfig;
use tracing::error;

/// SSE log collector - Collects raw SSE data, outputs only on error
pub struct SseLogCollector {
    buffer: Vec<String>,
    config: SseLogConfig,
}

impl SseLogCollector {
    pub fn new(config: SseLogConfig) -> Self {
        Self {
            buffer: Vec::new(),
            config,
        }
    }

    /// Push one SSE data entry
    pub fn push(&mut self, data: String) {
        self.buffer.push(data);
    }

    /// Get number of collected data entries
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Flush all SSE data to log on error
    pub fn flush_on_error(&self, error_context: &str) {
        if self.buffer.is_empty() {
            error!("SSE Error: {} (no SSE data collected)", error_context);
            return;
        }

        error!("SSE Error: {}", error_context);
        let mut sse_msg = format!("SSE history ({} events):\n", self.buffer.len());

        match self.config.max_output {
            None => {
                // No limit, output all
                for (i, data) in self.buffer.iter().enumerate() {
                    sse_msg.push_str(&format!("{:>6}: {}\n", i, data));
                }
            }
            Some(max) if self.buffer.len() <= max => {
                // Within limit, output all
                for (i, data) in self.buffer.iter().enumerate() {
                    sse_msg.push_str(&format!("{:>6}: {}\n", i, data));
                }
            }
            Some(max) => {
                // Exceeds limit, smart truncation: output beginning + end
                let head = 50.min(max / 2);
                let tail = max - head;
                let total = self.buffer.len();

                for (i, data) in self.buffer.iter().take(head).enumerate() {
                    sse_msg.push_str(&format!("{:>6}: {}\n", i, data));
                }
                sse_msg.push_str(&format!("... ({} events omitted) ...\n", total - max));
                for (i, data) in self.buffer.iter().skip(total - tail).enumerate() {
                    sse_msg.push_str(&format!("{:>6}: {}\n", total - tail + i, data));
                }
            }
        }

        error!("{}", sse_msg);
    }
}
