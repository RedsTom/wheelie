//! Application runtime: observable state, configuration and action execution.
//!
//! The runtime keeps the pieces that are shared between the UI thread and the
//! background action worker: the versioned configuration document, the
//! injection guard and a queue of actions. The UI thread owns the OS adapters
//! that need its event loop (global keys and the tray icon), while the action
//! worker owns input synthesis.

use super::action_worker::{self, ActionQueue};
use super::config_store::{ConfigStore, LoadedConfig};
use super::error::AppError;
use crate::core::{Action, Config, PlatformInfo};
use crate::platform::{InjectionGuard, PlatformError};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
use tracing::warn;

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
}

impl Default for RuntimeSnapshot {
    fn default() -> Self {
        Self {
            state: RuntimeState::Starting,
            capture_active: false,
            active_profile: None,
            last_error: None,
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
    /// A volume or brightness change the UI shows on screen.
    Indicator(Indicator),
}

/// Level reported after a volume or brightness change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indicator {
    /// System output volume and mute state.
    Volume(crate::platform::volume::Level),
    /// Screen brightness in percent.
    Brightness(u8),
}

/// Cloneable handle used by the UI to observe and control the runtime.
#[derive(Clone)]
pub struct RuntimeHandle {
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
    commands: Sender<AppCommand>,
    events: Receiver<AppEvent>,
    config: Arc<RwLock<Config>>,
    store: ConfigStore,
    guard: InjectionGuard,
    actions: ActionQueue,
}

impl RuntimeHandle {
    /// Latest known state.
    pub fn snapshot(&self) -> RuntimeSnapshot {
        read_snapshot(&self.snapshot).clone()
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
        read_config(&self.config).clone()
    }

    /// Applies a change, validates it and persists it.
    ///
    /// The in-memory document is only replaced when the candidate is valid, so
    /// an invalid edit never takes effect.
    pub fn update_config(&self, update: impl FnOnce(&mut Config)) -> Result<(), AppError> {
        let mut current = write_config(&self.config);
        let mut candidate = current.clone();
        update(&mut candidate);
        candidate.validate()?;
        self.store.save(&candidate)?;
        *current = candidate;
        Ok(())
    }

    /// Queues an action for the background worker.
    pub fn execute(&self, action: Action) -> Result<(), AppError> {
        self.actions.execute(action)
    }

    /// Guard shared with the action worker.
    pub fn injection_guard(&self) -> InjectionGuard {
        self.guard.clone()
    }

    /// Records whether knob keys are currently watched.
    pub fn set_capture_active(&self, active: bool) {
        write_snapshot(&self.snapshot).capture_active = active;
    }

    /// Records a user-facing error without stopping the runtime.
    pub fn report_error(&self, message: impl Into<String>) {
        write_snapshot(&self.snapshot).last_error = Some(message.into());
    }

    /// Enables or disables the start-at-login entry.
    pub fn set_start_at_login(&self, enabled: bool) -> Result<(), PlatformError> {
        let executable = std::env::current_exe()?;
        let service = crate::platform::AutoStartService::new(
            crate::core::APP_ID,
            &executable,
            &["--autostart"],
        )?;
        if enabled {
            service.enable()
        } else {
            service.disable()
        }
    }

    /// Whether the start-at-login entry currently exists.
    pub fn start_at_login_enabled(&self) -> Option<bool> {
        let executable = std::env::current_exe().ok()?;
        let service =
            crate::platform::AutoStartService::new(crate::core::APP_ID, &executable, &[]).ok()?;
        service.is_enabled().ok()
    }

    /// Path of the configuration document, for display in the About page.
    pub fn config_path(&self) -> PathBuf {
        self.store.path().to_path_buf()
    }

    /// Static platform capabilities.
    ///
    /// Exposed through the application layer so `wheelie-ui` never depends on
    /// `wheelie-platform` directly.
    pub fn platform_info(&self) -> PlatformInfo {
        crate::platform::info()
    }
}

/// Runtime owned by the binary. Keep it alive for the whole UI lifetime.
pub struct Runtime {
    handle: RuntimeHandle,
    command_tx: Sender<AppCommand>,
    worker: Option<JoinHandle<()>>,
    action_worker: Option<JoinHandle<()>>,
}

impl Runtime {
    /// Starts the runtime with the configuration at the default location.
    pub fn new() -> Result<Self, AppError> {
        Self::with_store(ConfigStore::at_default_location()?)
    }

