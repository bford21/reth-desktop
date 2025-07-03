use std::collections::VecDeque;
use std::process::{Command, Stdio, Child};
use std::io::{BufRead, BufReader, SeekFrom, Seek};
use std::sync::{Arc, Mutex};
use std::thread;
use std::path::PathBuf;
use std::fs::File;
use tokio::sync::mpsc;

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

pub struct RethNode {
    process: Option<Child>,
    log_buffer: Arc<Mutex<VecDeque<LogLine>>>,
    log_receiver: Option<mpsc::UnboundedReceiver<LogLine>>,
    is_running: bool,
    external_log_path: Option<PathBuf>,
    last_external_check: std::time::Instant,
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
        }
    }

    pub fn start(&mut self, reth_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        
        // Spawn the reth process with file logging enabled
        let mut command = Command::new(reth_path);
        command
            .arg("node")
            .arg("--full")
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
                
            // Store the log directory path - we'll find the actual log file later
            // Reth creates files with date patterns like reth-2024-01-15-20.log
            self.external_log_path = Some(log_path.clone());
        }
        
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Capture stdout
        if let Some(stdout) = child.stdout.take() {
            let sender = log_sender.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        let log_line = LogLine {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            content: line.clone(),
                            level: LogLevel::from_content(&line),
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
                        let log_line = LogLine {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            content: line.clone(),
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
            // Clear the log path for managed processes
            self.external_log_path = None;
        } else {
            // For external processes, just reset the running state
            self.is_running = false;
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
                }
                Ok(None) => {
                    // Process is still running
                }
                Err(_) => {
                    self.is_running = false;
                    self.process = None;
                    self.external_log_path = None;
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
                log_lines.push(LogLine {
                    timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                    content: trimmed.to_string(),
                    level: LogLevel::from_content(trimmed),
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
                        let log_line = LogLine {
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            content: trimmed.to_string(),
                            level: LogLevel::from_content(trimmed),
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
}