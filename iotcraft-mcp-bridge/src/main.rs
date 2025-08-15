use log::{debug, error, info};
use serde_json::Value;
use std::env;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Get the MCP server port from environment or use default
    let port = env::var("MCP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    info!(
        "IoTCraft MCP Bridge starting, connecting to localhost:{}",
        port
    );

    // Connect to the desktop client's MCP server
    let tcp_stream = match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
        Ok(stream) => {
            info!("Connected to IoTCraft MCP server on port {}", port);
            stream
        }
        Err(e) => {
            error!("Failed to connect to MCP server: {}", e);
            eprintln!("Error: Could not connect to IoTCraft desktop client on port {}. Make sure the desktop client is running with --mcp flag.", port);
            std::process::exit(1);
        }
    };

    // Set TCP_NODELAY to reduce latency
    if let Err(e) = tcp_stream.set_nodelay(true) {
        log::warn!("Failed to set TCP_NODELAY: {}", e);
    }

    let (tcp_reader, mut tcp_writer) = tcp_stream.into_split();
    let mut tcp_buf_reader = BufReader::new(tcp_reader).lines();

    // Set up stdin/stdout for MCP communication with Warp
    let stdin = tokio::io::stdin();
    let mut stdin_reader = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    info!("Bridge established - ready for MCP communication");

    loop {
        tokio::select! {
            // Read from Warp (stdin) and forward to desktop client
            line_result = stdin_reader.next_line() => {
                match line_result {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }

                        debug!("Received from Warp: {}", line);

                        // Validate JSON before forwarding
                        match serde_json::from_str::<Value>(&line) {
                            Ok(_) => {
                                // Forward to desktop client
                                tcp_writer.write_all(line.as_bytes()).await?;
                                tcp_writer.write_all(b"\n").await?;
                                tcp_writer.flush().await?;
                                debug!("Forwarded to desktop client: {}", line);
                            }
                            Err(e) => {
                                log::warn!("Invalid JSON from Warp, skipping: {}", e);

                                // Send error response back to Warp
                                let error_response = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": null,
                                    "error": {
                                        "code": -32700,
                                        "message": "Parse error"
                                    }
                                });

                                let response_str = serde_json::to_string(&error_response)?;
                                stdout.write_all(response_str.as_bytes()).await?;
                                stdout.write_all(b"\n").await?;
                                stdout.flush().await?;
                            }
                        }
                    }
                    Ok(None) => {
                        info!("Warp disconnected (stdin closed)");
                        break;
                    }
                    Err(e) => {
                        error!("Error reading from Warp: {}", e);
                        break;
                    }
                }
            }

            // Read from desktop client and forward to Warp (stdout)
            line_result = tcp_buf_reader.next_line() => {
                match line_result {
                    Ok(Some(line)) => {
                        if line.trim().is_empty() {
                            continue;
                        }

                        debug!("Received from desktop client: {}", line);

                        // Validate JSON before forwarding
                        match serde_json::from_str::<Value>(&line) {
                            Ok(_) => {
                                // Forward to Warp
                                stdout.write_all(line.as_bytes()).await?;
                                stdout.write_all(b"\n").await?;
                                stdout.flush().await?;
                                debug!("Forwarded to Warp: {}", line);
                            }
                            Err(e) => {
                                log::warn!("Invalid JSON from desktop client: {} - Raw: {}", e, line);
                            }
                        }
                    }
                    Ok(None) => {
                        info!("Desktop client disconnected");
                        break;
                    }
                    Err(e) => {
                        error!("Error reading from desktop client: {}", e);
                        break;
                    }
                }
            }
        }
    }

    info!("Bridge connection closed");
    Ok(())
}
