use crate::{
    config::MqttConfig,
    mcp::{mcp_protocol::error_codes, mcp_tools::McpToolRegistry, mcp_types::*},
    profile::PlayerProfile,
    script::script_types::PendingCommands,
};
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use log::{debug, error, info};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// MCP Server Plugin for Bevy
pub struct McpPlugin;

impl Plugin for McpPlugin {
    fn build(&self, app: &mut App) {
        // Initialize MCP resources
        app.init_resource::<McpServerState>()
            .init_resource::<McpToolRegistry>()
            .insert_resource(McpMqttClient { client: None });

        // Create communication channels using async_channel
        let (req_tx, req_rx) = async_channel::unbounded();
        app.insert_resource(McpRequestChannel {
            receiver: req_rx,
            sender: req_tx,
        });

        // Add event types
        app.add_event::<McpToolExecutionEvent>()
            .add_event::<CommandExecutedEvent>();

        // Initialize pending tool executions resource
        app.init_resource::<PendingToolExecutions>();

        // Add systems
        app.add_systems(Startup, (start_mcp_server, setup_mcp_mqtt_client))
            .add_systems(Update, (process_mcp_requests, handle_command_results));

        info!("MCP Plugin initialized");
    }
}

/// Startup system to launch the MCP server using Bevy's AsyncComputeTaskPool
fn start_mcp_server(
    mut server_state: ResMut<McpServerState>,
    request_channel: Res<McpRequestChannel>,
) {
    let sender = request_channel.sender.clone();
    let task_pool = AsyncComputeTaskPool::get();

    // Spawn the MCP server task with its own Tokio runtime
    let task = task_pool.spawn(async move {
        // Create a Tokio runtime for the MCP server
        match tokio::runtime::Runtime::new() {
            Ok(rt) => {
                rt.block_on(async {
                    if let Err(e) = run_mcp_server(sender).await {
                        error!("MCP server error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to create Tokio runtime for MCP server: {}", e);
            }
        }
    });

    server_state.server_task = Some(task);
    info!("MCP server started using Bevy AsyncComputeTaskPool");
}

/// Main MCP server implementation using TCP JSON-RPC
async fn run_mcp_server(
    request_sender: async_channel::Sender<McpRequest>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = std::env::var("MCP_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    info!("Starting MCP JSON-RPC server on port {}", port);

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    info!("MCP server listening on 127.0.0.1:{}", port);

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("New MCP connection from: {}", addr);
                let sender_clone = request_sender.clone();

                // Spawn a task to handle this connection
                tokio::spawn(async move {
                    if let Err(e) = handle_mcp_connection(stream, sender_clone).await {
                        error!("Error handling MCP connection from {}: {}", addr, e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

/// Handle a TCP connection for MCP JSON-RPC
async fn handle_mcp_connection(
    stream: TcpStream,
    request_sender: async_channel::Sender<McpRequest>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF reached, client disconnected
                info!("MCP client disconnected");
                break;
            }
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                debug!("MCP server received: {}", line);

                // Parse JSON-RPC request
                match serde_json::from_str::<serde_json::Value>(line) {
                    Ok(json_request) => {
                        // Check if this is a notification (no id field)
                        if json_request.get("id").is_none() {
                            debug!(
                                "Received notification: {}",
                                json_request.get("method").unwrap_or(&json!("unknown"))
                            );
                            // For notifications, just handle them but don't send a response
                            if let Some(method) =
                                json_request.get("method").and_then(|m| m.as_str())
                            {
                                match method {
                                    "notifications/initialized" => {
                                        info!(
                                            "MCP client initialization notification received - connection ready"
                                        );
                                    }
                                    _ => {
                                        warn!("Unknown notification method: {}", method);
                                    }
                                }
                            }
                            // Don't send a response for notifications
                            continue;
                        }

                        let response = handle_json_rpc_request(json_request, &request_sender).await;

                        // Send response
                        let response_str = serde_json::to_string(&response)?;
                        debug!("MCP server sending: {}", response_str);

                        writer.write_all(response_str.as_bytes()).await?;
                        writer.write_all(b"\n").await?;
                    }
                    Err(e) => {
                        error!("Failed to parse JSON-RPC request: {}", e);

                        let error_response = json!({
                            "jsonrpc": "2.0",
                            "id": null,
                            "error": {
                                "code": -32700, // Parse error
                                "message": format!("Parse error: {}", e)
                            }
                        });

                        let error_str = serde_json::to_string(&error_response)?;
                        writer.write_all(error_str.as_bytes()).await?;
                        writer.write_all(b"\n").await?;
                    }
                }
            }
            Err(e) => {
                error!("Failed to read from TCP stream: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Handle JSON-RPC request from TCP connection
async fn handle_json_rpc_request(
    request: serde_json::Value,
    request_sender: &async_channel::Sender<McpRequest>,
) -> serde_json::Value {
    // Parse the JSON-RPC request
    let method = match request.get("method").and_then(|m| m.as_str()) {
        Some(m) => m,
        None => {
            return json!({
                "jsonrpc": "2.0",
                "id": request.get("id"),
                "error": {
                    "code": -32600, // Invalid Request
                    "message": "Missing method field"
                }
            });
        }
    };

    let id = request.get("id").cloned();
    let params = request.get("params").cloned().unwrap_or(json!({}));

    // Handle notifications (no response expected)
    if id.is_none() {
        debug!("Received notification: {}", method);
        // For notifications, we don't send a response
        return json!({});
    }

    // Create a response channel for this request
    let (response_tx, response_rx) = tokio::sync::oneshot::channel();

    // Package the request for the Bevy system
    let mcp_request = McpRequest {
        id: id
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        method: method.to_string(),
        params,
        response_sender: response_tx,
    };

    // Send to Bevy for processing
    if let Err(e) = request_sender.send(mcp_request).await {
        error!("Failed to send MCP request to Bevy: {}", e);
        return json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32603, // Internal error
                "message": "Internal server error"
            }
        });
    }

    // Wait for the response from Bevy with timeout
    let response_result =
        tokio::time::timeout(std::time::Duration::from_secs(30), response_rx).await;

    match response_result {
        Ok(Ok(result)) => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            })
        }
        Ok(Err(_)) => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603, // Internal error
                    "message": "Response channel closed"
                }
            })
        }
        Err(_) => {
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603, // Internal error
                    "message": "Request timeout"
                }
            })
        }
    }
}

