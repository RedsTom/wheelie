//! Loads and saves the versioned TOML configuration.
//!
//! The file lives in the platform configuration directory and is written
//! atomically: a temporary file is fully written first, then renamed over the
//! real one. A document that cannot be parsed is kept next to the original as
//! `config.toml.bak` and the defaults are used instead, so a hand-edited or
//! truncated file never prevents the application from starting.

use super::error::AppError;
use crate::core::{Config, ConfigError};
use std::fs;
use std::path::{Path, PathBuf};

/// File name of the configuration document.
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// Outcome of reading the configuration at startup.
#[derive(Debug)]
pub struct LoadedConfig {
    /// The configuration to run with.
    pub config: Config,
    /// Human-readable warning when the file could not be used.
    pub warning: Option<String>,
}

/// Reads and writes one configuration file.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    /// Store for an explicit path, used by tests.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Store at the platform configuration location.
    pub fn at_default_location() -> Result<Self, AppError> {
        let directory = crate::platform::config_dir()?;
        Ok(Self::new(directory.join(CONFIG_FILE_NAME)))
    }

    /// Absolute path of the document.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads the document, falling back to defaults on any problem.
    pub fn load(&self) -> LoadedConfig {
        let document = match fs::read_to_string(&self.path) {
            Ok(document) => document,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return LoadedConfig {
                    config: Config::default(),
                    warning: None,
                };
            }
            Err(error) => {
                return LoadedConfig {
                    config: Config::default(),
                    warning: Some(format!("could not read {}: {error}", self.path.display())),
                };
            }
        };

        match Config::from_toml_str(&document) {
            Ok(config) => LoadedConfig {
                config,
                warning: None,
            },
            Err(error) => {
                let warning = self.preserve_broken_copy(&error);
                LoadedConfig {
                    config: Config::default(),
                    warning: Some(warning),
                }
            }
        }
    }

    /// Validates and writes the document atomically.
    pub fn save(&self, config: &Config) -> Result<(), AppError> {
        let document = config.to_toml_string()?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temporary = self.path.with_extension("toml.tmp");
        fs::write(&temporary, document)?;
        fs::rename(&temporary, &self.path)?;
        Ok(())
    }

    fn preserve_broken_copy(&self, error: &ConfigError) -> String {
        let backup = self.path.with_extension("toml.bak");
        let moved = fs::rename(&self.path, &backup);
        match moved {
            Ok(()) => format!(
                "invalid configuration ({error}); defaults restored, previous file kept at {}",
                backup.display()
            ),
            Err(backup_error) => format!(
                "invalid configuration ({error}); defaults restored, could not keep a copy: {backup_error}"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Language;

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("wheelie-{name}-{nanos}"));
        fs::create_dir_all(&directory).expect("temp directory");
        directory
    }

    #[test]
    fn missing_file_yields_defaults_without_warning() {
        let directory = temp_dir("missing");
        let store = ConfigStore::new(directory.join(CONFIG_FILE_NAME));
        let loaded = store.load();
        assert_eq!(loaded.config, Config::default());
        assert!(loaded.warning.is_none());
    }

    #[test]
    fn saved_config_round_trips() {
        let directory = temp_dir("roundtrip");
        let store = ConfigStore::new(directory.join(CONFIG_FILE_NAME));

        let config = Config {
            language: Language::En,
            ..Default::default()
        };
        store.save(&config).expect("save");

        let loaded = store.load();
        assert_eq!(loaded.config, config);
        assert!(loaded.warning.is_none());
    }

    #[test]
    fn broken_file_is_backed_up_and_defaults_are_used() {
        let directory = temp_dir("broken");
        let path = directory.join(CONFIG_FILE_NAME);
        let store = ConfigStore::new(&path);
        fs::write(&path, "this is not toml").expect("write");

        let loaded = store.load();
        assert_eq!(loaded.config, Config::default());
        assert!(loaded.warning.is_some());
        assert!(path.with_extension("toml.bak").exists());
        assert!(!path.exists());
    }

    #[test]
    fn unknown_fields_do_not_break_the_document() {
        let directory = temp_dir("unknown");
        let path = directory.join(CONFIG_FILE_NAME);
        let store = ConfigStore::new(&path);

        let mut document = Config::default().to_toml_string().expect("serialize");
        document.push_str("\nfuture_option = true\n");
        fs::write(&path, document).expect("write");

        let loaded = store.load();
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.config, Config::default());
    }
}
