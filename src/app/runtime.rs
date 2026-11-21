//! Application runtime: observable state, configuration and action execution.
//!
//! The runtime keeps the pieces that are shared between the UI thread and the
//! background action worker: the versioned configuration document, the
//! injection guard, a queue of actions and the OS services that must live on
//! the window event loop. The UI only calls [`RuntimeHandle::tick_services`]
//! and reports window changes; adapter types stay inside this module.

use super::action_worker::{self, ActionQueue, filter_injected};
use super::config_store::{ConfigStore, LoadedConfig};
use super::error::AppError;
use crate::core::{
    APP_ID, AUTOSTART_FLAG, Action, Config, Control, InputEvent, KeyChord, Language, PlatformInfo,
};
use crate::platform::hotkeys::HotkeySource;
use crate::platform::{GlobalHotkeyService, InjectionGuard, TrayCommand, TrayService};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::marker::PhantomData;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tracing::{info, warn};

/// How long a now-playing card stays on screen. Shared with the media watcher
/// so the card and the watcher cannot disagree about its lifetime.
pub const MEDIA_DISPLAY_DURATION: Duration = Duration::from_millis(5000);

/// Global runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuntimeState {
    /// The runtime is starting.
    #[default]
    Starting,
    /// Detection and actions are enabled.
    Running,
    /// Detection is suspended.
    Paused,
    /// The runtime is stopped.
    Stopped,
}

/// Immutable snapshot observed by the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    /// Current state.
    pub state: RuntimeState,
    /// Whether the knob keys are currently being watched.
    pub capture_active: bool,
    /// Active profile, when known.
    pub active_profile: Option<String>,
    /// Last user-facing error.
    pub last_error: Option<String>,
    /// Whether the input worker can still synthesize actions.
    pub healthy: bool,
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        Self {
            state: RuntimeState::Starting,
            capture_active: false,
            active_profile: None,
            last_error: None,
            healthy: true,
        }
    }
}

/// Commands sent to the runtime state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    /// Suspend detection.
    Pause,
    /// Resume detection.
    Resume,
    /// Stop the runtime cleanly.
    Quit,
}

/// Events emitted by the runtime toward the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    /// The observable state changed.
    StateChanged(RuntimeSnapshot),
    /// An error should be displayed.
    Error(String),
    /// A volume, brightness or media change the UI shows on screen.
    Indicator(Indicator),
    /// Hotkey events that survived injection filtering.
    Hotkeys(Vec<InputEvent>),
    /// Commands picked in the tray menu.
    Tray(Vec<TrayCommand>),
}

/// Encoded image format of an album cover, independent of any platform type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverImageFormat {
    /// PNG bytes.
    Png,
    /// JPEG bytes.
    Jpeg,
}

/// Encoded cover image handed to the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverView {
    /// Encoded image bytes.
    pub bytes: Vec<u8>,
    /// Encoding of `bytes`.
    pub format: CoverImageFormat,
}

/// Track state handed to the UI, decoupled from the media adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayingView {
    /// Track title.
    pub title: String,
    /// Artist name, possibly empty.
    pub artist: String,
    /// Cover art when the player published one.
    pub cover: Option<CoverView>,
    /// Whether the session is currently playing.
    pub playing: bool,
    /// Elapsed position inside the track, in milliseconds.
    pub position_ms: u64,
    /// Total track length, in milliseconds; 0 when the player hides it.
    pub duration_ms: u64,
}

/// Output volume handed to the UI, decoupled from the audio adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumeView {
    /// Volume in percent, from 0 to 100.
    pub percent: u8,
    /// Whether the output is muted.
    pub muted: bool,
}

/// What the UI shows on screen after an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Indicator {
    /// System output volume and mute state.
    Volume(VolumeView),
    /// Screen brightness in percent.
    Brightness(u8),
    /// Media session state after a transport command.
    Media(PlayingView),
}

