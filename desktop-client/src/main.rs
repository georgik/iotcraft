use bevy::asset::Assets;
use bevy::prelude::*;
use bevy::window::CursorGrabMode;
use bevy_console::{
    AddConsoleCommand, ConsoleCommand, ConsoleOpen, ConsoleSet, PrintConsoleLine, reply,
};
use bevy_console::{ConsoleConfiguration, ConsolePlugin};
use camera_controllers::{CameraController, CameraControllerPlugin};
use clap::Parser;
use log::{error, info};
use rumqttc::{Client, Event, MqttOptions, Outgoing, QoS};
use serde_json::json;
use std::time::Duration;

mod camera_controllers;
mod config;
mod console;
mod devices;
mod environment;
mod fonts;
mod interaction;
mod inventory;
mod localization;
mod mcp;
mod minimap;
mod mqtt;
mod script;
mod ui;
use mcp::mcp_types::CommandExecutedEvent;

mod multiplayer;
mod physics_manager;
mod player_avatar;
mod player_controller;
mod profile;
mod world;

// Re-export types for easier access
use config::MqttConfig;
use console::console_types::{ListCommand, LookCommand, TeleportCommand};
use console::*;
use devices::*;
use environment::*;
use fonts::{FontPlugin, Fonts};
use interaction::InteractionPlugin as MyInteractionPlugin;
use inventory::{InventoryPlugin, PlayerInventory, handle_give_command};
use localization::{LocalizationConfig, LocalizationPlugin};
use minimap::MinimapPlugin;
use mqtt::{MqttPlugin, *};
use multiplayer::{
    MultiplayerPlugin, SharedWorldPlugin, WorldDiscoveryPlugin, WorldPublisherPlugin,
};
use physics_manager::PhysicsManagerPlugin;
use player_avatar::PlayerAvatarPlugin;
use player_controller::PlayerControllerPlugin;
use ui::{CrosshairPlugin, ErrorIndicatorPlugin, GameState, InventoryUiPlugin, MainMenuPlugin};
use world::WorldPlugin;

// Define handle_blink_command function for console
fn handle_blink_command(
    mut log: ConsoleCommand<BlinkCommand>,
    mut blink_state: ResMut<BlinkState>,
) {
    if let Some(Ok(BlinkCommand { action })) = log.take() {
        info!("Console command: blink {}", action);
        match action.as_str() {
            "start" => {
                blink_state.blinking = true;
                reply!(log, "Blink started");
                info!("Blink started via console");
            }
            "stop" => {
                blink_state.blinking = false;
                reply!(log, "Blink stopped");
                info!("Blink stopped via console");
            }
            _ => {
                reply!(log, "Usage: blink [start|stop]");
            }
        }
    }
}

// CLI arguments
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
    #[arg(short, long)]
    player_id: Option<String>,
    /// Run in MCP (Model Context Protocol) server mode
    #[arg(long)]
    mcp: bool,
}

fn handle_place_block_command(
    mut log: ConsoleCommand<PlaceBlockCommand>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    if let Some(Ok(PlaceBlockCommand {
        block_type,
        x,
        y,
        z,
    })) = log.take()
    {
        let block_type = match block_type.as_str() {
            "grass" => BlockType::Grass,
            "dirt" => BlockType::Dirt,
            "stone" => BlockType::Stone,
            "quartz_block" => BlockType::QuartzBlock,
            "glass_pane" => BlockType::GlassPane,
            "cyan_terracotta" => BlockType::CyanTerracotta,
            _ => {
                reply!(log, "Invalid block type: {}", block_type);
                return;
            }
        };

        voxel_world.set_block(IVec3::new(x, y, z), block_type);

        // Spawn the block
        let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
        let texture_path = match block_type {
            BlockType::Grass => "textures/grass.webp",
            BlockType::Dirt => "textures/dirt.webp",
            BlockType::Stone => "textures/stone.webp",
            BlockType::QuartzBlock => "textures/quartz_block.webp",
            BlockType::GlassPane => "textures/glass_pane.webp",
            BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
        };
        let texture: Handle<Image> = asset_server.load(texture_path);
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            ..default()
        });

        commands.spawn((
            Mesh3d(cube_mesh),
            MeshMaterial3d(material),
            Transform::from_translation(Vec3::new(x as f32, y as f32, z as f32)),
            VoxelBlock {
                position: IVec3::new(x, y, z),
            },
            // Physics colliders are managed by PhysicsManagerPlugin based on distance and mode
        ));

        reply!(log, "Placed block at ({}, {}, {})", x, y, z);
    }
}

