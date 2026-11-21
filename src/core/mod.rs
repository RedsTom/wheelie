//! Pure domain model for Wheelie.
//!
//! This module must stay independent from the toolkit, operating-system APIs
//! and file-system side effects. It owns the configuration model, key chords,
//! actions, macros, gesture detection, profiles and wheel geometry.
//!
//! # Dependency direction
//!
//! Dependencies always point inward: `ui` may use `app`, `app` may use
//! `platform`, and `platform` may use `core`. `core` uses none of them and only
//! depends on data crates. In particular it never opens a window, calls an OS
//! API or spawns a thread: the only blocking call allowed here is the short,
//! cancellable sleep between steps of a macro replay.
//!
//! # Closed enums and forward compatibility
//!
//! Domain enums such as [`Action`], [`MacroStep`], [`SetIcon`] and [`KeyCode`]
//! are deliberately closed: a version only understands the variants it ships
//! with. New variants are added in a later configuration version, and serde
//! tags keep previously stored values readable. Unknown values must never be
//! guessed at runtime; they are rejected with a validation error instead.

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
pub use app_info::{APP_ID, APP_NAME, AUTOSTART_FLAG, REPOSITORY_URL, VERSION};
pub use config::{CONFIG_VERSION, Config, ConfigError, GeneralConfig};
pub use gesture::{
    Bindings, Control, DEFAULT_DOUBLE_CLICK_MS, DEFAULT_ROTATION_DEBOUNCE_MS, GestureConfig,
    GestureDetector, GestureEvent, InputEvent,
};
pub use i18n::{Language, TextKey, tr};
pub use key::{KeyChord, KeyCode, Modifiers};
pub use macro_engine::{
    MAX_TEXT_LEN, MacroDefinition, MacroSink, MacroStep, run_macro, validate_steps,
};
pub use platform::{CaptureMode, PlatformInfo};
pub use profile::{Actions, Profile};
pub use theme::ThemeMode;
pub use update::{Release, is_stable_tag, is_update_available, should_notify};
pub use wheel::WheelGeometry;
