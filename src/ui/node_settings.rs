use crate::config::*;
use crate::theme::RethTheme;

pub struct NodeSettingsWindow;

impl NodeSettingsWindow {
    /// Show the node settings window content
    pub fn show_content(
        ui: &mut egui::Ui,
        reth_config: &RethConfig,
        reth_config_path: &Option<std::path::PathBuf>,
        editable_config: &mut RethConfig,
        config_modified: &mut bool,
        settings_edit_mode: &mut bool,
    ) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(8.0);
            
            // Config file path
            let reth_data_dir = RethConfigManager::get_reth_data_dir();
            if let Some(config_path) = reth_config_path {
                ui.label(RethTheme::muted_text(&format!("Configuration file: {}", config_path.display())));
            } else {
                ui.label(RethTheme::muted_text("Configuration file: Not found (using defaults)"));
            }
            ui.label(RethTheme::muted_text(&format!("Reth data directory: {}", reth_data_dir.display())));
            ui.add_space(12.0);
            
            // Edit mode toggle
            ui.horizontal(|ui| {
                if !*settings_edit_mode {
                    if ui.button("🖊 Edit").clicked() {
                        *settings_edit_mode = true;
                        *editable_config = reth_config.clone();
                        *config_modified = false;
                    }
                } else {
                    if ui.button("👁 View Mode").clicked() {
                        *settings_edit_mode = false;
                        *editable_config = reth_config.clone();
                        *config_modified = false;
                    }
                    ui.add_space(8.0);
                    ui.label(RethTheme::success_text("✏ Edit mode active - you can modify configuration values"));
                }
            });
            ui.add_space(16.0);
            
            // Configuration sections
            Self::show_stages_config(ui, reth_config, editable_config, config_modified, *settings_edit_mode);
            ui.add_space(12.0);
            
            Self::show_peers_config(ui, reth_config, editable_config, config_modified, *settings_edit_mode);
            ui.add_space(12.0);
            
            Self::show_sessions_config(ui, reth_config, editable_config, config_modified, *settings_edit_mode);
            ui.add_space(12.0);
            
            Self::show_pruning_config(ui, reth_config, editable_config, config_modified, *settings_edit_mode);
            ui.add_space(24.0);
            
