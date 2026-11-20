# Manual verification checklist

Physical knob verification cannot run in automation. Follow this checklist on
each target platform before a release.

## Prerequisites

- A keyboard with a knob that can emit three distinct keys (or a keyboard tool
  that maps knob left, knob right and knob press to keys such as `F13`–`F15`).
- A text editor with an empty document, and system volume audible.
- On macOS: Accessibility and Input Monitoring granted to Wheelie when prompted.
- On Linux: an X11 session. Wayland is best-effort only.

## First launch

- [ ] Start Wheelie with no configuration file: the setup opens on the language
      page instead of the settings, with a three-step indicator at the top.
- [ ] Choose English: the remaining pages, the tray menu and the settings use
      English immediately.
- [ ] On the first page, switch start at login on and off: the choice is applied
      immediately and the settings page shows the same state afterwards.
- [ ] Assign the three knob keys from the setup; a duplicate key shows the
      warning and is refused.
- [ ] The last page explains the double click, choosing a set and running actions;
      the button invites you to try the knob now.
- [ ] Finish the setup, restart Wheelie: the settings open directly and the
      language and keys are kept.
- [ ] Start with an existing configuration file that predates the setup: the
      settings open directly.

## Key assignment

- [ ] Open **Devices** and start the live check for left rotation.
- [ ] Turn the knob left: the last key received matches the key chosen for left.
- [ ] Repeat for right rotation and press.
- [ ] Assign the same key to two controls: the warning appears and the change is
      refused.
- [ ] In a shortcut field, press Volume Up, Volume Down and Mute: each key is
      shown as the chosen key and the system volume does not change while the
      field is waiting.
- [ ] In a shortcut field, press Alt+Tab, Alt+Escape, Ctrl+Escape and a Windows
      chord such as Win+D: none of them switches windows, opens the start menu
      or shows the desktop, and each one is shown as the chosen key.
- [ ] Confirm or cancel the field: the system shortcuts work again right away,
      including in other applications.
- [ ] Start a capture, then close or hide the settings window without
      confirming: the shortcuts are given back as soon as the window is gone.
- [ ] Press Escape while a field is waiting: the capture stops without waiting
      for a key. Plain Escape and Alt+F4 are never taken over, so a field can
      always be cancelled and a window closed.
- [ ] Windows lock (Win+L), the secure attention sequence (Ctrl+Alt+Del) and a
      bare modifier stay with the system on purpose: pressing them during a
      capture does nothing to Wheelie.
- [ ] When another tool already owns one of these shortcuts, the field still
      records every other key; the log names the keys Wheelie could not take
      over.
- [ ] Restart Wheelie: the three assignments are still present.

## Action sets

- [ ] Open **Actions**: the volume, vertical scroll, horizontal scroll and media
      sets are listed, each with an icon. On macOS the brightness set is listed
      too; on Windows and Linux it is absent and brightness is not offered in the
      picker.
- [ ] Create a set, rename it, pick another icon and assign actions.
- [ ] Delete a set: it disappears from the wheel, and the active set moves to the
      first remaining set when it was active.

## Actions

- [ ] Make the volume set active from the wheel. Turn the knob right: volume
      rises by the configured number of steps and the indicator shows the new
      percentage.
- [ ] With the volume set active, press the knob: sound mutes, the indicator
      keeps the stored percentage and switches to the muted speaker icon. Press
      again to restore sound.
- [ ] Make the media set active: left goes to the previous track, right to the
      next track and press toggles play/pause.
- [ ] On Windows, with a player open (for example Groove Music or a browser
      tab playing audio): every media command shows a card at the bottom of
      the screen with the track title, the artist, the cover, a play or pause
      glyph matching the session and, when the player reports a timeline, a
      progress bar with the elapsed and total time. While the card is on
      screen and playback continues, the elapsed time and the bar advance
      with the track. With nothing playing, the card says the equivalent of
      "Nothing is playing" in the interface language. The card closes by
      itself after about five seconds and never steals focus from the
      foreground window.
- [ ] Make the scroll set active and turn the knob inside a long document: the
      view scrolls in the expected direction.
- [ ] On macOS, make the brightness set active and verify brightness changes. On
      Windows and Linux, check that a clear unavailable message is reported.
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

- [ ] Double-press the knob: the wheel opens centered on the primary screen, with
      one icon sector per action set.
- [ ] Rotate: the selection moves between sets.
- [ ] Press: the selected set becomes active and the wheel closes. No action runs.
- [ ] Reopen and click a sector with the mouse: that set becomes active.
- [ ] Reopen and press `Esc`: the wheel closes without changing the active set.
- [ ] Reopen and focus another window: the wheel closes by itself.
- [ ] With a set active, rotate outside the wheel: the set's left/right actions run.
- [ ] Single-press the knob without a second press: the press action of the
      active set runs after the double-click window, not before.

## Tray and window

- [ ] With the settings open, the tray menu offers **Hide**, one pause action
      and **Quit**: no **Show** entry and no paired pause action.
- [ ] Close the settings window: it disappears and the tray icon remains (on
      Linux, note whether a tray icon is available; if not, check the log). The
      menu now offers **Show** instead of **Hide**.
- [ ] Tray **Show**: the window returns with focus and the menu switches back.
- [ ] Tray **Pause**: knob input stops running actions. The menu now offers
      **Resume** only; **Resume** restores it.
- [ ] Tray **Quit**: Wheelie exits completely.
- [ ] Start Wheelie twice: the second process exits immediately.

## Persistence and appearance

- [ ] Switch language to English, restart: the interface stays in English.
- [ ] Switch the theme to light: page titles, card headings, key buttons,
      dropdown items and disabled controls stay readable immediately, without a
      restart. Switch back to dark and check the same pages.
- [ ] Switch the theme to light, restart: the light theme is kept.
- [ ] Enable start at login, restart the session: Wheelie starts automatically.
- [ ] Corrupt `config.toml` by hand, start Wheelie: it starts with defaults, keeps
      `config.toml.bak`, and logs the reason.

## Release notice

- [ ] Run a build older than the newest stable published release: a notice card
      opens over the settings, naming the version and pointing to its GitHub
      page. It never appears during the first-launch setup.
- [ ] Open the release page: the browser shows the GitHub release; the card is
      gone for this run and comes back after a restart.
- [ ] Ignore the version, restart: the notice stays hidden and
      `ignored_update` is recorded in `config.toml`.
- [ ] Turn **New version notifications** off, restart: no request goes to
      GitHub and no notice appears.
- [ ] Disconnect the network, start Wheelie: no notice, no error dialog; the
      log records that the release feed was unavailable and everything else
      works.