fn handle_wall_command(
    mut log: ConsoleCommand<WallCommand>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    if let Some(Ok(WallCommand {
        block_type,
        x1,
        y1,
        z1,
        x2,
        y2,
        z2,
    })) = log.take()
    {
        let block_type_enum = match block_type.as_str() {
            "grass" => BlockType::Grass,
            "dirt" => BlockType::Dirt,
            "stone" => BlockType::Stone,
            "quartz_block" => BlockType::QuartzBlock,
            "glass_pane" => BlockType::GlassPane,
            "cyan_terracotta" => BlockType::CyanTerracotta,
            _ => {
                reply!(log, "Invalid block type: {}", block_type);
                return;
            }
        };

        let texture_path = match block_type_enum {
            BlockType::Grass => "textures/grass.webp",
            BlockType::Dirt => "textures/dirt.webp",
            BlockType::Stone => "textures/stone.webp",
            BlockType::QuartzBlock => "textures/quartz_block.webp",
            BlockType::GlassPane => "textures/glass_pane.webp",
            BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
        };
        let texture: Handle<Image> = asset_server.load(texture_path);
        let material = materials.add(StandardMaterial {
            base_color_texture: Some(texture),
            ..default()
        });

        let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));

        for x in x1..=x2 {
            for y in y1..=y2 {
                for z in z1..=z2 {
                    voxel_world.set_block(IVec3::new(x, y, z), block_type_enum);

                    commands.spawn((
                        Mesh3d(cube_mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_translation(Vec3::new(x as f32, y as f32, z as f32)),
                        VoxelBlock {
                            position: IVec3::new(x, y, z),
                        },
                        // Physics colliders are managed by PhysicsManagerPlugin based on distance and mode
                    ));
                }
            }
        }

        reply!(
            log,
            "Created a wall of {} from ({}, {}, {}) to ({}, {}, {})",
            block_type,
            x1,
            y1,
            z1,
            x2,
            y2,
            z2
        );
    }
}

fn handle_remove_block_command(
    mut log: ConsoleCommand<RemoveBlockCommand>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut commands: Commands,
    query: Query<(Entity, &VoxelBlock)>,
) {
    if let Some(Ok(RemoveBlockCommand { x, y, z })) = log.take() {
        let position = IVec3::new(x, y, z);
        if voxel_world.remove_block(&position).is_some() {
            // Remove the block entity
            for (entity, block) in query.iter() {
                if block.position == position {
                    commands.entity(entity).despawn();
                }
            }
            reply!(log, "Removed block at ({}, {}, {})", x, y, z);
        } else {
            reply!(log, "No block found at ({}, {}, {})", x, y, z);
        }
    }
}

fn handle_save_map_command(mut log: ConsoleCommand<SaveMapCommand>, voxel_world: Res<VoxelWorld>) {
    if let Some(Ok(SaveMapCommand { filename })) = log.take() {
        match voxel_world.save_to_file(&filename) {
            Ok(_) => {
                reply!(
                    log,
                    "Map saved to '{}' with {} blocks",
                    filename,
                    voxel_world.blocks.len()
                );
                info!(
                    "Map saved to '{}' with {} blocks",
                    filename,
                    voxel_world.blocks.len()
                );
            }
            Err(e) => {
                reply!(log, "Failed to save map: {}", e);
                error!("Failed to save map to '{}': {}", filename, e);
            }
        }
    }
}

