//! Single-instance guard.

use crate::platform::error::PlatformError;
use single_instance::SingleInstance;

/// Keeps at most one Wheelie process alive per user session.
pub struct InstanceGuard {
    instance: SingleInstance,
}

impl InstanceGuard {
    /// Acquires the named instance slot.
    pub fn acquire(id: &str) -> Result<Self, PlatformError> {
        let instance =
            SingleInstance::new(id).map_err(|error| PlatformError::Other(error.to_string()))?;
        Ok(Self { instance })
    }

    /// Whether this process owns the slot.
    pub fn is_single(&self) -> bool {
        self.instance.is_single()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_SLOT: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn second_instance_is_detected() {
        // The process id keeps parallel test binaries apart; the counter keeps
        // repeated tests in the same binary apart without relying on the clock.
        let slot = TEST_SLOT.fetch_add(1, Ordering::Relaxed);
        let id = format!("wheelie-test-{}-{slot}", std::process::id());

        let first = InstanceGuard::acquire(&id).expect("first instance");
        assert!(first.is_single());

        let second = InstanceGuard::acquire(&id).expect("second instance");
        assert!(!second.is_single());

        drop(first);
        drop(second);
    }
}
