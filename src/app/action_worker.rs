//! Background worker that executes knob actions.
//!
//! The worker owns the input sink and the injection guard so the UI thread
//! never blocks on synthesized input. Every job runs inside an injection
//! scope, which the runtime uses to discard hotkey events produced by the
//! injected keys themselves. A media transport command also hands the card
//! to a short-lived watcher thread, so following the player never stalls
//! the action queue.

use super::error::AppError;
use super::runtime::{AppEvent, Indicator};
use crate::core::action::{Action, MediaCommand};
use crate::core::macro_engine::{MacroDefinition, MacroSink, run_macro};
use crate::core::{Config, InputEvent, TextKey, tr};
use crate::platform::media_session;
use crate::platform::volume;
use crate::platform::{EnigoInputSink, InjectionGuard, InputSink};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tracing::{error, warn};

/// Job accepted by the action worker.
#[derive(Debug)]
pub enum ActionJob {
    /// Execute one action.
    Execute(Action),
    /// Stop the worker.
    Quit,
}

/// Cloneable producer side of the worker queue.
#[derive(Clone)]
pub struct ActionQueue {
    sender: Sender<ActionJob>,
}

impl ActionQueue {
    /// Queues an action for execution.
    pub fn execute(&self, action: Action) -> Result<(), AppError> {
        self.sender
            .send(ActionJob::Execute(action))
            .map_err(|_| AppError::RuntimeStopped)
    }

    /// Stops the worker.
    pub fn shutdown(&self) {
        let _ = self.sender.send(ActionJob::Quit);
    }
}

/// Spawns the action worker and returns its queue and join handle.
pub fn spawn(
    config: Arc<RwLock<Config>>,
    guard: InjectionGuard,
    events: Sender<AppEvent>,
) -> Result<(ActionQueue, JoinHandle<()>), AppError> {
    let (sender, receiver) = unbounded();
    let worker = thread::Builder::new()
        .name("wheelie-actions".to_owned())
        .spawn(move || run(&receiver, &config, &guard, &events))?;
    Ok((ActionQueue { sender }, worker))
}

fn run(
    jobs: &Receiver<ActionJob>,
    config: &Arc<RwLock<Config>>,
    guard: &InjectionGuard,
    events: &Sender<AppEvent>,
) {
    let mut sink = match EnigoInputSink::new() {
        Ok(sink) => sink,
        Err(error) => {
            error!(%error, "could not create the input sink");
            let _ = events.send(AppEvent::Error(error.to_string()));
            return;
        }
    };

    let cancel = AtomicBool::new(false);
    let mut session = media_session::Reader::new();
    let generation = Arc::new(AtomicU64::new(0));

    while let Ok(job) = jobs.recv() {
        match job {
            ActionJob::Quit => break,
            ActionJob::Execute(action) => {
                let macros = read_config(config).macros.clone();
                cancel.store(false, Ordering::Relaxed);
                // A transport command changes the session asynchronously; the
                // previous state is the baseline the indicator polls against.
                let before = match &action {
                    Action::Media { .. } if media_session::SUPPORTED => Some(session.read()),
                    _ => None,
                };
                match execute_action(&mut sink, guard, &macros, &action, &cancel) {
                    Err(message) => {
                        warn!(%message, "action failed");
                        let _ = events.send(AppEvent::Error(message));
                    }
                    Ok(reported) => {
                        let indicator = match (&action, before) {
                            (Action::Volume { .. } | Action::Mute, _) => {
                                volume::level().ok().map(Indicator::Volume)
                            }
                            (Action::Brightness { .. }, _) => reported.map(Indicator::Brightness),
                            (Action::Media { command }, Some(before)) => {
                                // Watching the session stays off this thread:
                                // the knob queue must keep moving while the
                                // media card follows the player.
                                spawn_media_watch(&generation, *command, before, config, events);
                                None
                            }
                            _ => None,
                        };
                        if let Some(indicator) = indicator {
                            let _ = events.send(AppEvent::Indicator(indicator));
                        }
                    }
                }
            }
        }
    }
}