fn handle_teleport_command(
    mut log: ConsoleCommand<TeleportCommand>,
    mut camera_query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    if let Some(Ok(TeleportCommand { x, y, z })) = log.take() {
        if let Ok((mut transform, _camera_controller)) = camera_query.single_mut() {
            // Set the camera position
            transform.translation = Vec3::new(x, y, z);

            reply!(log, "Teleported to ({:.1}, {:.1}, {:.1})", x, y, z);
            info!("Camera teleported to ({:.1}, {:.1}, {:.1})", x, y, z);
        } else {
            reply!(log, "Error: Could not find camera to teleport");
        }
    }
}

fn handle_look_command(
    mut log: ConsoleCommand<LookCommand>,
    mut camera_query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    if let Some(Ok(LookCommand { yaw, pitch })) = log.take() {
        if let Ok((mut transform, mut camera_controller)) = camera_query.single_mut() {
            // Convert degrees to radians for internal use
            let yaw_rad = yaw.to_radians();
            let pitch_rad = pitch.to_radians();

            // Update the camera controller's internal yaw and pitch
            camera_controller.yaw = yaw_rad;
            camera_controller.pitch =
                pitch_rad.clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);

            // Apply the rotation to the transform using the same logic as the camera controller
            transform.rotation = Quat::from_euler(
                bevy::math::EulerRot::ZYX,
                0.0,
                camera_controller.yaw,
                camera_controller.pitch,
            );

            reply!(
                log,
                "Set look angles to yaw: {:.1}°, pitch: {:.1}°",
                yaw,
                pitch
            );
            info!(
                "Camera look angles set to yaw: {:.1}°, pitch: {:.1}°",
                yaw, pitch
            );
        } else {
            reply!(log, "Error: Could not find camera to set look direction");
        }
    }
}

fn handle_load_map_command(
    mut log: ConsoleCommand<LoadMapCommand>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    existing_blocks_query: Query<Entity, With<VoxelBlock>>,
) {
    if let Some(Ok(LoadMapCommand { filename })) = log.take() {
        // First, despawn all existing voxel blocks
        for entity in existing_blocks_query.iter() {
            commands.entity(entity).despawn();
        }

        // Load the map from file
        match voxel_world.load_from_file(&filename) {
            Ok(_) => {
                // Spawn all blocks from the loaded map
                let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));

                for (position, block_type) in voxel_world.blocks.iter() {
                    let texture_path = match block_type {
                        BlockType::Grass => "textures/grass.webp",
                        BlockType::Dirt => "textures/dirt.webp",
                        BlockType::Stone => "textures/stone.webp",
                        BlockType::QuartzBlock => "textures/quartz_block.webp",
                        BlockType::GlassPane => "textures/glass_pane.webp",
                        BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                    };
                    let texture: Handle<Image> = asset_server.load(texture_path);
                    let material = materials.add(StandardMaterial {
                        base_color_texture: Some(texture),
                        ..default()
                    });

                    commands.spawn((
                        Mesh3d(cube_mesh.clone()),
                        MeshMaterial3d(material),
                        Transform::from_translation(Vec3::new(
                            position.x as f32,
                            position.y as f32,
                            position.z as f32,
                        )),
                        VoxelBlock {
                            position: *position,
                        },
                        // Physics colliders are managed by PhysicsManagerPlugin based on distance and mode
                    ));
                }

                reply!(
                    log,
                    "Map loaded from '{}' with {} blocks",
                    filename,
                    voxel_world.blocks.len()
                );
                info!(
                    "Map loaded from '{}' with {} blocks",
                    filename,
                    voxel_world.blocks.len()
                );
            }
            Err(e) => {
                reply!(log, "Failed to load map: {}", e);
                error!("Failed to load map from '{}': {}", filename, e);
            }
        }
    }
}