/// OS services that must be created and polled on the window event loop.
///
/// The adapter types are private to the application layer: the UI calls
/// [`RuntimeHandle::tick_services`] and reports window changes through
/// [`RuntimeHandle::set_window_visible`] and
/// [`RuntimeHandle::set_key_capture_active`].
struct ServiceState {
    hotkeys: Option<GlobalHotkeyService>,
    hotkeys_failed: bool,
    hotkeys_dirty: bool,
    registered: Vec<KeyChord>,
    tray: Option<TrayService>,
    tray_failed: bool,
    tray_language: Option<Language>,
    window_visible: bool,
    capturing: bool,
}

impl Default for ServiceState {
    fn default() -> Self {
        Self {
            hotkeys: None,
            hotkeys_failed: false,
            hotkeys_dirty: false,
            registered: Vec::new(),
            tray: None,
            tray_failed: false,
            tray_language: None,
            window_visible: true,
            capturing: false,
        }
    }
}

/// Cloneable handle used by the UI to observe and control the runtime.
#[derive(Clone)]
pub struct RuntimeHandle {
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
    commands: Sender<AppCommand>,
    events: Arc<Receiver<AppEvent>>,
    event_tx: Sender<AppEvent>,
    config: Arc<RwLock<Config>>,
    generation: Arc<AtomicU64>,
    config_lock: Arc<Mutex<()>>,
    store: ConfigStore,
    guard: InjectionGuard,
    actions: ActionQueue,
    // The adapters are created and polled on the window event loop only.
    #[allow(clippy::arc_with_non_send_sync)]
    services: Arc<Mutex<ServiceState>>,
    executable: PathBuf,
}

impl RuntimeHandle {
    /// Latest known state.
    pub fn snapshot(&self) -> RuntimeSnapshot {
        PoisonTolerant::<RuntimeSnapshot>::read(&self.snapshot).clone()
    }

    /// Sends a command to the runtime.
    pub fn send(&self, command: AppCommand) -> Result<(), AppError> {
        self.commands
            .send(command)
            .map_err(|_| AppError::RuntimeStopped)
    }

    /// Pops the next event without blocking.
    pub fn try_recv_event(&self) -> Option<AppEvent> {
        self.events.try_recv().ok()
    }

    /// Latest configuration document.
    pub fn config(&self) -> Config {
        PoisonTolerant::<Config>::read(&self.config).clone()
    }

    /// Applies a change, validates it and persists it.
    ///
    /// The document is cloned under the read lock, validated and saved with
    /// the lock released, then the write lock is reacquired. A generation
    /// counter refuses an edit that was overtaken by a concurrent one, so a
    /// stale candidate can never replace a newer document.
    pub fn update_config(&self, update: impl FnOnce(&mut Config)) -> Result<(), AppError> {
        let _edit = self
            .config_lock
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let expected = self.generation.load(Ordering::SeqCst);
        let mut candidate = PoisonTolerant::<Config>::read(&self.config).clone();
        update(&mut candidate);
        candidate.validate()?;
        if self.generation.load(Ordering::SeqCst) != expected {
            return Err(AppError::StaleUpdate);
        }

        self.store.save(&candidate)?;
        let mut current = PoisonTolerant::<Config>::write(&self.config);
        if self
            .generation
            .compare_exchange(
                expected,
                expected.wrapping_add(1),
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            return Err(AppError::StaleUpdate);
        }
        *current = candidate;
        let active_profile = active_profile_name(&current);
        drop(current);
        self.mutate_snapshot(|snapshot| snapshot.active_profile = active_profile);
        Ok(())
    }

    /// Records that the first-launch introduction was completed.
    pub fn complete_onboarding(&self) -> Result<(), AppError> {
        self.update_config(|config| config.onboarding_complete = true)
    }

    /// Whether the startup release notice may run.
    pub fn startup_notices_allowed(&self) -> bool {
        PoisonTolerant::<Config>::read(&self.config).onboarding_complete
    }

    /// Makes the action set at `index` the active one.
    pub fn select_set(&self, index: usize) -> Result<(), AppError> {
        let id = PoisonTolerant::<Config>::read(&self.config)
            .action_sets
            .get(index)
            .map(|set| set.id.clone())
            .ok_or(AppError::UnknownActionSet(index))?;
        self.update_config(|config| config.active_set = Some(id))
    }

