use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
// Removed unused imports

/// Maximum number of data points to keep for each metric
const MAX_DATA_POINTS: usize = 600; // 600 points = 10 minutes of data at 1 second intervals

#[derive(Debug, Clone)]
pub struct MetricValue {
    pub timestamp: Instant,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct MetricHistory {
    pub name: String,
    pub values: VecDeque<MetricValue>,
    pub unit: String,
}

impl MetricHistory {
    pub fn new(name: String, unit: String) -> Self {
        Self {
            name,
            values: VecDeque::with_capacity(MAX_DATA_POINTS),
            unit,
        }
    }
    
    pub fn add_value(&mut self, value: f64) {
        self.values.push_back(MetricValue {
            timestamp: Instant::now(),
            value,
        });
        
        // Keep only the last MAX_DATA_POINTS
        while self.values.len() > MAX_DATA_POINTS {
            self.values.pop_front();
        }
    }
    
    pub fn get_latest(&self) -> Option<f64> {
        self.values.back().map(|v| v.value)
    }
    
    pub fn get_min_max(&self) -> (f64, f64) {
        if self.values.is_empty() {
            return (0.0, 1.0);
        }
        
        let mut min = f64::MAX;
        let mut max = f64::MIN;
        
        for value in &self.values {
            min = min.min(value.value);
            max = max.max(value.value);
        }
        
        (min, max)
    }
}

#[derive(Debug, Clone)]
pub struct RethMetrics {
    pub sync_progress: MetricHistory,
    pub peers_connected: MetricHistory,
    pub gas_price: MetricHistory,
    pub block_height: MetricHistory,
    pub transactions_per_second: MetricHistory,
    pub memory_usage: MetricHistory,
    pub cpu_usage: MetricHistory,
    pub disk_io: MetricHistory,
    
    // Custom metrics dynamically added by user
    pub custom_metrics: HashMap<String, MetricHistory>,
    
    last_poll_time: Option<Instant>,
}

impl RethMetrics {
    pub fn new() -> Self {
        Self {
            sync_progress: MetricHistory::new(
                "Sync Progress".to_string(),
                "%".to_string(),
            ),
            peers_connected: MetricHistory::new(
                "Connected Peers".to_string(),
                "peers".to_string(),
            ),
            gas_price: MetricHistory::new(
                "Gas Price".to_string(),
                "gwei".to_string(),
            ),
            block_height: MetricHistory::new(
                "Block Height".to_string(),
                "blocks".to_string(),
            ),
            transactions_per_second: MetricHistory::new(
                "TX Pool Size".to_string(),
                "txs".to_string(),
            ),
            memory_usage: MetricHistory::new(
                "Memory Usage".to_string(),
                "MB".to_string(),
            ),
            cpu_usage: MetricHistory::new(
                "CPU Usage".to_string(),
                "%".to_string(),
            ),
            disk_io: MetricHistory::new(
                "Active Downloads".to_string(),
                "blocks".to_string(),
            ),
            custom_metrics: HashMap::new(),
            last_poll_time: None,
        }
    }
    
    pub fn add_custom_metric(&mut self, metric_name: String) {
        if !self.custom_metrics.contains_key(&metric_name) {
            // Try to infer unit from metric name
            let unit = if metric_name.contains("_bytes") {
                "MB"  // Display as MB in the UI
            } else if metric_name.contains("_seconds") {
                "s"
            } else if metric_name.contains("_percent") {
                "%"
            } else if metric_name.contains("_count") || metric_name.contains("_total") {
                "count"
            } else {
                ""
            };
            
            // Create a display name for the metric
            let display_name = metric_name.replace('_', " ")
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str()
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            
            self.custom_metrics.insert(
                metric_name.clone(),
                MetricHistory::new(display_name, unit.to_string())
            );
        }
    }
    
    pub fn should_poll(&self) -> bool {
        match self.last_poll_time {
            None => true,
            Some(last_time) => last_time.elapsed() >= Duration::from_secs(1),
        }
    }
    
    pub fn mark_polled(&mut self) {
        self.last_poll_time = Some(Instant::now());
    }
    
