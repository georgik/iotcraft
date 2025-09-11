//! Async log collection for mcplay
//!
//! This module handles collecting stdout/stderr from spawned processes
//! and writing them to log files with periodic flushing.

use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStderr, ChildStdout};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

/// Async log collector that reads from process stdout/stderr and writes to files
pub struct ProcessLogCollector {
    client_id: String,
    log_buffer: Arc<Mutex<Vec<String>>>,
    log_file_path: String,
}

/// Async log collector with custom file path
pub struct ProcessLogCollectorWithPath {
    client_id: String,
    log_buffer: Arc<Mutex<Vec<String>>>,
    log_file_path: String,
}

impl ProcessLogCollector {
    pub fn new(client_id: String) -> Self {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let log_file_path = format!("logs/client_{}_{}.log", client_id, timestamp);

        Self {
            client_id,
            log_buffer: Arc::new(Mutex::new(Vec::new())),
            log_file_path,
        }
    }

    /// Start collecting logs from stdout and stderr with periodic flushing
    pub async fn start_collection(
        &self,
        stdout: ChildStdout,
        stderr: ChildStderr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create logs directory if it doesn't exist
        tokio::fs::create_dir_all("logs").await?;

        // Clone for async tasks
        let stdout_buffer = self.log_buffer.clone();
        let stderr_buffer = self.log_buffer.clone();
        let flush_buffer = self.log_buffer.clone();
        let log_path = self.log_file_path.clone();
        let client_id = self.client_id.clone();
        let client_id_stderr = self.client_id.clone();

        // Spawn stdout reader
        let client_id_stdout = client_id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            while let Ok(bytes_read) = reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break; // EOF
                }

                let clean_line = strip_ansi_colors(&line);
                let timestamped_line = format!(
                    "[{}] {}",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    clean_line.trim_end()
                );

                {
                    let mut buffer = stdout_buffer.lock().await;
                    buffer.push(timestamped_line);
                }

                line.clear();
            }

            eprintln!("stdout reader for {} finished", client_id_stdout);
        });

        // Spawn stderr reader
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();

            while let Ok(bytes_read) = reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break; // EOF
                }

                let clean_line = strip_ansi_colors(&line);
                let timestamped_line = format!(
                    "[{}] [STDERR] {}",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    clean_line.trim_end()
                );

                {
                    let mut buffer = stderr_buffer.lock().await;
                    buffer.push(timestamped_line);
                }

                line.clear();
            }

            eprintln!("stderr reader for {} finished", client_id_stderr);
        });

        // Spawn periodic flusher (every 1 second)
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                let lines_to_write = {
                    let mut buffer = flush_buffer.lock().await;
                    if buffer.is_empty() {
                        continue;
                    }
                    std::mem::take(&mut *buffer)
                };

                // Write all buffered lines to file
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path)
                    .await
                {
                    for line in lines_to_write {
                        if let Err(e) = file.write_all(format!("{}\n", line).as_bytes()).await {
                            eprintln!("Failed to write to log file {}: {}", log_path, e);
                        }
                    }

                    if let Err(e) = file.flush().await {
                        eprintln!("Failed to flush log file {}: {}", log_path, e);
                    }
                } else {
                    eprintln!("Failed to open log file: {}", log_path);
                }
            }
        });

        Ok(())
    }
}

impl ProcessLogCollectorWithPath {
    pub fn new(client_id: String, log_file_path: String) -> Self {
        Self {
            client_id,
            log_buffer: Arc::new(Mutex::new(Vec::new())),
            log_file_path,
        }
    }

    /// Start collecting logs from stdout and stderr with periodic flushing
    pub async fn start_collection(
        &self,
        stdout: ChildStdout,
        stderr: ChildStderr,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Create logs directory if it doesn't exist
        tokio::fs::create_dir_all("logs").await?;

        // Clone for async tasks
        let stdout_buffer = self.log_buffer.clone();
        let stderr_buffer = self.log_buffer.clone();
        let flush_buffer = self.log_buffer.clone();
        let log_path = self.log_file_path.clone();
        let client_id = self.client_id.clone();
        let client_id_stderr = self.client_id.clone();

        // Spawn stdout reader
        let client_id_stdout = client_id.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            while let Ok(bytes_read) = reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break; // EOF
                }

                let clean_line = strip_ansi_colors(&line);
                let timestamped_line = format!(
                    "[{}] {}",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    clean_line.trim_end()
                );

                {
                    let mut buffer = stdout_buffer.lock().await;
                    buffer.push(timestamped_line);
                }

                line.clear();
            }

            eprintln!("stdout reader for {} finished", client_id_stdout);
        });

        // Spawn stderr reader
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();

            while let Ok(bytes_read) = reader.read_line(&mut line).await {
                if bytes_read == 0 {
                    break; // EOF
                }

                let clean_line = strip_ansi_colors(&line);
                let timestamped_line = format!(
                    "[{}] [STDERR] {}",
                    chrono::Utc::now().format("%H:%M:%S%.3f"),
                    clean_line.trim_end()
                );

                {
                    let mut buffer = stderr_buffer.lock().await;
                    buffer.push(timestamped_line);
                }

                line.clear();
            }

            eprintln!("stderr reader for {} finished", client_id_stderr);
        });

        // Spawn periodic flusher (every 1 second)
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                let lines_to_write = {
                    let mut buffer = flush_buffer.lock().await;
                    if buffer.is_empty() {
                        continue;
                    }
                    std::mem::take(&mut *buffer)
                };

                // Write all buffered lines to file
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path)
                    .await
                {
                    for line in lines_to_write {
                        if let Err(e) = file.write_all(format!("{}\n", line).as_bytes()).await {
                            eprintln!("Failed to write to log file {}: {}", log_path, e);
                        }
                    }

                    if let Err(e) = file.flush().await {
                        eprintln!("Failed to flush log file {}: {}", log_path, e);
                    }
                } else {
                    eprintln!("Failed to open log file: {}", log_path);
                }
            }
        });

        Ok(())
    }
}

/// Collect logs from a process asynchronously
pub async fn collect_process_logs(client_id: String, stdout: ChildStdout, stderr: ChildStderr) {
    let collector = ProcessLogCollector::new(client_id.clone());

    if let Err(e) = collector.start_collection(stdout, stderr).await {
        eprintln!("Failed to start log collection for {}: {}", client_id, e);
    }
}

/// Collect logs from a process asynchronously to a specified file path
pub async fn collect_process_logs_to_file(
    client_id: String,
    stdout: ChildStdout,
    stderr: ChildStderr,
    log_file_path: String,
) {
    let collector = ProcessLogCollectorWithPath::new(client_id.clone(), log_file_path);

    if let Err(e) = collector.start_collection(stdout, stderr).await {
        eprintln!("Failed to start log collection for {}: {}", client_id, e);
    }
}

/// Strip ANSI escape codes from text for clean log files while preserving emojis
fn strip_ansi_colors(text: &str) -> String {
    use regex::Regex;

    // Regex to match ANSI escape sequences for colors and formatting
    static ANSI_REGEX: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    let regex = ANSI_REGEX.get_or_init(|| {
        // More comprehensive ANSI escape sequence pattern
        // \\x1B\\[ - ESC[
        // [0-9;?]* - parameter bytes (numbers, semicolons, question marks)
        // [a-zA-Z] - final byte (letter commands like m, K, H, J, etc.)
        Regex::new(r"\x1B\[[0-9;?]*[a-zA-Z]").expect("Valid ANSI regex")
    });

    regex.replace_all(text, "").to_string()
}
