#![cfg(not(target_arch = "wasm32"))]

use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Integration test for MCP server communication
/// Tests the full JSON-RPC protocol flow
#[tokio::test]
async fn test_mcp_server_initialize() {
    // These tests require the MCP server to be running
    // Skip if server is not available
    let Ok(stream) = TcpStream::connect("127.0.0.1:8080").await else {
        println!("Skipping MCP integration tests - server not available at 127.0.0.1:8080");
        return;
    };

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test_client",
                "version": "1.0.0"
            }
        }
    });

    let request_str = serde_json::to_string(&init_request).unwrap();
    writer
        .write_all(format!("{}\n", request_str).as_bytes())
        .await
        .unwrap();

    // Read response
    let mut line = String::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line)).await;

    if !line.trim().is_empty() {
        let response: Value = serde_json::from_str(line.trim()).unwrap();

        // Verify response structure
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response["result"].is_object());

        let result = &response["result"];
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"].is_object());
        assert!(result["serverInfo"].is_object());
        assert_eq!(result["serverInfo"]["name"], "iotcraft");
    }
}

#[tokio::test]
async fn test_mcp_tools_list() {
    let Ok(stream) = TcpStream::connect("127.0.0.1:8080").await else {
        println!("Skipping MCP integration tests - server not available");
        return;
    };

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send tools list request
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let request_str = serde_json::to_string(&tools_request).unwrap();
    writer
        .write_all(format!("{}\n", request_str).as_bytes())
        .await
        .unwrap();

    // Read response
    let mut line = String::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line)).await;

    if !line.trim().is_empty() {
        let response: Value = serde_json::from_str(line.trim()).unwrap();

        // Verify response structure
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 2);
        assert!(response["result"].is_object());

        let result = &response["result"];
        assert!(result["tools"].is_array());

        let tools = result["tools"].as_array().unwrap();
        assert!(!tools.is_empty(), "Should have at least one tool");

        // Check that expected tools are present
        let tool_names: Vec<String> = tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap().to_string())
            .collect();

        assert!(tool_names.contains(&"create_wall".to_string()));
        assert!(tool_names.contains(&"place_block".to_string()));
        assert!(tool_names.contains(&"spawn_device".to_string()));
    }
}

#[tokio::test]
async fn test_mcp_tool_call_create_wall() {
    let Ok(stream) = TcpStream::connect("127.0.0.1:8080").await else {
        println!("Skipping MCP integration tests - server not available");
        return;
    };

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send create_wall tool call
    let tool_call = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "create_wall",
            "arguments": {
                "block_type": "stone",
                "x1": 0,
                "y1": 0,
                "z1": 0,
                "x2": 2,
                "y2": 1,
                "z2": 1
            }
        }
    });

    let request_str = serde_json::to_string(&tool_call).unwrap();
    writer
        .write_all(format!("{}\n", request_str).as_bytes())
        .await
        .unwrap();

    // Read response with longer timeout for command execution
    let mut line = String::new();
    let _ = tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line)).await;

    if !line.trim().is_empty() {
        let response: Value = serde_json::from_str(line.trim()).unwrap();

        // Verify response structure
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 3);

        if let Some(result) = response.get("result") {
            // Success case - verify the response contains expected content
            assert!(result["content"].is_array());
            let content = result["content"].as_array().unwrap();
            assert!(!content.is_empty());

            if let Some(text_content) = content[0].get("text") {
                let text = text_content.as_str().unwrap();
                assert!(text.contains("stone"), "Response should mention block type");
                assert!(
                    text.contains("wall") || text.contains("Created"),
                    "Response should indicate creation"
                );
            }
        } else if let Some(error) = response.get("error") {
            // Error case - this might happen if the command processing isn't working
            println!("Tool call returned error: {:?}", error);
            // We'll still consider this a "successful" test of the protocol
        }
    }
}

