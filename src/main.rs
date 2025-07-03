use eframe::egui;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

mod installer;
mod system_check;
mod theme;

use installer::{RethInstaller, InstallStatus};
use system_check::SystemRequirements;
use theme::RethTheme;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([600.0, 500.0])
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
    system_requirements: SystemRequirements,
    reth_logo: Option<egui::TextureHandle>,
}

enum InstallCommand {
    StartInstall(Arc<Mutex<RethInstaller>>, egui::Context),
    ResetInstaller(Arc<Mutex<RethInstaller>>),
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let runtime = tokio::runtime::Runtime::new().expect("Unable to create Runtime");
        let (tx, mut rx) = mpsc::unbounded_channel::<InstallCommand>();
        
        // Load the Reth logo
        let reth_logo = Self::load_logo(&cc.egui_ctx);
        
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
            reth_logo,
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
        // Apply custom theme
        RethTheme::apply(ctx);
        
        // Update status from installer using try_lock
        if let Ok(installer) = self.installer.try_lock() {
            self.install_status = installer.status().clone();
            if matches!(self.install_status, InstallStatus::Completed | InstallStatus::Error(_)) {
                self.installing = false;
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
                let max_width = 600.0;
                
                // System Requirements Card
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
                
                // Installation section
                match &self.install_status {
                    InstallStatus::Idle => {
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
                        });
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
                                    ui.label(RethTheme::success_text("✓ Installation Completed!"));
                                    ui.add_space(8.0);
                                    ui.label(RethTheme::muted_text("Reth has been installed to ~/.reth-desktop/bin/"));
                                    ui.add_space(16.0);
                                    
                                    let button = egui::Button::new(RethTheme::body_text("Install Again"))
                                        .min_size(egui::vec2(120.0, 36.0));
                                    
                                    if ui.add(button).clicked() {
                                        self.install_status = InstallStatus::Idle;
                                        self.reset_installer();
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
                
                // Footer with platform info
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RethTheme::muted_text("Platform:"));
                        ui.label(RethTheme::muted_text(std::env::consts::OS));
                        ui.label(RethTheme::muted_text("•"));
                        ui.label(RethTheme::muted_text(std::env::consts::ARCH));
                    });
                });
                ui.add_space(20.0);
            });
        });
    }
}