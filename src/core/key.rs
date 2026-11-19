//! Normalized key codes, modifiers and chords.
//!
//! The domain never talks to an operating system. Platform crates map these
//! values to native representations later.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

macro_rules! define_key_codes {
    ($($variant:ident => $name:literal),* $(,)?) => {
        /// A normalized physical key independent from the keyboard layout.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum KeyCode {
            $( $variant ),*
        }

        impl KeyCode {
            /// Every supported key, in display order for the key picker.
            pub const ALL: &'static [Self] = &[ $( Self::$variant ),* ];

            /// Stable configuration name.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $( Self::$variant => $name ),*
                }
            }
        }

        impl fmt::Display for KeyCode {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for KeyCode {
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                $(
                    if value.eq_ignore_ascii_case($name) {
                        return Ok(Self::$variant);
                    }
                )*
                Err(format!("unknown key code: {value}"))
            }
        }
    };
}

define_key_codes! {
    KeyA => "A",
    KeyB => "B",
    KeyC => "C",
    KeyD => "D",
    KeyE => "E",
    KeyF => "F",
    KeyG => "G",
    KeyH => "H",
    KeyI => "I",
    KeyJ => "J",
    KeyK => "K",
    KeyL => "L",
    KeyM => "M",
    KeyN => "N",
    KeyO => "O",
    KeyP => "P",
    KeyQ => "Q",
    KeyR => "R",
    KeyS => "S",
    KeyT => "T",
    KeyU => "U",
    KeyV => "V",
    KeyW => "W",
    KeyX => "X",
    KeyY => "Y",
    KeyZ => "Z",
    Digit0 => "0",
    Digit1 => "1",
    Digit2 => "2",
    Digit3 => "3",
    Digit4 => "4",
    Digit5 => "5",
    Digit6 => "6",
    Digit7 => "7",
    Digit8 => "8",
    Digit9 => "9",
    F1 => "F1",
    F2 => "F2",
    F3 => "F3",
    F4 => "F4",
    F5 => "F5",
    F6 => "F6",
    F7 => "F7",
    F8 => "F8",
    F9 => "F9",
    F10 => "F10",
    F11 => "F11",
    F12 => "F12",
    F13 => "F13",
    F14 => "F14",
    F15 => "F15",
    F16 => "F16",
    F17 => "F17",
    F18 => "F18",
    F19 => "F19",
    F20 => "F20",
    F21 => "F21",
    F22 => "F22",
    F23 => "F23",
    F24 => "F24",
    ArrowUp => "ArrowUp",
    ArrowDown => "ArrowDown",
    ArrowLeft => "ArrowLeft",
    ArrowRight => "ArrowRight",
    Home => "Home",
    End => "End",
    PageUp => "PageUp",
    PageDown => "PageDown",
    Insert => "Insert",
    Delete => "Delete",
    Space => "Space",
    Enter => "Enter",
    Tab => "Tab",
    Escape => "Escape",
    Backspace => "Backspace",
    VolumeUp => "VolumeUp",
    VolumeDown => "VolumeDown",
    VolumeMute => "VolumeMute",
    MediaPlayPause => "MediaPlayPause",
    MediaNext => "MediaNext",
    MediaPrevious => "MediaPrevious",
    MediaStop => "MediaStop",
}

