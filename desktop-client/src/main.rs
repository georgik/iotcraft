// Desktop-only main application - WASM uses lib.rs instead

// Input module declaration for gamepad support
#[cfg(not(target_arch = "wasm32"))]
mod input;

#[cfg(not(target_arch = "wasm32"))]
use bevy::ecs::system::SystemParam;
#[cfg(not(target_arch = "wasm32"))]
use bevy::math::IVec2;
#[cfg(not(target_arch = "wasm32"))]
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::{CursorGrabMode, WindowPosition};

// Console imports - only available with console feature
#[cfg(feature = "console")]
use crate::console::{BlinkCube, BlinkState, ConsoleManager, ConsolePlugin, ConsoleSet};

#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
#[cfg(not(target_arch = "wasm32"))]
use log::{info, warn};
#[cfg(not(target_arch = "wasm32"))]
// Note: rumqttc imports removed as we now use the unified core MQTT service
use serde_json::json;
#[cfg(not(target_arch = "wasm32"))]
// Duration import removed - not needed after consolidating MQTT clients
#[cfg(not(target_arch = "wasm32"))]
mod camera_controllers;
#[cfg(not(target_arch = "wasm32"))]
mod config;
#[cfg(not(target_arch = "wasm32"))]
mod console;
#[cfg(not(target_arch = "wasm32"))]
mod debug;
#[cfg(not(target_arch = "wasm32"))]
mod devices;
#[cfg(not(target_arch = "wasm32"))]
mod discovery;
#[cfg(not(target_arch = "wasm32"))]
mod environment;
#[cfg(not(target_arch = "wasm32"))]
mod fonts;
#[cfg(not(target_arch = "wasm32"))]
mod interaction;
#[cfg(not(target_arch = "wasm32"))]
mod inventory;
#[cfg(not(target_arch = "wasm32"))]
mod localization;
#[cfg(not(target_arch = "wasm32"))]
mod mcp;
#[cfg(not(target_arch = "wasm32"))]
mod minimap;
#[cfg(not(target_arch = "wasm32"))]
mod mqtt;
#[cfg(not(target_arch = "wasm32"))]
mod script;
#[cfg(not(target_arch = "wasm32"))]
mod ui;
#[cfg(not(target_arch = "wasm32"))]
use mcp::mcp_types::CommandExecutedEvent;

#[cfg(not(target_arch = "wasm32"))]
mod multiplayer;
#[cfg(not(target_arch = "wasm32"))]
mod player_avatar;
#[cfg(not(target_arch = "wasm32"))]
mod player_controller;
#[cfg(not(target_arch = "wasm32"))]
mod profile;
#[cfg(not(target_arch = "wasm32"))]
mod rendering;
#[cfg(not(target_arch = "wasm32"))]
mod shared_materials;
#[cfg(not(target_arch = "wasm32"))]
mod world;

// Re-export types for easier access
#[cfg(not(target_arch = "wasm32"))]
use camera_controllers::{CameraController, CameraControllerPlugin};
#[cfg(not(target_arch = "wasm32"))]
use config::MqttConfig;
#[cfg(not(target_arch = "wasm32"))]
use debug::{debug_commands::*, debug_params::DiagnosticsVisible};
#[cfg(not(target_arch = "wasm32"))]
use devices::*;
#[cfg(not(target_arch = "wasm32"))]
use environment::*;
#[cfg(not(target_arch = "wasm32"))]
use fonts::{FontPlugin, Fonts};
#[cfg(not(target_arch = "wasm32"))]
use interaction::InteractionPlugin as MyInteractionPlugin;
#[cfg(not(target_arch = "wasm32"))]
use inventory::{InventoryPlugin, PlayerInventory, inventory_commands::*};
#[cfg(not(target_arch = "wasm32"))]
use localization::{LocalizationConfig, LocalizationPlugin};
#[cfg(not(target_arch = "wasm32"))]
use minimap::MinimapPlugin;
#[cfg(not(target_arch = "wasm32"))]
use mqtt::{MqttPlugin, *};
#[cfg(not(target_arch = "wasm32"))]
use multiplayer::{
    MultiplayerPlugin, SharedWorldPlugin, WorldDiscoveryPlugin, WorldPublisherPlugin,
};
#[cfg(not(target_arch = "wasm32"))]
use player_avatar::PlayerAvatarPlugin;
#[cfg(not(target_arch = "wasm32"))]
use player_controller::PlayerControllerPlugin;
#[cfg(not(target_arch = "wasm32"))]
use shared_materials::SharedMaterialsPlugin;
#[cfg(not(target_arch = "wasm32"))]
use ui::{CrosshairPlugin, ErrorIndicatorPlugin, GameState, InventoryUiPlugin, MainMenuPlugin};
#[cfg(not(target_arch = "wasm32"))]
use world::{CreateWorldEvent, LoadWorldEvent, WorldPlugin};

// Helper function to extract client number from player ID for window positioning
#[cfg(not(target_arch = "wasm32"))]
fn extract_client_number(player_id: &str) -> Option<u32> {
    // Try to extract number from formats like "player-1", "player-2", "client-1", etc.
    if let Some(last_part) = player_id.split('-').last() {
        if let Ok(num) = last_part.parse::<u32>() {
            return Some(num);
        }
    }

    // Try to parse the entire ID as a number (e.g., "1", "2")
    if let Ok(num) = player_id.parse::<u32>() {
        return Some(num);
    }

    // Extract any trailing number from the string
    let trailing_digits: String = player_id
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    if !trailing_digits.is_empty() {
        if let Ok(num) = trailing_digits.parse::<u32>() {
            return Some(num);
        }
    }

    None
}

// CLI arguments
#[cfg(not(target_arch = "wasm32"))]
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Script file to execute on startup
    #[arg(short, long)]
    script: Option<String>,
    /// Force a specific language (BCP 47 format, e.g., en-US, cs-CZ, pt-BR)
    #[arg(short, long)]
    language: Option<String>,
    /// MQTT server address (default: localhost)
    #[arg(short, long)]
    mqtt_server: Option<String>,
    /// Player ID override for multiplayer testing (default: auto-generated)
    #[arg(short = 'p', long = "player-id")]
    player_id: Option<String>,
    /// Player name override (default: system username, shows in window title)
    #[arg(short = 'n', long = "player-name")]
    player_name: Option<String>,
    /// Run in MCP (Model Context Protocol) server mode
    #[arg(long)]
    mcp: bool,
}

// Helper function to write to console if available
#[cfg(feature = "console")]
fn write_to_console(message: String) {
    // Log the message instead since PrintConsoleLine was removed
    info!("Console: {}", message);
}

