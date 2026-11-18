# AGENTS.md — Wheelie

## What this is

Wheelie is a cross-platform Rust desktop utility that turns a keyboard knob into a
configurable radial wheel. It recreates the **Windows Wheel UI/UX** (radial menu,
knob-driven selection); it is **not** a Windows Wheel emulator and does not install
a virtual HID device or a kernel driver.

The user tells Wheelie which keys their keyboard software emits for knob
left/right/press. Wheelie captures those keys exclusively and runs its own actions.

## Status

v0.1.0 is complete: foundations, UI shell, pure domain, platform adapters
(global-hotkey, enigo, autostart, single instance, tray), runtime wiring,
configuration persistence, settings pages, radial overlay and tests are green on
Windows, macOS and Ubuntu. `ROADMAP.md` is the authoritative checklist; update it
when a milestone item lands, and keep `docs/MANUAL_TEST.md` current for checks that
need a physical knob.

## Architecture

One package, layered modules with strict dependency direction:

```
src/main.rs       composition root (CLI, logging, runtime, eframe run)
src/lib.rs        module root
src/assets.rs     embedded logo and icon loading
src/core/         pure domain: config, keys, actions, macros, gestures,
                  profiles, i18n, wheel geometry
src/platform/     OS adapters: hotkeys, input simulation, autostart, launch,
                  instance, paths
src/app/          orchestration: config store, runtime, action worker, tray commands
src/ui/           eframe only: settings pages, radial overlay, theme, widgets
```

The brand mark lives in `assets/logo/` and is the single source of truth: use
`logo-4x.png` for in-app textures, `logo-2x.png` for window and tray icons, and
`logo.svg` for documentation. Never redraw the mark in code. The accent color
`#4B8BFF` in `src/ui/theme.rs` must stay in sync with the logo.

Rules:

- `core` has no UI, OS or filesystem dependencies; it does not spawn threads or
  use channels. The macro runner only sleeps between steps.
- `platform` is the only module allowed to call OS APIs.
- `app` contains business orchestration and no eframe.
- `ui` contains no business rules and no OS calls.
- `src/main.rs` only wires modules; no layout code.

## Commands

```bash
cargo run
cargo test
cargo test core::i18n
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

CI runs fmt on Linux, and clippy + tests on Windows, macOS and Ubuntu.

## Locked decisions

- Rust 1.89, edition 2024.
- `eframe`/`egui` 0.33.3. Do not bump to 0.37 until the toolchain is Rust 1.95.
- `egui-phosphor` 0.11 for action-set icons; later versions require newer egui.
- `tray-icon` 0.24.2. Do not bump to 0.26 until Rust 1.90.
- `global-hotkey` 0.10 for exclusive global hotkeys.
- `enigo` 0.6.1 for cross-platform volume/scroll/key/mouse/text injection.
- TOML config, versioned, atomic writes.
- UI FR/EN from v1. All UI prose goes through `wheelie_core::i18n`.
- GPL-3.0-only, public repo `RedsTom/wheelie`.
- Tray + optional autostart; closing settings hides to tray.
- v1 profiles are keyed by trigger keys; two devices emitting identical keys are
  indistinguishable. Hardware IDs are phase 2.
- v1 key assignment is a picker + test button; automatic capture is phase 2.

## Behavior contract

- Double-click knob opens the radial overlay centered on the primary screen, not
  the cursor.
- A single click is delayed by 350 ms so a double-click cannot also fire the
  single-click action.
- Rotation is immediate with debounce.
- The overlay shows one sector per action set with its icon. Confirming a sector
  makes that set the active one; the wheel never runs an action itself. Knob or
  mouse navigation, Esc closes.
- Outside the wheel, the three controls run the actions of the active set.
- Action sets: default volume, brightness and scroll sets; users add sets, name
  them and pick an icon. Actions: volume, brightness, scroll, macro, shortcut,
  mouse click, media, launch app.
- Macros: typed steps (key/tap, text, delay, mouse click, scroll); no script DSL.
- No arbitrary shell execution.

## Platform gotchas

- `global-hotkey` is exclusive but X11-only on Linux. Native Wayland cannot be
  captured by a normal app; show a warning and treat it as best-effort.
- macOS may require Accessibility/Input Monitoring for media keys (CGEvent tap).
  Provide onboarding, never assume permission.
- Windows UIPI: a non-elevated process cannot send input to elevated windows.
  Document it; optionally offer relaunch-as-admin.
- Never feed injected events back into the hotkey engine. The action worker must
  set an `injecting` guard and the runtime must drain/ignore hotkey events while
  injection is active.
- The global key service and the tray icon must be created on the thread that runs
  the window event loop, lazily from the first `update()`. On Linux the tray needs
  a GTK main loop that eframe does not provide: attempt creation, log failures and
  continue without a tray icon, never panic.
- The wheel must render in the root window, not in a secondary egui viewport:
  secondary viewports composite onto an opaque background with the wgpu backend on
  Windows (black or white rectangle). The shell borrows the root window for the
  wheel (undecorated, always on top, resized and recentred) and restores the
  settings geometry afterwards. Keep the app transparent in `NativeOptions` and
  return an alpha-0 `clear_color`.
- Volume via `enigo` media keys; scroll via `enigo` wheel. Brightness uses
  `enigo` brightness keys, which only exist on macOS in 0.6.1; report the action
  as unavailable elsewhere instead of adding platform audio/display code.
- Do not add platform-specific audio code unless `enigo` proves insufficient.
- The app must stay usable with no knob configured: settings and tray still work.

## Conventions

- Code, comments, docs, error messages, log messages and commit messages are
  **English**. French exists only in translation rows.
- Never expose developer vocabulary in the UI. This includes milestone IDs
  (M0–M6), module or layer names (`core`, `platform`, `app`, `ui`), and terms such
  as runtime, hotkey, config, capture, architecture, roadmap, workspace, test or
  simulation. UI copy describes what the user can do. Keep developer language in
  code, commits, docs and this file.
- UI strings only via `TextKey` + `TRANSLATIONS`; never hardcode French or English
  prose in UI modules. The `translations_do_not_expose_developer_vocabulary` test
  enforces a blocklist; extend it when a new internal term appears.
- The UI bundles Inter in `assets/fonts`; keep `OFL.txt` with it and prefer bundled
  assets over runtime system-font discovery.
- Keep the UI compact and professional: buttons, combo boxes and status chips share
  `theme::CONTROL_HEIGHT` (currently 32 px) and small radii. Status chips are
  disabled `egui::Button`s with an explicit fill; do not hand-roll frames with a
  different height. Avoid decorative glows, gradients or oversized cards.
- Error handling: `thiserror` in libraries, `anyhow` only in the composition root.
- No Tokio, Tauri, Electron, WebView, kernel driver or virtual HID.
- Keep `unsafe` out of the codebase unless a platform adapter cannot be implemented
  otherwise; isolate and document it.
- Add a test for every new domain rule; platform tests must not require a real knob.
- Commits: Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`).

## Config and data

- Windows: `%APPDATA%\Wheelie\config.toml`
- Linux: `~/.config/wheelie/config.toml`
- macOS: `~/Library/Application Support/Wheelie/config.toml`
- Logs: platform data dir `.../Wheelie/logs/wheelie.log`
- The schema is versioned; unknown fields must not silently break old configs.

## References

- `ROADMAP.md`: authoritative M0–M6 checklist.
- `README.md`: public overview.
- `.github/workflows/ci.yml`: exact CI matrix and Linux dependencies.
