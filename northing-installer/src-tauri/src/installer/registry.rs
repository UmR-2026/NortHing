use std::path::Path;

use anyhow::{Context, Result};

const UNINSTALL_REGISTRY_KEY: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\northhing";

#[derive(Debug, Clone)]
pub struct UninstallRegistration {
    pub display_name: String,
    pub display_version: String,
    pub uninstall_string: String,
    pub install_location: String,
    pub publisher: String,
}

pub fn build_uninstall_registration(
    install_path: &Path,
    display_version: &str,
) -> UninstallRegistration {
    let uninstall_exe = install_path.join("uninstall.exe");
    let uninstall_string = format!(
        "\"{}\" --uninstall \"{}\"",
        uninstall_exe.display(),
        install_path.display()
    );
    UninstallRegistration {
        display_name: "northhing".to_string(),
        display_version: display_version.to_string(),
        uninstall_string,
        install_location: install_path.to_string_lossy().to_string(),
        publisher: "northhing Team".to_string(),
    }
}

#[cfg(windows)]
pub fn write_uninstall_registration(reg: &UninstallRegistration) -> Result<()> {
    use winreg::enums::KEY_ALL_ACCESS;

    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (key, _disp) = hkcu
        .create_subkey_with_flags(UNINSTALL_REGISTRY_KEY, KEY_ALL_ACCESS)
        .with_context(|| format!("failed to create registry key: {}", UNINSTALL_REGISTRY_KEY))?;

    key.set_value("DisplayName", &reg.display_name)?;
    key.set_value("DisplayVersion", &reg.display_version)?;
    key.set_value("UninstallString", &reg.uninstall_string)?;
    key.set_value("InstallLocation", &reg.install_location)?;
    key.set_value("Publisher", &reg.publisher)?;
    key.set_value("NoModify", &1u32)?;
    key.set_value("NoRepair", &1u32)?;

    Ok(())
}

#[cfg(not(windows))]
pub fn write_uninstall_registration(_reg: &UninstallRegistration) -> Result<()> {
    Ok(())
}

#[cfg(windows)]
pub fn read_uninstall_registration() -> Option<UninstallRegistration> {
    use winreg::enums::KEY_READ;

    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let key = hkcu.open_subkey_with_flags(UNINSTALL_REGISTRY_KEY, KEY_READ).ok()?;

    let install_location: String = key.get_value("InstallLocation").ok()?;
    let display_name: String = key.get_value("DisplayName").ok()?;
    let display_version: String = key.get_value("DisplayVersion").ok()?;
    let uninstall_string: String = key.get_value("UninstallString").ok()?;
    let publisher: String = key.get_value("Publisher").ok().unwrap_or_default();

    Some(UninstallRegistration {
        display_name,
        display_version,
        uninstall_string,
        install_location,
        publisher,
    })
}

#[cfg(not(windows))]
pub fn read_uninstall_registration() -> Option<UninstallRegistration> {
    None
}

#[cfg(windows)]
pub fn remove_uninstall_registration() -> Result<()> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    match hkcu.delete_subkey_all(UNINSTALL_REGISTRY_KEY) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("failed to delete registry key: {}", UNINSTALL_REGISTRY_KEY)),
    }
}

#[cfg(not(windows))]
pub fn remove_uninstall_registration() -> Result<()> {
    Ok(())
}

pub fn launch_command(command: &str) -> Result<()> {
    let command = command.trim();
    if command.is_empty() {
        anyhow::bail!("empty command");
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        const DETACHED_PROCESS: u32 = 0x00000008;
        Command::new("cmd")
            .arg("/C")
            .arg(command)
            .creation_flags(CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS)
            .spawn()
            .with_context(|| format!("failed to launch command: {}", command))?;
    }

    #[cfg(not(windows))]
    {
        use std::process::Command;
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .spawn()
            .with_context(|| format!("failed to launch command: {}", command))?;
    }

    Ok(())
}
