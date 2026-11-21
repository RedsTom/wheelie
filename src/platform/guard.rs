//! Guard that suppresses hotkey events while input is being injected.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Extra time after the last injection during which events are still dropped.
///
/// Injected keys can be observed by the event loop slightly after the call
/// that sent them. Keeping a short quiet window closes that gap without
/// holding the hotkeys for an unbounded time.
const QUIET_PERIOD: Duration = Duration::from_millis(100);

/// Shared state used to break the injection feedback loop.
///
/// The action worker wraps every injection in [`InjectionGuard::scope`]; the
/// runtime drains hotkey events while the guard is active and for one quiet
/// window afterwards, so an injected key never triggers its own action.
#[derive(Clone, Default)]
pub struct InjectionGuard {
    scopes: Arc<AtomicUsize>,
    last_end: Arc<std::sync::Mutex<Option<Instant>>>,
}

impl InjectionGuard {
    /// Marks one more injection as active.
    ///
    /// Internal: callers should use [`InjectionGuard::scope`] so the matching
    /// [`InjectionGuard::end`] cannot be forgotten.
    fn begin(&self) {
        self.scopes.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut last_end) = self.last_end.lock() {
            *last_end = None;
        }
    }

    /// Marks one injection as finished.
    ///
    /// Internal: callers should use [`InjectionGuard::scope`].
    fn end(&self) {
        let previous = self
            .scopes
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                Some(count.saturating_sub(1))
            })
            .unwrap_or(0);
        if previous == 1
            && let Ok(mut last_end) = self.last_end.lock()
        {
            *last_end = Some(Instant::now());
        }
    }

    /// Whether an injection is currently in progress.
    pub fn is_active(&self) -> bool {
        self.scopes.load(Ordering::SeqCst) > 0
    }

    /// Whether hotkey events may be delivered again.
    ///
    /// False while any scope is open and for [`QUIET_PERIOD`] after the last
    /// scope closed.
    pub fn is_quiet(&self) -> bool {
        if self.is_active() {
            return false;
        }
        let last_end = self
            .last_end
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match *last_end {
            Some(end) => end.elapsed() >= QUIET_PERIOD,
            None => true,
        }
    }

    /// Returns an RAII scope that clears one level when dropped.
    pub fn scope(&self) -> InjectionScope<'_> {
        self.begin();
        InjectionScope { guard: self }
    }
}

/// RAII helper that ends one injection scope when it is dropped.
pub struct InjectionScope<'a> {
    guard: &'a InjectionGuard,
}

impl Drop for InjectionScope<'_> {
    fn drop(&mut self) {
        self.guard.end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_clears_the_flag_on_drop() {
        let guard = InjectionGuard::default();
        assert!(!guard.is_active());
        assert!(guard.is_quiet());

        {
            let _scope = guard.scope();
            assert!(guard.is_active());
            assert!(!guard.is_quiet());
        }

        assert!(!guard.is_active());
    }

    #[test]
    fn nested_scopes_keep_the_guard_active_until_the_last_drop() {
        let guard = InjectionGuard::default();
        let outer = guard.scope();
        let inner = guard.scope();

        drop(inner);
        assert!(guard.is_active(), "the outer scope still injects");

        drop(outer);
        assert!(!guard.is_active());
    }

    #[test]
    fn an_extra_end_cannot_underflow_nested_scopes() {
        let guard = InjectionGuard::default();
        guard.end();
        guard.end();
        assert!(!guard.is_active());
        assert!(guard.is_quiet());

        let scope = guard.scope();
        drop(scope);
        guard.end();
        assert!(!guard.is_active());
    }

    #[test]
    fn quiet_period_ends_after_the_grace_window() {
        let guard = InjectionGuard::default();
        drop(guard.scope());
        assert!(
            !guard.is_quiet(),
            "a just-finished injection is still noisy"
        );

        let mut last_end = guard.last_end.lock().expect("lock");
        *last_end = Some(Instant::now() - QUIET_PERIOD - Duration::from_millis(1));
        drop(last_end);
        assert!(guard.is_quiet());
    }
}
