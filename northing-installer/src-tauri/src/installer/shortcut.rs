use std::path::Path;

use anyhow::{Context, Result};

#[cfg(windows)]
pub fn create_desktop_shortcut(install_path: &Path) -> Result<()> {
    use mslnk::ShellLink;

    let exe_path = install_path.join("northhing.exe");
    if !exe_path.exists() {
        anyhow::bail!("main executable not found: {}", exe_path.display());
    }

    let desktop = dirs::desktop_dir().context("failed to resolve desktop directory")?;
    let shortcut_path = desktop.join("northhing.lnk");

    let mut sl = ShellLink::new(&exe_path).context("failed to create shell link")?;
    sl.set_working_dir(Some(install_path.to_string_lossy().to_string()));
    sl.set_name(Some("northhing".to_string()));
    sl.create_lnk(&shortcut_path)
        .with_context(|| format!("failed to write shortcut: {}", shortcut_path.display()))?;

    Ok(())
}

#[cfg(not(windows))]
pub fn create_desktop_shortcut(_install_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(windows)]
pub fn create_start_menu_shortcut(install_path: &Path) -> Result<()> {
    use mslnk::ShellLink;

    let exe_path = install_path.join("northhing.exe");
    if !exe_path.exists() {
        anyhow::bail!("main executable not found: {}", exe_path.display());
    }

    let start_menu = dirs::data_dir()
        .map(|d| d.join("Microsoft").join("Windows").join("Start Menu").join("Programs"))
        .context("failed to resolve start menu directory")?;
    std::fs::create_dir_all(&start_menu)?;
    let shortcut_path = start_menu.join("northhing.lnk");

    let mut sl = ShellLink::new(&exe_path).context("failed to create shell link")?;
    sl.set_working_dir(Some(install_path.to_string_lossy().to_string()));
    sl.set_name(Some("northhing".to_string()));
    sl.create_lnk(&shortcut_path)
        .with_context(|| format!("failed to write shortcut: {}", shortcut_path.display()))?;

    Ok(())
}

#[cfg(not(windows))]
pub fn create_start_menu_shortcut(_install_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(windows)]
pub fn remove_desktop_shortcut() -> Result<()> {
    let desktop = dirs::desktop_dir().context("failed to resolve desktop directory")?;
    let shortcut_path = desktop.join("northhing.lnk");
    if shortcut_path.exists() {
        std::fs::remove_file(&shortcut_path)
            .with_context(|| format!("failed to remove shortcut: {}", shortcut_path.display()))?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn remove_desktop_shortcut() -> Result<()> {
    Ok(())
}

#[cfg(windows)]
pub fn remove_start_menu_shortcut() -> Result<()> {
    let start_menu = dirs::data_dir()
        .map(|d| d.join("Microsoft").join("Windows").join("Start Menu").join("Programs"))
        .context("failed to resolve start menu directory")?;
    let shortcut_path = start_menu.join("northhing.lnk");
    if shortcut_path.exists() {
        std::fs::remove_file(&shortcut_path)
            .with_context(|| format!("failed to remove shortcut: {}", shortcut_path.display()))?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn remove_start_menu_shortcut() -> Result<()> {
    Ok(())
}
