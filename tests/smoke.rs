//! Smoke checks over the public library API.
//!
//! The CLI parser lives in the binary crate (`src/cli.rs`), so it is exercised
//! by `cargo run -- --version` in CI. These tests cover the identity, embedded
//! logo, logging setup and platform description exported by the library.

use wheelie::assets::logo_rgba;
use wheelie::core::{APP_ID, APP_NAME, CaptureMode, VERSION};
use wheelie::platform::info;

#[test]
fn app_identity_is_stable() {
    assert_eq!(APP_NAME, "Wheelie");
    assert_eq!(APP_ID, "io.github.redstom.wheelie");
    assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn embedded_logo_has_expected_dimensions() {
    let (rgba, width, height) = logo_rgba();
    assert_eq!((width, height), (128, 128));
    assert_eq!(rgba.len(), (width * height * 4) as usize);
}

#[test]
fn logging_initializes_without_panicking() {
    let guard = wheelie::app::logging::init().expect("logging must initialize");
    drop(guard);
}

#[test]
fn platform_info_is_usable() {
    let platform = info();
    assert!(!platform.os.is_empty());
    assert_ne!(platform.capture, CaptureMode::Unsupported);
    assert!(platform.brightness_supported);
}
