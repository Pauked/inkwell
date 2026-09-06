use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub default_export_folder: PathBuf,
    #[serde(default)]
    pub enable_painter_highlights: bool,
}

impl Default for Config {
    fn default() -> Self {
        // Default to user's home directory under Documents/Inkwell
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Self {
            default_export_folder: PathBuf::from(home).join("Documents/Inkwell"),
            enable_painter_highlights: false,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // Try local config.toml first (in same directory as binary or current directory)
        let local_config = PathBuf::from("./config.toml");

        // Try binary directory config
        let binary_dir_config = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.join("config.toml")));

        // Try user config directory
        let user_config = Self::user_config_path().ok();

        // Check in order: local, binary dir, user config
        let config_path = if local_config.exists() {
            local_config
        } else if let Some(ref path) = binary_dir_config {
            if path.exists() {
                path.clone()
            } else if let Some(ref user_path) = user_config {
                user_path.clone()
            } else {
                return Ok(Config::default());
            }
        } else if let Some(ref user_path) = user_config {
            user_path.clone()
        } else {
            return Ok(Config::default());
        };

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)
                .context(format!("Failed to read config file: {:?}", config_path))?;
            let config: Config =
                toml::from_str(&contents).context("Failed to parse config file")?;
            Ok(config)
        } else {
            // Only auto-create in user config directory
            if let Some(user_path) = user_config {
                let config = Config::default();
                if let Some(parent) = user_path.parent() {
                    fs::create_dir_all(parent).ok();
                }
                if let Ok(contents) = toml::to_string_pretty(&config) {
                    fs::write(&user_path, contents).ok();
                }
                Ok(config)
            } else {
                Ok(Config::default())
            }
        }
    }

    fn user_config_path() -> Result<PathBuf> {
        let home = std::env::var("HOME").context("HOME environment variable not set")?;
        Ok(PathBuf::from(home).join(".config/inkwell/config.toml"))
    }
}
