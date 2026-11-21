//! Loads and saves the versioned TOML configuration.
//!
//! The file lives in the platform configuration directory and is written
//! atomically: a uniquely named temporary file is fully written, flushed and
//! synced first, then renamed over the real one and the parent directory is
//! synced. A document that cannot be parsed is kept next to the original as a
//! timestamped `config.toml.bak.<milliseconds>` copy and the defaults are used
//! instead, so a hand-edited or truncated file never prevents the application
//! from starting.

use super::error::AppError;
use crate::core::{Config, ConfigError};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::warn;

/// File name of the configuration document.
pub const CONFIG_FILE_NAME: &str = "config.toml";

static TEMPORARY_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    /// Loads the document, falling back to the platform defaults on any problem.
    pub fn load(&self) -> LoadedConfig {
        self.load_with_default(Config::default())
    }

    /// Loads the document, using `default` when no usable file exists.
    ///
    /// The caller passes defaults filtered for the platform, so a fresh install
    /// never offers an action the input backend cannot perform.
    pub fn load_with_default(&self, default: Config) -> LoadedConfig {
        let document = match fs::read_to_string(&self.path) {
            Ok(document) => document,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return LoadedConfig {
                    config: default,
                    warning: None,
                };
            }
            Err(error) => {
                let warning = format!("could not read {}: {error}", self.path.display());
                warn!(%warning, "configuration was not readable");
                return LoadedConfig {
                    config: default,
                    warning: Some(warning),
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
                warn!(%warning, "invalid configuration was replaced by defaults");
                LoadedConfig {
                    config: default,
                    warning: Some(warning),
                }
            }
        }
    }

    /// Validates and writes the document atomically.
    ///
    /// The temporary file carries the process id and a counter, so two runs (or
    /// two threads) can never race on the same name. Every error removes that
    /// temporary file before it is returned.
    pub fn save(&self, config: &Config) -> Result<(), AppError> {
        let document = config.to_toml_string()?;
        let parent = self.path.parent();
        if let Some(parent) = parent {
            fs::create_dir_all(parent)?;
        }

        let serial = TEMPORARY_COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!("config.{}.{serial}.toml.tmp", std::process::id());
        let temporary = self.path.with_file_name(name);

        let result = self.write_atomic(&temporary, document.as_bytes());
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    fn write_atomic(&self, temporary: &Path, document: &[u8]) -> Result<(), AppError> {
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(temporary)?;
            file.write_all(document)?;
            file.sync_all()?;
        }
        fs::rename(temporary, &self.path)?;

        // Syncing the directory makes the rename durable. Some filesystems
        // (notably on Windows) refuse to open a directory as a file; there the
        // rename itself is the best durability the adapter can offer.
        if let Some(parent) = self.path.parent()
            && let Ok(directory) = File::open(parent)
            && let Err(error) = directory.sync_all()
        {
            warn!(%error, path = %parent.display(), "could not sync the configuration directory");
        }
        Ok(())
    }

    /// Keeps an invalid document under a timestamped name and describes where
    /// it went. The warning always names a path: either the backup or, when no
    /// copy could be made, the original file that is still on disk.
    fn preserve_broken_copy(&self, error: &ConfigError) -> String {
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis())
            .unwrap_or(0);
        let backup = self.path.with_extension(format!("toml.bak.{millis}"));

        let backed_up: Result<(), std::io::Error> = match fs::rename(&self.path, &backup) {
            Ok(()) => Ok(()),
            Err(rename_error) => match fs::copy(&self.path, &backup) {
                Ok(_) => {
                    let _ = fs::remove_file(&self.path);
                    Ok(())
                }
                Err(_) => Err(rename_error),
            },
        };

        match backed_up {
            Ok(()) => format!(
                "invalid configuration ({error}); defaults restored, previous file kept at {}",
                backup.display()
            ),
            Err(copy_error) => format!(
                "invalid configuration ({error}); defaults restored, could not keep a copy of {}: {copy_error}",
                self.path.display()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Language;

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
            let path = std::env::temp_dir().join(format!("wheelie-{name}-{nanos}"));
            fs::create_dir_all(&path).expect("temp directory");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn missing_file_yields_defaults_without_warning() {
        let directory = TempDir::new("missing");
        let store = ConfigStore::new(directory.path().join(CONFIG_FILE_NAME));
        let loaded = store.load();
        assert_eq!(loaded.config, Config::default());
        assert!(loaded.warning.is_none());
    }

    #[test]
    fn saved_config_round_trips() {
        let directory = TempDir::new("roundtrip");
        let store = ConfigStore::new(directory.path().join(CONFIG_FILE_NAME));

        let config = Config {
            language: Language::En,
            ..Default::default()
        };
        store.save(&config).expect("save");
        store
            .save(&config)
            .expect("save again over an existing file");

        let loaded = store.load();
        assert_eq!(loaded.config, config);
        assert!(loaded.warning.is_none());
        assert!(
            fs::read_dir(directory.path())
                .expect("read directory")
                .all(|entry| !entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .contains("tmp"))
        );
    }

    #[test]
    fn broken_file_is_backed_up_with_a_timestamp_and_defaults_are_used() {
        let directory = TempDir::new("broken");
        let path = directory.path().join(CONFIG_FILE_NAME);
        let store = ConfigStore::new(&path);
        fs::write(&path, "this is not toml").expect("write");

        let loaded = store.load();
        assert_eq!(loaded.config, Config::default());
        let warning = loaded.warning.expect("warning");
        assert!(warning.contains("config.toml.bak."), "{warning}");
        assert!(!path.exists());

        let backups: Vec<_> = fs::read_dir(directory.path())
            .expect("read directory")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.starts_with("config.toml.bak."))
            .collect();
        assert_eq!(backups.len(), 1);
        assert!(warning.contains(&backups[0]), "{warning}");
    }

    #[test]
    fn unknown_fields_do_not_break_the_document() {
        let directory = TempDir::new("unknown");
        let path = directory.path().join(CONFIG_FILE_NAME);
        let store = ConfigStore::new(&path);

        let mut document = Config::default().to_toml_string().expect("serialize");
        document.push_str("\nfuture_option = true\n");
        fs::write(&path, document).expect("write");

        let loaded = store.load();
        assert!(loaded.warning.is_none());
        assert_eq!(loaded.config, Config::default());
    }
}
