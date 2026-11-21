//! Background worker that executes knob actions.
//!
//! The worker owns the input sink and the injection guard so the UI thread
//! never blocks on synthesized input. Every job runs inside an injection
//! scope, which the runtime uses to discard hotkey events produced by the
//! injected keys themselves. A media transport command also hands the card
//! to a short-lived watcher thread, so following the player never stalls
//! the action queue. Macros run on their own worker: a long macro must never
//! delay the next rotation of the knob.

use super::error::AppError;
use super::runtime::{
    AppEvent, CoverImageFormat, CoverView, Indicator, PlayingView, RuntimeSnapshot, VolumeView,
};
use crate::core::action::{Action, MediaCommand};
use crate::core::key::KeyChord;
use crate::core::macro_engine::{
    MAX_TOTAL_DELAY_MS, MacroDefinition, MacroError, MacroSink, MacroStep, run_macro,
};
use crate::core::{Config, InputEvent, TextKey, tr};
use crate::platform::media_session;
use crate::platform::volume;
use crate::platform::{EnigoInputSink, InjectionGuard, InputSink, PlatformError};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use thiserror::Error;
use tracing::{error, warn};

/// How long an injected chord stays filtered after the injection scope ends.
/// The extra window catches events that were already on their way to the
/// hotkey channel when the last key was released.
const INJECTION_GRACE: Duration = Duration::from_millis(100);

/// Hard wall-clock bound of one macro, watchdog included. Validation already
/// caps the declared delays; this also bounds slow sinks and typing.
const MAX_MACRO_DURATION: Duration = Duration::from_millis(MAX_TOTAL_DELAY_MS + 5_000);

/// Compatibility shim for the platform quiet-window contract.
///
/// The platform guard grows an inherent `is_quiet()` that covers the short
/// tail after injection. Until it lands, `is_active()` is the same signal; an
/// inherent method always takes precedence over this trait method.
#[allow(dead_code)]
trait QuietInjectionGuard {
    /// Whether hotkey events may belong to injected keys.
    fn is_quiet(&self) -> bool;
}

impl QuietInjectionGuard for InjectionGuard {
    fn is_quiet(&self) -> bool {
        !self.is_active()
    }
}

