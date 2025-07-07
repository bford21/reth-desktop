use std::collections::VecDeque;
use std::process::{Command, Stdio, Child};
use std::io::{BufRead, BufReader, SeekFrom, Seek};
use std::sync::{Arc, Mutex};
use std::thread;
use std::path::PathBuf;
use std::fs::File;
use tokio::sync::mpsc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliOption {
    pub name: String,
    pub description: String,
    pub takes_value: bool,
    pub value_name: Option<String>,
    pub possible_values: Option<Vec<String>>,
    pub accepts_multiple: bool,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: String,
    pub content: String,
    pub level: LogLevel,
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
    Trace,
}

impl LogLevel {
    fn from_content(content: &str) -> Self {
        let lower = content.to_lowercase();
        if lower.contains("error") || lower.contains("err") {
            LogLevel::Error
        } else if lower.contains("warn") || lower.contains("warning") {
            LogLevel::Warn
        } else if lower.contains("debug") {
            LogLevel::Debug
        } else if lower.contains("trace") {
            LogLevel::Trace
        } else {
            LogLevel::Info
        }
    }
}

impl LogLine {
    /// Remove Reth's timestamp from the log content
    /// Reth timestamps follow the pattern: 2025-07-03T19:20:27.1514252
    fn clean_reth_timestamp(content: &str) -> String {
        // Look for the pattern YYYY-MM-DDTHH:MM:SS.microseconds
        // We'll find the first occurrence and remove everything up to the first space after it
        
        // Find a pattern that looks like: 4 digits, dash, 2 digits, dash, 2 digits, T, etc.
        let mut chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        
        // Look for timestamp pattern starting position
        for i in 0..len.saturating_sub(19) { // minimum timestamp length is about 19 chars
            // Check if we have YYYY-MM-DDTHH:MM:SS pattern starting at position i
            if i + 19 < len &&
               chars[i].is_ascii_digit() && chars[i+1].is_ascii_digit() && 
               chars[i+2].is_ascii_digit() && chars[i+3].is_ascii_digit() &&
               chars[i+4] == '-' &&
               chars[i+5].is_ascii_digit() && chars[i+6].is_ascii_digit() &&
               chars[i+7] == '-' &&
               chars[i+8].is_ascii_digit() && chars[i+9].is_ascii_digit() &&
               chars[i+10] == 'T' &&
               chars[i+11].is_ascii_digit() && chars[i+12].is_ascii_digit() &&
               chars[i+13] == ':' &&
               chars[i+14].is_ascii_digit() && chars[i+15].is_ascii_digit() &&
               chars[i+16] == ':' &&
               chars[i+17].is_ascii_digit() && chars[i+18].is_ascii_digit() {
                
                // Found timestamp start, now find where it ends (look for space or next non-timestamp char)
                let mut end_pos = i + 19;
                while end_pos < len && (chars[end_pos].is_ascii_digit() || chars[end_pos] == '.') {
                    end_pos += 1;
                }
                
                // Check for timezone indicator (Z or +/-offset)
                if end_pos < len && (chars[end_pos] == 'Z' || chars[end_pos] == '+' || chars[end_pos] == '-') {
                    end_pos += 1;
                    // If it's + or -, skip the offset (e.g., +00:00)
                    if end_pos > 0 && (chars[end_pos-1] == '+' || chars[end_pos-1] == '-') {
                        while end_pos < len && (chars[end_pos].is_ascii_digit() || chars[end_pos] == ':') {
                            end_pos += 1;
                        }
                    }
                }
                
                // Skip any trailing spaces
                while end_pos < len && chars[end_pos].is_whitespace() {
                    end_pos += 1;
                }
                
                // Return the string with the timestamp portion removed
                let before = &content[0..i];
                let after = &content[end_pos..];
                return format!("{}{}", before, after).trim().to_string();
            }
        }
        
        // If no timestamp pattern found, return original content
        content.to_string()
    }
}

pub struct RethNode {
    process: Option<Child>,
    log_buffer: Arc<Mutex<VecDeque<LogLine>>>,
    log_receiver: Option<mpsc::UnboundedReceiver<LogLine>>,
    is_running: bool,
    external_log_path: Option<PathBuf>,
    last_external_check: std::time::Instant,
    launch_command: Option<Vec<String>>,
}

