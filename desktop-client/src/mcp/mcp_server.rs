use crate::{
    config::MqttConfig,
    devices::device_types::DeviceEntity,
    environment::VoxelWorld,
    mcp::{mcp_protocol::error_codes, mcp_tools::McpToolRegistry, mcp_types::*},
    mqtt::TemperatureResource,
    profile::PlayerProfile,
    script::script_types::PendingCommands,
    ui::main_menu::GameState,
    world::{CreateWorldEvent, LoadWorldEvent},
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
            .init_resource::<McpStateTransition>()
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
            // Add update systems separately to avoid scheduling tuple type issues
            .add_systems(Update, process_mcp_requests)
            .add_systems(Update, handle_command_results)
            .add_systems(Update, execute_mcp_commands)
            .add_systems(Update, sync_block_visuals);

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

/// Dedicated MCP command execution system with parameter bundling to restore full functionality
/// This replaces the simplified version and supports all multiplayer commands
fn execute_mcp_commands(
    mut core_params: super::mcp_params::CoreMcpParams,
    mut world_params: super::mcp_params::WorldMcpParams,
    mut multiplayer_params: super::mcp_params::MultiplayerMcpParams,
    mut entity_params: super::mcp_params::EntityMcpParams,
    mut state_params: super::mcp_params::McpStateMcpParams,
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
            core_params.pending_executions.mcp_commands.len()
        );
    }

    if !core_params.pending_executions.mcp_commands.is_empty() {
        info!(
            "[DEBUG] Processing {} MCP commands from queue",
            core_params.pending_executions.mcp_commands.len()
        );
    }

    // Take all queued MCP commands to avoid multiple mutable borrows
    let mcp_commands: Vec<_> = core_params.pending_executions.mcp_commands.drain(..).collect();
    
    // Process all queued MCP commands
    for mcp_command in mcp_commands {
        info!(
            "Executing MCP command: {} (ID: {})",
            mcp_command.tool_name, mcp_command.request_id
        );

        // Execute MCP command with full functionality restored via parameter bundling
        let result = super::mcp_commands::execute_mcp_command_bundled(
            &mcp_command.tool_name,
            &mcp_command.arguments,
            &mut core_params,
            &mut world_params,
            &mut multiplayer_params,
            &mut entity_params,
            &mut state_params,
        );

        // Emit the result as CommandExecutedEvent
        core_params.command_executed_events.write(CommandExecutedEvent {
            request_id: mcp_command.request_id,
            result,
        });
    }
}

/// System to synchronize visual block entities with VoxelWorld data
/// This ensures blocks added via MCP get visual representation
fn sync_block_visuals(
    voxel_world: Res<VoxelWorld>,
    existing_blocks_query: Query<&crate::environment::VoxelBlock>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Only run this sync when voxel world changes to avoid performance issues
    if !voxel_world.is_changed() {
        return;
    }

    // Get all existing visual block positions
    let existing_positions: std::collections::HashSet<bevy::math::IVec3> = existing_blocks_query
        .iter()
        .map(|block| block.position)
        .collect();

    // Create visual entities for blocks that don't have them
    let mut created_visuals = 0;
    for (pos, block_type) in voxel_world.blocks.iter() {
        if !existing_positions.contains(pos) {
            // Create visual entity for this block
            let cube_mesh = meshes.add(bevy::math::primitives::Cuboid::new(
                crate::environment::CUBE_SIZE,
                crate::environment::CUBE_SIZE,
                crate::environment::CUBE_SIZE,
            ));

            let texture_path = match block_type {
                crate::environment::BlockType::Grass => "textures/grass.webp",
                crate::environment::BlockType::Dirt => "textures/dirt.webp",
                crate::environment::BlockType::Stone => "textures/stone.webp",
                crate::environment::BlockType::QuartzBlock => "textures/quartz_block.webp",
                crate::environment::BlockType::GlassPane => "textures/glass_pane.webp",
                crate::environment::BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                crate::environment::BlockType::Water => "textures/water.webp",
            };

            let texture: Handle<Image> = asset_server.load(texture_path);
            let material = materials.add(StandardMaterial {
                base_color_texture: Some(texture),
                ..Default::default()
            });

            commands.spawn((
                Mesh3d(cube_mesh),
                MeshMaterial3d(material),
                Transform::from_translation(pos.as_vec3()),
                crate::environment::VoxelBlock { position: *pos },
            ));

            created_visuals += 1;
        }
    }

    if created_visuals > 0 {
        info!(
            "Synced {} visual entities for VoxelWorld blocks",
            created_visuals
        );
    }
}