    /// Resolves and runs the action of `control` in the active action set.
    pub fn run_control(&self, control: Control) -> Result<(), AppError> {
        let action = PoisonTolerant::<Config>::read(&self.config)
            .active_set_or_first()
            .map(|set| set.actions.for_control(control).clone())
            .unwrap_or(Action::None);
        self.execute(action)
    }

    /// Queues an action for the background worker.
    pub fn execute(&self, action: Action) -> Result<(), AppError> {
        self.actions.execute(action)
    }

    /// Cancels the current action, typically a running macro.
    pub fn cancel_action(&self) {
        self.actions.cancel();
    }

    /// Whether the input worker is healthy.
    pub fn health(&self) -> bool {
        self.actions.healthy()
    }

    /// Guard shared with the action worker.
    pub fn injection_guard(&self) -> InjectionGuard {
        self.guard.clone()
    }

    /// Records whether knob keys are currently watched.
    pub fn set_capture_active(&self, active: bool) {
        self.mutate_snapshot(|snapshot| snapshot.capture_active = active);
    }

    /// Records a user-facing error without stopping the runtime.
    pub fn report_error(&self, message: impl Into<String>) {
        let message = message.into();
        self.mutate_snapshot(|snapshot| snapshot.last_error = Some(message));
    }

    /// Reports that the settings window is open or hidden to tray.
    pub fn set_window_visible(&self, visible: bool) {
        let mut services = self.services.lock().unwrap_or_else(PoisonError::into_inner);
        services.window_visible = visible;
    }

    /// Reports that a key field is waiting for a combination.
    pub fn set_key_capture_active(&self, active: bool) {
        let mut services = self.services.lock().unwrap_or_else(PoisonError::into_inner);
        if services.capturing != active {
            services.capturing = active;
            services.hotkeys_dirty = true;
        }
    }

