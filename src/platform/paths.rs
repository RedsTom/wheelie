//! Platform-specific user directories.
//!
//! Wheelie owns its configuration folder, independent of the toolkit, so that
//! the file can be edited by hand and survives UI changes.

use super::error::PlatformError;
use directories::ProjectDirs;
use std::path::PathBuf;

fn project_dirs() -> Result<ProjectDirs, PlatformError> {
    ProjectDirs::from("", "", "Wheelie")
        .ok_or_else(|| PlatformError::Other("could not resolve Wheelie user directories".into()))
}

/// Configuration directory: `%APPDATA%\Wheelie\config` on Windows,
/// `~/.config/Wheelie` on Linux (XDG_CONFIG_HOME) and
/// `~/Library/Application Support/Wheelie` on macOS.
pub fn config_dir() -> Result<PathBuf, PlatformError> {
    Ok(project_dirs()?.config_dir().to_path_buf())
}

/// Data directory: `%APPDATA%\Wheelie\data` on Windows,
/// `~/.local/share/Wheelie` on Linux (XDG_DATA_HOME) and the same
/// `~/Library/Application Support/Wheelie` as the configuration on macOS.
pub fn data_dir() -> Result<PathBuf, PlatformError> {
    Ok(project_dirs()?.data_dir().to_path_buf())
}

/// Log directory, created on demand.
pub fn log_dir() -> Result<PathBuf, PlatformError> {
    let dir = data_dir()?.join("logs");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
