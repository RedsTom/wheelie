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
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn second_instance_is_detected() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let id = format!("wheelie-test-{nanos}");

        let first = InstanceGuard::acquire(&id).expect("first instance");
        assert!(first.is_single());

        let second = InstanceGuard::acquire(&id).expect("second instance");
        assert!(!second.is_single());

        drop(first);
        drop(second);
    }
}
