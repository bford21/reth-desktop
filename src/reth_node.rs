use std::collections::VecDeque;
use std::process::{Command, Stdio, Child};
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
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
}

impl RethNode {
    pub fn new() -> Self {
        Self {
            process: None,
            log_buffer: Arc::new(Mutex::new(VecDeque::new())),
            log_receiver: None,
            is_running: false,
        }
    }

    pub fn start(&mut self, reth_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_running {
            return Err("Reth node is already running".into());
        }

        // Create channel for log communication
        let (log_sender, log_receiver) = mpsc::unbounded_channel();
        self.log_receiver = Some(log_receiver);

        // Spawn the reth process
        let mut child = Command::new(reth_path)
            .arg("node")
            .arg("--full")
            .arg("--log.stdout.format")
            .arg("terminal")
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
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.is_running
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
            match process.try_wait() {
                Ok(Some(_)) => {
                    self.is_running = false;
                    self.process = None;
                }
                Ok(None) => {
                    // Process is still running
                }
                Err(_) => {
                    self.is_running = false;
                    self.process = None;
                }
            }
        }
    }
}