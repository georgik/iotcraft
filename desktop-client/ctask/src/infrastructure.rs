/// Shared infrastructure utilities for MQTT server and process management
///
/// This module provides common functionality used by both mcplay and multi-client runner
/// for managing infrastructure components like MQTT servers, port checking, and process cleanup.
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::process::Command as TokioCommand;
use tokio::time::sleep;

/// Structure to track a running MQTT server process
pub struct MqttServerHandle {
    pub child: tokio::process::Child,
    pub port: u16,
}

impl MqttServerHandle {
    /// Kill the MQTT server process gracefully
    pub async fn kill(&mut self) -> Result<()> {
        if let Some(pid) = self.child.id() {
            println!("[DEBUG] Terminating MQTT server process PID: {}", pid);

            // Try graceful termination first
            #[cfg(unix)]
            {
                if let Err(e) = self.child.kill().await {
                    eprintln!("[WARN] Failed to kill MQTT server gracefully: {}", e);
                }
            }

            #[cfg(not(unix))]
            {
                if let Err(e) = self.child.kill().await {
                    eprintln!("[WARN] Failed to kill MQTT server: {}", e);
                }
            }

            // Wait a moment for the process to terminate
            match tokio::time::timeout(tokio::time::Duration::from_secs(3), self.child.wait()).await
            {
                Ok(Ok(status)) => {
                    println!("[DEBUG] MQTT server terminated with status: {:?}", status);
                }
                Ok(Err(e)) => {
                    eprintln!("[WARN] Error waiting for MQTT server termination: {}", e);
                }
                Err(_) => {
                    eprintln!("[WARN] MQTT server didn't terminate within timeout");

                    #[cfg(unix)]
                    {
                        // Force kill if it didn't terminate gracefully
                        use std::process::Command;
                        let _ = Command::new("kill")
                            .args(&["-9", &pid.to_string()])
                            .output();
                        println!("[DEBUG] Sent SIGKILL to MQTT server PID: {}", pid);
                    }
                }
            }
        }
        Ok(())
    }
}

/// Start an MQTT server from ../mqtt-server and return a handle to control it
pub async fn start_mqtt_server(log_file: PathBuf, port: u16) -> Result<MqttServerHandle> {
    // Resolve and validate the server working directory
    let server_dir = std::fs::canonicalize("../mqtt-server").with_context(|| {
        "Failed to resolve ../mqtt-server. Are you running xtask from desktop-client?"
    })?;
    let config_path = server_dir.join("rumqttd.toml");
    let has_config = config_path.exists();

    // Check for pre-built binary first (CI optimization)
    let pre_built_binary = server_dir.join("target/release/iotcraft-mqtt-server");
    let use_prebuilt = pre_built_binary.exists() && std::env::var("CI").is_ok();

    let (command, args, working_dir) = if use_prebuilt {
        println!("[DEBUG] Using pre-built MQTT server binary (CI optimization)");
        (
            pre_built_binary.to_string_lossy().to_string(),
            vec!["--port".to_string(), port.to_string()],
            server_dir.clone(),
        )
    } else {
        println!("[DEBUG] Building and running MQTT server with cargo");
        (
            "cargo".to_string(),
            vec![
                "run".to_string(),
                "--release".to_string(),
                "--".to_string(),
                "--port".to_string(),
                port.to_string(),
            ],
            server_dir.clone(),
        )
    };

    println!("[DEBUG] Starting MQTT server with:");
    println!("[DEBUG]   Command: {} {}", command, args.join(" "));
    println!("[DEBUG]   Working directory: {}", working_dir.display());
    println!(
        "[DEBUG]   Config file: {} ({})",
        config_path.display(),
        if has_config { "found" } else { "MISSING" }
    );
    println!("[DEBUG]   Log file: {}", log_file.display());
    if use_prebuilt {
        println!("[DEBUG]   Using pre-built binary to avoid rebuild");
    }

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
        "=== MQTT Server ===\n\
         Started at: {}\n\
         Port: {}\n\
         Working directory: {}\n\
         Config file: {} ({})\n\
         Command: {} {}\n\
         Mode: {}\n\
         ===================\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        port,
        working_dir.display(),
        config_path.display(),
        if has_config { "found" } else { "MISSING" },
        command,
        args.join(" "),
        if use_prebuilt {
            "pre-built binary"
        } else {
            "cargo run"
        }
    );

    log_handle.write_all(header.as_bytes()).await?;
    log_handle.flush().await?;

    if !has_config {
        println!("[WARN] rumqttd.toml not found at {}. The server may fail to start if it requires this config.", config_path.display());
    }

    // Start the MQTT server process
    let child = TokioCommand::new(&command)
        .args(&args)
        .current_dir(&working_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to start MQTT server in {}", working_dir.display()))?;

    println!(
        "[DEBUG] MQTT server process started with PID: {:?}",
        child.id()
    );

    // Return handle for control
    let handle = MqttServerHandle { child, port };
    Ok(handle)
}

