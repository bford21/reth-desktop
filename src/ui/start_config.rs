use crate::theme::RethTheme;
use crate::reth_node::{RethNode, CliOption};
use crate::settings::{DesktopSettings, DesktopSettingsManager};

pub struct StartConfigWindow;

impl StartConfigWindow {
    /// Show the start config window content
    pub fn show_content(
        ui: &mut egui::Ui,
        reth_node: &RethNode,
        desktop_settings: &mut DesktopSettings,
        available_cli_options: &[CliOption],
        selected_cli_option: &mut Option<usize>,
        parameter_value: &mut String,
        selected_values: &mut Vec<String>,
        pending_launch_args: &mut Vec<String>,
    ) -> bool {
        let mut restart_requested = false;
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            
            ui.heading("Start Configuration");
            ui.add_space(16.0);
            
            // Show reth binary location first
            Self::show_binary_location(ui);
            ui.add_space(16.0);
            
            // Parameter management section
            restart_requested = Self::show_parameter_management(
                ui,
                available_cli_options,
                selected_cli_option,
                parameter_value,
                selected_values,
                pending_launch_args,
                desktop_settings,
                reth_node,
            );
        });
        
        restart_requested
    }
    
    fn show_parameter_management(
        ui: &mut egui::Ui,
        available_cli_options: &[CliOption],
        selected_cli_option: &mut Option<usize>,
        parameter_value: &mut String,
        selected_values: &mut Vec<String>,
        pending_launch_args: &mut Vec<String>,
        desktop_settings: &mut DesktopSettings,
        reth_node: &RethNode,
    ) -> bool {
        let mut restart_requested = false;
        
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RethTheme::text("Parameter Management"));
            });
                
                ui.add_space(8.0);
                
                // Add new parameter section
                ui.collapsing("Add New Parameter", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Parameter:");
                        
                        let selected_option_name = if let Some(idx) = *selected_cli_option {
                            if idx < available_cli_options.len() {
                                available_cli_options[idx].name.clone()
                            } else {
                                "Select parameter".to_string()
                            }
                        } else {
                            "Select parameter".to_string()
                        };
                        
                        egui::ComboBox::from_id_source("cli_option_selector")
                            .selected_text(selected_option_name)
                            .show_ui(ui, |ui| {
                                for (idx, option) in available_cli_options.iter().enumerate() {
                                    let response = ui.selectable_value(
                                        selected_cli_option,
                                        Some(idx),
                                        &option.name,
                                    );
                                    
                                    if response.clicked() {
                                        parameter_value.clear();
                                        selected_values.clear();
                                    }
                                }
                            });
                    });
                    
                    // Show description and value input for selected parameter
                    if let Some(idx) = *selected_cli_option {
                        if idx < available_cli_options.len() {
                            let option = &available_cli_options[idx];
                            
                            ui.add_space(4.0);
                            ui.label(RethTheme::muted_text(&option.description));
                            
                            if option.takes_value {
                                ui.horizontal(|ui| {
                                    ui.label("Value:");
                                    
                                    if let Some(possible_values) = &option.possible_values {
                                        // Dropdown for predefined values
                                        egui::ComboBox::from_id_source("parameter_value_selector")
                                            .selected_text(parameter_value.as_str())
                                            .show_ui(ui, |ui| {
                                                for value in possible_values {
                                                    ui.selectable_value(parameter_value, value.clone(), value);
                                                }
                                            });
                                    } else {
                                        // Text input for free-form values
                                        ui.text_edit_singleline(parameter_value);
                                    }
                                });
                                
                                if option.accepts_multiple && !parameter_value.is_empty() {
                                    ui.horizontal(|ui| {
                                        if ui.button("Add Value").clicked() {
                                            if !selected_values.contains(parameter_value) {
                                                selected_values.push(parameter_value.clone());
                                                parameter_value.clear();
                                            }
                                        }
                                    });
                                    
                                    if !selected_values.is_empty() {
                                        ui.label("Selected values:");
                                        let mut to_remove = Vec::new();
                                        for (i, value) in selected_values.iter().enumerate() {
                                            ui.horizontal(|ui| {
                                                ui.label(format!("  • {}", value));
                                                if ui.small_button("🗑").clicked() {
                                                    to_remove.push(i);
                                                }
                                            });
                                        }
                                        for i in to_remove.into_iter().rev() {
                                            selected_values.remove(i);
                                        }
                                    }
                                }
                            }
                            
                            ui.add_space(8.0);
                            
                            // Add parameter button
                            ui.horizontal(|ui| {
                                let can_add = if option.takes_value {
                                    if option.accepts_multiple {
                                        !selected_values.is_empty()
                                    } else {
                                        !parameter_value.is_empty()
                                    }
                                } else {
                                    true
                                };
                                
                                if ui.add_enabled(can_add, egui::Button::new("Add Parameter")).clicked() {
                                    let mut new_args = vec![format!("--{}", option.name)];
                                    
                                    if option.takes_value {
                                        if option.accepts_multiple {
                                            for value in selected_values.iter() {
                                                new_args.push(value.clone());
                                            }
                                            selected_values.clear();
                                        } else {
                                            new_args.push(parameter_value.clone());
                                            parameter_value.clear();
                                        }
                                    }
                                    
                                    pending_launch_args.extend(new_args);
                                    *selected_cli_option = None;
                                }
                            });
                        }
                    }
                });
                
                ui.add_space(12.0);
                
                // Unified parameters section showing ALL parameters used to launch reth
                ui.label(RethTheme::text("All Reth Launch Parameters"));
                ui.add_space(4.0);
                
                ui.group(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RethTheme::muted_text("These are all the parameters that will be used when launching reth:"));
                        });
                        ui.add_space(8.0);
                        
                        // Show the base command
                        ui.horizontal(|ui| {
                            ui.label(RethTheme::monospace_text("reth node"));
                            ui.label(RethTheme::muted_text("(base command)"));
                        });
                        
                        // Get pending deletions to filter them out
                        let pending_deletions_id = egui::Id::new("pending_deletions");
                        let pending_deletions: Vec<String> = ui.ctx().memory(|mem| {
                            mem.data.get_temp::<Vec<String>>(pending_deletions_id)
                                .map(|v| v.clone())
                                .unwrap_or_default()
                        });
                        
                        // Create a unified list of all parameters with their current values
                        let mut all_parameters = vec![];
                        
                        // Add core parameters based on settings (unless pending deletion)
                        if desktop_settings.reth_defaults.enable_full_node && !pending_deletions.contains(&"--full".to_string()) {
                            all_parameters.push(("--full".to_string(), None));
                        }
                        
                        if desktop_settings.reth_defaults.enable_metrics && !pending_deletions.contains(&"--metrics".to_string()) {
                            all_parameters.push(("--metrics".to_string(), Some(desktop_settings.reth_defaults.metrics_address.clone())));
                        }
                        
                        // Network parameters (unless pending deletion)
                        if !pending_deletions.contains(&"--chain".to_string()) {
                            all_parameters.push(("--chain".to_string(), Some(desktop_settings.reth_defaults.chain.clone())));
                        }
                        if !pending_deletions.contains(&"--datadir".to_string()) {
                            all_parameters.push(("--datadir".to_string(), Some(desktop_settings.reth_defaults.datadir.clone())));
                        }
                        
                        // Logging parameters (unless pending deletion)
                        if desktop_settings.reth_defaults.enable_stdout_logging && !pending_deletions.contains(&"--log.stdout.format".to_string()) {
                            all_parameters.push(("--log.stdout.format".to_string(), Some(desktop_settings.reth_defaults.stdout_log_format.clone())));
                        }
                        
                        if desktop_settings.reth_defaults.enable_file_logging && !pending_deletions.iter().any(|p| p.starts_with("--log.file.")) {
                            if !pending_deletions.contains(&"--log.file.format".to_string()) {
                                all_parameters.push(("--log.file.format".to_string(), Some(desktop_settings.reth_defaults.file_log_format.clone())));
                            }
                            if !pending_deletions.contains(&"--log.file.filter".to_string()) {
                                all_parameters.push(("--log.file.filter".to_string(), Some(desktop_settings.reth_defaults.file_log_level.clone())));
                            }
                            if !pending_deletions.contains(&"--log.file.max-size".to_string()) {
                                all_parameters.push(("--log.file.max-size".to_string(), Some(desktop_settings.reth_defaults.file_log_max_size.clone())));
                            }
                            if !pending_deletions.contains(&"--log.file.max-files".to_string()) {
                                all_parameters.push(("--log.file.max-files".to_string(), Some(desktop_settings.reth_defaults.file_log_max_files.clone())));
                            }
                        }
                        
                        // Add custom parameters (unless pending deletion)
                        for custom_param in &desktop_settings.custom_launch_args {
                            if custom_param.starts_with("--") {
                                // Parse parameter and value if it has one
                                let parts: Vec<&str> = custom_param.splitn(2, ' ').collect();
                                let param_name = parts[0].to_string();
                                
                                if !pending_deletions.contains(&param_name) {
                                    if parts.len() == 2 {
                                        all_parameters.push((param_name, Some(parts[1].to_string())));
                                    } else {
                                        all_parameters.push((custom_param.clone(), None));
                                    }
                                }
                            }
                        }
                        
                        // Show all parameters with unified editing interface
                        let mut to_delete = Vec::new();
                        
                        // Get editing state from memory
                        let param_edit_id = egui::Id::new("param_edit_state");
                        let mut editing_state: Option<(usize, String)> = ui.ctx().memory(|mem| {
                            mem.data.get_temp::<(usize, String)>(param_edit_id).map(|state| state.clone())
                        });
                        
                        for (i, (param, value)) in all_parameters.iter().enumerate() {
                            ui.horizontal(|ui| {
                                // Display parameter
                                ui.label(RethTheme::monospace_text(param));
                                
                                // Show value or edit field
                                if let Some((edit_idx, ref mut edit_buffer)) = editing_state {
                                    if edit_idx == i {
                                        // Show inline edit field
                                        let response = ui.add(egui::TextEdit::singleline(edit_buffer).desired_width(200.0));
                                        
                                        // Update the stored state if text changed
                                        if response.changed() {
                                            let new_state: (usize, String) = (i, edit_buffer.clone());
                                            ui.ctx().memory_mut(|mem| {
                                                mem.data.insert_temp(param_edit_id, new_state);
                                            });
                                        }
                                        
                                        ui.add_space(8.0); // Add space between text field and buttons
                                        
                                        // Save button with better styling
                                        if ui.add(egui::Button::new("✓ Save")
                                            .fill(RethTheme::SUCCESS.gamma_multiply(0.2))
                                            .stroke(egui::Stroke::new(1.0, RethTheme::SUCCESS))
                                            .min_size(egui::Vec2::new(60.0, 20.0)))
                                            .on_hover_text("Save changes")
                                            .clicked() {
                                            // Apply the edit
                                            Self::apply_parameter_edit(param, edit_buffer, desktop_settings);
                                            
                                            // Clear editing state
                                            ui.ctx().memory_mut(|mem| {
                                                mem.data.remove::<(usize, String)>(param_edit_id);
                                            });
                                        }
                                        
                                        // Cancel button with better styling
                                        if ui.add(egui::Button::new("✕ Cancel")
                                            .fill(RethTheme::ERROR.gamma_multiply(0.2))
                                            .stroke(egui::Stroke::new(1.0, RethTheme::ERROR))
                                            .min_size(egui::Vec2::new(60.0, 20.0)))
                                            .on_hover_text("Cancel changes")
                                            .clicked() {
                                            // Clear editing state
                                            ui.ctx().memory_mut(|mem| {
                                                mem.data.remove::<(usize, String)>(param_edit_id);
                                            });
                                        }
                                    } else if let Some(val) = value {
                                        ui.label(RethTheme::monospace_text(val));
                                        
                                        // Edit button (only for parameters with values)
                                        if ui.add_sized([20.0, 20.0], egui::Button::new("📝")).on_hover_text("Edit value").clicked() {
                                            // Store editing state
                                            ui.ctx().memory_mut(|mem| {
                                                mem.data.insert_temp(param_edit_id, (i, val.clone()));
                                            });
                                        }
                                    }
                                } else {
                                    // Not editing this parameter
                                    if let Some(val) = value {
                                        ui.label(RethTheme::monospace_text(val));
                                        
                                        // Edit button (only for parameters with values)
                                        if ui.add_sized([20.0, 20.0], egui::Button::new("📝")).on_hover_text("Edit value").clicked() {
                                            // Store editing state
                                            ui.ctx().memory_mut(|mem| {
                                                mem.data.insert_temp(param_edit_id, (i, val.clone()));
                                            });
                                        }
                                    }
                                }
                                
                                // Delete button
                                if ui.add_sized([20.0, 20.0], egui::Button::new("🗑")).on_hover_text("Delete parameter").clicked() {
                                    to_delete.push(i);
                                }
                            });
                        }
                        
                        if all_parameters.is_empty() {
                            ui.horizontal(|ui| {
                                ui.label(RethTheme::muted_text("No parameters configured"));
                            });
                        }
                        
                        // Get or create pending deletions list from memory
                        let pending_deletions_id = egui::Id::new("pending_deletions");
                        let mut pending_deletions: Vec<String> = ui.ctx().memory(|mem| {
                            mem.data.get_temp::<Vec<String>>(pending_deletions_id)
                                .map(|v| v.clone())
                                .unwrap_or_default()
                        });
                        
                        // Handle parameter deletions - add to pending list
                        for &i in to_delete.iter() {
                            if i < all_parameters.len() {
                                let (param_name, _) = &all_parameters[i];
                                if !pending_deletions.contains(param_name) {
                                    pending_deletions.push(param_name.clone());
                                }
                            }
                        }
                        
                        // Update memory with pending deletions
                        if !pending_deletions.is_empty() {
                            ui.ctx().memory_mut(|mem| {
                                mem.data.insert_temp(pending_deletions_id, pending_deletions.clone());
                            });
                        }
                        
                        // Show pending deletions if any
                        if !pending_deletions.is_empty() {
                            ui.add_space(8.0);
                            ui.label(RethTheme::warning_text("Pending deletions:"));
                            for param in &pending_deletions {
                                ui.horizontal(|ui| {
                                    ui.label(RethTheme::warning_text(&format!("  • {} (will be removed)", param)));
                                });
                            }
                        }
                        
                        ui.add_space(8.0);

                    });
            });
            ui.add_space(8.0);
                
                // Show pending parameters
                if !pending_launch_args.is_empty() {
                    ui.label(RethTheme::warning_text("Pending parameters (not saved):"));
                    let mut to_remove = Vec::new();
                    for (i, arg) in pending_launch_args.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(RethTheme::warning_text(arg));
                            if ui.small_button("🗑").clicked() {
                                to_remove.push(i);
                            }
                        });
                    }
                    for i in to_remove.into_iter().rev() {
                        pending_launch_args.remove(i);
                    }
                    ui.add_space(8.0);
                }
                
                // Save/Clear buttons
                let mut parameters_saved = false;
                
                // Get pending deletions from memory
                let pending_deletions_id = egui::Id::new("pending_deletions");
                let pending_deletions: Vec<String> = ui.ctx().memory(|mem| {
                    mem.data.get_temp::<Vec<String>>(pending_deletions_id)
                        .map(|v| v.clone())
                        .unwrap_or_default()
                });
                
                let has_pending_changes = !pending_launch_args.is_empty() || !pending_deletions.is_empty();
                
                ui.horizontal(|ui| {
                    if ui.add_enabled(has_pending_changes, egui::Button::new("💾 Save Changes")).clicked() {
                        // Process pending deletions
                        for param_to_delete in &pending_deletions {
                            match param_to_delete.as_str() {
                                "--full" => desktop_settings.reth_defaults.enable_full_node = false,
                                "--metrics" => desktop_settings.reth_defaults.enable_metrics = false,
                                "--log.stdout.format" => desktop_settings.reth_defaults.enable_stdout_logging = false,
                                "--log.file.format" | "--log.file.filter" | "--log.file.max-size" | "--log.file.max-files" => {
                                    desktop_settings.reth_defaults.enable_file_logging = false;
                                }
                                _ => {
                                    // Remove from custom_launch_args
                                    desktop_settings.custom_launch_args.retain(|arg| !arg.starts_with(&format!("{} ", param_to_delete)) && arg != param_to_delete);
                                }
                            }
                        }
                        
                        // Process pending additions
                        desktop_settings.custom_launch_args.extend(pending_launch_args.drain(..));
                        
                        // Clear pending deletions
                        ui.ctx().memory_mut(|mem| {
                            mem.data.remove::<Vec<String>>(pending_deletions_id);
                        });
                        
                        parameters_saved = true;
                        
                        // Save desktop settings to file
                        if let Err(e) = DesktopSettingsManager::save_desktop_settings(desktop_settings) {
                            println!("Failed to save settings: {}", e);
                        } else {
                            println!("Settings saved successfully to settings.toml");
                        }
                    }
                    
                    if ui.add_enabled(has_pending_changes, egui::Button::new("🗑 Discard Changes")).clicked() {
                        pending_launch_args.clear();
                        // Clear pending deletions
                        ui.ctx().memory_mut(|mem| {
                            mem.data.remove::<Vec<String>>(pending_deletions_id);
                        });
                    }
                    
                    if ui.add_enabled(!desktop_settings.custom_launch_args.is_empty(), egui::Button::new("🗑 Clear All Saved")).clicked() {
                        desktop_settings.custom_launch_args.clear();
                        parameters_saved = true;
                        // Save desktop settings to file
                        if let Err(e) = DesktopSettingsManager::save_desktop_settings(desktop_settings) {
                            println!("Failed to save settings after clearing: {}", e);
                        } else {
                            println!("All custom parameters cleared and settings saved");
                        }
                    }
                });
                
                // Show restart button if parameters were saved and node is running
                if parameters_saved && reth_node.is_running() {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RethTheme::warning_text("⚠ Parameter changes require restart to take effect"));
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("🔄 Restart Node")
                            .color(RethTheme::WARNING))
                            .fill(RethTheme::WARNING.gamma_multiply(0.2))
                            .stroke(egui::Stroke::new(1.0, RethTheme::WARNING)))
                            .clicked() {
                            restart_requested = true;
                        }
                        
                        ui.label(RethTheme::muted_text("(This will stop and restart the node with new parameters)"));
                    });
                } else if parameters_saved {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RethTheme::success_text("✓ Parameters saved. They will be applied when the node is started."));
                    });
                }
            });
        
        restart_requested
    }
    
    fn show_binary_location(ui: &mut egui::Ui) {
        let reth_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".reth-desktop")
            .join("bin")
            .join("reth");
        
        ui.horizontal(|ui| {
            ui.label(RethTheme::text("Reth Binary Location:"));
            ui.label(RethTheme::monospace_text(&reth_path.to_string_lossy()));
        });
    }
    
    fn apply_parameter_edit(param_name: &str, new_value: &str, desktop_settings: &mut DesktopSettings) {
        match param_name {
            "--chain" => {
                desktop_settings.reth_defaults.chain = new_value.to_string();
            }
            "--datadir" => {
                desktop_settings.reth_defaults.datadir = new_value.to_string();
            }
            "--metrics" => {
                desktop_settings.reth_defaults.metrics_address = new_value.to_string();
            }
            "--log.stdout.format" => {
                desktop_settings.reth_defaults.stdout_log_format = new_value.to_string();
            }
            "--log.file.format" => {
                desktop_settings.reth_defaults.file_log_format = new_value.to_string();
            }
            "--log.file.filter" => {
                desktop_settings.reth_defaults.file_log_level = new_value.to_string();
            }
            "--log.file.max-size" => {
                desktop_settings.reth_defaults.file_log_max_size = new_value.to_string();
            }
            "--log.file.max-files" => {
                desktop_settings.reth_defaults.file_log_max_files = new_value.to_string();
            }
            _ => {
                // For custom parameters, update in custom_launch_args
                // First remove any existing version
                desktop_settings.custom_launch_args.retain(|arg| !arg.starts_with(&format!("{} ", param_name)));
                // Then add the new version
                desktop_settings.custom_launch_args.push(format!("{} {}", param_name, new_value));
            }
        }
        
        // Save changes immediately
        if let Err(e) = DesktopSettingsManager::save_desktop_settings(desktop_settings) {
            println!("Failed to save settings after edit: {}", e);
        } else {
            println!("Parameter {} updated to: {}", param_name, new_value);
        }
    }
}