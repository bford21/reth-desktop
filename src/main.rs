use eframe::egui;
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

use installer::{RethInstaller, InstallStatus};
use system_check::SystemRequirements;
use theme::RethTheme;
use reth_node::{RethNode, LogLine, LogLevel};
use config::{RethConfig, RethConfigManager};
use settings::{DesktopSettings, DesktopSettingsManager};
use ui::{DesktopSettingsWindow, NodeSettingsWindow};


fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Reth Desktop"),
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
    detected_existing_process: bool,
    installed_version: Option<String>,
    latest_version: Option<String>,
    update_available: bool,
    show_settings: bool,
    show_desktop_settings: bool,
    desktop_settings: DesktopSettings,
    reth_config: RethConfig,
    reth_config_path: Option<std::path::PathBuf>,
    editable_config: RethConfig,
    config_modified: bool,
    settings_edit_mode: bool,
    last_debug_log: std::time::Instant,
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
        let (reth_config, reth_config_path) = RethConfigManager::load_reth_config();
        
        // Load desktop settings
        let desktop_settings = DesktopSettingsManager::load_desktop_settings();
        
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
        
        let mut app = Self {
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
            desktop_settings,
            reth_config: reth_config.clone(),
            reth_config_path,
            editable_config: reth_config,
            config_modified: false,
            settings_edit_mode: false,
            last_debug_log: std::time::Instant::now()
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
    
    
    fn reset_editable_config(&mut self) {
        self.editable_config = self.reth_config.clone();
        self.config_modified = false;
        // Don't reset edit mode here - let the caller decide
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
                                    // Show different status based on whether we own the process
                                    if self.reth_node.is_monitoring_external() {
                                        ui.label(RethTheme::success_text("🟢 Monitoring External Reth Process"));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let disconnect_button = egui::Button::new(RethTheme::warning_text("Disconnect"))
                                                .min_size(egui::vec2(80.0, 24.0));
                                            
                                            if ui.add(disconnect_button).clicked() {
                                                self.install_status = InstallStatus::Completed;
                                                // We need a method to disconnect from external process
                                                self.disconnect_from_external_reth();
                                            }
                                        });
                                    } else {
                                        ui.label(RethTheme::success_text("🟢 Reth Node Running"));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let stop_button = egui::Button::new(RethTheme::error_text("Stop"))
                                                .min_size(egui::vec2(60.0, 24.0));
                                            
                                            if ui.add(stop_button).clicked() {
                                                self.stop_reth();
                                            }
                                        });
                                    }
                                });
                                
                                // Show log file path if we have one (for both external and managed processes)
                                if let Some(log_path) = self.reth_node.get_external_log_path() {
                                    ui.add_space(4.0);
                                    if self.reth_node.is_monitoring_external() {
                                        ui.label(RethTheme::muted_text(&format!("📄 Tailing log file: {}", log_path.display())));
                                    } else {
                                        ui.label(RethTheme::muted_text(&format!("📄 Logging to: {}", log_path.display())));
                                    }
                                }
                                
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
                                                    if self.reth_node.is_monitoring_external() {
                                                        if self.reth_node.get_external_log_path().is_some() {
                                                            ui.label(egui::RichText::new("Monitoring external Reth process...")
                                                                .size(12.0)
                                                                .color(egui::Color32::LIGHT_BLUE)
                                                                .monospace());
                                                            ui.label(egui::RichText::new("Tailing log file for real-time output...")
                                                                .size(11.0)
                                                                .color(egui::Color32::LIGHT_GREEN)
                                                                .monospace());
                                                            ui.label(egui::RichText::new("Logs will appear here as they are generated.")
                                                                .size(11.0)
                                                                .color(egui::Color32::GRAY)
                                                                .monospace());
                                                        } else {
                                                            ui.label(egui::RichText::new("Monitoring external Reth process...")
                                                                .size(12.0)
                                                                .color(egui::Color32::LIGHT_BLUE)
                                                                .monospace());
                                                            ui.label(egui::RichText::new("⚠ No log files found")
                                                                .size(11.0)
                                                                .color(egui::Color32::YELLOW)
                                                                .monospace());
                                                            ui.label(egui::RichText::new("Reth may not be configured for file logging.")
                                                                .size(10.0)
                                                                .color(egui::Color32::GRAY)
                                                                .monospace());
                                                            ui.label(egui::RichText::new("To enable: restart Reth with --log.file.directory <path>")
                                                                .size(10.0)
                                                                .color(egui::Color32::GRAY)
                                                                .monospace());
                                                            ui.add_space(8.0);
                                                            if ui.button("Disconnect and Start Managed Reth").clicked() {
                                                                self.disconnect_from_external_reth();
                                                                self.install_status = InstallStatus::Completed;
                                                            }
                                                        }
                                                    } else {
                                                        ui.label(egui::RichText::new("Starting Reth node...")
                                                            .size(12.0)
                                                            .color(egui::Color32::GRAY)
                                                            .monospace());
                                                    }
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