    /// Creates, registers and polls the OS services. Call on the UI thread.
    ///
    /// Hotkey events and tray commands are delivered as [`AppEvent`]s; the UI
    /// never touches a platform adapter. A failure only logs and clears the
    /// capture indicator, so a missing tray or watcher never stops the app.
    pub fn tick_services(&self) {
        let config = self.config();
        let capture_ok;
        let mut hotkey_error = None;
        {
            let mut services = self.services.lock().unwrap_or_else(PoisonError::into_inner);
            let language = config.language;

            if services.hotkeys.is_none() && !services.hotkeys_failed {
                match GlobalHotkeyService::new() {
                    Ok(service) => {
                        info!("global key watcher ready");
                        services.hotkeys = Some(service);
                        services.hotkeys_dirty = true;
                    }
                    Err(error) => {
                        warn!(%error, "global key watcher unavailable");
                        services.hotkeys_failed = true;
                        hotkey_error = Some(error.to_string());
                    }
                }
            }

            if services.tray.is_none() && !services.tray_failed && config.general.show_tray {
                let built = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    TrayService::new(language)
                }));
                match built {
                    Ok(Ok(tray)) => {
                        services.tray = Some(tray);
                        services.tray_language = Some(language);
                    }
                    Ok(Err(error)) => {
                        warn!(%error, "tray icon unavailable");
                        services.tray_failed = true;
                    }
                    Err(_) => {
                        warn!("tray icon unavailable on this system");
                        services.tray_failed = true;
                    }
                }
            }

            if services.tray_language != Some(language) && services.tray.take().is_some() {
                services.tray_failed = false;
                services.tray_language = None;
            }

            let paused = matches!(
                self.snapshot().state,
                RuntimeState::Paused | RuntimeState::Stopped
            );
            let window_visible = services.window_visible;
            if let Some(tray) = &mut services.tray
                && let Err(error) = tray.set_state(language, window_visible, paused)
            {
                warn!(%error, "tray menu update failed");
            }

            let desired: Vec<KeyChord> = if paused {
                Vec::new()
            } else if services.capturing {
                // Release the knob keys so a press reaches the field, but keep
                // the chords the desktop shell normally consumes.
                KeyChord::system_reserved()
            } else {
                config
                    .active_profile()
                    .map(|profile| {
                        vec![
                            profile.bindings.left,
                            profile.bindings.right,
                            profile.bindings.press,
                        ]
                    })
                    .unwrap_or_default()
            };

            let capturing = services.capturing;
            capture_ok = if services.hotkeys_dirty || desired != services.registered {
                services.hotkeys_dirty = false;
                match services.hotkeys.as_mut() {
                    Some(service) => {
                        let result = if desired.is_empty() {
                            service.unregister_all()
                        } else if capturing {
                            let refused = service.apply_optional(&desired);
                            if !refused.is_empty() {
                                let labels = refused
                                    .iter()
                                    .map(|chord| chord.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                warn!(keys = %labels, "the system keeps these keys during capture");
                            }
                            Ok(())
                        } else {
                            service.apply(&desired)
                        };
                        services.registered = if result.is_ok() { desired } else { Vec::new() };
                        result.is_ok()
                    }
                    None => false,
                }
            } else {
                true
            };

            let mut hotkeys = services
                .hotkeys
                .as_mut()
                .map(HotkeySource::poll)
                .unwrap_or_default();
            if self.actions.tracker().take_drain() {
                let extra = services
                    .hotkeys
                    .as_mut()
                    .map(HotkeySource::poll)
                    .unwrap_or_default();
                hotkeys.extend(extra);
            }
            let hotkeys = filter_injected(hotkeys, &self.guard, &self.actions.tracker());
            let tray_commands = services
                .tray
                .as_ref()
                .map(TrayService::poll)
                .unwrap_or_default();

            if !hotkeys.is_empty() {
                let _ = self.event_tx.send(AppEvent::Hotkeys(hotkeys));
            }
            if !tray_commands.is_empty() {
                let _ = self.event_tx.send(AppEvent::Tray(tray_commands));
            }
        }

        if let Some(error) = hotkey_error {
            self.report_error(error);
        }
        let running = !matches!(
            self.snapshot().state,
            RuntimeState::Paused | RuntimeState::Stopped
        );
        self.set_capture_active(capture_ok && running);
    }

    /// Enables or disables the start-at-login entry and returns its real state.
    pub fn set_start_at_login(&self, enabled: bool) -> Result<bool, AppError> {
        let args = [AUTOSTART_FLAG];
        crate::platform::autostart::set(APP_ID, &self.executable, &args, enabled)?;
        let enabled_now = crate::platform::autostart::is_enabled(APP_ID, &self.executable)?;
        self.update_config(|config| config.general.start_at_login = enabled_now)?;
        Ok(enabled_now)
    }

    /// Whether the start-at-login entry currently exists.
    pub fn start_at_login_enabled(&self) -> Result<bool, AppError> {
        Ok(crate::platform::autostart::is_enabled(
            APP_ID,
            &self.executable,
        )?)
    }

    /// Path of the configuration document, for display in the About page.
    pub fn config_path(&self) -> PathBuf {
        self.store.path().to_path_buf()
    }

    /// Static platform capabilities.
    ///
    /// Exposed through the application layer so the UI never depends on
    /// `wheelie-platform` directly.
    pub fn platform_info(&self) -> PlatformInfo {
        crate::platform::info()
    }

    /// Mutates the snapshot once and publishes the resulting state event.
    fn mutate_snapshot(&self, update: impl FnOnce(&mut RuntimeSnapshot)) {
        mutate_snapshot(&self.event_tx, &self.snapshot, update);
    }
}

/// Display name of the active profile, repaired when the stored id is stale.
fn active_profile_name(config: &Config) -> Option<String> {
    config
        .active_profile()
        .or_else(|| config.profiles.first())
        .map(|profile| profile.name.clone())
}

/// Runtime owned by the binary. Keep it alive for the whole UI lifetime.
pub struct Runtime {
    handle: RuntimeHandle,
    command_tx: Sender<AppCommand>,
    worker: Option<JoinHandle<()>>,
    action_worker: Option<JoinHandle<()>>,
}

// The OS service adapters are event-loop-thread bound; the Arc is an internal
// handle for the UI, never shared with a worker thread.
#[allow(clippy::arc_with_non_send_sync)]
impl Runtime {
    /// Starts the runtime with the configuration at the default location.
    pub fn new() -> Result<Self, AppError> {
        Self::with_executable(std::env::current_exe()?)
    }

    /// Starts the runtime with an explicit executable for the login entry.
    pub fn with_executable(executable: PathBuf) -> Result<Self, AppError> {
        Self::with_store_and_executable(ConfigStore::at_default_location()?, executable)
    }

