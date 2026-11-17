# Wheelie

Cross-platform knob utility written in Rust. Wheelie recreates the **UI/UX of the
Windows Wheel** — a radial menu driven by a keyboard knob — without emulating the
Windows Wheel itself. You assign the keys your keyboard software emits for knob
left, knob right and knob press; Wheelie captures them exclusively and runs its own
actions.

> **Status: M0 foundations complete.** The workspace, CI, license, documentation
> and application shell are in place. M1 (pure domain) is next; progress is tracked
> in [`ROADMAP.md`](ROADMAP.md).

Wheelie is not affiliated with Microsoft.

## Planned features

- Knob left/right/press key assignment with a live test button.
- Exclusive global capture of the configured keys.
- Radial overlay centered on the primary screen.
- Actions: volume, scroll, macros, keyboard shortcuts, mouse clicks, app launching.
- Macro editor with key, text, delay, mouse click and scroll steps.
- Tray icon with pause/resume, show wheel, settings and quit.
- Optional start at login.
- French and English user interface.
- Windows, macOS and Linux X11. Wayland is best-effort and clearly warned about.

## Platform support

| Platform | Global capture | Status |
| --- | --- | --- |
| Windows 10/11 | Exclusive global hotkeys | Primary target |
| macOS | Global hotkeys; Accessibility/Input Monitoring may be required for media keys | Planned |
| Linux X11 | Exclusive X11 grabs | Planned |
| Linux Wayland | Best-effort; native global capture is not generally available | Documented limitation |

## Stack

- Rust 1.89, edition 2024.
- `eframe`/`egui` for the settings window and radial overlay.
- `global-hotkey` for exclusive global hotkeys.
- `enigo` for cross-platform input simulation.
- `tray-icon` for the system tray.
- `serde` + `toml` for configuration.
- No Tauri, WebView, async runtime or kernel driver.

## Architecture

```
src/                      composition root
crates/wheelie-core/      pure domain and translations
crates/wheelie-platform/  operating-system adapters
crates/wheelie-app/       runtime orchestration
crates/wheelie-ui/        eframe views and widgets
```

Dependency direction: `core → platform → app → ui`. The root binary only wires
dependencies. See [`AGENTS.md`](AGENTS.md) for the constraints agents must follow.

## Development

```bash
cargo run
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
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

Logs are written to the platform data directory under `Wheelie/logs/`.

## License

GPL-3.0-only. See [`LICENSE`](LICENSE).