impl Serialize for KeyCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for KeyCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Modifier keys that may be combined with a [`KeyCode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Modifiers {
    /// Control key.
    #[serde(default)]
    pub control: bool,
    /// Alt key.
    #[serde(default)]
    pub alt: bool,
    /// Shift key.
    #[serde(default)]
    pub shift: bool,
    /// Meta key (Command, Super or Windows).
    #[serde(default)]
    pub meta: bool,
}

impl Modifiers {
    /// Returns true when no modifier is held.
    pub const fn is_empty(self) -> bool {
        !self.control && !self.alt && !self.shift && !self.meta
    }

    /// Returns true when every listed modifier is held.
    pub const fn contains(self, other: Self) -> bool {
        (!other.control || self.control)
            && (!other.alt || self.alt)
            && (!other.shift || self.shift)
            && (!other.meta || self.meta)
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.control {
            write!(formatter, "Ctrl+")?;
        }
        if self.alt {
            write!(formatter, "Alt+")?;
        }
        if self.shift {
            write!(formatter, "Shift+")?;
        }
        if self.meta {
            write!(formatter, "Meta+")?;
        }
        Ok(())
    }
}

impl FromStr for Modifiers {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut modifiers = Self::default();
        for part in value.split('+').filter(|part| !part.trim().is_empty()) {
            match part.trim().to_ascii_uppercase().as_str() {
                "CTRL" | "CONTROL" => modifiers.control = true,
                "ALT" | "OPTION" => modifiers.alt = true,
                "SHIFT" => modifiers.shift = true,
                "META" | "SUPER" | "WIN" | "WINDOWS" | "CMD" | "COMMAND" => {
                    modifiers.meta = true;
                }
                _ => return Err(format!("unknown modifier: {part}")),
            }
        }
        Ok(modifiers)
    }
}

/// A key plus the modifiers required to trigger it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    /// Physical key.
    pub key: KeyCode,
    /// Required modifiers.
    pub modifiers: Modifiers,
}

impl KeyChord {
    /// Creates a chord without modifiers.
    pub fn new(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: Modifiers::default(),
        }
    }

    /// Creates a chord with explicit modifiers.
    pub fn with_modifiers(key: KeyCode, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }

    /// Returns true when the chord is exactly the given chord.
    pub fn matches(self, other: Self) -> bool {
        self.key == other.key && self.modifiers == other.modifiers
    }

    /// Chords the operating system keeps for itself: window switching, the
    /// launcher and desktop shortcuts, volume and media transport.
    ///
    /// The system consumes a reserved chord before any window sees it, so a
    /// field that waits for a key can only record it by taking the chord over
    /// globally while the user presses it.
    ///
    /// Chords no application may take (the Windows lock, the secure attention
    /// sequence, a bare modifier) are left out on purpose: they stay with the
    /// operating system. Plain Escape is left out too, so a field can always be
    /// cancelled, and plain Alt+F4 so a window can still be closed.
    pub fn system_reserved() -> Vec<Self> {
        const ALT: Modifiers = Modifiers {
            control: false,
            alt: true,
            shift: false,
            meta: false,
        };
        const CONTROL: Modifiers = Modifiers {
            control: true,
            alt: false,
            shift: false,
            meta: false,
        };
        const META: Modifiers = Modifiers {
            control: false,
            alt: false,
            shift: false,
            meta: true,
        };

        // Volume and media transport go to the system mixer or the media
        // session, never to the focused window.
        let media = [
            KeyCode::VolumeUp,
            KeyCode::VolumeDown,
            KeyCode::VolumeMute,
            KeyCode::MediaPlayPause,
            KeyCode::MediaNext,
            KeyCode::MediaPrevious,
            KeyCode::MediaStop,
        ]
        .map(Self::new);

        // Chords the desktop shell answers before the window does.
        let shell = [
            (ALT, KeyCode::Tab),
            (ALT, KeyCode::Escape),
            (CONTROL, KeyCode::Escape),
            (META, KeyCode::Tab),
            (META, KeyCode::Space),
            (META, KeyCode::KeyD),
            (META, KeyCode::KeyE),
        ]
        .map(|(modifiers, key)| Self::with_modifiers(key, modifiers));

        media.into_iter().chain(shell).collect()
    }
}

impl From<KeyCode> for KeyChord {
    fn from(key: KeyCode) -> Self {
        Self::new(key)
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.modifiers, self.key)
    }
}

impl FromStr for KeyChord {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        let (modifiers_part, key_part) = match value.rfind('+') {
            Some(index) => (&value[..index], &value[index + 1..]),
            None => ("", value),
        };
        let key = key_part.trim().parse()?;
        let modifiers = if modifiers_part.trim().is_empty() {
            Modifiers::default()
        } else {
            modifiers_part.parse()?
        };
        Ok(Self { key, modifiers })
    }
}

impl Serialize for KeyChord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for KeyChord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_code_round_trips() {
        for key in KeyCode::ALL {
            let text = key.to_string();
            assert_eq!(
                text.parse::<KeyCode>(),
                Ok(*key),
                "roundtrip failed for {text}"
            );
        }
    }

    #[test]
    fn key_code_parsing_is_case_insensitive() {
        assert_eq!("f13".parse::<KeyCode>(), Ok(KeyCode::F13));
        assert_eq!("volumeup".parse::<KeyCode>(), Ok(KeyCode::VolumeUp));
        assert_eq!(
            "MediaPlayPause".parse::<KeyCode>(),
            Ok(KeyCode::MediaPlayPause)
        );
    }

    #[test]
    fn unknown_key_code_is_rejected() {
        assert!("F99".parse::<KeyCode>().is_err());
        assert!("".parse::<KeyCode>().is_err());
    }

    #[test]
    fn modifiers_display_and_parse() {
        let modifiers = Modifiers {
            control: true,
            alt: false,
            shift: true,
            meta: false,
        };
        assert_eq!(modifiers.to_string(), "Ctrl+Shift+");
        assert_eq!("Ctrl+Shift".parse::<Modifiers>(), Ok(modifiers));
        assert_eq!("control+shift".parse::<Modifiers>(), Ok(modifiers));
        assert!("Ctrl+NotAModifier".parse::<Modifiers>().is_err());
    }

    #[test]
    fn chord_round_trips() {
        let chord = KeyChord::with_modifiers(
            KeyCode::F13,
            Modifiers {
                control: true,
                alt: true,
                shift: false,
                meta: false,
            },
        );
        let text = chord.to_string();
        assert_eq!(text, "Ctrl+Alt+F13");
        assert_eq!(text.parse::<KeyChord>(), Ok(chord));
    }

    #[test]
    fn chord_without_modifiers_round_trips() {
        let chord = KeyChord::new(KeyCode::VolumeUp);
        assert_eq!(chord.to_string(), "VolumeUp");
        assert_eq!("VolumeUp".parse::<KeyChord>(), Ok(chord));
    }

    #[test]
    fn chord_rejects_missing_key() {
        assert!("Ctrl+".parse::<KeyChord>().is_err());
        assert!("Ctrl+NotAKey".parse::<KeyChord>().is_err());
    }

    #[test]
    fn system_reserved_covers_switchers_and_media_keys() {
        let reserved = KeyChord::system_reserved();
        let alt = |key| {
            KeyChord::with_modifiers(
                key,
                Modifiers {
                    alt: true,
                    ..Default::default()
                },
            )
        };
        let ctrl = |key| {
            KeyChord::with_modifiers(
                key,
                Modifiers {
                    control: true,
                    ..Default::default()
                },
            )
        };

        for chord in [
            KeyChord::new(KeyCode::VolumeUp),
            KeyChord::new(KeyCode::MediaPlayPause),
            alt(KeyCode::Tab),
            alt(KeyCode::Escape),
            ctrl(KeyCode::Escape),
        ] {
            assert!(reserved.contains(&chord), "missing reserved {chord}");
        }
    }

    #[test]
    fn system_reserved_lists_each_chord_once() {
        let reserved = KeyChord::system_reserved();
        let unique: std::collections::HashSet<KeyChord> = reserved.iter().copied().collect();
        assert_eq!(
            reserved.len(),
            unique.len(),
            "a chord is listed twice: {reserved:?}"
        );
    }

    #[test]
    fn system_reserved_leaves_plain_keys_and_cancellation_to_the_app() {
        let reserved = KeyChord::system_reserved();
        for chord in [
            KeyChord::new(KeyCode::Escape),
            KeyChord::new(KeyCode::Tab),
            KeyChord::new(KeyCode::KeyA),
            KeyChord::new(KeyCode::F13),
            KeyChord::with_modifiers(
                KeyCode::F4,
                Modifiers {
                    alt: true,
                    ..Default::default()
                },
            ),
        ] {
            assert!(
                !reserved.contains(&chord),
                "{chord} must stay a normal key press"
            );
        }
    }

    #[test]
    fn chord_serde_uses_stable_string() {
        #[derive(Serialize, Deserialize)]
        struct Wrapper {
            chord: KeyChord,
        }

        let chord = KeyChord::new(KeyCode::F14);
        let serialized = toml::to_string(&Wrapper { chord }).expect("serialize");
        assert!(serialized.contains("\"F14\""), "unexpected: {serialized}");

        let parsed: Wrapper = toml::from_str(&serialized).expect("deserialize");
        assert_eq!(parsed.chord, chord);
    }
}
