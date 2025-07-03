use eframe::egui;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

mod installer;
mod system_check;

use installer::{RethInstaller, InstallStatus};
use system_check::SystemRequirements;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
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
    system_requirements: SystemRequirements,
}

enum InstallCommand {
    StartInstall(Arc<Mutex<RethInstaller>>, egui::Context),
    ResetInstaller(Arc<Mutex<RethInstaller>>),
}

impl MyApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = tokio::runtime::Runtime::new().expect("Unable to create Runtime");
        let (tx, mut rx) = mpsc::unbounded_channel::<InstallCommand>();
        
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
        
        Self {
            installer: Arc::new(Mutex::new(RethInstaller::new())),
            install_status: InstallStatus::Idle,
            installing: false,
            _runtime: runtime,
            install_sender: tx,
            system_requirements: SystemRequirements::check(),
        }
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
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update status from installer using try_lock
        if let Ok(installer) = self.installer.try_lock() {
            self.install_status = installer.status().clone();
            if matches!(self.install_status, InstallStatus::Completed | InstallStatus::Error(_)) {
                self.installing = false;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Reth Desktop Installer");
            
            ui.separator();
            
            ui.label("Install Reth Ethereum execution client");
            
            ui.add_space(20.0);
            
            // System Requirements Section
            ui.group(|ui| {
                ui.label(egui::RichText::new("MinimumSystem Requirements").strong());
                ui.add_space(10.0);
                
                // Disk Space Requirement
                ui.horizontal(|ui| {
                    if self.system_requirements.disk_space.meets_requirement {
                        ui.colored_label(egui::Color32::GREEN, "✓");
                    } else {
                        ui.colored_label(egui::Color32::RED, "✗");
                    }
                    ui.label(format!(
                        "Disk Space: {:.1} GB available / {:.0} GB required",
                        self.system_requirements.disk_space.available_gb,
                        self.system_requirements.disk_space.required_gb
                    ));
                });
                
                // Memory Requirement
                ui.horizontal(|ui| {
                    if self.system_requirements.memory.meets_requirement {
                        ui.colored_label(egui::Color32::GREEN, "✓");
                    } else {
                        ui.colored_label(egui::Color32::RED, "✗");
                    }
                    ui.label(format!(
                        "Memory: {:.1} GB total / {:.0} GB required",
                        self.system_requirements.memory.total_gb,
                        self.system_requirements.memory.required_gb
                    ));
                });
            });
            
            ui.add_space(20.0);
            
            // Show warning if requirements not met
            if !self.system_requirements.all_requirements_met() {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 165, 0), // Orange
                    "⚠ Warning: Your system does not meet all requirements. Installation may fail or Reth may not run properly."
                );
                ui.add_space(10.0);
            }
            
            match &self.install_status {
                InstallStatus::Idle => {
                    if ui.button("Install Reth").clicked() && !self.installing {
                        self.start_installation(ctx.clone());
                    }
                }
                InstallStatus::Downloading(progress) => {
                    ui.label(format!("Downloading... {:.1}%", progress));
                    ui.add(egui::ProgressBar::new(progress / 100.0));
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                InstallStatus::Extracting => {
                    ui.label("Extracting files...");
                    ui.spinner();
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                }
                InstallStatus::Completed => {
                    ui.colored_label(egui::Color32::GREEN, "✓ Installation completed!");
                    ui.label("Reth has been installed to ~/.reth-desktop/bin/");
                    
                    if ui.button("Install Again").clicked() {
                        self.install_status = InstallStatus::Idle;
                        self.reset_installer();
                    }
                }
                InstallStatus::Error(error) => {
                    ui.colored_label(egui::Color32::RED, format!("❌ Error: {}", error));
                    
                    if ui.button("Try Again").clicked() {
                        self.install_status = InstallStatus::Idle;
                        self.reset_installer();
                    }
                }
            }
            
            ui.add_space(20.0);
            
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.label("Platform:");
                ui.label(std::env::consts::OS);
                ui.label(std::env::consts::ARCH);
            });
        });
    }
}