impl RethNode {
    pub fn new() -> Self {
        Self {
            process: None,
            log_buffer: Arc::new(Mutex::new(VecDeque::new())),
            log_receiver: None,
            is_running: false,
            external_log_path: None,
            last_external_check: std::time::Instant::now(),
            launch_command: None,
        }
    }

    pub fn start(&mut self, reth_path: &str, custom_args: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_running {
            return Err("Reth node is already running".into());
        }

        // Create channel for log communication
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        self.log_receiver = Some(log_receiver);

        // Determine log directory path based on platform
        let log_dir = Self::get_default_log_directory();
        
        // Ensure log directory exists
        if let Some(ref log_path) = log_dir {
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        
        // Build the command and track it for display
        let mut command = Command::new(reth_path);
        let mut command_parts = vec![
            reth_path.to_string(), 
            "node".to_string(), 
            "--full".to_string(),
            "--metrics".to_string(),
            "127.0.0.1:9001".to_string(), 
            "--log.stdout.format".to_string(), 
            "terminal".to_string()
        ];
        
        command
            .arg("node")
            .arg("--full")
            .arg("--metrics")
            .arg("127.0.0.1:9001")
            .arg("--log.stdout.format")
            .arg("terminal");
        
        // Add file logging configuration if we have a log directory
        if let Some(log_path) = &log_dir {
            println!("Configuring Reth to log to: {}", log_path.display());
            command
                .arg("--log.file.directory")
                .arg(log_path) // Directory path
                .arg("--log.file.format")
                .arg("terminal") // Use terminal format for readability
                .arg("--log.file.filter")
                .arg("info") // Log info level and above to file
                .arg("--log.file.max-size")
                .arg("50") // 50 MB max size per log file
                .arg("--log.file.max-files")
                .arg("3"); // Keep up to 3 log files
            
            // Add to command parts for display
            command_parts.extend(vec![
                "--log.file.directory".to_string(),
                log_path.display().to_string(),
                "--log.file.format".to_string(),
                "terminal".to_string(),
                "--log.file.filter".to_string(),
                "info".to_string(),
                "--log.file.max-size".to_string(),
                "50".to_string(),
                "--log.file.max-files".to_string(),
                "3".to_string(),
            ]);
                
            // Store the log directory path - we'll find the actual log file later
            // Reth creates files with date patterns like reth-2024-01-15-20.log
            self.external_log_path = Some(log_path.clone());
        }
        
        // Add custom arguments from settings
        println!("Adding {} custom arguments:", custom_args.len());
        for arg in custom_args {
            println!("  Adding custom arg: {}", arg);
            command.arg(arg);
            command_parts.push(arg.clone());
        }
        
        // Store the command parts for display
        self.launch_command = Some(command_parts);
        
        // Print the full command for debugging
        println!("Final command: {:?}", command);
        
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                eprintln!("Failed to spawn Reth process: {}", e);
                e
            })?;

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let sender = log_sender.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let cleaned_content = LogLine::clean_reth_timestamp(&line);
                        let log_line = LogLine {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            content: cleaned_content.clone(),
                            level: LogLevel::from_content(&cleaned_content),
                        };
                        if sender.send(log_line).is_err() {
                            break;
                        }
                    }
                }
            });
        }

        // Capture stderr
        if let Some(stderr) = child.stderr.take() {
            let sender = log_sender;
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let cleaned_content = LogLine::clean_reth_timestamp(&line);
                        let log_line = LogLine {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            content: cleaned_content,
                            level: LogLevel::Error,
                        };
                        if sender.send(log_line).is_err() {
                            break;
                        }
                    }
                }
            });
        }

        self.process = Some(child);
        self.is_running = true;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(mut process) = self.process.take() {
            process.kill()?;
            process.wait()?;
            self.is_running = false;
            // Clear the log path and command for managed processes
            self.external_log_path = None;
            self.launch_command = None;
        } else {
            // For external processes, just reset the running state
            self.is_running = false;
            // Clear the launch command when disconnecting
            self.launch_command = None;
            // Keep the log path for external processes in case we reconnect
        }
        
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Check if we're monitoring an external process (not one we started)
    pub fn is_monitoring_external(&self) -> bool {
        self.is_running && self.process.is_none()
    }

    /// Get the path of the external log file being monitored
    pub fn get_external_log_path(&self) -> Option<&PathBuf> {
        self.external_log_path.as_ref()
    }
    
    /// Get the command used to launch the Reth process
    pub fn get_launch_command(&self) -> Option<&Vec<String>> {
        self.launch_command.as_ref()
    }

    pub fn get_logs(&mut self) -> Vec<LogLine> {
        let mut logs = Vec::new();
        
        // Process any new logs from the receiver
        if let Some(receiver) = &mut self.log_receiver {
            while let Ok(log_line) = receiver.try_recv() {
                logs.push(log_line.clone());
                
                // Add to buffer (keep last 1000 lines)
                let mut buffer = self.log_buffer.lock().unwrap();
                buffer.push_back(log_line);
                if buffer.len() > 1000 {
                    buffer.pop_front();
                }
            }
        }
        
        logs
    }

    pub fn get_all_logs(&self) -> Vec<LogLine> {
        let buffer = self.log_buffer.lock().unwrap();
        buffer.iter().cloned().collect()
    }

    pub fn check_process_status(&mut self) {
        if let Some(process) = &mut self.process {
            // Check our own managed process
            match process.try_wait() {
                Ok(Some(_)) => {
                    self.is_running = false;
                    self.process = None;
                    self.external_log_path = None;
                    self.launch_command = None;
                }
                Ok(None) => {
                    // Process is still running
                }
                Err(_) => {
                    self.is_running = false;
                    self.process = None;
                    self.external_log_path = None;
                    self.launch_command = None;
                }
            }
        } else if self.is_running {
            // We're monitoring an external process - check if it's still running
            // Only check every 2 seconds to avoid excessive system calls
            let now = std::time::Instant::now();
            if now.duration_since(self.last_external_check).as_secs() >= 2 {
                self.last_external_check = now;
                if !Self::detect_existing_reth_process() {
                    self.is_running = false;
                    self.external_log_path = None;
                    self.launch_command = None;
                    println!("External Reth process has stopped");
                }
            }
        }
    }

    /// Check if any Reth process is currently running on the system
    /// Uses port checking as a more reliable method than process name matching
    pub fn detect_existing_reth_process() -> bool {
        // Check if Reth's default RPC port (8545) is listening
        // This is more reliable than process name matching
        let rpc_port = Self::is_port_listening(8545);
        let ws_port = Self::is_port_listening(8546);
        let engine_port = Self::is_port_listening(8551);
        
        let is_running = rpc_port || ws_port || engine_port;
        
        if is_running {
            println!("Detected Reth running - RPC:{} WS:{} Engine:{}", rpc_port, ws_port, engine_port);
        }
        
        is_running
    }
    
    /// Detect the command line of external Reth processes
    fn detect_external_reth_command() -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            // First try using lsof to find process using Reth ports
            let ports = vec![8545, 8546, 8551];
            for port in ports {
                match std::process::Command::new("lsof")
                    .arg("-ti")
                    .arg(format!(":{}", port))
                    .output()
                {
                    Ok(output) => {
                        let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if !pid_str.is_empty() {
                            // Found a PID, now get its command line
                            match std::process::Command::new("ps")
                                .arg("-p")
                                .arg(&pid_str)
                                .arg("-o")
                                .arg("command=")
                                .output()
                            {
                                Ok(cmd_output) => {
                                    let cmd = String::from_utf8_lossy(&cmd_output.stdout).trim().to_string();
                                    if !cmd.is_empty() && cmd.to_lowercase().contains("reth") {
                                        println!("Found Reth process via port {}: {}", port, cmd);
                                        return Some(cmd);
                                    }
                                }
                                Err(_) => {}
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
            
            // Fallback to ps search
            match std::process::Command::new("ps")
                .arg("-axo")
                .arg("command")
                .output()
            {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    println!("Searching for Reth process in ps output...");
                    for line in output_str.lines() {
                        // Look for lines containing reth executable
                        // This could be /path/to/reth, ./reth, or just reth
                        let line_lower = line.to_lowercase();
                        if (line_lower.contains("/reth") || line_lower.starts_with("reth")) 
                            && !line_lower.contains("reth-desktop") 
                            && !line.contains("grep") {
                            // Further check if it's actually a reth node command
                            if line.contains("node") || line.contains("--") {
                                let cleaned = line.trim().to_string();
                                println!("Found external Reth command: {}", cleaned);
                                return Some(cleaned);
                            }
                        }
                    }
                    println!("No external Reth process found in ps output");
                }
                Err(e) => {
                    println!("Failed to run ps command: {}", e);
                }
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            // On Linux, use ps with different args
            match std::process::Command::new("ps")
                .arg("-eo")
                .arg("command")
                .arg("--no-headers")
                .output()
            {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    println!("Searching for Reth process in ps output...");
                    for line in output_str.lines() {
                        let line_lower = line.to_lowercase();
                        if (line_lower.contains("/reth") || line_lower.starts_with("reth")) 
                            && !line_lower.contains("reth-desktop") 
                            && !line.contains("grep") {
                            if line.contains("node") || line.contains("--") {
                                let cleaned = line.trim().to_string();
                                println!("Found external Reth command: {}", cleaned);
                                return Some(cleaned);
                            }
                        }
                    }
                    println!("No external Reth process found in ps output");
                }
                Err(e) => {
                    println!("Failed to run ps command: {}", e);
                }
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            // On Windows, use wmic or tasklist
            match std::process::Command::new("wmic")
                .arg("process")
                .arg("where")
                .arg("name='reth.exe'")
                .arg("get")
                .arg("commandline")
                .arg("/format:value")
                .output()
            {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output.stdout);
                    for line in output_str.lines() {
                        if line.starts_with("CommandLine=") && line.contains("node") {
                            let command = line.strip_prefix("CommandLine=").unwrap_or(line);
                            let cleaned = command.trim().to_string();
                            println!("Found external Reth command: {}", cleaned);
                            return Some(cleaned);
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to run wmic command: {}", e);
                }
            }
        }
        
        None
    }

    /// Check if a specific port is listening (indicates Reth is running)
    fn is_port_listening(port: u16) -> bool {
        use std::net::{TcpStream, SocketAddr};
        use std::time::Duration;

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        
        // Try to connect with a very short timeout
        match TcpStream::connect_timeout(&addr, Duration::from_millis(100)) {
            Ok(_) => {
                // Port is listening
                true
            }
            Err(_) => {
                // Port is not listening or connection failed
                false
            }
        }
    }

    /// Get the platform-specific Reth log file path
    /// According to Reth docs, logs go to <CACHE_DIR>/logs by default
    fn get_reth_log_path() -> Option<PathBuf> {
        // First check cache directory (where Reth actually puts logs by default)
        if let Some(cache_dir) = dirs::cache_dir() {
            let cache_logs_base = cache_dir.join("reth").join("logs");
            
            // Try mainnet directory first (most common)
            let cache_logs_mainnet_path = cache_logs_base.join("mainnet");
            println!("Checking Reth cache logs mainnet directory: {}", cache_logs_mainnet_path.display());
            if let Some(log_file) = Self::find_log_files_in_directory(&cache_logs_mainnet_path) {
                return Some(log_file);
            }
            
            // Then try the general logs directory
            println!("Checking Reth cache logs directory: {}", cache_logs_base.display());
            if let Some(log_file) = Self::find_log_files_in_directory(&cache_logs_base) {
                return Some(log_file);
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            // Also check the data directory for backward compatibility
            let data_base_path = dirs::home_dir()?
                .join("Library")
                .join("Application Support")
                .join("reth")
                .join("mainnet")
                .join("logs");
            
            println!("Checking Reth data logs directory: {}", data_base_path.display());
            
            if let Some(log_file) = Self::find_log_files_in_directory(&data_base_path) {
                return Some(log_file);
            }
            
            // Check other common macOS cache locations
            if let Some(cache_dir) = dirs::cache_dir() {
                let alt_path = cache_dir.join("reth").join("mainnet").join("logs");
                println!("Checking alternative cache path: {}", alt_path.display());
                if let Some(log_file) = Self::find_log_files_in_directory(&alt_path) {
                    return Some(log_file);
                }
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            // Check cache directory first (default for Reth)
            if let Some(cache_dir) = dirs::cache_dir() {
                // Try network-specific log directory first
                let cache_logs_mainnet_path = cache_dir.join("reth").join("logs").join("mainnet");
                println!("Checking Linux cache logs mainnet directory: {}", cache_logs_mainnet_path.display());
                if let Some(log_file) = Self::find_log_files_in_directory(&cache_logs_mainnet_path) {
                    return Some(log_file);
                }
                
                let cache_logs_path = cache_dir.join("reth").join("logs");
                println!("Checking Linux cache logs directory: {}", cache_logs_path.display());
                if let Some(log_file) = Self::find_log_files_in_directory(&cache_logs_path) {
                    return Some(log_file);
                }
            }
            
            // Check XDG data directory
            if let Some(data_dir) = dirs::data_dir() {
                let data_logs_path = data_dir.join("reth").join("mainnet").join("logs");
                println!("Checking Linux data logs directory: {}", data_logs_path.display());
                if let Some(log_file) = Self::find_log_files_in_directory(&data_logs_path) {
                    return Some(log_file);
                }
            }
            
            // Check home directory
            let home_logs_path = dirs::home_dir()?
                .join(".local")
                .join("share")
                .join("reth")
                .join("mainnet")
                .join("logs");
            println!("Checking Linux home logs directory: {}", home_logs_path.display());
            if let Some(log_file) = Self::find_log_files_in_directory(&home_logs_path) {
                return Some(log_file);
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            // Check cache directory first
            if let Some(cache_dir) = dirs::cache_dir() {
                // Try network-specific log directory first
                let cache_logs_mainnet_path = cache_dir.join("reth").join("logs").join("mainnet");
                println!("Checking Windows cache logs mainnet directory: {}", cache_logs_mainnet_path.display());
                if let Some(log_file) = Self::find_log_files_in_directory(&cache_logs_mainnet_path) {
                    return Some(log_file);
                }
                
                let cache_logs_path = cache_dir.join("reth").join("logs");
                println!("Checking Windows cache logs directory: {}", cache_logs_path.display());
                if let Some(log_file) = Self::find_log_files_in_directory(&cache_logs_path) {
                    return Some(log_file);
                }
            }
            
            // Check data directory
            let data_logs_path = dirs::data_dir()?
                .join("reth")
                .join("mainnet")
                .join("logs");
            println!("Checking Windows data logs directory: {}", data_logs_path.display());
            if let Some(log_file) = Self::find_log_files_in_directory(&data_logs_path) {
                return Some(log_file);
            }
        }
        
        println!("No Reth log files found in any checked directories");
        None
    }

    /// Get the default log directory where we'll tell Reth to write logs
    fn get_default_log_directory() -> Option<PathBuf> {
        // Use cache directory as per Reth's default behavior
        // Don't include mainnet here - let Reth create its own network subdirectory
        if let Some(cache_dir) = dirs::cache_dir() {
            let log_dir = cache_dir.join("reth").join("logs");
            return Some(log_dir);
        }
        
        // Fallback to platform-specific paths
        #[cfg(target_os = "macos")]
        {
            if let Some(home) = dirs::home_dir() {
                return Some(home.join("Library").join("Caches").join("reth").join("logs"));
            }
        }
        
        #[cfg(target_os = "linux")]
        {
            if let Some(home) = dirs::home_dir() {
                return Some(home.join(".cache").join("reth").join("logs"));
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            if let Some(local_data) = dirs::data_local_dir() {
                return Some(local_data.join("reth").join("logs"));
            }
        }
        
        None
    }

    /// Helper function to find log files in a directory
    /// Looks for various log file patterns that Reth might use
    fn find_log_files_in_directory(dir_path: &PathBuf) -> Option<PathBuf> {
        if !dir_path.exists() {
            println!("Directory does not exist: {}", dir_path.display());
            return None;
        }
        
        println!("Searching for log files in: {}", dir_path.display());
        
        // Common log file names that Reth might use
        // Reth creates either reth.log or date-based files like reth-2024-01-15-20.log
        let log_patterns = vec![
            "reth.log",     // Primary log file
            "debug.log", 
            "info.log",
            "node.log",
            "reth_node.log"
        ];
        
        // First try exact matches
        for pattern in &log_patterns {
            let log_path = dir_path.join(pattern);
            if log_path.exists() {
                println!("Found exact match log file: {}", log_path.display());
                return Some(log_path);
            }
        }
        
        // If no exact matches, look for any .log files, prioritizing by modification time
        if let Ok(entries) = std::fs::read_dir(dir_path) {
            println!("Directory contents:");
            let mut log_files = Vec::new();
            
            for entry in entries {
                if let Ok(entry) = entry {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy();
                    println!("  - {}", file_name_str);
                    
                    // Collect all .log files with their metadata
                    if file_name_str.ends_with(".log") {
                        if let Ok(metadata) = entry.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                log_files.push((entry.path(), file_name_str.to_string(), modified));
                            }
                        }
                    }
                }
            }
            
            // Sort by priority: 1) reth.log first, 2) reth-* pattern files, 3) most recent by modification time
            log_files.sort_by(|a, b| {
                // Prioritize exact "reth.log" first
                let a_exact = a.1 == "reth.log";
                let b_exact = b.1 == "reth.log";
                
                if a_exact && !b_exact {
                    return std::cmp::Ordering::Less;
                }
                if !a_exact && b_exact {
                    return std::cmp::Ordering::Greater;
                }
                
                // Then prioritize files that start with "reth-"
                let a_is_reth = a.1.starts_with("reth-");
                let b_is_reth = b.1.starts_with("reth-");
                
                match (a_is_reth, b_is_reth) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => {
                        // Same priority level - sort by modification time (most recent first)
                        b.2.cmp(&a.2)
                    }
                }
            });
            
            // Return the first (highest priority) log file
            if let Some((log_path, file_name, modified)) = log_files.first() {
                println!("Found log file: {} (modified: {:?}, selected from {} total log files)", file_name, modified, log_files.len());
                return Some(log_path.clone());
            }
        }
        
        println!("No log files found in directory");
        None
    }

    /// Start tailing a log file for external process monitoring
    fn start_log_file_monitoring(&mut self, log_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Check if log_path is a directory or a file
        let actual_log_file = if log_path.is_dir() {
            // Find the actual log file in the directory
            println!("Log path is a directory, searching for log files in: {}", log_path.display());
            match Self::find_log_files_in_directory(&log_path) {
                Some(file) => {
                    println!("Found log file in directory: {}", file.display());
                    file
                }
                None => {
                    return Err("No log files found in directory".into());
                }
            }
        } else {
            log_path
        };
        
        println!("Starting log file monitoring for: {}", actual_log_file.display());
        
        let (sender, receiver) = mpsc::unbounded_channel::<LogLine>();
        self.log_receiver = Some(receiver);
        
        let log_buffer = self.log_buffer.clone();
        
        // Read last 50 lines of the log file to populate initial buffer
        match Self::read_recent_log_lines(&actual_log_file, 50) {
            Ok(recent_lines) => {
                println!("Read {} recent log lines", recent_lines.len());
                let mut buffer = log_buffer.lock().unwrap();
                for line in recent_lines {
                    buffer.push_back(line);
                }
            }
            Err(e) => {
                println!("Failed to read recent log lines: {}", e);
            }
        }
        
        let log_file_for_thread = actual_log_file.clone();
        thread::spawn(move || {
            println!("Log tailing thread started for: {}", log_file_for_thread.display());
            if let Err(e) = Self::tail_log_file(log_file_for_thread, sender, log_buffer) {
                eprintln!("Error tailing log file: {}", e);
            }
        });
        
        // Update the stored log path to the actual file path
        self.external_log_path = Some(actual_log_file);
        
        Ok(())
    }

    /// Read the last N lines from a log file
    fn read_recent_log_lines(log_path: &PathBuf, count: usize) -> Result<Vec<LogLine>, Box<dyn std::error::Error>> {
        println!("Reading recent lines from: {}", log_path.display());
        let file = File::open(log_path)?;
        let reader = BufReader::new(file);
        
        let mut lines: VecDeque<String> = VecDeque::new();
        let mut total_lines = 0;
        
        // Read all lines and keep only the last N
        for line in reader.lines() {
            if let Ok(line) = line {
                lines.push_back(line);
                total_lines += 1;
                if lines.len() > count {
                    lines.pop_front();
                }
            }
        }
        
        println!("Read {} total lines, keeping {} recent lines", total_lines, lines.len());
        
        // Convert to LogLine structs
        let mut log_lines = Vec::new();
        for line in lines {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let cleaned_content = LogLine::clean_reth_timestamp(trimmed);
                log_lines.push(LogLine {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    content: cleaned_content.clone(),
                    level: LogLevel::from_content(&cleaned_content),
                });
            }
        }
        
        println!("Converted to {} LogLine structs", log_lines.len());
        Ok(log_lines)
    }

    /// Tail a log file and send new lines to the channel
    fn tail_log_file(
        log_path: PathBuf,
        sender: mpsc::UnboundedSender<LogLine>,
        log_buffer: Arc<Mutex<VecDeque<LogLine>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::open(&log_path)?;
        
        // Seek to end of file to only read new content
        file.seek(SeekFrom::End(0))?;
        
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    // No new data, sleep briefly and try again
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
                Ok(_) => {
                    // Process the new line
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        let cleaned_content = LogLine::clean_reth_timestamp(trimmed);
                        let log_line = LogLine {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            content: cleaned_content.clone(),
                            level: LogLevel::from_content(&cleaned_content),
                        };
                        
                        // Add to buffer
                        {
                            let mut buffer = log_buffer.lock().unwrap();
                            buffer.push_back(log_line.clone());
                            if buffer.len() > 1000 {
                                buffer.pop_front();
                            }
                        }
                        
                        // Send to receiver
                        if sender.send(log_line).is_err() {
                            break; // Channel closed
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading log file: {}", e);
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            }
        }
        
        Ok(())
    }

    /// Connect to and start monitoring an existing Reth process
    pub fn connect_to_existing_process(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if Self::detect_existing_reth_process() {
            // Create a dummy "process" state to indicate we're monitoring an external process
            self.is_running = true;
            self.process = None; // We don't own this process
            
            // Try to detect the command used to launch the external process
            if let Some(cmd_string) = Self::detect_external_reth_command() {
                println!("Detected external Reth command: {}", cmd_string);
                // Parse the command string into parts
                let parts: Vec<String> = cmd_string.split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                self.launch_command = Some(parts);
            } else {
                println!("Connected to external Reth process (command detection failed)");
            }
            
            // Try to find and tail Reth's log file
            if let Some(log_path) = Self::get_reth_log_path() {
                println!("Found Reth log file: {}", log_path.display());
                self.external_log_path = Some(log_path.clone());
                self.start_log_file_monitoring(log_path)?;
                println!("Started monitoring external Reth process with log tailing");
            } else {
                println!("Connected to existing Reth process (no log file found)");
                println!("Note: Reth may not be configured to write log files.");
                println!("To enable file logging, restart Reth with: reth node --log.file.directory <path>");
                
                // Set a flag to show helpful message in UI
                self.external_log_path = None;
            }
            
            Ok(())
        } else {
            Err("No existing Reth process found".into())
        }
    }
    
    /// Parse available CLI options from reth node --help
    pub fn get_available_cli_options(reth_path: &str) -> Vec<CliOption> {
        let mut options = Vec::new();
        
        // Run reth node --help to get available options
        match Command::new(reth_path)
            .arg("node")
            .arg("--help")
            .output()
        {
            Ok(output) => {
                let help_text = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = help_text.lines().collect();
                
                let mut i = 0;
                while i < lines.len() {
                    let line = lines[i].trim();
                    
                    // Look for lines starting with spaces followed by -- (long options)
                    if line.trim_start().starts_with("--") && !line.trim_start().starts_with("---") {
                        // Parse the option line
                        // Format is: --option-name <VALUE>
                        let trimmed = line.trim();
                        
                        // Split by first space to separate option from value placeholder
                        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                        if parts.len() >= 1 {
                            let option_name = parts[0].to_string();
                            let mut takes_value = false;
                            let mut value_name = None;
                            
                            // Check if there's a value part
                            if parts.len() > 1 {
                                let rest = parts[1].trim();
                                if rest.starts_with('<') && rest.contains('>') {
                                    // Extract value name
                                    if let Some(end) = rest.find('>') {
                                        value_name = Some(rest[1..end].to_string());
                                        takes_value = true;
                                    }
                                }
                            }
                            
                            // Some known flags that don't take values
                            let flag_patterns = vec![
                                "--help", "--version", "--full", "--http", "--ws", 
                                "--disable-", "--enable-", "--with-", "--without-"
                            ];
                            
                            // If it's a known flag pattern and we didn't detect a value, mark as flag
                            if !takes_value {
                                for pattern in &flag_patterns {
                                    if option_name.starts_with(pattern) || option_name == *pattern {
                                        takes_value = false;
                                        break;
                                    }
                                }
                            }
                            
                            // Get description from the next line and look for possible values
                            let mut description = String::new();
                            let mut possible_values = None;
                            let mut accepts_multiple = false;
                            
                            // Check next few lines for description and possible values
                            let mut j = i + 1;
                            while j < lines.len() {
                                let line = lines[j].trim();
                                
                                // Stop if we hit another option
                                if line.starts_with("--") {
                                    break;
                                }
                                
                                // Skip empty lines
                                if line.is_empty() {
                                    j += 1;
                                    continue;
                                }
                                
                                // Check for possible values pattern: [possible values: ...]
                                if line.contains("[possible values:") {
                                    // Extract possible values
                                    if let Some(start) = line.find("[possible values:") {
                                        let values_part = &line[start + 17..]; // Skip "[possible values:"
                                        if let Some(end) = values_part.find(']') {
                                            let values_str = &values_part[..end];
                                            let values: Vec<String> = values_str
                                                .split(',')
                                                .map(|s| s.trim().to_string())
                                                .filter(|s| !s.is_empty())
                                                .collect();
                                            if !values.is_empty() {
                                                possible_values = Some(values);
                                            }
                                        }
                                    }
                                } else if description.is_empty() {
                                    // This is the description line
                                    description = line.to_string();
                                }
                                
                                j += 1;
                                
                                // Only look at a few lines to avoid going too far
                                if j > i + 5 {
                                    break;
                                }
                            }
                            
                            // Check if this parameter accepts multiple values
                            // Look for indicators in the description or parameter name
                            let desc_lower = description.to_lowercase();
                            accepts_multiple = desc_lower.contains("comma-separated") || 
                                             desc_lower.contains("comma separated") ||
                                             desc_lower.contains("list of") ||
                                             desc_lower.contains("multiple") ||
                                             option_name.contains(".api") ||
                                             option_name.contains(".namespaces");
                            
                            // Some options we want to skip or are already included
                            let skip_options = vec![
                                "--help", "--version", "--full", 
                                "--log.stdout.format", "--log.file.directory",
                                "--log.file.format", "--log.file.filter",
                                "--log.file.max-size", "--log.file.max-files"
                            ];
                            
                            if !skip_options.contains(&option_name.as_str()) && !description.is_empty() {
                                options.push(CliOption {
                                    name: option_name,
                                    description,
                                    takes_value,
                                    value_name,
                                    possible_values,
                                    accepts_multiple,
                                });
                            }
                        }
                    }
                    i += 1;
                }
            }
            Err(e) => {
                println!("Failed to get CLI options: {}", e);
            }
        }
        
        // Add some common options that might be useful
        if options.is_empty() {
            // Fallback options if parsing fails
            options.extend(vec![
                CliOption {
                    name: "--datadir".to_string(),
                    description: "The path to the data directory".to_string(),
                    takes_value: true,
                    value_name: Some("PATH".to_string()),
                    possible_values: None,
                    accepts_multiple: false,
                },
                CliOption {
                    name: "--port".to_string(),
                    description: "The port to listen on".to_string(),
                    takes_value: true,
                    value_name: Some("PORT".to_string()),
                    possible_values: None,
                    accepts_multiple: false,
                },
                CliOption {
                    name: "--http".to_string(),
                    description: "Enable the HTTP RPC server".to_string(),
                    takes_value: false,
                    value_name: None,
                    possible_values: None,
                    accepts_multiple: false,
                },
                CliOption {
                    name: "--ws".to_string(),
                    description: "Enable the WebSocket RPC server".to_string(),
                    takes_value: false,
                    value_name: None,
                    possible_values: None,
                    accepts_multiple: false,
                },
                CliOption {
                    name: "--authrpc.port".to_string(),
                    description: "The port to listen on for authenticated RPC".to_string(),
                    takes_value: true,
                    value_name: Some("PORT".to_string()),
                    possible_values: None,
                    accepts_multiple: false,
                },
            ]);
        }
        
        options
    }
}