#![allow(dead_code)]

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_monitor_port")]
    pub monitor_port: u16,
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            monitor_port: default_monitor_port(),
            theme: default_theme(),
        }
    }
}

fn default_monitor_port() -> u16 {
    4240
}

fn default_theme() -> String {
    "dark".into()
}

impl AppConfig {
    pub fn load(path: Option<&PathBuf>) -> Result<Self, String> {
        let _ = path; // Stub: return default
        Ok(Self::default())
    }
}