fn execute_pending_commands(
    mut pending_commands: ResMut<crate::script::script_types::PendingCommands>,
    mut print_console_line: EventWriter<PrintConsoleLine>,
    mut command_executed_events: EventWriter<CommandExecutedEvent>,
    mut blink_state: ResMut<BlinkState>,
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
                            blink_state.blinking = true;
                            print_console_line
                                .write(PrintConsoleLine::new("Blink started".to_string()));
                            info!("Blink started via script");
                        }
                        "stop" => {
                            blink_state.blinking = false;
                            print_console_line
                                .write(PrintConsoleLine::new("Blink stopped".to_string()));
                            info!("Blink stopped via script");
                        }
                        _ => {
                            print_console_line.write(PrintConsoleLine::new(
                                "Usage: blink [start|stop]".to_string(),
                            ));
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
                            print_console_line.write(PrintConsoleLine::new(status.to_string()));
                            info!("MQTT status requested via script");
                        }
                        "temp" => {
                            let temp_msg = if let Some(val) = temperature.value {
                                format!("Current temperature: {:.1}°C", val)
                            } else {
                                "No temperature data available".to_string()
                            };
                            print_console_line.write(PrintConsoleLine::new(temp_msg));
                        }
                        _ => {
                            print_console_line.write(PrintConsoleLine::new(
                                "Usage: mqtt [status|temp]".to_string(),
                            ));
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

                                // Create a temporary client for simulation
                                let mut mqtt_options = MqttOptions::new(
                                    "spawn-client",
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

                                print_console_line.write(PrintConsoleLine::new(format!(
                                    "Spawn command sent for device {}",
                                    device_id
                                )));
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

                                print_console_line.write(PrintConsoleLine::new(format!(
                                    "Spawn door command sent for device {}",
                                    device_id
                                )));
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
                                    _ => {
                                        print_console_line.write(PrintConsoleLine::new(format!(
                                            "Invalid block type: {}",
                                            block_type_str
                                        )));
                                        continue;
                                    }
                                };

                                voxel_world.set_block(IVec3::new(x, y, z), block_type);

                                // Spawn the block
                                let cube_mesh =
                                    meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
                                let texture_path = match block_type {
                                    BlockType::Grass => "textures/grass.webp",
                                    BlockType::Dirt => "textures/dirt.webp",
                                    BlockType::Stone => "textures/stone.webp",
                                    BlockType::QuartzBlock => "textures/quartz_block.webp",
                                    BlockType::GlassPane => "textures/glass_pane.webp",
                                    BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                                };
                                let texture: Handle<Image> = asset_server.load(texture_path);
                                let material = materials.add(StandardMaterial {
                                    base_color_texture: Some(texture),
                                    ..default()
                                });

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

                                print_console_line.write(PrintConsoleLine::new(format!(
                                    "Placed {} block at ({}, {}, {})",
                                    block_type_str, x, y, z
                                )));
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
                                if voxel_world.remove_block(&position).is_some() {
                                    // Remove the block entity
                                    for (entity, block) in query.iter() {
                                        if block.position == position {
                                            commands.entity(entity).despawn();
                                        }
                                    }
                                    print_console_line.write(PrintConsoleLine::new(format!(
                                        "Removed block at ({}, {}, {})",
                                        x, y, z
                                    )));
                                } else {
                                    print_console_line.write(PrintConsoleLine::new(format!(
                                        "No block found at ({}, {}, {})",
                                        x, y, z
                                    )));
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
                            _ => {
                                print_console_line.write(PrintConsoleLine::new(format!(
                                    "Invalid item type: {}",
                                    item_type_str
                                )));
                                continue;
                            }
                        };

                        inventory.add_items(item_type, quantity as u32);
                        print_console_line.write(PrintConsoleLine::new(format!(
                            "Added {} x {}",
                            quantity, item_type_str
                        )));
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
                                                _ => {
                                                    print_console_line.write(
                                                        PrintConsoleLine::new(format!(
                                                            "Invalid block type: {}",
                                                            block_type_str
                                                        )),
                                                    );
                                                    continue;
                                                }
                                            };

                                            // Debug: VoxelWorld before adding blocks
                                            info!(
                                                "VoxelWorld before wall command: {} blocks",
                                                voxel_world.blocks.len()
                                            );

                                            let texture_path = match block_type_enum {
                                                BlockType::Grass => "textures/grass.webp",
                                                BlockType::Dirt => "textures/dirt.webp",
                                                BlockType::Stone => "textures/stone.webp",
                                                BlockType::QuartzBlock => {
                                                    "textures/quartz_block.webp"
                                                }
                                                BlockType::GlassPane => "textures/glass_pane.webp",
                                                BlockType::CyanTerracotta => {
                                                    "textures/cyan_terracotta.webp"
                                                }
                                            };
                                            let texture: Handle<Image> =
                                                asset_server.load(texture_path);
                                            let material = materials.add(StandardMaterial {
                                                base_color_texture: Some(texture),
                                                ..default()
                                            });

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

                                            print_console_line.write(PrintConsoleLine::new(format!(
                                                "Created a wall of {} from ({}, {}, {}) to ({}, {}, {})",
                                                block_type_str, x1, y1, z1, x2, y2, z2
                                            )));
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

                                    print_console_line.write(PrintConsoleLine::new(format!(
                                        "Teleported to ({:.1}, {:.1}, {:.1})",
                                        x, y, z
                                    )));
                                    info!("Camera teleported to ({:.1}, {:.1}, {:.1})", x, y, z);
                                } else {
                                    print_console_line.write(PrintConsoleLine::new(
                                        "Error: Could not find camera to teleport".to_string(),
                                    ));
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

                                print_console_line.write(PrintConsoleLine::new(format!(
                                    "Set look angles to yaw: {:.1}°, pitch: {:.1}°",
                                    yaw, pitch
                                )));
                                info!(
                                    "Camera look angles set to yaw: {:.1}°, pitch: {:.1}°",
                                    yaw, pitch
                                );
                            } else {
                                print_console_line.write(PrintConsoleLine::new(
                                    "Error: Could not find camera to set look direction"
                                        .to_string(),
                                ));
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

                print_console_line.write(PrintConsoleLine::new(result_text.clone()));
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
                print_console_line.write(PrintConsoleLine::new(format!(
                    "Unknown command: {}",
                    command
                )));
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    // Initialize logging (TCP-only MCP server doesn't interfere with stdout)
    env_logger::init();

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

    // Add default plugins and initialize AssetServer
    app.add_plugins(DefaultPlugins);

    // Initialize fonts resource immediately after AssetServer is available
    // We need to do this in a way that ensures AssetServer exists
    app.world_mut()
        .resource_scope(|world, asset_server: Mut<AssetServer>| {
            let fonts = Fonts::new(&asset_server);
            world.insert_resource(fonts);
        });

    app.add_plugins(FontPlugin) // Keep FontPlugin for any additional font-related systems
        .add_plugins(LocalizationPlugin) // Load localization after fonts
        .add_plugins(avian3d::PhysicsPlugins::default()) // Add physics engine
        .add_plugins(PhysicsManagerPlugin) // Add physics optimization manager
        .add_plugins(CameraControllerPlugin)
        .add_plugins(PlayerControllerPlugin) // Add player controller for walking/flying modes
        .add_plugins(script::script_systems::ScriptPlugin) // Add script plugin early for PendingCommands resource
        .add_plugins(ConsolePlugin)
        .add_plugins(DevicePlugin)
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

    // Add MCP plugin only if --mcp flag is provided
    if args.mcp {
        info!("Starting in MCP server mode");
        app.add_plugins(mcp::McpPlugin);
    }

    app.init_state::<GameState>()
        .insert_resource(ConsoleConfiguration {
            keys: vec![KeyCode::F12],
            left_pos: 200.0,
            top_pos: 100.0,
            height: 400.0,
            width: 800.0,
            ..default()
        })
        .add_console_command::<BlinkCommand, _>(handle_blink_command)
        .add_console_command::<MqttCommand, _>(handle_mqtt_command)
        .add_console_command::<SpawnCommand, _>(handle_spawn_command)
        .add_console_command::<SpawnDoorCommand, _>(
            crate::console::console_systems::handle_spawn_door_command,
        )
        .add_console_command::<MoveCommand, _>(crate::console::console_systems::handle_move_command)
        .add_console_command::<PlaceBlockCommand, _>(handle_place_block_command)
        .add_console_command::<RemoveBlockCommand, _>(handle_remove_block_command)
        .add_console_command::<WallCommand, _>(handle_wall_command)
        .add_console_command::<SaveMapCommand, _>(handle_save_map_command)
        .add_console_command::<LoadMapCommand, _>(handle_load_map_command)
        .add_console_command::<GiveCommand, _>(handle_give_command)
        .add_console_command::<TestErrorCommand, _>(
            crate::console::console_systems::handle_test_error_command,
        )
        .add_console_command::<TeleportCommand, _>(handle_teleport_command)
        .add_console_command::<LookCommand, _>(handle_look_command)
        .add_console_command::<ListCommand, _>(crate::console::console_systems::handle_list_command)
        .insert_resource(BlinkState::default())
        // .add_systems(Update, draw_cursor) // Disabled: InteractionPlugin handles cursor drawing
        .add_systems(
            Update,
            (
                blink_publisher_system,
                rotate_logo_system,
                crate::devices::device_positioning::draw_drag_feedback,
            ),
        )
        .add_systems(Update, manage_camera_controller)
        .add_systems(Update, handle_console_t_key.after(ConsoleSet::Commands))
        .add_systems(Update, handle_mouse_capture.after(ConsoleSet::Commands))
        .add_systems(
            Update,
            crate::console::esc_handling::handle_esc_key.after(ConsoleSet::Commands),
        )
        .init_resource::<DiagnosticsVisible>()
        .add_systems(Startup, setup_diagnostics_ui)
        .add_systems(Update, execute_pending_commands)
        .add_systems(Update, handle_inventory_input)
        .add_systems(Update, handle_diagnostics_toggle)
        .add_systems(Update, update_diagnostics_content)
        .run();
}

