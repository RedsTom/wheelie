//! Application runtime: observable state and command channel.
//!
//! M0 provides the lifecycle skeleton (start, pause, resume, stop). M2 wires
//! global hotkeys, the action worker and configuration persistence into it.

use super::error::AppError;
use crate::core::PlatformInfo;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};

/// Global runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuntimeState {
    /// The runtime is starting.
    #[default]
    Starting,
    /// Capture and actions are enabled.
    Running,
    /// Capture is suspended.
    Paused,
    /// The runtime is stopped.
    Stopped,
}

/// Immutable snapshot observed by the UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSnapshot {
    /// Current state.
    pub state: RuntimeState,
    /// Whether global hotkeys are currently registered.
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

/// Commands sent to the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    /// Suspend capture.
    Pause,
    /// Resume capture.
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
}

/// Cloneable handle used by the UI to observe and control the runtime.
#[derive(Clone)]
pub struct RuntimeHandle {
    snapshot: Arc<RwLock<RuntimeSnapshot>>,
    commands: Sender<AppCommand>,
    events: Receiver<AppEvent>,
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
}

impl Runtime {
    /// Starts the runtime.
    pub fn new() -> Result<Self, AppError> {
        let (command_tx, command_rx) = unbounded();
        let (event_tx, event_rx) = unbounded();
        let snapshot = Arc::new(RwLock::new(RuntimeSnapshot::default()));

        let worker_snapshot = Arc::clone(&snapshot);
        let worker = thread::Builder::new()
            .name("wheelie-runtime".to_owned())
            .spawn(move || run_loop(&command_rx, &event_tx, &worker_snapshot))?;

        let handle = RuntimeHandle {
            snapshot: Arc::clone(&snapshot),
            commands: command_tx.clone(),
            events: event_rx,
        };

        Ok(Self {
            handle,
            command_tx,
            worker: Some(worker),
        })
    }

    /// Handle passed to the UI and background services.
    pub fn handle(&self) -> RuntimeHandle {
        self.handle.clone()
    }

    /// Stops the runtime thread. Idempotent.
    pub fn shutdown(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = self.command_tx.send(AppCommand::Quit);
            let _ = worker.join();
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
        // Hotkey registration lands in M2.
        state.capture_active = false;
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
                state.capture_active = false;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn pause_and_resume_update_the_snapshot() {
        let mut runtime = Runtime::new().expect("runtime");
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
