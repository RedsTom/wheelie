//! Screen brightness control for the primary display.
//!
//! macOS keeps using the brightness keys through `enigo`, which cannot report
//! the new level. Windows and Linux change the level directly and report it.

use crate::core::action::VolumeDirection;
use crate::platform::error::PlatformError;

/// Brightness change applied per step, in percent of the full scale.
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

    /// Changes the brightness through the WMI monitor classes, which only
    /// exist for displays that expose a brightness control (most laptop
    /// panels). The script prints the new level, or fails when the display
    /// has no such control.
    pub fn adjust(direction: VolumeDirection, steps: u8) -> Result<Option<u8>, PlatformError> {
        let sign = match direction {
            VolumeDirection::Up => 1,
            VolumeDirection::Down => -1,
        };
        let delta = sign * i32::from(STEP_PERCENT) * i32::from(steps);
        let script = format!(
            r#"$ErrorActionPreference = 'Stop'
$m = Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness | Select-Object -First 1
$n = [math]::Min(100, [math]::Max(0, [int]$m.CurrentBrightness + ({delta})))
$c = Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods | Select-Object -First 1
Invoke-CimMethod -InputObject $c -MethodName WmiSetBrightness -Arguments @{{Timeout = 1; Brightness = [byte]$n}} | Out-Null
$n"#
        );

        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()?;
        if !output.status.success() {
            return Err(PlatformError::Unsupported(
                "screen brightness is not available on this display",
            ));
        }
        let percent = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u8>()
            .map_err(|_| PlatformError::Other("unexpected brightness output".to_owned()))?;
        Ok(Some(percent))
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::*;
    use std::process::Command;

    /// Changes the brightness with `brightnessctl`, which handles the
    /// backlight permissions, and reads back the new percentage.
    pub fn adjust(direction: VolumeDirection, steps: u8) -> Result<Option<u8>, PlatformError> {
        let sign = match direction {
            VolumeDirection::Up => "+",
            VolumeDirection::Down => "-",
        };
        let amount = u32::from(STEP_PERCENT) * u32::from(steps);
        let delta = format!("{amount}%{sign}");

        let status = Command::new("brightnessctl")
            .args(["-q", "set", &delta])
            .status()
            .map_err(|_| {
                PlatformError::Unsupported("brightnessctl is required to change the brightness")
            })?;
        if !status.success() {
            return Err(PlatformError::Unsupported(
                "screen brightness is not available on this display",
            ));
        }

        let output = Command::new("brightnessctl").arg("-m").output()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let percent = text
            .lines()
            .next()
            .and_then(|line| line.split(',').nth(3))
            .and_then(|field| field.trim_end_matches('%').parse::<u8>().ok())
            .ok_or_else(|| PlatformError::Other("unexpected brightnessctl output".to_owned()))?;
        Ok(Some(percent))
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
