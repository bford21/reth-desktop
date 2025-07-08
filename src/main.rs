use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

mod installer;
mod system_check;
mod theme;
mod reth_node;
mod config;
mod settings;
mod ui;
mod metrics;

use installer::{RethInstaller, InstallStatus};
use system_check::SystemRequirements;
use theme::RethTheme;
use reth_node::{RethNode, LogLine, LogLevel};
use config::{RethConfig, RethConfigManager};
use settings::{DesktopSettings, DesktopSettingsManager};
use ui::{DesktopSettingsWindow, NodeSettingsWindow, StartConfigWindow};
use metrics::RethMetrics;


fn main() -> Result<(), eframe::Error> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1200.0, 800.0])
        .with_min_inner_size([800.0, 600.0])
        .with_title("Reth Desktop");
    
    // Try to load app icon using reth-docs.png
    match load_icon() {
        Ok(icon_data) => {
            viewport = viewport.with_icon(icon_data);
        }
        Err(e) => {
            eprintln!("Warning: Failed to load app icon: {}", e);
            // Continue without icon
        }
    }
    
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    
    eframe::run_native(
        "Reth Desktop",
        options,
        Box::new(|cc| Box::new(MyApp::new(cc))),
    )
}

fn load_icon() -> Result<egui::IconData, Box<dyn std::error::Error>> {
    let icon_data = include_bytes!("../assets/reth-docs.png");
    
    // Try to decode the image
    let icon_image = image::load_from_memory(icon_data)?;
    let icon_rgba = icon_image.to_rgba8();
    let (icon_width, icon_height) = icon_rgba.dimensions();
    
    Ok(egui::IconData {
        rgba: icon_rgba.to_vec(),
        width: icon_width,
        height: icon_height,
    })
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
    detected_existing_process: bool,
    installed_version: Option<String>,
    latest_version: Option<String>,
    update_available: bool,
    show_settings: bool,
    show_desktop_settings: bool,
    show_start_config: bool,
    desktop_settings: DesktopSettings,
    reth_config: RethConfig,
    reth_config_path: Option<std::path::PathBuf>,
    editable_config: RethConfig,
    config_modified: bool,
    settings_edit_mode: bool,
    last_debug_log: std::time::Instant,
    show_add_parameter: bool,
    available_cli_options: Vec<reth_node::CliOption>,
    selected_cli_option: Option<usize>,
    parameter_value: String,
    selected_values: Vec<String>,
    pending_launch_args: Vec<String>,
    show_restart_prompt: bool,
    command_section_collapsed: bool,
    metrics: RethMetrics,
    metrics_section_collapsed: bool,
    metrics_poll_sender: Option<mpsc::UnboundedSender<()>>,
    metrics_receiver: mpsc::UnboundedReceiver<String>,
    metrics_sender: mpsc::UnboundedSender<String>,
    expanded_metric: Option<String>, // Track which metric is expanded in popup
    available_metrics: Vec<String>, // All available metrics from Prometheus
    show_metric_selector: bool, // Show metric selection dialog
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
        let (metrics_tx, metrics_rx) = mpsc::unbounded_channel::<String>();
        
        // Load the Reth logo
        let reth_logo = Self::load_logo(&cc.egui_ctx);
        
        // Check if Reth is installed and get version
        let is_reth_installed = Self::check_reth_installed();
        let installed_version = Self::get_installed_version();
        
        // Load Reth configuration
        let (reth_config, reth_config_path) = RethConfigManager::load_reth_config();
        
        // Load desktop settings
        let desktop_settings = DesktopSettingsManager::load_desktop_settings();
        
        // Load CLI options if Reth is installed
        let available_cli_options = if is_reth_installed {
            let reth_path = dirs::home_dir()
                .unwrap_or_default()
                .join(".reth-desktop")
                .join("bin")
                .join("reth");
            RethNode::get_available_cli_options(&reth_path.to_string_lossy())
        } else {
            Vec::new()
        };
        
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
        
        // Create RethNode and check for existing processes
        let mut reth_node = RethNode::new();
        let detect_existing = RethNode::detect_existing_reth_process();
        
        println!("Startup: Reth installed: {}, External process detected: {}", is_reth_installed, detect_existing);
        
        // If Reth is running, try to connect to it
        if detect_existing {
            if let Ok(()) = reth_node.connect_to_existing_process() {
                println!("Found and connected to existing Reth process");
            } else {
                println!("Failed to connect to detected Reth process");
            }
        }
        
        // Initialize metrics with custom metrics from settings
        let mut metrics = RethMetrics::new();
        for metric_name in &desktop_settings.custom_metrics {
            metrics.add_custom_metric(metric_name.clone());
        }
        
        let app = Self {
            installer: Arc::new(Mutex::new(RethInstaller::new())),
            install_status: initial_status,
            installing: false,
            _runtime: runtime,
            install_sender: tx,
            update_receiver: update_rx,
            system_requirements: SystemRequirements::check(),
            reth_logo,
            reth_node,
            node_logs: Vec::new(),
            is_reth_installed,
            was_detected_on_startup: is_reth_installed,
            detected_existing_process: detect_existing,
            installed_version: installed_version.clone(),
            latest_version: None,
            update_available: false,
            show_settings: false,
            show_desktop_settings: false,
            show_start_config: false,
            desktop_settings,
            reth_config: reth_config.clone(),
            reth_config_path,
            editable_config: reth_config,
            config_modified: false,
            settings_edit_mode: false,
            last_debug_log: std::time::Instant::now(),
            show_add_parameter: false,
            available_cli_options,
            selected_cli_option: None,
            parameter_value: String::new(),
            selected_values: Vec::new(),
            pending_launch_args: Vec::new(),
            show_restart_prompt: false,
            command_section_collapsed: true,
            metrics,
            metrics_section_collapsed: false,
            metrics_poll_sender: None,
            metrics_receiver: metrics_rx,
            metrics_sender: metrics_tx,
            expanded_metric: None,
            available_metrics: Vec::new(),
            show_metric_selector: false
        };
        
        app
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
        
        match self.reth_node.start(&reth_path.to_string_lossy(), &self.desktop_settings.custom_launch_args, &self.desktop_settings) {
            Ok(()) => {
                self.install_status = InstallStatus::Running;
                // Clear pending args since they've been applied
                self.pending_launch_args.clear();
                
                // Start metrics polling
                self.start_metrics_polling();
            }
            Err(e) => {
                self.install_status = InstallStatus::Error(format!("Failed to launch Reth: {}", e));
            }
        }
    }
    
    fn stop_metrics_polling(&mut self) {
        if let Some(sender) = self.metrics_poll_sender.take() {
            // Send stop signal to the polling task
            let _ = sender.send(());
            println!("Sent stop signal to metrics polling task");
        }
    }
    
    fn stop_reth(&mut self) {
        // Stop metrics polling first
        self.stop_metrics_polling();
        
        if let Err(e) = self.reth_node.stop() {
            eprintln!("Error stopping Reth: {}", e);
        }
        self.install_status = InstallStatus::Stopped;
    }
    
    
    fn reset_editable_config(&mut self) {
        self.editable_config = self.reth_config.clone();
        self.config_modified = false;
        // Don't reset edit mode here - let the caller decide
    }
    
    fn start_metrics_polling(&mut self) {
        let (tx, mut rx) = mpsc::unbounded_channel::<()>();
        self.metrics_poll_sender = Some(tx);
        
        let metrics_sender = self.metrics_sender.clone();
        let metrics_url = format!("http://{}", self.desktop_settings.reth_defaults.metrics_address);
        
        // Spawn a task to poll metrics
        self._runtime.spawn(async move {
            // Wait a bit for the node to start
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            
            loop {
                // Check if we should stop polling
                if rx.try_recv().is_ok() {
                    break;
                }
                
                // Poll metrics
                match metrics::fetch_metrics(&metrics_url).await {
                    Ok(metrics_text) => {
                        // Send metrics to the UI thread
                        let _ = metrics_sender.send(metrics_text);
                    }
                    Err(e) => {
                        // Node might not be ready yet, that's OK
                        println!("Metrics not ready yet: {}", e);
                    }
                }
                
                // Wait before next poll
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }

    fn disconnect_from_external_reth(&mut self) {
        // Disconnect from monitoring external Reth process
        if let Err(e) = self.reth_node.stop() {
            eprintln!("Error disconnecting from external Reth process: {}", e);
        }
        // Clear logs and reset state
        self.node_logs.clear();
    }
    
    // Removed show_settings_content function - functionality moved to NodeSettingsWindow
    
    fn show_metrics_section(&mut self, ui: &mut egui::Ui) {
        // Show metrics in a clean 3-column grid
        ui.add_space(20.0);
        
        // Initialize custom metrics if needed
        for metric_name in &self.desktop_settings.custom_metrics.clone() {
            self.metrics.add_custom_metric(metric_name.clone());
        }
        
        let mut expanded_metric_name: Option<String> = None;
        let mut metric_to_remove: Option<String> = None;
        
        // Metrics grid matching mockup design
        egui::Grid::new("metrics_grid_mockup")
            .num_columns(3)
            .spacing([20.0, 20.0])
            .show(ui, |ui| {
                let mut count = 0;
                
                // Show default metrics
                let default_metrics = vec![
                    ("Connected Peers", self.metrics.peers_connected.clone()),
                    ("Block Height", self.metrics.block_height.clone()),
                    ("Sync Progress", self.metrics.sync_progress.clone()),
                    ("Memory Usage", self.metrics.memory_usage.clone()),
                    ("Active Downloads", self.metrics.disk_io.clone()),
                ];
                
                for (name, metric) in default_metrics {
                    if self.show_mockup_metric_card(ui, &metric) {
                        expanded_metric_name = Some(name.to_string());
                    }
                    count += 1;
                    if count % 3 == 0 {
                        ui.end_row();
                    }
                }
                
                // Show custom metrics
                let custom_metrics: Vec<(String, metrics::MetricHistory)> = self.metrics.custom_metrics
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                    
                for (metric_name, metric) in custom_metrics {
                    let (expand_clicked, remove_clicked) = self.show_custom_metric_card(ui, &metric, &metric_name);
                    if expand_clicked {
                        expanded_metric_name = Some(metric.name.clone());
                    }
                    if remove_clicked {
                        metric_to_remove = Some(metric_name.clone());
                    }
                    count += 1;
                    if count % 3 == 0 {
                        ui.end_row();
                    }
                }
                
                // Always show add metric card
                self.show_add_metric_card(ui);
                count += 1;
                
                // End row if needed
                if count % 3 != 0 {
                    ui.end_row();
                }
            });
            
        // Handle expanded metric
        if let Some(name) = expanded_metric_name {
            self.expanded_metric = Some(name);
        }
        
        // Handle metric removal
        if let Some(metric_name) = metric_to_remove {
            // Remove from settings
            self.desktop_settings.custom_metrics.retain(|m| m != &metric_name);
            // Remove from metrics
            self.metrics.custom_metrics.remove(&metric_name);
            // Save settings
            if let Err(e) = DesktopSettingsManager::save_desktop_settings(&self.desktop_settings) {
                eprintln!("Failed to save custom metrics: {}", e);
            }
        }
    }
    
    fn show_mockup_metric_card(&self, ui: &mut egui::Ui, metric: &metrics::MetricHistory) -> bool {
        let mut expand_clicked = false;
        
        ui.vertical(|ui| {
            // Title outside the box with expand button
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&metric.name)
                    .size(14.0)
                    .color(RethTheme::TEXT_PRIMARY)
                    .strong());
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Use simple text for better rendering
                    let button = egui::Button::new("View")
                        .min_size(egui::Vec2::new(0.0, 0.0)) // Reset minimum size
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::new(1.0, RethTheme::TEXT_SECONDARY));
                    
                    // Apply custom padding for equal spacing
                    ui.style_mut().spacing.button_padding = egui::Vec2::new(6.0, 4.0);
                    
                    if ui.add(button).on_hover_text("View full history").clicked() {
                        expand_clicked = true;
                    }
                });
            });
            
            ui.add_space(4.0);
            
            // Frame containing only the graph
            egui::Frame::none()
                .fill(RethTheme::SURFACE)
                .rounding(8.0)
                .inner_margin(egui::Margin::same(8.0)) // Equal padding all around
                .stroke(egui::Stroke::new(1.0, RethTheme::PRIMARY.gamma_multiply(0.3)))
                .show(ui, |ui| {
                    ui.set_min_size(egui::Vec2::new(350.0, 180.0));
                    
                    // Check if we have data
                    if metric.values.is_empty() {
                        // Show "No data" message
                        ui.centered_and_justified(|ui| {
                            ui.label(egui::RichText::new("No data")
                                .size(16.0)
                                .color(RethTheme::TEXT_SECONDARY));
                        });
                    } else {
                        // Draw graph that fills the frame (limited to 5 minutes)
                        self.draw_metric_graph_limited(ui, metric, 300); // 300 seconds = 5 minutes
                    }
                });
        });
        
        expand_clicked
    }
    
    fn show_custom_metric_card(&self, ui: &mut egui::Ui, metric: &metrics::MetricHistory, _metric_key: &str) -> (bool, bool) {
        let mut expand_clicked = false;
        let mut remove_clicked = false;
        
        ui.vertical(|ui| {
            // Title outside the box with expand and remove buttons
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&metric.name)
                    .size(14.0)
                    .color(RethTheme::TEXT_PRIMARY)
                    .strong());
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Apply custom padding for equal spacing
                    ui.style_mut().spacing.button_padding = egui::Vec2::new(6.0, 4.0);
                    
                    // Remove button
                    let remove_button = egui::Button::new("×")
                        .min_size(egui::Vec2::new(0.0, 0.0))
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::new(1.0, RethTheme::ERROR));
                    
                    if ui.add(remove_button).on_hover_text("Remove metric").clicked() {
                        remove_clicked = true;
                    }
                    
                    ui.add_space(4.0);
                    
                    // View button
                    let view_button = egui::Button::new("View")
                        .min_size(egui::Vec2::new(0.0, 0.0))
                        .fill(egui::Color32::TRANSPARENT)
                        .stroke(egui::Stroke::new(1.0, RethTheme::TEXT_SECONDARY));
                    
                    if ui.add(view_button).on_hover_text("View full history").clicked() {
                        expand_clicked = true;
                    }
                });
            });
            
            ui.add_space(4.0);
            
            // Frame containing only the graph
            egui::Frame::none()
                .fill(RethTheme::SURFACE)
                .rounding(8.0)
                .inner_margin(egui::Margin::same(8.0))
                .stroke(egui::Stroke::new(1.0, RethTheme::PRIMARY.gamma_multiply(0.3)))
                .show(ui, |ui| {
                    ui.set_min_size(egui::Vec2::new(350.0, 180.0));
                    
                    // Check if we have data
                    if metric.values.is_empty() {
                        // Show "No data" message
                        ui.centered_and_justified(|ui| {
                            ui.label(egui::RichText::new("No data")
                                .size(16.0)
                                .color(RethTheme::TEXT_SECONDARY));
                        });
                    } else {
                        // Draw graph that fills the frame (limited to 5 minutes)
                        self.draw_metric_graph_limited(ui, metric, 300);
                    }
                });
        });
        
        (expand_clicked, remove_clicked)
    }
    
    fn show_add_metric_card(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Empty label to match the height of other metric titles
            ui.label(egui::RichText::new(" ")
                .size(14.0));
            
            ui.add_space(4.0);
            
            // Create interactive area for the entire card
            let (rect, response) = ui.allocate_exact_size(
                egui::Vec2::new(350.0, 196.0), // Match the height of other cards (180 + margins)
                egui::Sense::click()
            );
            
            let is_hovered = response.hovered();
            
            // Draw the frame
            let painter = ui.painter();
            painter.rect(
                rect,
                8.0,
                if is_hovered { 
                    RethTheme::SURFACE.gamma_multiply(1.2) 
                } else { 
                    RethTheme::SURFACE 
                },
                egui::Stroke::new(
                    1.0, 
                    if is_hovered { 
                        RethTheme::PRIMARY 
                    } else { 
                        RethTheme::PRIMARY.gamma_multiply(0.3) 
                    }
                )
            );
            
            // Draw centered "+" sign
            let color = if is_hovered {
                RethTheme::PRIMARY
            } else {
                RethTheme::TEXT_SECONDARY
            };
            
            let stroke = egui::Stroke::new(3.0, color);
            let center = rect.center();
            let size = 20.0;
            
            // Horizontal line
            painter.line_segment(
                [
                    egui::Pos2::new(center.x - size, center.y),
                    egui::Pos2::new(center.x + size, center.y),
                ],
                stroke,
            );
            
            // Vertical line
            painter.line_segment(
                [
                    egui::Pos2::new(center.x, center.y - size),
                    egui::Pos2::new(center.x, center.y + size),
                ],
                stroke,
            );
            
            // Add tooltip and cursor change on hover
            if is_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            
            // Handle click and tooltip
            let response = response.on_hover_text("Add a new metric");
            if response.clicked() {
                self.show_metric_selector = true;
            }
        });
    }
    
    fn draw_metric_graph_limited(&self, ui: &mut egui::Ui, metric: &metrics::MetricHistory, max_seconds: usize) {
        // Don't draw anything if there's no data (handled by caller)
        if metric.values.is_empty() {
            return;
        }
        
        // Only show the last N data points (max_seconds)
        let start_idx = metric.values.len().saturating_sub(max_seconds);
        
        // Convert metric values to plot points with time on x-axis
        let points: Vec<[f64; 2]> = metric.values
            .iter()
            .skip(start_idx)
            .enumerate()
            .map(|(i, value)| {
                [i as f64, value.value]
            })
            .collect();
        let plot_points = PlotPoints::new(points);
        
        // Configure the plot
        let line = Line::new(plot_points)
            .color(RethTheme::PRIMARY)
            .style(egui_plot::LineStyle::Solid)
            .width(2.0)
            .fill(0.0); // Fill to y=0
        
        // Clone the unit to avoid lifetime issues
        let unit = metric.unit.clone();
        let unit_for_formatter = unit.clone();
        
        // Create the plot with proper axis labels and formatting
        let plot = Plot::new(format!("metric_plot_{}", metric.name))
            .auto_bounds(egui::Vec2b::new(true, true))
            .show_axes([true, true])
            .show_grid([false, false]) // Only show axes, no grid
            .include_y(0.0) // Always show y=0
            .allow_zoom(false)
            .allow_drag(false)
            .allow_boxed_zoom(false)
            .allow_scroll(false)
            .show_background(false)
            .y_axis_width(4) // Give more space for y-axis labels
            .label_formatter(move |_name, value| {
                // Format hover values
                match unit.as_str() {
                    "%" => format!("{:.0}%", value.y),
                    "MB" => format!("{:.0} MB", value.y),
                    "peers" => format!("{:.0} peers", value.y),
                    "blocks" => {
                        if value.y >= 1_000_000.0 {
                            format!("{:.1}M blocks", value.y / 1_000_000.0)
                        } else if value.y >= 1000.0 {
                            format!("{:.1}k blocks", value.y / 1000.0)
                        } else {
                            format!("{:.0} blocks", value.y)
                        }
                    },
                    "txs" => format!("{:.0} txs", value.y),
                    _ => format!("{:.1} {}", value.y, unit),
                }
            })
            .x_axis_formatter(|value, _max_chars, _range| {
                // Show time labels - convert from data point index to time
                let seconds = value as i32;
                if seconds == 0 {
                    "0s".to_string()
                } else if seconds % 60 == 0 {
                    format!("{}m", seconds / 60)
                } else if seconds < 60 && seconds % 15 == 0 {
                    format!("{}s", seconds)
                } else {
                    String::new() // Don't show label for non-round times
                }
            })
            .y_axis_formatter(move |value, _max_chars, _range| {
                // Format y-axis labels based on unit type with consistent formatting
                match unit_for_formatter.as_str() {
                    "%" => format!("{:.0}%", value),
                    "MB" => {
                        if value >= 10000.0 {
                            format!("{:.0}G", value / 1000.0)
                        } else if value >= 1000.0 {
                            format!("{:.1}G", value / 1000.0)
                        } else {
                            format!("{:.0}", value)
                        }
                    },
                    "blocks" => {
                        if value >= 1_000_000_000.0 {
                            format!("{:.0}B", value / 1_000_000_000.0)
                        } else if value >= 1_000_000.0 {
                            format!("{:.0}M", value / 1_000_000.0)
                        } else if value >= 1000.0 {
                            format!("{:.0}k", value / 1000.0)
                        } else {
                            format!("{:.0}", value)
                        }
                    },
                    "peers" => format!("{:.0}", value),
                    _ => {
                        if value >= 1000.0 {
                            format!("{:.0}k", value / 1000.0)
                        } else {
                            format!("{:.0}", value)
                        }
                    },
                }
            });
        
        // Show the plot
        plot.show(ui, |plot_ui| {
            plot_ui.line(line);
        });
    }
    
    fn draw_metric_graph(&self, ui: &mut egui::Ui, metric: &metrics::MetricHistory) {
        // Don't draw anything if there's no data (handled by caller)
        if metric.values.is_empty() {
            return;
        }
        
        // Convert metric values to plot points with time on x-axis
        let points: Vec<[f64; 2]> = metric.values
            .iter()
            .enumerate()
            .map(|(i, value)| {
                [i as f64, value.value]
            })
            .collect();
        let plot_points = PlotPoints::new(points);
        
        // Configure the plot
        let line = Line::new(plot_points)
            .color(RethTheme::PRIMARY)
            .style(egui_plot::LineStyle::Solid)
            .width(2.0)
            .fill(0.0); // Fill to y=0
        
        // Clone the unit to avoid lifetime issues
        let unit = metric.unit.clone();
        let unit_for_formatter = unit.clone();
        
        // Create the plot with proper axis labels and formatting
        let plot = Plot::new(format!("metric_plot_{}", metric.name))
            .auto_bounds(egui::Vec2b::new(true, true))
            .show_axes([true, true])
            .show_grid([false, false]) // Only show axes, no grid
            .include_y(0.0) // Always show y=0
            .allow_zoom(false)
            .allow_drag(false)
            .allow_boxed_zoom(false)
            .allow_scroll(false)
            .show_background(false)
            .y_axis_width(4) // Give more space for y-axis labels
            .label_formatter(move |_name, value| {
                // Format hover values
                match unit.as_str() {
                    "%" => format!("{:.0}%", value.y),
                    "MB" => format!("{:.0} MB", value.y),
                    "peers" => format!("{:.0} peers", value.y),
                    "blocks" => {
                        if value.y >= 1_000_000.0 {
                            format!("{:.1}M blocks", value.y / 1_000_000.0)
                        } else if value.y >= 1000.0 {
                            format!("{:.1}k blocks", value.y / 1000.0)
                        } else {
                            format!("{:.0} blocks", value.y)
                        }
                    },
                    "txs" => format!("{:.0} txs", value.y),
                    _ => format!("{:.1} {}", value.y, unit),
                }
            })
            .x_axis_formatter(|value, _max_chars, _range| {
                // Show time labels - convert from data point index to time
                let seconds = value as i32;
                if seconds == 0 {
                    "0s".to_string()
                } else if seconds % 60 == 0 {
                    format!("{}m", seconds / 60)
                } else if seconds < 60 {
                    format!("{}s", seconds)
                } else {
                    String::new() // Don't show label for non-round times
                }
            })
            .y_axis_formatter(move |value, _max_chars, _range| {
                // Format y-axis labels based on unit type with consistent formatting
                match unit_for_formatter.as_str() {
                    "%" => format!("{:.0}%", value),
                    "MB" => {
                        if value >= 10000.0 {
                            format!("{:.0}G", value / 1000.0)
                        } else if value >= 1000.0 {
                            format!("{:.1}G", value / 1000.0)
                        } else {
                            format!("{:.0}", value)
                        }
                    },
                    "blocks" => {
                        if value >= 1_000_000_000.0 {
                            format!("{:.0}B", value / 1_000_000_000.0)
                        } else if value >= 1_000_000.0 {
                            format!("{:.0}M", value / 1_000_000.0)
                        } else if value >= 1000.0 {
                            format!("{:.0}k", value / 1000.0)
                        } else {
                            format!("{:.0}", value)
                        }
                    },
                    "peers" => format!("{:.0}", value),
                    _ => {
                        if value >= 1000.0 {
                            format!("{:.0}k", value / 1000.0)
                        } else {
                            format!("{:.0}", value)
                        }
                    },
                }
            });
        
        // Show the plot
        plot.show(ui, |plot_ui| {
            plot_ui.line(line);
        });
    }
    
    fn show_large_metric_card(&self, ui: &mut egui::Ui, metric: &metrics::MetricHistory, is_primary: bool) {
        let bg_color = if is_primary { RethTheme::PRIMARY.gamma_multiply(0.1) } else { RethTheme::BACKGROUND };
        let border_color = if is_primary { RethTheme::PRIMARY.gamma_multiply(0.3) } else { RethTheme::BORDER };
        
        ui.vertical(|ui| {
            // Title with current value outside the box
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&metric.name)
                    .size(13.0)
                    .color(RethTheme::TEXT_PRIMARY)
                    .strong());
                
                ui.add_space(8.0);
                
                // Current value
                let current_value = metric.get_latest().unwrap_or(0.0);
                let (value_text, value_color) = if metric.unit == "%" {
                    let color = if current_value > 95.0 { RethTheme::SUCCESS } 
                              else if current_value > 80.0 { RethTheme::WARNING }
                              else { RethTheme::TEXT_PRIMARY };
                    (format!("{:.1}%", current_value), color)
                } else if metric.unit == "MB" {
                    let color = if current_value > 1000.0 { RethTheme::WARNING }
                              else if current_value > 2000.0 { RethTheme::ERROR }
                              else { RethTheme::TEXT_PRIMARY };
                    (format!("{:.1} MB", current_value), color)
                } else if metric.unit == "gwei" {
                    (format!("{:.2} gwei", current_value), RethTheme::TEXT_PRIMARY)
                } else if metric.unit == "peers" {
                    let color = if current_value >= 5.0 { RethTheme::SUCCESS }
                              else if current_value >= 1.0 { RethTheme::WARNING }
                              else { RethTheme::ERROR };
                    (format!("{:.0}", current_value), color)
                } else {
                    (format!("{:.0} {}", current_value, metric.unit), RethTheme::TEXT_PRIMARY)
                };
                
                ui.label(egui::RichText::new(&value_text)
                    .size(18.0)
                    .color(value_color));
            });
            
            ui.add_space(4.0);
            
            // Frame containing only the graph
            egui::Frame::none()
                .fill(bg_color)
                .rounding(8.0)
                .inner_margin(0.0)
                .stroke(egui::Stroke::new(1.5, border_color))
                .show(ui, |ui| {
                    ui.set_min_size(egui::Vec2::new(280.0, 140.0));
                    self.draw_large_graph(ui, metric);
                });
        });
    }
    
    fn draw_large_graph(&self, ui: &mut egui::Ui, metric: &metrics::MetricHistory) {
        // Use egui_plot for large graph with full axis labels
        let plot_points: PlotPoints = if metric.values.is_empty() {
            PlotPoints::new(vec![[0.0, 0.0]])
        } else {
            // Convert metric values to plot points with time on x-axis
            let points: Vec<[f64; 2]> = metric.values
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    // Use seconds ago for x-axis
                    let seconds_ago = (metric.values.len() - 1 - i) as f64;
                    [-seconds_ago, value.value]
                })
                .collect();
            PlotPoints::new(points)
        };
        
        // Configure the plot line
        let line = Line::new(plot_points)
            .color(RethTheme::PRIMARY)
            .style(egui_plot::LineStyle::Solid)
            .width(2.5)
            .fill(0.0); // Fill to y=0
        
        // Clone the unit to avoid lifetime issues
        let unit = metric.unit.clone();
        let unit_for_formatter = unit.clone();
        let unit_for_hover = unit.clone();
        
        // Create a larger plot with full labels
        let plot = Plot::new(format!("large_metric_plot_{}", metric.name))
            .auto_bounds(egui::Vec2b::new(true, true))
            .show_axes([true, true])
            .show_grid([false, false]) // Only show axes, no grid
            .include_y(0.0) // Always show y=0
            .show_background(false)
            .allow_zoom(false)
            .allow_drag(false)
            .allow_boxed_zoom(false)
            .allow_scroll(false)
            .label_formatter(move |_name, value| {
                // Detailed hover information
                let time_label = if value.x == 0.0 {
                    "Now".to_string()
                } else {
                    format!("{}s ago", -value.x as i64)
                };
                
                let value_label = match unit_for_hover.as_str() {
                    "%" => format!("{:.1}%", value.y),
                    "MB" => format!("{:.1} MB", value.y),
                    "peers" => format!("{:.0} peers", value.y),
                    "blocks" => {
                        if value.y >= 1_000_000.0 {
                            format!("{:.2}M blocks", value.y / 1_000_000.0)
                        } else if value.y >= 1000.0 {
                            format!("{:.1}k blocks", value.y / 1000.0)
                        } else {
                            format!("{:.0} blocks", value.y)
                        }
                    },
                    "txs" => format!("{:.0} txs", value.y),
                    "gwei" => format!("{:.2} gwei", value.y),
                    _ => format!("{:.2} {}", value.y, unit_for_hover),
                };
                
                format!("{}\n{}", time_label, value_label)
            })
            .x_axis_formatter(|value, _max_chars, _range| {
                // Show time labels on x-axis
                if value == 0.0 {
                    "Now".to_string()
                } else if value == -60.0 {
                    "60s".to_string()
                } else if value == -30.0 {
                    "30s".to_string()
                } else {
                    String::new()
                }
            })
            .y_axis_formatter(move |value, _max_chars, _range| {
                // Format y-axis based on metric type
                match unit_for_formatter.as_str() {
                    "%" => format!("{:.0}%", value),
                    "MB" => format!("{:.0}", value),
                    "blocks" => {
                        if value >= 1_000_000.0 {
                            format!("{:.1}M", value / 1_000_000.0)
                        } else if value >= 1000.0 {
                            format!("{:.0}k", value / 1000.0)
                        } else {
                            format!("{:.0}", value)
                        }
                    },
                    _ => format!("{:.0}", value),
                }
            })
            .x_grid_spacer(|_grid_input| {
                // Custom grid spacing for x-axis
                vec![
                    egui_plot::GridMark { value: 0.0, step_size: 15.0 },
                    egui_plot::GridMark { value: -15.0, step_size: 15.0 },
                    egui_plot::GridMark { value: -30.0, step_size: 15.0 },
                    egui_plot::GridMark { value: -45.0, step_size: 15.0 },
                    egui_plot::GridMark { value: -60.0, step_size: 15.0 },
                ]
            });
        
        // Show the plot
        plot.show(ui, |plot_ui| {
            plot_ui.line(line);
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
        
        // Auto-start terminal if we detected an existing Reth process
        if self.detected_existing_process && !matches!(self.install_status, InstallStatus::Running) {
            self.install_status = InstallStatus::Running;
            self.detected_existing_process = false; // Only do this once
        }
        
        // Process incoming metrics
        while let Ok(metrics_text) = self.metrics_receiver.try_recv() {
            // Update available metrics list
            self.available_metrics = metrics::RethMetrics::get_available_metrics(&metrics_text);
            
            self.metrics.update_from_prometheus_text(&metrics_text);
            self.metrics.mark_polled();
        }
        
        // Update Reth node status and collect logs
        if matches!(self.install_status, InstallStatus::Running) {
            self.reth_node.check_process_status();
            let new_logs = self.reth_node.get_logs();
            if !new_logs.is_empty() {
                println!("Got {} new log lines", new_logs.len());
            }
            self.node_logs.extend(new_logs);
            
            // Keep only last 1000 logs for performance
            if self.node_logs.len() > 1000 {
                self.node_logs.drain(0..self.node_logs.len() - 1000);
            }
            
            // Periodically log the current state for debugging
            let now = std::time::Instant::now();
            if now.duration_since(self.last_debug_log).as_secs() >= 5 {
                self.last_debug_log = now;
                println!("Current state - Total logs: {}, Is external: {}, Log path: {:?}", 
                    self.node_logs.len(), 
                    self.reth_node.is_monitoring_external(),
                    self.reth_node.get_external_log_path()
                );
            }
            
            if !self.reth_node.is_running() {
                // If we were monitoring an external process, go back to Completed
                // If we were running our own process, mark as Stopped
                if self.reth_node.get_external_log_path().is_some() {
                    println!("External Reth process stopped, returning to main interface");
                    self.install_status = InstallStatus::Completed;
                } else {
                    println!("Managed Reth process stopped");
                    self.install_status = InstallStatus::Stopped;
                }
            }
        }

        // Top menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Settings", |ui| {
                    if ui.button("Reth Desktop Config").clicked() {
                        self.show_desktop_settings = true;
                        ui.close_menu();
                    }
                    if ui.button("Node Configuration").clicked() {
                        self.show_settings = true;
                        self.reset_editable_config(); // Reset to current saved state when opening
                        ui.close_menu();
                    }
                    if ui.button("Start Config").clicked() {
                        self.show_start_config = true;
                        // Load CLI options if they're not already loaded
                        if self.available_cli_options.is_empty() && self.is_reth_installed {
                            let reth_path = dirs::home_dir()
                                .unwrap_or_default()
                                .join(".reth-desktop")
                                .join("bin")
                                .join("reth");
                            self.available_cli_options = RethNode::get_available_cli_options(&reth_path.to_string_lossy());
                        }
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
                    ui.spacing_mut().item_spacing.x = 4.0; // Consistent spacing between elements
                    
                    // "Open Source" link
                    let open_source_link = egui::RichText::new("Open Source")
                        .size(12.0)
                        .color(RethTheme::PRIMARY);
                    
                    if ui.link(open_source_link).clicked() {
                        let _ = std::process::Command::new("open")
                            .arg("https://github.com/bford21/reth-desktop")
                            .spawn();
                    }
                    
                    ui.label(egui::RichText::new("and made with").size(12.0).color(RethTheme::TEXT_SECONDARY));
                    ui.label(egui::RichText::new("❤").size(12.0).color(RethTheme::TEXT_SECONDARY)); // Clean heart emoji without extra characters
                    ui.label(egui::RichText::new("by").size(12.0).color(RethTheme::TEXT_SECONDARY));
                    
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
        
        // Desktop Settings window
        if self.show_desktop_settings {
            let mut open = true;
            egui::Window::new("Reth Desktop Configuration")
                .resizable(true)
                .default_width(400.0)
                .default_height(200.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    DesktopSettingsWindow::show_content(ui, &mut self.desktop_settings);
                });
            if !open {
                self.show_desktop_settings = false;
            }
        }
        
        // Node Settings window
        if self.show_settings {
            let mut open = true;
            egui::Window::new("Reth Node Configuration")
                .resizable(true)
                .default_width(600.0)
                .default_height(500.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    NodeSettingsWindow::show_content(
                        ui,
                        &self.reth_config,
                        &self.reth_config_path,
                        &mut self.editable_config,
                        &mut self.config_modified,
                        &mut self.settings_edit_mode,
                    );
                });
            if !open {
                self.show_settings = false;
            }
        }
        
        // Start Config window
        if self.show_start_config {
            let mut open = true;
            let mut restart_requested = false;
            egui::Window::new("Start Configuration")
                .resizable(true)
                .default_width(1200.0)
                .default_height(800.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    restart_requested = StartConfigWindow::show_content(
                        ui,
                        &self.reth_node,
                        &mut self.desktop_settings,
                        &self.available_cli_options,
                        &mut self.selected_cli_option,
                        &mut self.parameter_value,
                        &mut self.selected_values,
                        &mut self.pending_launch_args,
                    );
                });
            if !open {
                self.show_start_config = false;
            }
            
            // Handle restart request
            if restart_requested {
                if self.reth_node.is_running() {
                    // Stop the current node
                    if let Err(e) = self.reth_node.stop() {
                        eprintln!("Failed to stop Reth node: {}", e);
                    } else {
                        self.install_status = InstallStatus::Stopped;
                        self.stop_metrics_polling();
                        
                        // Start the node again with new parameters
                        let reth_path = dirs::home_dir()
                            .unwrap_or_default()
                            .join(".reth-desktop")
                            .join("bin")
                            .join("reth");
                        
                        match self.reth_node.start(&reth_path.to_string_lossy(), &self.pending_launch_args, &self.desktop_settings) {
                            Ok(()) => {
                                self.install_status = InstallStatus::Running;
                                self.pending_launch_args.clear();
                                self.start_metrics_polling();
                            }
                            Err(e) => {
                                self.install_status = InstallStatus::Error(format!("Failed to restart Reth: {}", e));
                            }
                        }
                    }
                }
            }
        }
        
        // Add Parameter window
        if self.show_add_parameter {
            let mut open = true;
            let mut should_add = false;
            let mut cancel_clicked = false;
            
            egui::Window::new("Add Launch Parameter")
                .resizable(false)
                .default_width(600.0)
                .default_height(500.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.label("Select a parameter to add:");
                        ui.add_space(8.0);
                        
                        // ComboBox for parameter selection
                        egui::ComboBox::from_label("Parameter")
                            .width(550.0)
                            .selected_text(
                                self.selected_cli_option
                                    .and_then(|i| self.available_cli_options.get(i))
                                    .map(|opt| opt.name.as_str())
                                    .unwrap_or("Select...")
                            )
                            .show_ui(ui, |ui| {
                                ui.set_min_width(550.0);
                                ui.set_min_height(300.0);
                                for (i, option) in self.available_cli_options.iter().enumerate() {
                                    // Make the entire line clickable
                                    let selected = self.selected_cli_option == Some(i);
                                    
                                    // Create a clickable area that covers the entire parameter info
                                    let response = ui.allocate_response(
                                        egui::Vec2::new(ui.available_width(), 35.0),
                                        egui::Sense::click()
                                    );
                                    
                                    // Handle selection
                                    if response.clicked() {
                                        self.selected_cli_option = Some(i);
                                        self.parameter_value.clear();
                                        self.selected_values.clear();
                                    }
                                    
                                    // Draw background if selected
                                    if selected {
                                        ui.painter().rect_filled(response.rect, 2.0, egui::Color32::from_rgb(70, 130, 180).linear_multiply(0.2));
                                    }
                                    
                                    // Draw parameter name and description
                                    ui.allocate_ui_at_rect(response.rect, |ui| {
                                        ui.vertical(|ui| {
                                            ui.add_space(4.0);
                                            ui.label(egui::RichText::new(&option.name).strong());
                                            
                                            // Description with indentation
                                            ui.horizontal(|ui| {
                                                ui.add_space(16.0); // Indent
                                                ui.label(egui::RichText::new(&option.description)
                                                    .size(10.0)
                                                    .color(egui::Color32::GRAY));
                                            });
                                        });
                                    });
                                    ui.add_space(4.0);
                                }
                            });
                        
                        ui.add_space(8.0);
                        
                        // Show value input if parameter takes a value
                        if let Some(selected) = self.selected_cli_option {
                            if let Some(option) = self.available_cli_options.get(selected) {
                                if option.takes_value {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label("Value:");
                                            if let Some(value_name) = &option.value_name {
                                                ui.label(RethTheme::muted_text(&format!("({})", value_name)));
                                            }
                                            if option.accepts_multiple {
                                                ui.label(RethTheme::muted_text("(comma-separated)"));
                                            }
                                        });
                                        
                                        // Show different UI based on whether it has possible values and accepts multiple
                                        if let Some(possible_values) = &option.possible_values {
                                            if !possible_values.is_empty() {
                                                if option.accepts_multiple {
                                                    // Multi-select checkboxes for comma-separated values
                                                    ui.label("Select values:");
                                                    ui.separator();
                                                    
                                                    for value in possible_values {
                                                        let mut selected = self.selected_values.contains(value);
                                                        if ui.checkbox(&mut selected, value).changed() {
                                                            if selected {
                                                                if !self.selected_values.contains(value) {
                                                                    self.selected_values.push(value.clone());
                                                                }
                                                            } else {
                                                                self.selected_values.retain(|v| v != value);
                                                            }
                                                            // Update parameter_value to be comma-separated
                                                            self.parameter_value = self.selected_values.join(",");
                                                        }
                                                    }
                                                    
                                                    if !self.selected_values.is_empty() {
                                                        ui.add_space(4.0);
                                                        ui.label(RethTheme::muted_text(&format!("Selected: {}", self.parameter_value)));
                                                    }
                                                } else {
                                                    // Single-select ComboBox
                                                    egui::ComboBox::from_id_source(format!("value_combo_{}", selected))
                                                        .width(200.0)
                                                        .selected_text(
                                                            if self.parameter_value.is_empty() {
                                                                "Select..."
                                                            } else {
                                                                &self.parameter_value
                                                            }
                                                        )
                                                        .show_ui(ui, |ui| {
                                                            for value in possible_values {
                                                                ui.selectable_value(&mut self.parameter_value, value.clone(), value);
                                                            }
                                                        });
                                                }
                                            } else {
                                                ui.text_edit_singleline(&mut self.parameter_value);
                                            }
                                        } else {
                                            ui.text_edit_singleline(&mut self.parameter_value);
                                        }
                                    });
                                    
                                    if self.parameter_value.trim().is_empty() && (!option.accepts_multiple || self.selected_values.is_empty()) {
                                        ui.label(RethTheme::warning_text("⚠ This parameter requires a value"));
                                    }
                                    
                                    ui.add_space(8.0);
                                }
                            }
                        }
                        
                        ui.add_space(16.0);
                        
                        ui.horizontal(|ui| {
                            let can_add = if let Some(selected) = self.selected_cli_option {
                                if let Some(option) = self.available_cli_options.get(selected) {
                                    // Can add if it's a flag OR if it requires a value and we have one
                                    !option.takes_value || 
                                    (!self.parameter_value.trim().is_empty() || 
                                     (option.accepts_multiple && !self.selected_values.is_empty()))
                                } else {
                                    false
                                }
                            } else {
                                false
                            };
                            
                            if ui.add_enabled(can_add, egui::Button::new("Add")).clicked() {
                                if let Some(selected) = self.selected_cli_option {
                                    if let Some(option) = self.available_cli_options.get(selected) {
                                        // Add the parameter
                                        if option.takes_value {
                                            if !self.parameter_value.is_empty() {
                                                self.desktop_settings.custom_launch_args.push(option.name.clone());
                                                self.desktop_settings.custom_launch_args.push(self.parameter_value.clone());
                                                // Also add to pending list for immediate display
                                                self.pending_launch_args.push(option.name.clone());
                                                self.pending_launch_args.push(self.parameter_value.clone());
                                            }
                                        } else {
                                            // Flag parameter - just add the name
                                            self.desktop_settings.custom_launch_args.push(option.name.clone());
                                            // Also add to pending list for immediate display
                                            self.pending_launch_args.push(option.name.clone());
                                        }
                                        
                                        // Save settings
                                        if let Err(e) = DesktopSettingsManager::save_desktop_settings(&self.desktop_settings) {
                                            eprintln!("Failed to save desktop settings: {}", e);
                                        }
                                        
                                        should_add = true;
                                    }
                                }
                            }
                            
                            if ui.button("Cancel").clicked() {
                                cancel_clicked = true;
                            }
                        });
                    });
                });
            
            if should_add || cancel_clicked || !open {
                self.show_add_parameter = false;
                self.selected_cli_option = None;
                self.parameter_value.clear();
                self.selected_values.clear();
            }
        }
        
        // Metric selector window
        if self.show_metric_selector {
            let mut open = true;
            let mut selected_metric: Option<String> = None;
            
            // Fetch available metrics if we haven't already
            if self.available_metrics.is_empty() {
                let metrics_endpoint = format!("http://{}/debug/metrics/prometheus", self.desktop_settings.reth_defaults.metrics_address);
                if let Ok(metrics_text) = std::process::Command::new("curl")
                    .arg("-s")
                    .arg(&metrics_endpoint)
                    .output()
                {
                    if metrics_text.status.success() {
                        if let Ok(text) = String::from_utf8(metrics_text.stdout) {
                            self.available_metrics = metrics::RethMetrics::get_available_metrics(&text);
                        }
                    }
                }
            }
            
            egui::Window::new("Select Metric to Add")
                .resizable(true)
                .default_width(600.0)
                .default_height(500.0)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.label("Select a metric from the list below:");
                    ui.separator();
                    
                    // Search filter using context data storage
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        let mut search_text = ui.ctx().data_mut(|d| 
                            d.get_temp::<String>(egui::Id::new("metric_search_text"))
                                .unwrap_or_default()
                        );
                        if ui.text_edit_singleline(&mut search_text).changed() {
                            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("metric_search_text"), search_text.clone()));
                        }
                    });
                    
                    ui.separator();
                    
                    let search_text = ui.ctx().data(|d| 
                        d.get_temp::<String>(egui::Id::new("metric_search_text"))
                            .unwrap_or_default()
                    );
                    
                    // Scrollable list of metrics
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for metric_name in &self.available_metrics {
                                // Filter by search text
                                if !search_text.is_empty() && !metric_name.to_lowercase().contains(&search_text.to_lowercase()) {
                                    continue;
                                }
                                
                                // Skip metrics we already have
                                if self.desktop_settings.custom_metrics.contains(metric_name) {
                                    continue;
                                }
                                
                                // Skip default metrics
                                if metric_name == "reth_network_connected_peers" ||
                                   metric_name == "reth_blockchain_tree_canonical_chain_height" ||
                                   metric_name == "reth_sync_execution_gas_per_second" ||
                                   metric_name == "reth_process_resident_memory_bytes" ||
                                   metric_name == "reth_consensus_engine_beacon_active_block_downloads" ||
                                   metric_name == "reth_transaction_pool_transactions" {
                                    continue;
                                }
                                
                                if ui.selectable_label(false, metric_name).clicked() {
                                    selected_metric = Some(metric_name.clone());
                                }
                            }
                        });
                });
                
            if !open {
                self.show_metric_selector = false;
                ctx.data_mut(|d| d.remove::<String>(egui::Id::new("metric_search_text")));
            }
            
            // Add the selected metric
            if let Some(metric_name) = selected_metric {
                self.desktop_settings.custom_metrics.push(metric_name.clone());
                self.metrics.add_custom_metric(metric_name);
                
                // Save settings
                if let Err(e) = DesktopSettingsManager::save_desktop_settings(&self.desktop_settings) {
                    eprintln!("Failed to save custom metrics: {}", e);
                }
                
                self.show_metric_selector = false;
                ctx.data_mut(|d| d.remove::<String>(egui::Id::new("metric_search_text")));
            }
        }
        
        // Metric popup window
        if let Some(metric_name) = &self.expanded_metric.clone() {
            let mut open = true;
            
            // Check default metrics first
            let metric = match metric_name.as_str() {
                "Connected Peers" => Some(&self.metrics.peers_connected),
                "Block Height" => Some(&self.metrics.block_height),
                "Sync Progress" => Some(&self.metrics.sync_progress),
                "Memory Usage" => Some(&self.metrics.memory_usage),
                "Active Downloads" => Some(&self.metrics.disk_io),
                _ => {
                    // Check custom metrics
                    self.metrics.custom_metrics.get(metric_name)
                }
            };
            
            if let Some(metric) = metric {
                let display_name = if metric_name.contains('_') {
                    // For custom metrics, display a nicer name
                    metric_name.replace('_', " ")
                        .split_whitespace()
                        .map(|word| {
                            let mut chars = word.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    metric_name.clone()
                };
                
                egui::Window::new(&format!("{} - Full History", display_name))
                    .resizable(true)
                    .default_width(900.0)
                    .default_height(600.0)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        // Show current value
                        if let Some(current) = metric.get_latest() {
                            let unit_display = if metric.unit == "bytes" {
                                "MB"
                            } else {
                                &metric.unit
                            };
                            ui.heading(format!("Current: {:.2} {}", current, unit_display));
                        }
                        ui.separator();
                        
                        // Show the full graph
                        ui.vertical(|ui| {
                            ui.set_height(500.0);
                            self.draw_metric_graph(ui, metric);
                        });
                        
                        ui.separator();
                        ui.label(format!("Showing {} data points (up to 10 minutes)", metric.values.len()));
                    });
                    
                if !open {
                    self.expanded_metric = None;
                }
            } else {
                self.expanded_metric = None;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.add_space(20.0);
                    
                    // Modern header with logo and status indicator
                    ui.horizontal(|ui| {
                        // Add space before logo
                        ui.add_space(20.0);
                        
                        // Logo on the left
                        if let Some(logo_texture) = &self.reth_logo {
                            let logo_size = logo_texture.size_vec2();
                            // Scale the logo to a reasonable size (max 50px height for header)
                            let scale = (50.0 / logo_size.y).min(1.0);
                            let display_size = logo_size * scale;
                            
                            ui.add(egui::Image::new(logo_texture).max_size(display_size));
                            ui.add_space(16.0);
                        }
                        
                        // Header text
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Ethereum Execution Client")
                                .size(24.0)
                                .color(RethTheme::TEXT_PRIMARY)
                                .strong());
                            ui.label(egui::RichText::new("Fast, lightweight desktop client")
                                .size(14.0)
                                .color(RethTheme::TEXT_SECONDARY));
                        });
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Add space to the right of the status section
                            ui.add_space(20.0);
                            
                            // Status indicator and controls
                            if self.reth_node.is_running() {
                                if ui.add(egui::Button::new(egui::RichText::new("Stop")
                                    .color(egui::Color32::WHITE))
                                    .fill(RethTheme::ERROR)
                                    .rounding(6.0)
                                    .min_size(egui::Vec2::new(60.0, 32.0)))
                                    .clicked() {
                                    self.stop_reth();
                                }
                                
                                ui.add_space(12.0);
                                
                                ui.horizontal(|ui| {
                                    ui.add(egui::widgets::Spinner::new().size(12.0).color(RethTheme::SUCCESS));
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new("Node Running")
                                        .size(14.0)
                                        .color(RethTheme::SUCCESS)
                                        .strong());
                                });
                            } else {
                                if ui.add(egui::Button::new(egui::RichText::new("Start")
                                    .color(egui::Color32::WHITE))
                                    .fill(RethTheme::SUCCESS)
                                    .rounding(6.0)
                                    .min_size(egui::Vec2::new(60.0, 32.0)))
                                    .clicked() {
                                    let reth_path = dirs::home_dir()
                                        .unwrap_or_default()
                                        .join(".reth-desktop")
                                        .join("bin")
                                        .join("reth");
                                    match self.reth_node.start(&reth_path.to_string_lossy(), &self.pending_launch_args, &self.desktop_settings) {
                                        Ok(()) => {
                                            self.install_status = InstallStatus::Running;
                                            // Clear pending args since they've been applied
                                            self.pending_launch_args.clear();
                                            
                                            // Start metrics polling
                                            self.start_metrics_polling();
                                        }
                                        Err(e) => {
                                            self.install_status = InstallStatus::Error(format!("Failed to launch Reth: {}", e));
                                            println!("Failed to start Reth: {}", e);
                                        }
                                    }
                                }
                                
                                ui.add_space(12.0);
                                
                                ui.label(egui::RichText::new("Node Stopped")
                                    .size(14.0)
                                    .color(RethTheme::TEXT_SECONDARY));
                            }
                        });
                    });
                    
                    ui.add_space(20.0);
                    
                    // Horizontal line separator below header
                    ui.separator();
                    
                    ui.add_space(30.0);
            
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
                    InstallStatus::Running => {
                        // Show metrics section
                        ui.set_max_width(max_width);
                        self.show_metrics_section(ui);
                        
                        ui.add_space(12.0);
                        
                        // Command Terminal section matching mockup
                        ui.add_space(20.0);
                        
                        // Terminal output matching mockup style
                        let _available_rect = ui.available_rect_before_wrap();
                        let terminal_height = 300.0; // Increased height for better visibility
                        
                        egui::Frame::none()
                            .fill(RethTheme::SURFACE)
                            .rounding(8.0)
                            .inner_margin(16.0)
                            .stroke(egui::Stroke::new(1.0, RethTheme::BORDER))
                            .show(ui, |ui| {
                                        ui.set_min_height(terminal_height);
                                        // Add both vertical and horizontal scroll areas
                                        egui::ScrollArea::both()
                                            .max_height(terminal_height)
                                            .auto_shrink([false; 2])
                                            .stick_to_bottom(true)
                                            .show(ui, |ui| {
                                                // Use a vertical layout with left alignment
                                                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                                                    // Show recent log lines or sample data if no logs
                                                    if self.node_logs.is_empty() {
                                                        // Show sample terminal output like in mockup
                                                        let sample_logs = vec![
                                                            "13:31:05 INFO Status connected_peers=4 latest_block=4",
                                                            "13:31:10 INFO Status connected_peers=4 latest_block=4", 
                                                            "13:31:15 INFO Status connected_peers=4 latest_block=4",
                                                            "13:31:20 INFO Very long log line that demonstrates horizontal scrolling capability when terminal output exceeds the visible width of the terminal window area and maintains proper left alignment"
                                                        ];
                                                        
                                                        for log in sample_logs {
                                                            // Disable wrapping for each line
                                                            ui.style_mut().wrap = Some(false);
                                                            ui.label(egui::RichText::new(log)
                                                                .size(12.0)
                                                                .color(egui::Color32::from_rgb(255, 193, 7)) // Orange like in mockup
                                                                .monospace());
                                                        }
                                                    } else {
                                                        // Show actual log lines - clean and left-aligned
                                                        let logs_to_show: Vec<_> = self.node_logs.iter().rev().take(40).collect();
                                                        
                                                        for log_line in logs_to_show.into_iter().rev() {
                                                            // Clean the log content to remove ANSI escape codes
                                                            let cleaned_content = Self::clean_log_content(&log_line.content);
                                                            
                                                            // Format: timestamp + cleaned content
                                                            let formatted_line = format!("{} {}", 
                                                                log_line.timestamp.split(' ').next().unwrap_or(""),
                                                                cleaned_content
                                                            );
                                                            
                                                            let color = match log_line.level {
                                                                LogLevel::Error => egui::Color32::from_rgb(255, 100, 100),
                                                                LogLevel::Warn => egui::Color32::from_rgb(255, 200, 100),
                                                                LogLevel::Info => egui::Color32::from_rgb(255, 193, 7), // Orange like mockup
                                                                LogLevel::Debug => egui::Color32::from_rgb(150, 150, 255),
                                                                LogLevel::Trace => egui::Color32::GRAY,
                                                            };
                                                            
                                                            // Disable wrapping for each line
                                                            ui.style_mut().wrap = Some(false);
                                                            ui.label(egui::RichText::new(&formatted_line)
                                                                .size(12.0)
                                                                .color(color)
                                                                .monospace());
                                                        }
                                                    }
                                                });
                                            });
                                    });
                        
                        // Auto-refresh for live updates
                        ctx.request_repaint_after(std::time::Duration::from_millis(500));
                    }
                    InstallStatus::Completed => {
                        // Reth is installed and ready - no UI needed, use header controls
                    }
                    InstallStatus::Stopped => {
                        // Reth is stopped - no UI needed, use header controls  
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
        });
        });
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Handle application shutdown based on settings
        if self.reth_node.is_running() {
            if self.desktop_settings.keep_reth_running_in_background {
                println!("Keeping Reth running in background (setting enabled)");
                // Don't stop the process - let it continue running
            } else {
                println!("Stopping Reth on application exit (setting disabled)");
                if let Err(e) = self.reth_node.stop() {
                    eprintln!("Error stopping Reth on exit: {}", e);
                }
            }
        }
        
        // Save desktop settings before closing
        if let Err(e) = DesktopSettingsManager::save_desktop_settings(&self.desktop_settings) {
            eprintln!("Failed to save desktop settings on exit: {}", e);
        }
    }
}