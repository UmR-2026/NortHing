use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::installer::ai_config::{write_model_config, write_theme_preference};
use crate::installer::extract::{extract_payload, find_embedded_payload_dir, load_payload_manifest, validate_payload_sha256};
use crate::installer::registry::{
    build_uninstall_registration, launch_command, read_uninstall_registration, remove_uninstall_registration,
    write_uninstall_registration,
};
use crate::installer::shortcut::{
    create_desktop_shortcut, create_start_menu_shortcut, remove_desktop_shortcut, remove_start_menu_shortcut,
};
use crate::installer::types::{
    ConnectionTestResult, DiskSpaceInfo, ExistingInstallation, InstallPathValidation, InstallProgress,
    LaunchApplicationRequest, LaunchContext, LaunchRegisteredUninstallerRequest, ListModelConfigRequest,
    RemoteModelInfo, SetModelConfigRequest, SetThemePreferenceRequest, StartInstallationRequest,
    TestModelConfigRequest, UninstallRequest, ValidateInstallPathRequest, install_path_error,
};
use crate::installer::types::AppState;

const DISPLAY_VERSION: &str = "0.2.10";
const REQUIRED_SPACE_BYTES: u64 = 500 * 1024 * 1024;

#[tauri::command]
pub async fn get_launch_context() -> Result<LaunchContext, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut mode = "install".to_string();
    let mut uninstall_path = None;

    let mut i = 0;
    while i < args.len() {
        if args[i] == "--uninstall" && i + 1 < args.len() {
            mode = "uninstall".to_string();
            uninstall_path = Some(args[i + 1].clone());
            i += 2;
            continue;
        }
        i += 1;
    }

    if uninstall_path.is_none() {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(name) = exe.file_stem().and_then(|s| s.to_str()) {
                if name.to_lowercase().contains("uninstall") {
                    mode = "uninstall".to_string();
                }
            }
        }
    }

    Ok(LaunchContext {
        mode,
        uninstall_path,
        app_language: None,
    })
}

