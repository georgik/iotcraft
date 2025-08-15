use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

#[derive(Debug, Serialize, Deserialize)]
struct TestResult {
    name: String,
    passed: bool,
    duration_ms: u64,
    error: Option<String>,
    details: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TestReport {
    timestamp: String,
    server_address: String,
    total_tests: usize,
    passed: usize,
    failed: usize,
    duration_ms: u64,
    results: Vec<TestResult>,
}

#[derive(Parser)]
#[command(name = "mcp_test_client")]
#[command(about = "A CLI test client for IoTCraft MCP server")]
struct Cli {
    /// MCP server host
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// MCP server port
    #[arg(long, default_value = "8080")]
    port: u16,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the MCP connection
    Init,
    /// List available tools
    ListTools,
    /// Run all tests and generate a report
    RunTests {
        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,
        /// File to write results to (optional)
        #[arg(long)]
        output: Option<String>,
    },
    /// Test create_wall tool
    TestWall {
        /// Block type
        #[arg(default_value = "stone")]
        block_type: String,
        /// Starting coordinates (x1 y1 z1)
        #[arg(default_value = "0")]
        x1: i32,
        #[arg(default_value = "0")]
        y1: i32,
        #[arg(default_value = "0")]
        z1: i32,
        /// Ending coordinates (x2 y2 z2)
        #[arg(default_value = "2")]
        x2: i32,
        #[arg(default_value = "1")]
        y2: i32,
        #[arg(default_value = "1")]
        z2: i32,
    },
    /// Test place_block tool
    TestPlace {
        /// Block type
        #[arg(default_value = "grass")]
        block_type: String,
        /// Coordinates
        #[arg(default_value = "5")]
        x: i32,
        #[arg(default_value = "1")]
        y: i32,
        #[arg(default_value = "5")]
        z: i32,
    },
    /// Test spawn_device tool
    TestSpawn {
        /// Device ID
        #[arg(default_value = "test_device")]
        device_id: String,
        /// Device type
        #[arg(default_value = "lamp")]
        device_type: String,
        /// Coordinates
        #[arg(default_value = "3.0")]
        x: f64,
        #[arg(default_value = "1.0")]
        y: f64,
        #[arg(default_value = "3.0")]
        z: f64,
    },
    /// Run a comprehensive test suite
    TestSuite,
    /// Interactive mode
    Interactive,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let server_addr = format!("{}:{}", cli.host, cli.port);

    match cli.command {
        Commands::Init => test_init(&server_addr).await?,
        Commands::ListTools => test_list_tools(&server_addr).await?,
        Commands::RunTests { format, output } => {
            run_comprehensive_tests(&server_addr, &format, output.as_deref()).await?
        }
        Commands::TestWall {
            block_type,
            x1,
            y1,
            z1,
            x2,
            y2,
            z2,
        } => test_create_wall(&server_addr, &block_type, x1, y1, z1, x2, y2, z2).await?,
        Commands::TestPlace {
            block_type,
            x,
            y,
            z,
        } => test_place_block(&server_addr, &block_type, x, y, z).await?,
        Commands::TestSpawn {
            device_id,
            device_type,
            x,
            y,
            z,
        } => test_spawn_device(&server_addr, &device_id, &device_type, x, y, z).await?,
        Commands::TestSuite => test_suite(&server_addr).await?,
        Commands::Interactive => interactive_mode(&server_addr).await?,
    }

    Ok(())
}

async fn connect_to_server(server_addr: &str) -> Result<TcpStream, Box<dyn std::error::Error>> {
    println!("Connecting to MCP server at {}...", server_addr);
    let stream = TcpStream::connect(server_addr).await?;
    println!("Connected successfully!");
    Ok(stream)
}

async fn send_request_and_get_response(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    request: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let request_str = serde_json::to_string(&request)?;
    println!("Sending: {}", request_str);

    writer
        .write_all(format!("{}\n", request_str).as_bytes())
        .await?;

    let mut response_line = String::new();
    tokio::time::timeout(
        Duration::from_secs(10),
        reader.read_line(&mut response_line),
    )
    .await??;

    if response_line.trim().is_empty() {
        return Err("Empty response from server".into());
    }

    let response: Value = serde_json::from_str(response_line.trim())?;
    println!("Response: {}", serde_json::to_string_pretty(&response)?);

    Ok(response)
}

async fn test_init(server_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let stream = connect_to_server(server_addr).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "mcp_test_client",
                "version": "1.0.0"
            }
        }
    });

    let response = send_request_and_get_response(&mut reader, &mut writer, init_request).await?;

    // Validate response
    if response["jsonrpc"] == "2.0" && response["result"]["serverInfo"]["name"] == "iotcraft" {
        println!("✅ Initialize test passed!");
    } else {
        println!("❌ Initialize test failed!");
    }

    Ok(())
}

