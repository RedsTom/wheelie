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
- [x] Bundled Rubik font with OFL license.
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

## M6 — Publication (historical)

- [x] Version 0.1.3 (transparent wheel in the root window, wheel preview in the
  actions page, movable sets, "No action" set).
- [x] Startup release notice: best-effort read of the published GitHub
  versions, notice card linking to the page of a newer stable release,
  ignorable per version and switchable off (v0.3.2).
- [x] Tags `v0.1.0`, `v0.1.1`, `v0.1.2` and `v0.1.3` (early tags without artifacts).

## 0.4.x — Shipped releases

- [x] v0.4.0 — gpui-only shell, pure domain, platform adapters, runtime wiring,
  persistence and tests green on Windows, macOS and Ubuntu.
- [x] v0.4.1 — light theme fixed, eframe/egui removed, CI and CD with eight
  release assets per published tag.

## UI toolkit: gpui migration (F0–F4)

- [x] F0 — spike (`examples/gpui_spike.rs`): transparent wheel window, `canvas`
  sector painting, tray + global-hotkey on the gpui main thread, close-to-tray
  with `QuitMode::Explicit`.
- [x] F1 — gpui shell: settings window lifecycle, sidebar navigation, poll tick,
  Lucide icons, Rubik fonts and the visual tokens in `src/ui/style.rs`.
- [x] F2 — shared controls in `src/ui/controls.rs` (cards, rows, pills, selects,
  switches, fields) on top of gpui-kit.
- [x] F3 — windows: radial overlay, volume/media indicator (OSD) and the wheel
  painter moved to gpui.
- [x] F4 — pages ported (Overview, Devices, Actions, Macros, Settings) with
  their rules and tests carried over.
- [x] F4 cleanup (v0.4.1): eframe, egui and egui-phosphor removed from the
  dependency tree; the legacy eframe shell, theme, widgets, icons, overlay and
  pages deleted; wheel state moved to `src/ui/wheel.rs`, navigation state to
  `src/ui/nav.rs`.
- [x] v0.4.1 — light theme fixed: gpui-kit's theme is re-synced whenever the
  mode changes, `Label` colors are set explicitly, disabled surfaces and fields
  use token-derived colors, and `src/ui/style.rs` tests guard text contrast in
  both palettes.

## Code review 0.4.1 — action plan (P0–P2)

This checklist tracks the 132 findings from the full code review. P0 must land
before the next release; P1 is the following iteration; P2 is cleanup debt.

### P0 — before the next release

- [x] Align the MSRV truth: `AGENTS.md` and `README.md` say Rust 1.98 (required by
  `gpui-icons 0.7`), pin `rust-toolchain.toml` to 1.98 and add a CI `+1.98 --locked` job.
- [x] Make the injection guard reliable: time-based quiet window, drain of pending
  hotkey events, scoped rollback of injected modifiers, and a post-scope test.
- [x] Balance macro `Press`/`Release` and release held chords on every exit path;
  roll back pressed modifiers when `enigo` injection fails.
- [x] Secure the CD workflow: checkout the supplied tag, verify the tag is on
  `main`, least-privilege permissions, pin actions/downloads, and assert the eight
  release assets exist before upload.
- [x] Make config writes durable: `sync_all` on file and parent, unique temp name,
  timestamped backups, cleanup on failure.
- [x] Move macros off the action queue and make cancellation reachable from the
  runtime, so rotation stays immediate.
- [x] Fix UI blockers: single-set wheel ring, mouse/keyboard navigation in the
  overlay, and replace window-opening `expect` calls with logged retries.
- [x] Reconcile brightness documentation with the three-platform implementation.

### P1 — next iteration

- [x] Replace platform types in `app::Indicator` with app DTOs; move autostart
  behind a platform façade and return `Result` to the UI.
- [x] Move OS wiring out of `ui/services.rs` and business rules out of
  `ui/gestures.rs` and the settings pages into `app`/`core` via typed intents.
- [x] Centralize every color in `ui/style.rs`, add opaque `on_surface` contrast
  helpers and extend palette/contrast tests.
- [x] Validate stale UI selections and cancel key capture on outside clicks.
- [x] Ship genuinely portable Linux assets (or document system dependencies) and
  generate a portable `.desktop` file with an absolute `Exec` path.
- [x] Add `tests/smoke.rs` and a packaging validation workflow; add Dependabot,
  `cargo audit`/`cargo deny` and workflow concurrency groups.
- [x] Unify error representations and mark `PlatformError` `#[non_exhaustive]`.

### P2 — cleanup debt

- [x] Centralize duplicated constants (350/20, scroll limit, max scroll amount).
- [x] Fix the misleading config-upgrade test, remove dead `PageState` fields and
  the empty `[dev-dependencies]` table.
- [x] Finish i18n hardening: default profile name via `TextKey`, word-boundary
  vocabulary blocklist, one French term for "Settings".
- [x] Add raw TOML fixtures per config version and document the policy for closed
  enums facing future documents.
- [x] Prepare `device_id: Option<String>` for phase 2 hardware IDs and document
  the wheel angle convention and cooperative macro contract.
- [x] Archive the gpui spike, remove the empty `decor_probe.log`, and clean stale
  roadmap/packaging comments.

## Manual-test follow-up (v0.4.1)

- [x] Wheel selection no longer logs a reentrant `App` borrow: the native overlay
  hide is posted with `ShowWindowAsync`, so Windows cannot re-activate the
  settings window inside gpui's update cycle.

## Known v1 limitations

- Profiles are identified by trigger keys, not hardware device IDs.
- Key assignment uses a picker + test button; automatic capture is phase 2.
- Native Wayland global capture is not guaranteed.
- The now-playing popup (title, artist and cover after a media command) reads
  the Windows system media session (SMTC); macOS and Linux have no equivalent
  unified session yet, so the popup stays silent there. It also needs the
  Windows 11 now-playing API and falls back to no popup when a player
  publishes no metadata.
- macOS media keys may require extra permissions.
- Windows elevated windows may ignore injected input.
- Brightness is available on all three platforms: `enigo` brightness keys on
  macOS (no level is reported), WMI `WmiMonitorBrightnessMethods` on Windows
  (laptop panels only) and `brightnessctl` on Linux. A display without brightness
  control reports an error when the action runs.
- The tray icon is best-effort on Linux: it needs a GTK main loop that the
  window toolkit does not provide. When creation fails, Wheelie logs the reason
  and runs without a tray icon.
- Only one wheel profile is active at a time; the profile list UI lands with
  hardware identification in phase 2.