    /// Alias for [`Self::with_executable`], used by tests and embedding code.
    // OS service adapters live on the event-loop thread; the Arc is only a
    // convenient handle for the UI, never shared with a worker thread.
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn from_executable(executable: PathBuf) -> Result<Self, AppError> {
        Self::with_executable(executable)
    }

    /// Starts the runtime with an explicit configuration store.
    pub fn with_store(store: ConfigStore) -> Result<Self, AppError> {
        Self::with_store_and_executable(store, std::env::current_exe()?)
    }

    /// Starts the runtime with an explicit store and executable.
    pub fn with_store_and_executable(
        store: ConfigStore,
        executable: PathBuf,
    ) -> Result<Self, AppError> {
        let (command_tx, command_rx) = unbounded();
        let (event_tx, event_rx) = unbounded();
        let snapshot = Arc::new(RwLock::new(RuntimeSnapshot::default()));

        let platform = crate::platform::info();
        let LoadedConfig {
            mut config,
            warning,
        } = store.load_with_default(Config::default_with(platform.brightness_supported));

        if !platform.brightness_supported {
            let original = config.clone();
            config.remove_default_brightness_set();
            if config != original {
                match store.save(&config) {
                    Ok(()) => warn!(
                        "removed the default brightness set: this platform cannot change brightness"
                    ),
                    Err(error) => warn!(%error, "could not persist the brightness filter"),
                }
            }
        }

        let config = Arc::new(RwLock::new(config));
        let active_profile = active_profile_name(&PoisonTolerant::<Config>::read(&config));
        mutate_snapshot(&event_tx, &snapshot, |current| {
            current.active_profile = active_profile;
            if let Some(warning) = &warning {
                current.last_error = Some(warning.clone());
            }
        });
        if let Some(warning) = &warning {
            warn!(%warning, "configuration was not usable");
        }

        let guard = InjectionGuard::default();
        let (actions, action_worker) = action_worker::spawn(
            Arc::clone(&config),
            guard.clone(),
            event_tx.clone(),
            Arc::clone(&snapshot),
        )?;

        let worker_snapshot = Arc::clone(&snapshot);
        let worker_events = event_tx.clone();
        let worker = thread::Builder::new()
            .name("wheelie-runtime".to_owned())
            .spawn(move || run_loop(&command_rx, &worker_events, &worker_snapshot))?;

        let handle = RuntimeHandle {
            snapshot: Arc::clone(&snapshot),
            commands: command_tx.clone(),
            events: Arc::new(event_rx),
            event_tx,
            config,
            generation: Arc::new(AtomicU64::new(0)),
            config_lock: Arc::new(Mutex::new(())),
            store,
            guard,
            actions: actions.clone(),
            services: Arc::new(Mutex::new(ServiceState::default())),
            executable,
        };

        Ok(Self {
            handle,
            command_tx,
            worker: Some(worker),
            action_worker: Some(action_worker),
        })
    }

    /// Handle passed to the UI and background services.
    pub fn handle(&self) -> RuntimeHandle {
        self.handle.clone()
    }

