// Desktop-only main application - WASM uses lib.rs instead
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
use rumqttc::{Client, Event, MqttOptions, Outgoing, QoS};
#[cfg(not(target_arch = "wasm32"))]
use serde_json::json;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
mod camera_controllers;
#[cfg(not(target_arch = "wasm32"))]
mod config;
#[cfg(not(target_arch = "wasm32"))]
mod console;
#[cfg(not(target_arch = "wasm32"))]
mod devices;
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
use devices::*;
#[cfg(not(target_arch = "wasm32"))]
use environment::*;
#[cfg(not(target_arch = "wasm32"))]
use fonts::{FontPlugin, Fonts};
#[cfg(not(target_arch = "wasm32"))]
use interaction::InteractionPlugin as MyInteractionPlugin;
#[cfg(not(target_arch = "wasm32"))]
use inventory::{InventoryPlugin, PlayerInventory};
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
use world::WorldPlugin;

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
    /// Run in MCP (Model Context Protocol) server mode
    #[arg(long)]
    mcp: bool,
}

// Helper function to write to console if available
#[cfg(feature = "console")]
fn write_to_console(_writer: &mut Option<()>, message: String) {
    // Log the message instead since PrintConsoleLine was removed
    info!("Console: {}", message);
}

#[cfg(not(target_arch = "wasm32"))]
fn execute_pending_commands(
    mut pending_commands: ResMut<crate::script::script_types::PendingCommands>,
    #[cfg(feature = "console")] mut print_console_line: Option<()>,
    mut command_executed_events: EventWriter<CommandExecutedEvent>,
    #[cfg(feature = "console")] mut blink_state: ResMut<BlinkState>,
    temperature: Res<TemperatureResource>,
    mqtt_config: Res<MqttConfig>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    query: Query<(Entity, &VoxelBlock)>,
    device_query: Query<(&DeviceEntity, &Transform), Without<Camera>>,
    mut inventory: ResMut<PlayerInventory>,
    mut camera_query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    for command in pending_commands.commands.drain(..) {
        info!("Executing queued command: {}", command);

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
            "blink" => {
                if parts.len() == 2 {
                    let action = parts[1];
                    match action {
                        "start" => {
                            #[cfg(feature = "console")]
                            {
                                blink_state.blinking = true;
                                write_to_console(
                                    &mut print_console_line,
                                    "Blink started".to_string(),
                                );
                            }
                            info!("Blink started via script");
                        }
                        "stop" => {
                            #[cfg(feature = "console")]
                            {
                                blink_state.blinking = false;
                                write_to_console(
                                    &mut print_console_line,
                                    "Blink stopped".to_string(),
                                );
                            }
                            info!("Blink stopped via script");
                        }
                        _ => {
                            #[cfg(feature = "console")]
                            write_to_console(
                                &mut print_console_line,
                                "Usage: blink [start|stop]".to_string(),
                            );
                        }
                    }
                }
            }
            "mqtt" => {
                if parts.len() == 2 {
                    let action = parts[1];
                    match action {
                        "status" => {
                            let status = if temperature.value.is_some() {
                                "Connected to MQTT broker"
                            } else {
                                "Connecting to MQTT broker..."
                            };
                            #[cfg(feature = "console")]
                            write_to_console(&mut print_console_line, status.to_string());
                            info!("MQTT status requested via script");
                        }
                        "temp" => {
                            let temp_msg = if let Some(val) = temperature.value {
                                format!("Current temperature: {:.1}°C", val)
                            } else {
                                "No temperature data available".to_string()
                            };
                            #[cfg(feature = "console")]
                            write_to_console(&mut print_console_line, temp_msg);
                        }
                        _ => {
                            #[cfg(feature = "console")]
                            write_to_console(
                                &mut print_console_line,
                                "Usage: mqtt [status|temp]".to_string(),
                            );
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

                                // Try to create a temporary client for simulation
                                // If MQTT fails, still complete the spawn command
                                match (|| -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                                    let mut mqtt_options = MqttOptions::new(
                                        "spawn-client",
                                        &mqtt_config.host,
                                        mqtt_config.port,
                                    );
                                    mqtt_options.set_keep_alive(Duration::from_secs(5));
                                    // Note: set_connection_timeout doesn't exist, using default timeout
                                    let (client, mut connection) = Client::new(mqtt_options, 10);

                                    client.publish(
                                        "devices/announce",
                                        QoS::AtMostOnce,
                                        false,
                                        payload.as_bytes(),
                                    )?;

                                    // Try to drive the event loop briefly with timeout
                                    let start_time = std::time::Instant::now();
                                    const TIMEOUT_MS: u64 = 1000; // 1 second max

                                    while start_time.elapsed().as_millis() < TIMEOUT_MS as u128 {
                                        match connection.try_recv() {
                                            Ok(Ok(Event::Outgoing(Outgoing::Publish(_)))) => {
                                                info!("MQTT publish successful for spawn command");
                                                return Ok(());
                                            }
                                            Ok(Ok(_)) => {}, // Other events, continue
                                            Ok(Err(_)) | Err(_) => {
                                                // No more events or connection error, wait a bit
                                                std::thread::sleep(Duration::from_millis(50));
                                            }
                                        }
                                    }

                                    // Timeout reached, but still continue
                                    info!("MQTT publish timeout for spawn command, continuing anyway");
                                    Ok(())
                                })() {
                                    Ok(_) => info!("MQTT spawn announcement completed"),
                                    Err(e) => warn!("MQTT spawn announcement failed: {} (continuing anyway)", e),
                                }

                                let result_msg =
                                    format!("Spawn command sent for device {}", device_id);
                                #[cfg(feature = "console")]
                                write_to_console(&mut print_console_line, result_msg.clone());

                                // Emit command executed event if this was from MCP
                                if let Some(req_id) = request_id.clone() {
                                    command_executed_events.write(CommandExecutedEvent {
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

                                // Create a temporary client for simulation
                                let mut mqtt_options = MqttOptions::new(
                                    "spawn-door-client",
                                    &mqtt_config.host,
                                    mqtt_config.port,
                                );
                                mqtt_options.set_keep_alive(Duration::from_secs(5));
                                let (client, mut connection) = Client::new(mqtt_options, 10);

                                client
                                    .publish(
                                        "devices/announce",
                                        QoS::AtMostOnce,
                                        false,
                                        payload.as_bytes(),
                                    )
                                    .unwrap();

                                // Drive the event loop to ensure the message is sent
                                for notification in connection.iter() {
                                    if let Ok(Event::Outgoing(Outgoing::Publish(_))) = notification
                                    {
                                        break;
                                    }
                                }

                                let result_msg =
                                    format!("Spawn door command sent for device {}", device_id);
                                #[cfg(feature = "console")]
                                write_to_console(&mut print_console_line, result_msg.clone());

                                // Emit command executed event if this was from MCP
                                if let Some(req_id) = request_id.clone() {
                                    command_executed_events.write(CommandExecutedEvent {
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
                                        write_to_console(
                                            &mut print_console_line,
                                            error_msg.clone(),
                                        );

                                        // Emit error event if this was from MCP
                                        if let Some(req_id) = request_id.clone() {
                                            command_executed_events.write(CommandExecutedEvent {
                                                request_id: req_id,
                                                result: error_msg,
                                            });
                                        }
                                        continue;
                                    }
                                };

                                voxel_world.set_block(IVec3::new(x, y, z), block_type);

                                // Spawn the block
                                let cube_mesh =
                                    meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
                                let material = match block_type {
                                    BlockType::Water => materials.add(StandardMaterial {
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
                                            asset_server.load(texture_path);
                                        materials.add(StandardMaterial {
                                            base_color_texture: Some(texture),
                                            ..default()
                                        })
                                    }
                                };

                                commands.spawn((
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
                                write_to_console(&mut print_console_line, result_msg.clone());

                                // Emit command executed event if this was from MCP
                                if let Some(req_id) = request_id.clone() {
                                    command_executed_events.write(CommandExecutedEvent {
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
                                let result_msg = if voxel_world.remove_block(&position).is_some() {
                                    // Remove the block entity
                                    for (entity, block) in query.iter() {
                                        if block.position == position {
                                            commands.entity(entity).despawn();
                                        }
                                    }
                                    format!("Removed block at ({}, {}, {})", x, y, z)
                                } else {
                                    format!("No block found at ({}, {}, {})", x, y, z)
                                };

                                #[cfg(feature = "console")]
                                write_to_console(&mut print_console_line, result_msg.clone());

                                // Emit command executed event if this was from MCP
                                if let Some(req_id) = request_id.clone() {
                                    command_executed_events.write(CommandExecutedEvent {
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
                                write_to_console(
                                    &mut print_console_line,
                                    format!("Invalid item type: {}", item_type_str),
                                );
                                continue;
                            }
                        };

                        inventory.add_items(item_type, quantity as u32);
                        #[cfg(feature = "console")]
                        write_to_console(
                            &mut print_console_line,
                            format!("Added {} x {}", quantity, item_type_str),
                        );
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
                                                    write_to_console(
                                                        &mut print_console_line,
                                                        format!(
                                                            "Invalid block type: {}",
                                                            block_type_str
                                                        ),
                                                    );
                                                    continue;
                                                }
                                            };

                                            // Debug: VoxelWorld before adding blocks
                                            info!(
                                                "VoxelWorld before wall command: {} blocks",
                                                voxel_world.blocks.len()
                                            );

                                            let material = match block_type_enum {
                                                BlockType::Water => {
                                                    materials.add(StandardMaterial {
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
                                                        asset_server.load(texture_path);
                                                    materials.add(StandardMaterial {
                                                        base_color_texture: Some(texture),
                                                        ..default()
                                                    })
                                                }
                                            };

                                            let cube_mesh = meshes
                                                .add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));

                                            let mut blocks_added = 0;
                                            for x in x1..=x2 {
                                                for y in y1..=y2 {
                                                    for z in z1..=z2 {
                                                        voxel_world.set_block(
                                                            IVec3::new(x, y, z),
                                                            block_type_enum,
                                                        );
                                                        blocks_added += 1;

                                                        commands.spawn((
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
                                                voxel_world.blocks.len(),
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
                                                let has_block = voxel_world.is_block_at(pos);
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
                                            write_to_console(
                                                &mut print_console_line,
                                                result_msg.clone(),
                                            );

                                            // Emit command executed event if this was from MCP
                                            if let Some(req_id) = request_id.clone() {
                                                command_executed_events.write(
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
                                    camera_query.single_mut()
                                {
                                    // Set the camera position
                                    transform.translation = Vec3::new(x, y, z);

                                    #[cfg(feature = "console")]
                                    write_to_console(
                                        &mut print_console_line,
                                        format!("Teleported to ({:.1}, {:.1}, {:.1})", x, y, z),
                                    );
                                    info!("Camera teleported to ({:.1}, {:.1}, {:.1})", x, y, z);
                                } else {
                                    #[cfg(feature = "console")]
                                    write_to_console(
                                        &mut print_console_line,
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
                                camera_query.single_mut()
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
                                write_to_console(
                                    &mut print_console_line,
                                    format!(
                                        "Set look angles to yaw: {:.1}°, pitch: {:.1}°",
                                        yaw, pitch
                                    ),
                                );
                                info!(
                                    "Camera look angles set to yaw: {:.1}°, pitch: {:.1}°",
                                    yaw, pitch
                                );
                            } else {
                                #[cfg(feature = "console")]
                                write_to_console(
                                    &mut print_console_line,
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
                let device_list: Vec<String> = device_query
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
                write_to_console(&mut print_console_line, result_text.clone());
                info!("Executed list command, found {} devices", device_list.len());

                // Emit command executed event if this was from MCP
                if let Some(req_id) = request_id {
                    command_executed_events.write(CommandExecutedEvent {
                        request_id: req_id,
                        result: result_text,
                    });
                }
            }
            _ => {
                #[cfg(feature = "console")]
                write_to_console(
                    &mut print_console_line,
                    format!("Unknown command: {}", command),
                );
            }
        }
    }
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
fn main() {
    let args = Args::parse();

    // Logging is now handled by Bevy's LogPlugin in DefaultPlugins

    // Note: Script execution is now handled by the ScriptPlugin

    // Load MQTT configuration from CLI args and environment variables
    let mqtt_config = MqttConfig::from_env_with_override(args.mqtt_server);
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
        .insert_resource(profile::load_or_create_profile_with_override(
            args.player_id,
        ));
    // Script resources are now handled by the ScriptPlugin

    // Add default plugins with custom window configuration
    let player_profile = app.world().resource::<profile::PlayerProfile>();
    let window_title = format!("IoTCraft - {}", player_profile.player_id);

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
        info!("Starting in MCP server mode");
        app.add_plugins(mcp::McpPlugin);
    }

    app.init_state::<GameState>();

    // Console is now managed by ConsolePlugin - no additional configuration needed

    #[cfg(feature = "console")]
    app.insert_resource(BlinkState::default());

    // .add_systems(Update, draw_cursor) // Disabled: InteractionPlugin handles cursor drawing
    app.add_systems(
        Update,
        (
            rotate_logo_system,
            crate::devices::device_positioning::draw_drag_feedback,
        ),
    );

    // Add console-specific systems only when console feature is enabled
    #[cfg(feature = "console")]
    app.add_systems(Update, blink_publisher_system);

    app.add_systems(Update, manage_camera_controller)
        .add_systems(Update, handle_mouse_capture);

    // Add console-dependent systems only when console feature is enabled
    #[cfg(feature = "console")]
    app.add_systems(
        Update,
        crate::console::esc_handling::handle_esc_key.after(ConsoleSet::COMMANDS),
    );

    app.init_resource::<DiagnosticsVisible>()
        .add_systems(Startup, setup_diagnostics_ui)
        .add_systems(Update, execute_pending_commands)
        .add_systems(Update, handle_inventory_input)
        .add_systems(Update, handle_diagnostics_toggle)
        .add_systems(Update, update_diagnostics_content)
        .run();
}

#[cfg(feature = "console")]
fn blink_publisher_system(
    mut blink_state: ResMut<BlinkState>,
    #[cfg(feature = "console")] device_query: Query<&DeviceEntity, With<BlinkCube>>,
    mqtt_config: Res<MqttConfig>,
) {
    if blink_state.light_state != blink_state.last_sent {
        let payload = if blink_state.light_state { "ON" } else { "OFF" };

        // Send blink command to all registered lamp devices
        for device in device_query.iter() {
            if device.device_type == "lamp" {
                let device_id = device.device_id.clone();
                let payload_str = payload.to_string();
                let mqtt_host = mqtt_config.host.clone();
                let mqtt_port = mqtt_config.port;

                // Send MQTT message in a separate thread to avoid blocking
                std::thread::spawn(move || {
                    info!(
                        "MQTT: Connecting blink client to publish to device {}",
                        device_id
                    );
                    let mut opts = MqttOptions::new("bevy_blink_client", &mqtt_host, mqtt_port);
                    opts.set_keep_alive(Duration::from_secs(5));
                    let (client, mut connection) = Client::new(opts, 10);

                    let topic = format!("home/{}/light", device_id);
                    info!(
                        "MQTT: Publishing blink command '{}' to topic '{}'",
                        payload_str, topic
                    );
                    match client.publish(&topic, QoS::AtMostOnce, false, payload_str.as_bytes()) {
                        Ok(_) => {
                            // Drive until publish is sent
                            for notification in connection.iter() {
                                if let Ok(Event::Outgoing(Outgoing::Publish(_))) = notification {
                                    info!(
                                        "MQTT: Blink command sent successfully: {} to {}",
                                        payload_str, topic
                                    );
                                    break;
                                }
                            }
                        }
                        Err(e) => error!("MQTT: Failed to publish blink command: {}", e),
                    }
                });
            }
        }

        // Give broker time
        std::thread::sleep(Duration::from_millis(100));
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

// Resource to track diagnostics visibility
#[derive(Resource)]
struct DiagnosticsVisible {
    visible: bool,
}

impl Default for DiagnosticsVisible {
    fn default() -> Self {
        Self { visible: false }
    }
}

#[derive(Component)]
struct DiagnosticsText;

#[derive(Component)]
struct DiagnosticsOverlay;

// System to handle F3 key toggle
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
