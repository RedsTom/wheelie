//! Now-playing report for the system media session.
//!
//! Windows exposes one system-wide playback session (System Media Transport
//! Controls). Wheelie reads the title, artist, cover and playback state from
//! it so a media action can show what is now playing. Other platforms have
//! no equivalent unified session yet, so [`Reader`] reports
//! [`Reading::Unavailable`] there and [`SUPPORTED`] turns the indicator off.

/// Whether the operating system reports the current track.
pub const SUPPORTED: bool = cfg!(windows);

/// Encoded image format of a cover, as the player published it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverFormat {
    /// PNG (`.png` magic: `0x89 'P' 'N' 'G'`).
    Png,
    /// JPEG (`.jpg` magic: `0xFF 0xD8 0xFF`).
    Jpeg,
}

/// Cover art bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cover {
    /// Encoded image bytes.
    pub bytes: Vec<u8>,
    /// Encoding of `bytes`.
    pub format: CoverFormat,
}

/// Identifies the image so two equal covers share one decoded texture.
impl Cover {
    /// Wraps `bytes` as a cover when their magic bytes name a format Wheelie
    /// can draw.
    pub fn detect(bytes: &[u8]) -> Option<Cover> {
        let format = detect_format(bytes)?;
        Some(Cover {
            bytes: bytes.to_vec(),
            format,
        })
    }
}

/// Format sniffing from the magic bytes; `None` for anything Wheelie cannot
/// draw.
fn detect_format(bytes: &[u8]) -> Option<CoverFormat> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some(CoverFormat::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(CoverFormat::Jpeg)
    } else {
        None
    }
}

/// What the media session reports at the moment of the call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NowPlaying {
    /// Track title, as the player reports it.
    pub title: String,
    /// Artist name, possibly empty.
    pub artist: String,
    /// Cover art when the player published one.
    pub cover: Option<Cover>,
    /// Whether the session is currently playing (versus paused or stopped).
    pub playing: bool,
    /// Elapsed position inside the track, in milliseconds.
    pub position_ms: u64,
    /// Total track length, in milliseconds; 0 when the player hides it.
    pub duration_ms: u64,
}

/// What the system session reports at the moment of the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading {
    /// A session exists; its fields may still be incomplete.
    Track(NowPlaying),
    /// No app owns the session: nothing has been playing.
    NoSession,
    /// The session exists but cannot be read (player without metadata, or a
    /// Windows release without the now-playing API). Stay silent.
    Unavailable,
}

/// Reads the session, keeping the manager handle alive between reads.
///
/// Re-activating the session manager on every poll would add latency to the
/// indicator, which polls right after a transport key; the action worker
/// therefore owns one reader for its lifetime.
pub struct Reader(imp::Reader);

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}

impl Reader {
    /// A reader with no session manager activated yet.
    pub fn new() -> Self {
        Self(imp::Reader::new())
    }

    /// Best-effort read of the current session.
    pub fn read(&mut self) -> Reading {
        self.0.read()
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use crate::platform::error::PlatformError;
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Storage::Streams::{
        DataReader, IRandomAccessStream, IRandomAccessStreamReference,
    };
    use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};
    use windows::core::Interface;

    /// Covers are album art, not documents; anything larger is a runaway
    /// stream the popup does not need.
    const MAX_COVER_BYTES: u64 = 4 * 1024 * 1024;

    /// Balances the single WinRT initialisation made for one read.
    struct WinRtScope;

    impl WinRtScope {
        fn enter() -> windows::core::Result<Self> {
            // SAFETY: WinRT is initialised once here in the multi-threaded
            // apartment; every successful call (including S_FALSE) is balanced
            // in `Drop`. A mode mismatch is reported instead of swallowed.
            let result = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
            if let Err(error) = result {
                if error.code() == RPC_E_CHANGED_MODE {
                    tracing::debug!(%error, "thread already uses another WinRT apartment");
                }
                return Err(error);
            }
            Ok(Self)
        }
    }

    impl Drop for WinRtScope {
        fn drop(&mut self) {
            // SAFETY: paired with the successful `RoInitialize` above.
            unsafe { RoUninitialize() };
        }
    }

    fn com(error: windows::core::Error) -> PlatformError {
        PlatformError::Com(Box::new(error))
    }

    pub struct Reader {
        manager: Option<GlobalSystemMediaTransportControlsSessionManager>,
    }

    impl Reader {
        pub fn new() -> Self {
            Self { manager: None }
        }

        pub fn read(&mut self) -> Reading {
            match self.inner_read() {
                Ok(reading) => reading,
                Err(error) => {
                    tracing::debug!(%error, "could not read the media session");
                    Reading::Unavailable
                }
            }
        }

