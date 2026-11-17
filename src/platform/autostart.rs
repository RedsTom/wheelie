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
    pub fn new(app_name: &str, app_path: &Path, args: &[&str]) -> Result<Self, PlatformError> {
        let mut builder = AutoLaunchBuilder::new();
        builder.set_app_name(app_name);
        builder.set_app_path(&app_path.to_string_lossy());
        builder.set_args(args);

        let launcher = builder
            .build()
            .map_err(|error| PlatformError::Other(error.to_string()))?;
        Ok(Self { launcher })
    }

    /// Adds Wheelie to start at login.
    pub fn enable(&self) -> Result<(), PlatformError> {
        self.launcher
            .enable()
            .map_err(|error| PlatformError::Other(error.to_string()))
    }

    /// Removes Wheelie from start at login.
    pub fn disable(&self) -> Result<(), PlatformError> {
        self.launcher
            .disable()
            .map_err(|error| PlatformError::Other(error.to_string()))
    }

    /// Whether Wheelie is currently set to start at login.
    pub fn is_enabled(&self) -> Result<bool, PlatformError> {
        self.launcher
            .is_enabled()
            .map_err(|error| PlatformError::Other(error.to_string()))
    }
}
