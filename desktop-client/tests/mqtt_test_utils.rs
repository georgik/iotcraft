use anyhow::{Context, Result};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::process::{Child, ChildStderr, ChildStdout};
use tokio::sync::oneshot;
use tokio::time::sleep;

/// MQTT server instance for tests
pub struct MqttTestServer {
    process: Child,
    port: u16,
    log_file: PathBuf,
    shutdown_tx: oneshot::Sender<()>,
}

impl MqttTestServer {
    /// Start a new MQTT server instance for testing
    pub async fn start(port: u16) -> Result<Self> {
        // Create temporary log file
        let log_file = std::env::temp_dir().join(format!("mqtt-test-server-{}.log", port));

        // Resolve the server working directory
        let server_dir = std::fs::canonicalize("../mqtt-server").with_context(
            || "Failed to resolve ../mqtt-server. Are you running tests from desktop-client?",
        )?;
        let config_path = server_dir.join("rumqttd.toml");

        println!("🟢 Starting MQTT test server on port {}", port);
        println!("   Working directory: {}", server_dir.display());
        println!(
            "   Config file: {} ({})",
            config_path.display(),
            if config_path.exists() {
                "found"
            } else {
                "MISSING"
            }
        );
        println!("   Log file: {}", log_file.display());

        // Create and open log file
        let mut log_handle = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_file)
            .await
            .with_context(|| format!("Failed to create log file: {}", log_file.display()))?;

        // Write header to log file
        let header = format!(
            "=== MQTT Test Server ===\n\
             Started at: {}\n\
             Port: {}\n\
             Working directory: {}\n\
             Config file: {} ({})\n\
             Command: cargo run -- --port {}\n\
             ========================\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            port,
            server_dir.display(),
            config_path.display(),
            if config_path.exists() {
                "found"
            } else {
                "MISSING"
            },
            port
        );

        log_handle.write_all(header.as_bytes()).await?;
        log_handle.flush().await?;

        // Start the MQTT server process
        let mut child = TokioCommand::new("cargo")
            .args(&["run", "--", "--port", &port.to_string()])
            .current_dir(&server_dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to start MQTT server in {}", server_dir.display()))?;

        println!("   Process started with PID: {:?}", child.id());

        // Get stdout and stderr for logging
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let log_file_stdout = log_file.clone();
        let log_file_stderr = log_file.clone();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Spawn tasks to handle stdout and stderr
        let stdout_task = tokio::spawn(async move {
            handle_server_stdout_stream(stdout_reader, log_file_stdout).await
        });

        let stderr_task = tokio::spawn(async move {
            handle_server_stderr_stream(stderr_reader, log_file_stderr).await
        });

        // Spawn a background task to monitor the process and handle shutdown
        tokio::spawn(async move {
            let mut shutdown_rx = shutdown_rx;
            let mut stdout_task = stdout_task;
            let mut stderr_task = stderr_task;

            tokio::select! {
                _ = &mut shutdown_rx => {
                    // Shutdown requested
                    stdout_task.abort();
                    stderr_task.abort();
                }
                result = &mut stdout_task => {
                    // stdout task finished (process might have died)
                    stderr_task.abort();
                    if let Err(e) = result {
                        eprintln!("Stdout task error: {:?}", e);
                    }
                }
                result = &mut stderr_task => {
                    // stderr task finished (process might have died)
                    stdout_task.abort();
                    if let Err(e) = result {
                        eprintln!("Stderr task error: {:?}", e);
                    }
                }
            }
        });

        let server = MqttTestServer {
            process: child,
            port,
            log_file,
            shutdown_tx,
        };

        // Wait for server to be ready
        server.wait_for_ready().await?;

        println!(
            "   ✅ MQTT test server is ready and listening on port {}",
            port
        );

        Ok(server)
    }

    /// Wait for the server to become ready by testing port connectivity
    async fn wait_for_ready(&self) -> Result<()> {
        let timeout_secs = 30;
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            if is_port_open("localhost", self.port) {
                return Ok(());
            }
            sleep(Duration::from_millis(200)).await;
        }

