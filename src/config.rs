//! Configuration module for the GUI library.
//!
//! This module contains the `AppConfig` struct which defines all configurable
//! options for the application window and webview.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub with_decorations: bool,
    pub dev_url: Option<String>,
    pub build_path: PathBuf,
    pub with_devtools: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            with_decorations: false,
            dev_url: None,
            build_path: PathBuf::from("frontend/build"),
            with_devtools: true,
        }
    }
}

impl AppConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn development(url: impl Into<String>) -> Self {
        Self {
            dev_url: Some(url.into()),
            with_devtools: true,
            ..Default::default()
        }
    }

    pub fn production(build_path: impl Into<PathBuf>) -> Self {
        Self {
            dev_url: None,
            build_path: build_path.into(),
            with_devtools: false,
            ..Default::default()
        }
    }

    pub fn is_development(&self) -> bool {
        self.dev_url.is_some()
    }

    pub fn is_production(&self) -> bool {
        !self.is_development()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert!(!config.with_decorations);
        assert!(config.dev_url.is_none());
        assert!(config.with_devtools);
        assert_eq!(config.build_path, PathBuf::from("frontend/build"));
    }

    #[test]
    fn test_development_config() {
        let config = AppConfig::development("http://localhost:5173");
        assert!(config.is_development());
        assert!(!config.is_production());
        assert_eq!(config.dev_url, Some("http://localhost:5173".into()));
        assert!(config.with_devtools);
    }

    #[test]
    fn test_production_config() {
        let config = AppConfig::production("dist");
        assert!(config.is_production());
        assert!(!config.is_development());
        assert!(config.dev_url.is_none());
        assert!(!config.with_devtools);
        assert_eq!(config.build_path, PathBuf::from("dist"));
    }
}