/// Runs one action through `sink`, wrapped in an injection scope.
///
/// Kept generic so tests can drive it with the recording mock sink.
pub fn execute_action<S>(
    sink: &mut S,
    guard: &InjectionGuard,
    macros: &[MacroDefinition],
    action: &Action,
    cancel: &AtomicBool,
) -> Result<Option<u8>, String>
where
    S: InputSink + MacroSink,
{
    let _scope = guard.scope();

    match action {
        Action::None => Ok(None),
        Action::Volume { direction, steps } => sink
            .volume(*direction, *steps)
            .map(|()| None)
            .map_err(|error| error.to_string()),
        Action::Mute => sink
            .volume_mute()
            .map(|()| None)
            .map_err(|error| error.to_string()),
        Action::Brightness { direction, steps } => sink
            .brightness(*direction, *steps)
            .map_err(|error| error.to_string()),
        Action::Scroll { axis, amount } => InputSink::scroll(sink, *axis, *amount)
            .map(|()| None)
            .map_err(|error| error.to_string()),
        Action::Shortcut { chord } => sink
            .tap_chord(*chord)
            .map(|()| None)
            .map_err(|error| error.to_string()),
        Action::MouseClick { button } => InputSink::mouse_click(sink, *button)
            .map(|()| None)
            .map_err(|error| error.to_string()),
        Action::Media { command } => sink
            .media(*command)
            .map(|()| None)
            .map_err(|error| error.to_string()),
        Action::Launch { path } => sink
            .launch(path)
            .map(|()| None)
            .map_err(|error| error.to_string()),
        Action::Macro { name } => {
            let definition = macros
                .iter()
                .find(|definition| &definition.name == name)
                .ok_or_else(|| format!("unknown macro: {name}"))?;
            run_macro(sink, &definition.steps, cancel)
                .map(|()| None)
                .map_err(|error| error.to_string())
        }
    }
}

fn read_config(config: &RwLock<Config>) -> std::sync::RwLockReadGuard<'_, Config> {
    config
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// How long the indicator waits for the player to reflect a transport
/// command, and how often the session is re-read during that window. The
/// reader keeps the session manager alive, so a poll is two cheap calls and
/// the interval can stay short.
const MEDIA_POLL_TIMEOUT: Duration = Duration::from_millis(1200);
const MEDIA_POLL_INTERVAL: Duration = Duration::from_millis(40);
/// How long a card stays interesting after its last update: the watcher
/// keeps reporting while it is on screen, so the artwork follows the player
/// even when the change lands after the polling timeout. This mirrors the
/// UI's `MEDIA_DISPLAY_DURATION`.
const MEDIA_WATCH_DURATION: Duration = Duration::from_millis(5000);
/// Hard lifetime of one watcher, extensions included. A player that keeps
/// fluctuating can refresh a card a few times, never forever.
const MEDIA_WATCH_MAX: Duration = Duration::from_millis(12_000);

/// Spawns the watcher for one transport command. Only the newest watcher
/// emits: the shared generation lets a second press retire the previous
/// thread before it can push a stale track over the fresh card.
fn spawn_media_watch(
    generation: &Arc<AtomicU64>,
    command: MediaCommand,
    before: media_session::Reading,
    config: &Arc<RwLock<Config>>,
    events: &Sender<AppEvent>,
) {
    let own = generation.fetch_add(1, Ordering::Relaxed) + 1;
    let generation = Arc::clone(generation);
    let config = Arc::clone(config);
    let events = events.clone();
    if let Err(error) = thread::Builder::new()
        .name("wheelie-media-watch".to_owned())
        .spawn(move || watch_media(own, generation, command, before, config, events))
    {
        warn!(%error, "could not watch the media session");
    }
}