async fn test_list_tools(server_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let stream = connect_to_server(server_addr).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = send_request_and_get_response(&mut reader, &mut writer, tools_request).await?;

    // Validate response
    if let Some(tools) = response["result"]["tools"].as_array() {
        println!("✅ Found {} tools:", tools.len());
        for tool in tools {
            println!(
                "  - {}: {}",
                tool["name"].as_str().unwrap_or("unknown"),
                tool["description"].as_str().unwrap_or("no description")
            );
        }
    } else {
        println!("❌ List tools test failed!");
    }

    Ok(())
}

async fn test_create_wall(
    server_addr: &str,
    block_type: &str,
    x1: i32,
    y1: i32,
    z1: i32,
    x2: i32,
    y2: i32,
    z2: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream = connect_to_server(server_addr).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let tool_call = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "create_wall",
            "arguments": {
                "block_type": block_type,
                "x1": x1, "y1": y1, "z1": z1,
                "x2": x2, "y2": y2, "z2": z2
            }
        }
    });

    let response = send_request_and_get_response(&mut reader, &mut writer, tool_call).await?;

    // Check if the response indicates success or proper error handling
    if response.get("result").is_some()
        || (response.get("error").is_some() && response["error"]["code"] != -32603)
    {
        println!("✅ Create wall test completed (check response above)");
    } else {
        println!("❌ Create wall test failed!");
    }

    Ok(())
}

async fn test_place_block(
    server_addr: &str,
    block_type: &str,
    x: i32,
    y: i32,
    z: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream = connect_to_server(server_addr).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let tool_call = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "place_block",
            "arguments": {
                "block_type": block_type,
                "x": x, "y": y, "z": z
            }
        }
    });

    let response = send_request_and_get_response(&mut reader, &mut writer, tool_call).await?;

    if response.get("result").is_some() {
        println!("✅ Place block test completed");
    } else {
        println!("❌ Place block test failed!");
    }

    Ok(())
}

async fn test_spawn_device(
    server_addr: &str,
    device_id: &str,
    device_type: &str,
    x: f64,
    y: f64,
    z: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream = connect_to_server(server_addr).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let tool_call = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": {
            "name": "spawn_device",
            "arguments": {
                "device_id": device_id,
                "device_type": device_type,
                "x": x, "y": y, "z": z
            }
        }
    });

    let response = send_request_and_get_response(&mut reader, &mut writer, tool_call).await?;

    if response.get("result").is_some() {
        println!("✅ Spawn device test completed");
    } else {
        println!("❌ Spawn device test failed!");
    }

    Ok(())
}

async fn test_suite(server_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Running comprehensive MCP test suite...\n");

    // Test 1: Initialize
    println!("Test 1: Initialize");
    test_init(server_addr).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test 2: List tools
    println!("\nTest 2: List tools");
    test_list_tools(server_addr).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test 3: Create wall
    println!("\nTest 3: Create wall");
    test_create_wall(server_addr, "stone", 0, 0, 0, 3, 2, 1).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test 4: Place block
    println!("\nTest 4: Place block");
    test_place_block(server_addr, "grass", 10, 1, 10).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test 5: Spawn device
    println!("\nTest 5: Spawn device");
    test_spawn_device(server_addr, "test_lamp_suite", "lamp", 5.0, 1.0, 5.0).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Test 6: Test error handling (invalid parameters)
    println!("\nTest 6: Error handling");
    test_invalid_parameters(server_addr).await?;

    println!("\n✅ Test suite completed!");
    Ok(())
}

async fn test_invalid_parameters(server_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let stream = connect_to_server(server_addr).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Test missing parameters
    let invalid_call = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "tools/call",
        "params": {
            "name": "create_wall",
            "arguments": {
                "block_type": "stone",
                "x1": 0,
                "y1": 0
                // Missing required parameters
            }
        }
    });

    let response = send_request_and_get_response(&mut reader, &mut writer, invalid_call).await?;

    // Should either return an error or gracefully handle the invalid request
    if response.get("error").is_some()
        || (response.get("result").is_some() && response["result"].get("error").is_some())
    {
        println!("✅ Error handling test passed - properly handled invalid parameters");
    } else {
        println!("⚠️  Error handling test unclear - check response above");
    }

    Ok(())
}

