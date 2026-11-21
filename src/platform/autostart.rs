//! Start-at-login adapter built on `auto-launch`.

use crate::platform::error::PlatformError;
use auto_launch::{AutoLaunch, AutoLaunchBuilder};
use std::path::Path;

/// Manages the current user's start-at-login entry.
pub struct AutoStartService {
    launcher: AutoLaunch,
}

impl AutoStartService {
    /// Builds the service for the Wheelie executable.
    ///
    /// `app_path` must name an existing file: a stale or missing executable
    /// would create a login entry that can never start the application.
    pub fn new(app_name: &str, app_path: &Path, args: &[&str]) -> Result<Self, PlatformError> {
        if !app_path.is_file() {
            return Err(PlatformError::Autostart(format!(
                "application path is not a file: {}",
                app_path.display()
            )));
        }

        let mut builder = AutoLaunchBuilder::new();
        builder.set_app_name(app_name);
        builder.set_app_path(&app_path.to_string_lossy());
        builder.set_args(args);

        let launcher = builder
            .build()
            .map_err(|error| PlatformError::Autostart(error.to_string()))?;
        Ok(Self { launcher })
    }

    /// Adds Wheelie to start at login.
    pub fn enable(&self) -> Result<(), PlatformError> {
        self.launcher
            .enable()
            .map_err(|error| PlatformError::Autostart(error.to_string()))
    }

    /// Removes Wheelie from start at login.
    pub fn disable(&self) -> Result<(), PlatformError> {
        self.launcher
            .disable()
            .map_err(|error| PlatformError::Autostart(error.to_string()))
    }

    /// Whether Wheelie is currently set to start at login.
    pub fn is_enabled(&self) -> Result<bool, PlatformError> {
        self.launcher
            .is_enabled()
            .map_err(|error| PlatformError::Autostart(error.to_string()))
    }
}

/// Enables or disables the login entry for `path`.
///
/// `args` must match the ones used by [`is_enabled`] lookups; the application
/// passes its own start-at-login flag.
pub fn set(app_id: &str, path: &Path, args: &[&str], enabled: bool) -> Result<(), PlatformError> {
    let service = AutoStartService::new(app_id, path, args)?;
    if enabled {
        service.enable()
    } else {
        service.disable()
    }
}

/// Whether the login entry for `path` currently exists.
pub fn is_enabled(app_id: &str, path: &Path) -> Result<bool, PlatformError> {
    AutoStartService::new(app_id, path, &[])?.is_enabled()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_executable_is_rejected() {
        let missing = Path::new("/definitely/not/a/real/application");
        assert!(AutoStartService::new("wheelie", missing, &[]).is_err());
        assert!(set("wheelie", missing, &[], true).is_err());
        assert!(is_enabled("wheelie", missing).is_err());
    }
}
