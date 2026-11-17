//! Persisted appearance preference.

use serde::{Deserialize, Serialize};

/// Application color theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Dark theme.
    #[default]
    Dark,
    /// Light theme.
    Light,
}

impl ThemeMode {
    /// All modes, in display order.
    pub const ALL: [Self; 2] = [Self::Dark, Self::Light];
}