/// System to process MCP requests in Bevy's main thread
fn process_mcp_requests(
    request_channel: ResMut<McpRequestChannel>,
    _tool_execution_events: EventWriter<McpToolExecutionEvent>,
    tool_registry: Res<McpToolRegistry>,
    mut pending_commands: ResMut<PendingCommands>,
    mut pending_executions: ResMut<PendingToolExecutions>,
) {
    // Process all pending requests
    while let Ok(request) = request_channel.receiver.try_recv() {
        debug!("Processing MCP request: {}", request.method);

        match request.method.as_str() {
            "initialize" => {
                let response = handle_initialize_request(request.params);
                if request.response_sender.send(response).is_err() {
                    error!("Failed to send initialize response");
                }
            }
            "tools/list" => {
                let response = handle_tools_list_request(&tool_registry);
                if request.response_sender.send(response).is_err() {
                    error!("Failed to send tools list response");
                }
            }
            "tools/call" => {
                // Handle tool calls with async execution
                handle_async_tool_call_request(
                    request,
                    &mut pending_commands,
                    &mut pending_executions,
                );
            }
            "resources/list" => {
                let response = handle_resources_list_request();
                if request.response_sender.send(response).is_err() {
                    error!("Failed to send resources list response");
                }
            }
            _ => {
                let error_response = json!({
                    "error": {
                        "code": error_codes::METHOD_NOT_FOUND,
                        "message": format!("Method '{}' not found", request.method)
                    }
                });
                if request.response_sender.send(error_response).is_err() {
                    error!("Failed to send method not found response");
                }
            }
        }
    }
}

/// Handle MCP initialize request
fn handle_initialize_request(_params: Value) -> Value {
    info!("Handling MCP initialize request");

    // Build response with camelCase field names for mcp-remote compatibility
    json!({
        "protocolVersion": MCP_VERSION,
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": "iotcraft",
            "version": "1.0.0"
        }
    })
}

/// Handle tools/list request
fn handle_tools_list_request(tool_registry: &McpToolRegistry) -> Value {
    debug!("Handling tools list request");

    json!({
        "tools": tool_registry.tools
    })
}