            // Save/Reset buttons
            Self::show_action_buttons(ui, config_modified, settings_edit_mode, editable_config, reth_config, reth_config_path);
        });
    }
    
    /// Editable field helpers
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
    
    // Note: The actual implementation of show_stages_config, show_peers_config, etc.
    // would be quite long. For now, I'll create stub implementations to show the structure.
    // The full implementations can be moved from main.rs in the refactoring step.
    
    fn show_stages_config(
        ui: &mut egui::Ui,
        reth_config: &RethConfig,
        editable_config: &mut RethConfig,
        config_modified: &mut bool,
        settings_edit_mode: bool,
    ) {
        ui.collapsing("Stages Configuration", |ui| {
            // Era Stage
            if reth_config.stages.era.is_some() {
                ui.label("Era Stage: Configured");
            }
            
            // Headers Stage
            if settings_edit_mode {
                if let Some(headers) = &mut editable_config.stages.headers {
                    ui.label("Headers Stage:");
                    ui.indent("headers", |ui| {
                        if Self::editable_u32_field(ui, "Max Concurrent Requests", &mut headers.downloader_max_concurrent_requests) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Min Concurrent Requests", &mut headers.downloader_min_concurrent_requests) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Max Buffered Responses", &mut headers.downloader_max_buffered_responses) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Request Limit", &mut headers.downloader_request_limit) {
                            *config_modified = true;
                        }
                        if Self::editable_u64_field(ui, "Commit Threshold", &mut headers.commit_threshold) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Headers Stage").clicked() {
                        editable_config.stages.headers = Some(HeadersStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                // Read-only view
                if let Some(headers) = &reth_config.stages.headers {
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
            if settings_edit_mode {
                if let Some(bodies) = &mut editable_config.stages.bodies {
                    ui.label("Bodies Stage:");
                    ui.indent("bodies", |ui| {
                        if Self::editable_u32_field(ui, "Request Limit", &mut bodies.downloader_request_limit) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Stream Batch Size", &mut bodies.downloader_stream_batch_size) {
                            *config_modified = true;
                        }
                        if Self::editable_u64_field(ui, "Max Buffered Blocks Size (bytes)", &mut bodies.downloader_max_buffered_blocks_size_bytes) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Min Concurrent Requests", &mut bodies.downloader_min_concurrent_requests) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Max Concurrent Requests", &mut bodies.downloader_max_concurrent_requests) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Bodies Stage").clicked() {
                        editable_config.stages.bodies = Some(BodiesStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(bodies) = &reth_config.stages.bodies {
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
            if settings_edit_mode {
                if let Some(sender_recovery) = &mut editable_config.stages.sender_recovery {
                    ui.label("Sender Recovery Stage:");
                    ui.indent("sender_recovery", |ui| {
                        if Self::editable_u64_field(ui, "Commit Threshold", &mut sender_recovery.commit_threshold) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Sender Recovery Stage").clicked() {
                        editable_config.stages.sender_recovery = Some(SenderRecoveryStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(sender_recovery) = &reth_config.stages.sender_recovery {
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
            if settings_edit_mode {
                if let Some(execution) = &mut editable_config.stages.execution {
                    ui.label("Execution Stage:");
                    ui.indent("execution", |ui| {
                        if Self::editable_u64_field(ui, "Max Blocks", &mut execution.max_blocks) {
                            *config_modified = true;
                        }
                        if Self::editable_u64_field(ui, "Max Changes", &mut execution.max_changes) {
                            *config_modified = true;
                        }
                        if Self::editable_u64_field(ui, "Max Cumulative Gas", &mut execution.max_cumulative_gas) {
                            *config_modified = true;
                        }
                        if Self::editable_string_field(ui, "Max Duration", &mut execution.max_duration) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Execution Stage").clicked() {
                        editable_config.stages.execution = Some(ExecutionStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(execution) = &reth_config.stages.execution {
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
            if settings_edit_mode {
                if let Some(prune_stage) = &mut editable_config.stages.prune {
                    ui.label("Prune Stage:");
                    ui.indent("prune_stage", |ui| {
                        if Self::editable_u64_field(ui, "Commit Threshold", &mut prune_stage.commit_threshold) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Prune Stage").clicked() {
                        editable_config.stages.prune = Some(PruneStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(prune_stage) = &reth_config.stages.prune {
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
            if settings_edit_mode {
                if let Some(account_hashing) = &mut editable_config.stages.account_hashing {
                    ui.label("Account Hashing Stage:");
                    ui.indent("account_hashing", |ui| {
                        if Self::editable_u64_field(ui, "Clean Threshold", &mut account_hashing.clean_threshold) {
                            *config_modified = true;
                        }
                        if Self::editable_u64_field(ui, "Commit Threshold", &mut account_hashing.commit_threshold) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Account Hashing Stage").clicked() {
                        editable_config.stages.account_hashing = Some(AccountHashingStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(account_hashing) = &reth_config.stages.account_hashing {
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
            if settings_edit_mode {
                if let Some(storage_hashing) = &mut editable_config.stages.storage_hashing {
                    ui.label("Storage Hashing Stage:");
                    ui.indent("storage_hashing", |ui| {
                        if Self::editable_u64_field(ui, "Clean Threshold", &mut storage_hashing.clean_threshold) {
                            *config_modified = true;
                        }
                        if Self::editable_u64_field(ui, "Commit Threshold", &mut storage_hashing.commit_threshold) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Storage Hashing Stage").clicked() {
                        editable_config.stages.storage_hashing = Some(StorageHashingStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(storage_hashing) = &reth_config.stages.storage_hashing {
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
            if settings_edit_mode {
                if let Some(merkle) = &mut editable_config.stages.merkle {
                    ui.label("Merkle Stage:");
                    ui.indent("merkle", |ui| {
                        if Self::editable_u64_field(ui, "Incremental Threshold", &mut merkle.incremental_threshold) {
                            *config_modified = true;
                        }
                        if Self::editable_u64_field(ui, "Rebuild Threshold", &mut merkle.rebuild_threshold) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Merkle Stage").clicked() {
                        editable_config.stages.merkle = Some(MerkleStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(merkle) = &reth_config.stages.merkle {
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
            if settings_edit_mode {
                if let Some(tx_lookup) = &mut editable_config.stages.transaction_lookup {
                    ui.label("Transaction Lookup Stage:");
                    ui.indent("transaction_lookup", |ui| {
                        if Self::editable_u64_field(ui, "Chunk Size", &mut tx_lookup.chunk_size) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Transaction Lookup Stage").clicked() {
                        editable_config.stages.transaction_lookup = Some(TransactionLookupStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(tx_lookup) = &reth_config.stages.transaction_lookup {
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
            if settings_edit_mode {
                if let Some(index_account) = &mut editable_config.stages.index_account_history {
                    ui.label("Index Account History Stage:");
                    ui.indent("index_account_history", |ui| {
                        if Self::editable_u64_field(ui, "Commit Threshold", &mut index_account.commit_threshold) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Index Account History Stage").clicked() {
                        editable_config.stages.index_account_history = Some(IndexAccountHistoryStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(index_account) = &reth_config.stages.index_account_history {
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
            if settings_edit_mode {
                if let Some(index_storage) = &mut editable_config.stages.index_storage_history {
                    ui.label("Index Storage History Stage:");
                    ui.indent("index_storage_history", |ui| {
                        if Self::editable_u64_field(ui, "Commit Threshold", &mut index_storage.commit_threshold) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add Index Storage History Stage").clicked() {
                        editable_config.stages.index_storage_history = Some(IndexStorageHistoryStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(index_storage) = &reth_config.stages.index_storage_history {
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
            if settings_edit_mode {
                if let Some(etl) = &mut editable_config.stages.etl {
                    ui.label("ETL Stage:");
                    ui.indent("etl", |ui| {
                        if Self::editable_u64_field(ui, "File Size (bytes)", &mut etl.file_size) {
                            *config_modified = true;
                        }
                    });
                    ui.add_space(8.0);
                } else {
                    if ui.button("+ Add ETL Stage").clicked() {
                        editable_config.stages.etl = Some(EtlStageConfig::default());
                        *config_modified = true;
                    }
                    ui.add_space(8.0);
                }
            } else {
                if let Some(etl) = &reth_config.stages.etl {
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
    }
    
    fn show_peers_config(
        ui: &mut egui::Ui,
        reth_config: &RethConfig,
        editable_config: &mut RethConfig,
        config_modified: &mut bool,
        settings_edit_mode: bool,
    ) {
        ui.collapsing("Peers Configuration", |ui| {
            if settings_edit_mode {
                // Basic peer settings
                if Self::editable_string_field(ui, "Refill Slots Interval", &mut editable_config.peers.refill_slots_interval) {
                    *config_modified = true;
                }
                if Self::editable_bool_field(ui, "Trusted Nodes Only", &mut editable_config.peers.trusted_nodes_only) {
                    *config_modified = true;
                }
                if Self::editable_string_field(ui, "Trusted Nodes Resolution Interval", &mut editable_config.peers.trusted_nodes_resolution_interval) {
                    *config_modified = true;
                }
                if Self::editable_u32_field(ui, "Max Backoff Count", &mut editable_config.peers.max_backoff_count) {
                    *config_modified = true;
                }
                if Self::editable_string_field(ui, "Ban Duration", &mut editable_config.peers.ban_duration) {
                    *config_modified = true;
                }
                if Self::editable_string_field(ui, "Incoming IP Throttle Duration", &mut editable_config.peers.incoming_ip_throttle_duration) {
                    *config_modified = true;
                }
                
                ui.add_space(8.0);
                
                // Trusted nodes list
                ui.label("Trusted Nodes:");
                ui.indent("trusted_nodes", |ui| {
                    if editable_config.peers.trusted_nodes.is_none() {
                        editable_config.peers.trusted_nodes = Some(Vec::new());
                    }
                    
                    if let Some(trusted_nodes) = &mut editable_config.peers.trusted_nodes {
                        let mut to_remove = Vec::new();
                        for (i, node) in trusted_nodes.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                if ui.text_edit_singleline(node).changed() {
                                    *config_modified = true;
                                }
                                if ui.button("🗑").clicked() {
                                    to_remove.push(i);
                                    *config_modified = true;
                                }
                            });
                        }
                        
                        // Remove nodes marked for deletion
                        for i in to_remove.into_iter().rev() {
                            trusted_nodes.remove(i);
                        }
                        
                        if ui.button("+ Add Trusted Node").clicked() {
                            trusted_nodes.push(String::new());
                            *config_modified = true;
                        }
                    }
                });
                
                ui.add_space(8.0);
                
                // Connection info
                ui.collapsing("Connection Info", |ui| {
                    if editable_config.peers.connection_info.is_none() {
                        if ui.button("+ Add Connection Info").clicked() {
                            editable_config.peers.connection_info = Some(ConnectionInfoConfig::default());
                            *config_modified = true;
                        }
                    } else if let Some(conn_info) = &mut editable_config.peers.connection_info {
                        if Self::editable_u32_field(ui, "Max Outbound", &mut conn_info.max_outbound) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Max Inbound", &mut conn_info.max_inbound) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Max Concurrent Outbound Dials", &mut conn_info.max_concurrent_outbound_dials) {
                            *config_modified = true;
                        }
                        
                        if ui.button("🗑 Remove Connection Info").clicked() {
                            editable_config.peers.connection_info = None;
                            *config_modified = true;
                        }
                    }
                });
                
                ui.add_space(8.0);
                
                // Reputation weights
                ui.collapsing("Reputation Weights", |ui| {
                    if editable_config.peers.reputation_weights.is_none() {
                        if ui.button("+ Add Reputation Weights").clicked() {
                            editable_config.peers.reputation_weights = Some(ReputationWeightsConfig::default());
                            *config_modified = true;
                        }
                    } else if let Some(rep_weights) = &mut editable_config.peers.reputation_weights {
                        if Self::editable_i32_field(ui, "Bad Message", &mut rep_weights.bad_message) {
                            *config_modified = true;
                        }
                        if Self::editable_i32_field(ui, "Bad Block", &mut rep_weights.bad_block) {
                            *config_modified = true;
                        }
                        if Self::editable_i32_field(ui, "Bad Transactions", &mut rep_weights.bad_transactions) {
                            *config_modified = true;
                        }
                        if Self::editable_i32_field(ui, "Already Seen Transactions", &mut rep_weights.already_seen_transactions) {
                            *config_modified = true;
                        }
                        if Self::editable_i32_field(ui, "Timeout", &mut rep_weights.timeout) {
                            *config_modified = true;
                        }
                        if Self::editable_i32_field(ui, "Bad Protocol", &mut rep_weights.bad_protocol) {
                            *config_modified = true;
                        }
                        if Self::editable_i32_field(ui, "Failed to Connect", &mut rep_weights.failed_to_connect) {
                            *config_modified = true;
                        }
                        if Self::editable_i32_field(ui, "Dropped", &mut rep_weights.dropped) {
                            *config_modified = true;
                        }
                        if Self::editable_i32_field(ui, "Bad Announcement", &mut rep_weights.bad_announcement) {
                            *config_modified = true;
                        }
                        
                        if ui.button("🗑 Remove Reputation Weights").clicked() {
                            editable_config.peers.reputation_weights = None;
                            *config_modified = true;
                        }
                    }
                });
                
                ui.add_space(8.0);
                
                // Backoff durations
                ui.collapsing("Backoff Durations", |ui| {
                    if editable_config.peers.backoff_durations.is_none() {
                        if ui.button("+ Add Backoff Durations").clicked() {
                            editable_config.peers.backoff_durations = Some(BackoffDurationsConfig::default());
                            *config_modified = true;
                        }
                    } else if let Some(backoff) = &mut editable_config.peers.backoff_durations {
                        if Self::editable_string_field(ui, "Low", &mut backoff.low) {
                            *config_modified = true;
                        }
                        if Self::editable_string_field(ui, "Medium", &mut backoff.medium) {
                            *config_modified = true;
                        }
                        if Self::editable_string_field(ui, "High", &mut backoff.high) {
                            *config_modified = true;
                        }
                        if Self::editable_string_field(ui, "Max", &mut backoff.max) {
                            *config_modified = true;
                        }
                        
                        if ui.button("🗑 Remove Backoff Durations").clicked() {
                            editable_config.peers.backoff_durations = None;
                            *config_modified = true;
                        }
                    }
                });
            } else {
                // Read-only view
                if let Some(val) = &reth_config.peers.refill_slots_interval {
                    ui.label(&format!("Refill Slots Interval: {}", val));
                }
                if let Some(val) = reth_config.peers.trusted_nodes_only {
                    ui.label(&format!("Trusted Nodes Only: {}", val));
                }
                if let Some(val) = &reth_config.peers.trusted_nodes_resolution_interval {
                    ui.label(&format!("Trusted Nodes Resolution Interval: {}", val));
                }
                if let Some(val) = reth_config.peers.max_backoff_count {
                    ui.label(&format!("Max Backoff Count: {}", val));
                }
                if let Some(val) = &reth_config.peers.ban_duration {
                    ui.label(&format!("Ban Duration: {}", val));
                }
                if let Some(val) = &reth_config.peers.incoming_ip_throttle_duration {
                    ui.label(&format!("Incoming IP Throttle Duration: {}", val));
                }
                
                if let Some(trusted_nodes) = &reth_config.peers.trusted_nodes {
                    ui.label(&format!("Trusted Nodes ({}):", trusted_nodes.len()));
                    ui.indent("trusted_nodes_readonly", |ui| {
                        for node in trusted_nodes {
                            ui.label(&format!("• {}", node));
                        }
                    });
                }
                
                if let Some(conn_info) = &reth_config.peers.connection_info {
                    ui.label("Connection Info:");
                    ui.indent("conn_info_readonly", |ui| {
                        if let Some(val) = conn_info.max_outbound {
                            ui.label(&format!("Max Outbound: {}", val));
                        }
                        if let Some(val) = conn_info.max_inbound {
                            ui.label(&format!("Max Inbound: {}", val));
                        }
                        if let Some(val) = conn_info.max_concurrent_outbound_dials {
                            ui.label(&format!("Max Concurrent Outbound Dials: {}", val));
                        }
                    });
                }
                
                if reth_config.peers.reputation_weights.is_some() {
                    ui.label("Reputation Weights: Configured");
                }
                
                if reth_config.peers.backoff_durations.is_some() {
                    ui.label("Backoff Durations: Configured");
                }
            }
        });
    }
    
    fn show_sessions_config(
        ui: &mut egui::Ui,
        reth_config: &RethConfig,
        editable_config: &mut RethConfig,
        config_modified: &mut bool,
        settings_edit_mode: bool,
    ) {
        ui.collapsing("Sessions Configuration", |ui| {
            if settings_edit_mode {
                if Self::editable_u32_field(ui, "Session Command Buffer", &mut editable_config.sessions.session_command_buffer) {
                    *config_modified = true;
                }
                if Self::editable_u32_field(ui, "Session Event Buffer", &mut editable_config.sessions.session_event_buffer) {
                    *config_modified = true;
                }
                
                ui.add_space(8.0);
                
                // Session limits (appears to be empty struct, so just show add/remove)
                ui.horizontal(|ui| {
                    ui.label("Session Limits:");
                    if editable_config.sessions.limits.is_none() {
                        if ui.button("+ Add").clicked() {
                            editable_config.sessions.limits = Some(SessionLimitsConfig::default());
                            *config_modified = true;
                        }
                    } else {
                        ui.label("Configured");
                        if ui.button("🗑 Remove").clicked() {
                            editable_config.sessions.limits = None;
                            *config_modified = true;
                        }
                    }
                });
                
                ui.add_space(8.0);
                
                // Timeout configurations
                ui.collapsing("Initial Internal Request Timeout", |ui| {
                    if editable_config.sessions.initial_internal_request_timeout.is_none() {
                        if ui.button("+ Add Timeout").clicked() {
                            editable_config.sessions.initial_internal_request_timeout = Some(TimeoutConfig::default());
                            *config_modified = true;
                        }
                    } else if let Some(timeout) = &mut editable_config.sessions.initial_internal_request_timeout {
                        if Self::editable_u64_field(ui, "Seconds", &mut timeout.secs) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Nanoseconds", &mut timeout.nanos) {
                            *config_modified = true;
                        }
                        
                        if ui.button("🗑 Remove Timeout").clicked() {
                            editable_config.sessions.initial_internal_request_timeout = None;
                            *config_modified = true;
                        }
                    }
                });
                
                ui.collapsing("Protocol Breach Request Timeout", |ui| {
                    if editable_config.sessions.protocol_breach_request_timeout.is_none() {
                        if ui.button("+ Add Timeout").clicked() {
                            editable_config.sessions.protocol_breach_request_timeout = Some(TimeoutConfig::default());
                            *config_modified = true;
                        }
                    } else if let Some(timeout) = &mut editable_config.sessions.protocol_breach_request_timeout {
                        if Self::editable_u64_field(ui, "Seconds", &mut timeout.secs) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Nanoseconds", &mut timeout.nanos) {
                            *config_modified = true;
                        }
                        
                        if ui.button("🗑 Remove Timeout").clicked() {
                            editable_config.sessions.protocol_breach_request_timeout = None;
                            *config_modified = true;
                        }
                    }
                });
                
                ui.collapsing("Pending Session Timeout", |ui| {
                    if editable_config.sessions.pending_session_timeout.is_none() {
                        if ui.button("+ Add Timeout").clicked() {
                            editable_config.sessions.pending_session_timeout = Some(TimeoutConfig::default());
                            *config_modified = true;
                        }
                    } else if let Some(timeout) = &mut editable_config.sessions.pending_session_timeout {
                        if Self::editable_u64_field(ui, "Seconds", &mut timeout.secs) {
                            *config_modified = true;
                        }
                        if Self::editable_u32_field(ui, "Nanoseconds", &mut timeout.nanos) {
                            *config_modified = true;
                        }
                        
                        if ui.button("🗑 Remove Timeout").clicked() {
                            editable_config.sessions.pending_session_timeout = None;
                            *config_modified = true;
                        }
                    }
                });
            } else {
                // Read-only view
                if let Some(val) = reth_config.sessions.session_command_buffer {
                    ui.label(&format!("Session Command Buffer: {}", val));
                }
                if let Some(val) = reth_config.sessions.session_event_buffer {
                    ui.label(&format!("Session Event Buffer: {}", val));
                }
                
                if reth_config.sessions.limits.is_some() {
                    ui.label("Session Limits: Configured");
                }
                
                if let Some(timeout) = &reth_config.sessions.initial_internal_request_timeout {
                    ui.label("Initial Internal Request Timeout:");
                    ui.indent("initial_timeout_readonly", |ui| {
                        if let Some(secs) = timeout.secs {
                            ui.label(&format!("Seconds: {}", secs));
                        }
                        if let Some(nanos) = timeout.nanos {
                            ui.label(&format!("Nanoseconds: {}", nanos));
                        }
                    });
                }
                
                if let Some(timeout) = &reth_config.sessions.protocol_breach_request_timeout {
                    ui.label("Protocol Breach Request Timeout:");
                    ui.indent("breach_timeout_readonly", |ui| {
                        if let Some(secs) = timeout.secs {
                            ui.label(&format!("Seconds: {}", secs));
                        }
                        if let Some(nanos) = timeout.nanos {
                            ui.label(&format!("Nanoseconds: {}", nanos));
                        }
                    });
                }
                
                if let Some(timeout) = &reth_config.sessions.pending_session_timeout {
                    ui.label("Pending Session Timeout:");
                    ui.indent("pending_timeout_readonly", |ui| {
                        if let Some(secs) = timeout.secs {
                            ui.label(&format!("Seconds: {}", secs));
                        }
                        if let Some(nanos) = timeout.nanos {
                            ui.label(&format!("Nanoseconds: {}", nanos));
                        }
                    });
                }
            }
        });
    }
    
    fn show_pruning_config(
        ui: &mut egui::Ui,
        reth_config: &RethConfig,
        editable_config: &mut RethConfig,
        config_modified: &mut bool,
        settings_edit_mode: bool,
    ) {
        ui.collapsing("Pruning Configuration", |ui| {
            if settings_edit_mode {
                if Self::editable_u64_field(ui, "Block Interval", &mut editable_config.prune.block_interval) {
                    *config_modified = true;
                }
                
                ui.add_space(8.0);
                
                // Prune segments
                ui.collapsing("Prune Segments", |ui| {
                    if editable_config.prune.segments.is_none() {
                        if ui.button("+ Add Prune Segments").clicked() {
                            editable_config.prune.segments = Some(PruneSegments::default());
                            *config_modified = true;
                        }
                    } else if let Some(segments) = &mut editable_config.prune.segments {
                        if Self::editable_string_field(ui, "Sender Recovery", &mut segments.sender_recovery) {
                            *config_modified = true;
                        }
                        
                        ui.add_space(4.0);
                        
                        // Receipts config
                        ui.collapsing("Receipts", |ui| {
                            if segments.receipts.is_none() {
                                if ui.button("+ Add Receipts Config").clicked() {
                                    segments.receipts = Some(PruneReceiptsConfig::default());
                                    *config_modified = true;
                                }
                            } else if let Some(receipts) = &mut segments.receipts {
                                if Self::editable_u64_field(ui, "Distance", &mut receipts.distance) {
                                    *config_modified = true;
                                }
                                
                                if ui.button("🗑 Remove Receipts Config").clicked() {
                                    segments.receipts = None;
                                    *config_modified = true;
                                }
                            }
                        });
                        
                        // Account history config
                        ui.collapsing("Account History", |ui| {
                            if segments.account_history.is_none() {
                                if ui.button("+ Add Account History Config").clicked() {
                                    segments.account_history = Some(PruneHistoryConfig::default());
                                    *config_modified = true;
                                }
                            } else if let Some(account_history) = &mut segments.account_history {
                                if Self::editable_u64_field(ui, "Distance", &mut account_history.distance) {
                                    *config_modified = true;
                                }
                                
                                if ui.button("🗑 Remove Account History Config").clicked() {
                                    segments.account_history = None;
                                    *config_modified = true;
                                }
                            }
                        });
                        
                        // Storage history config
                        ui.collapsing("Storage History", |ui| {
                            if segments.storage_history.is_none() {
                                if ui.button("+ Add Storage History Config").clicked() {
                                    segments.storage_history = Some(PruneHistoryConfig::default());
                                    *config_modified = true;
                                }
                            } else if let Some(storage_history) = &mut segments.storage_history {
                                if Self::editable_u64_field(ui, "Distance", &mut storage_history.distance) {
                                    *config_modified = true;
                                }
                                
                                if ui.button("🗑 Remove Storage History Config").clicked() {
                                    segments.storage_history = None;
                                    *config_modified = true;
                                }
                            }
                        });
                        
                        // Receipts log filter (empty struct)
                        ui.horizontal(|ui| {
                            ui.label("Receipts Log Filter:");
                            if segments.receipts_log_filter.is_none() {
                                if ui.button("+ Add").clicked() {
                                    segments.receipts_log_filter = Some(PruneReceiptsLogFilterConfig::default());
                                    *config_modified = true;
                                }
                            } else {
                                ui.label("Configured");
                                if ui.button("🗑 Remove").clicked() {
                                    segments.receipts_log_filter = None;
                                    *config_modified = true;
                                }
                            }
                        });
                        
                        ui.add_space(8.0);
                        
                        if ui.button("🗑 Remove All Prune Segments").clicked() {
                            editable_config.prune.segments = None;
                            *config_modified = true;
                        }
                    }
                });
            } else {
                // Read-only view
                if let Some(val) = reth_config.prune.block_interval {
                    ui.label(&format!("Block Interval: {}", val));
                }
                
                if let Some(segments) = &reth_config.prune.segments {
                    ui.label("Prune Segments:");
                    ui.indent("prune_segments_readonly", |ui| {
                        if let Some(val) = &segments.sender_recovery {
                            ui.label(&format!("Sender Recovery: {}", val));
                        }
                        
                        if let Some(receipts) = &segments.receipts {
                            ui.label("Receipts:");
                            ui.indent("receipts_readonly", |ui| {
                                if let Some(distance) = receipts.distance {
                                    ui.label(&format!("Distance: {}", distance));
                                }
                            });
                        }
                        
                        if let Some(account_history) = &segments.account_history {
                            ui.label("Account History:");
                            ui.indent("account_history_readonly", |ui| {
                                if let Some(distance) = account_history.distance {
                                    ui.label(&format!("Distance: {}", distance));
                                }
                            });
                        }
                        
                        if let Some(storage_history) = &segments.storage_history {
                            ui.label("Storage History:");
                            ui.indent("storage_history_readonly", |ui| {
                                if let Some(distance) = storage_history.distance {
                                    ui.label(&format!("Distance: {}", distance));
                                }
                            });
                        }
                        
                        if segments.receipts_log_filter.is_some() {
                            ui.label("Receipts Log Filter: Configured");
                        }
                    });
                }
            }
        });
    }
    
    fn show_action_buttons(
        ui: &mut egui::Ui,
        config_modified: &mut bool,
        settings_edit_mode: &mut bool,
        editable_config: &mut RethConfig,
        reth_config: &RethConfig,
        reth_config_path: &Option<std::path::PathBuf>,
    ) {
        ui.horizontal(|ui| {
            if *settings_edit_mode {
                // Save button (only enabled if there are changes)
                let save_button = egui::Button::new("💾 Save Changes")
                    .fill(if *config_modified { RethTheme::SUCCESS } else { RethTheme::SURFACE });
                
                if ui.add_enabled(*config_modified, save_button).clicked() {
                    if let Some(config_path) = reth_config_path {
                        match RethConfigManager::save_reth_config(editable_config, config_path) {
                            Ok(()) => {
                                *settings_edit_mode = false; // Exit edit mode after saving
                                *config_modified = false;
                            }
                            Err(e) => {
                                eprintln!("Failed to save configuration: {}", e);
                            }
                        }
                    }
                }
                
                ui.add_space(8.0);
                
                // Cancel/Reset button (only enabled if there are changes)
                if ui.add_enabled(*config_modified, egui::Button::new("↶ Reset Changes")).clicked() {
                    *editable_config = reth_config.clone();
                    *config_modified = false;
                }
                
                ui.add_space(8.0);
                
                if *config_modified {
                    ui.label(RethTheme::warning_text("⚠ Unsaved changes"));
                }
            } else {
                if ui.button("🔄 Reload Config").clicked() {
                    let (_config, _path) = RethConfigManager::load_reth_config();
                    // TODO: Update the main app state with reloaded config
                    // This would need to be handled at the app level
                }
            }
        });
    }
}