//! MQTT Test Infrastructure
//!
//! This module provides infrastructure for running MQTT integration tests
//! with proper MQTT server setup and teardown.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::time::sleep;

/// Test port range to avoid conflicts
const TEST_PORT_START: u16 = 18830;
const TEST_PORT_END: u16 = 18930;

/// Global port counter to avoid port conflicts across tests
static PORT_COUNTER: Mutex<u16> = Mutex::new(TEST_PORT_START);

/// MQTT Test Server instance
pub struct MqttTestServer {
    _temp_dir: TempDir,
    pub port: u16,
    pub host: String,
    server_handle: Option<tokio::task::JoinHandle<Result<()>>>,
    abort_handle: Option<tokio::task::AbortHandle>,
}

impl MqttTestServer {
    /// Start a new MQTT test server instance
    pub async fn start() -> Result<Self> {
        let port = get_next_test_port()?;
        let temp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
        let log_file = temp_dir.path().join("mqtt-server.log");

        println!("🟢 Starting MQTT test server on port {}", port);

        let server_handle = tokio::spawn(run_mqtt_server(log_file.clone(), port));
        let abort_handle = server_handle.abort_handle();

        // Wait for server to become available
        wait_for_port("localhost", port, 30)
            .await
            .with_context(|| format!("MQTT test server failed to start on port {}", port))?;

        println!("✅ MQTT test server ready on port {}", port);

        Ok(Self {
            _temp_dir: temp_dir,
            port,
            host: "localhost".to_string(),
            server_handle: Some(server_handle),
            abort_handle: Some(abort_handle),
        })
    }

    /// Get the MQTT broker address for clients
    pub fn broker_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Get the MQTT broker URL
    pub fn broker_url(&self) -> String {
        format!("mqtt://{}:{}", self.host, self.port)
    }

    /// Stop the MQTT server
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(abort_handle) = self.abort_handle.take() {
            abort_handle.abort();
        }

        if let Some(handle) = self.server_handle.take() {
            let _ = handle.await;
        }

        println!("🛑 MQTT test server stopped on port {}", self.port);
        Ok(())
    }
}

impl Drop for MqttTestServer {
    fn drop(&mut self) {
        if let Some(abort_handle) = &self.abort_handle {
            abort_handle.abort();
        }
    }
}

/// Get the next available test port
fn get_next_test_port() -> Result<u16> {
    let mut counter = PORT_COUNTER.lock().unwrap();
    let port = *counter;

    if port >= TEST_PORT_END {
        *counter = TEST_PORT_START;
    } else {
        *counter += 1;
    }

    Ok(port)
}

/// Check if a TCP port is open and accepting connections
async fn is_port_open(host: &str, port: u16) -> bool {
    match tokio::net::TcpStream::connect((host, port)).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Wait for a port to become available with timeout
async fn wait_for_port(host: &str, port: u16, timeout_secs: u64) -> Result<()> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        if is_port_open(host, port).await {
            return Ok(());
        }
        sleep(Duration::from_millis(100)).await;
    }

    Err(anyhow::anyhow!(
        "Port {}:{} did not become available within {} seconds",
        host,
        port,
        timeout_secs
    ))
}

/// Run the MQTT server from ../mqtt-server directory
async fn run_mqtt_server(log_file: PathBuf, port: u16) -> Result<()> {
    // Resolve the server working directory relative to the desktop-client directory
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let server_dir = current_dir
        .parent()
        .context("Failed to get parent directory")?
        .join("mqtt-server");

    if !server_dir.exists() {
        return Err(anyhow::anyhow!(
            "MQTT server directory not found: {}. Make sure the mqtt-server project exists.",
            server_dir.display()
        ));
    }

    let server_dir = server_dir.canonicalize().with_context(|| {
        format!(
            "Failed to resolve mqtt-server directory: {}",
            server_dir.display()
        )
    })?;

    // Create log file
    let mut log_handle = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file)
        .await
        .with_context(|| format!("Failed to create log file: {}", log_file.display()))?;

    let header = format!(
        "=== MQTT Test Server ===\n\
         Started at: {}\n\
         Port: {}\n\
         Working directory: {}\n\
         Command: cargo run -- --port {}\n\
         =======================\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        port,
        server_dir.display(),
        port
    );

    log_handle.write_all(header.as_bytes()).await?;
    log_handle.flush().await?;

    // Start the MQTT server process
    let mut child = Command::new("cargo")
        .args(&["run", "--", "--port", &port.to_string()])
        .current_dir(&server_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "Failed to start MQTT server. Command: cargo run -- --port {} (working dir: {})",
                port,
                server_dir.display()
            )
        })?;

    // Handle stdout and stderr
    if let Some(stdout) = child.stdout.take() {
        let log_file_clone = log_file.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut log_handle = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file_clone)
                .await
                .unwrap();

            while let Ok(Some(line)) = lines.next_line().await {
                let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
                let log_line = format!("[{}] [STDOUT] {}\n", timestamp, line);
                let _ = log_handle.write_all(log_line.as_bytes()).await;
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        let log_file_clone = log_file.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut log_handle = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file_clone)
                .await
                .unwrap();

            while let Ok(Some(line)) = lines.next_line().await {
                let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
                let log_line = format!("[{}] [STDERR] {}\n", timestamp, line);
                let _ = log_handle.write_all(log_line.as_bytes()).await;
            }
        });
    }

    // Wait for the process to complete
    let exit_status = child
        .wait()
        .await
        .context("Failed to wait for MQTT server")?;

    if !exit_status.success() {
        return Err(anyhow::anyhow!(
            "MQTT server exited with non-zero status: {:?}",
            exit_status
        ));
    }

    Ok(())
}

/// Test helper macros and utilities
#[macro_export]
macro_rules! mqtt_test {
    ($test_name:ident, $test_fn:expr) => {
        #[tokio::test]
        async fn $test_name() -> Result<(), Box<dyn std::error::Error>> {
            let _server = crate::mqtt_test_infrastructure::MqttTestServer::start().await?;
            $test_fn(_server).await
        }
    };
}

/// Test configuration for MQTT tests
pub struct MqttTestConfig {
    pub timeout_secs: u64,
    pub retry_attempts: usize,
    pub retry_delay_ms: u64,
}

impl Default for MqttTestConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Run a test with MQTT server infrastructure
pub async fn with_mqtt_server<F, Fut>(test_fn: F) -> Result<()>
where
    F: FnOnce(MqttTestServer) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let server = MqttTestServer::start().await?;
    let result = test_fn(server).await;
    result
}

/// Utility function to wait for MQTT connection stability
pub async fn wait_for_mqtt_stability(duration_ms: u64) {
    sleep(Duration::from_millis(duration_ms)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mqtt_server_starts_and_stops() -> Result<()> {
        let mut server = MqttTestServer::start().await?;

        // Verify server is running
        assert!(is_port_open(&server.host, server.port).await);

        // Test server methods
        assert!(!server.broker_address().is_empty());
        assert!(server.broker_url().starts_with("mqtt://"));

        server.stop().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_multiple_servers_different_ports() -> Result<()> {
        let server1 = MqttTestServer::start().await?;
        let server2 = MqttTestServer::start().await?;

        // Should have different ports
        assert_ne!(server1.port, server2.port);

        // Both should be accessible
        assert!(is_port_open(&server1.host, server1.port).await);
        assert!(is_port_open(&server2.host, server2.port).await);

        Ok(())
    }
}
