//! Action sets selectable from the radial wheel.
//!
//! A set groups the actions of the three knob controls under a name and an
//! icon. The wheel shows one sector per set; confirming a sector makes that set
//! the active one for the knob.

use crate::core::i18n::{Language, TextKey, tr};
use crate::core::profile::Actions;
use serde::{Deserialize, Serialize};

/// Icon shown for a set in the wheel and in the actions list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SetIcon {
    /// Speaker with sound waves.
    #[default]
    Volume,
    /// Sun.
    Brightness,
    /// Vertical arrows.
    Scroll,
    /// Horizontal arrows.
    ScrollHorizontal,
    /// Keyboard.
    Keyboard,
    /// Mouse.
    Mouse,
    /// Musical note.
    Media,
    /// Star.
    Star,
    /// Heart.
    Heart,
    /// Gear.
    Gear,
    /// Magnifier.
    Search,
    /// Folder.
    Folder,
    /// Rocket.
    App,
    /// Film strip.
    Video,
    /// Web browser.
    Browser,
    /// Envelope.
    Mail,
    /// Calculator.
    Calculator,
    /// Terminal window.
    Terminal,
    /// Picture.
    Photo,
    /// Game controller.
    Game,
    /// House.
    Home,
    /// Padlock.
    Lock,
    /// Wi-Fi signal.
    Wifi,
    /// Bluetooth mark.
    Bluetooth,
    /// Battery.
    Battery,
    /// Clock face.
    Clock,
    /// Calendar.
    Calendar,
    /// Bell.
    Bell,
    /// Bookmark.
    Bookmark,
    /// Push pin.
    Pin,
    /// Code brackets.
    Code,
    /// Database cylinders.
    Database,
    /// Cloud.
    Cloud,
    /// Download arrow.
    Download,
    /// Upload arrow.
    Upload,
    /// Desktop monitor.
    Monitor,
    /// Laptop.
    Laptop,
    /// Phone.
    Phone,
    /// Headphones.
    Headphones,
    /// Microphone.
    Microphone,
    /// Paint palette.
    Palette,
    /// Pencil.
    Pencil,
    /// Compass.
    Compass,
    /// Flag.
    Flag,
    /// Gift.
    Gift,
    /// Lightning bolt.
    Lightning,
    /// Flame.
    Fire,
    /// Leaf.
    Leaf,
    /// Crescent moon.
    Moon,
    /// Prohibition sign.
    Prohibit,
}

impl SetIcon {
    /// Every icon, in display order for the picker.
    pub const ALL: &'static [Self] = &[
        Self::Volume,
        Self::Brightness,
        Self::Scroll,
        Self::ScrollHorizontal,
        Self::Keyboard,
        Self::Mouse,
        Self::Media,
        Self::Star,
        Self::Heart,
        Self::Gear,
        Self::Search,
        Self::Folder,
        Self::App,
        Self::Video,
        Self::Browser,
        Self::Mail,
        Self::Calculator,
        Self::Terminal,
        Self::Photo,
        Self::Game,
        Self::Home,
        Self::Lock,
        Self::Wifi,
        Self::Bluetooth,
        Self::Battery,
        Self::Clock,
        Self::Calendar,
        Self::Bell,
        Self::Bookmark,
        Self::Pin,
        Self::Code,
        Self::Database,
        Self::Cloud,
        Self::Download,
        Self::Upload,
        Self::Monitor,
        Self::Laptop,
        Self::Phone,
        Self::Headphones,
        Self::Microphone,
        Self::Palette,
        Self::Pencil,
        Self::Compass,
        Self::Flag,
        Self::Gift,
        Self::Lightning,
        Self::Fire,
        Self::Leaf,
        Self::Moon,
        Self::Prohibit,
    ];
}

/// One selectable group of actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionSet {
    /// Stable identifier.
    pub id: String,
    /// Display name, editable by the user.
    pub name: String,
    /// Icon shown in the wheel and in the list.
    #[serde(default)]
    pub icon: SetIcon,
    /// Actions for the three knob controls.
    #[serde(default)]
    pub actions: Actions,
}

impl ActionSet {
    /// Creates a set with no actions.
    pub fn new(id: impl Into<String>, name: impl Into<String>, icon: SetIcon) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            icon,
            actions: Actions::default(),
        }
    }
}

/// Identifier of the default volume set.
pub const VOLUME_SET_ID: &str = "volume";
/// Identifier of the default brightness set.
pub const BRIGHTNESS_SET_ID: &str = "brightness";
/// Identifier of the default vertical scroll set.
pub const SCROLL_SET_ID: &str = "scroll";
/// Identifier of the default horizontal scroll set.
pub const SCROLL_HORIZONTAL_SET_ID: &str = "scroll-horizontal";
/// Identifier of the default "no action" set.
pub const NONE_SET_ID: &str = "none";

/// Set that disables every knob control.
pub fn none_set(language: Language) -> ActionSet {
    ActionSet::new(
        NONE_SET_ID,
        tr(language, TextKey::SetNone),
        SetIcon::Prohibit,
    )
}