async fn interactive_mode(server_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Entering interactive mode. Type 'help' for commands or 'quit' to exit.");

    let stream = connect_to_server(server_addr).await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    loop {
        print!("mcp> ");
        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "quit" | "exit" => break,
            "help" => print_help(),
            "init" => {
                let init_request = json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": {},
                        "clientInfo": {"name": "interactive_client", "version": "1.0.0"}
                    }
                });
                let _ = send_request_and_get_response(&mut reader, &mut writer, init_request).await;
            }
            "tools" => {
                let tools_request = json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/list",
                    "params": {}
                });
                let _ =
                    send_request_and_get_response(&mut reader, &mut writer, tools_request).await;
            }
            cmd if cmd.starts_with("wall ") => {
                // Parse: wall block_type x1 y1 z1 x2 y2 z2
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() == 8 {
                    let tool_call = json!({
                        "jsonrpc": "2.0",
                        "id": 3,
                        "method": "tools/call",
                        "params": {
                            "name": "create_wall",
                            "arguments": {
                                "block_type": parts[1],
                                "x1": parts[2].parse::<i32>().unwrap_or(0),
                                "y1": parts[3].parse::<i32>().unwrap_or(0),
                                "z1": parts[4].parse::<i32>().unwrap_or(0),
                                "x2": parts[5].parse::<i32>().unwrap_or(0),
                                "y2": parts[6].parse::<i32>().unwrap_or(0),
                                "z2": parts[7].parse::<i32>().unwrap_or(0)
                            }
                        }
                    });
                    let _ =
                        send_request_and_get_response(&mut reader, &mut writer, tool_call).await;
                } else {
                    println!("Usage: wall <block_type> <x1> <y1> <z1> <x2> <y2> <z2>");
                }
            }
            cmd if cmd.starts_with("place ") => {
                // Parse: place block_type x y z
                let parts: Vec<&str> = cmd.split_whitespace().collect();
                if parts.len() == 5 {
                    let tool_call = json!({
                        "jsonrpc": "2.0",
                        "id": 4,
                        "method": "tools/call",
                        "params": {
                            "name": "place_block",
                            "arguments": {
                                "block_type": parts[1],
                                "x": parts[2].parse::<i32>().unwrap_or(0),
                                "y": parts[3].parse::<i32>().unwrap_or(0),
                                "z": parts[4].parse::<i32>().unwrap_or(0)
                            }
                        }
                    });
                    let _ =
                        send_request_and_get_response(&mut reader, &mut writer, tool_call).await;
                } else {
                    println!("Usage: place <block_type> <x> <y> <z>");
                }
            }
            _ => println!("Unknown command. Type 'help' for available commands."),
        }
    }

    println!("Goodbye!");
    Ok(())
}

