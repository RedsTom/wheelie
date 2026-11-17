//! Mappings between the normalized domain keys and platform crates.

use crate::core::key::{KeyCode, Modifiers};
use enigo::Key as EnigoKey;
use global_hotkey::hotkey::{Code, Modifiers as HotkeyModifiers};

macro_rules! define_key_mappings {
    ($($variant:ident => $code:ident, $enigo:expr),* $(,)?) => {
        /// Maps a domain key to the global-hotkey code.
        pub fn to_global_code(key: KeyCode) -> Option<Code> {
            Some(match key {
                $( KeyCode::$variant => Code::$code ),*
            })
        }

        /// Maps a domain key to an enigo key.
        pub fn to_enigo_key(key: KeyCode) -> Option<EnigoKey> {
            Some(match key {
                $( KeyCode::$variant => $enigo ),*
            })
        }
    };
}

define_key_mappings! {
    KeyA => KeyA, EnigoKey::Unicode('a'),
    KeyB => KeyB, EnigoKey::Unicode('b'),
    KeyC => KeyC, EnigoKey::Unicode('c'),
    KeyD => KeyD, EnigoKey::Unicode('d'),
    KeyE => KeyE, EnigoKey::Unicode('e'),
    KeyF => KeyF, EnigoKey::Unicode('f'),
    KeyG => KeyG, EnigoKey::Unicode('g'),
    KeyH => KeyH, EnigoKey::Unicode('h'),
    KeyI => KeyI, EnigoKey::Unicode('i'),
    KeyJ => KeyJ, EnigoKey::Unicode('j'),
    KeyK => KeyK, EnigoKey::Unicode('k'),
    KeyL => KeyL, EnigoKey::Unicode('l'),
    KeyM => KeyM, EnigoKey::Unicode('m'),
    KeyN => KeyN, EnigoKey::Unicode('n'),
    KeyO => KeyO, EnigoKey::Unicode('o'),
    KeyP => KeyP, EnigoKey::Unicode('p'),
    KeyQ => KeyQ, EnigoKey::Unicode('q'),
    KeyR => KeyR, EnigoKey::Unicode('r'),
    KeyS => KeyS, EnigoKey::Unicode('s'),
    KeyT => KeyT, EnigoKey::Unicode('t'),
    KeyU => KeyU, EnigoKey::Unicode('u'),
    KeyV => KeyV, EnigoKey::Unicode('v'),
    KeyW => KeyW, EnigoKey::Unicode('w'),
    KeyX => KeyX, EnigoKey::Unicode('x'),
    KeyY => KeyY, EnigoKey::Unicode('y'),
    KeyZ => KeyZ, EnigoKey::Unicode('z'),
    Digit0 => Digit0, EnigoKey::Unicode('0'),
    Digit1 => Digit1, EnigoKey::Unicode('1'),
    Digit2 => Digit2, EnigoKey::Unicode('2'),
    Digit3 => Digit3, EnigoKey::Unicode('3'),
    Digit4 => Digit4, EnigoKey::Unicode('4'),
    Digit5 => Digit5, EnigoKey::Unicode('5'),
    Digit6 => Digit6, EnigoKey::Unicode('6'),
    Digit7 => Digit7, EnigoKey::Unicode('7'),
    Digit8 => Digit8, EnigoKey::Unicode('8'),
    Digit9 => Digit9, EnigoKey::Unicode('9'),
    F1 => F1, EnigoKey::F1,
    F2 => F2, EnigoKey::F2,
    F3 => F3, EnigoKey::F3,
    F4 => F4, EnigoKey::F4,
    F5 => F5, EnigoKey::F5,
    F6 => F6, EnigoKey::F6,
    F7 => F7, EnigoKey::F7,
    F8 => F8, EnigoKey::F8,
    F9 => F9, EnigoKey::F9,
    F10 => F10, EnigoKey::F10,
    F11 => F11, EnigoKey::F11,
    F12 => F12, EnigoKey::F12,
    F13 => F13, EnigoKey::F13,
    F14 => F14, EnigoKey::F14,
    F15 => F15, EnigoKey::F15,
    F16 => F16, EnigoKey::F16,
    F17 => F17, EnigoKey::F17,
    F18 => F18, EnigoKey::F18,
    F19 => F19, EnigoKey::F19,
    F20 => F20, EnigoKey::F20,
    F21 => F21, EnigoKey::F21,
    F22 => F22, EnigoKey::F22,
    F23 => F23, EnigoKey::F23,
    F24 => F24, EnigoKey::F24,
    ArrowUp => ArrowUp, EnigoKey::UpArrow,
    ArrowDown => ArrowDown, EnigoKey::DownArrow,
    ArrowLeft => ArrowLeft, EnigoKey::LeftArrow,
    ArrowRight => ArrowRight, EnigoKey::RightArrow,
    Home => Home, EnigoKey::Home,
    End => End, EnigoKey::End,
    PageUp => PageUp, EnigoKey::PageUp,
    PageDown => PageDown, EnigoKey::PageDown,
    Insert => Insert, EnigoKey::Insert,
    Delete => Delete, EnigoKey::Delete,
    Space => Space, EnigoKey::Space,
    Enter => Enter, EnigoKey::Return,
    Tab => Tab, EnigoKey::Tab,
    Escape => Escape, EnigoKey::Escape,
    Backspace => Backspace, EnigoKey::Backspace,
    VolumeUp => AudioVolumeUp, EnigoKey::VolumeUp,
    VolumeDown => AudioVolumeDown, EnigoKey::VolumeDown,
    VolumeMute => AudioVolumeMute, EnigoKey::VolumeMute,
    MediaPlayPause => MediaPlayPause, EnigoKey::MediaPlayPause,
    MediaNext => MediaTrackNext, EnigoKey::MediaNextTrack,
    MediaPrevious => MediaTrackPrevious, EnigoKey::MediaPrevTrack,
    MediaStop => MediaStop, EnigoKey::MediaStop,
}