    /// Starts the runtime with an explicit configuration store.
    pub fn with_store(store: ConfigStore) -> Result<Self, AppError> {
        let (command_tx, command_rx) = unbounded();
        let (event_tx, event_rx) = unbounded();
        let snapshot = Arc::new(RwLock::new(RuntimeSnapshot::default()));

        let platform = crate::platform::info();
        let LoadedConfig {
            mut config,
            warning,
        } = store.load_with_default(Config::default_with(platform.brightness_supported));

        if !platform.brightness_supported {
            config.remove_default_brightness_set();
        }
        if let Some(warning) = &warning {
            warn!(%warning, "configuration was not usable");
            let mut current = write_snapshot(&snapshot);
            current.last_error = Some(warning.clone());
        }

        let config = Arc::new(RwLock::new(config));
        let active_profile = read_config(&config).active_profile.clone();
        {
            let mut current = write_snapshot(&snapshot);
            current.active_profile = active_profile;
        }

        let guard = InjectionGuard::default();
        let (actions, action_worker) =
            action_worker::spawn(Arc::clone(&config), guard.clone(), event_tx.clone())?;

        let worker_snapshot = Arc::clone(&snapshot);
        let worker = thread::Builder::new()
            .name("wheelie-runtime".to_owned())
            .spawn(move || run_loop(&command_rx, &event_tx, &worker_snapshot))?;

        let handle = RuntimeHandle {
            snapshot: Arc::clone(&snapshot),
            commands: command_tx.clone(),
            events: event_rx,
            config,
            store,
            guard,
            actions: actions.clone(),
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

    /// Stops the runtime threads. Idempotent.
    pub fn shutdown(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = self.command_tx.send(AppCommand::Quit);
            let _ = worker.join();
        }
        self.handle.actions.shutdown();
        if let Some(action_worker) = self.action_worker.take() {
            let _ = action_worker.join();
        }
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
    {
        let mut state = write_snapshot(snapshot);
        state.state = RuntimeState::Running;
        let _ = events.send(AppEvent::StateChanged(state.clone()));
    }

    while let Ok(command) = commands.recv() {
        let mut state = write_snapshot(snapshot);
        match command {
            AppCommand::Pause => {
                state.state = RuntimeState::Paused;
                state.capture_active = false;
            }
            AppCommand::Resume => {
                state.state = RuntimeState::Running;
            }
            AppCommand::Quit => {
                state.state = RuntimeState::Stopped;
                state.capture_active = false;
                let _ = events.send(AppEvent::StateChanged(state.clone()));
                break;
            }
        }
        let _ = events.send(AppEvent::StateChanged(state.clone()));
    }
}

fn read_snapshot(
    lock: &RwLock<RuntimeSnapshot>,
) -> std::sync::RwLockReadGuard<'_, RuntimeSnapshot> {
    lock.read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write_snapshot(
    lock: &RwLock<RuntimeSnapshot>,
) -> std::sync::RwLockWriteGuard<'_, RuntimeSnapshot> {
    lock.write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn read_config(lock: &RwLock<Config>) -> std::sync::RwLockReadGuard<'_, Config> {
    lock.read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn write_config(lock: &RwLock<Config>) -> std::sync::RwLockWriteGuard<'_, Config> {
    lock.write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn temp_store(name: &str) -> ConfigStore {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("wheelie-runtime-{name}-{nanos}"));
        ConfigStore::new(directory.join("config.toml"))
    }

    #[test]
    fn pause_and_resume_update_the_snapshot() {
        let mut runtime = Runtime::with_store(temp_store("pause")).expect("runtime");
        let handle = runtime.handle();

        let started = wait_for(&handle, |snapshot| snapshot.state == RuntimeState::Running);
        assert!(!started.capture_active);

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
        let store = temp_store("update");
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
        let store = temp_store("restart");

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
    fn invalid_configuration_on_disk_falls_back_to_defaults() {
        let store = temp_store("broken");
        std::fs::create_dir_all(store.path().parent().expect("parent")).expect("directory");
        std::fs::write(store.path(), "not = { valid").expect("write");

        let mut runtime = Runtime::with_store(store.clone()).expect("runtime");
        let handle = runtime.handle();
        assert!(handle.snapshot().last_error.is_some());
        let expected = Config::default_with(crate::platform::info().brightness_supported);
        assert_eq!(handle.config(), expected);

        runtime.shutdown();
    }

    fn wait_for(
        handle: &RuntimeHandle,
        predicate: impl Fn(&RuntimeSnapshot) -> bool,
    ) -> RuntimeSnapshot {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            let snapshot = handle.snapshot();
            if predicate(&snapshot) {
                return snapshot;
            }
            thread::sleep(Duration::from_millis(1));
        }
        panic!("expected runtime state was not reached");
    }
}
