use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RethConfig {
    #[serde(default)]
    pub stages: StagesConfig,
    #[serde(default)]
    pub peers: PeersConfig,
    #[serde(default)]
    pub sessions: SessionsConfig,
    #[serde(default)]
    pub prune: PruneConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct StagesConfig {
    #[serde(default)]
    pub era: Option<EraStageConfig>,
    #[serde(default)]
    pub headers: Option<HeadersStageConfig>,
    #[serde(default)]
    pub bodies: Option<BodiesStageConfig>,
    #[serde(default)]
    pub sender_recovery: Option<SenderRecoveryStageConfig>,
    #[serde(default)]
    pub execution: Option<ExecutionStageConfig>,
    #[serde(default)]
    pub prune: Option<PruneStageConfig>,
    #[serde(default)]
    pub account_hashing: Option<AccountHashingStageConfig>,
    #[serde(default)]
    pub storage_hashing: Option<StorageHashingStageConfig>,
    #[serde(default)]
    pub merkle: Option<MerkleStageConfig>,
    #[serde(default)]
    pub transaction_lookup: Option<TransactionLookupStageConfig>,
    #[serde(default)]
    pub index_account_history: Option<IndexAccountHistoryStageConfig>,
    #[serde(default)]
    pub index_storage_history: Option<IndexStorageHistoryStageConfig>,
    #[serde(default)]
    pub etl: Option<EtlStageConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EraStageConfig {
    // Era stage appears to be empty in config
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HeadersStageConfig {
    #[serde(default)]
    pub downloader_max_concurrent_requests: Option<u32>,
    #[serde(default)]
    pub downloader_min_concurrent_requests: Option<u32>,
    #[serde(default)]
    pub downloader_max_buffered_responses: Option<u32>,
    #[serde(default)]
    pub downloader_request_limit: Option<u32>,
    #[serde(default)]
    pub commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BodiesStageConfig {
    #[serde(default)]
    pub downloader_request_limit: Option<u32>,
    #[serde(default)]
    pub downloader_stream_batch_size: Option<u32>,
    #[serde(default)]
    pub downloader_max_buffered_blocks_size_bytes: Option<u64>,
    #[serde(default)]
    pub downloader_min_concurrent_requests: Option<u32>,
    #[serde(default)]
    pub downloader_max_concurrent_requests: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SenderRecoveryStageConfig {
    #[serde(default)]
    pub commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ExecutionStageConfig {
    #[serde(default)]
    pub max_blocks: Option<u64>,
    #[serde(default)]
    pub max_changes: Option<u64>,
    #[serde(default)]
    pub max_cumulative_gas: Option<u64>,
    #[serde(default)]
    pub max_duration: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PruneStageConfig {
    #[serde(default)]
    pub commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AccountHashingStageConfig {
    #[serde(default)]
    pub clean_threshold: Option<u64>,
    #[serde(default)]
    pub commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct StorageHashingStageConfig {
    #[serde(default)]
    pub clean_threshold: Option<u64>,
    #[serde(default)]
    pub commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MerkleStageConfig {
    #[serde(default)]
    pub incremental_threshold: Option<u64>,
    #[serde(default)]
    pub rebuild_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TransactionLookupStageConfig {
    #[serde(default)]
    pub chunk_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct IndexAccountHistoryStageConfig {
    #[serde(default)]
    pub commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct IndexStorageHistoryStageConfig {
    #[serde(default)]
    pub commit_threshold: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EtlStageConfig {
    #[serde(default)]
    pub file_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PeersConfig {
    #[serde(default)]
    pub refill_slots_interval: Option<String>,
    #[serde(default)]
    pub trusted_nodes: Option<Vec<String>>,
    #[serde(default)]
    pub trusted_nodes_only: Option<bool>,
    #[serde(default)]
    pub trusted_nodes_resolution_interval: Option<String>,
    #[serde(default)]
    pub max_backoff_count: Option<u32>,
    #[serde(default)]
    pub ban_duration: Option<String>,
    #[serde(default)]
    pub incoming_ip_throttle_duration: Option<String>,
    #[serde(default)]
    pub connection_info: Option<ConnectionInfoConfig>,
    #[serde(default)]
    pub reputation_weights: Option<ReputationWeightsConfig>,
    #[serde(default)]
    pub backoff_durations: Option<BackoffDurationsConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ConnectionInfoConfig {
    #[serde(default)]
    pub max_outbound: Option<u32>,
    #[serde(default)]
    pub max_inbound: Option<u32>,
    #[serde(default)]
    pub max_concurrent_outbound_dials: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ReputationWeightsConfig {
    #[serde(default)]
    pub bad_message: Option<i32>,
    #[serde(default)]
    pub bad_block: Option<i32>,
    #[serde(default)]
    pub bad_transactions: Option<i32>,
    #[serde(default)]
    pub already_seen_transactions: Option<i32>,
    #[serde(default)]
    pub timeout: Option<i32>,
    #[serde(default)]
    pub bad_protocol: Option<i32>,
    #[serde(default)]
    pub failed_to_connect: Option<i32>,
    #[serde(default)]
    pub dropped: Option<i32>,
    #[serde(default)]
    pub bad_announcement: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct BackoffDurationsConfig {
    #[serde(default)]
    pub low: Option<String>,
    #[serde(default)]
    pub medium: Option<String>,
    #[serde(default)]
    pub high: Option<String>,
    #[serde(default)]
    pub max: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SessionsConfig {
    #[serde(default)]
    pub session_command_buffer: Option<u32>,
    #[serde(default)]
    pub session_event_buffer: Option<u32>,
    #[serde(default)]
    pub limits: Option<SessionLimitsConfig>,
    #[serde(default)]
    pub initial_internal_request_timeout: Option<TimeoutConfig>,
    #[serde(default)]
    pub protocol_breach_request_timeout: Option<TimeoutConfig>,
    #[serde(default)]
    pub pending_session_timeout: Option<TimeoutConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SessionLimitsConfig {
    // This appears to be empty in your config
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TimeoutConfig {
    #[serde(default)]
    pub secs: Option<u64>,
    #[serde(default)]
    pub nanos: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PruneConfig {
    #[serde(default)]
    pub block_interval: Option<u64>,
    #[serde(default)]
    pub segments: Option<PruneSegments>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PruneSegments {
    #[serde(default)]
    pub sender_recovery: Option<String>,
    #[serde(default)]
    pub receipts: Option<PruneReceiptsConfig>,
    #[serde(default)]
    pub account_history: Option<PruneHistoryConfig>,
    #[serde(default)]
    pub storage_history: Option<PruneHistoryConfig>,
    #[serde(default)]
    pub receipts_log_filter: Option<PruneReceiptsLogFilterConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PruneReceiptsConfig {
    #[serde(default)]
    pub distance: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PruneHistoryConfig {
    #[serde(default)]
    pub distance: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct PruneReceiptsLogFilterConfig {
    // This appears to be empty in your config
}

/// Configuration management for Reth node settings
pub struct RethConfigManager;

impl RethConfigManager {
    /// Get platform-specific Reth data directory
    pub fn get_reth_data_dir() -> PathBuf {
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
                PathBuf::from(xdg_data).join("reth")
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
    
    /// Load Reth configuration from reth.toml
    pub fn load_reth_config() -> (RethConfig, Option<PathBuf>) {
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
    
    /// Save Reth configuration to reth.toml
    pub fn save_reth_config(config: &RethConfig, config_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let toml_string = toml::to_string_pretty(config)?;
        std::fs::write(config_path, toml_string)?;
        println!("Saved configuration to: {}", config_path.display());
        Ok(())
    }
}