/// Maps domain modifiers to global-hotkey modifiers.
pub fn to_global_modifiers(modifiers: Modifiers) -> HotkeyModifiers {
    let mut mapped = HotkeyModifiers::empty();
    if modifiers.control {
        mapped |= HotkeyModifiers::CONTROL;
    }
    if modifiers.alt {
        mapped |= HotkeyModifiers::ALT;
    }
    if modifiers.shift {
        mapped |= HotkeyModifiers::SHIFT;
    }
    if modifiers.meta {
        mapped |= HotkeyModifiers::META;
    }
    mapped
}

/// Maps domain modifiers to enigo modifier keys, in press order.
pub fn to_enigo_modifiers(modifiers: Modifiers) -> Vec<EnigoKey> {
    let mut mapped = Vec::new();
    if modifiers.control {
        mapped.push(EnigoKey::Control);
    }
    if modifiers.alt {
        mapped.push(EnigoKey::Alt);
    }
    if modifiers.shift {
        mapped.push(EnigoKey::Shift);
    }
    if modifiers.meta {
        mapped.push(EnigoKey::Meta);
    }
    mapped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::key::KeyChord;

    #[test]
    fn every_key_code_maps_to_both_platforms() {
        for key in KeyCode::ALL {
            assert!(
                to_global_code(*key).is_some(),
                "missing global-hotkey mapping for {key}"
            );
            assert!(
                to_enigo_key(*key).is_some(),
                "missing enigo mapping for {key}"
            );
        }
    }

    #[test]
    fn knob_keys_map_to_function_keys() {
        assert_eq!(to_global_code(KeyCode::F13), Some(Code::F13));
        assert_eq!(to_enigo_key(KeyCode::F13), Some(EnigoKey::F13));
        assert_eq!(to_global_code(KeyCode::F24), Some(Code::F24));
        assert_eq!(to_enigo_key(KeyCode::F24), Some(EnigoKey::F24));
    }

    #[test]
    fn media_keys_map_to_transport_controls() {
        assert_eq!(
            to_global_code(KeyCode::MediaNext),
            Some(Code::MediaTrackNext)
        );
        assert_eq!(
            to_enigo_key(KeyCode::MediaPrevious),
            Some(EnigoKey::MediaPrevTrack)
        );
        assert_eq!(to_global_code(KeyCode::VolumeUp), Some(Code::AudioVolumeUp));
    }

    #[test]
    fn modifiers_map_to_both_platforms() {
        let modifiers = Modifiers {
            control: true,
            alt: true,
            shift: true,
            meta: true,
        };
        let global = to_global_modifiers(modifiers);
        assert!(global.contains(HotkeyModifiers::CONTROL));
        assert!(global.contains(HotkeyModifiers::ALT));
        assert!(global.contains(HotkeyModifiers::SHIFT));
        assert!(global.contains(HotkeyModifiers::META));

        let enigo = to_enigo_modifiers(modifiers);
        assert_eq!(
            enigo,
            vec![
                EnigoKey::Control,
                EnigoKey::Alt,
                EnigoKey::Shift,
                EnigoKey::Meta
            ]
        );
    }

    #[test]
    fn chord_conversion_is_stable() {
        let chord = KeyChord::new(KeyCode::F15);
        let code = to_global_code(chord.key).expect("code");
        let hotkey = global_hotkey::hotkey::HotKey::new(None, code);
        assert_eq!(hotkey.key, code);
    }
}