fn watch_media(
    own: u64,
    generation: Arc<AtomicU64>,
    command: MediaCommand,
    before: media_session::Reading,
    config: Arc<RwLock<Config>>,
    events: Sender<AppEvent>,
) {
    let language = read_config(&config).language;
    let mut reader = media_session::Reader::new();
    let deadline = Instant::now() + MEDIA_POLL_TIMEOUT;

    // Phase 1: wait for the command to visibly take effect, then report
    // whatever the session says (the timeout may still show the old track,
    // phase 2 replaces it as soon as the player catches up). A first
    // "changed" read is confirmed by one more, so the card does not open
    // on a half-published state (new title, cover still to come).
    let last = loop {
        if generation.load(Ordering::Relaxed) != own {
            return;
        }
        thread::sleep(MEDIA_POLL_INTERVAL);
        let reading = reader.read();
        if !session_changed(command, &before, &reading) {
            if Instant::now() >= deadline {
                break reading;
            }
            continue;
        }
        thread::sleep(MEDIA_POLL_INTERVAL);
        let confirmed = reader.read();
        if session_changed(command, &before, &confirmed) || Instant::now() >= deadline {
            break confirmed;
        }
    };
    if let Some(playing) = indicator_from(&last, language) {
        let _ = events.send(AppEvent::Indicator(playing));
    }

    // Phase 2: follow the player while the card is on screen. Only the
    // fields that define the track are compared (title, artist, cover,
    // whether a timeline exists) and a new fingerprint must be seen on two
    // consecutive reads before it is reported: transient fluctuations
    // (buffering states, a half-loaded timeline, a cover arriving one poll
    // after the title) must never re-arm the card on their own.
    let hard_deadline = Instant::now() + MEDIA_WATCH_MAX;
    let mut watch_until = Instant::now() + MEDIA_WATCH_DURATION;
    let mut watcher = Watcher {
        last: snapshot_of(&last),
        candidate: None,
    };
    while generation.load(Ordering::Relaxed) == own
        && Instant::now() < watch_until
        && Instant::now() < hard_deadline
    {
        thread::sleep(MEDIA_POLL_INTERVAL);
        let reading = reader.read();
        let Some(snapshot) = snapshot_of(&reading) else {
            continue;
        };
        if watcher.observe(snapshot) {
            if let Some(playing) = indicator_from(&reading, language) {
                let _ = events.send(AppEvent::Indicator(playing));
            }
            watch_until = Instant::now() + MEDIA_WATCH_DURATION;
        }
    }
}

/// Indicator for one reading: the track, or the fallback text when the
/// session says nothing is playing. `Unavailable` stays silent.
fn indicator_from(
    reading: &media_session::Reading,
    language: crate::core::Language,
) -> Option<Indicator> {
    let track = match reading {
        media_session::Reading::Track(track) if !track.title.is_empty() => track.clone(),
        media_session::Reading::Track(_) | media_session::Reading::NoSession => {
            media_session::NowPlaying {
                title: tr(language, TextKey::MediaNoTrack).to_owned(),
                ..Default::default()
            }
        }
        media_session::Reading::Unavailable => return None,
    };
    Some(Indicator::Media(track))
}

/// What the card is showing, reduced to the fields that define a track.
///
/// Playback state and position are deliberately absent: the card advances
/// the clock locally, and players toggle `Buffering`/`Paused` transiently
/// without changing the track.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Snapshot {
    /// The fallback text: no session, or a session without a title.
    Absent,
    /// A titled track.
    Present {
        title: String,
        artist: String,
        /// Fingerprint of the published cover, when there is one.
        cover: Option<u64>,
        /// Whether the player exposes a timeline for the progress bar.
        timed: bool,
    },
}

/// Fingerprint of the reading: `None` when the read failed, which says
/// nothing about the session and must not move the card.
fn snapshot_of(reading: &media_session::Reading) -> Option<Snapshot> {
    match reading {
        media_session::Reading::Unavailable => None,
        media_session::Reading::NoSession => Some(Snapshot::Absent),
        media_session::Reading::Track(track) if track.title.is_empty() => Some(Snapshot::Absent),
        media_session::Reading::Track(track) => Some(Snapshot::Present {
            title: track.title.clone(),
            artist: track.artist.clone(),
            cover: track.cover.as_ref().map(fingerprint),
            timed: track.duration_ms > 0,
        }),
    }
}

