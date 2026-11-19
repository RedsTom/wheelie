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
    use windows::Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    };
    use windows::Storage::Streams::{
        DataReader, IRandomAccessStream, IRandomAccessStreamReference,
    };
    use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};
    use windows::core::Interface;

    /// Covers are album art, not documents; anything larger is a runaway
    /// stream the popup does not need.
    const MAX_COVER_BYTES: u64 = 4 * 1024 * 1024;

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

        fn inner_read(&mut self) -> windows::core::Result<Reading> {
            // WinRT calls need per-thread initialisation and the action worker
            // thread starts without it. Re-initialising with the same model is
            // Ok; a different apartment only means the thread is already set up.
            let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };

            if self.manager.is_none() {
                self.manager =
                    Some(GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.join()?);
            }
            let manager = self.manager.as_ref().expect("activated above");

            // A null or failing current session means nothing is playing.
            let Ok(session) = manager.GetCurrentSession() else {
                return Ok(Reading::NoSession);
            };
            // `TryGetMediaPropertiesAsync` is the API windows-rs binds today
            // (Windows 11+). Failing it is not "no track": stay silent rather
            // than announce a stop that did not happen.
            let Ok(properties) = session.TryGetMediaPropertiesAsync()?.join() else {
                return Ok(Reading::Unavailable);
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
                title: properties.Title()?.to_string(),
                artist: properties.Artist()?.to_string(),
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
        let (start, end, position) = (millis(start), millis(end), millis(position));
        let duration = end.saturating_sub(start);
        (position.saturating_sub(start).min(duration), duration)
    }

    /// WinRT `TimeSpan` (100 ns ticks) to whole milliseconds.
    fn millis(value: windows::Foundation::TimeSpan) -> u64 {
        u64::try_from(value.Duration.max(0) / 10_000).unwrap_or(u64::MAX)
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
