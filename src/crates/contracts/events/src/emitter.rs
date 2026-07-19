/// EventEmitter Trait
///
/// All event sending interfaces for all platforms, core layer sends events through this trait
use async_trait::async_trait;

/// Event emitter trait
///
/// Core services send events through this trait, without directly depending on specific platforms
#[async_trait]
pub trait EventEmitter: Send + Sync {
    /// Send generic events
    async fn emit(&self, event_name: &str, payload: serde_json::Value) -> anyhow::Result<()>;

    /// Send Snapshot events
    async fn emit_snapshot(&self, snapshot_id: &str, event_data: serde_json::Value) -> anyhow::Result<()> {
        self.emit(
            "snapshot-event",
            serde_json::json!({
                "snapshot_id": snapshot_id,
                "event_data": event_data
            }),
        )
        .await
    }
}
