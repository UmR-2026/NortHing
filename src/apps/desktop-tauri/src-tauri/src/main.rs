mod core_rt;
mod event_bridge;
mod commands;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    core_rt::init_core_runtime();

    tauri::Builder::default()
        .setup(|app| {
            event_bridge::register(&app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_session,
            commands::list_sessions,
            commands::send_message,
            commands::get_messages,
            commands::get_or_create_latest_session,
            commands::stop_streaming,
        ])
        .run(tauri::generate_context!())
        .expect("error while running northhing desktop");
}
