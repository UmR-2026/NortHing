/// Transport adapters for different platforms
pub mod cli;
pub mod websocket;

#[cfg(feature = "tauri-adapter")]
pub mod tauri;

#[cfg(feature = "slint-adapter")]
pub mod slint;

pub use cli::{CliEvent, CliTransportAdapter};
pub use websocket::{WebSocketTransportAdapter, WsMessage};

#[cfg(feature = "tauri-adapter")]
pub use tauri::TauriTransportAdapter;

#[cfg(feature = "slint-adapter")]
pub use slint::{SlintEvent, SlintTransportAdapter};
