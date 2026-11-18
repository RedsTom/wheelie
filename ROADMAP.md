# Wheelie roadmap

This file is the live task list for the project. Keep checkboxes up to date.

## Scope

Wheelie is a standalone, cross-platform knob utility that recreates the Windows
Wheel UI/UX. It is not a Windows Wheel emulator. The user configures the keys for
knob left/right/press in their keyboard software, then assigns them in Wheelie.

## M0 — Foundations

- [x] Initialize Git repository and ignore rules.
- [x] Create the single-crate module layout (`src/core`, `src/platform`,
  `src/app`, `src/ui`).
- [x] Add GPL-3.0 license.
- [x] Add README, AGENTS.md and ROADMAP.md.
- [x] Add three-OS GitHub Actions CI.
- [x] Verify `cargo fmt`, `cargo clippy` and `cargo test` locally.
- [x] Commit and push the initial repository to `RedsTom/wheelie`.

## UI foundation (pre-M1)

- [x] Fluent-inspired dark and light themes.
- [x] Bundled Inter font with OFL license.
- [x] Full-height sidebar, cards, large actions and status pills.
- [x] User-facing FR/EN copy with a developer-vocabulary blocklist test.
- [x] Empty states for Devices, Actions and Macros.
- [x] Provided logo used for the window icon and in-app branding.

## M1 - Pure domain

- [x] Versioned configuration schema with serde, defaults and validation.
- [x] `KeyCode`, `Modifiers` and `KeyChord` with stable serde representation.
- [x] Action model and action catalog.
- [x] Macro model, validation and runner abstraction.
- [x] Gesture detector: debounce, single click, double click, rotation.
- [x] Profile model with key-based matching.
- [x] Radial wheel geometry and hit testing.
- [x] FR/EN translation completeness tests.
- [x] Unit tests for every domain rule.

## M2 - Platform adapters

- [x] `global-hotkey` registration and unregistration.
- [x] `enigo` input sink: volume, scroll, shortcuts, mouse clicks, text.
- [x] App launch action.
- [x] Autostart adapter.
- [x] Single-instance adapter.
- [x] Injection guard to prevent hotkey feedback loops.
- [x] Mock hotkey and input adapters for tests.
- [x] Platform-specific key availability handling for `enigo`.

## M3 - UI

- [x] Wire the runtime to config, hotkeys, the gesture detector and the action worker.
- [x] Settings: General, Devices, Actions, Macros, About.
- [x] Action sets with names and icons, selected from the wheel (volume with mute
  on press, brightness where supported, vertical and horizontal scroll, media
  transport by default).
- [x] Knob key picker with live test button.
- [x] Radial overlay centered on the primary screen.
- [x] Knob and mouse navigation in the overlay.
- [x] Tray icon and menu with only the relevant show/hide and pause/resume actions.
- [x] Hide settings to tray and reopen from tray.
- [x] Theme selection.
- [x] First-launch setup that sets the language and knob keys, then explains the
  wheel.

## M4 — OS hardening

- [x] macOS Accessibility/Input Monitoring onboarding.
- [x] Linux X11 verification and Wayland warning.
- [x] Windows elevated-app limitation messaging.
- [x] CI green on Windows, macOS and Ubuntu.

## M5 — Validation

- [x] Manual knob checklist documented in [`docs/MANUAL_TEST.md`](docs/MANUAL_TEST.md).
- [x] Automated coverage for volume, scroll, shortcuts, mouse and macros using the
  recording input double.
- [x] Persistence across restarts covered by tests.
- [x] Single-instance behavior covered by tests.
- [x] Update user documentation.

## M6 — Publication

- [x] Version 0.1.3 (transparent wheel in the root window, wheel preview in the
  actions page, movable sets, "No action" set).
- [x] Final commit and push.
- [x] Tags `v0.1.0`, `v0.1.1`, `v0.1.2` and `v0.1.3` (no release artifacts for now).

## Known v1 limitations

- Profiles are identified by trigger keys, not hardware device IDs.
- Key assignment uses a picker + test button; automatic capture is phase 2.
- Native Wayland global capture is not guaranteed.
- macOS media keys may require extra permissions.
- Windows elevated windows may ignore injected input.
- The brightness action relies on the brightness keys exposed by `enigo` 0.6.1,
  which only exist on macOS. On Windows and Linux the action reports that it is
  unavailable instead of guessing at vendor APIs.
- The tray icon is best-effort on Linux: it needs a GTK main loop that the
  window toolkit does not provide. When creation fails, Wheelie logs the reason
  and runs without a tray icon.
- Only one wheel profile is active at a time; the profile list UI lands with
  hardware identification in phase 2.