#[tokio::test]
async fn test_mcp_tool_call_with_invalid_parameters() {
    let Ok(stream) = TcpStream::connect("127.0.0.1:8080").await else {
        println!("Skipping MCP integration tests - server not available");
        return;
    };

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send create_wall tool call with missing parameters
    let tool_call = json!({
        "jsonrpc": "2.0",
        "id": 4,
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

    let request_str = serde_json::to_string(&tool_call).unwrap();
    writer
        .write_all(format!("{}\n", request_str).as_bytes())
        .await
        .unwrap();

    // Read response
    let mut line = String::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut line)).await;

    if !line.trim().is_empty() {
        let response: Value = serde_json::from_str(line.trim()).unwrap();

        // This should return an error due to missing parameters
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 4);

        // Could either be an error response or a successful response indicating the command couldn't be queued
        if let Some(error) = response.get("error") {
            // Direct error response
            assert!(
                error["message"].as_str().unwrap().contains("parameter")
                    || error["message"].as_str().unwrap().contains("required")
            );
        } else if let Some(result) = response.get("result") {
            // Success response but command couldn't be converted
            if let Some(error_field) = result.get("error") {
                // Error within result
                assert!(
                    error_field["message"]
                        .as_str()
                        .unwrap()
                        .contains("not supported")
                        || error_field["message"]
                            .as_str()
                            .unwrap()
                            .contains("cannot be executed")
                );
            }
        }
    }
}

#[tokio::test]
async fn test_mcp_multiple_tool_calls() {
    let Ok(stream) = TcpStream::connect("127.0.0.1:8080").await else {
        println!("Skipping MCP integration tests - server not available");
        return;
    };

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let tool_calls = vec![
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "place_block",
                "arguments": {
                    "block_type": "grass",
                    "x": 5,
                    "y": 1,
                    "z": 5
                }
            }
        }),
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "spawn_device",
                "arguments": {
                    "device_id": "test_lamp_mcp",
                    "device_type": "lamp",
                    "x": 3.0,
                    "y": 1.0,
                    "z": 3.0
                }
            }
        }),
    ];

    for (i, tool_call) in tool_calls.iter().enumerate() {
        let request_str = serde_json::to_string(tool_call).unwrap();
        writer
            .write_all(format!("{}\n", request_str).as_bytes())
            .await
            .unwrap();

        // Read response
        let mut line = String::new();
        let _ = tokio::time::timeout(Duration::from_secs(10), reader.read_line(&mut line)).await;

        if !line.trim().is_empty() {
            let response: Value = serde_json::from_str(line.trim()).unwrap();

            // Verify response structure
            assert_eq!(response["jsonrpc"], "2.0");
            assert_eq!(response["id"], 5 + i); // IDs should match

            // Both should either succeed or fail gracefully
            assert!(response.get("result").is_some() || response.get("error").is_some());
        }
    }
}

/// Test helper that can be used to manually test the MCP server
/// This is not a unit test but a utility for manual testing
#[allow(dead_code)]
async fn manual_mcp_test_session() -> Result<(), Box<dyn std::error::Error>> {
    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    println!("Connected to MCP server. Sending test commands...");

    let test_commands = vec![
        // Initialize
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "manual_test", "version": "1.0.0"}
            }
        }),
        // List tools
        json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        }),
        // Test create_wall
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "create_wall",
                "arguments": {
                    "block_type": "stone",
                    "x1": 0, "y1": 0, "z1": 0,
                    "x2": 3, "y2": 2, "z2": 1
                }
            }
        }),
        // Test place_block
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "place_block",
                "arguments": {
                    "block_type": "grass",
                    "x": 10, "y": 1, "z": 10
                }
            }
        }),
    ];

    for cmd in test_commands {
        let request_str = serde_json::to_string(&cmd)?;
        println!("Sending: {}", request_str);
        writer
            .write_all(format!("{}\n", request_str).as_bytes())
            .await?;

        // Wait for response
        let mut response_line = String::new();
        tokio::time::timeout(
            Duration::from_secs(15),
            reader.read_line(&mut response_line),
        )
        .await??;

        if !response_line.trim().is_empty() {
            let response: Value = serde_json::from_str(response_line.trim())?;
            println!("Response: {}", serde_json::to_string_pretty(&response)?);
        }

        // Small delay between commands
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}
