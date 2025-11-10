use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use crate::taocket_window::AssetProvider;

type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct WindowSize {
    pub width: f64,
    pub height: f64,
}

impl Default for WindowSize {
    fn default() -> Self {
        Self {
            width: 300.0,
            height: 600.0,
        }
    }
}

impl From<(f64, f64)> for WindowSize {
    fn from((width, height): (f64, f64)) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct TaocketConfig {
    #[serde(skip)]
    config_path: PathBuf,
    pub dev_url: String,
    pub build_path: PathBuf,
    pub websocket_port: u16,
    pub devtools: bool,
    pub top_most: bool,

    #[serde(default)]
    pub size: WindowSize,

    #[serde(default)]
    pub keys: HashMap<String, String>,
}

impl Default for TaocketConfig {
    fn default() -> Self {
        Self {
            config_path: PathBuf::from("taocket_config.toml"),
            dev_url: "http://localhost:5173".into(),
            build_path: PathBuf::from("./frontend"),
            websocket_port: 1818,
            devtools: true,
            top_most: false,
            size: WindowSize::default(),
            keys: HashMap::new(),
        }
    }
}

impl TaocketConfig {
    /// Load configuration from file or create default if not exists
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        let mut config = if path.exists() {
            Self::read_from_file(path)?
        } else {
            log::info!("Config file not found at {:?}, creating default", path);
            let default = Self::default();
            default.write_to_file(path)?;
            default
        };

        config.config_path = path.to_path_buf();
        Ok(config)
    }

    /// Read configuration from file
    fn read_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config = toml::from_str(&content)?;
        Ok(config)
    }

    /// Write configuration to file
    fn write_to_file(&self, path: &Path) -> Result<()> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_str = toml::to_string_pretty(self)?;
        let mut file = fs::File::create(path)?;
        file.write_all(toml_str.as_bytes())?;

        log::info!("Config written to {:?}", path);
        Ok(())
    }

    /// Add or update a hotkey binding
    pub fn add_hotkey(
        &mut self,
        key: impl Into<String>,
        func: impl Into<String>,
    ) -> Option<String> {
        let key = key.into();
        let func = func.into();

        let previous = self.keys.insert(key.clone(), func);

        if previous.is_some() {
            log::debug!("Updated hotkey binding: {}", key);
        } else {
            log::debug!("Added new hotkey binding: {}", key);
        }

        previous
    }

    /// Remove a hotkey binding
    pub fn remove_hotkey(&mut self, key: &str) -> Option<String> {
        self.keys.remove(key)
    }

    /// Get a hotkey binding
    pub fn get_hotkey(&self, key: &str) -> Option<&String> {
        self.keys.get(key)
    }

    /// Save current configuration to disk
    pub fn save(&self) -> Result<()> {
        self.write_to_file(&self.config_path)
    }

    /// Get the configuration file path
    pub fn path(&self) -> &Path {
        &self.config_path
    }

    /// Validate configuration
    pub fn validate(&self) -> std::result::Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.size.width <= 0.0 || self.size.height <= 0.0 {
            errors.push("Window size must be positive".to_string());
        }

        if self.dev_url.is_empty() {
            errors.push("dev_url cannot be empty".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// Builder pattern for easier configuration construction
pub struct TaocketConfigBuilder {
    config: TaocketConfig,
}

impl TaocketConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: TaocketConfig::default(),
        }
    }

    pub fn dev_url(mut self, url: impl Into<String>) -> Self {
        self.config.dev_url = url.into();
        self
    }

    pub fn build_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.build_path = path.into();
        self
    }

    pub fn devtools(mut self, enabled: bool) -> Self {
        self.config.devtools = enabled;
        self
    }

    pub fn top_most(mut self, enabled: bool) -> Self {
        self.config.top_most = enabled;
        self
    }

    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.config.size = WindowSize { width, height };
        self
    }

    pub fn hotkey(mut self, key: impl Into<String>, func: impl Into<String>) -> Self {
        self.config.add_hotkey(key, func);
        self
    }

    pub fn build(self) -> TaocketConfig {
        self.config
    }
}

impl Default for TaocketConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_create_default() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("taocket.toml");
        let config = TaocketConfig::load(&config_path).unwrap();
        assert_eq!(config.dev_url, "http://localhost:5173");
        assert!(config_path.exists());
    }

    #[test]
    fn test_add_hotkey() {
        let mut config = TaocketConfig::default();

        let previous = config.add_hotkey("ctrl+b", "something::UseFull");
        assert!(previous.is_none());

        let previous = config.add_hotkey("ctrl+b", "something::Different");
        assert_eq!(previous, Some("something::UseFull".to_string()));
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("taocket.toml");

        let mut config = TaocketConfig::load(&config_path).unwrap();
        config.add_hotkey("ctrl+c", "something::Copy");
        config.save().unwrap();

        let loaded = TaocketConfig::load(&config_path).unwrap();
        assert_eq!(
            loaded.get_hotkey("ctrl+c"),
            Some(&"something::Copy".to_string())
        );
    }

    #[test]
    fn test_builder() {
        let config = TaocketConfigBuilder::new()
            .dev_url("http://localhost:3000")
            .size(800.0, 600.0)
            .hotkey("ctrl+s", "save")
            .devtools(false)
            .build();

        assert_eq!(config.dev_url, "http://localhost:3000");
        assert_eq!(config.size.width, 800.0);
        assert!(!config.devtools);
        assert_eq!(config.get_hotkey("ctrl+s"), Some(&"save".to_string()));
    }

    #[test]
    fn test_validation() {
        let mut config = TaocketConfig::default();
        assert!(config.validate().is_ok());

        config.size.width = -100.0;
        assert!(config.validate().is_err());
    }
}