        Err(anyhow::anyhow!(
            "MQTT server port {}:{} did not become available within {} seconds",
            "localhost",
            self.port,
            timeout_secs
        ))
    }

    /// Get the port the server is running on
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the log file path
    pub fn log_file(&self) -> &PathBuf {
        &self.log_file
    }

    /// Shutdown the server
    pub async fn shutdown(mut self) -> Result<()> {
        println!("🛑 Shutting down MQTT test server on port {}", self.port);

        // Signal shutdown to background tasks
        let _ = self.shutdown_tx.send(());

        // Kill the process
        if let Err(e) = self.process.kill().await {
            eprintln!("Warning: Failed to kill MQTT server process: {}", e);
        }

        // Wait for process to exit
        match self.process.wait().await {
            Ok(status) => {
                if status.success() {
                    println!("   ✅ MQTT test server shut down cleanly");
                } else {
                    println!(
                        "   ⚠️  MQTT test server exited with status: {:?}",
                        status.code()
                    );
                }
            }
            Err(e) => {
                println!("   ⚠️  Error waiting for MQTT server shutdown: {}", e);
            }
        }

        // Clean up log file
        if let Err(e) = fs::remove_file(&self.log_file).await {
            eprintln!(
                "Warning: Failed to remove log file {}: {}",
                self.log_file.display(),
                e
            );
        }

        Ok(())
    }
}

/// Check if a TCP port is open and accepting connections
fn is_port_open(host: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;

    let addr_str = format!("{}:{}", host, port);

    // Use ToSocketAddrs to resolve hostname (including localhost) to actual IP addresses
    match addr_str.to_socket_addrs() {
        Ok(mut addrs) => {
            // Try to connect to the first resolved address
            if let Some(socket_addr) = addrs.next() {
                match TcpStream::connect_timeout(&socket_addr, Duration::from_millis(500)) {
                    Ok(_) => {
                        // Successfully connected, close and return true
                        true
                    }
                    Err(_) => false,
                }
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// Handle stdout stream from the MQTT server process
async fn handle_server_stdout_stream(
    mut reader: BufReader<ChildStdout>,
    log_file: PathBuf,
) -> Result<()> {
    let mut log_handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .await
        .with_context(|| {
            format!(
                "Failed to open log file for appending: {}",
                log_file.display()
            )
        })?;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");

        // Write to log file with timestamp
        let log_line = format!("[{}] [STDOUT] [MQTT-Test-Server] {}", timestamp, line);
        log_handle.write_all(log_line.as_bytes()).await?;

        line.clear();
    }

    log_handle.flush().await?;
    Ok(())
}

/// Handle stderr stream from the MQTT server process
async fn handle_server_stderr_stream(
    mut reader: BufReader<ChildStderr>,
    log_file: PathBuf,
) -> Result<()> {
    let mut log_handle = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .await
        .with_context(|| {
            format!(
                "Failed to open log file for appending: {}",
                log_file.display()
            )
        })?;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");

        // Write to log file with timestamp
        let log_line = format!("[{}] [STDERR] [MQTT-Test-Server] {}", timestamp, line);
        log_handle.write_all(log_line.as_bytes()).await?;

        line.clear();
    }

    log_handle.flush().await?;
    Ok(())
}

/// Find an available port for testing
pub fn find_available_port() -> u16 {
    use std::net::TcpListener;

    // Try to bind to port 0 to get an available port assigned by the OS
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to port 0");
    let addr = listener.local_addr().expect("Failed to get local address");
    addr.port()
}

/// Helper for integration tests that need MQTT server
pub struct MqttTestEnvironment {
    pub server: MqttTestServer,
    pub host: String,
    pub port: u16,
}

impl MqttTestEnvironment {
    /// Set up a complete MQTT test environment
    pub async fn setup() -> Result<Self> {
        let port = find_available_port();
        let server = MqttTestServer::start(port).await?;
        let host = "localhost".to_string();

        Ok(MqttTestEnvironment { server, host, port })
    }

    /// Get the MQTT broker URL
    pub fn broker_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Shutdown the test environment
    pub async fn shutdown(self) -> Result<()> {
        self.server.shutdown().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mqtt_server_startup_shutdown() -> Result<()> {
        let env = MqttTestEnvironment::setup().await?;

        // Verify server is accessible
        assert!(is_port_open(&env.host, env.port));

        // Capture values before shutdown
        let host = env.host.clone();
        let port = env.port;

        // Shutdown
        env.shutdown().await?;

        // Wait a moment and verify port is no longer accessible
        tokio::time::sleep(Duration::from_millis(500)).await;
        assert!(!is_port_open(&host, port));

        Ok(())
    }
}
