//! Guard that suppresses hotkey events while input is being injected.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Shared flag used to break the injection feedback loop.
///
/// The action worker wraps every injection in [`InjectionGuard::scope`]; the
/// runtime drains hotkey events while the guard is active so an injected key
/// never triggers its own action.
#[derive(Clone, Default)]
pub struct InjectionGuard {
    active: Arc<AtomicBool>,
}

impl InjectionGuard {
    /// Marks injection as active.
    pub fn begin(&self) {
        self.active.store(true, Ordering::SeqCst);
    }

    /// Marks injection as finished.
    pub fn end(&self) {
        self.active.store(false, Ordering::SeqCst);
    }

    /// Whether injection is currently active.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Returns an RAII scope that clears the flag when dropped.
    pub fn scope(&self) -> InjectionScope<'_> {
        self.begin();
        InjectionScope { guard: self }
    }
}

/// RAII helper around [`InjectionGuard::begin`] and [`InjectionGuard::end`].
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

        {
            let _scope = guard.scope();
            assert!(guard.is_active());
        }

        assert!(!guard.is_active());
    }

    #[test]
    fn manual_begin_and_end_are_idempotent() {
        let guard = InjectionGuard::default();
        guard.begin();
        guard.begin();
        assert!(guard.is_active());
        guard.end();
        guard.end();
        assert!(!guard.is_active());
    }
}
