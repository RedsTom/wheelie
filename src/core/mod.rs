//! Pure domain model for Wheelie.
//!
//! This module must stay independent from eframe, operating-system APIs and
//! file-system side effects. It owns the configuration model, key chords,
//! actions, macros, gesture detection, profiles and wheel geometry.

pub mod action;
pub mod action_set;
pub mod app_info;
pub mod config;
pub mod gesture;
pub mod i18n;
pub mod key;
pub mod macro_engine;
pub mod platform;
pub mod profile;
pub mod theme;
pub mod update;
pub mod wheel;

pub use action::{Action, MediaCommand, MouseButton, ScrollAxis, VolumeDirection};
pub use action_set::{
    ActionSet, BRIGHTNESS_SET_ID, SCROLL_SET_ID, SetIcon, VOLUME_SET_ID, default_sets,
};
pub use app_info::{APP_ID, APP_NAME, REPOSITORY_URL, VERSION};
pub use config::{CONFIG_VERSION, Config, ConfigError, GeneralConfig};
pub use gesture::{Bindings, Control, GestureConfig, GestureDetector, GestureEvent, InputEvent};
pub use i18n::{Language, TextKey, tr};
pub use key::{KeyChord, KeyCode, Modifiers};
pub use macro_engine::{MacroDefinition, MacroSink, MacroStep, run_macro};
pub use platform::{CaptureMode, PlatformInfo};
pub use profile::{Actions, Profile};
pub use theme::ThemeMode;
pub use update::{Release, is_update_available, should_notify};
pub use wheel::WheelGeometry;
