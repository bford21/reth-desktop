use crate::settings::{DesktopSettings, DesktopSettingsManager};
use crate::theme::RethTheme;

pub struct DesktopSettingsWindow;

impl DesktopSettingsWindow {
    /// Show the desktop settings window content
    pub fn show_content(ui: &mut egui::Ui, desktop_settings: &mut DesktopSettings) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            
            // Background running setting
            ui.horizontal(|ui| {
                ui.label("Keep Reth running in the background:");
                if ui.checkbox(&mut desktop_settings.keep_reth_running_in_background, "").changed() {
                    // Save settings when changed
                    if let Err(e) = DesktopSettingsManager::save_desktop_settings(desktop_settings) {
                        eprintln!("Failed to save desktop settings: {}", e);
                    }
                }
            });
            
            ui.add_space(8.0);
            ui.label(RethTheme::muted_text("When enabled, Reth will continue running even when the application window is closed."));
        });
    }
}