    /// Stops the runtime threads and every watcher. Idempotent.
    pub fn shutdown(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = self.command_tx.send(AppCommand::Quit);
            let _ = worker.join();
        }
        self.handle.actions.shutdown();
        if let Some(action_worker) = self.action_worker.take() {
            let _ = action_worker.join();
        }
        self.handle.actions.join_workers();
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn run_loop(
    commands: &Receiver<AppCommand>,
    events: &Sender<AppEvent>,
    snapshot: &Arc<RwLock<RuntimeSnapshot>>,
) {
    mutate_snapshot(events, snapshot, |state| {
        state.state = RuntimeState::Running;
    });

    while let Ok(command) = commands.recv() {
        match command {
            AppCommand::Pause => {
                mutate_snapshot(events, snapshot, |state| {
                    state.state = RuntimeState::Paused;
                    state.capture_active = false;
                });
            }
            AppCommand::Resume => {
                mutate_snapshot(events, snapshot, |state| {
                    state.state = RuntimeState::Running;
                });
            }
            AppCommand::Quit => break,
        }
    }

    // Every exit path, including a disconnected command channel, is a full
    // stop: the UI must drop its window and the caller joins every worker.
    mutate_snapshot(events, snapshot, |state| {
        state.state = RuntimeState::Stopped;
        state.capture_active = false;
    });
}

/// Applies one snapshot mutation and emits exactly one state event.
fn mutate_snapshot(
    events: &Sender<AppEvent>,
    snapshot: &Arc<RwLock<RuntimeSnapshot>>,
    update: impl FnOnce(&mut RuntimeSnapshot),
) {
    let next = {
        let mut current = PoisonTolerant::<RuntimeSnapshot>::write(snapshot);
        update(&mut current);
        current.clone()
    };
    let _ = events.send(AppEvent::StateChanged(next));
}

/// Locks that keep working after a panic poisoned them.
///
/// A poisoned lock is a development error, not a reason to freeze the whole
/// application: the data is still structurally valid and the UI can recover.
struct PoisonTolerant<T>(PhantomData<T>);

impl<T> PoisonTolerant<T> {
    fn read(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
        lock.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
        lock.write().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::action::MediaCommand;
    use crate::core::profile::Actions;
    use std::fs;

    /// Temporary directory removed on drop, including on test failure.
    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("wheelie-runtime-{name}-{nanos}"));
            fs::create_dir_all(&path).expect("temp directory");
            Self { path }
        }

