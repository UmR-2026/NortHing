use crate::agentic::tools::framework::ToolUseContext;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

const PROGRESS_CHANNEL_CAPACITY: usize = 256;

pub(super) struct ExecOutputProgressBridge {
    tx: mpsc::Sender<String>,
    task: JoinHandle<()>,
}

impl ExecOutputProgressBridge {
    pub(super) fn start(context: &ToolUseContext, _tool_name: &str) -> Option<Self> {
        let _tool_use_id = context.tool_call_id.clone()?;
        let (tx, mut rx) = mpsc::channel::<String>(PROGRESS_CHANNEL_CAPACITY);
        let task = tokio::spawn(async move { while let Some(_chunk) = rx.recv().await {} });

        Some(Self { tx, task })
    }

    pub(super) fn sender(&self) -> mpsc::Sender<String> {
        self.tx.clone()
    }

    pub(super) async fn finish(self) {
        drop(self.tx);
        let _ = tokio::time::timeout(Duration::from_millis(500), self.task).await;
    }
}