#[tauri::command]
pub async fn get_initial_install_path() -> Result<String, String> {
    let base = dirs::data_local_dir()
        .or_else(|| dirs::home_dir())
        .map(|d| d.join("Programs"))
        .unwrap_or_else(|| PathBuf::from("C:\\Program Files"));
    let path = base.join("northhing");
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn validate_install_path(request: ValidateInstallPathRequest) -> Result<InstallPathValidation, String> {
    let path_str = request.path.trim().to_string();
    if path_str.is_empty() {
        return Err(install_path_error("path_empty"));
    }

    let path = Path::new(&path_str);

    if !path.is_absolute() {
        return Err(install_path_error("not_absolute"));
    }

    let components: Vec<_> = path.components().collect();
    if components.len() <= 1 {
        return Err(install_path_error("filesystem_root"));
    }

    if path.exists() {
        if !path.is_dir() {
            return Err(install_path_error("path_not_directory"));
        }
        match fs::read_dir(path) {
            Ok(mut entries) => {
                if entries.next().is_some() {
                    let is_northhing = path.join("northhing.exe").exists()
                        || path.join("uninstall.exe").exists();
                    if !is_northhing {
                        return Err(install_path_error("directory_must_be_empty_or_northhing"));
                    }
                }
            }
            Err(_) => {
                return Err(install_path_error("inspect_directory_failed"));
            }
        }
    } else {
        if let Some(parent) = path.parent() {
            if parent.exists() && !is_writable(parent) {
                return Err(install_path_error("parent_not_writable"));
            }
        }
    }

    let canonical = normalize_path(path);
    Ok(InstallPathValidation {
        install_path: canonical,
    })
}

#[tauri::command]
pub async fn get_disk_space(request: ValidateInstallPathRequest) -> Result<DiskSpaceInfo, String> {
    let path = Path::new(&request.path);
    let (total, available) = disk_usage(path);
    let sufficient = available >= REQUIRED_SPACE_BYTES;
    Ok(DiskSpaceInfo {
        total,
        available,
        required: REQUIRED_SPACE_BYTES,
        sufficient,
    })
}

#[tauri::command]
pub async fn get_existing_installation() -> Result<ExistingInstallation, String> {
    if let Some(reg) = read_uninstall_registration() {
        let install_path = Path::new(&reg.install_location);
        let main_binary_present = install_path.join("northhing.exe").exists();
        return Ok(ExistingInstallation {
            detected: true,
            install_location: Some(reg.install_location),
            display_version: Some(reg.display_version),
            uninstall_string: Some(reg.uninstall_string),
            main_binary_present,
            source: Some("registry".to_string()),
        });
    }

    Ok(ExistingInstallation {
        detected: false,
        install_location: None,
        display_version: None,
        uninstall_string: None,
        main_binary_present: false,
        source: None,
    })
}

#[tauri::command]
pub async fn start_installation(
    app: AppHandle,
    state: State<'_, AppState>,
    request: StartInstallationRequest,
) -> Result<(), String> {
    let options = request.options;
    let install_path = Path::new(&options.install_path);

    let payload_dir = find_embedded_payload_dir().ok_or_else(|| "payload directory not found".to_string())?;

    let progress = |p: InstallProgress| {
        let _ = app.emit("install-progress", p);
    };

    progress(InstallProgress {
        step: "prepare".to_string(),
        percent: 0,
        message: "Validating payload".to_string(),
    });

    let manifest = load_payload_manifest(&payload_dir).map_err(|e| format!("Failed to load payload manifest: {}", e))?;
    let failures = validate_payload_sha256(&payload_dir, &manifest).map_err(|e| format!("Payload validation failed: {}", e))?;
    if !failures.is_empty() {
        return Err(format!("Payload validation failed: {}", failures.join("; ")));
    }

    progress(InstallProgress {
        step: "extract".to_string(),
        percent: 0,
        message: "Extracting files".to_string(),
    });

    if install_path.exists() {
        if let Err(e) = fs::remove_dir_all(install_path) {
            if install_path.exists() {
                return Err(format!(
                    "Failed to remove existing install directory: {}",
                    e
                ));
            }
        }
    }

    extract_payload(&payload_dir, install_path, &progress).map_err(|e| format!("Extraction failed: {}", e))?;

    progress(InstallProgress {
        step: "registry".to_string(),
        percent: 90,
        message: "Registering application".to_string(),
    });

    let reg = build_uninstall_registration(install_path, DISPLAY_VERSION);
    write_uninstall_registration(&reg).map_err(|e| format!("Failed to write uninstall registration: {}", e))?;

    progress(InstallProgress {
        step: "shortcuts".to_string(),
        percent: 95,
        message: "Creating shortcuts".to_string(),
    });

    if options.desktop_shortcut {
        if let Err(e) = create_desktop_shortcut(install_path) {
            app.emit("install-progress", InstallProgress {
                step: "shortcuts".to_string(),
                percent: 96,
                message: format!("Desktop shortcut failed: {}", e),
            }).ok();
        }
    }
    if options.start_menu {
        if let Err(e) = create_start_menu_shortcut(install_path) {
            app.emit("install-progress", InstallProgress {
                step: "shortcuts".to_string(),
                percent: 97,
                message: format!("Start menu shortcut failed: {}", e),
            }).ok();
        }
    }

    progress(InstallProgress {
        step: "complete".to_string(),
        percent: 100,
        message: "Installation complete".to_string(),
    });

    if let Ok(mut guard) = state.last_install_path.lock() {
        *guard = Some(PathBuf::from(options.install_path.clone()));
    }

    Ok(())
}

#[tauri::command]
pub async fn launch_registered_uninstaller(request: LaunchRegisteredUninstallerRequest) -> Result<(), String> {
    launch_command(&request.uninstall_command).map_err(|e| format!("Failed to launch uninstaller: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn set_model_config(request: SetModelConfigRequest) -> Result<(), String> {
    write_model_config(&request.model_config)
        .map_err(|e| format!("Failed to save model config: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn test_model_config_connection(request: TestModelConfigRequest) -> Result<ConnectionTestResult, String> {
    let model_config = request.model_config;
    let start = std::time::Instant::now();

    let format = match model_config.format.as_str() {
        "openai" => "openai",
        "responses" => "responses",
        "anthropic" => "anthropic",
        "gemini" => "gemini",
        other => other,
    };

    let ai_config = northhing_ai_adapters::AIConfig {
        name: model_config.config_name.clone().unwrap_or_else(|| model_config.provider.clone()),
        base_url: model_config.base_url.clone(),
        request_url: String::new(),
        api_key: model_config.api_key.clone(),
        model: model_config.model_name.clone(),
        format: format.to_string(),
        context_window: 0,
        max_tokens: None,
        temperature: None,
        top_p: None,
        reasoning_mode: northhing_ai_adapters::ReasoningMode::Default,
        inline_think_in_text: true,
        custom_headers: model_config.custom_headers.clone(),
        custom_headers_mode: model_config.custom_headers_mode.clone(),
        skip_ssl_verify: model_config.skip_ssl_verify.unwrap_or(false),
        reasoning_effort: None,
        thinking_budget_tokens: None,
        custom_request_body: model_config
            .custom_request_body
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
        custom_request_body_mode: None,
    };

    let client = northhing_ai_adapters::AIClient::new(ai_config);
    let timeout = Duration::from_secs(30);

    match tokio::time::timeout(timeout, client.test_connection()).await {
        Ok(Ok(result)) => {
            let elapsed = start.elapsed().as_millis() as u64;
            Ok(ConnectionTestResult {
                success: result.success,
                response_time_ms: elapsed,
                model_response: result.model_response,
                message_code: result.message_code.map(|c| format!("{:?}", c).to_lowercase()),
                error_details: result.error_details,
            })
        }
        Ok(Err(e)) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let err_str = e.to_string();
            let classified = classify_connection_error(&err_str);
            Ok(ConnectionTestResult {
                success: false,
                response_time_ms: elapsed,
                model_response: None,
                message_code: None,
                error_details: Some(classified),
            })
        }
        Err(_) => {
            let elapsed = start.elapsed().as_millis() as u64;
            Ok(ConnectionTestResult {
                success: false,
                response_time_ms: elapsed,
                model_response: None,
                message_code: None,
                error_details: Some("Connection timed out after 30 seconds".to_string()),
            })
        }
    }
}

#[tauri::command]
pub async fn list_model_config_models(
    request: ListModelConfigRequest,
) -> Result<Vec<RemoteModelInfo>, String> {
    let model_config = request.model_config;

    let format = match model_config.format.as_str() {
        "openai" => "openai",
        "responses" => "responses",
        "anthropic" => "anthropic",
        "gemini" => "gemini",
        other => other,
    };

    let ai_config = northhing_ai_adapters::AIConfig {
        name: model_config.config_name.clone().unwrap_or_else(|| model_config.provider.clone()),
        base_url: model_config.base_url.clone(),
        request_url: String::new(),
        api_key: model_config.api_key.clone(),
        model: model_config.model_name.clone(),
        format: format.to_string(),
        context_window: 0,
        max_tokens: None,
        temperature: None,
        top_p: None,
        reasoning_mode: northhing_ai_adapters::ReasoningMode::Default,
        inline_think_in_text: true,
        custom_headers: model_config.custom_headers.clone(),
        custom_headers_mode: model_config.custom_headers_mode.clone(),
        skip_ssl_verify: model_config.skip_ssl_verify.unwrap_or(false),
        reasoning_effort: None,
        thinking_budget_tokens: None,
        custom_request_body: model_config
            .custom_request_body
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok()),
        custom_request_body_mode: None,
    };

    let client = northhing_ai_adapters::AIClient::new(ai_config);
    let timeout = Duration::from_secs(30);

    match tokio::time::timeout(timeout, client.list_models()).await {
        Ok(Ok(models)) => {
            Ok(models
                .into_iter()
                .map(|m| RemoteModelInfo {
                    id: m.id,
                    display_name: m.display_name,
                })
                .collect())
        }
        Ok(Err(e)) => Err(format!("Failed to list models: {}", e)),
        Err(_) => Err("Model list request timed out after 30 seconds".to_string()),
    }
}

#[tauri::command]
pub async fn set_theme_preference(request: SetThemePreferenceRequest) -> Result<(), String> {
    write_theme_preference(&request.theme_preference)
        .map_err(|e| format!("Failed to save theme preference: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn launch_application(request: LaunchApplicationRequest) -> Result<(), String> {
    let exe_path = Path::new(&request.install_path).join("northhing.exe");
    if !exe_path.exists() {
        return Err(format!("Executable not found: {}", exe_path.display()));
    }
    std::process::Command::new(&exe_path)
        .spawn()
        .map_err(|e| format!("Failed to launch application: {}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn close_installer(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("installer") {
        let _ = window.close();
    } else {
        app.exit(0);
    }
    Ok(())
}

#[tauri::command]
pub async fn uninstall(request: UninstallRequest) -> Result<(), String> {
    let install_path = Path::new(&request.install_path);

    if request.install_path.trim().is_empty() {
        return Err("Install path is empty; nothing to uninstall.".to_string());
    }

    if !install_path.exists() {
        return Err(format!(
            "Install path does not exist: {}",
            install_path.display()
        ));
    }

    if install_path.exists() {
        fs::remove_dir_all(install_path).map_err(|e| format!("Failed to remove install directory: {}", e))?;
    }

    if let Err(e) = remove_desktop_shortcut() {
        log::warn!("Failed to remove desktop shortcut: {}", e);
    }
    if let Err(e) = remove_start_menu_shortcut() {
        log::warn!("Failed to remove start menu shortcut: {}", e);
    }

    if let Err(e) = remove_uninstall_registration() {
        log::warn!("Failed to remove uninstall registration: {}", e);
    }

    if request.delete_user_data {
        if let Some(data_dir) = dirs::config_dir() {
            let app_data = data_dir.join("northhing");
            if app_data.exists() {
                if let Err(e) = fs::remove_dir_all(&app_data) {
                    log::warn!("Failed to remove app data dir: {}", e);
                }
            }
        }
        if let Some(home) = dirs::home_dir() {
            let user_data = home.join(".northhing");
            if user_data.exists() {
                if let Err(e) = fs::remove_dir_all(&user_data) {
                    log::warn!("Failed to remove user data dir: {}", e);
                }
            }
        }
    }

    Ok(())
}

fn normalize_path(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    canonical.to_string_lossy().to_string()
}

fn is_writable(path: &Path) -> bool {
    let test_file = path.join(".northhing_write_test");
    match fs::File::create(&test_file) {
        Ok(_) => {
            let _ = fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

fn disk_usage(path: &Path) -> (u64, u64) {
    let target = if path.exists() {
        path.to_path_buf()
    } else {
        path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
    };

    #[cfg(windows)]
    {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let path_str = target.to_string_lossy().to_string();
        let wide: Vec<u16> = OsStr::new(&path_str)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut free_available: u64 = 0;
        let mut total_bytes: u64 = 0;
        let mut total_free: u64 = 0;
        let ok = unsafe {
            windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut free_available as *mut u64,
                &mut total_bytes as *mut u64,
                &mut total_free as *mut u64,
            )
        };
        if ok != 0 {
            return (total_bytes, free_available);
        }
    }

    (0, 0)
}

fn classify_connection_error(err: &str) -> String {
    let lower = err.to_lowercase();
    if lower.contains("timeout") || lower.contains("timed out") {
        "Connection timed out. Check your network and try again.".to_string()
    } else if lower.contains("dns") || lower.contains("resolve") || lower.contains("connect") {
        "Network error: could not reach the server. Check your network connection.".to_string()
    } else if lower.contains("401") || lower.contains("403") || lower.contains("unauthorized") || lower.contains("invalid api key") || lower.contains("authentication") {
        "Authentication error: the API key was rejected by the server.".to_string()
    } else if lower.contains("429") || lower.contains("rate limit") {
        "Rate limited by the provider. Try again later.".to_string()
    } else if lower.contains("certificate") || lower.contains("ssl") || lower.contains("tls") {
        "TLS/SSL error. You may enable skip SSL verify in advanced settings.".to_string()
    } else {
        format!("Connection failed: {}", err)
    }
}