/// Gets the worlds directory path
#[cfg(not(target_arch = "wasm32"))]
fn get_worlds_directory() -> std::path::PathBuf {
    let mut path = dirs::document_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    path.push("IOTCraft");
    path.push("worlds");
    path
}

/// For web, we'll use a virtual directory concept
#[cfg(target_arch = "wasm32")]
fn get_worlds_directory() -> std::path::PathBuf {
    // Return a dummy path for web - we'll handle storage differently
    std::path::PathBuf::from("web_worlds")
}

/// Parse block type from string for template execution
fn parse_block_type(block_type_str: &str) -> Option<crate::environment::BlockType> {
    match block_type_str {
        "grass" => Some(crate::environment::BlockType::Grass),
        "dirt" => Some(crate::environment::BlockType::Dirt),
        "stone" => Some(crate::environment::BlockType::Stone),
        "quartz_block" => Some(crate::environment::BlockType::QuartzBlock),
        "glass_pane" => Some(crate::environment::BlockType::GlassPane),
        "cyan_terracotta" => Some(crate::environment::BlockType::CyanTerracotta),
        "water" => Some(crate::environment::BlockType::Water),
        _ => None,
    }
}

// Simplified function removed - replaced with parameter bundled version in mcp_commands.rs

/// Execute MCP command directly with access to game resources (full version - currently unused)
fn execute_mcp_command_directly(
    tool_name: &str,
    arguments: &serde_json::Value,
    temperature: &TemperatureResource,
    _mqtt_config: &MqttConfig,
    voxel_world: &mut VoxelWorld,
    device_query: &Query<(&DeviceEntity, &Transform), Without<Camera>>,
    camera_query: &mut Query<
        (
            &mut Transform,
            &mut crate::camera_controllers::CameraController,
        ),
        With<Camera>,
    >,
    _create_world_events: &mut EventWriter<CreateWorldEvent>,
    load_world_events: &mut EventWriter<LoadWorldEvent>,
    publish_world_events: &mut EventWriter<crate::multiplayer::shared_world::PublishWorldEvent>,
    next_game_state: &mut Option<ResMut<NextState<GameState>>>,
    current_world: Option<&crate::world::world_types::CurrentWorld>,
    // New params for synchronous world creation in MCP
    commands: &mut Commands,
    existing_blocks_query: &Query<Entity, With<crate::environment::VoxelBlock>>,
    discovered_worlds: &mut ResMut<crate::world::world_types::DiscoveredWorlds>,
    mcp_state_transition: &mut ResMut<McpStateTransition>,
    // Multiplayer resources for MCP commands
    online_worlds: Option<&crate::multiplayer::shared_world::OnlineWorlds>,
    multiplayer_mode: Option<&crate::multiplayer::shared_world::MultiplayerMode>,
    refresh_online_worlds_events: &mut EventWriter<crate::multiplayer::shared_world::RefreshOnlineWorldsEvent>,
    join_shared_world_events: &mut EventWriter<crate::multiplayer::shared_world::JoinSharedWorldEvent>,
    leave_shared_world_events: &mut EventWriter<crate::multiplayer::shared_world::LeaveSharedWorldEvent>,
    unpublish_world_events: &mut EventWriter<crate::multiplayer::shared_world::UnpublishWorldEvent>,
) -> String {
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
            let world_name = current_world
                .map(|cw| cw.name.as_str())
                .unwrap_or("No World Loaded");

            json!({
                "blocks": block_count,
                "devices": device_count,
                "uptime_seconds": 3600, // Should be calculated properly
                "world_name": world_name
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

                // Create world synchronously in MCP context
                info!(
                    "MCP: Creating world directory and metadata for '{}'",
                    world_name
                );

                // Create world directory structure
                let worlds_dir = get_worlds_directory();
                let world_path = worlds_dir.join(&world_name);

                if let Err(e) = std::fs::create_dir_all(&world_path) {
                    return format!(
                        "Error: Failed to create world directory for {}: {}",
                        world_name, e
                    );
                }

                // Create metadata
                let metadata = crate::world::world_types::WorldMetadata {
                    name: world_name.to_string(),
                    description: description.to_string(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    last_played: chrono::Utc::now().to_rfc3339(),
                    version: "1.0.0".to_string(),
                };

                // Save metadata
                let metadata_path = world_path.join("metadata.json");
                match serde_json::to_string_pretty(&metadata) {
                    Ok(json) => {
                        if let Err(e) = std::fs::write(&metadata_path, json) {
                            return format!(
                                "Error: Failed to write metadata for {}: {}",
                                world_name, e
                            );
                        }
                    }
                    Err(e) => {
                        return format!(
                            "Error: Failed to serialize metadata for {}: {}",
                            world_name, e
                        );
                    }
                }

                // Clear existing blocks from scene and voxel world
                let mut cleared_entities = 0;
                for entity in existing_blocks_query.iter() {
                    commands.entity(entity).despawn();
                    cleared_entities += 1;
                }
                info!(
                    "MCP: Cleared {} existing block entities from scene",
                    cleared_entities
                );

                // Clear voxel world blocks
                voxel_world.blocks.clear();
                info!("MCP: Cleared voxel world blocks");

                // Set current world resource
                commands.insert_resource(crate::world::world_types::CurrentWorld {
                    name: world_name.to_string(),
                    path: world_path.clone(),
                    metadata: metadata.clone(),
                });

                // Add to discovered worlds
                discovered_worlds
                    .worlds
                    .push(crate::world::world_types::WorldInfo {
                        name: world_name.to_string(),
                        path: world_path,
                        metadata,
                    });

                // Set game state to InGame to transition UI from main menu
                if let Some(next_state) = next_game_state.as_mut() {
                    next_state.set(crate::ui::main_menu::GameState::InGame);
                    info!("MCP: Set game state to InGame for world creation transition");
                }

                // Execute template commands directly in MCP context
                if let Ok(template_content) = std::fs::read_to_string(&template_path) {
                    let mut blocks_created = 0;

                    for line in template_content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') {
                            continue;
                        }

                        // Parse and execute template commands directly
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.is_empty() {
                            continue;
                        }

                        match parts[0] {
                            "place" => {
                                if parts.len() == 5 {
                                    if let (Ok(x), Ok(y), Ok(z)) = (
                                        parts[2].parse::<i32>(),
                                        parts[3].parse::<i32>(),
                                        parts[4].parse::<i32>(),
                                    ) {
                                        if let Some(block_type) = parse_block_type(parts[1]) {
                                            let pos = bevy::math::IVec3::new(x, y, z);
                                            voxel_world.blocks.insert(pos, block_type);
                                            blocks_created += 1;
                                            info!(
                                                "MCP: Placed {} block at ({}, {}, {}) - visual will be created by sync system",
                                                parts[1], x, y, z
                                            );
                                        }
                                    }
                                }
                            }
                            "wall" => {
                                if parts.len() == 8 {
                                    if let (Ok(x1), Ok(y1), Ok(z1), Ok(x2), Ok(y2), Ok(z2)) = (
                                        parts[2].parse::<i32>(),
                                        parts[3].parse::<i32>(),
                                        parts[4].parse::<i32>(),
                                        parts[5].parse::<i32>(),
                                        parts[6].parse::<i32>(),
                                        parts[7].parse::<i32>(),
                                    ) {
                                        if let Some(block_type) = parse_block_type(parts[1]) {
                                            // Create wall by filling area between two points
                                            let min_x = x1.min(x2);
                                            let max_x = x1.max(x2);
                                            let min_y = y1.min(y2);
                                            let max_y = y1.max(y2);
                                            let min_z = z1.min(z2);
                                            let max_z = z1.max(z2);

                                            for x in min_x..=max_x {
                                                for y in min_y..=max_y {
                                                    for z in min_z..=max_z {
                                                        let pos = bevy::math::IVec3::new(x, y, z);
                                                        voxel_world.blocks.insert(pos, block_type);
                                                        blocks_created += 1;
                                                    }
                                                }
                                            }
                                            info!(
                                                "MCP: Created {} wall from ({},{},{}) to ({},{},{}) with blocks",
                                                parts[1], x1, y1, z1, x2, y2, z2
                                            );
                                        }
                                    }
                                }
                            }
                            "tp" => {
                                if parts.len() == 4 {
                                    if let (Ok(x), Ok(y), Ok(z)) = (
                                        parts[1].parse::<f32>(),
                                        parts[2].parse::<f32>(),
                                        parts[3].parse::<f32>(),
                                    ) {
                                        // Set camera position directly
                                        for (mut camera_transform, _) in camera_query.iter_mut() {
                                            camera_transform.translation =
                                                bevy::math::Vec3::new(x, y, z);
                                            info!(
                                                "MCP: Teleported camera to ({}, {}, {})",
                                                x, y, z
                                            );
                                            break; // Only teleport the first camera
                                        }
                                    }
                                }
                            }
                            "look" => {
                                if parts.len() == 3 {
                                    if let (Ok(yaw), Ok(pitch)) =
                                        (parts[1].parse::<f32>(), parts[2].parse::<f32>())
                                    {
                                        // Set camera rotation
                                        for (mut camera_transform, _) in camera_query.iter_mut() {
                                            let yaw_rad = yaw.to_radians();
                                            let pitch_rad = pitch.to_radians();
                                            camera_transform.rotation =
                                                bevy::math::Quat::from_euler(
                                                    bevy::math::EulerRot::YXZ,
                                                    yaw_rad,
                                                    pitch_rad,
                                                    0.0,
                                                );
                                            info!(
                                                "MCP: Set camera look to yaw={}, pitch={}",
                                                yaw, pitch
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                            "give" => {
                                // Note: Inventory operations would need access to inventory resource
                                // This is a simplified implementation - full version would need inventory access
                                info!("MCP: Give command executed: {}", line);
                            }
                            _ => {
                                info!("MCP: Unknown template command: {}", parts[0]);
                            }
                        }
                    }

                    info!(
                        "MCP: Template '{}' executed, created {} blocks",
                        template, blocks_created
                    );
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
                // Set camera position directly (simplified implementation without multiplayer events)
                for (mut camera_transform, _) in camera_query.iter_mut() {
                    camera_transform.translation =
                        bevy::math::Vec3::new(x as f32, y as f32, z as f32);
                    break; // Only move the first camera
                }
                format!("Player moved to ({}, {}, {})", x, y, z)
            } else {
                "Error: player_move requires x, y, z coordinates".to_string()
            }
        }
        "list_online_worlds" => {
            // Trigger refresh of online worlds first
            refresh_online_worlds_events.write(crate::multiplayer::shared_world::RefreshOnlineWorldsEvent);
            
            // Get current online worlds using the proper resource
            if let Some(worlds) = online_worlds {
                let mut world_list = Vec::new();
                for (world_id, world_info) in &worlds.worlds {
                    world_list.push(json!({
                        "world_id": world_id,
                        "world_name": world_info.world_name,
                        "description": world_info.description,
                        "host_name": world_info.host_name,
                        "host_player": world_info.host_player,
                        "player_count": world_info.player_count,
                        "max_players": world_info.max_players,
                        "is_public": world_info.is_public,
                        "created_at": world_info.created_at,
                        "last_updated": world_info.last_updated
                    }));
                }
                
                json!({
                    "online_worlds": world_list,
                    "total_count": world_list.len(),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
                .to_string()
            } else {
                json!({
                    "online_worlds": [],
                    "message": "Multiplayer resources not initialized",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
                .to_string()
            }
        }
        "get_multiplayer_status" => {
            // Use actual multiplayer mode if available
            if let Some(mode) = multiplayer_mode {
                match mode {
                    crate::multiplayer::shared_world::MultiplayerMode::SinglePlayer => {
                        json!({
                            "multiplayer_mode": "SinglePlayer",
                            "world_id": null,
                            "is_published": false,
                            "host_player": null,
                            "player_positions": [],
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        })
                        .to_string()
                    }
                    crate::multiplayer::shared_world::MultiplayerMode::HostingWorld { world_id, is_published } => {
                        json!({
                            "multiplayer_mode": "HostingWorld",
                            "world_id": world_id,
                            "is_published": is_published,
                            "host_player": null,
                            "player_positions": [],
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        })
                        .to_string()
                    }
                    crate::multiplayer::shared_world::MultiplayerMode::JoinedWorld { world_id, host_player } => {
                        json!({
                            "multiplayer_mode": "JoinedWorld",
                            "world_id": world_id,
                            "is_published": false,
                            "host_player": host_player,
                            "player_positions": [],
                            "timestamp": chrono::Utc::now().to_rfc3339()
                        })
                        .to_string()
                    }
                }
            } else {
                // Fallback when multiplayer resources not available
                json!({
                    "multiplayer_mode": "SinglePlayer",
                    "world_id": null,
                    "is_published": false,
                    "host_player": null,
                    "player_positions": [],
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })
                .to_string()
            }
        }
        "load_world_by_name" => {
            if let Some(world_name) = arguments.get("world_name").and_then(|v| v.as_str()) {
                info!("Loading world via MCP: name='{}'", world_name);

                // Emit LoadWorldEvent to trigger world loading
                load_world_events.write(LoadWorldEvent {
                    world_name: world_name.to_string(),
                });

                // Set game state to InGame to transition UI from main menu
                if let Some(next_state) = next_game_state.as_mut() {
                    next_state.set(crate::ui::main_menu::GameState::InGame);
                    info!("Set game state to InGame for world loading transition");
                }

                format!("Loaded world '{}' and transitioned to InGame", world_name)
            } else {
                "Error: world_name is required for load_world_by_name".to_string()
            }
        }
        "publish_world" => {
            if let Some(world_name) = arguments.get("world_name").and_then(|v| v.as_str()) {
                let max_players = arguments
                    .get("max_players")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as u32;
                let is_public = arguments
                    .get("is_public")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                info!("Publishing world via MCP: {}", world_name);

                // Emit PublishWorldEvent to trigger multiplayer mode transition (same as UI Share button)
                publish_world_events.write(crate::multiplayer::shared_world::PublishWorldEvent {
                    world_name: world_name.to_string(),
                    max_players,
                    is_public,
                });

                format!(
                    "World '{}' published for multiplayer (max_players: {}, public: {})",
                    world_name, max_players, is_public
                )
            } else {
                "Error: world_name is required for publish_world".to_string()
            }
        }
        "set_game_state" => {
            if let Some(state_str) = arguments.get("state").and_then(|v| v.as_str()) {
                info!("Setting game state via MCP to: {}", state_str);

                // Convert state string to GameState enum
                let new_state = match state_str.to_lowercase().as_str() {
                    "mainmenu" | "main_menu" => crate::ui::main_menu::GameState::MainMenu,
                    "ingame" | "in_game" => crate::ui::main_menu::GameState::InGame,
                    "settings" => crate::ui::main_menu::GameState::Settings,
                    "worldselection" | "world_selection" => {
                        crate::ui::main_menu::GameState::WorldSelection
                    }
                    "gameplaymenu" | "gameplay_menu" => {
                        crate::ui::main_menu::GameState::GameplayMenu
                    }
                    "consoleopen" | "console_open" => crate::ui::main_menu::GameState::ConsoleOpen,
                    _ => {
                        return format!(
                            "Error: Invalid game state '{}'. Valid states: MainMenu, InGame, Settings, WorldSelection, GameplayMenu, ConsoleOpen",
                            state_str
                        );
                    }
                };

                // Set the next game state with MCP flag
                if let Some(next_state) = next_game_state.as_mut() {
                    // Set flag to indicate this is an MCP-triggered transition
                    mcp_state_transition.is_mcp_transition = true;
                    next_state.set(new_state.clone());
                    format!(
                        "Game state will be set to {:?} on next frame (MCP transition)",
                        new_state
                    )
                } else {
                    "Error: Game state resource not available".to_string()
                }
            } else {
                "Error: state parameter is required for set_game_state".to_string()
            }
        }
        // World building commands
        "place_block" => {
            if let (Some(block_type), Some(x), Some(y), Some(z)) = (
                arguments.get("block_type").and_then(|v| v.as_str()),
                arguments.get("x").and_then(|v| v.as_i64()),
                arguments.get("y").and_then(|v| v.as_i64()),
                arguments.get("z").and_then(|v| v.as_i64()),
            ) {
                use crate::environment::BlockType;
                use bevy::math::IVec3;

                // Parse block type
                let block_type_enum = match block_type.to_lowercase().as_str() {
                    "grass" => BlockType::Grass,
                    "dirt" => BlockType::Dirt,
                    "stone" => BlockType::Stone,
                    "quartz_block" => BlockType::QuartzBlock,
                    "glass_pane" => BlockType::GlassPane,
                    "cyan_terracotta" => BlockType::CyanTerracotta,
                    _ => {
                        return format!(
                            "Error: Unknown block type '{}'. Valid types: grass, dirt, stone, quartz_block, glass_pane, cyan_terracotta",
                            block_type
                        );
                    }
                };

                // Place block in voxel world
                let position = IVec3::new(x as i32, y as i32, z as i32);
                voxel_world.set_block(position, block_type_enum);

                format!("Placed {} block at ({}, {}, {})", block_type, x, y, z)
            } else {
                "Error: place_block requires block_type, x, y, z parameters".to_string()
            }
        }
        "remove_block" => {
            if let (Some(x), Some(y), Some(z)) = (
                arguments.get("x").and_then(|v| v.as_i64()),
                arguments.get("y").and_then(|v| v.as_i64()),
                arguments.get("z").and_then(|v| v.as_i64()),
            ) {
                use bevy::math::IVec3;

                let position = IVec3::new(x as i32, y as i32, z as i32);
                if voxel_world.remove_block(&position).is_some() {
                    format!("Removed block at ({}, {}, {})", x, y, z)
                } else {
                    format!("No block found at ({}, {}, {}) to remove", x, y, z)
                }
            } else {
                "Error: remove_block requires x, y, z parameters".to_string()
            }
        }
        "create_wall" => {
            if let (Some(block_type), Some(x1), Some(y1), Some(z1), Some(x2), Some(y2), Some(z2)) = (
                arguments.get("block_type").and_then(|v| v.as_str()),
                arguments.get("x1").and_then(|v| v.as_i64()),
                arguments.get("y1").and_then(|v| v.as_i64()),
                arguments.get("z1").and_then(|v| v.as_i64()),
                arguments.get("x2").and_then(|v| v.as_i64()),
                arguments.get("y2").and_then(|v| v.as_i64()),
                arguments.get("z2").and_then(|v| v.as_i64()),
            ) {
                use crate::environment::BlockType;
                use bevy::math::IVec3;

                // Parse block type
                let block_type_enum = match block_type.to_lowercase().as_str() {
                    "grass" => BlockType::Grass,
                    "dirt" => BlockType::Dirt,
                    "stone" => BlockType::Stone,
                    "quartz_block" => BlockType::QuartzBlock,
                    "glass_pane" => BlockType::GlassPane,
                    "cyan_terracotta" => BlockType::CyanTerracotta,
                    _ => {
                        return format!(
                            "Error: Unknown block type '{}'. Valid types: grass, dirt, stone, quartz_block, glass_pane, cyan_terracotta",
                            block_type
                        );
                    }
                };

                // Calculate bounds (ensure x1 <= x2, y1 <= y2, z1 <= z2)
                let min_x = x1.min(x2) as i32;
                let max_x = x1.max(x2) as i32;
                let min_y = y1.min(y2) as i32;
                let max_y = y1.max(y2) as i32;
                let min_z = z1.min(z2) as i32;
                let max_z = z1.max(z2) as i32;

                let mut blocks_placed = 0;
                // Create wall by placing blocks in the rectangular area
                for x in min_x..=max_x {
                    for y in min_y..=max_y {
                        for z in min_z..=max_z {
                            let position = IVec3::new(x, y, z);
                            voxel_world.set_block(position, block_type_enum);
                            blocks_placed += 1;
                        }
                    }
                }

                format!(
                    "Created {} wall from ({},{},{}) to ({},{},{}) - {} blocks placed",
                    block_type, x1, y1, z1, x2, y2, z2, blocks_placed
                )
            } else {
                "Error: create_wall requires block_type, x1, y1, z1, x2, y2, z2 parameters"
                    .to_string()
            }
        }
        // Device management commands
        "spawn_device" => {
            if let (Some(device_id), Some(device_type), Some(x), Some(y), Some(z)) = (
                arguments.get("device_id").and_then(|v| v.as_str()),
                arguments.get("device_type").and_then(|v| v.as_str()),
                arguments.get("x").and_then(|v| v.as_f64()),
                arguments.get("y").and_then(|v| v.as_f64()),
                arguments.get("z").and_then(|v| v.as_f64()),
            ) {
                // TODO: Implement actual device spawning via device events
                format!(
                    "Spawned {} device '{}' at ({:.1}, {:.1}, {:.1})",
                    device_type, device_id, x, y, z
                )
            } else {
                "Error: spawn_device requires device_id, device_type, x, y, z parameters"
                    .to_string()
            }
        }
        "control_device" => {
            if let (Some(device_id), Some(command)) = (
                arguments.get("device_id").and_then(|v| v.as_str()),
                arguments.get("command").and_then(|v| v.as_str()),
            ) {
                // TODO: Implement actual device control via MQTT
                format!("Sent '{}' command to device '{}'", command, device_id)
            } else {
                "Error: control_device requires device_id and command parameters".to_string()
            }
        }
        "move_device" => {
            if let (Some(device_id), Some(x), Some(y), Some(z)) = (
                arguments.get("device_id").and_then(|v| v.as_str()),
                arguments.get("x").and_then(|v| v.as_f64()),
                arguments.get("y").and_then(|v| v.as_f64()),
                arguments.get("z").and_then(|v| v.as_f64()),
            ) {
                // TODO: Implement actual device movement via device events
                format!(
                    "Moved device '{}' to ({:.1}, {:.1}, {:.1})",
                    device_id, x, y, z
                )
            } else {
                "Error: move_device requires device_id, x, y, z parameters".to_string()
            }
        }
        "list_devices" => {
            let device_list: Vec<serde_json::Value> = device_query
                .iter()
                .map(|(device, transform)| {
                    json!({
                        "device_id": device.device_id,
                        "device_type": device.device_type,
                        "position": {
                            "x": transform.translation.x,
                            "y": transform.translation.y,
                            "z": transform.translation.z
                        },
                        "status": "online"
                    })
                })
                .collect();

            json!({
                "devices": device_list,
                "count": device_list.len(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            })
            .to_string()
        }
        // Camera control commands
        "teleport_camera" => {
            if let (Some(x), Some(y), Some(z)) = (
                arguments.get("x").and_then(|v| v.as_f64()),
                arguments.get("y").and_then(|v| v.as_f64()),
                arguments.get("z").and_then(|v| v.as_f64()),
            ) {
                // Move camera to new position
                let mut camera_moved = false;
                for (mut transform, _camera_controller) in camera_query.iter_mut() {
                    transform.translation = bevy::math::Vec3::new(x as f32, y as f32, z as f32);
                    camera_moved = true;
                    break; // Only move the first camera found
                }

                if camera_moved {
                    format!("Camera teleported to ({:.1}, {:.1}, {:.1})", x, y, z)
                } else {
                    "Error: No camera found to teleport".to_string()
                }
            } else {
                "Error: teleport_camera requires x, y, z parameters".to_string()
            }
        }
        "set_camera_angle" => {
            if let (Some(yaw), Some(pitch)) = (
                arguments.get("yaw").and_then(|v| v.as_f64()),
                arguments.get("pitch").and_then(|v| v.as_f64()),
            ) {
                // Set camera rotation via camera controller
                let mut camera_rotated = false;
                for (_transform, mut camera_controller) in camera_query.iter_mut() {
                    camera_controller.yaw = yaw as f32;
                    camera_controller.pitch = pitch as f32;
                    camera_rotated = true;
                    break; // Only modify the first camera found
                }

                if camera_rotated {
                    format!("Camera angle set to yaw: {:.1}°, pitch: {:.1}°", yaw, pitch)
                } else {
                    "Error: No camera found to set angle".to_string()
                }
            } else {
                "Error: set_camera_angle requires yaw and pitch parameters".to_string()
            }
        }
        // File operations
        "save_world" => {
            if let Some(filename) = arguments.get("filename").and_then(|v| v.as_str()) {
                // TODO: Implement actual world saving
                format!("World saved to file: {}", filename)
            } else {
                "Error: save_world requires filename parameter".to_string()
            }
        }
        "load_world" => {
            if let Some(world_name) = arguments.get("world_name").and_then(|v| v.as_str()) {
                info!("MCP load_world command: world_name={}", world_name);
                
                // Emit LoadWorldEvent for single-player world loading from filesystem
                load_world_events.write(LoadWorldEvent {
                    world_name: world_name.to_string(),
                });
                
                // Set game state to InGame to transition UI
                if let Some(next_state) = next_game_state {
                    next_state.set(GameState::InGame);
                    info!("MCP load_world: set game state to InGame");
                }
                
                format!("Loading world '{}' from filesystem and transitioning to InGame state", world_name)
            } else {
                "Error: load_world requires world_name parameter".to_string()
            }
        }
        "load_world_by_file" => {
            if let Some(filename) = arguments.get("filename").and_then(|v| v.as_str()) {
                // TODO: Implement actual world loading from file
                format!("World loaded from file: {}", filename)
            } else {
                "Error: load_world_by_file requires filename parameter".to_string()
            }
        }
        // Multiplayer world management
        "unpublish_world" => {
            info!("MCP unpublish_world command");
            
            // Get current world ID from multiplayer mode if available
            if let Some(mode) = multiplayer_mode {
                match mode {
                    crate::multiplayer::shared_world::MultiplayerMode::HostingWorld { world_id, .. } => {
                        // Emit UnpublishWorldEvent with the current world ID
                        unpublish_world_events.write(crate::multiplayer::shared_world::UnpublishWorldEvent {
                            world_id: world_id.clone(),
                        });
                        format!("World '{}' unpublished and returned to single-player mode", world_id)
                    }
                    crate::multiplayer::shared_world::MultiplayerMode::JoinedWorld { .. } => {
                        "Error: Cannot unpublish a joined world - use leave_world instead".to_string()
                    }
                    crate::multiplayer::shared_world::MultiplayerMode::SinglePlayer => {
                        "Error: No world is currently published".to_string()
                    }
                }
            } else {
                "Error: Multiplayer mode not available".to_string()
            }
        }
        "join_world" => {
            if let Some(world_id) = arguments.get("world_id").and_then(|v| v.as_str()) {
                info!("MCP join_world command: world_id={}", world_id);
                
                // Emit JoinSharedWorldEvent to trigger multiplayer world joining
                join_shared_world_events.write(crate::multiplayer::shared_world::JoinSharedWorldEvent {
                    world_id: world_id.to_string(),
                });
                
                // Set game state to InGame to transition UI  
                if let Some(next_state) = next_game_state {
                    // Set flag to indicate this is an MCP-triggered transition
                    mcp_state_transition.is_mcp_transition = true;
                    next_state.set(GameState::InGame);
                    info!("MCP join_world: set game state to InGame with MCP transition flag");
                }
                
                format!("Attempting to join multiplayer world '{}' and transitioning to InGame state", world_id)
            } else {
                "Error: join_world requires world_id parameter".to_string()
            }
        }
        "leave_world" => {
            info!("MCP leave_world command");
            
            // Emit LeaveSharedWorldEvent to trigger leaving multiplayer
            leave_shared_world_events.write(crate::multiplayer::shared_world::LeaveSharedWorldEvent);
            
            "Left shared world and returned to single-player mode".to_string()
        }
        // Utility commands
        "wait_for_condition" => {
            if let Some(condition) = arguments.get("condition").and_then(|v| v.as_str()) {
                let timeout = arguments
                    .get("timeout_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(30);
                let expected = arguments
                    .get("expected_value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // TODO: Implement actual condition waiting
                if expected.is_empty() {
                    format!(
                        "Waiting for condition '{}' (timeout: {}s)",
                        condition, timeout
                    )
                } else {
                    format!(
                        "Waiting for condition '{}' = '{}' (timeout: {}s)",
                        condition, expected, timeout
                    )
                }
            } else {
                "Error: wait_for_condition requires condition parameter".to_string()
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

            // Check if the result indicates an error and set is_error accordingly
            let is_error = event.result.starts_with("Error:");

            // Use proper McpToolResult struct for serialization
            let tool_result = McpToolResult {
                content: vec![McpContent::Text {
                    text: event.result.clone(),
                }],
                is_error: Some(is_error),
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

/// Resource to track whether game state transitions are triggered by MCP
/// This prevents automatic cursor grabbing when MCP controls the state
#[derive(Resource, Default)]
pub struct McpStateTransition {
    pub is_mcp_transition: bool,
}
