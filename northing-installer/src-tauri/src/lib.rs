pub mod installer;

use installer::commands::{
    close_installer, get_disk_space, get_existing_installation, get_initial_install_path,
    get_launch_context, launch_application, launch_registered_uninstaller, list_model_config_models,
    set_model_config, set_theme_preference, start_installation, test_model_config_connection,
    uninstall, validate_install_path,
};
use installer::types::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_launch_context,
            get_initial_install_path,
            validate_install_path,
            get_disk_space,
            get_existing_installation,
            start_installation,
            launch_registered_uninstaller,
            set_model_config,
            test_model_config_connection,
            list_model_config_models,
            set_theme_preference,
            launch_application,
            close_installer,
            uninstall,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
