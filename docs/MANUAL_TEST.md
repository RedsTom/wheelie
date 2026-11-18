# Manual verification checklist

Physical knob verification cannot run in automation. Follow this checklist on
each target platform before a release.

## Prerequisites

- A keyboard with a knob that can emit three distinct keys (or a keyboard tool
  that maps knob left, knob right and knob press to keys such as `F13`–`F15`).
- A text editor with an empty document, and system volume audible.
- On macOS: Accessibility and Input Monitoring granted to Wheelie when prompted.
- On Linux: an X11 session. Wayland is best-effort only.

## Key assignment

- [ ] Open **Devices** and start the live check for left rotation.
- [ ] Turn the knob left: the last key received matches the key chosen for left.
- [ ] Repeat for right rotation and press.
- [ ] Assign the same key to two controls: the warning appears and the change is
      refused.
- [ ] Restart Wheelie: the three assignments are still present.

## Actions

- [ ] Assign volume up to right rotation. Turn the knob right: volume rises by
      the configured number of steps.
- [ ] Assign scroll to left rotation and turn the knob inside a long document:
      the view scrolls in the expected direction.
- [ ] Assign a keyboard shortcut (for example `Ctrl+C`) to press and verify the
      shortcut reaches the focused application.
- [ ] Assign a mouse click to press and verify the click lands under the cursor.
- [ ] Assign a media command and verify the player reacts.
- [ ] Assign an app launch and verify the application starts.

## Macros

- [ ] Create a macro with a text step, a delay and a shortcut step.
- [ ] Assign it to press and verify the whole sequence runs in the editor.
- [ ] Reorder steps, delete one, and verify the saved macro follows the change.

## Radial wheel

- [ ] Double-press the knob: the wheel opens centered on the primary screen.
- [ ] Rotate: the selection moves between left, press and right.
- [ ] Press: the selected action runs and the wheel closes.
- [ ] Reopen and press `Esc`: the wheel closes without running an action.
- [ ] Reopen and click the center with the mouse: the press action runs.
- [ ] Reopen and click outside the wheel: the wheel closes.
- [ ] Single-press the knob without a second press: the press action runs after
      the double-click window, not before.

## Tray and window

- [ ] Close the settings window: it disappears and the tray icon remains (on
      Linux, note whether a tray icon is available; if not, check the log).
- [ ] Tray **Show**: the window returns with focus.
- [ ] Tray **Pause**: knob input stops running actions. **Resume** restores it.
- [ ] Tray **Quit**: Wheelie exits completely.
- [ ] Start Wheelie twice: the second process exits immediately.

## Persistence and appearance

- [ ] Switch language to English, restart: the interface stays in English.
- [ ] Switch the theme to light, restart: the light theme is kept.
- [ ] Enable start at login, restart the session: Wheelie starts automatically.
- [ ] Corrupt `config.toml` by hand, start Wheelie: it starts with defaults, keeps
      `config.toml.bak`, and logs the reason.
