//! Screen brightness control for the primary display.
//!
//! macOS keeps using the brightness keys through `enigo`, which cannot report
//! the new level. Windows and Linux change the level directly and report it.

use crate::core::action::VolumeDirection;
use crate::platform::error::PlatformError;

/// Brightness change applied per step, in percent of the full scale. Unused
/// on macOS, where the brightness keys are sent through `enigo` instead.
#[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
const STEP_PERCENT: u8 = 5;

/// Changes the brightness by `steps` steps. Returns the new level in percent
/// when the operating system reports it.
pub fn adjust(direction: VolumeDirection, steps: u8) -> Result<Option<u8>, PlatformError> {
    imp::adjust(direction, steps)
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    /// Keeps PowerShell from opening a console window.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    /// Signed percentage change for a direction and step count.
    fn delta(direction: VolumeDirection, steps: u8) -> i32 {
        let sign = match direction {
            VolumeDirection::Up => 1,
            VolumeDirection::Down => -1,
        };
        sign * i32::from(STEP_PERCENT) * i32::from(steps)
    }

    /// Changes the brightness through the WMI monitor classes, which only
    /// exist for displays that expose a brightness control (most laptop
    /// panels). The script prints the new level, or fails when the display
    /// has no such control.
    ///
    /// Known limitation: each call starts a fresh, hidden PowerShell process,
    /// which adds noticeable latency per knob step. A persistent session is a
    /// future improvement; the current design keeps the adapter stateless.
    pub fn adjust(direction: VolumeDirection, steps: u8) -> Result<Option<u8>, PlatformError> {
        let change = delta(direction, steps);
        let script = format!(
            r#"$ErrorActionPreference = 'Stop'
$m = Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness | Select-Object -First 1
$n = [math]::Min(100, [math]::Max(0, [int]$m.CurrentBrightness + ({change})))
$c = Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods | Select-Object -First 1
Invoke-CimMethod -InputObject $c -MethodName WmiSetBrightness -Arguments @{{Timeout = 1; Brightness = [byte]$n}} | Out-Null
$n"#
        );

        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // The display is usually the cause, but keep the script's own
            // diagnostics for unsupported or locked-down WMI classes.
            return Err(PlatformError::Other(format!(
                "screen brightness is not available on this display: {}",
                stderr.trim()
            )));
        }
        let percent = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u8>()
            .map_err(|_| PlatformError::Other("unexpected brightness output".to_owned()))?;
        Ok(Some(percent))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn delta_follows_direction_and_step_count() {
            assert_eq!(delta(VolumeDirection::Up, 2), 10);
            assert_eq!(delta(VolumeDirection::Down, 3), -15);
            assert_eq!(delta(VolumeDirection::Up, 0), 0);
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::*;
    use std::io::ErrorKind;
    use std::process::Command;

    /// Signed percentage change in `brightnessctl`'s `VALUE%SIGN` syntax.
    fn delta(direction: VolumeDirection, steps: u8) -> String {
        let sign = match direction {
            VolumeDirection::Up => "+",
            VolumeDirection::Down => "-",
        };
        format!("{}{sign}", u32::from(STEP_PERCENT) * u32::from(steps))
    }

    /// Changes the brightness with `brightnessctl`, which handles the
    /// backlight permissions, and reads back the new percentage.
    pub fn adjust(direction: VolumeDirection, steps: u8) -> Result<Option<u8>, PlatformError> {
        let change = delta(direction, steps);

        let output = Command::new("brightnessctl")
            .args(["-q", "set", &change])
            .output()
            .map_err(|error| match error.kind() {
                ErrorKind::NotFound => {
                    PlatformError::Unsupported("brightnessctl is required to change the brightness")
                }
                ErrorKind::PermissionDenied => {
                    PlatformError::Other(format!("changing the brightness was denied: {error}"))
                }
                _ => PlatformError::Other(error.to_string()),
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PlatformError::Other(format!(
                "screen brightness is not available on this display: {}",
                stderr.trim()
            )));
        }

        let output = Command::new("brightnessctl").arg("-m").output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PlatformError::Other(format!(
                "could not read the brightness: {}",
                stderr.trim()
            )));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let percent = text
            .lines()
            .next()
            .and_then(|line| line.split(',').nth(3))
            .and_then(|field| field.trim_end_matches('%').parse::<u8>().ok())
            .ok_or_else(|| {
                PlatformError::Other(format!(
                    "unexpected brightnessctl output: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ))
            })?;
        Ok(Some(percent))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn delta_uses_brightnessctl_sign_suffix() {
            assert_eq!(delta(VolumeDirection::Up, 2), "10%+");
            assert_eq!(delta(VolumeDirection::Down, 3), "15%-");
        }
    }
}

#[cfg(not(any(target_os = "windows", all(unix, not(target_os = "macos")))))]
mod imp {
    use super::*;

    pub fn adjust(_direction: VolumeDirection, _steps: u8) -> Result<Option<u8>, PlatformError> {
        Err(PlatformError::Unsupported(
            "screen brightness is not available on this platform",
        ))
    }
}
