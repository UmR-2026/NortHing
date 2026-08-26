//! SSE log collector.
//!
//! Buffers raw SSE data in a bounded ring buffer while a stream is in-flight and only flushes the
//! captured entries to the log on error. Output respects `SseLogConfig`'s
//! `max_output` budget by dropping the oldest entries on overflow so long
//! histories remain debuggable within a fixed memory bound.

use crate::types::SseLogConfig;
use std::collections::VecDeque;
use tracing::error;

/// SSE log collector - Collects raw SSE data in a bounded ring buffer, outputs only on error
pub struct SseLogCollector {
    buffer: VecDeque<String>,
    config: SseLogConfig,
    evicted: usize,
}

impl SseLogCollector {
    pub fn new(config: SseLogConfig) -> Self {
        Self {
            buffer: VecDeque::new(),
            config,
            evicted: 0,
        }
    }

    /// Push one SSE data entry
    pub fn push(&mut self, data: String) {
        if let Some(max) = self.config.max_output {
            if max == 0 {
                self.evicted += 1;
                return;
            }
            while self.buffer.len() >= max {
                self.buffer.pop_front();
                self.evicted += 1;
            }
        }
        self.buffer.push_back(data);
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
        let total_received = self.buffer.len() + self.evicted;
        let mut sse_msg = if self.evicted > 0 {
            format!(
                "SSE history (showing last {} of {} events):\n",
                self.buffer.len(),
                total_received
            )
        } else {
            format!("SSE history ({} events):\n", self.buffer.len())
        };

        for (i, data) in self.buffer.iter().enumerate() {
            sse_msg.push_str(&format!("{:>6}: {}\n", i, data));
        }

        error!("{}", sse_msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SseLogConfig;

    #[test]
    fn default_config_max_output_is_2000() {
        assert_eq!(SseLogConfig::default().max_output, Some(2000));
    }

    #[test]
    fn bounded_collector_evicts_oldest_on_overflow() {
        let config = SseLogConfig {
            max_output: Some(3),
        };
        let mut collector = SseLogCollector::new(config);
        for i in 1..=5 {
            collector.push(format!("data-{}", i));
        }
        assert_eq!(collector.len(), 3);
        assert_eq!(collector.evicted, 2);
        let items: Vec<&String> = collector.buffer.iter().collect();
        assert_eq!(items, vec!["data-3", "data-4", "data-5"]);
    }

    #[test]
    fn unbounded_collector_keeps_all_entries() {
        let config = SseLogConfig { max_output: None };
        let mut collector = SseLogCollector::new(config);
        for i in 1..=5 {
            collector.push(format!("data-{}", i));
        }
        assert_eq!(collector.len(), 5);
        assert_eq!(collector.evicted, 0);
    }
}
