use crate::{
    config::MqttConfig,
    devices::device_types::DeviceEntity,
    environment::VoxelWorld,
    mcp::{mcp_protocol::error_codes, mcp_tools::McpToolRegistry, mcp_types::*},
    mqtt::TemperatureResource,
    profile::PlayerProfile,
    script::script_types::PendingCommands,
    ui::main_menu::GameState,
    world::CreateWorldEvent,
};
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use chrono;
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

        // Add event types (CommandExecutedEvent is added unconditionally in main.rs)

        // Initialize pending tool executions resource
        app.init_resource::<PendingToolExecutions>();

        // Add systems
        app.add_systems(Startup, (start_mcp_server, setup_mcp_mqtt_client))
            .add_systems(
                Update,
                (
                    process_mcp_requests,
                    execute_mcp_commands,
                    handle_command_results,
                ),
            );

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

    // Wait for the response from Bevy with timeout (reduced to 10 seconds for faster error handling)
    let response_result =
        tokio::time::timeout(std::time::Duration::from_secs(10), response_rx).await;

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
            "ping" => {
                // Handle ping immediately - no queuing needed
                let response = json!({
                    "content": [{
                        "type": "text",
                        "text": "pong"
                    }],
                    "isError": false
                });
                if request.response_sender.send(response).is_err() {
                    error!("Failed to send ping response");
                }
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

/// Check if a tool should be queued as a command instead of executed directly
pub fn should_queue_as_command(tool_name: &str) -> bool {
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
            | "create_world"
            | "set_game_state"
            | "publish_world"
            | "unpublish_world"
            | "join_world"
            | "leave_world"
            | "list_online_worlds"
            | "get_multiplayer_status"
            | "get_world_status"
            | "wait_for_condition"
            | "get_client_info"
            | "get_game_state"
            | "health_check"
            | "get_system_info"
            | "get_sensor_data"
            | "list_world_templates"
    )
}