/// Handle tools/call request
fn handle_tool_call_request(params: Value, pending_commands: &mut PendingCommands) -> Value {
    debug!("Handling tool call request: {}", params);

    let tool_name = match params.get("name").and_then(|n| n.as_str()) {
        Some(name) => name,
        None => {
            return json!({
                "error": {
                    "code": error_codes::INVALID_PARAMS,
                    "message": "Tool name is required"
                }
            });
        }
    };

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    // For certain tools, we queue commands instead of executing immediately
    // This maintains compatibility with the existing command system
    if should_queue_as_command(tool_name) {
        let command = convert_tool_call_to_command(tool_name, &arguments);
        if let Some(cmd) = command {
            pending_commands.commands.push(cmd);
            return json!({
                "content": [{
                    "type": "text",
                    "text": format!("Queued command for tool '{}'", tool_name)
                }]
            });
        }
    }

    // For now, we only support queued commands as most tools need world access
    // Read-only tools could be implemented here in the future
    json!({
        "error": {
            "code": error_codes::METHOD_NOT_FOUND,
            "message": format!("Tool '{}' must be queued for execution", tool_name)
        }
    })
}

/// Check if a tool should be queued as a command instead of executed directly
fn should_queue_as_command(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "list_devices"
            | "control_device"
            | "spawn_device"
            | "place_block"
            | "remove_block"
            | "create_wall"
            | "move_device"
            | "teleport_camera"
            | "set_camera_angle"
            | "save_world"
            | "load_world"
    )
}

/// Convert MCP tool call to console command string
fn convert_tool_call_to_command(tool_name: &str, arguments: &Value) -> Option<String> {
    match tool_name {
        "list_devices" => {
            // List devices command
            Some("list".to_string())
        }
        "control_device" => {
            let device_id = arguments.get("device_id")?.as_str()?;
            let command = arguments.get("command")?.as_str()?;
            // Convert to device control command
            Some(format!("control {} {}", device_id, command))
        }
        "spawn_device" => {
            let device_id = arguments.get("device_id")?.as_str()?;
            let device_type = arguments.get("device_type")?.as_str()?;
            // For minimal spawn_device, use default position (0,1,0) if coordinates not provided
            let x = arguments.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let y = arguments.get("y").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let z = arguments.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0);
            match device_type {
                "lamp" => Some(format!("spawn {} {} {} {}", device_id, x, y, z)),
                "door" => Some(format!("spawn_door {} {} {} {}", device_id, x, y, z)),
                _ => None,
            }
        }
        "place_block" => {
            let block_type = arguments.get("block_type")?.as_str()?;
            let x = arguments.get("x")?.as_i64()?;
            let y = arguments.get("y")?.as_i64()?;
            let z = arguments.get("z")?.as_i64()?;
            Some(format!("place {} {} {} {}", block_type, x, y, z))
        }
        "remove_block" => {
            let x = arguments.get("x")?.as_i64()?;
            let y = arguments.get("y")?.as_i64()?;
            let z = arguments.get("z")?.as_i64()?;
            Some(format!("remove {} {} {}", x, y, z))
        }
        "create_wall" => {
            let block_type = arguments.get("block_type")?.as_str()?;
            let x1 = arguments.get("x1")?.as_i64()?;
            let y1 = arguments.get("y1")?.as_i64()?;
            let z1 = arguments.get("z1")?.as_i64()?;
            let x2 = arguments.get("x2")?.as_i64()?;
            let y2 = arguments.get("y2")?.as_i64()?;
            let z2 = arguments.get("z2")?.as_i64()?;
            Some(format!(
                "wall {} {} {} {} {} {} {}",
                block_type, x1, y1, z1, x2, y2, z2
            ))
        }
        "move_device" => {
            let device_id = arguments.get("device_id")?.as_str()?;
            let x = arguments.get("x")?.as_f64()?;
            let y = arguments.get("y")?.as_f64()?;
            let z = arguments.get("z")?.as_f64()?;
            Some(format!("move {} {} {} {}", device_id, x, y, z))
        }
        "teleport_camera" => {
            let x = arguments.get("x")?.as_f64()?;
            let y = arguments.get("y")?.as_f64()?;
            let z = arguments.get("z")?.as_f64()?;
            Some(format!("tp {} {} {}", x, y, z))
        }
        "set_camera_angle" => {
            let yaw = arguments.get("yaw")?.as_f64()?;
            let pitch = arguments.get("pitch")?.as_f64()?;
            Some(format!("look {} {}", yaw, pitch))
        }
        "save_world" => {
            let filename = arguments.get("filename")?.as_str()?;
            Some(format!("save_map {}", filename))
        }
        "load_world" => {
            let filename = arguments.get("filename")?.as_str()?;
            Some(format!("load_map {}", filename))
        }
        _ => None,
    }
}

