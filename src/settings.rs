use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DesktopSettings {
    #[serde(default)]
    pub keep_reth_running_in_background: bool,
    #[serde(default)]
    pub custom_launch_args: Vec<String>,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            keep_reth_running_in_background: false,
            custom_launch_args: Vec::new(),
        }
    }
}

/// Desktop settings manager for persistent configuration
pub struct DesktopSettingsManager;

impl DesktopSettingsManager {
    /// Get the path to the settings.toml file
    pub fn get_settings_file_path() -> PathBuf {
        // Place settings.toml in the same directory as the reth binary
        dirs::home_dir()
            .unwrap_or_default()
            .join(".reth-desktop")
            .join("settings.toml")
    }
    
    /// Load desktop settings from settings.toml
    pub fn load_desktop_settings() -> DesktopSettings {
        let settings_path = Self::get_settings_file_path();
        
        match std::fs::read_to_string(&settings_path) {
            Ok(content) => {
                match toml::from_str::<DesktopSettings>(&content) {
                    Ok(settings) => {
                        println!("Loaded desktop settings from: {}", settings_path.display());
                        settings
                    }
                    Err(e) => {
                        eprintln!("Failed to parse settings.toml: {}, using defaults", e);
                        DesktopSettings::default()
                    }
                }
            }
            Err(_) => {
                println!("No settings.toml found, creating with defaults at: {}", settings_path.display());
                let default_settings = DesktopSettings::default();
                // Try to create the settings file with defaults
                if let Err(e) = Self::save_desktop_settings(&default_settings) {
                    eprintln!("Failed to create default settings.toml: {}", e);
                }
                default_settings
            }
        }
    }
    
    /// Save desktop settings to settings.toml
    pub fn save_desktop_settings(settings: &DesktopSettings) -> Result<(), Box<dyn std::error::Error>> {
        let settings_path = Self::get_settings_file_path();
        
        // Create the directory if it doesn't exist
        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let toml_string = toml::to_string_pretty(settings)?;
        std::fs::write(&settings_path, toml_string)?;
        println!("Saved desktop settings to: {}", settings_path.display());
        Ok(())
    }
}