        fn path(&self) -> &PathBuf {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn temp_store(name: &str) -> (TempDir, ConfigStore) {
        let directory = TempDir::new(name);
        let store = ConfigStore::new(directory.path().join("config.toml"));
        (directory, store)
    }

    #[test]
    fn pause_and_resume_update_the_snapshot() {
        let (_dir, store) = temp_store("pause");
        let mut runtime = Runtime::with_store(store).expect("runtime");
        let handle = runtime.handle();

        let started = wait_for(&handle, |snapshot| snapshot.state == RuntimeState::Running);
        assert!(!started.capture_active);
        assert!(started.healthy);

        handle.send(AppCommand::Pause).expect("pause");
        let paused = wait_for(&handle, |snapshot| snapshot.state == RuntimeState::Paused);
        assert!(!paused.capture_active);

        handle.send(AppCommand::Resume).expect("resume");
        wait_for(&handle, |snapshot| snapshot.state == RuntimeState::Running);

        runtime.shutdown();
        assert_eq!(handle.snapshot().state, RuntimeState::Stopped);
    }

    #[test]
    fn configuration_updates_are_persisted_and_validated() {
        let (_dir, store) = temp_store("update");
        let mut runtime = Runtime::with_store(store.clone()).expect("runtime");
        let handle = runtime.handle();

        handle
            .update_config(|config| config.language = crate::core::Language::En)
            .expect("language update");
        assert_eq!(handle.config().language, crate::core::Language::En);
        assert_eq!(store.load().config.language, crate::core::Language::En);

        let rejected = handle.update_config(|config| {
            config.profiles[0].bindings.left = config.profiles[0].bindings.right;
        });
        assert!(rejected.is_err(), "invalid bindings must be rejected");
        assert_ne!(
            handle.config().profiles[0].bindings.left,
            handle.config().profiles[0].bindings.right
        );

        runtime.shutdown();
    }

    #[test]
    fn configuration_survives_a_restart() {
        let (_dir, store) = temp_store("restart");

        {
            let mut runtime = Runtime::with_store(store.clone()).expect("first run");
            runtime
                .handle()
                .update_config(|config| config.language = crate::core::Language::En)
                .expect("update");
            runtime.shutdown();
        }

        let mut runtime = Runtime::with_store(store).expect("second run");
        assert_eq!(
            runtime.handle().config().language,
            crate::core::Language::En
        );
        runtime.shutdown();
    }

    #[test]
    fn completing_the_introduction_is_persisted() {
        let (_dir, store) = temp_store("onboarding");
        let mut runtime = Runtime::with_store(store.clone()).expect("runtime");
        let handle = runtime.handle();
        assert!(!handle.config().onboarding_complete);
        assert!(!handle.startup_notices_allowed());

        handle.complete_onboarding().expect("complete introduction");
        assert!(store.load().config.onboarding_complete);
        assert!(handle.startup_notices_allowed());

        runtime.shutdown();
    }

    #[test]
    fn invalid_configuration_on_disk_falls_back_to_defaults() {
        let (_dir, store) = temp_store("broken");
        std::fs::create_dir_all(store.path().parent().expect("parent")).expect("directory");
        std::fs::write(store.path(), "not = { valid").expect("write");

        let mut runtime = Runtime::with_store(store).expect("runtime");
        let handle = runtime.handle();
        assert!(handle.snapshot().last_error.is_some());
        let expected = Config::default_with(crate::platform::info().brightness_supported);
        assert_eq!(handle.config(), expected);

        runtime.shutdown();
    }

    #[test]
    fn select_set_and_run_control_use_the_active_set() {
        let (_dir, store) = temp_store("controls");
        let mut runtime = Runtime::with_store(store).expect("runtime");
        let handle = runtime.handle();
        wait_for(&handle, |snapshot| snapshot.state == RuntimeState::Running);

        handle.select_set(1).expect("select set");
        assert_eq!(
            handle.config().active_set,
            handle.config().action_sets.get(1).map(|set| set.id.clone())
        );
        assert!(matches!(
            handle.select_set(usize::MAX),
            Err(AppError::UnknownActionSet(_))
        ));

        handle
            .update_config(|config| {
                if let Some(set) = config.action_sets.first_mut() {
                    set.actions = Actions {
                        right: Action::Media {
                            command: MediaCommand::PlayPause,
                        },
                        ..Actions::default()
                    };
                }
                config.active_set = config.action_sets.first().map(|set| set.id.clone());
            })
            .expect("configure actions");
        handle.run_control(Control::Right).expect("run control");
        assert!(handle.health());

        runtime.shutdown();
    }

    #[test]
    fn cancellation_flag_is_reachable_from_the_handle() {
        let (_dir, store) = temp_store("cancel");
        let mut runtime = Runtime::with_store(store).expect("runtime");
        let handle = runtime.handle();
        wait_for(&handle, |snapshot| snapshot.state == RuntimeState::Running);

        handle.cancel_action();
        handle
            .run_control(Control::Press)
            .expect("action after cancel");
        assert!(handle.health());

        runtime.shutdown();
    }

    #[test]
    fn snapshot_setters_emit_state_events() {
        let (_dir, store) = temp_store("events");
        let mut runtime = Runtime::with_store(store).expect("runtime");
        let handle = runtime.handle();
        wait_for(&handle, |snapshot| snapshot.state == RuntimeState::Running);
        while handle.try_recv_event().is_some() {}

        handle.set_capture_active(true);
        assert!(matches!(
            handle.try_recv_event(),
            Some(AppEvent::StateChanged(snapshot)) if snapshot.capture_active
        ));
        handle.report_error("example");
        assert!(matches!(
            handle.try_recv_event(),
            Some(AppEvent::StateChanged(snapshot)) if snapshot.last_error.as_deref() == Some("example")
        ));

        runtime.shutdown();
    }

    #[test]
    fn views_are_toolkit_independent() {
        let view = PlayingView {
            title: "A".to_owned(),
            artist: "B".to_owned(),
            cover: Some(CoverView {
                bytes: vec![1, 2, 3],
                format: CoverImageFormat::Png,
            }),
            playing: true,
            position_ms: 1,
            duration_ms: 2,
        };
        assert_eq!(
            view.cover.as_ref().map(|cover| cover.format),
            Some(CoverImageFormat::Png)
        );
        let event = AppEvent::Indicator(Indicator::Volume(VolumeView {
            percent: 7,
            muted: false,
        }));
        assert!(matches!(event, AppEvent::Indicator(Indicator::Volume(_))));
    }

    fn wait_for(
        handle: &RuntimeHandle,
        predicate: impl Fn(&RuntimeSnapshot) -> bool,
    ) -> RuntimeSnapshot {
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let mut last = handle.snapshot();
        while std::time::Instant::now() < deadline {
            last = handle.snapshot();
            if predicate(&last) {
                return last;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("expected runtime state was not reached; last snapshot: {last:?}");
    }
}
