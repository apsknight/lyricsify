use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::error::{LyricsifyError, Result};

/// Application configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Window position as (x, y) coordinates
    pub window_position: (f64, f64),
    
    /// Whether the overlay is currently visible
    pub overlay_visible: bool,
    
    /// Polling interval in seconds for Spotify API
    pub poll_interval_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            // Default to top-right corner (will be adjusted based on screen size)
            window_position: (100.0, 100.0),
            overlay_visible: true,
            poll_interval_secs: 5,
        }
    }
}

impl AppConfig {
    /// Get the path to the config directory
    fn config_dir() -> Result<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| LyricsifyError::ConfigError("HOME environment variable not set".to_string()))?;
        
        let config_path = PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("com.lyricsify");
        
        Ok(config_path)
    }
    
    /// Get the path to the config file
    fn config_file_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.json"))
    }
    
    /// Load configuration from disk, or return default if file doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_file_path()?;
        
        if !config_path.exists() {
            log::info!("Config file not found, using defaults");
            return Ok(Self::default());
        }
        
        let contents = fs::read_to_string(&config_path)
            .map_err(|e| LyricsifyError::ConfigError(format!("Failed to read config file: {}", e)))?;
        
        match serde_json::from_str(&contents) {
            Ok(config) => {
                log::info!("Loaded configuration from {:?}", config_path);
                Ok(config)
            }
            Err(e) => {
                log::warn!("Failed to parse config file ({}), using defaults", e);
                Ok(Self::default())
            }
        }
    }
    
    /// Save configuration to disk
    pub fn save(&self) -> Result<()> {
        let config_dir = Self::config_dir()?;
        let config_path = Self::config_file_path()?;
        
        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .map_err(|e| LyricsifyError::ConfigError(format!("Failed to create config directory: {}", e)))?;
            log::info!("Created config directory at {:?}", config_dir);
        }
        
        // Serialize config to JSON
        let json = serde_json::to_string_pretty(self)?;
        
        // Write to file
        fs::write(&config_path, json)
            .map_err(|e| LyricsifyError::ConfigError(format!("Failed to write config file: {}", e)))?;
        
        log::info!("Saved configuration to {:?}", config_path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.window_position, (100.0, 100.0));
        assert_eq!(config.overlay_visible, true);
        assert_eq!(config.poll_interval_secs, 5);
    }
    
    #[test]
    fn test_config_serialization() {
        let config = AppConfig {
            window_position: (200.0, 300.0),
            overlay_visible: false,
            poll_interval_secs: 10,
        };
        
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.window_position, config.window_position);
        assert_eq!(deserialized.overlay_visible, config.overlay_visible);
        assert_eq!(deserialized.poll_interval_secs, config.poll_interval_secs);
    }
}
