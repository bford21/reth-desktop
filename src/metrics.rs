use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Maximum number of data points to keep for each metric
const MAX_DATA_POINTS: usize = 60; // 60 points = 1 minute of data at 1 second intervals

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
    pub description: String,
}

impl MetricHistory {
    pub fn new(name: String, unit: String, description: String) -> Self {
        Self {
            name,
            values: VecDeque::with_capacity(MAX_DATA_POINTS),
            unit,
            description,
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
    
    last_poll_time: Option<Instant>,
}

impl RethMetrics {
    pub fn new() -> Self {
        Self {
            sync_progress: MetricHistory::new(
                "Sync Progress".to_string(),
                "%".to_string(),
                "Blockchain synchronization progress".to_string(),
            ),
            peers_connected: MetricHistory::new(
                "Connected Peers".to_string(),
                "peers".to_string(),
                "Number of connected network peers".to_string(),
            ),
            gas_price: MetricHistory::new(
                "Gas Price".to_string(),
                "gwei".to_string(),
                "Current network gas price".to_string(),
            ),
            block_height: MetricHistory::new(
                "Block Height".to_string(),
                "blocks".to_string(),
                "Current blockchain height".to_string(),
            ),
            transactions_per_second: MetricHistory::new(
                "Transactions/sec".to_string(),
                "tx/s".to_string(),
                "Transactions processed per second".to_string(),
            ),
            memory_usage: MetricHistory::new(
                "Memory Usage".to_string(),
                "MB".to_string(),
                "Memory consumption of the node".to_string(),
            ),
            cpu_usage: MetricHistory::new(
                "CPU Usage".to_string(),
                "%".to_string(),
                "CPU utilization of the node".to_string(),
            ),
            disk_io: MetricHistory::new(
                "Disk I/O".to_string(),
                "MB/s".to_string(),
                "Disk read/write throughput".to_string(),
            ),
            last_poll_time: None,
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
        
        // Update sync progress
        if let Some(value) = metrics.get("reth_sync_progress") {
            if let Ok(v) = value.parse::<f64>() {
                self.sync_progress.add_value(v * 100.0); // Convert to percentage
            }
        }
        
        // Update connected peers
        if let Some(value) = metrics.get("reth_p2p_connected_peers") {
            if let Ok(v) = value.parse::<f64>() {
                self.peers_connected.add_value(v);
            }
        }
        
        // Update block height
        if let Some(value) = metrics.get("reth_sync_height") {
            if let Ok(v) = value.parse::<f64>() {
                self.block_height.add_value(v);
            }
        }
        
        // Update memory usage (convert from bytes to MB)
        if let Some(value) = metrics.get("process_resident_memory_bytes") {
            if let Ok(v) = value.parse::<f64>() {
                self.memory_usage.add_value(v / 1_048_576.0); // Convert to MB
            }
        }
        
        // Update CPU usage
        if let Some(value) = metrics.get("process_cpu_seconds_total") {
            if let Ok(v) = value.parse::<f64>() {
                // This is cumulative, so we'd need to calculate the rate
                // For now, we'll use a placeholder
                // TODO: Calculate actual CPU usage rate
            }
        }
        
        // Update transactions per second
        if let Some(value) = metrics.get("reth_transaction_pool_transactions_total") {
            if let Ok(v) = value.parse::<f64>() {
                // This is cumulative, so we'd need to calculate the rate
                // For now, we'll use a placeholder
                // TODO: Calculate actual TPS
            }
        }
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