/// Default sets offered on a fresh configuration.
///
/// The brightness set is only included when the input backend supports it.
pub fn default_sets(language: Language, brightness_supported: bool) -> Vec<ActionSet> {
    use crate::core::action::{Action, ScrollAxis, VolumeDirection};

    let mut volume = ActionSet::new(
        VOLUME_SET_ID,
        tr(language, TextKey::SetVolume),
        SetIcon::Volume,
    );
    volume.actions.left = Action::Volume {
        direction: VolumeDirection::Down,
        steps: 1,
    };
    volume.actions.right = Action::Volume {
        direction: VolumeDirection::Up,
        steps: 1,
    };

    let mut sets = vec![volume];

    if brightness_supported {
        let mut brightness = ActionSet::new(
            BRIGHTNESS_SET_ID,
            tr(language, TextKey::SetBrightness),
            SetIcon::Brightness,
        );
        brightness.actions.left = Action::Brightness {
            direction: VolumeDirection::Down,
            steps: 1,
        };
        brightness.actions.right = Action::Brightness {
            direction: VolumeDirection::Up,
            steps: 1,
        };
        sets.push(brightness);
    }

    let mut scroll = ActionSet::new(
        SCROLL_SET_ID,
        tr(language, TextKey::SetScroll),
        SetIcon::Scroll,
    );
    scroll.actions.left = Action::Scroll {
        axis: ScrollAxis::Vertical,
        amount: -3,
    };
    scroll.actions.right = Action::Scroll {
        axis: ScrollAxis::Vertical,
        amount: 3,
    };
    sets.push(scroll);
    sets.push(horizontal_scroll_set(language));
    sets.push(none_set(language));

    sets
}

/// True when `set` is the untouched brightness set created by default.
///
/// Used to drop that set on platforms whose input backend cannot change the
/// brightness, without touching a set the user customized.
pub fn is_default_brightness_set(set: &ActionSet) -> bool {
    use crate::core::action::{Action, VolumeDirection};

    set.id == BRIGHTNESS_SET_ID
        && set.actions.left
            == (Action::Brightness {
                direction: VolumeDirection::Down,
                steps: 1,
            })
        && set.actions.right
            == (Action::Brightness {
                direction: VolumeDirection::Up,
                steps: 1,
            })
        && set.actions.press == Action::None
}

/// Horizontal scroll set, also used when upgrading older documents.
pub fn horizontal_scroll_set(language: Language) -> ActionSet {
    use crate::core::action::{Action, ScrollAxis};

    let mut set = ActionSet::new(
        SCROLL_HORIZONTAL_SET_ID,
        tr(language, TextKey::SetScrollHorizontal),
        SetIcon::ScrollHorizontal,
    );
    set.actions.left = Action::Scroll {
        axis: ScrollAxis::Horizontal,
        amount: -3,
    };
    set.actions.right = Action::Scroll {
        axis: ScrollAxis::Horizontal,
        amount: 3,
    };
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_sets_cover_volume_brightness_and_both_scroll_axes() {
        let sets = default_sets(Language::Fr, true);
        assert_eq!(sets.len(), 5);
        assert_eq!(sets[0].id, VOLUME_SET_ID);
        assert_eq!(sets[1].id, BRIGHTNESS_SET_ID);
        assert_eq!(sets[2].id, SCROLL_SET_ID);
        assert_eq!(sets[3].id, SCROLL_HORIZONTAL_SET_ID);
        assert_eq!(sets[4].id, NONE_SET_ID);
        assert_eq!(sets[0].name, "Volume");
        assert_eq!(sets[1].name, "Luminosité");
        assert_eq!(sets[2].name, "Défilement");
        assert_eq!(sets[3].name, "Défilement horizontal");
        assert_eq!(sets[4].name, "Aucune action");
        assert_eq!(sets[0].icon, SetIcon::Volume);
        assert_eq!(sets[3].icon, SetIcon::ScrollHorizontal);
        assert_eq!(sets[4].icon, SetIcon::Prohibit);
    }

    #[test]
    fn brightness_set_is_omitted_without_backend_support() {
        let sets = default_sets(Language::En, false);
        assert_eq!(sets.len(), 4);
        assert!(sets.iter().all(|set| set.id != BRIGHTNESS_SET_ID));
        assert_eq!(sets[1].id, SCROLL_SET_ID);
        assert_eq!(sets[2].id, SCROLL_HORIZONTAL_SET_ID);
        assert_eq!(sets[3].id, NONE_SET_ID);
    }

    #[test]
    fn icons_round_trip_through_toml() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Wrapper {
            icon: SetIcon,
        }

        for icon in SetIcon::ALL {
            let document = toml::to_string(&Wrapper { icon: *icon }).expect("serialize");
            let parsed: Wrapper = toml::from_str(&document).expect("deserialize");
            assert_eq!(parsed.icon, *icon);
        }
    }
}
