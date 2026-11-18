//! Background worker that executes knob actions.
//!
//! The worker owns the input sink and the injection guard so the UI thread
//! never blocks on synthesized input. Every job runs inside an injection scope,
//! which the runtime uses to discard hotkey events produced by the injected
//! keys themselves.

use super::error::AppError;
use super::runtime::{AppEvent, Indicator};
use crate::core::action::Action;
use crate::core::macro_engine::{MacroDefinition, MacroSink, run_macro};
use crate::core::{Config, InputEvent};
use crate::platform::volume;
use crate::platform::{EnigoInputSink, InjectionGuard, InputSink};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{self, JoinHandle};
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

    while let Ok(job) = jobs.recv() {
        match job {
            ActionJob::Quit => break,
            ActionJob::Execute(action) => {
                let macros = read_config(config).macros.clone();
                cancel.store(false, Ordering::Relaxed);
                match execute_action(&mut sink, guard, &macros, &action, &cancel) {
                    Err(message) => {
                        warn!(%message, "action failed");
                        let _ = events.send(AppEvent::Error(message));
                    }
                    Ok(reported) => {
                        let indicator = match &action {
                            Action::Volume { .. } | Action::Mute => {
                                volume::level().ok().map(Indicator::Volume)
                            }
                            Action::Brightness { .. } => reported.map(Indicator::Brightness),
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
