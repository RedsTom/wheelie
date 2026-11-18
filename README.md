<div align="center">

<img src="assets/logo/logo-4x.png" alt="Wheelie logo" width="120">

# Wheelie

**Turn a keyboard knob into a radial command wheel.**

[![CI][ci-shield]][ci-link]
[![Release][release-shield]][releases-link]
[![Downloads][downloads-shield]][releases-link]
[![License][license-shield]][license-link]
[![Platforms][platforms-shield]](#platform-support)

Wheelie recreates the **Windows Wheel UI/UX** — a radial menu driven by a
keyboard knob — on Windows, macOS and Linux. It is not a Windows Wheel
emulator, needs no driver, and is not affiliated with Microsoft.

</div>

## How it works

You assign the keys your keyboard software emits for **knob left**, **knob
right** and **knob press**. Wheelie captures them exclusively — no app loses
them to the background — and runs your own actions instead.

1. **Set up once.** On first launch, a short guided setup picks the interface
   language, assigns the three knob keys with a live test button, and explains
   the wheel.
2. **Compose your wheel.** Each sector is an *action set* with its own name and
   icon — volume, brightness, scrolling, media, or anything you invent.
3. **Double-click the knob.** The radial overlay opens on your primary screen.
   Rotate or point to select a set, press to make it active. Outside the wheel,
   rotation and a single press run the active set's actions.

## Highlights

- **Action sets** ship ready to use: volume (press mutes), brightness where the
  system accepts it, vertical and horizontal scroll, and media transport. Add as
  many as you like, each with a name, an icon and three actions.
- **Eight action types**: volume, brightness, scroll, macro, keyboard shortcut,
  mouse click, media control and app launching.
- **Macro editor** with typed steps — keys, text, delays, clicks and scrolls.
  No scripting language, no arbitrary shell execution.
- **Tray + autostart**: closing the window hides Wheelie to the tray; the menu
  only shows what applies right now. Optionally start at login, single-instance
  guaranteed.
- **French and English**, dark and light themes.
- **Portable and installed builds** for every platform, built automatically for
  each release.

## Download

Grab the latest release for your platform:

| Platform | Installer | Portable |
| --- | --- | --- |
| Windows 10/11 (x64) | `*-win-x64-setup.exe` | `*-win-x64-portable.zip` |
| macOS 12+ (universal) | `*-macos-universal.dmg` | `*-macos-portable.zip` |
| Linux x64 | `.deb`, `.rpm`, `.AppImage` | `*-linux-x64-portable.tar.gz` |

> Builds are not code-signed yet: expect one SmartScreen prompt on Windows and a
> Gatekeeper confirmation on macOS (right-click → Open).

Releases: [latest stable][releases-link] · [all releases][releases-link]

## Platform support

| Platform | Global capture | Status |
| --- | --- | --- |
| Windows 10/11 | Exclusive global hotkeys | Supported |
| macOS | Global hotkeys; Accessibility/Input Monitoring may be required for media keys | Supported with permissions |
| Linux X11 | Exclusive X11 grabs | Supported |
| Linux Wayland | Native global capture is not available to normal apps | Best-effort, clearly warned |

The tray icon is best-effort on Linux: when creation fails, Wheelie logs the
reason and keeps running without it. The app stays fully usable from the
settings window with no knob configured.

## Configuration

| Platform | Path |
| --- | --- |
| Windows | `%APPDATA%\Wheelie\config.toml` |
| Linux | `~/.config/wheelie/config.toml` |
| macOS | `~/Library/Application Support/Wheelie/config.toml` |

Config writes are atomic; an unreadable file is kept as `config.toml.bak` and
Wheelie starts from defaults. The schema is versioned: older configurations are
upgraded in place and unknown fields never break a downgrade path. Logs live in
the platform data directory under `Wheelie/logs/`.

## Build from source

Requires a recent stable Rust toolchain (see `rust-version` in `Cargo.toml`).

```bash
cargo run              # develop
cargo test             # domain, persistence and mocked platform tests
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

On Linux, install the GTK/X11/Wayland development packages listed in
[`.github/workflows/ci.yml`](.github/workflows/ci.yml) first. Physical-knob
verification follows [`docs/MANUAL_TEST.md`](docs/MANUAL_TEST.md); the
development plan lives in [`ROADMAP.md`](ROADMAP.md).

Under the hood: Rust with `eframe`/`egui` (wgpu) and `gpui` components,
`global-hotkey`, `enigo`, `tray-icon`, `serde` + TOML, bundled Rubik font.
No Tauri, Electron, WebView, async runtime or kernel driver.

## License

[GPL-3.0-only][license-link]. See [`LICENSE`](LICENSE).

[ci-shield]: https://github.com/RedsTom/wheelie/actions/workflows/ci.yml/badge.svg
[ci-link]: https://github.com/RedsTom/wheelie/actions/workflows/ci.yml
[release-shield]: https://img.shields.io/github/v/release/RedsTom/wheelie?label=release&color=4B8BFF
[downloads-shield]: https://img.shields.io/github/downloads/RedsTom/wheelie/total?label=downloads
[releases-link]: https://github.com/RedsTom/wheelie/releases
[license-shield]: https://img.shields.io/badge/license-GPL--3.0-blue
[license-link]: LICENSE
[platforms-shield]: https://img.shields.io/badge/platforms-Windows%20%C2%B7%20macOS%20%C2%B7%20Linux-lightgrey
