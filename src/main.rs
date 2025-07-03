use eframe::egui;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

mod installer;
mod system_check;
mod theme;
mod reth_node;

use installer::{RethInstaller, InstallStatus};
use system_check::SystemRequirements;
use theme::RethTheme;
use reth_node::{RethNode, LogLine, LogLevel};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct RethConfig {
    #[serde(default)]
    stages: StagesConfig,
    #[serde(default)]
    peers: PeersConfig,
    #[serde(default)]
    sessions: SessionsConfig,
    #[serde(default)]
    prune: PruneConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct StagesConfig {
    #[serde(default)]
    era: Option<EraStageConfig>,
    #[serde(default)]
    headers: Option<HeadersStageConfig>,
    #[serde(default)]
    bodies: Option<BodiesStageConfig>,
    #[serde(default)]
    sender_recovery: Option<SenderRecoveryStageConfig>,
    #[serde(default)]
    execution: Option<ExecutionStageConfig>,
    #[serde(default)]
    prune: Option<PruneStageConfig>,
    #[serde(default)]
    account_hashing: Option<AccountHashingStageConfig>,
    #[serde(default)]
    storage_hashing: Option<StorageHashingStageConfig>,
    #[serde(default)]
    merkle: Option<MerkleStageConfig>,
    #[serde(default)]
    transaction_lookup: Option<TransactionLookupStageConfig>,
    #[serde(default)]
    index_account_history: Option<IndexAccountHistoryStageConfig>,
    #[serde(default)]
    index_storage_history: Option<IndexStorageHistoryStageConfig>,
    #[serde(default)]
    etl: Option<EtlStageConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct EraStageConfig {
    // Era stage appears to be empty in config
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct HeadersStageConfig {
    #[serde(default)]
    downloader_max_concurrent_requests: Option<u32>,
    #[serde(default)]
    downloader_min_concurrent_requests: Option<u32>,
    #[serde(default)]
    downloader_max_buffered_responses: Option<u32>,
    #[serde(default)]
    downloader_request_limit: Option<u32>,
    #[serde(default)]
    commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct BodiesStageConfig {
    #[serde(default)]
    downloader_request_limit: Option<u32>,
    #[serde(default)]
    downloader_stream_batch_size: Option<u32>,
    #[serde(default)]
    downloader_max_buffered_blocks_size_bytes: Option<u64>,
    #[serde(default)]
    downloader_min_concurrent_requests: Option<u32>,
    #[serde(default)]
    downloader_max_concurrent_requests: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct SenderRecoveryStageConfig {
    #[serde(default)]
    commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ExecutionStageConfig {
    #[serde(default)]
    max_blocks: Option<u64>,
    #[serde(default)]
    max_changes: Option<u64>,
    #[serde(default)]
    max_cumulative_gas: Option<u64>,
    #[serde(default)]
    max_duration: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct PruneStageConfig {
    #[serde(default)]
    commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct AccountHashingStageConfig {
    #[serde(default)]
    clean_threshold: Option<u64>,
    #[serde(default)]
    commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct StorageHashingStageConfig {
    #[serde(default)]
    clean_threshold: Option<u64>,
    #[serde(default)]
    commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct MerkleStageConfig {
    #[serde(default)]
    incremental_threshold: Option<u64>,
    #[serde(default)]
    rebuild_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct TransactionLookupStageConfig {
    #[serde(default)]
    chunk_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct IndexAccountHistoryStageConfig {
    #[serde(default)]
    commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct IndexStorageHistoryStageConfig {
    #[serde(default)]
    commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct EtlStageConfig {
    #[serde(default)]
    file_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct PeersConfig {
    #[serde(default)]
    refill_slots_interval: Option<String>,
    #[serde(default)]
    trusted_nodes: Option<Vec<String>>,
    #[serde(default)]
    trusted_nodes_only: Option<bool>,
    #[serde(default)]
    trusted_nodes_resolution_interval: Option<String>,
    #[serde(default)]
    max_backoff_count: Option<u32>,
    #[serde(default)]
    ban_duration: Option<String>,
    #[serde(default)]
    incoming_ip_throttle_duration: Option<String>,
    #[serde(default)]
    connection_info: Option<ConnectionInfoConfig>,
    #[serde(default)]
    reputation_weights: Option<ReputationWeightsConfig>,
    #[serde(default)]
    backoff_durations: Option<BackoffDurationsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ConnectionInfoConfig {
    #[serde(default)]
    max_outbound: Option<u32>,
    #[serde(default)]
    max_inbound: Option<u32>,
    #[serde(default)]
    max_concurrent_outbound_dials: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ReputationWeightsConfig {
    #[serde(default)]
    bad_message: Option<i32>,
    #[serde(default)]
    bad_block: Option<i32>,
    #[serde(default)]
    bad_transactions: Option<i32>,
    #[serde(default)]
    already_seen_transactions: Option<i32>,
    #[serde(default)]
    timeout: Option<i32>,
    #[serde(default)]
    bad_protocol: Option<i32>,
    #[serde(default)]
    failed_to_connect: Option<i32>,
    #[serde(default)]
    dropped: Option<i32>,
    #[serde(default)]
    bad_announcement: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct BackoffDurationsConfig {
    #[serde(default)]
    low: Option<String>,
    #[serde(default)]
    medium: Option<String>,
    #[serde(default)]
    high: Option<String>,
    #[serde(default)]
    max: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct SessionsConfig {
    #[serde(default)]
    session_command_buffer: Option<u32>,
    #[serde(default)]
    session_event_buffer: Option<u32>,
    #[serde(default)]
    limits: Option<SessionLimitsConfig>,
    #[serde(default)]
    initial_internal_request_timeout: Option<TimeoutConfig>,
    #[serde(default)]
    protocol_breach_request_timeout: Option<TimeoutConfig>,
    #[serde(default)]
    pending_session_timeout: Option<TimeoutConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct SessionLimitsConfig {
    // This appears to be empty in your config
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct TimeoutConfig {
    #[serde(default)]
    secs: Option<u64>,
    #[serde(default)]
    nanos: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct PruneConfig {
    #[serde(default)]
    block_interval: Option<u64>,
    #[serde(default)]
    segments: Option<PruneSegments>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct PruneSegments {
    #[serde(default)]
    sender_recovery: Option<String>,
    #[serde(default)]
    receipts: Option<PruneReceiptsConfig>,
    #[serde(default)]
    account_history: Option<PruneHistoryConfig>,
    #[serde(default)]
    storage_history: Option<PruneHistoryConfig>,
    #[serde(default)]
    receipts_log_filter: Option<PruneReceiptsLogFilterConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct PruneReceiptsConfig {
    #[serde(default)]
    distance: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct PruneHistoryConfig {
    #[serde(default)]
    distance: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct PruneReceiptsLogFilterConfig {
    // This appears to be empty in your config
}

fn default_max_peers() -> u32 { 50 }
fn default_min_peers() -> u32 { 1 }

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Reth Desktop Installer"),
        ..Default::default()
    };
    
    eframe::run_native(
        "Reth Desktop",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    )
}

struct MyApp {
    installer: Arc<Mutex<RethInstaller>>,
    install_status: InstallStatus,
    installing: bool,
    _runtime: tokio::runtime::Runtime,
    install_sender: mpsc::UnboundedSender<InstallCommand>,
    update_receiver: mpsc::UnboundedReceiver<(String, bool)>,
    system_requirements: SystemRequirements,
    reth_logo: Option<egui::TextureHandle>,
    reth_node: RethNode,
    node_logs: Vec<LogLine>,
    is_reth_installed: bool,
    was_detected_on_startup: bool,
    installed_version: Option<String>,
    latest_version: Option<String>,
    update_available: bool,
    show_settings: bool,
    reth_config: RethConfig,
    reth_config_path: Option<std::path::PathBuf>,
    editable_config: RethConfig,
    config_modified: bool,
    settings_edit_mode: bool,
}

enum InstallCommand {
    StartInstall(Arc<Mutex<RethInstaller>>, egui::Context),
    ResetInstaller(Arc<Mutex<RethInstaller>>),
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = tokio::runtime::Runtime::new().expect("Unable to create Runtime");
        let (tx, mut rx) = mpsc::unbounded_channel::<InstallCommand>();
        let (update_tx, update_rx) = mpsc::unbounded_channel::<(String, bool)>();
        
        // Load the Reth logo
        let reth_logo = Self::load_logo(&cc.egui_ctx);
        
        // Check if Reth is installed and get version
        let is_reth_installed = Self::check_reth_installed();
        let installed_version = Self::get_installed_version();
        
        // Load Reth configuration
        let (reth_config, reth_config_path) = Self::load_reth_config();
        
        // Spawn a task to handle installation commands
        runtime.spawn(async move {
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    InstallCommand::StartInstall(installer, ctx) => {
                        let mut installer = installer.lock().await;
                        if let Err(_e) = installer.install_reth().await {
                            // Error is already handled in the installer
                        }
                        ctx.request_repaint();
                    }
                    InstallCommand::ResetInstaller(installer) => {
                        let mut installer = installer.lock().await;
                        *installer = RethInstaller::new();
                    }
                }
            }
        });
        
        // Start update check if Reth is installed
        if is_reth_installed {
            let update_sender = update_tx.clone();
            let installed_ver = installed_version.clone();
            runtime.spawn(async move {
                if let Some(installed) = installed_ver {
                    match Self::fetch_latest_version_static().await {
                        Ok(latest) => {
                            let update_available = Self::is_update_available_static(&installed, &latest);
                            let _ = update_sender.send((latest, update_available));
                        }
                        Err(_) => {}
                    }
                }
            });
        }
        
        let initial_status = if is_reth_installed {
            InstallStatus::Completed
        } else {
            InstallStatus::Idle
        };
        
        Self {
            installer: Arc::new(Mutex::new(RethInstaller::new())),
            install_status: initial_status,
            installing: false,
            _runtime: runtime,
            install_sender: tx,
            update_receiver: update_rx,
            system_requirements: SystemRequirements::check(),
            reth_logo,
            reth_node: RethNode::new(),
            node_logs: Vec::new(),
            is_reth_installed,
            was_detected_on_startup: is_reth_installed,
            installed_version,
            latest_version: None,
            update_available: false,
            show_settings: false,
            reth_config: reth_config.clone(),
            reth_config_path,
            editable_config: reth_config,
            config_modified: false,
            settings_edit_mode: false,
        }
    }
    
    fn load_logo(ctx: &egui::Context) -> Option<egui::TextureHandle> {
        // Try multiple possible paths for the reth-docs.png image
        let possible_paths = [
            "assets/reth-docs.png",
            "./assets/reth-docs.png", 
            "../assets/reth-docs.png",
            "reth-docs.png"
        ];
        
        for path in &possible_paths {
            match image::open(path) {
                Ok(img) => {
                    let rgba = img.to_rgba8();
                    let size = [img.width() as usize, img.height() as usize];
                    let pixels = rgba.as_flat_samples();
                    
                    let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                    println!("Successfully loaded logo from: {}", path);
                    return Some(ctx.load_texture("reth-logo", color_image, egui::TextureOptions::default()));
                }
                Err(_) => continue,
            }
        }
        
        eprintln!("Failed to load reth-docs.png from any path");
        None
    }
    
    fn check_reth_installed() -> bool {
        let reth_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".reth-desktop")
            .join("bin")
            .join("reth");
        
        // Check if the reth binary exists and is executable
        if reth_path.exists() {
            // Try to run reth --version to verify it works
            match std::process::Command::new(&reth_path)
                .arg("--version")
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        let version_str = String::from_utf8_lossy(&output.stdout);
                        println!("Found existing Reth installation: {}", version_str.trim());
                        return true;
                    }
                }
                Err(_) => {
                    // Binary exists but doesn't work properly
                    eprintln!("Reth binary exists but is not functional");
                }
            }
        }
        
        false
    }
    
    fn get_installed_version() -> Option<String> {
        let reth_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".reth-desktop")
            .join("bin")
            .join("reth");
        
        match std::process::Command::new(&reth_path)
            .arg("--version")
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    let version_str = String::from_utf8_lossy(&output.stdout);
                    // Parse version from output like "reth-ethereum-cli Version: 1.5.0"
                    if let Some(version_line) = version_str.lines().next() {
                        if let Some(version_part) = version_line.split("Version: ").nth(1) {
                            let version = version_part.trim();
                            println!("Detected installed version: {}", version);
                            return Some(version.to_string());
                        }
                    }
                }
            }
            Err(_) => {}
        }
        
        None
    }
    
    fn get_reth_data_dir() -> std::path::PathBuf {
        // Get platform-specific Reth data directory
        #[cfg(target_os = "macos")]
        {
            dirs::home_dir()
                .unwrap_or_default()
                .join("Library")
                .join("Application Support")
                .join("reth")
        }
        
        #[cfg(target_os = "linux")]
        {
            // Try XDG_DATA_HOME first, fallback to ~/.local/share/reth
            if let Some(xdg_data) = std::env::var_os("XDG_DATA_HOME") {
                std::path::PathBuf::from(xdg_data).join("reth")
            } else {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join(".local")
                    .join("share")
                    .join("reth")
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            dirs::data_dir()
                .unwrap_or_default()
                .join("reth")
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            // Fallback for unsupported platforms
            dirs::home_dir()
                .unwrap_or_default()
                .join(".reth")
        }
    }
    
    fn load_reth_config() -> (RethConfig, Option<std::path::PathBuf>) {
        let reth_data_dir = Self::get_reth_data_dir();
        
        // Try different possible config locations
        let possible_paths = [
            reth_data_dir.join("mainnet").join("reth.toml"),  // Network-specific (mainnet)
            reth_data_dir.join("reth.toml"),                  // Root directory
            reth_data_dir.join("goerli").join("reth.toml"),   // Other networks
            reth_data_dir.join("sepolia").join("reth.toml"),
        ];
        
        for config_path in &possible_paths {
            match std::fs::read_to_string(config_path) {
                Ok(content) => {
                    match toml::from_str::<RethConfig>(&content) {
                        Ok(config) => {
                            println!("Loaded Reth configuration from: {}", config_path.display());
                            return (config, Some(config_path.clone()));
                        }
                        Err(e) => {
                            eprintln!("Failed to parse reth.toml at {}: {}", config_path.display(), e);
                            continue;
                        }
                    }
                }
                Err(_) => continue,
            }
        }
        
        println!("No reth.toml found in any expected location, using defaults");
        println!("Searched locations:");
        for path in &possible_paths {
            println!("  - {}", path.display());
        }
        (RethConfig::default(), None)
    }
    
    async fn check_for_updates(&mut self) {
        // Fetch latest version from GitHub
        if let Some(installed) = &self.installed_version {
            match self.fetch_latest_version_async().await {
                Ok(latest) => {
                    self.latest_version = Some(latest.clone());
                    self.update_available = self.is_update_available(installed, &latest);
                    
                    if self.update_available {
                        println!("Update available: {} -> {}", installed, latest);
                    } else {
                        println!("Already on latest version: {}", installed);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to check for updates: {}", e);
                }
            }
        }
    }
    
    async fn fetch_latest_version_async(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        const FALLBACK_VERSION: &str = "1.5.0";
        
        let url = "https://api.github.com/repos/paradigmxyz/reth/releases/latest";
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        
        match client
            .get(url)
            .header("User-Agent", "reth-desktop/1.0")
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    return Ok(FALLBACK_VERSION.to_string());
                }
                
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<serde_json::Value>(&body) {
                            Ok(json) => {
                                if let Some(tag_name) = json["tag_name"].as_str() {
                                    // Remove 'v' prefix if present
                                    let version = tag_name.strip_prefix('v').unwrap_or(tag_name);
                                    return Ok(version.to_string());
                                }
                            }
                            Err(_) => {}
                        }
                    }
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }
        
        Ok(FALLBACK_VERSION.to_string())
    }
    
    fn is_update_available(&self, installed: &str, latest: &str) -> bool {
        Self::is_update_available_static(installed, latest)
    }
    
    fn is_update_available_static(installed: &str, latest: &str) -> bool {
        match (semver::Version::parse(installed), semver::Version::parse(latest)) {
            (Ok(installed_ver), Ok(latest_ver)) => {
                latest_ver > installed_ver
            }
            _ => {
                // Fallback to string comparison if semver parsing fails
                installed != latest
            }
        }
    }
    
    fn clean_log_content(content: &str) -> String {
        // Remove ANSI escape codes and replace problematic characters
        let mut cleaned = String::new();
        let mut chars = content.chars().peekable();
        
        while let Some(ch) = chars.next() {
            match ch {
                // Skip ANSI escape sequences
                '\x1b' => {
                    // Skip the escape sequence
                    if chars.peek() == Some(&'[') {
                        chars.next(); // consume '['
                        // Skip until we find a letter (end of ANSI sequence)
                        while let Some(next_ch) = chars.next() {
                            if next_ch.is_ascii_alphabetic() || next_ch == 'm' {
                                break;
                            }
                        }
                    }
                }
                // Replace various problematic characters
                '\u{00A0}' => cleaned.push(' '), // Non-breaking space
                '\u{2009}' => cleaned.push(' '), // Thin space
                '\u{2060}' => {},               // Word joiner (remove completely)
                '\u{FEFF}' => {},               // Zero-width no-break space (remove completely)
                '\u{200B}' => {},               // Zero-width space (remove completely)
                '\u{200C}' => {},               // Zero-width non-joiner (remove completely)
                '\u{200D}' => {},               // Zero-width joiner (remove completely)
                // Replace other non-printable control characters with spaces
                ch if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' => {
                    cleaned.push(' ');
                }
                // Replace tab with 2 spaces for better formatting
                '\t' => {
                    cleaned.push_str("  ");
                }
                // Keep normal characters
                _ => {
                    cleaned.push(ch);
                }
            }
        }
        
        // Clean up multiple consecutive spaces while preserving single spaces
        let mut result = String::new();
        let mut space_count = 0;
        
        for ch in cleaned.chars() {
            if ch == ' ' {
                space_count += 1;
                if space_count == 1 {
                    result.push(ch);
                }
            } else {
                space_count = 0;
                result.push(ch);
            }
        }
        
        result.trim().to_string()
    }
    
    async fn fetch_latest_version_static() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        const FALLBACK_VERSION: &str = "1.5.0";
        
        let url = "https://api.github.com/repos/paradigmxyz/reth/releases/latest";
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        
        match client
            .get(url)
            .header("User-Agent", "reth-desktop/1.0")
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    return Ok(FALLBACK_VERSION.to_string());
                }
                
                match response.text().await {
                    Ok(body) => {
                        match serde_json::from_str::<serde_json::Value>(&body) {
                            Ok(json) => {
                                if let Some(tag_name) = json["tag_name"].as_str() {
                                    // Remove 'v' prefix if present
                                    let version = tag_name.strip_prefix('v').unwrap_or(tag_name);
                                    return Ok(version.to_string());
                                }
                            }
                            Err(_) => {}
                        }
                    }
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }
        
        Ok(FALLBACK_VERSION.to_string())
    }

    fn start_installation(&mut self, ctx: egui::Context) {
        self.installing = true;
        let installer = Arc::clone(&self.installer);
        
        // Send command to tokio runtime
        let _ = self.install_sender.send(InstallCommand::StartInstall(installer, ctx));
    }
    
    fn reset_installer(&mut self) {
        let installer = Arc::clone(&self.installer);
        let _ = self.install_sender.send(InstallCommand::ResetInstaller(installer));
    }
    
    fn launch_reth(&mut self) {
        let reth_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".reth-desktop")
            .join("bin")
            .join("reth");
        
        match self.reth_node.start(&reth_path.to_string_lossy()) {
            Ok(()) => {
                self.install_status = InstallStatus::Running;
            }
            Err(e) => {
                self.install_status = InstallStatus::Error(format!("Failed to launch Reth: {}", e));
            }
        }
    }
    
    fn stop_reth(&mut self) {
        if let Err(e) = self.reth_node.stop() {
            eprintln!("Error stopping Reth: {}", e);
        }
        self.install_status = InstallStatus::Stopped;
    }
    
    fn save_reth_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(config_path) = &self.reth_config_path {
            let toml_string = toml::to_string_pretty(&self.editable_config)?;
            std::fs::write(config_path, toml_string)?;
            self.reth_config = self.editable_config.clone();
            self.config_modified = false;
            println!("Saved configuration to: {}", config_path.display());
            Ok(())
        } else {
            Err("No configuration file path available".into())
        }
    }
    
    fn reset_editable_config(&mut self) {
        self.editable_config = self.reth_config.clone();
        self.config_modified = false;
        // Don't reset edit mode here - let the caller decide
    }
    
    fn editable_u32_field(ui: &mut egui::Ui, label: &str, value: &mut Option<u32>) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(format!("{}:", label));
            let mut text = value.map_or_else(String::new, |v| v.to_string());
            if ui.add_sized([150.0, 20.0], egui::TextEdit::singleline(&mut text)).changed() {
                if text.is_empty() {
                    *value = None;
                } else if let Ok(parsed) = text.parse::<u32>() {
                    *value = Some(parsed);
                }
                changed = true;
            }
        });
        changed
    }
    
    fn editable_u64_field(ui: &mut egui::Ui, label: &str, value: &mut Option<u64>) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(format!("{}:", label));
            let mut text = value.map_or_else(String::new, |v| v.to_string());
            if ui.add_sized([150.0, 20.0], egui::TextEdit::singleline(&mut text)).changed() {
                if text.is_empty() {
                    *value = None;
                } else if let Ok(parsed) = text.parse::<u64>() {
                    *value = Some(parsed);
                }
                changed = true;
            }
        });
        changed
    }
    
    fn editable_string_field(ui: &mut egui::Ui, label: &str, value: &mut Option<String>) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(format!("{}:", label));
            let mut text = value.as_ref().map_or_else(String::new, |v| v.clone());
            if ui.add_sized([150.0, 20.0], egui::TextEdit::singleline(&mut text)).changed() {
                if text.is_empty() {
                    *value = None;
                } else {
                    *value = Some(text);
                }
                changed = true;
            }
        });
        changed
    }
    
    fn editable_bool_field(ui: &mut egui::Ui, label: &str, value: &mut Option<bool>) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(format!("{}:", label));
            let mut checkbox_value = value.unwrap_or(false);
            if ui.checkbox(&mut checkbox_value, "").changed() {
                *value = Some(checkbox_value);
                changed = true;
            }
        });
        changed
    }
    
    fn editable_i32_field(ui: &mut egui::Ui, label: &str, value: &mut Option<i32>) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(format!("{}:", label));
            let mut text = value.map_or_else(String::new, |v| v.to_string());
            if ui.add_sized([150.0, 20.0], egui::TextEdit::singleline(&mut text)).changed() {
                if text.is_empty() {
                    *value = None;
                } else if let Ok(parsed) = text.parse::<i32>() {
                    *value = Some(parsed);
                }
                changed = true;
            }
        });
        changed
    }
    
    fn show_settings_content(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Header with title and close button
            ui.horizontal(|ui| {
                ui.heading("Reth Node Configuration");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖ Close").clicked() {
                        self.show_settings = false;
                    }
                });
            });
            ui.add_space(8.0);
            
            // Config file path
            let reth_data_dir = Self::get_reth_data_dir();
            if let Some(config_path) = &self.reth_config_path {
                ui.label(RethTheme::muted_text(&format!("Configuration file: {}", config_path.display())));
            } else {
                ui.label(RethTheme::muted_text("Configuration file: Not found (using defaults)"));
            }
            ui.label(RethTheme::muted_text(&format!("Reth data directory: {}", reth_data_dir.display())));
            ui.add_space(12.0);
            
            // Edit mode toggle
            ui.horizontal(|ui| {
                if !self.settings_edit_mode {
                    if ui.button("🖊 Edit").clicked() {
                        self.settings_edit_mode = true;
                        self.reset_editable_config(); // Reset to ensure clean state
                    }
                } else {
                    if ui.button("👁 View Mode").clicked() {
                        self.settings_edit_mode = false;
                        self.reset_editable_config();
                    }
                    ui.add_space(8.0);
                    ui.label(RethTheme::success_text("✏ Edit mode active - you can modify configuration values"));
                }
            });
            ui.add_space(16.0);
            
            // Stages Configuration
            ui.collapsing("Stages Configuration", |ui| {
                // Era Stage
                if self.reth_config.stages.era.is_some() {
                    ui.label("Era Stage: Configured");
                }
                
                // Headers Stage
                if self.settings_edit_mode {
                    if let Some(headers) = &mut self.editable_config.stages.headers {
                        ui.label("Headers Stage:");
                        ui.indent("headers", |ui| {
                            if Self::editable_u32_field(ui, "Max Concurrent Requests", &mut headers.downloader_max_concurrent_requests) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Min Concurrent Requests", &mut headers.downloader_min_concurrent_requests) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Max Buffered Responses", &mut headers.downloader_max_buffered_responses) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Request Limit", &mut headers.downloader_request_limit) {
                                self.config_modified = true;
                            }
                            if Self::editable_u64_field(ui, "Commit Threshold", &mut headers.commit_threshold) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Headers Stage").clicked() {
                            self.editable_config.stages.headers = Some(HeadersStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    // Read-only view
                    if let Some(headers) = &self.reth_config.stages.headers {
                        ui.label("Headers Stage:");
                        ui.indent("headers_readonly", |ui| {
                            if let Some(val) = headers.downloader_max_concurrent_requests {
                                ui.label(&format!("Max Concurrent Requests: {}", val));
                            }
                            if let Some(val) = headers.downloader_min_concurrent_requests {
                                ui.label(&format!("Min Concurrent Requests: {}", val));
                            }
                            if let Some(val) = headers.downloader_max_buffered_responses {
                                ui.label(&format!("Max Buffered Responses: {}", val));
                            }
                            if let Some(val) = headers.downloader_request_limit {
                                ui.label(&format!("Request Limit: {}", val));
                            }
                            if let Some(val) = headers.commit_threshold {
                                ui.label(&format!("Commit Threshold: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // Bodies Stage
                if self.settings_edit_mode {
                    if let Some(bodies) = &mut self.editable_config.stages.bodies {
                        ui.label("Bodies Stage:");
                        ui.indent("bodies", |ui| {
                            if Self::editable_u32_field(ui, "Request Limit", &mut bodies.downloader_request_limit) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Stream Batch Size", &mut bodies.downloader_stream_batch_size) {
                                self.config_modified = true;
                            }
                            if Self::editable_u64_field(ui, "Max Buffered Blocks Size (bytes)", &mut bodies.downloader_max_buffered_blocks_size_bytes) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Min Concurrent Requests", &mut bodies.downloader_min_concurrent_requests) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Max Concurrent Requests", &mut bodies.downloader_max_concurrent_requests) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Bodies Stage").clicked() {
                            self.editable_config.stages.bodies = Some(BodiesStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(bodies) = &self.reth_config.stages.bodies {
                        ui.label("Bodies Stage:");
                        ui.indent("bodies_readonly", |ui| {
                            if let Some(val) = bodies.downloader_request_limit {
                                ui.label(&format!("Request Limit: {}", val));
                            }
                            if let Some(val) = bodies.downloader_stream_batch_size {
                                ui.label(&format!("Stream Batch Size: {}", val));
                            }
                            if let Some(val) = bodies.downloader_max_buffered_blocks_size_bytes {
                                ui.label(&format!("Max Buffered Blocks Size: {} bytes", val));
                            }
                            if let Some(val) = bodies.downloader_min_concurrent_requests {
                                ui.label(&format!("Min Concurrent Requests: {}", val));
                            }
                            if let Some(val) = bodies.downloader_max_concurrent_requests {
                                ui.label(&format!("Max Concurrent Requests: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // Sender Recovery Stage
                if self.settings_edit_mode {
                    if let Some(sender_recovery) = &mut self.editable_config.stages.sender_recovery {
                        ui.label("Sender Recovery Stage:");
                        ui.indent("sender_recovery", |ui| {
                            if Self::editable_u64_field(ui, "Commit Threshold", &mut sender_recovery.commit_threshold) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Sender Recovery Stage").clicked() {
                            self.editable_config.stages.sender_recovery = Some(SenderRecoveryStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(sender_recovery) = &self.reth_config.stages.sender_recovery {
                        ui.label("Sender Recovery Stage:");
                        ui.indent("sender_recovery_readonly", |ui| {
                            if let Some(val) = sender_recovery.commit_threshold {
                                ui.label(&format!("Commit Threshold: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // Execution Stage
                if self.settings_edit_mode {
                    if let Some(execution) = &mut self.editable_config.stages.execution {
                        ui.label("Execution Stage:");
                        ui.indent("execution", |ui| {
                            if Self::editable_u64_field(ui, "Max Blocks", &mut execution.max_blocks) {
                                self.config_modified = true;
                            }
                            if Self::editable_u64_field(ui, "Max Changes", &mut execution.max_changes) {
                                self.config_modified = true;
                            }
                            if Self::editable_u64_field(ui, "Max Cumulative Gas", &mut execution.max_cumulative_gas) {
                                self.config_modified = true;
                            }
                            if Self::editable_string_field(ui, "Max Duration", &mut execution.max_duration) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Execution Stage").clicked() {
                            self.editable_config.stages.execution = Some(ExecutionStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(execution) = &self.reth_config.stages.execution {
                        ui.label("Execution Stage:");
                        ui.indent("execution_readonly", |ui| {
                            if let Some(val) = execution.max_blocks {
                                ui.label(&format!("Max Blocks: {}", val));
                            }
                            if let Some(val) = execution.max_changes {
                                ui.label(&format!("Max Changes: {}", val));
                            }
                            if let Some(val) = execution.max_cumulative_gas {
                                ui.label(&format!("Max Cumulative Gas: {}", val));
                            }
                            if let Some(val) = &execution.max_duration {
                                ui.label(&format!("Max Duration: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // Prune Stage
                if self.settings_edit_mode {
                    if let Some(prune_stage) = &mut self.editable_config.stages.prune {
                        ui.label("Prune Stage:");
                        ui.indent("prune_stage", |ui| {
                            if Self::editable_u64_field(ui, "Commit Threshold", &mut prune_stage.commit_threshold) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Prune Stage").clicked() {
                            self.editable_config.stages.prune = Some(PruneStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(prune_stage) = &self.reth_config.stages.prune {
                        ui.label("Prune Stage:");
                        ui.indent("prune_stage_readonly", |ui| {
                            if let Some(val) = prune_stage.commit_threshold {
                                ui.label(&format!("Commit Threshold: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // Account Hashing Stage
                if self.settings_edit_mode {
                    if let Some(account_hashing) = &mut self.editable_config.stages.account_hashing {
                        ui.label("Account Hashing Stage:");
                        ui.indent("account_hashing", |ui| {
                            if Self::editable_u64_field(ui, "Clean Threshold", &mut account_hashing.clean_threshold) {
                                self.config_modified = true;
                            }
                            if Self::editable_u64_field(ui, "Commit Threshold", &mut account_hashing.commit_threshold) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Account Hashing Stage").clicked() {
                            self.editable_config.stages.account_hashing = Some(AccountHashingStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(account_hashing) = &self.reth_config.stages.account_hashing {
                        ui.label("Account Hashing Stage:");
                        ui.indent("account_hashing_readonly", |ui| {
                            if let Some(val) = account_hashing.clean_threshold {
                                ui.label(&format!("Clean Threshold: {}", val));
                            }
                            if let Some(val) = account_hashing.commit_threshold {
                                ui.label(&format!("Commit Threshold: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // Storage Hashing Stage
                if self.settings_edit_mode {
                    if let Some(storage_hashing) = &mut self.editable_config.stages.storage_hashing {
                        ui.label("Storage Hashing Stage:");
                        ui.indent("storage_hashing", |ui| {
                            if Self::editable_u64_field(ui, "Clean Threshold", &mut storage_hashing.clean_threshold) {
                                self.config_modified = true;
                            }
                            if Self::editable_u64_field(ui, "Commit Threshold", &mut storage_hashing.commit_threshold) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Storage Hashing Stage").clicked() {
                            self.editable_config.stages.storage_hashing = Some(StorageHashingStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(storage_hashing) = &self.reth_config.stages.storage_hashing {
                        ui.label("Storage Hashing Stage:");
                        ui.indent("storage_hashing_readonly", |ui| {
                            if let Some(val) = storage_hashing.clean_threshold {
                                ui.label(&format!("Clean Threshold: {}", val));
                            }
                            if let Some(val) = storage_hashing.commit_threshold {
                                ui.label(&format!("Commit Threshold: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // Merkle Stage
                if self.settings_edit_mode {
                    if let Some(merkle) = &mut self.editable_config.stages.merkle {
                        ui.label("Merkle Stage:");
                        ui.indent("merkle", |ui| {
                            if Self::editable_u64_field(ui, "Incremental Threshold", &mut merkle.incremental_threshold) {
                                self.config_modified = true;
                            }
                            if Self::editable_u64_field(ui, "Rebuild Threshold", &mut merkle.rebuild_threshold) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Merkle Stage").clicked() {
                            self.editable_config.stages.merkle = Some(MerkleStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(merkle) = &self.reth_config.stages.merkle {
                        ui.label("Merkle Stage:");
                        ui.indent("merkle_readonly", |ui| {
                            if let Some(val) = merkle.incremental_threshold {
                                ui.label(&format!("Incremental Threshold: {}", val));
                            }
                            if let Some(val) = merkle.rebuild_threshold {
                                ui.label(&format!("Rebuild Threshold: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // Transaction Lookup Stage
                if self.settings_edit_mode {
                    if let Some(tx_lookup) = &mut self.editable_config.stages.transaction_lookup {
                        ui.label("Transaction Lookup Stage:");
                        ui.indent("transaction_lookup", |ui| {
                            if Self::editable_u64_field(ui, "Chunk Size", &mut tx_lookup.chunk_size) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Transaction Lookup Stage").clicked() {
                            self.editable_config.stages.transaction_lookup = Some(TransactionLookupStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(tx_lookup) = &self.reth_config.stages.transaction_lookup {
                        ui.label("Transaction Lookup Stage:");
                        ui.indent("transaction_lookup_readonly", |ui| {
                            if let Some(val) = tx_lookup.chunk_size {
                                ui.label(&format!("Chunk Size: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // Index Account History Stage
                if self.settings_edit_mode {
                    if let Some(index_account) = &mut self.editable_config.stages.index_account_history {
                        ui.label("Index Account History Stage:");
                        ui.indent("index_account_history", |ui| {
                            if Self::editable_u64_field(ui, "Commit Threshold", &mut index_account.commit_threshold) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Index Account History Stage").clicked() {
                            self.editable_config.stages.index_account_history = Some(IndexAccountHistoryStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(index_account) = &self.reth_config.stages.index_account_history {
                        ui.label("Index Account History Stage:");
                        ui.indent("index_account_history_readonly", |ui| {
                            if let Some(val) = index_account.commit_threshold {
                                ui.label(&format!("Commit Threshold: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // Index Storage History Stage
                if self.settings_edit_mode {
                    if let Some(index_storage) = &mut self.editable_config.stages.index_storage_history {
                        ui.label("Index Storage History Stage:");
                        ui.indent("index_storage_history", |ui| {
                            if Self::editable_u64_field(ui, "Commit Threshold", &mut index_storage.commit_threshold) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add Index Storage History Stage").clicked() {
                            self.editable_config.stages.index_storage_history = Some(IndexStorageHistoryStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(index_storage) = &self.reth_config.stages.index_storage_history {
                        ui.label("Index Storage History Stage:");
                        ui.indent("index_storage_history_readonly", |ui| {
                            if let Some(val) = index_storage.commit_threshold {
                                ui.label(&format!("Commit Threshold: {}", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
                
                // ETL Stage
                if self.settings_edit_mode {
                    if let Some(etl) = &mut self.editable_config.stages.etl {
                        ui.label("ETL Stage:");
                        ui.indent("etl", |ui| {
                            if Self::editable_u64_field(ui, "File Size (bytes)", &mut etl.file_size) {
                                self.config_modified = true;
                            }
                        });
                        ui.add_space(8.0);
                    } else {
                        if ui.button("+ Add ETL Stage").clicked() {
                            self.editable_config.stages.etl = Some(EtlStageConfig::default());
                            self.config_modified = true;
                        }
                        ui.add_space(8.0);
                    }
                } else {
                    if let Some(etl) = &self.reth_config.stages.etl {
                        ui.label("ETL Stage:");
                        ui.indent("etl_readonly", |ui| {
                            if let Some(val) = etl.file_size {
                                ui.label(&format!("File Size: {} bytes", val));
                            }
                        });
                        ui.add_space(8.0);
                    }
                }
            });
            
            ui.add_space(12.0);
            
            // Peers Configuration
            ui.collapsing("Peers Configuration", |ui| {
                if self.settings_edit_mode {
                    // Basic peer settings
                    if Self::editable_string_field(ui, "Refill Slots Interval", &mut self.editable_config.peers.refill_slots_interval) {
                        self.config_modified = true;
                    }
                    
                    // TODO: Handle trusted_nodes array editing (skip for now as it's complex)
                    if let Some(trusted_nodes) = &self.editable_config.peers.trusted_nodes {
                        ui.label(&format!("Trusted Nodes: {} configured", trusted_nodes.len()));
                    }
                    
                    if Self::editable_bool_field(ui, "Trusted Nodes Only", &mut self.editable_config.peers.trusted_nodes_only) {
                        self.config_modified = true;
                    }
                    
                    if Self::editable_string_field(ui, "Trusted Nodes Resolution Interval", &mut self.editable_config.peers.trusted_nodes_resolution_interval) {
                        self.config_modified = true;
                    }
                    
                    if Self::editable_u32_field(ui, "Max Backoff Count", &mut self.editable_config.peers.max_backoff_count) {
                        self.config_modified = true;
                    }
                    
                    if Self::editable_string_field(ui, "Ban Duration", &mut self.editable_config.peers.ban_duration) {
                        self.config_modified = true;
                    }
                    
                    if Self::editable_string_field(ui, "Incoming IP Throttle Duration", &mut self.editable_config.peers.incoming_ip_throttle_duration) {
                        self.config_modified = true;
                    }
                    
                    ui.add_space(8.0);
                    
                    // Connection Info
                    ui.label("Connection Info:");
                    if let Some(conn_info) = &mut self.editable_config.peers.connection_info {
                        ui.indent("connection_info", |ui| {
                            if Self::editable_u32_field(ui, "Max Outbound", &mut conn_info.max_outbound) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Max Inbound", &mut conn_info.max_inbound) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Max Concurrent Outbound Dials", &mut conn_info.max_concurrent_outbound_dials) {
                                self.config_modified = true;
                            }
                        });
                    } else {
                        if ui.button("+ Add Connection Info").clicked() {
                            self.editable_config.peers.connection_info = Some(ConnectionInfoConfig::default());
                            self.config_modified = true;
                        }
                    }
                    
                    ui.add_space(8.0);
                    
                    // Reputation Weights
                    ui.label("Reputation Weights:");
                    if let Some(rep_weights) = &mut self.editable_config.peers.reputation_weights {
                        ui.indent("reputation_weights", |ui| {
                            if Self::editable_i32_field(ui, "Bad Message", &mut rep_weights.bad_message) {
                                self.config_modified = true;
                            }
                            if Self::editable_i32_field(ui, "Bad Block", &mut rep_weights.bad_block) {
                                self.config_modified = true;
                            }
                            if Self::editable_i32_field(ui, "Bad Transactions", &mut rep_weights.bad_transactions) {
                                self.config_modified = true;
                            }
                            if Self::editable_i32_field(ui, "Already Seen Transactions", &mut rep_weights.already_seen_transactions) {
                                self.config_modified = true;
                            }
                            if Self::editable_i32_field(ui, "Timeout", &mut rep_weights.timeout) {
                                self.config_modified = true;
                            }
                            if Self::editable_i32_field(ui, "Bad Protocol", &mut rep_weights.bad_protocol) {
                                self.config_modified = true;
                            }
                            if Self::editable_i32_field(ui, "Failed to Connect", &mut rep_weights.failed_to_connect) {
                                self.config_modified = true;
                            }
                            if Self::editable_i32_field(ui, "Dropped", &mut rep_weights.dropped) {
                                self.config_modified = true;
                            }
                            if Self::editable_i32_field(ui, "Bad Announcement", &mut rep_weights.bad_announcement) {
                                self.config_modified = true;
                            }
                        });
                    } else {
                        if ui.button("+ Add Reputation Weights").clicked() {
                            self.editable_config.peers.reputation_weights = Some(ReputationWeightsConfig::default());
                            self.config_modified = true;
                        }
                    }
                    
                    ui.add_space(8.0);
                    
                    // Backoff Durations
                    ui.label("Backoff Durations:");
                    if let Some(backoff) = &mut self.editable_config.peers.backoff_durations {
                        ui.indent("backoff_durations", |ui| {
                            if Self::editable_string_field(ui, "Low", &mut backoff.low) {
                                self.config_modified = true;
                            }
                            if Self::editable_string_field(ui, "Medium", &mut backoff.medium) {
                                self.config_modified = true;
                            }
                            if Self::editable_string_field(ui, "High", &mut backoff.high) {
                                self.config_modified = true;
                            }
                            if Self::editable_string_field(ui, "Max", &mut backoff.max) {
                                self.config_modified = true;
                            }
                        });
                    } else {
                        if ui.button("+ Add Backoff Durations").clicked() {
                            self.editable_config.peers.backoff_durations = Some(BackoffDurationsConfig::default());
                            self.config_modified = true;
                        }
                    }
                } else {
                    // Read-only view
                    if let Some(val) = &self.reth_config.peers.refill_slots_interval {
                        ui.label(&format!("Refill Slots Interval: {}", val));
                    }
                    if let Some(val) = &self.reth_config.peers.trusted_nodes {
                        ui.label(&format!("Trusted Nodes: {} configured", val.len()));
                        if val.is_empty() {
                            ui.label("  (Empty list)");
                        }
                    }
                    if let Some(val) = self.reth_config.peers.trusted_nodes_only {
                        ui.label(&format!("Trusted Nodes Only: {}", val));
                    }
                    if let Some(val) = &self.reth_config.peers.trusted_nodes_resolution_interval {
                        ui.label(&format!("Trusted Nodes Resolution Interval: {}", val));
                    }
                    if let Some(val) = self.reth_config.peers.max_backoff_count {
                        ui.label(&format!("Max Backoff Count: {}", val));
                    }
                    if let Some(val) = &self.reth_config.peers.ban_duration {
                        ui.label(&format!("Ban Duration: {}", val));
                    }
                    if let Some(val) = &self.reth_config.peers.incoming_ip_throttle_duration {
                        ui.label(&format!("Incoming IP Throttle Duration: {}", val));
                    }
                    
                    // Connection Info
                    if let Some(conn_info) = &self.reth_config.peers.connection_info {
                        ui.add_space(8.0);
                        ui.label("Connection Info:");
                        if let Some(val) = conn_info.max_outbound {
                            ui.label(&format!("  • Max Outbound: {}", val));
                        }
                        if let Some(val) = conn_info.max_inbound {
                            ui.label(&format!("  • Max Inbound: {}", val));
                        }
                        if let Some(val) = conn_info.max_concurrent_outbound_dials {
                            ui.label(&format!("  • Max Concurrent Outbound Dials: {}", val));
                        }
                    }
                    
                    // Reputation Weights
                    if let Some(rep_weights) = &self.reth_config.peers.reputation_weights {
                        ui.add_space(8.0);
                        ui.label("Reputation Weights:");
                        if let Some(val) = rep_weights.bad_message {
                            ui.label(&format!("  • Bad Message: {}", val));
                        }
                        if let Some(val) = rep_weights.bad_block {
                            ui.label(&format!("  • Bad Block: {}", val));
                        }
                        if let Some(val) = rep_weights.bad_transactions {
                            ui.label(&format!("  • Bad Transactions: {}", val));
                        }
                        if let Some(val) = rep_weights.already_seen_transactions {
                            ui.label(&format!("  • Already Seen Transactions: {}", val));
                        }
                        if let Some(val) = rep_weights.timeout {
                            ui.label(&format!("  • Timeout: {}", val));
                        }
                        if let Some(val) = rep_weights.bad_protocol {
                            ui.label(&format!("  • Bad Protocol: {}", val));
                        }
                        if let Some(val) = rep_weights.failed_to_connect {
                            ui.label(&format!("  • Failed to Connect: {}", val));
                        }
                        if let Some(val) = rep_weights.dropped {
                            ui.label(&format!("  • Dropped: {}", val));
                        }
                        if let Some(val) = rep_weights.bad_announcement {
                            ui.label(&format!("  • Bad Announcement: {}", val));
                        }
                    }
                    
                    // Backoff Durations
                    if let Some(backoff) = &self.reth_config.peers.backoff_durations {
                        ui.add_space(8.0);
                        ui.label("Backoff Durations:");
                        if let Some(val) = &backoff.low {
                            ui.label(&format!("  • Low: {}", val));
                        }
                        if let Some(val) = &backoff.medium {
                            ui.label(&format!("  • Medium: {}", val));
                        }
                        if let Some(val) = &backoff.high {
                            ui.label(&format!("  • High: {}", val));
                        }
                        if let Some(val) = &backoff.max {
                            ui.label(&format!("  • Max: {}", val));
                        }
                    }
                }
            });
            
            ui.add_space(12.0);
            
            // Sessions Configuration
            ui.collapsing("Sessions Configuration", |ui| {
                if self.settings_edit_mode {
                    // Basic session settings
                    if Self::editable_u32_field(ui, "Session Command Buffer", &mut self.editable_config.sessions.session_command_buffer) {
                        self.config_modified = true;
                    }
                    
                    if Self::editable_u32_field(ui, "Session Event Buffer", &mut self.editable_config.sessions.session_event_buffer) {
                        self.config_modified = true;
                    }
                    
                    ui.add_space(8.0);
                    
                    // Session Limits (empty struct, just show configured status)
                    if self.editable_config.sessions.limits.is_some() {
                        ui.label("Limits: Configured");
                        if ui.button("Remove Limits").clicked() {
                            self.editable_config.sessions.limits = None;
                            self.config_modified = true;
                        }
                    } else {
                        if ui.button("+ Add Limits").clicked() {
                            self.editable_config.sessions.limits = Some(SessionLimitsConfig::default());
                            self.config_modified = true;
                        }
                    }
                    
                    ui.add_space(8.0);
                    
                    // Initial Internal Request Timeout
                    ui.label("Initial Internal Request Timeout:");
                    if let Some(timeout) = &mut self.editable_config.sessions.initial_internal_request_timeout {
                        ui.indent("initial_timeout", |ui| {
                            if Self::editable_u64_field(ui, "Seconds", &mut timeout.secs) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Nanoseconds", &mut timeout.nanos) {
                                self.config_modified = true;
                            }
                        });
                    } else {
                        if ui.button("+ Add Initial Internal Request Timeout").clicked() {
                            self.editable_config.sessions.initial_internal_request_timeout = Some(TimeoutConfig::default());
                            self.config_modified = true;
                        }
                    }
                    
                    ui.add_space(8.0);
                    
                    // Protocol Breach Request Timeout
                    ui.label("Protocol Breach Request Timeout:");
                    if let Some(timeout) = &mut self.editable_config.sessions.protocol_breach_request_timeout {
                        ui.indent("protocol_breach_timeout", |ui| {
                            if Self::editable_u64_field(ui, "Seconds", &mut timeout.secs) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Nanoseconds", &mut timeout.nanos) {
                                self.config_modified = true;
                            }
                        });
                    } else {
                        if ui.button("+ Add Protocol Breach Request Timeout").clicked() {
                            self.editable_config.sessions.protocol_breach_request_timeout = Some(TimeoutConfig::default());
                            self.config_modified = true;
                        }
                    }
                    
                    ui.add_space(8.0);
                    
                    // Pending Session Timeout
                    ui.label("Pending Session Timeout:");
                    if let Some(timeout) = &mut self.editable_config.sessions.pending_session_timeout {
                        ui.indent("pending_session_timeout", |ui| {
                            if Self::editable_u64_field(ui, "Seconds", &mut timeout.secs) {
                                self.config_modified = true;
                            }
                            if Self::editable_u32_field(ui, "Nanoseconds", &mut timeout.nanos) {
                                self.config_modified = true;
                            }
                        });
                    } else {
                        if ui.button("+ Add Pending Session Timeout").clicked() {
                            self.editable_config.sessions.pending_session_timeout = Some(TimeoutConfig::default());
                            self.config_modified = true;
                        }
                    }
                } else {
                    // Read-only view
                    if let Some(val) = self.reth_config.sessions.session_command_buffer {
                        ui.label(&format!("Session Command Buffer: {}", val));
                    }
                    if let Some(val) = self.reth_config.sessions.session_event_buffer {
                        ui.label(&format!("Session Event Buffer: {}", val));
                    }
                    
                    if self.reth_config.sessions.limits.is_some() {
                        ui.label("Limits: Configured");
                    }
                    
                    if let Some(timeout) = &self.reth_config.sessions.initial_internal_request_timeout {
                        ui.label("Initial Internal Request Timeout:");
                        if let Some(secs) = timeout.secs {
                            ui.label(&format!("  • Seconds: {}", secs));
                        }
                        if let Some(nanos) = timeout.nanos {
                            ui.label(&format!("  • Nanoseconds: {}", nanos));
                        }
                    }
                    
                    if let Some(timeout) = &self.reth_config.sessions.protocol_breach_request_timeout {
                        ui.label("Protocol Breach Request Timeout:");
                        if let Some(secs) = timeout.secs {
                            ui.label(&format!("  • Seconds: {}", secs));
                        }
                        if let Some(nanos) = timeout.nanos {
                            ui.label(&format!("  • Nanoseconds: {}", nanos));
                        }
                    }
                    
                    if let Some(timeout) = &self.reth_config.sessions.pending_session_timeout {
                        ui.label("Pending Session Timeout:");
                        if let Some(secs) = timeout.secs {
                            ui.label(&format!("  • Seconds: {}", secs));
                        }
                        if let Some(nanos) = timeout.nanos {
                            ui.label(&format!("  • Nanoseconds: {}", nanos));
                        }
                    }
                }
            });
            
            ui.add_space(12.0);
            
            // Pruning Configuration
            ui.collapsing("Pruning Configuration", |ui| {
                if self.settings_edit_mode {
                    // Basic prune settings
                    if Self::editable_u64_field(ui, "Block Interval", &mut self.editable_config.prune.block_interval) {
                        self.config_modified = true;
                    }
                    
                    ui.add_space(8.0);
                    
                    // Pruning Segments
                    ui.label("Pruning Segments:");
                    if let Some(segments) = &mut self.editable_config.prune.segments {
                        ui.indent("prune_segments", |ui| {
                            // Sender Recovery
                            if Self::editable_string_field(ui, "Sender Recovery", &mut segments.sender_recovery) {
                                self.config_modified = true;
                            }
                            
                            ui.add_space(4.0);
                            
                            // Receipts
                            ui.label("Receipts:");
                            if let Some(receipts) = &mut segments.receipts {
                                ui.indent("receipts", |ui| {
                                    if Self::editable_u64_field(ui, "Distance", &mut receipts.distance) {
                                        self.config_modified = true;
                                    }
                                });
                            } else {
                                if ui.button("+ Add Receipts").clicked() {
                                    segments.receipts = Some(PruneReceiptsConfig::default());
                                    self.config_modified = true;
                                }
                            }
                            
                            ui.add_space(4.0);
                            
                            // Account History
                            ui.label("Account History:");
                            if let Some(account_history) = &mut segments.account_history {
                                ui.indent("account_history", |ui| {
                                    if Self::editable_u64_field(ui, "Distance", &mut account_history.distance) {
                                        self.config_modified = true;
                                    }
                                });
                            } else {
                                if ui.button("+ Add Account History").clicked() {
                                    segments.account_history = Some(PruneHistoryConfig::default());
                                    self.config_modified = true;
                                }
                            }
                            
                            ui.add_space(4.0);
                            
                            // Storage History
                            ui.label("Storage History:");
                            if let Some(storage_history) = &mut segments.storage_history {
                                ui.indent("storage_history", |ui| {
                                    if Self::editable_u64_field(ui, "Distance", &mut storage_history.distance) {
                                        self.config_modified = true;
                                    }
                                });
                            } else {
                                if ui.button("+ Add Storage History").clicked() {
                                    segments.storage_history = Some(PruneHistoryConfig::default());
                                    self.config_modified = true;
                                }
                            }
                            
                            ui.add_space(4.0);
                            
                            // Receipts Log Filter (empty struct)
                            if segments.receipts_log_filter.is_some() {
                                ui.label("Receipts Log Filter: Configured");
                                if ui.button("Remove Receipts Log Filter").clicked() {
                                    segments.receipts_log_filter = None;
                                    self.config_modified = true;
                                }
                            } else {
                                if ui.button("+ Add Receipts Log Filter").clicked() {
                                    segments.receipts_log_filter = Some(PruneReceiptsLogFilterConfig::default());
                                    self.config_modified = true;
                                }
                            }
                        });
                    } else {
                        if ui.button("+ Add Pruning Segments").clicked() {
                            self.editable_config.prune.segments = Some(PruneSegments::default());
                            self.config_modified = true;
                        }
                    }
                } else {
                    // Read-only view
                    if let Some(val) = self.reth_config.prune.block_interval {
                        ui.label(&format!("Block Interval: {}", val));
                    }
                    
                    if let Some(segments) = &self.reth_config.prune.segments {
                        ui.label("Pruning Segments:");
                        ui.add_space(4.0);
                        
                        if let Some(val) = &segments.sender_recovery {
                            ui.label(&format!("  • Sender Recovery: {}", val));
                        }
                        if let Some(receipts) = &segments.receipts {
                            ui.label("  • Receipts:");
                            if let Some(distance) = receipts.distance {
                                ui.label(&format!("    - Distance: {}", distance));
                            }
                        }
                        if let Some(account_history) = &segments.account_history {
                            ui.label("  • Account History:");
                            if let Some(distance) = account_history.distance {
                                ui.label(&format!("    - Distance: {}", distance));
                            }
                        }
                        if let Some(storage_history) = &segments.storage_history {
                            ui.label("  • Storage History:");
                            if let Some(distance) = storage_history.distance {
                                ui.label(&format!("    - Distance: {}", distance));
                            }
                        }
                        if segments.receipts_log_filter.is_some() {
                            ui.label("  • Receipts Log Filter: Configured");
                        }
                    }
                }
            });
            
            ui.add_space(24.0);
            
            ui.horizontal(|ui| {
                if self.settings_edit_mode {
                    // Save button (only enabled if there are changes)
                    let save_button = egui::Button::new("💾 Save Changes")
                        .fill(if self.config_modified { RethTheme::SUCCESS } else { RethTheme::SURFACE });
                    
                    if ui.add_enabled(self.config_modified, save_button).clicked() {
                        match self.save_reth_config() {
                            Ok(()) => {
                                self.settings_edit_mode = false; // Exit edit mode after saving
                            }
                            Err(e) => {
                                eprintln!("Failed to save configuration: {}", e);
                            }
                        }
                    }
                    
                    ui.add_space(8.0);
                    
                    // Cancel/Reset button (only enabled if there are changes)
                    if ui.add_enabled(self.config_modified, egui::Button::new("↶ Reset Changes")).clicked() {
                        self.reset_editable_config();
                    }
                    
                    ui.add_space(8.0);
                    
                    if self.config_modified {
                        ui.label(RethTheme::warning_text("⚠ Unsaved changes"));
                    }
                } else {
                    if ui.button("🔄 Reload Config").clicked() {
                        let (config, path) = Self::load_reth_config();
                        self.reth_config = config.clone();
                        self.reth_config_path = path;
                        self.editable_config = config;
                        self.config_modified = false;
                    }
                }
            });
        });
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply custom theme
        RethTheme::apply(ctx);
        
        // Update status from installer using try_lock (only if we're actively installing)
        if self.installing {
            if let Ok(installer) = self.installer.try_lock() {
                let new_status = installer.status().clone();
                
                // Check if installation just completed
                if matches!(new_status, InstallStatus::Completed) && !matches!(self.install_status, InstallStatus::Completed) {
                    self.is_reth_installed = true;
                    self.was_detected_on_startup = false; // This was a fresh install
                }
                
                self.install_status = new_status;
                if matches!(self.install_status, InstallStatus::Completed | InstallStatus::Error(_)) {
                    self.installing = false;
                }
            }
        }
        
        // Handle update check results from background task
        while let Ok((latest, update_available)) = self.update_receiver.try_recv() {
            self.latest_version = Some(latest.clone());
            self.update_available = update_available;
            if update_available {
                println!("Update available: {} -> {}", 
                    self.installed_version.as_ref().unwrap_or(&"unknown".to_string()), 
                    latest);
            }
        }
        
        // Update Reth node status and collect logs
        if matches!(self.install_status, InstallStatus::Running) {
            self.reth_node.check_process_status();
            let new_logs = self.reth_node.get_logs();
            self.node_logs.extend(new_logs);
            
            // Keep only last 1000 logs for performance
            if self.node_logs.len() > 1000 {
                self.node_logs.drain(0..self.node_logs.len() - 1000);
            }
            
            if !self.reth_node.is_running() {
                self.install_status = InstallStatus::Stopped;
            }
        }

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Settings", |ui| {
                    if ui.button("Node Configuration").clicked() {
                        self.show_settings = true;
                        self.reset_editable_config(); // Reset to current saved state when opening
                        ui.close_menu();
                    }
                });
            });
        });

        // Footer panel (fixed at bottom)
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    // "Open Source" link
                    let open_source_link = egui::RichText::new("Open Source")
                        .size(12.0)
                        .color(RethTheme::PRIMARY);
                    
                    if ui.link(open_source_link).clicked() {
                        let _ = std::process::Command::new("open")
                            .arg("https://github.com/bford21/reth-desktop")
                            .spawn();
                    }
                    
                    ui.label(RethTheme::muted_text("and made with"));
                    ui.label(RethTheme::muted_text("❤️"));
                    ui.label(RethTheme::muted_text("by"));
                    
                    // "beef" link
                    let beef_link = egui::RichText::new("beef")
                        .size(12.0)
                        .color(RethTheme::PRIMARY);
                    
                    if ui.link(beef_link).clicked() {
                        let _ = std::process::Command::new("open")
                            .arg("https://x.com/cryptodevbrian")
                            .spawn();
                    }
                });
            });
            ui.add_space(8.0);
        });
        
        // Settings window
        if self.show_settings {
            egui::Window::new("Reth Node Configuration")
                .resizable(true)
                .default_width(600.0)
                .default_height(500.0)
                .show(ctx, |ui| {
                    self.show_settings_content(ui);
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.add_space(40.0);
                    
                    // Header section with Reth branding and logo
                    ui.vertical_centered(|ui| {
                        // Display logo if available
                        if let Some(logo_texture) = &self.reth_logo {
                            let logo_size = logo_texture.size_vec2();
                            // Scale the logo to a reasonable size (max 200px width)
                            let scale = (200.0 / logo_size.x).min(1.0);
                            let display_size = logo_size * scale;
                            
                            ui.add(egui::Image::new(logo_texture).max_size(display_size));
                            ui.add_space(16.0);
                        } else {
                            // Fallback text header if image fails to load
                            ui.label(RethTheme::heading_text("RETH"));
                            ui.add_space(8.0);
                        }
                        
                        ui.label(RethTheme::muted_text("Rust Ethereum Execution Client"));
                        ui.add_space(4.0);
                        ui.label(RethTheme::muted_text("Modular, contributor-friendly and blazing-fast"));
                    });
                    
                    ui.add_space(40.0);
            
            // Main content area
            ui.vertical_centered_justified(|ui| {
                let max_width = 1100.0;
                
                // System Requirements Card (only show if not installed and before installation is completed)
                if !self.is_reth_installed && !matches!(self.install_status, InstallStatus::Completed | InstallStatus::Running | InstallStatus::Stopped) {
                    egui::Frame::none()
                        .fill(RethTheme::SURFACE)
                        .rounding(12.0)
                        .inner_margin(24.0)
                        .stroke(egui::Stroke::new(1.0, RethTheme::BORDER))
                        .show(ui, |ui| {
                        ui.set_max_width(max_width);
                        
                        ui.label(RethTheme::subheading_text("System Requirements"));
                        ui.add_space(16.0);
                        
                        // Disk Space Requirement with modern styling
                        egui::Frame::none()
                            .fill(RethTheme::BACKGROUND)
                            .rounding(8.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (icon, color) = if self.system_requirements.disk_space.meets_requirement {
                                        ("✓", RethTheme::SUCCESS)
                                    } else {
                                        ("✗", RethTheme::ERROR)
                                    };
                                    
                                    ui.label(egui::RichText::new(icon).size(18.0).color(color));
                                    ui.add_space(12.0);
                                    
                                    ui.vertical(|ui| {
                                        ui.label(RethTheme::body_text("Storage Space"));
                                        ui.label(RethTheme::muted_text(&format!(
                                            "{:.1} GB available / {:.0} GB required",
                                            self.system_requirements.disk_space.available_gb,
                                            self.system_requirements.disk_space.required_gb
                                        )));
                                    });
                                });
                            });
                        
                        ui.add_space(12.0);
                        
                        // Memory Requirement with modern styling
                        egui::Frame::none()
                            .fill(RethTheme::BACKGROUND)
                            .rounding(8.0)
                            .inner_margin(16.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let (icon, color) = if self.system_requirements.memory.meets_requirement {
                                        ("✓", RethTheme::SUCCESS)
                                    } else {
                                        ("✗", RethTheme::ERROR)
                                    };
                                    
                                    ui.label(egui::RichText::new(icon).size(18.0).color(color));
                                    ui.add_space(12.0);
                                    
                                    ui.vertical(|ui| {
                                        ui.label(RethTheme::body_text("Memory (RAM)"));
                                        ui.label(RethTheme::muted_text(&format!(
                                            "{:.1} GB total / {:.0} GB required",
                                            self.system_requirements.memory.total_gb,
                                            self.system_requirements.memory.required_gb
                                        )));
                                    });
                                });
                            });
                        });
                    
                    ui.add_space(24.0);
                    
                    // Warning message if requirements not met
                    if !self.system_requirements.all_requirements_met() {
                        egui::Frame::none()
                            .fill(RethTheme::WARNING.gamma_multiply(0.1))
                            .rounding(8.0)
                            .inner_margin(16.0)
                            .stroke(egui::Stroke::new(1.0, RethTheme::WARNING))
                            .show(ui, |ui| {
                                ui.set_max_width(max_width);
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new("⚠").size(18.0).color(RethTheme::WARNING));
                                    ui.add_space(8.0);
                                    ui.vertical(|ui| {
                                        ui.label(RethTheme::warning_text("System Requirements Warning"));
                                        ui.label(RethTheme::muted_text("Your system does not meet all requirements. Installation may fail or Reth may not run properly."));
                                    });
                                });
                            });
                        ui.add_space(16.0);
                    }
                }
                
                
                // Installation section
                match &self.install_status {
                    InstallStatus::Idle => {
                        // Only show install button if Reth is not already installed
                        if !self.is_reth_installed {
                            ui.vertical_centered(|ui| {
                                let button = egui::Button::new(
                                    egui::RichText::new("Install Reth")
                                        .size(16.0)
                                        .color(RethTheme::TEXT_PRIMARY)
                                )
                                .min_size(egui::vec2(200.0, 50.0))
                                .fill(RethTheme::PRIMARY);
                                
                                if ui.add(button).clicked() && !self.installing {
                                    self.start_installation(ctx.clone());
                                }
                                
                                // Show platform info when installing
                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    ui.label(RethTheme::muted_text("Platform:"));
                                    ui.label(RethTheme::muted_text(std::env::consts::OS));
                                    ui.label(RethTheme::muted_text("•"));
                                    ui.label(RethTheme::muted_text(std::env::consts::ARCH));
                                });
                            });
                        }
                    }
                    InstallStatus::FetchingVersion => {
                        egui::Frame::none()
                            .fill(RethTheme::SURFACE)
                            .rounding(8.0)
                            .inner_margin(20.0)
                            .show(ui, |ui| {
                                ui.set_max_width(max_width);
                                ui.vertical_centered(|ui| {
                                    ui.label(RethTheme::body_text("Fetching latest version..."));
                                    ui.add_space(8.0);
                                    ui.spinner();
                                });
                            });
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    }
                    InstallStatus::Downloading(progress) => {
                        egui::Frame::none()
                            .fill(RethTheme::SURFACE)
                            .rounding(8.0)
                            .inner_margin(20.0)
                            .show(ui, |ui| {
                                ui.set_max_width(max_width);
                                ui.vertical_centered(|ui| {
                                    ui.label(RethTheme::body_text(&format!("Downloading Reth... {:.1}%", progress)));
                                    ui.add_space(8.0);
                                    
                                    let progress_bar = egui::ProgressBar::new(progress / 100.0)
                                        .desired_width(max_width - 40.0)
                                        .animate(true)
                                        .fill(RethTheme::PRIMARY);
                                    ui.add(progress_bar);
                                });
                            });
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    }
                    InstallStatus::Extracting => {
                        egui::Frame::none()
                            .fill(RethTheme::SURFACE)
                            .rounding(8.0)
                            .inner_margin(20.0)
                            .show(ui, |ui| {
                                ui.set_max_width(max_width);
                                ui.vertical_centered(|ui| {
                                    ui.label(RethTheme::body_text("Extracting files..."));
                                    ui.add_space(8.0);
                                    ui.spinner();
                                });
                            });
                        ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    }
                    InstallStatus::Completed => {
                        egui::Frame::none()
                            .fill(RethTheme::SUCCESS.gamma_multiply(0.1))
                            .rounding(8.0)
                            .inner_margin(20.0)
                            .stroke(egui::Stroke::new(1.0, RethTheme::SUCCESS))
                            .show(ui, |ui| {
                                ui.set_max_width(max_width);
                                ui.vertical_centered(|ui| {
                                    // Show different message based on whether this was a fresh install or detected install
                                    if self.was_detected_on_startup {
                                        ui.label(RethTheme::success_text("✓ Reth Installation Detected"));
                                        ui.add_space(8.0);
                                        ui.label(RethTheme::muted_text("Reth is ready to launch from ~/.reth-desktop/bin/"));
                                    } else {
                                        ui.label(RethTheme::success_text("✓ Installation Completed!"));
                                        ui.add_space(8.0);
                                        ui.label(RethTheme::muted_text("Reth has been installed to ~/.reth-desktop/bin/"));
                                    }
                                    
                                    ui.add_space(16.0);
                                    
                                    let launch_button = egui::Button::new(
                                        egui::RichText::new("Launch Reth")
                                            .size(16.0)
                                            .color(RethTheme::TEXT_PRIMARY)
                                    )
                                    .min_size(egui::vec2(160.0, 40.0))
                                    .fill(RethTheme::PRIMARY);
                                    
                                    if ui.add(launch_button).clicked() {
                                        self.launch_reth();
                                    }
                                    
                                    ui.add_space(8.0);
                                    
                                    // Show Update button only if an update is available
                                    if self.update_available {
                                        let update_button = egui::Button::new(
                                            egui::RichText::new("Update Reth")
                                                .size(14.0)
                                                .color(RethTheme::WARNING)
                                        )
                                        .min_size(egui::vec2(120.0, 32.0))
                                        .fill(RethTheme::WARNING.gamma_multiply(0.2));
                                        
                                        if ui.add(update_button).clicked() {
                                            self.install_status = InstallStatus::Idle;
                                            self.is_reth_installed = false; // Allow update installation
                                            self.was_detected_on_startup = false; // Reset detection flag
                                            self.reset_installer();
                                        }
                                        
                                        if let (Some(installed), Some(latest)) = (&self.installed_version, &self.latest_version) {
                                            ui.add_space(4.0);
                                            ui.label(RethTheme::muted_text(&format!("Current: {} → Latest: {}", installed, latest)));
                                        }
                                    }
                                });
                            });
                    }
                    InstallStatus::Running => {
                        // Terminal interface for Reth output
                        egui::Frame::none()
                            .fill(RethTheme::BACKGROUND)
                            .rounding(8.0)
                            .inner_margin(16.0)
                            .stroke(egui::Stroke::new(1.0, RethTheme::BORDER))
                            .show(ui, |ui| {
                                ui.set_max_width(max_width);
                                
                                ui.horizontal(|ui| {
                                    ui.label(RethTheme::success_text("🟢 Reth Node Running"));
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let stop_button = egui::Button::new(RethTheme::error_text("Stop"))
                                            .min_size(egui::vec2(60.0, 24.0));
                                        
                                        if ui.add(stop_button).clicked() {
                                            self.stop_reth();
                                        }
                                    });
                                });
                                
                                ui.add_space(12.0);
                                
                                // Terminal output - scale with GUI size
                                let available_rect = ui.available_rect_before_wrap();
                                let terminal_height = (available_rect.height() * 0.6).max(200.0).min(500.0);
                                
                                egui::Frame::none()
                                    .fill(egui::Color32::BLACK)
                                    .rounding(4.0)
                                    .inner_margin(12.0)
                                    .show(ui, |ui| {
                                        egui::ScrollArea::both()
                                            .max_height(terminal_height)
                                            .auto_shrink([false; 2])
                                            .stick_to_bottom(true)
                                            .show(ui, |ui| {
                                                // Don't constrain width for horizontal scrolling
                                                
                                                for log_line in &self.node_logs {
                                                    ui.horizontal(|ui| {
                                                        // Timestamp
                                                        ui.label(egui::RichText::new(&log_line.timestamp)
                                                            .size(11.0)
                                                            .color(egui::Color32::GRAY)
                                                            .monospace());
                                                        
                                                        ui.add_space(8.0);
                                                        
                                                        // Log content with color based on level
                                                        let color = match log_line.level {
                                                            LogLevel::Error => egui::Color32::from_rgb(255, 100, 100),
                                                            LogLevel::Warn => egui::Color32::from_rgb(255, 200, 100),
                                                            LogLevel::Info => egui::Color32::WHITE,
                                                            LogLevel::Debug => egui::Color32::from_rgb(150, 150, 255),
                                                            LogLevel::Trace => egui::Color32::GRAY,
                                                        };
                                                        
                                                        // Clean the log content to remove ANSI codes and strange characters
                                                        let cleaned_content = Self::clean_log_content(&log_line.content);
                                                        
                                                        ui.label(egui::RichText::new(&cleaned_content)
                                                            .size(12.0)
                                                            .color(color)
                                                            .monospace());
                                                    });
                                                }
                                                
                                                if self.node_logs.is_empty() {
                                                    ui.label(egui::RichText::new("Starting Reth node...")
                                                        .size(12.0)
                                                        .color(egui::Color32::GRAY)
                                                        .monospace());
                                                }
                                            });
                                    });
                            });
                        
                        // Auto-refresh for live updates
                        ctx.request_repaint_after(std::time::Duration::from_millis(500));
                    }
                    InstallStatus::Stopped => {
                        egui::Frame::none()
                            .fill(RethTheme::SURFACE)
                            .rounding(8.0)
                            .inner_margin(20.0)
                            .show(ui, |ui| {
                                ui.set_max_width(max_width);
                                ui.vertical_centered(|ui| {
                                    ui.label(RethTheme::muted_text("⭕ Reth Node Stopped"));
                                    ui.add_space(16.0);
                                    
                                    let restart_button = egui::Button::new(
                                        egui::RichText::new("Start Reth")
                                            .size(16.0)
                                            .color(RethTheme::TEXT_PRIMARY)
                                    )
                                    .min_size(egui::vec2(160.0, 40.0))
                                    .fill(RethTheme::PRIMARY);
                                    
                                    if ui.add(restart_button).clicked() {
                                        self.launch_reth();
                                    }
                                });
                            });
                    }
                    InstallStatus::Error(error) => {
                        let error_message = error.clone();
                        egui::Frame::none()
                            .fill(RethTheme::ERROR.gamma_multiply(0.1))
                            .rounding(8.0)
                            .inner_margin(20.0)
                            .stroke(egui::Stroke::new(1.0, RethTheme::ERROR))
                            .show(ui, |ui| {
                                ui.set_max_width(max_width);
                                ui.vertical_centered(|ui| {
                                    ui.label(RethTheme::error_text("❌ Installation Failed"));
                                    ui.add_space(8.0);
                                    ui.label(RethTheme::muted_text(&error_message));
                                    ui.add_space(16.0);
                                    
                                    let button = egui::Button::new(RethTheme::body_text("Try Again"))
                                        .min_size(egui::vec2(120.0, 36.0));
                                    
                                    if ui.add(button).clicked() {
                                        self.install_status = InstallStatus::Idle;
                                        self.reset_installer();
                                    }
                                });
                            });
                    }
                }
                });
                
                ui.add_space(40.0);
            });
        });
    }
}