fn handle_mqtt_command(
    mut log: ConsoleCommand<MqttCommand>,
    temperature: Res<TemperatureResource>,
) {
    if let Some(Ok(MqttCommand { action })) = log.take() {
        info!("Console command: mqtt {}", action);
        match action.as_str() {
            "status" => {
                let status = if temperature.value.is_some() {
                    "Connected to MQTT broker"
                } else {
                    "Connecting to MQTT broker..."
                };
                reply!(log, "{}", status);
                info!("MQTT status requested via console");
            }
            "temp" => {
                let temp_msg = if let Some(val) = temperature.value {
                    format!("Current temperature: {:.1}°C", val)
                } else {
                    "No temperature data available".to_string()
                };
                reply!(log, "{}", temp_msg);
            }
            _ => {
                reply!(log, "Usage: mqtt [status|temp]");
            }
        }
    }
}

fn handle_spawn_command(mut log: ConsoleCommand<SpawnCommand>, mqtt_config: Res<MqttConfig>) {
    if let Some(Ok(SpawnCommand { device_id, x, y, z })) = log.take() {
        info!("Console command: spawn {} {} {} {}", device_id, x, y, z);

        // Use the same MQTT announcement system as spawn_door for consistency
        let payload = json!({
            "device_id": device_id,
            "device_type": "lamp",
            "state": "online",
            "location": { "x": x, "y": y, "z": z }
        })
        .to_string();

        // Create a temporary client for simulation
        let mut mqtt_options =
            MqttOptions::new("spawn-client", &mqtt_config.host, mqtt_config.port);
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
            if let Ok(Event::Outgoing(Outgoing::Publish(_))) = notification {
                break;
            }
        }

        reply!(log, "Spawn command sent for device {}", device_id);
    }
}

