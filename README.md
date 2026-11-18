# Wheelie

<img src="assets/logo/logo-2x.png" alt="Wheelie logo" width="96">

Cross-platform knob utility written in Rust. Wheelie recreates the **UI/UX of the
Windows Wheel** — a radial menu driven by a keyboard knob — without emulating the
Windows Wheel itself. You assign the keys your keyboard software emits for knob
left, knob right and knob press; Wheelie captures them exclusively and runs its own
actions.

> **Status: v0.1.0.** The full application is implemented: runtime wiring,
> configuration persistence, settings pages, radial overlay, tray icon, platform
> notices and tests. Progress is tracked in [`ROADMAP.md`](ROADMAP.md); physical
> knob verification follows the checklist in [`docs/MANUAL_TEST.md`](docs/MANUAL_TEST.md).

Wheelie is not affiliated with Microsoft.

## Features

- Knob left/right/press key assignment with a live test button.
- Exclusive global capture of the configured keys.
- Radial overlay centered on the primary screen, navigated with the knob or the
  mouse. Each sector is an action set with its own icon; confirming one makes it
  the active set.
- Action sets: volume, brightness (where the system accepts it), vertical and
  horizontal scroll by default, and as many more as you like, each with a name,
  a Phosphor icon and three actions.
- Actions: volume, mute, brightness (macOS), scroll, macros, keyboard shortcuts,
  mouse clicks, media keys and app launching. Brightness is only offered when the
  input backend supports it.
- Macro editor with key, text, delay, mouse click and scroll steps.
- Tray icon with only the relevant show/hide and pause/resume actions, plus quit;
  closing settings keeps Wheelie running in the tray.
- Optional start at login.
- French and English user interface, dark and light themes.
- First-launch setup that picks the language, assigns the knob keys and explains
  the wheel.
- Windows, macOS and Linux X11. Wayland is best-effort and clearly warned about.

## Platform support

| Platform | Global capture | Status |
| --- | --- | --- |
| Windows 10/11 | Exclusive global hotkeys | Supported |
| macOS | Global hotkeys; Accessibility/Input Monitoring may be required for media keys | Supported with permissions |
| Linux X11 | Exclusive X11 grabs | Supported |
| Linux Wayland | Best-effort; native global capture is not generally available | Documented limitation |

The tray icon is best-effort on Linux: it needs a GTK main loop on the window
thread, which the UI toolkit does not provide. When creation fails, Wheelie logs
the reason and keeps running without a tray icon.

## Stack

- Rust 1.89, edition 2024.
- `eframe`/`egui` for the settings window and radial overlay.
- `global-hotkey` for exclusive global hotkeys.
- `enigo` for cross-platform input simulation.
- `tray-icon` for the system tray.
- `egui-phosphor` for the icon set.
- `serde` + `toml` for configuration.
- Inter font (OFL-1.1), bundled in `assets/fonts`.
- No Tauri, WebView, async runtime or kernel driver.

## Architecture

```
src/main.rs       composition root
src/lib.rs        module root
src/core/         pure domain and translations
src/platform/     operating-system adapters
src/app/          runtime orchestration
src/ui/           eframe views and widgets
```

Dependency direction: `core → platform → app → ui`. The root binary only wires
dependencies. See [`AGENTS.md`](AGENTS.md) for the constraints agents must follow.

## Using Wheelie

On a fresh install, a short setup opens first: it asks for the interface
language, assigns the three knob keys, then explains how to use the wheel. It
appears only once; later launches go straight to the settings.

1. Open **Devices** and pick the keys your keyboard software sends for knob left,
   knob right and knob press. The three keys must be different; the live check
   shows the last key received.
2. Open **Actions** to create action sets. The wheel preview on the left selects a
   set; arrow buttons move it around the wheel and the right side edits its name,
   icon and three actions. A "No action" set ships by default so the knob can be
   parked without side effects.
3. Open **Macros** to build reusable sequences of keys, text, delays, scrolls and
   mouse clicks.
4. Press the knob twice to open the wheel. Rotate to move between sets, press to
   make one active, or click its sector. The wheel closes with `Esc`, a click
   outside it, or as soon as another window takes focus. Outside the wheel,
   rotation and a single press run the actions of the active set; a single press
   waits for the double-click window (350 ms by default).

Closing the settings window hides Wheelie to the tray. The tray menu shows only
the action that applies: **Hide** while the window is open, **Show** while it is
closed, and **Pause** or **Resume** depending on the current state. **Quit** is
always available. Only one Wheelie instance runs at a time.

## Development

```bash
cargo run
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

The CI workflow builds and tests on Windows, macOS and Ubuntu. Linux requires the
GTK/X11/Wayland development packages listed in `.github/workflows/ci.yml`.

## Configuration

| Platform | Path |
| --- | --- |
| Windows | `%APPDATA%\Wheelie\config.toml` |
| Linux | `~/.config/wheelie/config.toml` |
| macOS | `~/Library/Application Support/Wheelie/config.toml` |

Logs are written to the platform data directory under `Wheelie/logs/`. The
configuration is written atomically; a file that cannot be read is kept as
`config.toml.bak` and Wheelie starts from defaults. The schema is version 2:
version 1 documents are upgraded in place and unknown fields are ignored so older
documents keep working.

## Verification

Automated tests cover the domain rules, configuration persistence, the action
worker and platform adapters with mocks. Physical knob verification is manual:
follow [`docs/MANUAL_TEST.md`](docs/MANUAL_TEST.md).

## License

GPL-3.0-only. See [`LICENSE`](LICENSE).