    /// Parse Prometheus-style metrics text and update the metric histories
    pub fn update_from_prometheus_text(&mut self, text: &str) {
        let metrics = parse_prometheus_metrics(text);
        
        // Update connected peers (this metric exists in the endpoint)
        if let Some(value) = metrics.get("reth_network_connected_peers") {
            if let Ok(v) = value.parse::<f64>() {
                self.peers_connected.add_value(v);
            }
        }
        
        // Update block height using canonical chain height
        if let Some(value) = metrics.get("reth_blockchain_tree_canonical_chain_height") {
            if let Ok(v) = value.parse::<f64>() {
                self.block_height.add_value(v);
            }
        }
        
        // Update memory usage (convert from bytes to MB) - this metric exists
        if let Some(value) = metrics.get("reth_process_resident_memory_bytes") {
            if let Ok(v) = value.parse::<f64>() {
                self.memory_usage.add_value(v / 1_048_576.0); // Convert to MB
            }
        }
        
        // Calculate sync progress based on multiple indicators
        let mut is_syncing = false;
        
        // Check gas per second (active sync indicator)
        if let Some(value) = metrics.get("reth_sync_execution_gas_per_second") {
            if let Ok(v) = value.parse::<f64>() {
                if v > 0.0 {
                    is_syncing = true;
                }
            }
        }
        
        // Check active block downloads
        if let Some(value) = metrics.get("reth_consensus_engine_beacon_active_block_downloads") {
            if let Ok(v) = value.parse::<f64>() {
                if v > 0.0 {
                    is_syncing = true;
                }
            }
        }
        
        // If we have block height, we can show it instead of a percentage
        // For now, show syncing status rather than a misleading percentage
        if is_syncing {
            // Don't show 100% when syncing, show a value that indicates ongoing sync
            self.sync_progress.add_value(0.0); // Will show as "Syncing"
        } else if self.block_height.get_latest().unwrap_or(0.0) > 0.0 {
            // Only show 100% if we have a block height and no sync activity
            self.sync_progress.add_value(100.0);
        }
        
        // Update CPU usage (using the correct metric name)
        if let Some(value) = metrics.get("reth_process_cpu_seconds_total") {
            if let Ok(_v) = value.parse::<f64>() {
                // This is cumulative, so we'd need to calculate the rate
                // For now, we'll use a placeholder
                // TODO: Calculate actual CPU usage rate
            }
        }
        
        // For transactions per second, we can use a different approach
        // Look at the transaction pool size as an indicator
        if let Some(value) = metrics.get("reth_transaction_pool_transactions") {
            if let Ok(v) = value.parse::<f64>() {
                // This shows current pool size, not TPS, but it's useful
                self.transactions_per_second.add_value(v);
            }
        }
        
        // Update gas price if available (useful for node operators)
        // Note: Gas price might come from a different metric or RPC call
        // For now, we'll use a placeholder since it's not in the metrics endpoint
        
        // Track active downloads (useful during sync)
        if let Some(value) = metrics.get("reth_consensus_engine_beacon_active_block_downloads") {
            if let Ok(v) = value.parse::<f64>() {
                // Could be used to show sync activity
                self.disk_io.add_value(v); // Repurpose disk_io for active downloads
            }
        }
        
        // Update custom metrics
        for (metric_name, metric_history) in &mut self.custom_metrics {
            if let Some(value) = metrics.get(metric_name) {
                if let Ok(v) = value.parse::<f64>() {
                    // Convert bytes to MB if it's a bytes metric
                    let final_value = if metric_history.unit == "MB" && metric_name.contains("_bytes") {
                        v / 1_048_576.0 // Convert to MB
                    } else {
                        v
                    };
                    metric_history.add_value(final_value);
                }
            }
        }
    }
    
    /// Get all available metric names from the prometheus text
    pub fn get_available_metrics(text: &str) -> Vec<String> {
        let metrics = parse_prometheus_metrics(text);
        let mut metric_names: Vec<String> = metrics.keys().cloned().collect();
        metric_names.sort();
        metric_names
    }
}

/// Parse Prometheus-style metrics text into a HashMap
fn parse_prometheus_metrics(text: &str) -> HashMap<String, String> {
    let mut metrics = HashMap::new();
    
    for line in text.lines() {
        // Skip comments and empty lines
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        
        // Parse metric lines (format: metric_name{labels} value)
        // or simple format: metric_name value
        if let Some(space_pos) = line.rfind(' ') {
            let (name_part, value) = line.split_at(space_pos);
            let value = value.trim();
            
            // Extract metric name (before any labels)
            let metric_name = if let Some(brace_pos) = name_part.find('{') {
                &name_part[..brace_pos]
            } else {
                name_part
            }.trim();
            
            metrics.insert(metric_name.to_string(), value.to_string());
        }
    }
    
    metrics
}

/// Fetch metrics from the Reth metrics endpoint
pub async fn fetch_metrics(endpoint: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let response = reqwest::get(endpoint).await?;
    let text = response.text().await?;
    Ok(text)
}