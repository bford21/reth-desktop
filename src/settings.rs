use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DesktopSettings {
    #[serde(default)]
    pub keep_reth_running_in_background: bool,
    #[serde(default)]
    pub custom_launch_args: Vec<String>,
    #[serde(default)]
    pub reth_defaults: RethDefaults,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RethDefaults {
    // Core node parameters
    #[serde(default = "default_true")]
    pub enable_full_node: bool,
    #[serde(default = "default_true")]
    pub enable_metrics: bool,
    #[serde(default = "default_metrics_address")]
    pub metrics_address: String,
    
    // Network parameters
    #[serde(default = "default_chain")]
    pub chain: String,
    #[serde(default = "default_datadir")]
    pub datadir: String,
    
    // Stdout logging parameters
    #[serde(default = "default_true")]
    pub enable_stdout_logging: bool,
    #[serde(default = "default_log_format")]
    pub stdout_log_format: String,
    
    // File logging parameters
    #[serde(default = "default_true")]
    pub enable_file_logging: bool,
    #[serde(default = "default_log_format")]
    pub file_log_format: String,
    #[serde(default = "default_log_level")]
    pub file_log_level: String,
    #[serde(default = "default_log_max_size")]
    pub file_log_max_size: String,
    #[serde(default = "default_log_max_files")]
    pub file_log_max_files: String,
    
    // Port detection parameters
    #[serde(default = "default_rpc_port")]
    pub default_rpc_port: u16,
    #[serde(default = "default_ws_port")]
    pub default_ws_port: u16,
    #[serde(default = "default_engine_port")]
    pub default_engine_port: u16,
}

// Default value functions
fn default_true() -> bool { true }
fn default_metrics_address() -> String { "127.0.0.1:9001".to_string() }
fn default_chain() -> String { "mainnet".to_string() }
fn default_datadir() -> String {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".reth-desktop")
        .join("data")
        .to_string_lossy()
        .to_string()
}
fn default_log_format() -> String { "terminal".to_string() }
fn default_log_level() -> String { "info".to_string() }
fn default_log_max_size() -> String { "50".to_string() }
fn default_log_max_files() -> String { "3".to_string() }
fn default_rpc_port() -> u16 { 8545 }
fn default_ws_port() -> u16 { 8546 }
fn default_engine_port() -> u16 { 8551 }

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            keep_reth_running_in_background: false,
            custom_launch_args: Vec::new(),
            reth_defaults: RethDefaults::default(),
        }
    }
}

impl Default for RethDefaults {
    fn default() -> Self {
        Self {
            enable_full_node: default_true(),
            enable_metrics: default_true(),
            metrics_address: default_metrics_address(),
            chain: default_chain(),
            datadir: default_datadir(),
            enable_stdout_logging: default_true(),
            stdout_log_format: default_log_format(),
            enable_file_logging: default_true(),
            file_log_format: default_log_format(),
            file_log_level: default_log_level(),
            file_log_max_size: default_log_max_size(),
            file_log_max_files: default_log_max_files(),
            default_rpc_port: default_rpc_port(),
            default_ws_port: default_ws_port(),
            default_engine_port: default_engine_port(),
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