fn fingerprint(cover: &media_session::Cover) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (cover.format as u8, cover.bytes.as_slice()).hash(&mut hasher);
    hasher.finish()
}

/// Tracks what the card shows and requires a fingerprint to be confirmed by
/// two consecutive reads before reporting it.
struct Watcher {
    /// Fingerprint currently on screen (`None`: nothing confirmed yet).
    last: Option<Snapshot>,
    /// Fingerprint seen on the previous read, waiting for confirmation.
    candidate: Option<Snapshot>,
}

impl Watcher {
    /// Whether `snapshot` must be reported now.
    ///
    /// A cover landing on the track already shown is published on sight:
    /// players push the title first and the artwork one beat later, and
    /// waiting for a second identical read is exactly the lag the eye
    /// catches. Every other change (another track, an appearance or a
    /// disappearance, a timeline materialising) is confirmed by two
    /// consecutive identical reads to filter transient fluctuations.
    fn observe(&mut self, snapshot: Snapshot) -> bool {
        if self
            .last
            .as_ref()
            .is_some_and(|last| cover_landed(last, &snapshot))
        {
            self.last = Some(snapshot);
            self.candidate = None;
            return true;
        }
        if self.candidate.as_ref() != Some(&snapshot) {
            self.candidate = Some(snapshot);
            return false;
        }
        if self.last.as_ref() == Some(&snapshot) {
            return false;
        }
        self.last = Some(snapshot);
        self.candidate = None;
        true
    }
}

/// Whether `next` is the very track of `previous` with different artwork.
fn cover_landed(previous: &Snapshot, next: &Snapshot) -> bool {
    match (previous, next) {
        (
            Snapshot::Present {
                title: first,
                artist: first_artist,
                cover: first_cover,
                ..
            },
            Snapshot::Present {
                title,
                artist,
                cover,
                ..
            },
        ) => first == title && first_artist == artist && first_cover != cover,
        _ => false,
    }
}

/// Whether `current` reflects the effect of `command` compared to `before`.
///
/// A transport command either moves to another track or changes the playback
/// state; anything else (a slow player, the same track twice) keeps polling
/// until the timeout.
fn session_changed(
    command: MediaCommand,
    before: &media_session::Reading,
    current: &media_session::Reading,
) -> bool {
    let state = |reading: &media_session::Reading| match reading {
        media_session::Reading::Track(track) => {
            Some((track.title.clone(), track.artist.clone(), track.playing))
        }
        media_session::Reading::NoSession | media_session::Reading::Unavailable => None,
    };
    match (state(before), state(current)) {
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => true,
        (Some((was_title, was_artist, was_playing)), Some((title, artist, playing))) => {
            match command {
                MediaCommand::PlayPause => was_playing != playing,
                MediaCommand::Stop => !playing,
                MediaCommand::Next | MediaCommand::Previous => {
                    was_title != title || was_artist != artist
                }
            }
        }
    }
}

