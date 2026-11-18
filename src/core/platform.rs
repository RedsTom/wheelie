//! Platform description shared across layers.

/// Expected global-capture mechanism for the current platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    /// Exclusive global hotkeys (Windows).
    ExclusiveHotkeys,
    /// Global hotkeys that may require system permissions (macOS).
    PermissionedHotkeys,
    /// X11 grabs with best-effort Wayland support (Linux).
    X11BestEffort,
    /// No global capture support.
    Unsupported,
}

/// Human-readable platform name and capture description.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformInfo {
    /// Display name of the operating system.
    pub os: &'static str,
    /// Expected global capture mode.
    pub capture: CaptureMode,
    /// True when the compositor or permissions may restrict global capture.
    pub capture_is_limited: bool,
    /// True when the input backend can change the screen brightness.
    pub brightness_supported: bool,
}
