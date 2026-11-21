//! System volume control through each operating system's audio API.
//!
//! Volume is changed directly instead of sending media keys: injected media
//! keys come back through the global key watcher and would be read as knob
//! presses when the knob itself is bound to those keys.

use crate::core::action::VolumeDirection;
use crate::platform::error::PlatformError;

/// Volume change applied per step, in percent of the displayed scale.
const STEP_PERCENT: u8 = 2;

/// Output volume as the operating system reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    /// Volume in percent, from 0 to 100.
    pub percent: u8,
    /// Whether the output is muted.
    pub muted: bool,
}

/// Changes the default output volume by `steps` linear steps.
pub fn adjust(direction: VolumeDirection, steps: u8) -> Result<(), PlatformError> {
    imp::adjust(direction, steps)
}

/// Toggles the default output mute state.
pub fn toggle_mute() -> Result<(), PlatformError> {
    imp::toggle_mute()
}

/// Reads the current default output volume and mute state.
pub fn level() -> Result<Level, PlatformError> {
    imp::level()
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        IMMDeviceEnumerator, MMDeviceEnumerator, eMultimedia, eRender,
    };
    use windows::Win32::System::Com::{
        CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
    };

    /// Balances `CoInitializeEx` on the calling thread when it succeeded.
    struct ComScope {
        initialized: bool,
    }

    impl ComScope {
        fn enter() -> Result<Self, PlatformError> {
            // SAFETY: COM is initialised once on the current thread in the
            // multi-threaded apartment, with a matching `CoUninitialize` in
            // `Drop` (also for `S_FALSE`, which still needs balancing).
            let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if hr.is_err() {
                let error = windows::core::Error::from(hr);
                if hr == RPC_E_CHANGED_MODE {
                    tracing::debug!(%error, "thread already uses another COM apartment");
                }
                return Err(PlatformError::Com(Box::new(error)));
            }
            Ok(Self { initialized: true })
        }
    }

    impl Drop for ComScope {
        fn drop(&mut self) {
            if self.initialized {
                // SAFETY: paired with the successful `CoInitializeEx` above.
                unsafe { CoUninitialize() };
            }
        }
    }

    fn map(error: windows::core::Error) -> PlatformError {
        PlatformError::Com(Box::new(error))
    }

    fn endpoint() -> Result<IAudioEndpointVolume, PlatformError> {
        // SAFETY: standard COM activation of the system device enumerator and
        // the default render endpoint; the returned interfaces are owned.
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(map)?;
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eMultimedia)
                .map_err(map)?;
            device
                .Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)
                .map_err(map)
        }
    }

    /// Dynamic range, in dB, that the Windows volume slider spans: 0 % is
    /// -50 dB and 100 % is 0 dB, linear in dB. Matches the slider: 0.5 of
    /// the scalar (-6.02 dB) is displayed as 88 %.
    const SLIDER_RANGE_DB: f32 = 50.0;

    /// Slider percent (0..=100) for a linear scalar (0..=1).
    fn scalar_to_percent(scalar: f32) -> f32 {
        if scalar <= 0.0 {
            return 0.0;
        }
        let db = 20.0 * scalar.log10();
        ((db + SLIDER_RANGE_DB) / SLIDER_RANGE_DB * 100.0).clamp(0.0, 100.0)
    }

    /// Linear scalar (0..=1) for a slider percent (0..=100).
    fn percent_to_scalar(percent: f32) -> f32 {
        if percent <= 0.0 {
            return 0.0;
        }
        let db = percent / 100.0 * SLIDER_RANGE_DB - SLIDER_RANGE_DB;
        10f32.powf(db / 20.0).clamp(0.0, 1.0)
    }

    /// Signed percentage change for a direction and step count.
    fn delta(direction: VolumeDirection, steps: u8) -> f32 {
        let sign = match direction {
            VolumeDirection::Up => 1.0,
            VolumeDirection::Down => -1.0,
        };
        sign * f32::from(STEP_PERCENT) * f32::from(steps)
    }

    pub fn adjust(direction: VolumeDirection, steps: u8) -> Result<(), PlatformError> {
        let _com = ComScope::enter()?;
        let volume = endpoint()?;
        let change = delta(direction, steps);
        // SAFETY: the endpoint interface is valid for the whole call.
        unsafe {
            let current = volume.GetMasterVolumeLevelScalar().map_err(map)?;
            let next = percent_to_scalar(scalar_to_percent(current) + change);
            volume
                .SetMasterVolumeLevelScalar(next, std::ptr::null())
                .map_err(map)?;
        }
        Ok(())
    }

    pub fn toggle_mute() -> Result<(), PlatformError> {
        let _com = ComScope::enter()?;
        let volume = endpoint()?;
        // SAFETY: the endpoint interface is valid for the whole call.
        unsafe {
            let muted = volume.GetMute().map_err(map)?.as_bool();
            volume.SetMute(!muted, std::ptr::null()).map_err(map)?;
        }
        Ok(())
    }

    pub fn level() -> Result<Level, PlatformError> {
        let _com = ComScope::enter()?;
        let volume = endpoint()?;
        // SAFETY: the endpoint interface is valid for the whole call.
        unsafe {
            let scalar = volume.GetMasterVolumeLevelScalar().map_err(map)?;
            let muted = volume.GetMute().map_err(map)?.as_bool();
            Ok(Level {
                percent: scalar_to_percent(scalar).round() as u8,
                muted,
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn scalar_and_slider_round_trip() {
            assert_eq!(scalar_to_percent(0.0), 0.0);
            assert_eq!(scalar_to_percent(1.0), 100.0);
            assert!((scalar_to_percent(0.5) - 87.96).abs() < 0.1);
            assert_eq!(percent_to_scalar(0.0), 0.0);
            assert_eq!(percent_to_scalar(100.0), 1.0);
            assert!((percent_to_scalar(scalar_to_percent(0.5)) - 0.5).abs() < 0.001);
        }

        #[test]
        fn delta_follows_direction_and_step_count() {
            assert_eq!(delta(VolumeDirection::Up, 2), 4.0);
            assert_eq!(delta(VolumeDirection::Down, 3), -6.0);
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::*;
    use std::io::ErrorKind;
    use std::process::Command;

    fn run(program: &str, args: &[&str]) -> Result<(), PlatformError> {
        let status = Command::new(program).args(args).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(PlatformError::Other(format!(
                "{program} exited with {status}"
            )))
        }
    }

    fn capture(program: &str, args: &[&str]) -> Result<String, PlatformError> {
        let output = Command::new(program).args(args).output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(PlatformError::Other(format!(
                "{program} exited with {}",
                output.status
            )))
        }
    }

    /// Whether the primary tool is simply not installed.
    fn missing(error: &PlatformError) -> bool {
        matches!(error, PlatformError::Io(source) if source.kind() == ErrorKind::NotFound)
    }

    /// Runs `wpctl` (PipeWire) and falls back to `pactl` (PulseAudio).
    ///
    /// A missing `wpctl` is the normal fallback case. An unexpected failure is
    /// returned as-is: retrying through another backend would hide the real
    /// cause (for example a denied permission).
    fn with_fallback(wpctl: &[&str], pactl: &[&str]) -> Result<(), PlatformError> {
        match run("wpctl", wpctl) {
            Ok(()) => Ok(()),
            Err(primary) if missing(&primary) => run("pactl", pactl),
            Err(primary) => Err(primary),
        }
    }

    /// Percentage payloads for both backends.
    fn deltas(direction: VolumeDirection, steps: u8) -> (String, String) {
        let sign = match direction {
            VolumeDirection::Up => "+",
            VolumeDirection::Down => "-",
        };
        let amount = u32::from(STEP_PERCENT) * u32::from(steps);
        (format!("{amount}%{sign}"), format!("{sign}{amount}%"))
    }

    pub fn adjust(direction: VolumeDirection, steps: u8) -> Result<(), PlatformError> {
        let (wpctl_delta, pactl_delta) = deltas(direction, steps);
        with_fallback(
            &[
                "set-volume",
                "-l",
                "1.5",
                "@DEFAULT_AUDIO_SINK@",
                &wpctl_delta,
            ],
            &["set-sink-volume", "@DEFAULT_SINK@", &pactl_delta],
        )
    }

    pub fn toggle_mute() -> Result<(), PlatformError> {
        with_fallback(
            &["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"],
            &["set-sink-mute", "@DEFAULT_SINK@", "toggle"],
        )
    }

    fn wpctl_level() -> Result<Level, PlatformError> {
        let text = capture("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])?;
        let scalar: f32 = text
            .split_whitespace()
            .find_map(|token| token.parse().ok())
            .ok_or_else(|| PlatformError::Other("unexpected wpctl output".to_owned()))?;
        Ok(Level {
            percent: (scalar * 100.0).round().clamp(0.0, 100.0) as u8,
            muted: text.contains("MUTED"),
        })
    }

    fn pactl_level() -> Result<Level, PlatformError> {
        let volume = capture("pactl", &["get-sink-volume", "@DEFAULT_SINK@"])?;
        let percent: u8 = volume
            .split_whitespace()
            .find_map(|token| token.strip_suffix('%')?.parse().ok())
            .ok_or_else(|| PlatformError::Other("unexpected pactl output".to_owned()))?;
        let mute = capture("pactl", &["get-sink-mute", "@DEFAULT_SINK@"])?;
        Ok(Level {
            percent,
            muted: mute.contains("yes"),
        })
    }

    pub fn level() -> Result<Level, PlatformError> {
        match wpctl_level() {
            Ok(level) => Ok(level),
            Err(primary) if missing(&primary) => pactl_level(),
            Err(primary) => Err(primary),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn deltas_match_each_backend_syntax() {
            assert_eq!(
                deltas(VolumeDirection::Up, 2),
                ("4%+".to_owned(), "+4%".to_owned())
            );
            assert_eq!(
                deltas(VolumeDirection::Down, 3),
                ("6%-".to_owned(), "-6%".to_owned())
            );
        }

        #[test]
        fn only_a_missing_tool_is_a_fallback_case() {
            let not_found =
                PlatformError::Io(std::io::Error::new(ErrorKind::NotFound, "wpctl missing"));
            let denied = PlatformError::Io(std::io::Error::new(
                ErrorKind::PermissionDenied,
                "wpctl denied",
            ));
            assert!(missing(&not_found));
            assert!(!missing(&denied));
            assert!(!missing(&PlatformError::Other("wpctl failed".to_owned())));
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::process::Command;

    fn osascript(script: &str) -> Result<(), PlatformError> {
        let status = Command::new("osascript").arg("-e").arg(script).status()?;
        if status.success() {
            Ok(())
        } else {
            Err(PlatformError::Other(format!(
                "osascript exited with {status}"
            )))
        }
    }

    fn capture(script: &str) -> Result<String, PlatformError> {
        let output = Command::new("osascript").arg("-e").arg(script).output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        } else {
            Err(PlatformError::Other(format!(
                "osascript exited with {}",
                output.status
            )))
        }
    }

    pub fn adjust(direction: VolumeDirection, steps: u8) -> Result<(), PlatformError> {
        let delta = i32::from(STEP_PERCENT) * i32::from(steps);
        let signed = match direction {
            VolumeDirection::Up => delta,
            VolumeDirection::Down => -delta,
        };
        osascript(&format!(
            "set volume output volume (max 0 (min 100 (output volume of (get volume settings) + {signed})))"
        ))
    }

    pub fn toggle_mute() -> Result<(), PlatformError> {
        osascript("set volume output muted not (output muted of (get volume settings))")
    }

    pub fn level() -> Result<Level, PlatformError> {
        let percent: u8 = capture("output volume of (get volume settings)")?
            .parse()
            .map_err(|_| PlatformError::Other("unexpected volume output".to_owned()))?;
        let muted = capture("output muted of (get volume settings)")? == "true";
        Ok(Level { percent, muted })
    }
}

#[cfg(not(any(target_os = "windows", unix)))]
mod imp {
    use super::*;

    pub fn adjust(_direction: VolumeDirection, _steps: u8) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "system volume is not available on this platform",
        ))
    }

    pub fn toggle_mute() -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported(
            "system mute is not available on this platform",
        ))
    }

    pub fn level() -> Result<Level, PlatformError> {
        Err(PlatformError::Unsupported(
            "system volume is not available on this platform",
        ))
    }
}
