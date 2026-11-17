//! Platform description shared across layers.

/// Human-readable platform name and expected capture mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformInfo {
    /// Display name of the operating system.
    pub os: &'static str,
    /// Expected global capture mode.
    pub input_capture: &'static str,
    /// True when the compositor or permissions may restrict global capture.
    pub capture_is_limited: bool,
}