        fn inner_read(&mut self) -> Result<Reading, PlatformError> {
            let _winrt = WinRtScope::enter().map_err(com)?;

            if self.manager.is_none() {
                self.manager = Some(
                    GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
                        .map_err(com)?
                        .join()
                        .map_err(com)?,
                );
            }
            let Some(manager) = self.manager.as_ref() else {
                return Err(PlatformError::Other(
                    "the media session manager was not activated".to_owned(),
                ));
            };

            // A failing session lookup means nothing owns the session; a
            // readable session with no metadata is a different case and is
            // reported as unavailable below.
            let session = match manager.GetCurrentSession() {
                Ok(session) => session,
                Err(error) => {
                    tracing::debug!(%error, "no current media session");
                    return Ok(Reading::NoSession);
                }
            };
            // `TryGetMediaPropertiesAsync` is the API windows-rs binds today
            // (Windows 11+). Failing it is not "no track": stay silent rather
            // than announce a stop that did not happen.
            let properties = match session.TryGetMediaPropertiesAsync().map_err(com)?.join() {
                Ok(properties) => properties,
                Err(error) => {
                    tracing::debug!(%error, "current session publishes no readable metadata");
                    return Ok(Reading::Unavailable);
                }
            };
            let playing = matches!(
                session
                    .GetPlaybackInfo()
                    .and_then(|info| info.PlaybackStatus()),
                Ok(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
            );
            let cover = properties
                .Thumbnail()
                .ok()
                .as_ref()
                .and_then(cover_from_reference);
            let (position_ms, duration_ms) = timeline_position(&session);
            Ok(Reading::Track(NowPlaying {
                title: properties.Title().map_err(com)?.to_string(),
                artist: properties.Artist().map_err(com)?.to_string(),
                cover,
                playing,
                position_ms,
                duration_ms,
            }))
        }
    }

    /// Elapsed and total milliseconds from the session timeline.
    ///
    /// WinRT reports positions against `StartTime`, in 100-nanosecond units;
    /// a session without a readable timeline simply gets no bar.
    fn timeline_position(session: &GlobalSystemMediaTransportControlsSession) -> (u64, u64) {
        let Ok(timeline) = session.GetTimelineProperties() else {
            return (0, 0);
        };
        let (Ok(start), Ok(end), Ok(position)) = (
            timeline.StartTime(),
            timeline.EndTime(),
            timeline.Position(),
        ) else {
            return (0, 0);
        };
        timeline_bounds(start.Duration, end.Duration, position.Duration)
    }

    /// Pure timeline arithmetic, relative to the reported start time.
    fn timeline_bounds(start: i64, end: i64, position: i64) -> (u64, u64) {
        let (start, end, position) = (
            millis_ticks(start),
            millis_ticks(end),
            millis_ticks(position),
        );
        let duration = end.saturating_sub(start);
        (position.saturating_sub(start).min(duration), duration)
    }

    /// WinRT timestamps are 100 ns ticks; convert them to whole milliseconds.
    fn millis_ticks(ticks: i64) -> u64 {
        u64::try_from(ticks.max(0) / 10_000).unwrap_or(u64::MAX)
    }

    /// Reads and sniffs the cover stream; `None` on any failure, which only
    /// costs the artwork.
    fn cover_from_reference(reference: &IRandomAccessStreamReference) -> Option<Cover> {
        let operation = reference.OpenReadAsync().ok()?;
        let stream: IRandomAccessStream = operation.join().ok()?.cast().ok()?;
        let size = stream.Size().ok()?;
        if size == 0 || size > MAX_COVER_BYTES {
            return None;
        }
        let reader = DataReader::CreateDataReader(&stream).ok()?;
        reader.LoadAsync(size as u32).ok()?.join().ok()?;
        let mut bytes = vec![0u8; size as usize];
        reader.ReadBytes(&mut bytes).ok()?;
        let format = detect_format(&bytes)?;
        Some(Cover { bytes, format })
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn millis_converts_winrt_ticks() {
            assert_eq!(millis_ticks(10_000), 1);
            assert_eq!(millis_ticks(1_234_567), 123);
            assert_eq!(millis_ticks(-10_000), 0);
        }

        #[test]
        fn timeline_is_relative_to_start_and_clamped() {
            // 10 000 ticks are one millisecond.
            assert_eq!(timeline_bounds(0, 100_000, 50_000), (5, 10));
            assert_eq!(timeline_bounds(10_000, 110_000, 5_000), (0, 10));
            assert_eq!(timeline_bounds(10_000, 110_000, 200_000), (10, 10));
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod imp {
    use super::*;

    pub struct Reader;

    impl Reader {
        pub fn new() -> Self {
            Self
        }

        pub fn read(&mut self) -> Reading {
            Reading::Unavailable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_covers_are_detected() {
        let bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let cover = Cover::detect(&bytes).expect("png");
        assert_eq!(cover.format, CoverFormat::Png);
        assert_eq!(cover.bytes, bytes);
    }

    #[test]
    fn jpeg_covers_are_detected() {
        let cover = Cover::detect(&[0xFF, 0xD8, 0xFF, 0xE0]).expect("jpeg");
        assert_eq!(cover.format, CoverFormat::Jpeg);
    }

    #[test]
    fn unknown_formats_are_rejected() {
        assert!(Cover::detect(&[]).is_none());
        assert!(Cover::detect(b"GIF89a").is_none());
        assert!(detect_format(b"GIF89a").is_none());
    }
}
