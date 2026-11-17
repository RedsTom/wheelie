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

## M1 — Pure domain

- [ ] Versioned configuration schema with serde, defaults and validation.
- [ ] `KeyCode`, `Modifiers` and `KeyChord` with stable serde representation.
- [ ] Action model and action catalog.
- [ ] Macro model, validation and runner abstraction.
- [ ] Gesture detector: debounce, single click, double click, rotation.
- [ ] Profile model with key-based matching.
- [ ] Radial wheel geometry and hit testing.
- [ ] FR/EN translation completeness tests.
- [ ] Unit tests for every domain rule.

## M2 — Platform adapters

- [ ] `global-hotkey` registration and unregistration.
- [ ] `enigo` input sink: volume, scroll, shortcuts, mouse clicks, text.
- [ ] App launch action.
- [ ] Autostart adapter.
- [ ] Single-instance adapter.
- [ ] Injection guard to prevent hotkey feedback loops.
- [ ] Mock hotkey and input adapters for tests.

## M3 — UI

- [ ] Settings: General, Devices, Actions, Macros, About.
- [ ] Knob key picker with live test button.
- [ ] Radial overlay centered on the primary screen.
- [ ] Knob and mouse navigation in the overlay.
- [ ] Tray icon and menu.
- [ ] Hide settings to tray and reopen from tray.
- [ ] Theme selection.

## M4 — OS hardening

- [ ] macOS Accessibility/Input Monitoring onboarding.
- [ ] Linux X11 verification and Wayland warning.
- [ ] Windows elevated-app limitation messaging.
- [ ] CI green on Windows, macOS and Ubuntu.

## M5 — Validation

- [ ] Manual test with a real knob.
- [ ] Verify volume, scroll, shortcuts, mouse and macros in Notepad/TextEdit.
- [ ] Verify persistence across restarts.
- [ ] Verify single-instance behavior.
- [ ] Update user documentation.

## M6 — Publication

- [ ] Bump to version 0.1.0.
- [ ] Final commit and push.
- [ ] Optional tag `v0.1.0` (no automated release artifacts for now).

## Known v1 limitations

- Profiles are identified by trigger keys, not hardware device IDs.
- Key assignment uses a picker + test button; automatic capture is phase 2.
- Native Wayland global capture is not guaranteed.
- macOS media keys may require extra permissions.
- Windows elevated windows may ignore injected input.