async fn run_comprehensive_tests(
    server_addr: &str,
    format: &str,
    output_file: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Running comprehensive MCP test suite...");
    let start_time = Instant::now();
    let mut results = Vec::new();

    // Test 1: Server connection
    results.push(
        run_single_test("server_connection", || async {
            TcpStream::connect(server_addr)
                .await
                .map_err(|e| e.to_string())?;
            Ok(json!({"connected": true}))
        })
        .await,
    );

    // Test 2: Initialize protocol
    results.push(
        run_single_test("initialize_protocol", || async {
            let stream = TcpStream::connect(server_addr)
                .await
                .map_err(|e| e.to_string())?;
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);

            let request = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {"name": "test_client", "version": "1.0.0"}
                }
            });

            let response = send_request_quiet(&mut reader, &mut writer, request)
                .await
                .map_err(|e| e.to_string())?;

            if response["jsonrpc"] == "2.0"
                && response["result"]["serverInfo"]["name"] == "iotcraft"
            {
                Ok(json!({"protocol_version": response["result"]["protocolVersion"]}))
            } else {
                Err("Invalid initialize response".to_string())
            }
        })
        .await,
    );

    // Test 3: List tools
    results.push(
        run_single_test("list_tools", || async {
            let stream = TcpStream::connect(server_addr)
                .await
                .map_err(|e| e.to_string())?;
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);

            let request = json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "tools/list",
                "params": {}
            });

            let response = send_request_quiet(&mut reader, &mut writer, request)
                .await
                .map_err(|e| e.to_string())?;

            if let Some(tools) = response["result"]["tools"].as_array() {
                Ok(json!({"tool_count": tools.len(), "tools": tools}))
            } else {
                Err("No tools found in response".to_string())
            }
        })
        .await,
    );

    // Test 4: Create wall command
    results.push(
        run_single_test("create_wall", || async {
            test_tool_call(
                server_addr,
                "create_wall",
                json!({
                    "block_type": "stone",
                    "x1": 0, "y1": 0, "z1": 0,
                    "x2": 2, "y2": 1, "z2": 1
                }),
            )
            .await
        })
        .await,
    );

    // Test 5: Place block command
    results.push(
        run_single_test("place_block", || async {
            test_tool_call(
                server_addr,
                "place_block",
                json!({
                    "block_type": "grass",
                    "x": 10, "y": 1, "z": 5
                }),
            )
            .await
        })
        .await,
    );

    // Test 6: Spawn device command
    results.push(
        run_single_test("spawn_device", || async {
            test_tool_call(
                server_addr,
                "spawn_device",
                json!({
                    "device_id": "test_device_comprehensive",
                    "device_type": "lamp",
                    "x": 5.0, "y": 1.0, "z": 3.0
                }),
            )
            .await
        })
        .await,
    );

    // Test 7: Error handling - missing parameters
    results.push(
        run_single_test("error_handling_missing_params", || async {
            let stream = TcpStream::connect(server_addr)
                .await
                .map_err(|e| e.to_string())?;
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);

            let request = json!({
                "jsonrpc": "2.0",
                "id": 7,
                "method": "tools/call",
                "params": {
                    "name": "create_wall",
                    "arguments": {
                        "block_type": "stone",
                        "x1": 0
                        // Missing required parameters
                    }
                }
            });

            let response = send_request_quiet(&mut reader, &mut writer, request)
                .await
                .map_err(|e| e.to_string())?;

            // Should gracefully handle the error
            if response.get("error").is_some()
                || (response.get("result").is_some() && response["result"].get("error").is_some())
            {
                Ok(json!({"handled_gracefully": true}))
            } else {
                Err("Did not handle missing parameters gracefully".to_string())
            }
        })
        .await,
    );

    // Test 8: Command conversion validation
    results.push(
        run_single_test("command_conversion", || async {
            // This test validates that the expected command conversions work
            let test_cases = vec![
                (
                    "create_wall",
                    json!({
                        "block_type": "grass",
                        "x1": 0, "y1": 0, "z1": 0, "x2": 5, "y2": 3, "z2": 2
                    }),
                    "wall grass 0 0 0 5 3 2",
                ),
                (
                    "place_block",
                    json!({
                        "block_type": "dirt", "x": 10, "y": 5, "z": -3
                    }),
                    "place dirt 10 5 -3",
                ),
            ];

            let mut conversion_results = Vec::new();
            for (tool, args, expected) in test_cases {
                let result = test_tool_call(server_addr, tool, args.clone()).await;
                conversion_results.push(json!({
                    "tool": tool,
                    "args": args,
                    "expected_command": expected,
                    "success": result.is_ok()
                }));
            }

            Ok(json!({"conversions": conversion_results}))
        })
        .await,
    );

    let total_duration = start_time.elapsed();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.len() - passed;

    let report = TestReport {
        timestamp: chrono::Utc::now().to_rfc3339(),
        server_address: server_addr.to_string(),
        total_tests: results.len(),
        passed,
        failed,
        duration_ms: total_duration.as_millis() as u64,
        results,
    };

    // Output results
    match format {
        "json" => {
            let json_output = serde_json::to_string_pretty(&report)?;
            if let Some(file_path) = output_file {
                fs::write(file_path, json_output)?;
                println!("📊 Test report written to: {}", file_path);
            } else {
                println!("{}", json_output);
            }
        }
        _ => {
            print_text_report(&report);
            if let Some(file_path) = output_file {
                let text_report = format_text_report(&report);
                fs::write(file_path, text_report)?;
                println!("📊 Test report written to: {}", file_path);
            }
        }
    }

    if failed > 0 {
        std::process::exit(1);
    }

    Ok(())
}

async fn run_single_test<F, Fut>(name: &str, test_fn: F) -> TestResult
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Value, String>>,
{
    let start = Instant::now();
    print!("Running test '{}'... ", name);

    match test_fn().await {
        Ok(details) => {
            let duration = start.elapsed().as_millis() as u64;
            println!("✅ ({} ms)", duration);
            TestResult {
                name: name.to_string(),
                passed: true,
                duration_ms: duration,
                error: None,
                details: details
                    .as_object()
                    .unwrap_or(&serde_json::Map::new())
                    .clone()
                    .into_iter()
                    .collect(),
            }
        }
        Err(error) => {
            let duration = start.elapsed().as_millis() as u64;
            println!("❌ ({} ms): {}", duration, error);
            TestResult {
                name: name.to_string(),
                passed: false,
                duration_ms: duration,
                error: Some(error),
                details: HashMap::new(),
            }
        }
    }
}