/// Drains hotkey events while injection is active.
///
/// The runtime calls this every frame: an injected key must never reach the
/// gesture detector, or an action could trigger itself.
pub fn discard_injected(events: Vec<InputEvent>, guard: &InjectionGuard) -> Vec<InputEvent> {
    if guard.is_active() {
        Vec::new()
    } else {
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::action::{MouseButton, ScrollAxis, VolumeDirection};
    use crate::core::key::{KeyChord, KeyCode};
    use crate::core::macro_engine::MacroStep;
    use crate::platform::mock::MockInputSink;

    #[test]
    fn action_worker_injects_through_the_sink_and_sets_the_guard() {
        let mut sink = MockInputSink::default();
        let guard = InjectionGuard::default();
        let cancel = AtomicBool::new(false);

        execute_action(
            &mut sink,
            &guard,
            &[],
            &Action::Volume {
                direction: VolumeDirection::Up,
                steps: 2,
            },
            &cancel,
        )
        .expect("volume");

        assert_eq!(sink.calls, vec!["volume Up 2".to_owned()]);
        assert!(!guard.is_active(), "guard must be released after the job");
    }

    #[test]
    fn every_action_variant_reaches_the_sink() {
        let mut sink = MockInputSink::default();
        let guard = InjectionGuard::default();
        let cancel = AtomicBool::new(false);
        let macros = vec![MacroDefinition {
            name: "paste".to_owned(),
            steps: vec![MacroStep::Text {
                text: "hi".to_owned(),
            }],
        }];

        let actions = [
            Action::None,
            Action::Scroll {
                axis: ScrollAxis::Vertical,
                amount: -2,
            },
            Action::Shortcut {
                chord: KeyChord::new(KeyCode::F13),
            },
            Action::MouseClick {
                button: MouseButton::Middle,
            },
            Action::Media {
                command: crate::core::action::MediaCommand::PlayPause,
            },
            Action::Macro {
                name: "paste".to_owned(),
            },
        ];

        for action in &actions {
            execute_action(&mut sink, &guard, &macros, action, &cancel).expect("action");
        }

        assert_eq!(sink.calls.len(), 5);
        assert!(sink.calls.iter().any(|call| call == "text hi"));
    }

    fn track(title: &str, artist: &str, playing: bool) -> media_session::Reading {
        media_session::Reading::Track(media_session::NowPlaying {
            title: title.to_owned(),
            artist: artist.to_owned(),
            playing,
            ..Default::default()
        })
    }

    #[test]
    fn next_only_counts_when_another_track_is_reported() {
        let before = track("A", "X", true);
        let same = track("A", "X", true);
        let moved = track("B", "X", true);

        assert!(!session_changed(MediaCommand::Next, &before, &same));
        assert!(session_changed(MediaCommand::Next, &before, &moved));
    }

    #[test]
    fn play_pause_only_counts_when_the_state_flips() {
        let before = track("A", "X", true);
        let flipped = track("A", "X", false);

        assert!(!session_changed(MediaCommand::PlayPause, &before, &before));
        assert!(session_changed(MediaCommand::PlayPause, &before, &flipped));
    }

    #[test]
    fn stop_counts_when_playback_is_not_running_anymore() {
        let before = track("A", "X", true);
        let stopped = track("A", "X", false);

        assert!(!session_changed(
            MediaCommand::Stop,
            &before,
            &track("A", "X", true)
        ));
        assert!(session_changed(MediaCommand::Stop, &before, &stopped));
        assert!(session_changed(
            MediaCommand::Stop,
            &before,
            &media_session::Reading::NoSession
        ));
    }

    #[test]
    fn a_session_that_appears_or_disappears_counts_as_changed() {
        let before = track("A", "X", true);

        assert!(session_changed(
            MediaCommand::Next,
            &before,
            &media_session::Reading::NoSession
        ));
        assert!(session_changed(
            MediaCommand::Next,
            &media_session::Reading::NoSession,
            &before
        ));
        assert!(session_changed(
            MediaCommand::Next,
            &media_session::Reading::NoSession,
            &media_session::Reading::NoSession
        ));
    }

    fn snapshot(title: &str, cover: Option<u64>, timed: bool) -> Snapshot {
        Snapshot::Present {
            title: title.to_owned(),
            artist: "X".to_owned(),
            cover,
            timed,
        }
    }

    #[test]
    fn a_steady_track_never_re_reports() {
        let steady = snapshot("A", None, true);
        let mut watcher = Watcher {
            last: Some(steady.clone()),
            candidate: None,
        };

        assert!(!watcher.observe(steady.clone()));
        assert!(!watcher.observe(steady));
    }

    #[test]
    fn a_new_track_is_reported_once_confirmed() {
        let mut watcher = Watcher {
            last: Some(snapshot("A", None, true)),
            candidate: None,
        };

        assert!(!watcher.observe(snapshot("B", None, true)));
        assert!(watcher.observe(snapshot("B", None, true)));
        assert!(!watcher.observe(snapshot("B", None, true)));
        assert!(!watcher.observe(snapshot("B", None, true)));
    }

    #[test]
    fn flapping_fingerprints_never_report() {
        let mut watcher = Watcher {
            last: Some(snapshot("A", None, false)),
            candidate: None,
        };

        for step in 0..8 {
            let timed = step % 2 == 0;
            assert!(!watcher.observe(snapshot("B", None, timed)));
        }
        assert!(watcher.observe(snapshot("B", None, false)));
        assert!(!watcher.observe(snapshot("B", None, false)));
        assert!(!watcher.observe(snapshot("B", None, false)));
    }

    #[test]
    fn a_cover_arriving_for_a_shown_track_reports_at_once() {
        let mut watcher = Watcher {
            last: Some(snapshot("A", None, true)),
            candidate: None,
        };

        assert!(watcher.observe(snapshot("A", Some(7), true)));
        assert!(!watcher.observe(snapshot("A", Some(7), true)));
    }

    #[test]
    fn a_new_track_is_not_published_on_its_cover_alone() {
        let mut watcher = Watcher {
            last: Some(snapshot("A", Some(1), true)),
            candidate: None,
        };

        assert!(!watcher.observe(snapshot("B", Some(2), true)));
        assert!(watcher.observe(snapshot("B", Some(2), true)));
    }

    #[test]
    fn a_failed_read_is_no_information() {
        assert!(snapshot_of(&media_session::Reading::Unavailable).is_none());
        assert_eq!(
            snapshot_of(&media_session::Reading::NoSession),
            Some(Snapshot::Absent)
        );
        assert_eq!(
            snapshot_of(&track("A", "X", true)),
            Some(snapshot("A", None, false))
        );
    }

    #[test]
    fn unknown_macro_is_reported() {
        let mut sink = MockInputSink::default();
        let guard = InjectionGuard::default();
        let cancel = AtomicBool::new(false);

        let error = execute_action(
            &mut sink,
            &guard,
            &[],
            &Action::Macro {
                name: "missing".to_owned(),
            },
            &cancel,
        )
        .expect_err("must fail");

        assert!(error.contains("missing"));
        assert!(!guard.is_active());
    }

    #[test]
    fn knob_gestures_drive_actions_end_to_end() {
        use crate::core::gesture::{
            Bindings, Control, GestureConfig, GestureDetector, GestureEvent,
        };
        use crate::core::key::KeyCode;
        use crate::core::profile::Actions;
        use std::time::Instant;

        let bindings = Bindings {
            left: KeyChord::new(KeyCode::F13),
            right: KeyChord::new(KeyCode::F14),
            press: KeyChord::new(KeyCode::F15),
        };
        let mut detector = GestureDetector::new(bindings, GestureConfig::default());
        let start = Instant::now();

        let gestures = detector.feed(InputEvent::Pressed(bindings.right), start);
        assert_eq!(gestures, vec![GestureEvent::RotateRight]);

        let actions = Actions {
            right: Action::Volume {
                direction: VolumeDirection::Up,
                steps: 1,
            },
            ..Default::default()
        };

        let mut sink = MockInputSink::default();
        let guard = InjectionGuard::default();
        let cancel = AtomicBool::new(false);

        for gesture in gestures {
            let control = match gesture {
                GestureEvent::RotateLeft => Control::Left,
                GestureEvent::RotateRight => Control::Right,
                GestureEvent::SinglePress | GestureEvent::DoublePress => Control::Press,
            };
            execute_action(
                &mut sink,
                &guard,
                &[],
                actions.for_control(control),
                &cancel,
            )
            .expect("gesture action");
        }

        assert_eq!(sink.calls, vec!["volume Up 1".to_owned()]);
    }

    #[test]
    fn injected_events_are_discarded_while_the_guard_is_active() {
        let guard = InjectionGuard::default();
        let events = vec![InputEvent::Pressed(KeyChord::new(KeyCode::F13))];

        assert_eq!(discard_injected(events.clone(), &guard).len(), 1);

        guard.begin();
        assert!(discard_injected(events, &guard).is_empty());
        guard.end();
    }
}