/// Setup MQTT client for MCP tools
fn setup_mcp_mqtt_client(
    mut mcp_mqtt: ResMut<McpMqttClient>,
    mqtt_config: Res<MqttConfig>,
    profile: Res<PlayerProfile>,
) {
    use rumqttc::{Client, MqttOptions};
    use std::time::Duration;

    let client_id = format!("desktop-{}-mcp", profile.player_id);
    let mut mqtt_options = MqttOptions::new(&client_id, &mqtt_config.host, mqtt_config.port);
    mqtt_options.set_keep_alive(Duration::from_secs(5));

    let (client, _connection) = Client::new(mqtt_options, 10);
    mcp_mqtt.client = Some(client);
    info!("MCP MQTT client initialized for device control");
}

/// Handle async tool call request - queues command and waits for execution result
fn handle_async_tool_call_request(
    request: McpRequest,
    pending_commands: &mut PendingCommands,
    pending_executions: &mut PendingToolExecutions,
) {
    debug!("Handling async tool call request: {}", request.params);

    let tool_name = match request.params.get("name").and_then(|n| n.as_str()) {
        Some(name) => name,
        None => {
            let error_response = json!({
                "error": {
                    "code": error_codes::INVALID_PARAMS,
                    "message": "Tool name is required"
                }
            });
            if request.response_sender.send(error_response).is_err() {
                error!("Failed to send error response");
            }
            return;
        }
    };

    let arguments = request
        .params
        .get("arguments")
        .cloned()
        .unwrap_or(json!({}));

    // For tools that should be queued, convert to command and track execution
    if should_queue_as_command(tool_name) {
        if let Some(cmd) = convert_tool_call_to_command(tool_name, &arguments) {
            info!("Queueing MCP command: {} for tool {}", cmd, tool_name);

            // Generate a unique request ID for tracking
            let request_id = uuid::Uuid::new_v4().to_string();

            // Store the pending execution for later response
            pending_executions.executions.insert(
                request_id.clone(),
                PendingToolExecution {
                    request_id: request_id.clone(),
                    tool_name: tool_name.to_string(),
                    command: cmd.clone(),
                    response_sender: request.response_sender,
                },
            );

            // Add command with request ID for tracking
            pending_commands
                .commands
                .push(format!("{} #{}", cmd, request_id));

            return;
        }
    }

    // If we can't queue the command, return an error
    let error_response = json!({
        "error": {
            "code": error_codes::METHOD_NOT_FOUND,
            "message": format!("Tool '{}' is not supported or cannot be executed", tool_name)
        }
    });
    if request.response_sender.send(error_response).is_err() {
        error!("Failed to send error response");
    }
}

/// System to handle command execution results and send responses back to MCP clients
fn handle_command_results(
    mut pending_executions: ResMut<PendingToolExecutions>,
    mut command_executed_events: EventReader<CommandExecutedEvent>,
) {
    for event in command_executed_events.read() {
        if let Some(execution) = pending_executions.executions.remove(&event.request_id) {
            info!(
                "Sending MCP response for request {}: {}",
                event.request_id, event.result
            );

            // Use proper McpToolResult struct for serialization
            let tool_result = McpToolResult {
                content: vec![McpContent::Text {
                    text: event.result.clone(),
                }],
                is_error: Some(false),
            };

            let response = serde_json::to_value(tool_result).unwrap_or_else(|_| {
                json!({
                    "content": [{
                        "type": "text",
                        "text": event.result
                    }]
                })
            });

            if execution.response_sender.send(response).is_err() {
                error!("Failed to send command result response");
            }
        }
    }
}

/// Handle resources/list request
fn handle_resources_list_request() -> Value {
    debug!("Handling resources list request");

    // For now, return an empty list of resources
    // This can be extended to provide access to world files, device configs, etc.
    json!({
        "resources": []
    })
}