// Parameter bundle to handle system parameter limit
#[derive(SystemParam)]
struct ExecuteCommandsParams<'w, 's> {
    pending_commands: ResMut<'w, crate::script::script_types::PendingCommands>,
    command_executed_events: EventWriter<'w, CommandExecutedEvent>,
    temperature: Res<'w, TemperatureResource>,
    mqtt_config: Res<'w, MqttConfig>,
    voxel_world: ResMut<'w, VoxelWorld>,
    commands: Commands<'w, 's>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
    asset_server: Res<'w, AssetServer>,
    block_query: Query<'w, 's, (Entity, &'static VoxelBlock)>,
    device_query: Query<'w, 's, (&'static DeviceEntity, &'static Transform), Without<Camera>>,
    inventory: ResMut<'w, PlayerInventory>,
    camera_query:
        Query<'w, 's, (&'static mut Transform, &'static mut CameraController), With<Camera>>,
    create_world_events: EventWriter<'w, CreateWorldEvent>,
    load_world_events: EventWriter<'w, LoadWorldEvent>,
    next_game_state: Option<ResMut<'w, NextState<GameState>>>,
    mqtt_outgoing_tx: Option<Res<'w, crate::mqtt::core_service::MqttOutgoingTx>>,
}

// Split the large system into smaller, more manageable parts
#[cfg(not(target_arch = "wasm32"))]
fn execute_pending_commands(mut params: ExecuteCommandsParams) {
    // Add comprehensive debug logging
    use std::sync::atomic::{AtomicU64, Ordering};
    static DEBUG_COUNTER: AtomicU64 = AtomicU64::new(0);

    let counter = DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
    if counter % 3600 == 0 {
        // Log every minute at 60fps
        info!(
            "[DEBUG] execute_pending_commands system running, tick {}, queue size: {}",
            counter,
            params.pending_commands.commands.len()
        );
    }

    let command_count = params.pending_commands.commands.len();
    if command_count > 0 {
        info!(
            "[COMMAND] Processing {} commands from pending queue",
            command_count
        );
    }

    for command in params.pending_commands.commands.drain(..) {
        info!("[COMMAND] Executing: {}", command);

        // Check if command has a request ID (format: "command #request_id")
        let (actual_command, request_id) = if let Some(hash_pos) = command.rfind(" #") {
            let (cmd, id_part) = command.split_at(hash_pos);
            let request_id = id_part.trim_start_matches(" #").to_string();
            (cmd.to_string(), Some(request_id))
        } else {
            (command.clone(), None)
        };

        // Parse command string and dispatch to appropriate handler
        let parts: Vec<&str> = actual_command.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            // MCP system commands
            "get_client_info" => {
                let result_msg = json!({
                    "client_id": profile::load_or_create_profile_with_override(None).player_id,
                    "version": "1.0.0",
                    "status": "ready",
                    "capabilities": ["world_building", "device_management", "mqtt_integration"]
                })
                .to_string();

                info!("get_client_info command executed");

                // Emit command executed event if this was from MCP
                if let Some(req_id) = request_id.clone() {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_msg,
                    });
                }
            }
            "get_game_state" => {
                // Get current game state (this would need to be implemented properly with game state access)
                let result_msg = json!({
                    "game_state": "InGame", // This should get the actual game state
                    "world_loaded": true,
                    "multiplayer_active": false
                })
                .to_string();

                info!("get_game_state command executed");

                if let Some(req_id) = request_id.clone() {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_msg,
                    });
                }
            }
            "health_check" => {
                let result_msg = json!({
                    "status": "healthy",
                    "uptime_seconds": 3600, // This should be calculated properly
                    "memory_usage_mb": 256,  // This should be actual memory usage
                    "services_running": ["mqtt_client", "mcp_server"]
                })
                .to_string();

                info!("health_check command executed");

                if let Some(req_id) = request_id.clone() {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_msg,
                    });
                }
            }
            "get_system_info" => {
                let result_msg = json!({
                    "platform": std::env::consts::OS,
                    "architecture": std::env::consts::ARCH,
                    "rust_version": env!("CARGO_PKG_RUST_VERSION"),
                    "app_version": env!("CARGO_PKG_VERSION")
                })
                .to_string();

                info!("get_system_info command executed");

                if let Some(req_id) = request_id.clone() {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_msg,
                    });
                }
            }
            "get_world_status" => {
                let block_count = params.voxel_world.blocks.len();
                let device_count = params.device_query.iter().count();

                let result_msg = json!({
                    "blocks": block_count,
                    "devices": device_count,
                    "uptime_seconds": 3600, // Should be calculated properly
                    "world_name": "Default World"
                })
                .to_string();

                info!("get_world_status command executed");

                if let Some(req_id) = request_id.clone() {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_msg,
                    });
                }
            }
            "get_sensor_data" => {
                let result_msg = json!({
                    "temperature": params.temperature.value,
                    "devices_online": params.device_query.iter().count(),
                    "mqtt_connected": params.temperature.value.is_some()
                })
                .to_string();

                info!("get_sensor_data command executed");

                if let Some(req_id) = request_id.clone() {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_msg,
                    });
                }
            }
            "get_mqtt_status" => {
                // Check MQTT connection status comprehensively
                let mqtt_connected = params.temperature.value.is_some();
                let has_mqtt_outgoing = params.mqtt_outgoing_tx.is_some();

                let result_msg = json!({
                    "mqtt_connected": mqtt_connected,
                    "core_mqtt_service_available": has_mqtt_outgoing,
                    "temperature_data_available": params.temperature.value.is_some(),
                    "temperature_value": params.temperature.value,
                    "status": if mqtt_connected && has_mqtt_outgoing { "healthy" } else { "degraded" },
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "details": {
                        "mqtt_outgoing_channel": has_mqtt_outgoing,
                        "temperature_receiver_working": params.temperature.value.is_some()
                    }
                })
                .to_string();

                info!(
                    "get_mqtt_status command executed - MQTT connected: {}, Core service: {}",
                    mqtt_connected, has_mqtt_outgoing
                );

                if let Some(req_id) = request_id.clone() {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_msg,
                    });
                }
            }
            // load_world is now handled by the MCP command system in mcp_commands.rs
            "blink" => {
                if parts.len() == 2 {
                    let action = parts[1];
                    match action {
                        "start" => {
                            info!(
                                "Blink started via script (state management disabled in MCP mode)"
                            );
                        }
                        "stop" => {
                            info!(
                                "Blink stopped via script (state management disabled in MCP mode)"
                            );
                        }
                        _ => {
                            info!("Usage: blink [start|stop]");
                        }
                    }
                }
            }
            "mqtt" => {
                if parts.len() == 2 {
                    let action = parts[1];
                    match action {
                        "status" => {
                            let status = if params.temperature.value.is_some() {
                                "Connected to MQTT broker"
                            } else {
                                "Connecting to MQTT broker..."
                            };
                            #[cfg(feature = "console")]
                            write_to_console(status.to_string());
                            info!("MQTT status requested via script");
                        }
                        "temp" => {
                            let temp_msg = if let Some(val) = params.temperature.value {
                                format!("Current temperature: {:.1}°C", val)
                            } else {
                                "No temperature data available".to_string()
                            };
                            #[cfg(feature = "console")]
                            write_to_console(temp_msg);
                        }
                        _ => {
                            #[cfg(feature = "console")]
                            write_to_console("Usage: mqtt [status|temp]".to_string());
                        }
                    }
                }
            }
            "spawn" => {
                if parts.len() == 5 {
                    if let Ok(x) = parts[2].parse::<f32>() {
                        if let Ok(y) = parts[3].parse::<f32>() {
                            if let Ok(z) = parts[4].parse::<f32>() {
                                let device_id = parts[1].to_string();

                                // Create spawn command payload
                                let payload = json!({
                                    "device_id": device_id,
                                    "device_type": "lamp",
                                    "state": "online",
                                    "location": { "x": x, "y": y, "z": z }
                                })
                                .to_string();

                                // Use Core MQTT Service for device announcement
                                if let Some(outgoing_tx) = params.mqtt_outgoing_tx.as_ref() {
                                    if let Ok(tx) = outgoing_tx.0.lock() {
                                        let outgoing_msg = crate::mqtt::core_service::OutgoingMqttMessage::GenericPublish {
                                            topic: "devices/announce".to_string(),
                                            payload,
                                            qos: rumqttc::QoS::AtMostOnce,
                                            retain: false,
                                        };

                                        match tx.send(outgoing_msg) {
                                            Ok(_) => info!(
                                                "MQTT spawn announcement queued via Core MQTT Service"
                                            ),
                                            Err(e) => warn!(
                                                "Failed to queue MQTT spawn announcement: {} (continuing anyway)",
                                                e
                                            ),
                                        }
                                    }
                                } else {
                                    warn!(
                                        "Core MQTT Service not available for spawn announcement (continuing anyway)"
                                    );
                                }

                                let result_msg =
                                    format!("Spawn command sent for device {}", device_id);
                                #[cfg(feature = "console")]
                                write_to_console(result_msg.clone());

                                // Emit command executed event if this was from MCP
                                if let Some(req_id) = request_id.clone() {
                                    params.command_executed_events.write(CommandExecutedEvent {
                                        request_id: req_id,
                                        result: result_msg,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            "spawn_door" => {
                if parts.len() == 5 {
                    if let Ok(x) = parts[2].parse::<f32>() {
                        if let Ok(y) = parts[3].parse::<f32>() {
                            if let Ok(z) = parts[4].parse::<f32>() {
                                let device_id = parts[1].to_string();

                                // Create spawn door command payload
                                let payload = json!({
                                    "device_id": device_id,
                                    "device_type": "door",
                                    "state": "online",
                                    "location": { "x": x, "y": y, "z": z }
                                })
                                .to_string();

                                // Use Core MQTT Service for device announcement
                                if let Some(outgoing_tx) = params.mqtt_outgoing_tx.as_ref() {
                                    if let Ok(tx) = outgoing_tx.0.lock() {
                                        let outgoing_msg = crate::mqtt::core_service::OutgoingMqttMessage::GenericPublish {
                                            topic: "devices/announce".to_string(),
                                            payload,
                                            qos: rumqttc::QoS::AtMostOnce,
                                            retain: false,
                                        };

                                        match tx.send(outgoing_msg) {
                                            Ok(_) => info!(
                                                "MQTT spawn_door announcement queued via Core MQTT Service"
                                            ),
                                            Err(e) => warn!(
                                                "Failed to queue MQTT spawn_door announcement: {} (continuing anyway)",
                                                e
                                            ),
                                        }
                                    }
                                } else {
                                    warn!(
                                        "Core MQTT Service not available for spawn_door announcement (continuing anyway)"
                                    );
                                }

                                let result_msg =
                                    format!("Spawn door command sent for device {}", device_id);
                                #[cfg(feature = "console")]
                                write_to_console(result_msg.clone());

                                // Emit command executed event if this was from MCP
                                if let Some(req_id) = request_id.clone() {
                                    params.command_executed_events.write(CommandExecutedEvent {
                                        request_id: req_id,
                                        result: result_msg,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            "place" => {
                if parts.len() == 5 {
                    if let Ok(x) = parts[2].parse::<i32>() {
                        if let Ok(y) = parts[3].parse::<i32>() {
                            if let Ok(z) = parts[4].parse::<i32>() {
                                let block_type_str = parts[1];
                                let block_type = match block_type_str {
                                    "grass" => BlockType::Grass,
                                    "dirt" => BlockType::Dirt,
                                    "stone" => BlockType::Stone,
                                    "quartz_block" => BlockType::QuartzBlock,
                                    "glass_pane" => BlockType::GlassPane,
                                    "cyan_terracotta" => BlockType::CyanTerracotta,
                                    "water" => BlockType::Water,
                                    _ => {
                                        let error_msg =
                                            format!("Invalid block type: {}", block_type_str);
                                        #[cfg(feature = "console")]
                                        write_to_console(error_msg.clone());

                                        // Emit error event if this was from MCP
                                        if let Some(req_id) = request_id.clone() {
                                            params.command_executed_events.write(
                                                CommandExecutedEvent {
                                                    request_id: req_id,
                                                    result: error_msg,
                                                },
                                            );
                                        }
                                        continue;
                                    }
                                };

                                params
                                    .voxel_world
                                    .set_block(IVec3::new(x, y, z), block_type);

                                // Spawn the block
                                let cube_mesh = params
                                    .meshes
                                    .add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
                                let material = match block_type {
                                    BlockType::Water => params.materials.add(StandardMaterial {
                                        base_color: Color::srgba(0.0, 0.35, 0.9, 0.6),
                                        alpha_mode: AlphaMode::Blend,
                                        ..default()
                                    }),
                                    _ => {
                                        let texture_path = match block_type {
                                            BlockType::Grass => "textures/grass.webp",
                                            BlockType::Dirt => "textures/dirt.webp",
                                            BlockType::Stone => "textures/stone.webp",
                                            BlockType::QuartzBlock => "textures/quartz_block.webp",
                                            BlockType::GlassPane => "textures/glass_pane.webp",
                                            BlockType::CyanTerracotta => {
                                                "textures/cyan_terracotta.webp"
                                            }
                                            _ => unreachable!(),
                                        };
                                        let texture: Handle<Image> =
                                            params.asset_server.load(texture_path);
                                        params.materials.add(StandardMaterial {
                                            base_color_texture: Some(texture),
                                            ..default()
                                        })
                                    }
                                };

                                params.commands.spawn((
                                    Mesh3d(cube_mesh),
                                    MeshMaterial3d(material),
                                    Transform::from_translation(Vec3::new(
                                        x as f32, y as f32, z as f32,
                                    )),
                                    VoxelBlock {
                                        position: IVec3::new(x, y, z),
                                    },
                                    // Physics colliders are managed by PhysicsManagerPlugin based on distance and mode
                                ));

                                let result_msg = format!(
                                    "Placed {} block at ({}, {}, {})",
                                    block_type_str, x, y, z
                                );
                                #[cfg(feature = "console")]
                                write_to_console(result_msg.clone());

                                // Emit command executed event if this was from MCP
                                if let Some(req_id) = request_id.clone() {
                                    params.command_executed_events.write(CommandExecutedEvent {
                                        request_id: req_id,
                                        result: result_msg,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            "remove" => {
                if parts.len() == 4 {
                    if let Ok(x) = parts[1].parse::<i32>() {
                        if let Ok(y) = parts[2].parse::<i32>() {
                            if let Ok(z) = parts[3].parse::<i32>() {
                                let position = IVec3::new(x, y, z);
                                let result_msg =
                                    if params.voxel_world.remove_block(&position).is_some() {
                                        // Remove the block entity
                                        for (entity, block) in params.block_query.iter() {
                                            if block.position == position {
                                                params.commands.entity(entity).despawn();
                                            }
                                        }
                                        format!("Removed block at ({}, {}, {})", x, y, z)
                                    } else {
                                        format!("No block found at ({}, {}, {})", x, y, z)
                                    };

                                #[cfg(feature = "console")]
                                write_to_console(result_msg.clone());

                                // Emit command executed event if this was from MCP
                                if let Some(req_id) = request_id.clone() {
                                    params.command_executed_events.write(CommandExecutedEvent {
                                        request_id: req_id,
                                        result: result_msg,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            "give" => {
                if parts.len() == 3 {
                    if let Ok(quantity) = parts[2].parse::<usize>() {
                        let item_type_str = parts[1];
                        let item_type = match item_type_str {
                            "grass" => crate::inventory::ItemType::Block(BlockType::Grass),
                            "dirt" => crate::inventory::ItemType::Block(BlockType::Dirt),
                            "stone" => crate::inventory::ItemType::Block(BlockType::Stone),
                            "quartz_block" => {
                                crate::inventory::ItemType::Block(BlockType::QuartzBlock)
                            }
                            "glass_pane" => crate::inventory::ItemType::Block(BlockType::GlassPane),
                            "cyan_terracotta" => {
                                crate::inventory::ItemType::Block(BlockType::CyanTerracotta)
                            }
                            "water" => crate::inventory::ItemType::Block(BlockType::Water),
                            _ => {
                                #[cfg(feature = "console")]
                                write_to_console(format!("Invalid item type: {}", item_type_str));
                                continue;
                            }
                        };

                        params.inventory.add_items(item_type, quantity as u32);
                        #[cfg(feature = "console")]
                        write_to_console(format!("Added {} x {}", quantity, item_type_str));
                    }
                }
            }
            "wall" => {
                if parts.len() == 8 {
                    if let Ok(x1) = parts[2].parse::<i32>() {
                        if let Ok(y1) = parts[3].parse::<i32>() {
                            if let Ok(z1) = parts[4].parse::<i32>() {
                                if let Ok(x2) = parts[5].parse::<i32>() {
                                    if let Ok(y2) = parts[6].parse::<i32>() {
                                        if let Ok(z2) = parts[7].parse::<i32>() {
                                            let block_type_str = parts[1];
                                            let block_type_enum = match block_type_str {
                                                "grass" => BlockType::Grass,
                                                "dirt" => BlockType::Dirt,
                                                "stone" => BlockType::Stone,
                                                "quartz_block" => BlockType::QuartzBlock,
                                                "glass_pane" => BlockType::GlassPane,
                                                "cyan_terracotta" => BlockType::CyanTerracotta,
                                                "water" => BlockType::Water,
                                                _ => {
                                                    #[cfg(feature = "console")]
                                                    write_to_console(format!(
                                                        "Invalid block type: {}",
                                                        block_type_str
                                                    ));
                                                    continue;
                                                }
                                            };

                                            // Debug: VoxelWorld before adding blocks
                                            info!(
                                                "[DEBUG] VoxelWorld before wall command: {} blocks",
                                                params.voxel_world.blocks.len()
                                            );
                                            info!(
                                                "[DEBUG] Wall command: {} from ({}, {}, {}) to ({}, {}, {})",
                                                block_type_str, x1, y1, z1, x2, y2, z2
                                            );

                                            let material = match block_type_enum {
                                                BlockType::Water => {
                                                    params.materials.add(StandardMaterial {
                                                        base_color: Color::srgba(
                                                            0.0, 0.35, 0.9, 0.6,
                                                        ),
                                                        alpha_mode: AlphaMode::Blend,
                                                        ..default()
                                                    })
                                                }
                                                _ => {
                                                    let texture_path = match block_type_enum {
                                                        BlockType::Grass => "textures/grass.webp",
                                                        BlockType::Dirt => "textures/dirt.webp",
                                                        BlockType::Stone => "textures/stone.webp",
                                                        BlockType::QuartzBlock => {
                                                            "textures/quartz_block.webp"
                                                        }
                                                        BlockType::GlassPane => {
                                                            "textures/glass_pane.webp"
                                                        }
                                                        BlockType::CyanTerracotta => {
                                                            "textures/cyan_terracotta.webp"
                                                        }
                                                        _ => unreachable!(),
                                                    };
                                                    let texture: Handle<Image> =
                                                        params.asset_server.load(texture_path);
                                                    params.materials.add(StandardMaterial {
                                                        base_color_texture: Some(texture),
                                                        ..default()
                                                    })
                                                }
                                            };

                                            let cube_mesh = params
                                                .meshes
                                                .add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));

                                            let mut blocks_added = 0;
                                            for x in x1..=x2 {
                                                for y in y1..=y2 {
                                                    for z in z1..=z2 {
                                                        params.voxel_world.set_block(
                                                            IVec3::new(x, y, z),
                                                            block_type_enum,
                                                        );
                                                        blocks_added += 1;

                                                        params.commands.spawn((
                                                            Mesh3d(cube_mesh.clone()),
                                                            MeshMaterial3d(material.clone()),
                                                            Transform::from_translation(Vec3::new(
                                                                x as f32, y as f32, z as f32,
                                                            )),
                                                            VoxelBlock {
                                                                position: IVec3::new(x, y, z),
                                                            },
                                                            // Physics colliders are managed by PhysicsManagerPlugin based on distance and mode
                                                        ));
                                                    }
                                                }
                                            }

                                            // Debug: VoxelWorld after adding blocks
                                            info!(
                                                "VoxelWorld after wall command: {} blocks (added {})",
                                                params.voxel_world.blocks.len(),
                                                blocks_added
                                            );

                                            // Debug: Show a few sample blocks that were just added
                                            let sample_positions = [
                                                IVec3::new(x1, y1, z1),
                                                IVec3::new(x2, y2, z2),
                                                IVec3::new(
                                                    (x1 + x2) / 2,
                                                    (y1 + y2) / 2,
                                                    (z1 + z2) / 2,
                                                ),
                                            ];

                                            for pos in sample_positions {
                                                let has_block = params.voxel_world.is_block_at(pos);
                                                info!(
                                                    "Sample block check at {:?}: has_block={}",
                                                    pos, has_block
                                                );
                                            }

                                            let result_msg = format!(
                                                "Created a wall of {} from ({}, {}, {}) to ({}, {}, {}) - {} blocks",
                                                block_type_str,
                                                x1,
                                                y1,
                                                z1,
                                                x2,
                                                y2,
                                                z2,
                                                blocks_added
                                            );
                                            #[cfg(feature = "console")]
                                            write_to_console(result_msg.clone());

                                            // Emit command executed event if this was from MCP
                                            if let Some(req_id) = request_id.clone() {
                                                params.command_executed_events.write(
                                                    CommandExecutedEvent {
                                                        request_id: req_id,
                                                        result: result_msg,
                                                    },
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "tp" => {
                if parts.len() == 4 {
                    if let Ok(x) = parts[1].parse::<f32>() {
                        if let Ok(y) = parts[2].parse::<f32>() {
                            if let Ok(z) = parts[3].parse::<f32>() {
                                if let Ok((mut transform, _camera_controller)) =
                                    params.camera_query.single_mut()
                                {
                                    // Set the camera position
                                    transform.translation = Vec3::new(x, y, z);

                                    #[cfg(feature = "console")]
                                    write_to_console(format!(
                                        "Teleported to ({:.1}, {:.1}, {:.1})",
                                        x, y, z
                                    ));
                                    info!("Camera teleported to ({:.1}, {:.1}, {:.1})", x, y, z);
                                } else {
                                    #[cfg(feature = "console")]
                                    write_to_console(
                                        "Error: Could not find camera to teleport".to_string(),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            "look" => {
                if parts.len() == 3 {
                    if let Ok(yaw) = parts[1].parse::<f32>() {
                        if let Ok(pitch) = parts[2].parse::<f32>() {
                            if let Ok((mut transform, mut camera_controller)) =
                                params.camera_query.single_mut()
                            {
                                // Convert degrees to radians for internal use
                                let yaw_rad = yaw.to_radians();
                                let pitch_rad = pitch.to_radians();

                                // Update the camera controller's internal yaw and pitch
                                camera_controller.yaw = yaw_rad;
                                camera_controller.pitch = pitch_rad
                                    .clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);

                                // Apply the rotation to the transform using the same logic as the camera controller
                                transform.rotation = Quat::from_euler(
                                    bevy::math::EulerRot::ZYX,
                                    0.0,
                                    camera_controller.yaw,
                                    camera_controller.pitch,
                                );

                                #[cfg(feature = "console")]
                                write_to_console(format!(
                                    "Set look angles to yaw: {:.1}°, pitch: {:.1}°",
                                    yaw, pitch
                                ));
                                info!(
                                    "Camera look angles set to yaw: {:.1}°, pitch: {:.1}°",
                                    yaw, pitch
                                );
                            } else {
                                #[cfg(feature = "console")]
                                write_to_console(
                                    "Error: Could not find camera to set look direction"
                                        .to_string(),
                                );
                            }
                        }
                    }
                }
            }
            "list" => {
                // Handle list devices command
                let device_list: Vec<String> = params
                    .device_query
                    .iter()
                    .map(|(device, transform)| {
                        format!(
                            "{}: {} at ({:.1}, {:.1}, {:.1})",
                            device.device_id,
                            device.device_type,
                            transform.translation.x,
                            transform.translation.y,
                            transform.translation.z
                        )
                    })
                    .collect();

                let result_text = if device_list.is_empty() {
                    "No devices found".to_string()
                } else {
                    format!("Devices:\n{}", device_list.join("\n"))
                };

                #[cfg(feature = "console")]
                write_to_console(result_text.clone());
                info!("Executed list command, found {} devices", device_list.len());

                // Emit command executed event if this was from MCP
                if let Some(req_id) = request_id {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_text,
                    });
                }
            }
            "publish_world" => {
                info!(
                    "Executing publish_world command with args: {:?}",
                    &parts[1..]
                );
                let result_msg = "Publishing world (multiplayer command executed)".to_string();
                #[cfg(feature = "console")]
                write_to_console(result_msg.clone());

                // Emit command executed event if this was from MCP
                if let Some(req_id) = request_id {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_msg,
                    });
                }
            }
            "join_world" => {
                info!("Executing join_world command with args: {:?}", &parts[1..]);
                let result_msg = "Joining world (multiplayer command executed)".to_string();
                #[cfg(feature = "console")]
                write_to_console(result_msg.clone());

                // Emit command executed event if this was from MCP
                if let Some(req_id) = request_id {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_msg,
                    });
                }
            }
            "create_world" => {
                if parts.len() >= 3 {
                    let world_name = parts[1].to_string();
                    let description = parts[2..].join(" "); // Join remaining parts as description

                    info!(
                        "Executing create_world command: name='{}', description='{}'",
                        world_name, description
                    );

                    // Send CreateWorldEvent to trigger world creation
                    params.create_world_events.write(CreateWorldEvent {
                        world_name: world_name.clone(),
                        description: description.clone(),
                        template: None, // Use default template for command execution
                    });

                    // Set game state to InGame to transition UI from main menu
                    if let Some(next_state) = params.next_game_state.as_mut() {
                        next_state.set(crate::ui::main_menu::GameState::InGame);
                        info!("Set game state to InGame for world creation transition");
                    } else {
                        warn!("NextState<GameState> resource not available for state transition");
                    }

                    let result_msg = format!(
                        "Created new world: {} ({}) and transitioned to InGame",
                        world_name, description
                    );
                    #[cfg(feature = "console")]
                    write_to_console(result_msg.clone());

                    // Emit command executed event if this was from MCP
                    if let Some(req_id) = request_id {
                        params.command_executed_events.write(CommandExecutedEvent {
                            request_id: req_id,
                            result: result_msg,
                        });
                    }
                } else {
                    let error_msg = "Usage: create_world <world_name> <description>".to_string();
                    #[cfg(feature = "console")]
                    write_to_console(error_msg.clone());

                    if let Some(req_id) = request_id {
                        params.command_executed_events.write(CommandExecutedEvent {
                            request_id: req_id,
                            result: error_msg,
                        });
                    }
                }
            }
            // load_world is now handled by the MCP command system in mcp_commands.rs (second duplicate removed)
            "set_game_state" => {
                if parts.len() >= 2 {
                    let state_str = parts[1];

                    // Parse state string to GameState enum
                    use crate::ui::main_menu::GameState;
                    let new_state = match state_str {
                        "MainMenu" => GameState::MainMenu,
                        "WorldSelection" => GameState::WorldSelection,
                        "InGame" => GameState::InGame,
                        "Settings" => GameState::Settings,
                        "GameplayMenu" => GameState::GameplayMenu,
                        _ => {
                            let error_msg = format!(
                                "Invalid game state: {}. Valid states: MainMenu, WorldSelection, InGame, Settings, GameplayMenu",
                                state_str
                            );
                            #[cfg(feature = "console")]
                            write_to_console(error_msg.clone());

                            if let Some(req_id) = request_id {
                                params.command_executed_events.write(CommandExecutedEvent {
                                    request_id: req_id,
                                    result: error_msg,
                                });
                            }
                            return;
                        }
                    };

                    // Set the game state
                    if let Some(next_state) = params.next_game_state.as_mut() {
                        next_state.set(new_state);
                        info!("Set game state to {}", state_str);

                        let result_msg = format!("Game state set to {}", state_str);
                        #[cfg(feature = "console")]
                        write_to_console(result_msg.clone());

                        // Emit command executed event if this was from MCP
                        if let Some(req_id) = request_id {
                            params.command_executed_events.write(CommandExecutedEvent {
                                request_id: req_id,
                                result: result_msg,
                            });
                        }
                    } else {
                        let error_msg =
                            "NextState<GameState> resource not available for state transition"
                                .to_string();
                        warn!("{}", error_msg);

                        #[cfg(feature = "console")]
                        write_to_console(error_msg.clone());

                        if let Some(req_id) = request_id {
                            params.command_executed_events.write(CommandExecutedEvent {
                                request_id: req_id,
                                result: error_msg,
                            });
                        }
                    }
                } else {
                    let error_msg = "Usage: set_game_state <state>".to_string();
                    #[cfg(feature = "console")]
                    write_to_console(error_msg.clone());

                    if let Some(req_id) = request_id {
                        params.command_executed_events.write(CommandExecutedEvent {
                            request_id: req_id,
                            result: error_msg,
                        });
                    }
                }
            }
            _ => {
                let error_msg = format!("Unknown command: {}", command);
                #[cfg(feature = "console")]
                write_to_console(error_msg.clone());
                info!("Console: {}", error_msg);

                // Emit command executed event if this was from MCP
                if let Some(req_id) = request_id {
                    params.command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: error_msg,
                    });
                }
            }
        }
    }
}

// System to handle console commands that were queued by execute_pending_commands
#[cfg(all(not(target_arch = "wasm32"), feature = "console"))]
fn execute_console_commands(
    mut pending_commands: ResMut<crate::script::script_types::PendingCommands>,
    mut command_executed_events: EventWriter<CommandExecutedEvent>,
    mut console_manager: ResMut<ConsoleManager>,
) {
    use crate::console::command_parser::CommandParser;

    // Process commands that start with "CONSOLE_COMMAND:"
    let console_commands: Vec<String> = pending_commands
        .commands
        .drain(..)
        .filter(|cmd| cmd.starts_with("CONSOLE_COMMAND:"))
        .collect();

    // Re-add non-console commands back to the queue
    let other_commands: Vec<String> = pending_commands
        .commands
        .drain(..)
        .filter(|cmd| !cmd.starts_with("CONSOLE_COMMAND:"))
        .collect();
    pending_commands.commands.extend(other_commands);

    for command_with_prefix in console_commands {
        let command_without_prefix = command_with_prefix.trim_start_matches("CONSOLE_COMMAND:");
        info!("Executing console command: {}", command_without_prefix);

        // Check if command has a request ID (format: "command #request_id")
        let (actual_command, request_id) =
            if let Some(hash_pos) = command_without_prefix.rfind(" #") {
                let (cmd, id_part) = command_without_prefix.split_at(hash_pos);
                let request_id = id_part.trim_start_matches(" #").to_string();
                (cmd.to_string(), Some(request_id))
            } else {
                (command_without_prefix.to_string(), None)
            };

        // Use the console's command parser - note: parser is created but not currently used
        // for actual command execution due to world access limitations in this context
        let _parser = CommandParser::new();
        // We need to access the world, so we'll do this through the console manager
        // For now, just try to add the command as output and execute it via console
        console_manager.add_message(&format!("Executing: {}", actual_command));

        // For MCP compatibility, we need to signal the result
        if let Some(req_id) = request_id {
            // Send a basic success response
            command_executed_events.write(CommandExecutedEvent {
                request_id: req_id,
                result: format!("Attempted to execute console command: {}", actual_command),
            });
        }
    }
}

// Stub version for non-console builds or WASM
#[cfg(any(target_arch = "wasm32", not(feature = "console")))]
fn execute_console_commands(
    mut pending_commands: ResMut<crate::script::script_types::PendingCommands>,
    mut command_executed_events: EventWriter<CommandExecutedEvent>,
) {
    // Process commands that start with "CONSOLE_COMMAND:" and remove them
    let mut remaining_commands = Vec::new();

    for command in pending_commands.commands.drain(..) {
        if command.starts_with("CONSOLE_COMMAND:") {
            let command_without_prefix = command.trim_start_matches("CONSOLE_COMMAND:");
            info!(
                "Console not available, skipping command: {}",
                command_without_prefix
            );

            // Check if command has a request ID and respond
            if let Some(hash_pos) = command_without_prefix.rfind(" #") {
                let (_cmd, id_part) = command_without_prefix.split_at(hash_pos);
                let request_id = id_part.trim_start_matches(" #").to_string();
                command_executed_events.write(CommandExecutedEvent {
                    request_id,
                    result: "Console not available".to_string(),
                });
            }
        } else {
            remaining_commands.push(command);
        }
    }

    pending_commands.commands = remaining_commands;
}

// WASM stub - WASM targets use lib.rs for the main entry point
#[cfg(target_arch = "wasm32")]
fn main() {
    // No-op: WASM builds use the lib.rs entry point instead
    panic!("main() should not be called on WASM target - use lib.rs instead");
}

/// Tests for core game commands to prevent regressions
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_player_id_short_form() {
        // Test the short form -p argument
        let args = Args::try_parse_from(&["iotcraft", "-p", "test-123"]).unwrap();
        assert_eq!(args.player_id, Some("test-123".to_string()));
    }

    #[test]
    fn test_cli_player_id_long_form() {
        // Test the long form --player-id argument
        let args = Args::try_parse_from(&["iotcraft", "--player-id", "test-456"]).unwrap();
        assert_eq!(args.player_id, Some("test-456".to_string()));
    }

    #[test]
    fn test_cli_player_id_numeric() {
        // Test numeric player IDs like the one used in the logs
        let args = Args::try_parse_from(&["iotcraft", "-p", "2"]).unwrap();
        assert_eq!(args.player_id, Some("2".to_string()));
    }

    #[test]
    fn test_cli_no_player_id() {
        // Test when no player ID is provided
        let args = Args::try_parse_from(&["iotcraft"]).unwrap();
        assert_eq!(args.player_id, None);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod wall_command_tests {
    use super::*;

    /// Test the core wall command coordinate behavior to ensure we don't regress
    /// on the coordinate ordering issue found in new_world.txt script.
    #[test]
    fn test_wall_command_coordinate_ordering_regression() {
        // This test specifically addresses the issue where wall coordinates
        // in new_world.txt were incorrectly ordered, causing 0 blocks to be created.
        // Example: wall stone 21 1 -21 26 1 -26 created 0 blocks because -21 > -26

        let mut voxel_world = VoxelWorld::default();
        let initial_count = voxel_world.blocks.len();

        // Test case 1: Backwards Z coordinates (the problematic case from new_world.txt)
        // This should create 0 blocks due to invalid range (start > end)
        let (x1, y1, z1) = (21, 1, -21);
        let (x2, y2, z2) = (26, 1, -26);

        let mut blocks_added = 0;
        for x in x1..=x2 {
            for y in y1..=y2 {
                for z in z1..=z2 {
                    // This range is invalid: -21..=-26 is empty
                    voxel_world.set_block(IVec3::new(x, y, z), BlockType::Stone);
                    blocks_added += 1;
                }
            }
        }

        assert_eq!(
            blocks_added, 0,
            "Backwards Z coordinates (-21 to -26) should create 0 blocks"
        );
        assert_eq!(
            voxel_world.blocks.len(),
            initial_count,
            "VoxelWorld should have no new blocks with invalid coordinate range"
        );

        // Test case 2: Correctly ordered coordinates (the fix)
        let (x1, y1, z1) = (21, 1, -26); // Start with smaller Z value
        let (x2, y2, z2) = (26, 1, -21); // End with larger Z value

        let mut blocks_added_correct = 0;
        for x in x1..=x2 {
            for y in y1..=y2 {
                for z in z1..=z2 {
                    // Now valid range: -26..=-21
                    voxel_world.set_block(IVec3::new(x, y, z), BlockType::Stone);
                    blocks_added_correct += 1;
                }
            }
        }

        // Should be (26-21+1) * (1-1+1) * (-21-(-26)+1) = 6*1*6 = 36 blocks
        assert_eq!(
            blocks_added_correct, 36,
            "Correctly ordered coordinates should create 36 blocks"
        );
        assert_eq!(
            voxel_world.blocks.len(),
            initial_count + 36,
            "VoxelWorld should have 36 new blocks with valid coordinate range"
        );
    }

    #[test]
    fn test_new_world_script_pond_coordinates() {
        // Test the specific coordinates used in the fixed new_world.txt script
        // to ensure they work as expected

        let mut voxel_world = VoxelWorld::default();

        // Test the pond walls from the fixed script:
        // North wall: wall stone 21 1 -26 26 1 -26
        let mut north_wall_blocks = 0;
        for x in 21..=26 {
            for y in 1..=1 {
                for z in -26..=-26 {
                    voxel_world.set_block(IVec3::new(x, y, z), BlockType::Stone);
                    north_wall_blocks += 1;
                }
            }
        }
        assert_eq!(north_wall_blocks, 6, "North wall should have 6 blocks");

        // Water inside: wall water 22 1 -25 25 1 -22
        let mut water_blocks = 0;
        for x in 22..=25 {
            for y in 1..=1 {
                for z in -25..=-22 {
                    voxel_world.set_block(IVec3::new(x, y, z), BlockType::Water);
                    water_blocks += 1;
                }
            }
        }
        assert_eq!(water_blocks, 16, "Water area should have 4x4 = 16 blocks");
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Logging is now handled by Bevy's LogPlugin in DefaultPlugins

    // Note: Script execution is now handled by the ScriptPlugin

    // Load MQTT configuration with mDNS discovery fallback
    let mqtt_config = MqttConfig::from_env_with_discovery(args.mqtt_server).await;
    info!("Using MQTT broker: {}", mqtt_config.broker_address());

    // Determine the language configuration
    let localization_config = LocalizationConfig::new(args.language);
    info!(
        "Initial language set to: {:?}",
        localization_config.current_language
    );

    let mut app = App::new();

    // Initialize resources first
    app.insert_resource(localization_config)
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .insert_resource(mqtt_config)
        .insert_resource(profile::load_or_create_profile_with_override_full(
            args.player_id,
            args.player_name,
        ));
    // Script resources are now handled by the ScriptPlugin

    // Add default plugins with custom window configuration
    let player_profile = app.world().resource::<profile::PlayerProfile>();
    let window_title = format!("IoTCraft - {}", player_profile.player_name);

    // Get client number from player_id (assuming format like "player-1", "player-2", etc.)
    let client_offset = extract_client_number(&player_profile.player_id).unwrap_or(0);
    let window_x = 50.0 + (client_offset as f32 * 300.0); // Offset by 300px per client
    let window_y = 50.0 + (client_offset as f32 * 50.0); // Offset by 50px per client

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: window_title,
            resolution: bevy::window::WindowResolution::new(1280, 720),
            position: WindowPosition::At(IVec2::new(window_x as i32, window_y as i32)),
            ..default()
        }),
        ..default()
    }));

    // Initialize fonts resource immediately after AssetServer is available
    // We need to do this in a way that ensures AssetServer exists
    app.world_mut()
        .resource_scope(|world, asset_server: Mut<AssetServer>| {
            let fonts = Fonts::new(&asset_server);
            world.insert_resource(fonts);
        });

    app.add_plugins(FontPlugin) // Keep FontPlugin for any additional font-related systems
        .add_plugins(LocalizationPlugin); // Load localization after fonts

    app.add_plugins(CameraControllerPlugin)
        .add_plugins(PlayerControllerPlugin) // Add player controller for walking/flying modes
        .add_plugins(input::GamepadInputPlugin) // Add gamepad input support
        .add_plugins(script::script_systems::ScriptPlugin) // Add script plugin early for PendingCommands resource
        .add_plugins(SharedMaterialsPlugin); // Add shared materials for optimized rendering

    // Add console plugin conditionally
    #[cfg(feature = "console")]
    app.add_plugins(ConsolePlugin);

    app.add_plugins(DevicePlugin)
        .add_plugins(DevicePositioningPlugin)
        .add_plugins(EnvironmentPlugin)
        .add_plugins(MyInteractionPlugin)
        .add_plugins(MqttPlugin)
        .add_plugins(InventoryPlugin)
        .add_plugins(InventoryUiPlugin)
        .add_plugins(CrosshairPlugin)
        .add_plugins(ErrorIndicatorPlugin)
        .add_plugins(MainMenuPlugin)
        .add_plugins(MinimapPlugin)
        .add_plugins(WorldPlugin)
        .add_plugins(MultiplayerPlugin)
        .add_plugins(SharedWorldPlugin)
        .add_plugins(WorldPublisherPlugin)
        .add_plugins(WorldDiscoveryPlugin)
        .add_plugins(PlayerAvatarPlugin);

    // Add CommandExecutedEvent unconditionally since it's used by execute_pending_commands
    app.add_event::<CommandExecutedEvent>();

    // Add MCP plugin only if --mcp flag is provided
    if args.mcp {
        let mcp_port = std::env::var("MCP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse::<u16>()
            .unwrap_or(8080);
        info!("Starting in MCP server mode on port {}", mcp_port);
        app.add_plugins(mcp::McpPlugin);
    }

    app.init_state::<GameState>();

    // Console is now managed by ConsolePlugin - no additional configuration needed

    #[cfg(feature = "console")]
    app.insert_resource(BlinkState::default());

    // .add_systems(Update, draw_cursor) // Disabled: InteractionPlugin handles cursor drawing

    // Define SystemSets for proper ordering
    #[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
    enum GameSystemSet {
        Commands,  // Execute commands and spawn entities
        Logic,     // Game logic that depends on spawned entities
        Rendering, // Visual updates and UI
    }

    // Configure system set ordering
    app.configure_sets(
        Update,
        (
            GameSystemSet::Commands,
            GameSystemSet::Logic,
            GameSystemSet::Rendering,
        )
            .chain(),
    );

    // Entity spawning should happen in PreUpdate to ensure entities are available in Update
    app.add_systems(
        PreUpdate,
        (
            // Commands must run first to spawn entities - run in PreUpdate for better isolation
            execute_pending_commands,
            execute_console_commands,
        ),
    );

    app.add_systems(
        Update,
        (
            // Logic systems that depend on spawned entities
            manage_camera_controller,
            handle_mouse_capture,
            handle_inventory_input_bundled,
        )
            .in_set(GameSystemSet::Logic),
    );

    app.add_systems(
        Update,
        (
            // Visual/rendering systems run last
            rotate_logo_system,
            crate::devices::device_positioning::draw_drag_feedback,
            handle_diagnostics_toggle_bundled,
            update_diagnostics_content_bundled,
        )
            .in_set(GameSystemSet::Rendering),
    );

    // Add console-specific systems only when console feature is enabled
    #[cfg(feature = "console")]
    app.add_systems(Update, blink_publisher_system.in_set(GameSystemSet::Logic));

    // Add console-dependent systems only when console feature is enabled
    #[cfg(feature = "console")]
    app.add_systems(
        Update,
        crate::console::esc_handling::handle_esc_key
            .after(ConsoleSet::COMMANDS)
            .in_set(GameSystemSet::Logic),
    );

    app.init_resource::<DiagnosticsVisible>()
        .init_resource::<input::GamepadConfig>() // Initialize gamepad configuration
        .add_systems(Startup, setup_diagnostics_ui_bundled)
        .run();
}

#[cfg(feature = "console")]
fn blink_publisher_system(
    mut blink_state: ResMut<BlinkState>,
    #[cfg(feature = "console")] device_query: Query<&DeviceEntity, With<BlinkCube>>,
    mqtt_outgoing: Option<Res<crate::mqtt::core_service::MqttOutgoingTx>>,
) {
    if blink_state.light_state != blink_state.last_sent {
        let payload = if blink_state.light_state { "ON" } else { "OFF" };

        // Send blink command to all registered lamp devices using Core MQTT Service
        if let Some(outgoing_tx) = mqtt_outgoing {
            for device in device_query.iter() {
                if device.device_type == "lamp" {
                    let device_id = device.device_id.clone();
                    let topic = format!("home/{}/light", device_id);
                    let payload_str = payload.to_string();

                    info!(
                        "MQTT: Publishing blink command '{}' to topic '{}' via Core MQTT Service",
                        payload_str, topic
                    );

                    if let Ok(tx) = outgoing_tx.0.lock() {
                        let outgoing_msg =
                            crate::mqtt::core_service::OutgoingMqttMessage::GenericPublish {
                                topic,
                                payload: payload_str,
                                qos: rumqttc::QoS::AtMostOnce,
                                retain: false,
                            };

                        if let Err(e) = tx.send(outgoing_msg) {
                            error!(
                                "MQTT: Failed to send blink command to Core MQTT Service: {}",
                                e
                            );
                        } else {
                            info!("MQTT: Blink command queued successfully via Core MQTT Service");
                        }
                    }
                }
            }
        } else {
            warn!("MQTT: Core MQTT Service not available for blink commands");
        }

        blink_state.last_sent = blink_state.light_state;
    }
}

fn rotate_logo_system(time: Res<Time>, mut query: Query<&mut Transform, With<LogoCube>>) {
    for mut transform in &mut query {
        // rotate slowly around the Y axis
        transform.rotate_y(time.delta_secs() * 0.5);
        transform.rotate_x(time.delta_secs() * 0.5);
    }
}

// System to manage camera controller state based on console state
#[cfg(feature = "console")]
fn manage_camera_controller(
    console_manager: Option<Res<ConsoleManager>>,
    mut camera_query: Query<&mut CameraController, With<Camera>>,
) {
    if let Ok(mut camera_controller) = camera_query.single_mut() {
        // Disable camera controller when console is open
        let console_open = console_manager
            .map(|manager| manager.console.is_visible())
            .unwrap_or(false);
        camera_controller.enabled = !console_open;
    }
}

// Alternative system when console feature is disabled
#[cfg(not(feature = "console"))]
fn manage_camera_controller(mut camera_query: Query<&mut CameraController, With<Camera>>) {
    if let Ok(mut camera_controller) = camera_query.single_mut() {
        // Always enable camera controller when console is not available
        camera_controller.enabled = true;
    }
}

// LEGACY: Resource and systems moved to debug module (debug_params.rs)
// Resource to track diagnostics visibility
// #[derive(Resource)]
// struct DiagnosticsVisible {
//     visible: bool,
// }

// impl Default for DiagnosticsVisible {
//     fn default() -> Self {
//         Self { visible: false }
//     }
// }

// #[derive(Component)]
// struct DiagnosticsText;

// #[derive(Component)]
// struct DiagnosticsOverlay;

// LEGACY: System replaced by handle_diagnostics_toggle_bundled in debug_commands.rs
// System to handle F3 key toggle
/*
fn handle_diagnostics_toggle(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut diagnostics_visible: ResMut<DiagnosticsVisible>,
    mut diagnostics_query: Query<&mut Visibility, With<DiagnosticsOverlay>>,
) {
    if keyboard_input.just_pressed(KeyCode::F3) {
        diagnostics_visible.visible = !diagnostics_visible.visible;

        if let Ok(mut visibility) = diagnostics_query.single_mut() {
            *visibility = if diagnostics_visible.visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }

        info!(
            "Diagnostics screen toggled: {}",
            diagnostics_visible.visible
        );
    }
}
*/

// LEGACY: System replaced by update_diagnostics_content_bundled in debug_commands.rs
/*
// System to update diagnostics content
fn update_diagnostics_content(
    diagnostics_visible: Res<DiagnosticsVisible>,
    mut diagnostics_text_query: Query<&mut Text, With<DiagnosticsText>>,
    camera_query: Query<(&Transform, &CameraController), With<Camera>>,
    voxel_world: Res<VoxelWorld>,
    inventory: Res<PlayerInventory>,
    device_query: Query<&DeviceEntity>,
    player_avatar_query: Query<
        (&Transform, &crate::player_avatar::PlayerAvatar),
        With<crate::multiplayer::RemotePlayer>,
    >,
    local_profile: Res<crate::profile::PlayerProfile>,
    temperature: Res<TemperatureResource>,
    time: Res<Time>,
    multiplayer_mode: Res<crate::multiplayer::MultiplayerMode>,
    multiplayer_status: Res<crate::multiplayer::MultiplayerConnectionStatus>,
    world_discovery: Res<crate::multiplayer::WorldDiscovery>,
) {
    if !diagnostics_visible.visible {
        return;
    }

    if let Ok(mut text) = diagnostics_text_query.single_mut() {
        if let Ok((transform, camera_controller)) = camera_query.single() {
            let translation = transform.translation;
            let yaw_degrees = camera_controller.yaw.to_degrees();
            let pitch_degrees = camera_controller.pitch.to_degrees();

            // Calculate additional useful information
            let device_count = device_query.iter().count();
            let block_count = voxel_world.blocks.len();
            let selected_slot = inventory.selected_slot + 1; // 1-indexed for display

            // Get selected item info
            let selected_item = if let Some(item_stack) = inventory
                .slots
                .get(inventory.selected_slot)
                .and_then(|slot| slot.as_ref())
            {
                if item_stack.count > 0 {
                    format!(
                        "{} x {}",
                        item_stack.count,
                        match item_stack.item_type {
                            crate::inventory::ItemType::Block(block_type) => match block_type {
                                BlockType::Grass => "Grass",
                                BlockType::Dirt => "Dirt",
                                BlockType::Stone => "Stone",
                                BlockType::QuartzBlock => "Quartz Block",
                                BlockType::GlassPane => "Glass Pane",
                                BlockType::CyanTerracotta => "Cyan Terracotta",
                                BlockType::Water => "Water",
                            },
                        }
                    )
                } else {
                    "Empty".to_string()
                }
            } else {
                "Empty".to_string()
            };

            // Get multiplayer player information
            let remote_player_count = player_avatar_query.iter().count();
            let mut player_list = Vec::new();
            player_list.push(format!(
                "  {} (Local): X={:.1} Y={:.1} Z={:.1}",
                local_profile.player_name, translation.x, translation.y, translation.z
            ));

            for (player_transform, player_avatar) in player_avatar_query.iter() {
                let pos = player_transform.translation;
                player_list.push(format!(
                    "  {} (Remote): X={:.1} Y={:.1} Z={:.1}",
                    player_avatar.player_id, pos.x, pos.y, pos.z
                ));
            }

            let players_text = if player_list.len() <= 1 {
                "  No other players connected".to_string()
            } else {
                player_list.join("\n")
            };

            // Get MQTT broker connection status using temperature resource as indicator
            let mqtt_connection_status = if temperature.value.is_some() {
                "✅ Connected (MQTT broker available)"
            } else {
                "🔄 Connecting to MQTT broker..."
            };

            // Get multiplayer mode information and world ID
            let (multiplayer_mode_text, current_world_id) = match &*multiplayer_mode {
                crate::multiplayer::MultiplayerMode::SinglePlayer => {
                    ("🚫 SinglePlayer".to_string(), "None".to_string())
                }
                crate::multiplayer::MultiplayerMode::HostingWorld {
                    world_id,
                    is_published,
                } => {
                    let mode_text = if *is_published {
                        "🏠 Hosting (Public)"
                    } else {
                        "🏠 Hosting (Private)"
                    };
                    (mode_text.to_string(), world_id.clone())
                }
                crate::multiplayer::MultiplayerMode::JoinedWorld {
                    world_id,
                    host_player: _,
                } => ("👥 Joined World".to_string(), world_id.clone()),
            };

            let multiplayer_enabled = if multiplayer_status.connection_available {
                "✅ Enabled"
            } else {
                "❌ Disabled"
            };

            // Get MQTT subscription information from WorldDiscovery resource
            let subscribed_topics = vec![
                "iotcraft/worlds/+/info".to_string(),
                "iotcraft/worlds/+/data".to_string(),
                "iotcraft/worlds/+/data/chunk".to_string(),
                "iotcraft/worlds/+/changes".to_string(),
                "iotcraft/worlds/+/state/blocks/placed".to_string(),
                "iotcraft/worlds/+/state/blocks/removed".to_string(),
            ];

            // Get last messages from WorldDiscovery resource
            let last_messages = if let Ok(messages) = world_discovery.last_messages.try_lock() {
                messages.clone()
            } else {
                std::collections::HashMap::new()
            };

            let topics_text = subscribed_topics
                .iter()
                .map(|topic| {
                    // Find matching topic in last_messages (handling wildcards)
                    let _pattern = topic.replace("+", "[^/]+");
                    let matching_message = last_messages.iter().find(|(msg_topic, _)| {
                        // Simple pattern matching for wildcard topics
                        if topic.contains("+") {
                            // Create a basic regex-like match
                            let pattern_parts: Vec<&str> = topic.split("+").collect();
                            if pattern_parts.len() == 2 {
                                msg_topic.starts_with(pattern_parts[0])
                                    && msg_topic.ends_with(pattern_parts[1])
                            } else {
                                false
                            }
                        } else {
                            *msg_topic == topic
                        }
                    });

                    if let Some((_, last_msg)) = matching_message {
                        format!("  • {}: {}", topic, last_msg.content)
                    } else {
                        format!("  • {}: (no messages)", topic)
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");

            let uptime = time.elapsed_secs();
            let minutes = (uptime / 60.0) as u32;
            let seconds = (uptime % 60.0) as u32;

            text.0 = format!(
                "IoTCraft Debug Information (Press F3 to toggle)                        MQTT SUBSCRIPTIONS\n\
                -------------------------------------------------  |  --------------------------------------\n\
                                                               |\n\
                - PLAYER INFORMATION                           |  Current World Filter: {}\n\
                Position: X={:.2}  Y={:.2}  Z={:.2}               |\n\
                Rotation: Yaw={:.1}°  Pitch={:.1}°                    |  Subscribed Topics:\n\
                Selected Slot: {} ({})                        |  {}\n\
                                                               |\n\
                - MULTIPLAYER INFORMATION                      |\n\
                MQTT Broker: {}                               |\n\
                Multiplayer Status: {}                        |\n\
                Multiplayer Mode: {}                          |\n\
                Current World ID: {}                          |\n\
                Connected Players: {} (1 local + {} remote)   |\n\
                {}                                             |\n\
                                                               |\n\
                - WORLD INFORMATION                            |\n\
                Total Blocks: {}                              |\n\
                IoT Devices: {}                               |\n\
                Session Time: {}m {}s                         |\n\
                                                               |\n\
                - SCRIPT COMMANDS                              |\n\
                Teleport: tp {:.2} {:.2} {:.2}                    |\n\
                Look Direction: look {:.1} {:.1}                  |\n\
                                                               |\n\
                - CONTROLS                                     |\n\
                F3: Toggle this debug screen                  |\n\
                T: Open console                               |\n\
                1-9: Select inventory slot                    |\n\
                Mouse Wheel: Scroll inventory slots           |",
                current_world_id, // Used for the filter line
                translation.x,
                translation.y,
                translation.z,
                yaw_degrees,
                pitch_degrees,
                selected_slot,
                selected_item,
                topics_text,
                mqtt_connection_status,
                multiplayer_enabled,
                multiplayer_mode_text,
                current_world_id,
                remote_player_count + 1,
                remote_player_count,
                players_text,
                block_count,
                device_count,
                minutes,
                seconds,
                translation.x,
                translation.y,
                translation.z,
                yaw_degrees,
                pitch_degrees
            );
        }
    }
}
*/

// LEGACY: System replaced by setup_diagnostics_ui_bundled in debug_commands.rs
/*
// System to setup diagnostics UI
fn setup_diagnostics_ui(mut commands: Commands, fonts: Res<Fonts>) {
    // Create a full-width diagnostics panel at the top
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                right: Val::Px(10.0),
                width: Val::Auto,
                height: Val::Px(480.0), // Fixed height to ensure proper display
                padding: UiRect::all(Val::Px(20.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)), // Dark semi-transparent background
            Visibility::Hidden,                                 // Start hidden
            DiagnosticsOverlay,                                 // Add the component for toggling
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("IoTCraft Debug Information (Press F3 to toggle)\n\nLoading..."),
                TextFont {
                    font: fonts.regular.clone(),
                    font_size: 16.0,
                    font_smoothing: bevy::text::FontSmoothing::default(),
                    line_height: bevy::text::LineHeight::default(),
                },
                TextColor(Color::WHITE),
                DiagnosticsText, // Component for text updates
            ));
        });
}
*/

// System to handle mouse capture when window is clicked...
fn handle_mouse_capture(
    mut windows: Query<&mut Window>,
    mut cursor_options_query: Query<&mut bevy::window::CursorOptions>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    game_state: Res<State<GameState>>,
) {
    // Only handle mouse recapture in InGame state (after it was released)
    if *game_state.get() == GameState::InGame && mouse_button_input.just_pressed(MouseButton::Left)
    {
        for window in &mut windows {
            if !window.focused {
                continue;
            }

            // Query for cursor options - now in separate component in Bevy 0.17
            if let Ok(mut cursor_options) = cursor_options_query.single_mut() {
                // Only capture if cursor is currently not captured
                if cursor_options.grab_mode == CursorGrabMode::None {
                    cursor_options.grab_mode = CursorGrabMode::Locked;
                    cursor_options.visible = false;
                }
            }
        }
    }
}

// LEGACY: Systems replaced by handle_inventory_input_bundled in inventory_commands.rs
/*
// System to handle inventory slot selection with number keys and mouse wheel
#[cfg(feature = "console")]
fn handle_inventory_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<PlayerInventory>,
    accumulated_mouse_scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    console_manager: Option<Res<ConsoleManager>>,
    game_state: Res<State<GameState>>,
) {
    // Don't handle input when console is open or in any menu state
    let console_open = console_manager
        .map(|manager| manager.console.is_visible())
        .unwrap_or(false);
    if console_open || *game_state.get() != GameState::InGame {
        return;
    }

    // Handle mouse wheel for inventory slot switching
    if accumulated_mouse_scroll.delta.y != 0.0 {
        let current_slot = inventory.selected_slot;
        let new_slot = if accumulated_mouse_scroll.delta.y > 0.0 {
            // Scroll up - previous slot (wraps around)
            if current_slot == 0 {
                8
            } else {
                current_slot - 1
            }
        } else {
            // Scroll down - next slot (wraps around)
            if current_slot == 8 {
                0
            } else {
                current_slot + 1
            }
        };

        if new_slot != current_slot {
            inventory.select_slot(new_slot);
            info!("Selected inventory slot {}", new_slot + 1);
        }
    }

    // Handle number keys 1-9 for slot selection
    let key_mappings = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (key, slot) in key_mappings {
        if keyboard_input.just_pressed(key) {
            inventory.select_slot(slot);
            info!("Selected inventory slot {}", slot + 1);
            break;
        }
    }
}

// Alternative system for inventory input when console feature is disabled
#[cfg(not(feature = "console"))]
fn handle_inventory_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<PlayerInventory>,
    accumulated_mouse_scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    game_state: Res<State<GameState>>,
) {
    // Don't handle input when in any menu state (console not available to check)
    if *game_state.get() != GameState::InGame {
        return;
    }

    // Handle mouse wheel for inventory slot switching
    if accumulated_mouse_scroll.delta.y != 0.0 {
        let current_slot = inventory.selected_slot;
        let new_slot = if accumulated_mouse_scroll.delta.y > 0.0 {
            // Scroll up - previous slot (wraps around)
            if current_slot == 0 {
                8
            } else {
                current_slot - 1
            }
        } else {
            // Scroll down - next slot (wraps around)
            if current_slot == 8 {
                0
            } else {
                current_slot + 1
            }
        };

        if new_slot != current_slot {
            inventory.select_slot(new_slot);
            info!("Selected inventory slot {}", new_slot + 1);
        }
    }

    // Handle number keys 1-9 for slot selection
    let key_mappings = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (key, slot) in key_mappings {
        if keyboard_input.just_pressed(key) {
            inventory.select_slot(slot);
            info!("Selected inventory slot {}", slot + 1);
            break;
        }
    }
}
*/
