use std::path::PathBuf;
use std::process::Stdio;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::task::JoinHandle;

pub struct MqttTestServer {
    _temp_dir: TempDir,
    log_file: PathBuf,
    server_handle: JoinHandle<()>,
}

impl MqttTestServer {
    pub fn log_file(&self) -> &PathBuf {
        &self.log_file
    }
}

impl Drop for MqttTestServer {
    fn drop(&mut self) {
        // Abort the server process when dropped
        self.server_handle.abort();
    }
}

pub struct MqttTestEnvironment {
    pub port: u16,
    pub server: MqttTestServer,
}

impl MqttTestEnvironment {
    /// Set up test environment using the project's mqtt-server
    pub async fn setup() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Find an available port
        let port = find_available_port().await?;

        // Create temporary directory for test files
        let temp_dir = TempDir::new()?;
        let log_file = temp_dir.path().join("mqtt_test.log");

        // Start MQTT server from ../mqtt-server using the same approach as xtask
        let server_handle = start_mqtt_server(port, log_file.clone()).await?;

        let server = MqttTestServer {
            _temp_dir: temp_dir,
            log_file,
            server_handle,
        };

        // Wait for server to be ready
        wait_for_port("localhost", port, 10).await?;
        println!("MQTT server ready on localhost:{}", port);

        Ok(MqttTestEnvironment { port, server })
    }

    pub async fn shutdown(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Server will be automatically stopped when dropped
        drop(self.server);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await; // Give time for cleanup
        Ok(())
    }
}

/// Start the MQTT server from ../mqtt-server (same as xtask does)
async fn start_mqtt_server(
    port: u16,
    log_file: PathBuf,
) -> Result<JoinHandle<()>, Box<dyn std::error::Error + Send + Sync>> {
    // Resolve the server directory
    let server_dir = std::fs::canonicalize("../mqtt-server")
        .map_err(|e| format!("Failed to find ../mqtt-server: {}", e))?;

    println!(
        "Starting MQTT server on port {} from {}",
        port,
        server_dir.display()
    );

    let handle = tokio::spawn(async move {
        if let Err(e) = run_mqtt_server_process(server_dir, port, log_file).await {
            eprintln!("MQTT server error: {}", e);
        }
    });

    Ok(handle)
}

/// Run the MQTT server process (similar to xtask implementation)
async fn run_mqtt_server_process(
    server_dir: PathBuf,
    port: u16,
    log_file: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Start the MQTT server process
    let mut child = Command::new("cargo")
        .args(&["run", "--", "--port", &port.to_string()])
        .current_dir(&server_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start MQTT server: {}", e))?;

    // Get stdout and stderr for logging
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let log_file_clone = log_file.clone();

    // Spawn tasks to handle output
    let stdout_task = tokio::spawn(async move {
        let _ = handle_server_output(stdout_reader, log_file_clone, "STDOUT").await;
    });

    let stderr_task = tokio::spawn(async move {
        let _ = handle_server_output(stderr_reader, log_file, "STDERR").await;
    });

    // Wait for the process
    let exit_status = child
        .wait()
        .await
        .map_err(|e| format!("Failed to wait for MQTT server: {}", e))?;

    // Clean up output tasks
    stdout_task.abort();
    stderr_task.abort();

    if !exit_status.success() {
        return Err(format!("MQTT server exited with code: {:?}", exit_status.code()).into());
    }

    Ok(())
}

/// Handle server output and log it
async fn handle_server_output<R>(
    mut reader: BufReader<R>,
    log_file: PathBuf,
    stream_type: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::fs::OpenOptions;
    use tokio::io::AsyncWriteExt;

    let mut log_handle = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .await?;

    let mut line = String::new();
    while reader.read_line(&mut line).await? > 0 {
        let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
        let log_line = format!("[{}] [{}] [MQTT-Server] {}", timestamp, stream_type, line);

        log_handle.write_all(log_line.as_bytes()).await?;
        print!("[MQTT-Server] {}", line);

        line.clear();
    }

    log_handle.flush().await?;
    Ok(())
}

/// Wait for a port to become available with timeout
async fn wait_for_port(
    host: &str,
    port: u16,
    timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::net::TcpStream;
    use std::time::{Duration, Instant};

    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    while start.elapsed() < timeout {
        if is_port_open(host, port) {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    Err(format!(
        "Port {}:{} did not become available within {} seconds",
        host, port, timeout_secs
    )
    .into())
}

/// Check if a TCP port is open
fn is_port_open(host: &str, port: u16) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Duration;

    let addr_str = format!("{}:{}", host, port);

    match addr_str.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(socket_addr) = addrs.next() {
                TcpStream::connect_timeout(&socket_addr, Duration::from_millis(500)).is_ok()
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

async fn find_available_port() -> Result<u16, Box<dyn std::error::Error + Send + Sync>> {
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}