/// Convert MCP tool call to console command string
pub fn convert_tool_call_to_command(tool_name: &str, arguments: &Value) -> Option<String> {
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
            // Handle both integer and floating point coordinates by converting to f64 first
            let x = arguments.get("x")?.as_f64()? as i64;
            let y = arguments.get("y")?.as_f64()? as i64;
            let z = arguments.get("z")?.as_f64()? as i64;
            Some(format!("place {} {} {} {}", block_type, x, y, z))
        }
        "remove_block" => {
            // Handle both integer and floating point coordinates by converting to f64 first
            let x = arguments.get("x")?.as_f64()? as i64;
            let y = arguments.get("y")?.as_f64()? as i64;
            let z = arguments.get("z")?.as_f64()? as i64;
            Some(format!("remove {} {} {}", x, y, z))
        }
        "create_wall" => {
            let block_type = arguments.get("block_type")?.as_str()?;
            // Handle both integer and floating point coordinates by converting to f64 first
            let x1 = arguments.get("x1")?.as_f64()? as i64;
            let y1 = arguments.get("y1")?.as_f64()? as i64;
            let z1 = arguments.get("z1")?.as_f64()? as i64;
            let x2 = arguments.get("x2")?.as_f64()? as i64;
            let y2 = arguments.get("y2")?.as_f64()? as i64;
            let z2 = arguments.get("z2")?.as_f64()? as i64;
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
        "load_world_by_file" => {
            let filename = arguments.get("filename")?.as_str()?;
            Some(format!("load_map {}", filename))
        }
        "publish_world" => {
            let world_name = arguments
                .get("world_name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let max_players = arguments
                .get("max_players")
                .and_then(|v| v.as_u64())
                .unwrap_or(4);
            let is_public = arguments
                .get("is_public")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            Some(format!(
                "publish_world {} {} {}",
                world_name, max_players, is_public
            ))
        }
        "unpublish_world" => Some("unpublish_world".to_string()),
        "join_world" => {
            let world_id = arguments.get("world_id")?.as_str()?;
            Some(format!("join_world {}", world_id))
        }
        "leave_world" => Some("leave_world".to_string()),
        "list_online_worlds" => Some("list_online_worlds".to_string()),
        "get_multiplayer_status" => Some("get_multiplayer_status".to_string()),
        "wait_for_condition" => {
            let condition = arguments.get("condition")?.as_str()?;
            let timeout = arguments
                .get("timeout_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(30);
            let expected = arguments
                .get("expected_value")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if expected.is_empty() {
                Some(format!("wait_for_condition {} {}", condition, timeout))
            } else {
                Some(format!(
                    "wait_for_condition {} {} {}",
                    condition, timeout, expected
                ))
            }
        }
        "create_world" => {
            let world_name = arguments.get("world_name")?.as_str()?;
            let description = arguments
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("A new world created via MCP");
            Some(format!("create_world {} {}", world_name, description))
        }
        "load_world_by_name" => {
            let world_name = arguments.get("world_name")?.as_str()?;
            Some(format!("load_world {}", world_name))
        }
        "set_game_state" => {
            let state = arguments.get("state")?.as_str()?;
            Some(format!("set_game_state {}", state))
        }
        "get_client_info" => Some("get_client_info".to_string()),
        "get_game_state" => Some("get_game_state".to_string()),
        "health_check" => Some("health_check".to_string()),
        "get_system_info" => Some("get_system_info".to_string()),
        "get_sensor_data" => Some("get_sensor_data".to_string()),
        "get_world_status" => Some("get_world_status".to_string()),
        "player_move" => {
            let x = arguments.get("x")?.as_f64()?;
            let y = arguments.get("y")?.as_f64()?;
            let z = arguments.get("z")?.as_f64()?;
            Some(format!("player_move {} {} {}", x, y, z))
        }
        "load_world" => {
            let world_name = arguments.get("world_name")?.as_str()?;
            Some(format!("load_world {}", world_name))
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

/// Handle async tool call request - execute directly without shared queue
fn handle_async_tool_call_request(
    request: McpRequest,
    _pending_commands: &mut PendingCommands, // No longer used for MCP
    pending_executions: &mut PendingToolExecutions,
) {
    debug!("Handling MCP tool call request: {}", request.params);

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

    // Handle ping tool call directly without queueing
    if tool_name == "ping" {
        debug!("Handling ping tool call directly");
        let response = json!({
            "content": [{
                "type": "text",
                "text": "pong"
            }],
            "isError": false
        });
        if request.response_sender.send(response).is_err() {
            error!("Failed to send ping tool response");
        }
        return;
    }

    // Generate a unique request ID for tracking
    let request_id = uuid::Uuid::new_v4().to_string();
    info!(
        "Handling MCP tool '{}' with request ID: {}",
        tool_name, request_id
    );

    // Store the pending execution for response tracking
    pending_executions.executions.insert(
        request_id.clone(),
        PendingToolExecution {
            response_sender: request.response_sender,
        },
    );

    // Create MCP command execution request
    let mcp_command = McpCommandExecution {
        request_id: request_id.clone(),
        tool_name: tool_name.to_string(),
        arguments,
    };

    // Add to dedicated MCP execution queue
    pending_executions.mcp_commands.push(mcp_command);
    info!(
        "Queued MCP command '{}' for execution (queue size: {})",
        tool_name,
        pending_executions.mcp_commands.len()
    );
}

/// Dedicated MCP command execution system (separate from script system)
fn execute_mcp_commands(
    mut pending_executions: ResMut<PendingToolExecutions>,
    mut command_executed_events: EventWriter<CommandExecutedEvent>,
    // Import required resources for command execution
    temperature: Res<TemperatureResource>,
    mqtt_config: Res<MqttConfig>,
    voxel_world: Res<VoxelWorld>,
    device_query: Query<(&DeviceEntity, &Transform), Without<Camera>>,
    mut create_world_events: EventWriter<CreateWorldEvent>,
    mut next_game_state: Option<ResMut<NextState<GameState>>>,
    // Add multiplayer resources for better MCP responses
    multiplayer_mode: Option<Res<crate::multiplayer::shared_world::MultiplayerMode>>,
    online_worlds: Option<Res<crate::multiplayer::shared_world::OnlineWorlds>>,
    player_positions: Option<Res<crate::multiplayer::shared_world::MultiplayerPlayerPositions>>,
    mut player_move_events: EventWriter<crate::multiplayer::shared_world::PlayerMoveEvent>,
) {
    // Add comprehensive debug logging
    use std::sync::atomic::{AtomicU64, Ordering};
    static DEBUG_COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
    if counter % 300 == 0 {
        // Log every 5 seconds at 60fps
        info!(
            "[DEBUG] execute_mcp_commands system running, tick {}, queue size: {}",
            counter,
            pending_executions.mcp_commands.len()
        );
    }

    if !pending_executions.mcp_commands.is_empty() {
        info!(
            "[DEBUG] Processing {} MCP commands from queue",
            pending_executions.mcp_commands.len()
        );
    }

    // Process all queued MCP commands
    for mcp_command in pending_executions.mcp_commands.drain(..) {
        info!(
            "Executing MCP command: {} (ID: {})",
            mcp_command.tool_name, mcp_command.request_id
        );

        let result = execute_mcp_command_directly(
            &mcp_command.tool_name,
            &mcp_command.arguments,
            &temperature,
            &mqtt_config,
            &voxel_world,
            &device_query,
            &mut create_world_events,
            &mut next_game_state,
            multiplayer_mode.as_deref(),
            online_worlds.as_deref(),
            player_positions.as_deref(),
            &mut player_move_events,
        );

        // Emit the result as CommandExecutedEvent
        command_executed_events.write(CommandExecutedEvent {
            request_id: mcp_command.request_id,
            result,
        });
    }
}

/// Execute MCP command directly with access to game resources
fn execute_mcp_command_directly(
    tool_name: &str,
    arguments: &serde_json::Value,
    temperature: &TemperatureResource,
    _mqtt_config: &MqttConfig,
    voxel_world: &VoxelWorld,
    device_query: &Query<(&DeviceEntity, &Transform), Without<Camera>>,
    create_world_events: &mut EventWriter<CreateWorldEvent>,
    next_game_state: &mut Option<ResMut<NextState<GameState>>>,
    multiplayer_mode: Option<&crate::multiplayer::shared_world::MultiplayerMode>,
    online_worlds: Option<&crate::multiplayer::shared_world::OnlineWorlds>,
    player_positions: Option<&crate::multiplayer::shared_world::MultiplayerPlayerPositions>,
    player_move_events: &mut EventWriter<crate::multiplayer::shared_world::PlayerMoveEvent>,
) -> String {
    use crate::multiplayer::shared_world::MultiplayerMode;

    match tool_name {
        "get_client_info" => json!({
            "client_id": crate::profile::load_or_create_profile_with_override(None).player_id,
            "version": "1.0.0",
            "status": "ready",
            "capabilities": ["world_building", "device_management", "mqtt_integration"]
        })
        .to_string(),
        "get_game_state" => {
            json!({
                "game_state": "InGame", // This should get the actual game state
                "world_loaded": true,
                "multiplayer_active": false
            })
            .to_string()
        }
        "health_check" => {
            json!({
                "status": "healthy",
                "uptime_seconds": 3600, // This should be calculated properly
                "memory_usage_mb": 256,  // This should be actual memory usage
                "services_running": ["mqtt_client", "mcp_server"]
            })
            .to_string()
        }
        "get_system_info" => json!({
            "platform": std::env::consts::OS,
            "architecture": std::env::consts::ARCH,
            "rust_version": env!("CARGO_PKG_RUST_VERSION"),
            "app_version": env!("CARGO_PKG_VERSION")
        })
        .to_string(),
        "get_world_status" => {
            let block_count = voxel_world.blocks.len();
            let device_count = device_query.iter().count();

            json!({
                "blocks": block_count,
                "devices": device_count,
                "uptime_seconds": 3600, // Should be calculated properly
                "world_name": "Default World"
            })
            .to_string()
        }
        "get_sensor_data" => json!({
            "temperature": temperature.value,
            "devices_online": device_query.iter().count(),
            "mqtt_connected": temperature.value.is_some()
        })
        .to_string(),
        "list_world_templates" => {
            // List available world templates from scripts/world_templates/
            let templates_dir = std::path::Path::new("scripts/world_templates");
            if templates_dir.exists() {
                match std::fs::read_dir(templates_dir) {
                    Ok(entries) => {
                        let mut templates = Vec::new();
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_file()
                                && path.extension().map(|ext| ext == "txt").unwrap_or(false)
                            {
                                if let Some(template_name) =
                                    path.file_stem().and_then(|s| s.to_str())
                                {
                                    // Read first few lines to get description
                                    let description = if let Ok(content) =
                                        std::fs::read_to_string(&path)
                                    {
                                        content
                                            .lines()
                                            .find(|line| {
                                                line.starts_with("# ") && !line.contains("Template")
                                            })
                                            .map(|line| line.trim_start_matches("# "))
                                            .unwrap_or("World template")
                                            .to_string()
                                    } else {
                                        "World template".to_string()
                                    };

                                    templates.push(json!({
                                        "name": template_name,
                                        "description": description,
                                        "file": format!("{}.txt", template_name)
                                    }));
                                }
                            }
                        }
                        templates.sort_by(|a, b| {
                            a["name"]
                                .as_str()
                                .unwrap_or("")
                                .cmp(b["name"].as_str().unwrap_or(""))
                        });

                        json!({
                            "templates": templates,
                            "count": templates.len()
                        })
                        .to_string()
                    }
                    Err(e) => format!("Error reading templates directory: {}", e),
                }
            } else {
                "Error: templates directory not found at scripts/world_templates/".to_string()
            }
        }
        "create_world" => {
            if let Some(world_name) = arguments.get("world_name").and_then(|v| v.as_str()) {
                let description = arguments
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("A new world created via MCP");
                let template = arguments
                    .get("template")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");

                info!(
                    "Creating world via MCP: name='{}', description='{}', template='{}'",
                    world_name, description, template
                );

                // Validate template exists
                let template_path = format!("scripts/world_templates/{}.txt", template);
                if !std::path::Path::new(&template_path).exists() {
                    return format!(
                        "Error: Template '{}' not found. Available templates: default, medieval, modern, creative",
                        template
                    );
                }

                // Send CreateWorldEvent with template info to trigger world creation
                create_world_events.write(CreateWorldEvent {
                    world_name: world_name.to_string(),
                    description: description.to_string(),
                    template: Some(template.to_string()),
                });

                // Set game state to InGame to transition UI from main menu
                if let Some(next_state) = next_game_state.as_mut() {
                    next_state.set(crate::ui::main_menu::GameState::InGame);
                    info!("Set game state to InGame for world creation transition");
                }

                format!(
                    "Created new world: {} ({}) using template '{}' and transitioned to InGame",
                    world_name, description, template
                )
            } else {
                "Error: world_name is required for create_world".to_string()
            }
        }
        "player_move" => {
            if let (Some(x), Some(y), Some(z)) = (
                arguments.get("x").and_then(|v| v.as_f64()),
                arguments.get("y").and_then(|v| v.as_f64()),
                arguments.get("z").and_then(|v| v.as_f64()),
            ) {
                // Emit PlayerMoveEvent for immediate processing
                player_move_events.write(crate::multiplayer::shared_world::PlayerMoveEvent {
                    x: x as f32,
                    y: y as f32,
                    z: z as f32,
                });
                format!("Player moved to ({}, {}, {})", x, y, z)
            } else {
                "Error: player_move requires x, y, z coordinates".to_string()
            }
        }
        "list_online_worlds" => {
            if let Some(worlds) = online_worlds {
                if worlds.worlds.is_empty() {
                    json!({
                        "online_worlds": [],
                        "message": "No online worlds found.",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    })
                    .to_string()
                } else {
                    let world_list: Vec<serde_json::Value> = worlds
                        .worlds
                        .iter()
                        .map(|(world_id, world_info)| {
                            json!({
                                "world_id": world_id,
                                "world_name": world_info.world_name,
                                "host_name": world_info.host_name,
                                "player_count": world_info.player_count,
                                "max_players": world_info.max_players,
                                "is_public": world_info.is_public
                            })
                        })
                        .collect();

                    json!({
                        "online_worlds": world_list,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    })
                    .to_string()
                }
            } else {
                json!({
                    "online_worlds": [],
                    "message": "Online worlds resource not available",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
                .to_string()
            }
        }
        "get_multiplayer_status" => {
            if let Some(mode) = multiplayer_mode {
                let (multiplayer_mode_str, world_id, is_published, host_player) = match mode {
                    MultiplayerMode::SinglePlayer => {
                        ("SinglePlayer".to_string(), None, false, None)
                    }
                    MultiplayerMode::HostingWorld {
                        world_id,
                        is_published,
                    } => (
                        "HostingWorld".to_string(),
                        Some(world_id.clone()),
                        *is_published,
                        None,
                    ),
                    MultiplayerMode::JoinedWorld {
                        world_id,
                        host_player,
                    } => (
                        "JoinedWorld".to_string(),
                        Some(world_id.clone()),
                        false,
                        Some(host_player.clone()),
                    ),
                };

                // Include player positions if available
                let player_positions_json = if let Some(positions) = player_positions {
                    let positions_list: Vec<serde_json::Value> = positions
                        .positions
                        .values()
                        .map(|pos| {
                            json!({
                                "player_id": pos.player_id,
                                "player_name": pos.player_name,
                                "x": pos.x,
                                "y": pos.y,
                                "z": pos.z,
                                "last_updated": pos.last_updated
                            })
                        })
                        .collect();
                    positions_list
                } else {
                    vec![]
                };

                json!({
                    "multiplayer_mode": multiplayer_mode_str,
                    "world_id": world_id,
                    "is_published": is_published,
                    "host_player": host_player,
                    "player_positions": player_positions_json,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
                .to_string()
            } else {
                json!({
                    "multiplayer_mode": "SinglePlayer",
                    "world_id": null,
                    "is_published": false,
                    "host_player": null,
                    "error": "Multiplayer mode resource not available",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
                .to_string()
            }
        }
        _ => {
            format!("Error: Unknown MCP command: {}", tool_name)
        }
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