fn blink_publisher_system(
    mut blink_state: ResMut<BlinkState>,
    device_query: Query<&DeviceEntity, With<BlinkCube>>,
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
fn manage_camera_controller(
    console_open: Res<ConsoleOpen>,
    mut camera_query: Query<&mut CameraController, With<Camera>>,
) {
    if let Ok(mut camera_controller) = camera_query.single_mut() {
        // Disable camera controller when console is open
        camera_controller.enabled = !console_open.open;
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
    time: Res<Time>,
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
                            },
                        }
                    )
                } else {
                    "Empty".to_string()
                }
            } else {
                "Empty".to_string()
            };

            let uptime = time.elapsed_secs();
            let minutes = (uptime / 60.0) as u32;
            let seconds = (uptime % 60.0) as u32;

            text.0 = format!(
                "IoTCraft Debug Information (Press F3 to toggle)\n\
                ------------------------------------------------------------------------------------------
                \n\
                - PLAYER INFORMATION\n\
                Position: X={:.2}  Y={:.2}  Z={:.2}\n\
                Rotation: Yaw={:.1}°  Pitch={:.1}°\n\
                Selected Slot: {} ({})\n\
                \n\
                - WORLD INFORMATION\n\
                Total Blocks: {}\n\
                IoT Devices: {}\n\
                Session Time: {}m {}s\n\
                \n\
                - SCRIPT COMMANDS\n\
                Teleport: tp {:.2} {:.2} {:.2}\n\
                Look Direction: look {:.1} {:.1}\n\
                \n\
                - CONTROLS\n\
                F3: Toggle this debug screen\n\
                T: Open console\n\
                1-9: Select inventory slot\n\
                Mouse Wheel: Scroll inventory slots",
                translation.x, translation.y, translation.z,
                yaw_degrees, pitch_degrees,
                selected_slot, selected_item,
                block_count,
                device_count,
                minutes, seconds,
                translation.x, translation.y, translation.z,
                yaw_degrees, pitch_degrees
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
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    game_state: Res<State<GameState>>,
) {
    // Only handle mouse recapture in InGame state (after it was released)
    if *game_state.get() == GameState::InGame && mouse_button_input.just_pressed(MouseButton::Left)
    {
        for mut window in &mut windows {
            if !window.focused {
                continue;
            }

            // Only capture if cursor is currently not captured
            if window.cursor_options.grab_mode == CursorGrabMode::None {
                window.cursor_options.grab_mode = CursorGrabMode::Locked;
                window.cursor_options.visible = false;
            }
        }
    }
}

// System to handle 't' key to open console (only when closed)
fn handle_console_t_key(
    mut console_open: ResMut<ConsoleOpen>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
) {
    // Only open console with 't' when it's currently closed and in game
    if keyboard_input.just_pressed(KeyCode::KeyT)
        && !console_open.open
        && *current_state.get() == GameState::InGame
    {
        console_open.open = true;
        game_state.set(GameState::ConsoleOpen);
    }
}

// System to handle inventory slot selection with number keys and mouse wheel
fn handle_inventory_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut inventory: ResMut<PlayerInventory>,
    accumulated_mouse_scroll: Res<bevy::input::mouse::AccumulatedMouseScroll>,
    console_open: Res<ConsoleOpen>,
) {
    // Don't handle input when console is open
    if console_open.open {
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