/// Check if a TCP port is open and accepting connections
pub async fn is_port_open(host: &str, port: u16) -> bool {
    let addr_str = format!("{}:{}", host, port);

    // Use async TcpStream::connect with a timeout
    match tokio::time::timeout(
        Duration::from_millis(1000), // 1 second timeout
        TcpStream::connect(&addr_str),
    )
    .await
    {
        Ok(Ok(_)) => {
            // Successfully connected
            println!(
                "[DEBUG] Port {}:{} is open and accepting connections",
                host, port
            );
            true
        }
        Ok(Err(e)) => {
            println!("[DEBUG] Failed to connect to {}:{}: {}", host, port, e);
            false
        }
        Err(_) => {
            // Timeout occurred
            println!("[DEBUG] Connection timeout to {}:{}", host, port);
            false
        }
    }
}

/// Wait for a port to become available with timeout
pub async fn wait_for_port(host: &str, port: u16, timeout_secs: u64) -> Result<()> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    let mut attempt_count = 0;

    println!(
        "[DEBUG] Starting port availability check for {}:{} (timeout: {}s)",
        host, port, timeout_secs
    );

    while start.elapsed() < timeout {
        attempt_count += 1;
        println!(
            "[DEBUG] Attempt {}: Checking if port {}:{} is available...",
            attempt_count, host, port
        );

        if is_port_open(host, port).await {
            println!(
                "[DEBUG] Port {}:{} became available after {} attempts in {:.2}s",
                host,
                port,
                attempt_count,
                start.elapsed().as_secs_f64()
            );
            return Ok(());
        }

        println!(
            "[DEBUG] Port {}:{} not yet available, waiting...",
            host, port
        );
        sleep(Duration::from_millis(500)).await; // 500ms between attempts
    }

    Err(anyhow::anyhow!(
        "Port {}:{} did not become available within {} seconds after {} attempts",
        host,
        port,
        timeout_secs,
        attempt_count
    ))
}

/// Get appropriate timeout for MQTT server startup based on environment
pub fn get_mqtt_server_timeout() -> u64 {
    // Check if we're in CI environment
    let is_ci = std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("TRAVIS").is_ok()
        || std::env::var("JENKINS_URL").is_ok();

    // Allow override via environment variable
    if let Ok(timeout_str) = std::env::var("MQTT_SERVER_TIMEOUT") {
        if let Ok(timeout) = timeout_str.parse::<u64>() {
            println!("   Using custom MQTT server timeout: {} seconds", timeout);
            return timeout;
        }
    }

    if is_ci {
        println!("   Detected CI environment, using extended timeout: 120 seconds");
        120 // 2 minutes for CI environments where build might be needed
    } else {
        println!("   Using standard timeout: 30 seconds");
        30 // 30 seconds for local development
    }
}
