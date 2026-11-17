//! Platform capability description.

use wheelie_core::{CaptureMode, PlatformInfo};

#[cfg(target_os = "windows")]
const OS_NAME: &str = "Windows";
#[cfg(target_os = "macos")]
const OS_NAME: &str = "macOS";
#[cfg(all(unix, not(target_os = "macos")))]
const OS_NAME: &str = "Linux";
#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
const OS_NAME: &str = "Unknown";

#[cfg(target_os = "windows")]
const CAPTURE: CaptureMode = CaptureMode::ExclusiveHotkeys;
#[cfg(target_os = "macos")]
const CAPTURE: CaptureMode = CaptureMode::PermissionedHotkeys;
#[cfg(all(unix, not(target_os = "macos")))]
const CAPTURE: CaptureMode = CaptureMode::X11BestEffort;
#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
const CAPTURE: CaptureMode = CaptureMode::Unsupported;

#[cfg(all(unix, not(target_os = "macos")))]
const CAPTURE_IS_LIMITED: bool = true;
#[cfg(not(all(unix, not(target_os = "macos"))))]
const CAPTURE_IS_LIMITED: bool = false;

/// Returns the capabilities of the current platform.
pub const fn info() -> PlatformInfo {
    PlatformInfo {
        os: OS_NAME,
        capture: CAPTURE,
        capture_is_limited: CAPTURE_IS_LIMITED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_info_is_non_empty() {
        let platform = info();
        assert!(!platform.os.is_empty());
    }
}