/// Typed failure of one action job.
#[derive(Debug, Error)]
pub enum ActionError {
    /// The input adapter failed.
    #[error("input error: {0}")]
    Input(#[from] PlatformError),

    /// A macro step failed or was cancelled.
    #[error("macro error: {0}")]
    Macro(#[from] MacroError),

    /// The action names a macro that is not in the configuration.
    #[error("unknown macro: {0}")]
    UnknownMacro(String),
}

/// Job accepted by the action worker.
#[derive(Debug)]
pub enum ActionJob {
    /// Execute one action. The flag is shared with the cancellation command.
    Execute {
        /// Action to run.
        action: Action,
        /// Cancellation flag of this job.
        cancel: Arc<AtomicBool>,
    },
    /// Cancel the running (macro) job.
    Cancel,
    /// Stop the worker.
    Quit,
}

/// State needed to tell injected hotkey events from genuine ones.
#[derive(Debug, Default)]
struct InjectionState {
    chords: HashSet<KeyChord>,
    quiet_until: Option<Instant>,
    drain_requested: bool,
}

/// Shared injection fingerprint of the current job.
///
/// Only the chords an action can actually inject are filtered. A volume step,
/// for example, leaves the knob keys free: events arriving during its quiet
/// window must still reach the gesture detector.
#[derive(Clone, Default)]
pub struct InjectionTracker(Arc<Mutex<InjectionState>>);

impl InjectionTracker {
    /// Records the chords `action` may inject and opens a fresh quiet window.
    fn begin(&self, action: &Action, macros: &[MacroDefinition]) {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        state.chords.clear();
        collect_injected_chords(action, macros, &mut state.chords);
        state.quiet_until = None;
        state.drain_requested = false;
    }

    /// Marks the end of an injection scope and requests one extra hotkey poll.
    fn finish(&self) {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        state.quiet_until = Some(Instant::now() + INJECTION_GRACE);
        state.drain_requested = true;
    }

    /// Whether the UI service tick owes the hotkey channel one extra drain.
    pub fn take_drain(&self) -> bool {
        let mut state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut state.drain_requested)
    }

    /// Drops only the events that belong to the current injection.
    fn filter(&self, events: Vec<InputEvent>, guard: &InjectionGuard) -> Vec<InputEvent> {
        let state = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        // `is_quiet()` is false while injection is active and for the
        // platform's short quiet period after it. The tracker adds its own
        // horizon so an event already queued at scope exit is still caught.
        let noisy = !guard.is_quiet()
            || state
                .quiet_until
                .is_some_and(|until| Instant::now() < until);
        if !noisy {
            return events;
        }

        events
            .into_iter()
            .filter(|event| match event {
                InputEvent::Pressed(chord) | InputEvent::Released(chord) => {
                    !state.chords.contains(chord)
                }
            })
            .collect()
    }
}

fn collect_injected_chords(
    action: &Action,
    macros: &[MacroDefinition],
    into: &mut HashSet<KeyChord>,
) {
    match action {
        Action::Shortcut { chord } => {
            into.insert(*chord);
        }
        Action::Macro { name } => {
            if let Some(definition) = macros.iter().find(|definition| &definition.name == name) {
                for step in &definition.steps {
                    match step {
                        MacroStep::Tap { chord }
                        | MacroStep::Press { chord }
                        | MacroStep::Release { chord } => {
                            into.insert(*chord);
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }
}

/// Filters events with the full injection state. Used by the service tick.
pub fn filter_injected(
    events: Vec<InputEvent>,
    guard: &InjectionGuard,
    tracker: &InjectionTracker,
) -> Vec<InputEvent> {
    tracker.filter(events, guard)
}

/// Legacy two-argument filter for callers that do not own a tracker yet.
///
/// With no known chords it only filters while the guard itself is quiet.
pub fn discard_injected(events: Vec<InputEvent>, guard: &InjectionGuard) -> Vec<InputEvent> {
    let tracker = InjectionTracker::default();
    tracker.filter(events, guard)
}

/// One detached worker and the flag that stops it.
struct SpawnedWorker {
    cancel: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

/// Registry of macro and media watchers joined during shutdown.
type WorkerRegistry = Arc<Mutex<Vec<SpawnedWorker>>>;

/// Cloneable producer side of the worker queue.
#[derive(Clone)]
pub struct ActionQueue {
    sender: Sender<ActionJob>,
    health: Arc<AtomicBool>,
    cancel: Arc<Mutex<Arc<AtomicBool>>>,
    workers: WorkerRegistry,
    stopping: Arc<AtomicBool>,
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
    events: Sender<AppEvent>,
    tracker: InjectionTracker,
}

impl ActionQueue {
    /// Queues an action for execution. A submission resets the cancel flag.
    pub fn execute(&self, action: Action) -> Result<(), AppError> {
        if !self.healthy() {
            return Err(AppError::RuntimeStopped);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *self.cancel.lock().unwrap_or_else(PoisonError::into_inner) = Arc::clone(&cancel);
        self.sender
            .send(ActionJob::Execute { action, cancel })
            .map_err(|_| AppError::RuntimeStopped)
    }

    /// Cancels the current job, typically a running macro.
    pub fn cancel(&self) {
        let guard = self.cancel.lock().unwrap_or_else(PoisonError::into_inner);
        guard.store(true, Ordering::Relaxed);
    }

    /// Whether the input sink is still usable.
    pub fn healthy(&self) -> bool {
        self.health.load(Ordering::SeqCst)
    }

    /// Fingerprint of the current injection, for the UI service tick.
    pub fn tracker(&self) -> InjectionTracker {
        self.tracker.clone()
    }

    /// Stops every detached worker and the main worker queue.
    pub fn shutdown(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        self.cancel_workers();
        let _ = self.sender.send(ActionJob::Quit);
    }

    /// Cancels all detached workers so they can be joined quickly.
    pub fn cancel_workers(&self) {
        let workers = self.workers.lock().unwrap_or_else(PoisonError::into_inner);
        for worker in workers.iter() {
            worker.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Joins every detached worker. Called after the main worker has exited.
    pub fn join_workers(&self) {
        let workers: Vec<_> =
            std::mem::take(&mut *self.workers.lock().unwrap_or_else(PoisonError::into_inner));
        for worker in workers {
            let _ = worker.handle.join();
        }
    }

    /// Records an unusable input sink in the queue and the observable state.
    fn mark_unhealthy(&self, message: &str) {
        self.health.store(false, Ordering::SeqCst);
        let snapshot = {
            let mut current = self
                .snapshot
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            current.healthy = false;
            current.last_error = Some(message.to_owned());
            current.clone()
        };
        let _ = self.events.send(AppEvent::StateChanged(snapshot));
        let _ = self.events.send(AppEvent::Error(message.to_owned()));
    }
}

/// Spawns the action worker and returns its queue and join handle.
pub fn spawn(
    config: Arc<RwLock<Config>>,
    guard: InjectionGuard,
    events: Sender<AppEvent>,
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
) -> Result<(ActionQueue, JoinHandle<()>), AppError> {
    let (sender, receiver) = unbounded();
    let queue = ActionQueue {
        sender,
        health: Arc::new(AtomicBool::new(true)),
        cancel: Arc::new(Mutex::new(Arc::new(AtomicBool::new(false)))),
        workers: Arc::new(Mutex::new(Vec::new())),
        stopping: Arc::new(AtomicBool::new(false)),
        snapshot,
        events,
        tracker: InjectionTracker::default(),
    };
    let run_queue = queue.clone();
    let worker = thread::Builder::new()
        .name("wheelie-actions".to_owned())
        .spawn(move || run(&receiver, &config, &guard, &run_queue))?;
    Ok((queue, worker))
}

fn run(
    jobs: &Receiver<ActionJob>,
    config: &Arc<RwLock<Config>>,
    guard: &InjectionGuard,
    queue: &ActionQueue,
) {
    let mut sink = match EnigoInputSink::new() {
        Ok(sink) => sink,
        Err(error) => {
            error!(%error, "could not create the input sink");
            queue.mark_unhealthy(&format!("could not create the input sink: {error}"));
            return;
        }
    };

    let mut session = media_session::Reader::new();
    let generation = Arc::new(AtomicU64::new(0));
    let mut active_cancel = Arc::new(AtomicBool::new(false));

    while let Ok(job) = jobs.recv() {
        match job {
            ActionJob::Quit => break,
            ActionJob::Cancel => active_cancel.store(true, Ordering::Relaxed),
            ActionJob::Execute { action, cancel } => {
                active_cancel = Arc::clone(&cancel);
                let macros = read_config(config).macros.clone();
                queue.tracker().begin(&action, &macros);

                // A transport command changes the session asynchronously; the
                // previous state is the baseline the indicator polls against.
                let before = match &action {
                    Action::Media { .. } if media_session::SUPPORTED => Some(session.read()),
                    _ => None,
                };

                if let Action::Macro { name } = &action {
                    match macros
                        .iter()
                        .find(|definition| &definition.name == name)
                        .cloned()
                    {
                        Some(definition) => spawn_macro(
                            &definition,
                            guard,
                            &active_cancel,
                            &queue.events,
                            &queue.workers,
                            &queue.stopping,
                        ),
                        None => {
                            let error = ActionError::UnknownMacro(name.clone());
                            warn!(%error, "action failed");
                            let _ = queue.events.send(AppEvent::Error(error.to_string()));
                        }
                    }
                    queue.tracker().finish();
                    continue;
                }

                let executed = {
                    let _scope = guard.scope();
                    execute_action_inner(&mut sink, &macros, &action, &active_cancel)
                };
                // Drain one more time while the scope is still meaningful: an
                // injected release may already be waiting in the channel.
                queue.tracker().finish();

                match executed {
                    Err(error) => {
                        warn!(%error, "action failed");
                        let _ = queue.events.send(AppEvent::Error(error.to_string()));
                    }
                    Ok(reported) => {
                        let indicator = match (&action, before) {
                            (Action::Volume { .. } | Action::Mute, _) => match volume::level() {
                                Ok(level) => Some(Indicator::Volume(map_level(level))),
                                Err(error) => {
                                    warn!(%error, "could not read the volume level");
                                    None
                                }
                            },
                            (Action::Brightness { .. }, _) => reported.map(Indicator::Brightness),
                            (Action::Media { command }, Some(before)) => {
                                // Watching the session stays off this thread:
                                // the knob queue must keep moving while the
                                // media card follows the player.
                                spawn_media_watch(
                                    &generation,
                                    *command,
                                    before,
                                    config,
                                    &queue.events,
                                    &queue.workers,
                                    &queue.stopping,
                                    &active_cancel,
                                );
                                None
                            }
                            _ => None,
                        };
                        if let Some(indicator) = indicator {
                            let _ = queue.events.send(AppEvent::Indicator(indicator));
                        }
                    }
                }
            }
        }
    }

    queue.cancel_workers();
    queue.join_workers();
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
) -> Result<Option<u8>, ActionError>
where
    S: InputSink + MacroSink,
{
    let _scope = guard.scope();
    execute_action_inner(sink, macros, action, cancel)
}

fn execute_action_inner<S>(
    sink: &mut S,
    macros: &[MacroDefinition],
    action: &Action,
    cancel: &AtomicBool,
) -> Result<Option<u8>, ActionError>
where
    S: InputSink + MacroSink,
{
    match action {
        Action::None => Ok(None),
        Action::Volume { direction, steps } => sink
            .volume(*direction, *steps)
            .map(|()| None)
            .map_err(ActionError::from),
        Action::Mute => sink.volume_mute().map(|()| None).map_err(ActionError::from),
        Action::Brightness { direction, steps } => sink
            .brightness(*direction, *steps)
            .map_err(ActionError::from),
        Action::Scroll { axis, amount } => InputSink::scroll(sink, *axis, *amount)
            .map(|()| None)
            .map_err(ActionError::from),
        Action::Shortcut { chord } => sink
            .tap_chord(*chord)
            .map(|()| None)
            .map_err(ActionError::from),
        Action::MouseClick { button } => InputSink::mouse_click(sink, *button)
            .map(|()| None)
            .map_err(ActionError::from),
        Action::Media { command } => sink
            .media(*command)
            .map(|()| None)
            .map_err(ActionError::from),
        Action::Launch { path } => sink.launch(path).map(|()| None).map_err(ActionError::from),
        Action::Macro { name } => {
            let definition = macros
                .iter()
                .find(|definition| &definition.name == name)
                .ok_or_else(|| ActionError::UnknownMacro(name.clone()))?;
            run_macro(sink, &definition.steps, cancel).map_err(ActionError::from)?;
            Ok(None)
        }
    }
}

fn read_config(config: &RwLock<Config>) -> std::sync::RwLockReadGuard<'_, Config> {
    config
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn register_worker(workers: &WorkerRegistry, cancel: Arc<AtomicBool>, handle: JoinHandle<()>) {
    let mut workers = workers.lock().unwrap_or_else(PoisonError::into_inner);
    workers.retain(|worker| !worker.handle.is_finished());
    workers.push(SpawnedWorker { cancel, handle });
}

/// Runs a macro on its own worker so the action queue stays immediate.
fn spawn_macro(
    definition: &MacroDefinition,
    guard: &InjectionGuard,
    cancel: &Arc<AtomicBool>,
    events: &Sender<AppEvent>,
    workers: &WorkerRegistry,
    stopping: &Arc<AtomicBool>,
) {
    if stopping.load(Ordering::SeqCst) {
        return;
    }
    let definition = definition.clone();
    let thread_guard = guard.clone();
    let thread_cancel = Arc::clone(cancel);
    let worker_cancel = Arc::clone(cancel);
    let thread_events = events.clone();

    let handle = thread::Builder::new()
        .name("wheelie-macro".to_owned())
        .spawn(move || {
            let _scope = thread_guard.scope();
            let result = match EnigoInputSink::new() {
                Ok(mut sink) => run_macro_bounded(
                    &mut sink,
                    &definition.steps,
                    &thread_cancel,
                    Instant::now() + MAX_MACRO_DURATION,
                )
                .map_err(ActionError::from),
                Err(error) => Err(ActionError::from(error)),
            };
            if let Err(error) = result {
                warn!(%error, "macro failed");
                let _ = thread_events.send(AppEvent::Error(error.to_string()));
            }
        });
    match handle {
        Ok(handle) => register_worker(workers, worker_cancel, handle),
        Err(error) => warn!(%error, "could not start the macro worker"),
    }
}

/// Replays steps with a hard wall-clock deadline.
///
/// The core runner already checks the cancellation flag; this variant also
/// caps each wait at the remaining budget, so a slow sink cannot outlive the
/// macro's allowed duration.
fn run_macro_bounded<S: MacroSink>(
    sink: &mut S,
    steps: &[MacroStep],
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<(), MacroError> {
    for step in steps {
        if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
            return Err(MacroError::Cancelled);
        }
        match step {
            MacroStep::Delay { ms } => sleep_bounded(*ms, cancel, deadline)?,
            MacroStep::Tap { chord } => sink.tap(*chord)?,
            MacroStep::Press { chord } => sink.press(*chord)?,
            MacroStep::Release { chord } => sink.release(*chord)?,
            MacroStep::Text { text } => sink.text(text)?,
            MacroStep::Scroll { axis, amount } => sink.scroll(*axis, *amount)?,
            MacroStep::MouseClick { button } => sink.mouse_click(*button)?,
        }
    }
    Ok(())
}

fn sleep_bounded(ms: u64, cancel: &AtomicBool, deadline: Instant) -> Result<(), MacroError> {
    let until = (Instant::now() + Duration::from_millis(ms)).min(deadline);
    loop {
        let now = Instant::now();
        if now >= until {
            break;
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(MacroError::Cancelled);
        }
        thread::sleep((until - now).min(Duration::from_millis(10)));
    }
    if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
        return Err(MacroError::Cancelled);
    }
    Ok(())
}

/// How long the indicator waits for the player to reflect a transport
/// command, and how often the session is re-read during that window. The
/// reader keeps the session manager alive, so a poll is two cheap calls and
/// the interval can stay short.
const MEDIA_POLL_TIMEOUT: Duration = Duration::from_millis(1200);
const MEDIA_POLL_INTERVAL: Duration = Duration::from_millis(40);
/// How long a card stays interesting after its last update. Shared with the UI
/// through `app::MEDIA_DISPLAY_DURATION` so the watcher and the card cannot
/// disagree about the card's lifetime.
const MEDIA_WATCH_DURATION: Duration = crate::app::runtime::MEDIA_DISPLAY_DURATION;
/// Hard lifetime of one watcher, extensions included. A player that keeps
/// fluctuating can refresh a card a few times, never forever.
const MEDIA_WATCH_MAX: Duration = Duration::from_millis(12_000);

/// Spawns the watcher for one transport command. Only the newest watcher
/// emits: the shared generation lets a second press retire the previous
/// thread before it can push a stale track over the fresh card.
// The watcher handoff carries the full indicator context to the detached thread.
#[allow(clippy::too_many_arguments)]
fn spawn_media_watch(
    generation: &Arc<AtomicU64>,
    command: MediaCommand,
    before: media_session::Reading,
    config: &Arc<RwLock<Config>>,
    events: &Sender<AppEvent>,
    workers: &WorkerRegistry,
    stopping: &Arc<AtomicBool>,
    cancel: &Arc<AtomicBool>,
) {
    let own = generation.fetch_add(1, Ordering::Relaxed) + 1;
    let generation = Arc::clone(generation);
    let config = Arc::clone(config);
    let events = events.clone();
    let stopping = Arc::clone(stopping);
    let worker_cancel = Arc::clone(cancel);
    let watch_cancel = Arc::clone(cancel);
    let handle = thread::Builder::new()
        .name("wheelie-media-watch".to_owned())
        .spawn(move || {
            watch_media(
                own,
                generation,
                command,
                before,
                config,
                events,
                stopping,
                watch_cancel,
            )
        });
    match handle {
        Ok(handle) => register_worker(workers, worker_cancel, handle),
        Err(error) => warn!(%error, "could not watch the media session"),
    }
}

// Kept as a free function so the watcher body stays testable without a thread.
#[allow(clippy::too_many_arguments)]
fn watch_media(
    own: u64,
    generation: Arc<AtomicU64>,
    command: MediaCommand,
    before: media_session::Reading,
    config: Arc<RwLock<Config>>,
    events: Sender<AppEvent>,
    stopping: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
) {
    let mut reader = media_session::Reader::new();
    let deadline = Instant::now() + MEDIA_POLL_TIMEOUT;
    let active = || {
        generation.load(Ordering::Relaxed) == own
            && !stopping.load(Ordering::SeqCst)
            && !cancel.load(Ordering::Relaxed)
    };

    // Phase 1: wait for the command to visibly take effect, then report
    // whatever the session says (the timeout may still show the old track,
    // phase 2 replaces it as soon as the player catches up). A first
    // "changed" read is confirmed by one more, so the card does not open
    // on a half-published state (new title, cover still to come).
    let last = loop {
        if !active() {
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
    if let Some(playing) = indicator_from(&last, &config) {
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
    while active() && Instant::now() < watch_until && Instant::now() < hard_deadline {
        thread::sleep(MEDIA_POLL_INTERVAL);
        let reading = reader.read();
        let Some(snapshot) = snapshot_of(&reading) else {
            continue;
        };
        if watcher.observe(snapshot) {
            if let Some(playing) = indicator_from(&reading, &config) {
                let _ = events.send(AppEvent::Indicator(playing));
            }
            watch_until = Instant::now() + MEDIA_WATCH_DURATION;
        }
    }
}

/// Indicator for one reading: the track, or the fallback text when the
/// session says nothing is playing. `Unavailable` stays silent. The language
/// is re-read on every call so a settings change reaches the next card.
fn indicator_from(reading: &media_session::Reading, config: &RwLock<Config>) -> Option<Indicator> {
    let track = match reading {
        media_session::Reading::Track(track) if !track.title.is_empty() => track.clone(),
        media_session::Reading::Track(_) | media_session::Reading::NoSession => {
            let language = read_config(config).language;
            media_session::NowPlaying {
                title: tr(language, TextKey::MediaNoTrack).to_owned(),
                ..Default::default()
            }
        }
        media_session::Reading::Unavailable => return None,
    };
    Some(Indicator::Media(map_playing(&track)))
}

/// Maps the platform volume reading onto the UI-owned view type.
fn map_level(level: volume::Level) -> VolumeView {
    VolumeView {
        percent: level.percent,
        muted: level.muted,
    }
}

fn map_cover(cover: &media_session::Cover) -> CoverView {
    let format = match cover.format {
        media_session::CoverFormat::Png => CoverImageFormat::Png,
        media_session::CoverFormat::Jpeg => CoverImageFormat::Jpeg,
    };
    CoverView {
        bytes: cover.bytes.clone(),
        format,
    }
}

/// Maps the platform now-playing report onto the UI-owned view type.
fn map_playing(track: &media_session::NowPlaying) -> PlayingView {
    PlayingView {
        title: track.title.clone(),
        artist: track.artist.clone(),
        cover: track.cover.as_ref().map(map_cover),
        playing: track.playing,
        position_ms: track.position_ms,
        duration_ms: track.duration_ms,
    }
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
/// state. The platform report has no stable track identifier, so title,
/// artist, duration and (when the player publishes a timeline) position are
/// used as the closest available fingerprint. Two different tracks sharing
/// the same metadata without a timeline therefore cannot be told apart; that
/// is a documented limit of the media adapter, not a silent success.
fn session_changed(
    command: MediaCommand,
    before: &media_session::Reading,
    current: &media_session::Reading,
) -> bool {
    let state = |reading: &media_session::Reading| match reading {
        media_session::Reading::Track(track) => Some((
            track.title.clone(),
            track.artist.clone(),
            track.playing,
            track.position_ms,
            track.duration_ms,
        )),
        media_session::Reading::NoSession | media_session::Reading::Unavailable => None,
    };
    match (state(before), state(current)) {
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => true,
        (
            Some((was_title, was_artist, was_playing, was_position, was_duration)),
            Some((title, artist, playing, position, duration)),
        ) => {
            let same_track = was_title == title && was_artist == artist && was_duration == duration;
            match command {
                MediaCommand::PlayPause => was_playing != playing,
                MediaCommand::Stop => !playing,
                MediaCommand::Next | MediaCommand::Previous => {
                    !same_track || was_position.abs_diff(position) > 1000
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::runtime::{RuntimeSnapshot, RuntimeState};
    use crate::core::action::{MouseButton, ScrollAxis, VolumeDirection};
    use crate::core::key::{KeyCode, Modifiers};
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
                command: MediaCommand::PlayPause,
            },
            Action::Macro {
                name: "paste".to_owned(),
            },
        ];

        for action in &actions {
            execute_action(&mut sink, &guard, &macros, action, &cancel).expect("action");
        }

        // `Action::None` is intentionally silent; every other variant must
        // produce exactly one recorded call.
        assert_eq!(
            sink.calls,
            vec![
                "scroll Vertical -2".to_owned(),
                "tap F13".to_owned(),
                "click Middle".to_owned(),
                "media PlayPause".to_owned(),
                "text hi".to_owned(),
            ]
        );
    }

    #[test]
    fn input_failures_are_typed_errors() {
        let mut sink = MockInputSink {
            fail: true,
            ..Default::default()
        };
        let guard = InjectionGuard::default();
        let cancel = AtomicBool::new(false);

        let error = execute_action(&mut sink, &guard, &[], &Action::Mute, &cancel)
            .expect_err("the mock sink fails");
        assert!(matches!(error, ActionError::Input(_)));
        assert!(!guard.is_active());
    }

    #[test]
    fn injected_event_after_scope_is_filtered_during_the_grace_window() {
        let guard = InjectionGuard::default();
        let tracker = InjectionTracker::default();
        let injected = KeyChord::new(KeyCode::F13);
        let genuine = KeyChord::new(KeyCode::F14);
        let action = Action::Shortcut { chord: injected };

        tracker.begin(&action, &[]);
        {
            let _scope = guard.scope();
            // The injected key is released here; its event is already queued.
        }
        tracker.finish();
        assert!(tracker.take_drain());

        let events = vec![
            InputEvent::Pressed(injected),
            InputEvent::Released(injected),
            InputEvent::Pressed(genuine),
        ];
        assert_eq!(
            filter_injected(events, &guard, &tracker),
            vec![InputEvent::Pressed(genuine)]
        );
        assert!(!tracker.take_drain(), "the extra drain happens only once");
    }

    #[test]
    fn cancel_flag_reaches_a_macro_as_macro_error_cancelled() {
        let mut sink = MockInputSink::default();
        let guard = InjectionGuard::default();
        let cancel = AtomicBool::new(true);
        let macros = vec![MacroDefinition {
            name: "slow".to_owned(),
            steps: vec![MacroStep::Text {
                text: "nope".to_owned(),
            }],
        }];

        let error = execute_action(
            &mut sink,
            &guard,
            &macros,
            &Action::Macro {
                name: "slow".to_owned(),
            },
            &cancel,
        )
        .expect_err("cancelled macro");
        assert!(matches!(error, ActionError::Macro(MacroError::Cancelled)));
        assert!(sink.calls.is_empty());
    }

    #[test]
    fn macro_runner_respects_the_wall_clock_deadline() {
        let mut sink = MockInputSink::default();
        let cancel = AtomicBool::new(false);
        let steps = vec![MacroStep::Delay { ms: 60_000 }];

        let started = Instant::now();
        let error = run_macro_bounded(
            &mut sink,
            &steps,
            &cancel,
            started + Duration::from_millis(10),
        )
        .expect_err("the deadline cancels the wait");
        assert_eq!(error, MacroError::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn platform_dtos_are_mapped_without_leaking_platform_types() {
        let cover = media_session::Cover {
            bytes: vec![0xFF, 0xD8, 0xFF, 0x00],
            format: media_session::CoverFormat::Jpeg,
        };
        let track = media_session::NowPlaying {
            title: "A".to_owned(),
            artist: "B".to_owned(),
            cover: Some(cover.clone()),
            playing: true,
            position_ms: 12,
            duration_ms: 345,
        };

        let view = map_playing(&track);
        assert_eq!(view.title, "A");
        assert_eq!(view.artist, "B");
        assert_eq!(
            view.cover,
            Some(CoverView {
                bytes: cover.bytes,
                format: CoverImageFormat::Jpeg,
            })
        );
        assert!(view.playing);
        assert_eq!(view.position_ms, 12);
        assert_eq!(view.duration_ms, 345);

        let level = map_level(volume::Level {
            percent: 42,
            muted: true,
        });
        assert_eq!(
            level,
            VolumeView {
                percent: 42,
                muted: true
            }
        );
    }

    #[test]
    fn queue_exposes_an_unhealthy_input_sink() {
        let (sender, _receiver) = unbounded::<ActionJob>();
        let (event_tx, event_rx) = unbounded::<AppEvent>();
        let snapshot = Arc::new(RwLock::new(RuntimeSnapshot {
            state: RuntimeState::Running,
            ..Default::default()
        }));
        let queue = ActionQueue {
            sender,
            health: Arc::new(AtomicBool::new(true)),
            cancel: Arc::new(Mutex::new(Arc::new(AtomicBool::new(false)))),
            workers: Arc::new(Mutex::new(Vec::new())),
            stopping: Arc::new(AtomicBool::new(false)),
            snapshot: Arc::clone(&snapshot),
            events: event_tx,
            tracker: InjectionTracker::default(),
        };

        assert!(queue.healthy());
        queue.mark_unhealthy("sink failed");
        assert!(!queue.healthy());
        assert!(!snapshot.read().expect("snapshot").healthy);
        assert!(matches!(
            event_rx.recv().expect("state event"),
            AppEvent::StateChanged(state) if !state.healthy
        ));
        assert!(matches!(
            queue.execute(Action::None),
            Err(AppError::RuntimeStopped)
        ));
    }

    #[test]
    fn injected_chords_are_collected_from_shortcuts_and_macros() {
        let shortcut = KeyChord::new(KeyCode::F13);
        let tapped = KeyChord::with_modifiers(
            KeyCode::KeyC,
            Modifiers {
                control: true,
                ..Default::default()
            },
        );
        let macros = vec![MacroDefinition {
            name: "copy".to_owned(),
            steps: vec![MacroStep::Tap { chord: tapped }],
        }];

        let mut chords = HashSet::new();
        collect_injected_chords(&Action::Shortcut { chord: shortcut }, &macros, &mut chords);
        assert!(chords.contains(&shortcut));

        chords.clear();
        collect_injected_chords(
            &Action::Macro {
                name: "copy".to_owned(),
            },
            &macros,
            &mut chords,
        );
        assert!(chords.contains(&tapped));
    }

    fn track(title: &str, artist: &str, playing: bool) -> media_session::Reading {
        media_session::Reading::Track(media_session::NowPlaying {
            title: title.to_owned(),
            artist: artist.to_owned(),
            playing,
            ..Default::default()
        })
    }

    fn positioned_track(position_ms: u64) -> media_session::Reading {
        media_session::Reading::Track(media_session::NowPlaying {
            title: "A".to_owned(),
            artist: "X".to_owned(),
            cover: None,
            playing: true,
            position_ms,
            duration_ms: 10_000,
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
    fn a_track_restart_is_detected_from_its_position() {
        let before = positioned_track(9000);
        let restarted = positioned_track(0);

        assert!(session_changed(MediaCommand::Next, &before, &restarted));
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

        assert!(error.to_string().contains("missing"));
        assert!(!guard.is_active());
    }

    #[test]
    fn knob_gestures_drive_actions_end_to_end() {
        use crate::core::gesture::{
            Bindings, Control, GestureConfig, GestureDetector, GestureEvent,
        };

        let bindings = Bindings {
            left: KeyChord::new(KeyCode::F13),
            right: KeyChord::new(KeyCode::F14),
            press: KeyChord::new(KeyCode::F15),
        };
        let mut detector = GestureDetector::new(bindings, GestureConfig::default());
        let start = Instant::now();

        let gestures = detector.feed(InputEvent::Pressed(bindings.right), start);
        assert_eq!(gestures, vec![GestureEvent::RotateRight]);

        let actions = crate::core::profile::Actions {
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
        let tracker = InjectionTracker::default();
        let injected = KeyChord::new(KeyCode::F13);
        let genuine = KeyChord::new(KeyCode::F14);
        tracker.begin(&Action::Shortcut { chord: injected }, &[]);

        {
            let _scope = guard.scope();
            assert_eq!(
                filter_injected(
                    vec![InputEvent::Pressed(injected), InputEvent::Pressed(genuine),],
                    &guard,
                    &tracker,
                ),
                vec![InputEvent::Pressed(genuine)],
            );
        }
    }
}