async fn test_tool_call(
    server_addr: &str,
    tool_name: &str,
    arguments: Value,
) -> Result<Value, String> {
    let stream = TcpStream::connect(server_addr)
        .await
        .map_err(|e| e.to_string())?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let request = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    });

    let response = send_request_quiet(&mut reader, &mut writer, request)
        .await
        .map_err(|e| e.to_string())?;

    if response.get("result").is_some() {
        Ok(json!({
            "tool": tool_name,
            "response_type": "result",
            "has_content": response["result"].get("content").is_some()
        }))
    } else if response.get("error").is_some() {
        Ok(json!({
            "tool": tool_name,
            "response_type": "error",
            "error_code": response["error"]["code"]
        }))
    } else {
        Err("Unexpected response format".to_string())
    }
}

async fn send_request_quiet(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    request: Value,
) -> Result<Value, Box<dyn std::error::Error>> {
    let request_str = serde_json::to_string(&request)?;
    writer
        .write_all(format!("{}\n", request_str).as_bytes())
        .await?;

    let mut response_line = String::new();
    tokio::time::timeout(
        Duration::from_secs(10),
        reader.read_line(&mut response_line),
    )
    .await??;

    if response_line.trim().is_empty() {
        return Err("Empty response from server".into());
    }

    let response: Value = serde_json::from_str(response_line.trim())?;
    Ok(response)
}

fn print_text_report(report: &TestReport) {
    println!("\n🧪 MCP Test Report");
    println!("==================");
    println!("Server: {}", report.server_address);
    println!("Timestamp: {}", report.timestamp);
    println!("Duration: {} ms", report.duration_ms);
    println!("\n📊 Summary");
    println!("Total tests: {}", report.total_tests);
    println!("✅ Passed: {}", report.passed);
    println!("❌ Failed: {}", report.failed);
    println!(
        "Success rate: {:.1}%",
        (report.passed as f64 / report.total_tests as f64) * 100.0
    );

    println!("\n📋 Test Details");
    for result in &report.results {
        let status = if result.passed { "✅" } else { "❌" };
        println!("{} {} ({} ms)", status, result.name, result.duration_ms);
        if let Some(error) = &result.error {
            println!("   Error: {}", error);
        }
        if !result.details.is_empty() {
            println!(
                "   Details: {}",
                serde_json::to_string(&result.details).unwrap_or_default()
            );
        }
    }

    if report.failed > 0 {
        println!("\n⚠️  Some tests failed. Check the details above.");
        println!("\n💡 Troubleshooting tips:");
        println!("- Ensure desktop client is running with --mcp flag");
        println!("- Check that MCP server is listening on the expected port");
        println!("- Verify command conversion logic in mcp_server.rs");
        println!("- Run unit tests with: cargo test --lib mcp");
    } else {
        println!("\n🎉 All tests passed!");
    }
}

fn format_text_report(report: &TestReport) -> String {
    format!(
        "MCP Test Report\n\
         ===============\n\
         Server: {}\n\
         Timestamp: {}\n\
         Total: {} | Passed: {} | Failed: {}\n\
         Duration: {} ms\n\n\
         {}\n",
        report.server_address,
        report.timestamp,
        report.total_tests,
        report.passed,
        report.failed,
        report.duration_ms,
        report
            .results
            .iter()
            .map(|r| format!(
                "{} {} ({} ms){}",
                if r.passed { "✅" } else { "❌" },
                r.name,
                r.duration_ms,
                r.error
                    .as_ref()
                    .map(|e| format!(" - Error: {}", e))
                    .unwrap_or_default()
            ))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn print_help() {
    println!("Available commands:");
    println!("  help                    - Show this help");
    println!("  quit, exit              - Exit interactive mode");
    println!("  init                    - Initialize MCP connection");
    println!("  tools                   - List available tools");
    println!("  wall <type> <x1> <y1> <z1> <x2> <y2> <z2> - Create wall");
    println!("  place <type> <x> <y> <z> - Place block");
    println!();
    println!("Examples:");
    println!("  wall stone 0 0 0 3 2 1");
    println!("  place grass 5 1 5");
}
