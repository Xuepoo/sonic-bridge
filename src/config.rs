use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct SonicConfig {
    pub step_size: f32,
    pub onset_mode: bool,
    pub onset_threshold: f32,
    pub cache_dir: String,
}

impl Default for SonicConfig {
    fn default() -> Self {
        Self {
            step_size: 5.0,
            onset_mode: false,
            onset_threshold: 0.5,
            cache_dir: get_xdg_cache_path().to_str().unwrap_or("").to_string(),
        }
    }
}

pub fn get_xdg_config_path() -> PathBuf {
    let mut path = if let Ok(xdg_home) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_home)
    } else if let Ok(home) = env::var("HOME") {
        let mut p = PathBuf::from(home);
        p.push(".config");
        p
    } else {
        PathBuf::from(".")
    };
    path.push("sonic-bridge");
    path.push("config.toml");
    path
}

pub fn get_xdg_cache_path() -> PathBuf {
    let mut path = if let Ok(xdg_cache) = env::var("XDG_CACHE_HOME") {
        PathBuf::from(xdg_cache)
    } else if let Ok(home) = env::var("HOME") {
        let mut p = PathBuf::from(home);
        p.push(".cache");
        p
    } else {
        PathBuf::from(".")
    };
    path.push("sonic-bridge");
    path
}

impl SonicConfig {
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let config: Self = toml::from_str(&content).map_err(|e| e.to_string())?;
        Ok(config)
    }

    pub fn load_or_default() -> Self {
        let path = get_xdg_config_path();
        if path.exists() {
            Self::load_from_file(&path).unwrap_or_else(|_| Self::default())
        } else {
            Self::default()
        }
    }
}
