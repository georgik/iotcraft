// Web-compatible console system for IoTCraft
use bevy::prelude::*;
use log::{info, warn};

// Re-export from console_types_web
pub use crate::console::console_types_web::*;

use crate::environment::*;
use crate::devices::*;
use crate::inventory::*;
use crate::mqtt::*;
use crate::camera_controllers::*;

pub struct WebConsolePlugin;

impl Plugin for WebConsolePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConsoleState>()
            .add_systems(Update, (
                update_blink_system,
                handle_escape_key,
            ));
    }
}

#[derive(Resource, Default)]
pub struct ConsoleState {
    pub open: bool,
    pub current_input: String,
}

/// Execute a parsed console command
pub fn execute_web_console_command(
    command: ConsoleCommandType,
    blink_state: &mut ResMut<BlinkState>,
    temperature: &Res<TemperatureResource>,
    voxel_world: &mut ResMut<VoxelWorld>,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
    block_query: &Query<(Entity, &VoxelBlock)>,
    device_query: &Query<(&DeviceEntity, &Transform), Without<Camera>>,
    inventory: &mut ResMut<PlayerInventory>,
    camera_query: &mut Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    match command {
        ConsoleCommandType::Blink(cmd) => {
            info!("Executing blink command: {}", cmd.action);
            match cmd.action.as_str() {
                "start" => {
                    blink_state.blinking = true;
                    info!("Blink started via web console");
                }
                "stop" => {
                    blink_state.blinking = false;
                    info!("Blink stopped via web console");
                }
                _ => {
                    warn!("Invalid blink action: {}", cmd.action);
                }
            }
        }
        ConsoleCommandType::Mqtt(cmd) => {
            info!("Executing MQTT command: {}", cmd.action);
            match cmd.action.as_str() {
                "status" => {
                    let status = if temperature.value.is_some() {
                        "Connected to MQTT broker (WebSocket)"
                    } else {
                        "Connecting to MQTT broker..."
                    };
                    info!("MQTT status: {}", status);
                }
                "temp" => {
                    let temp_msg = if let Some(val) = temperature.value {
                        format!("Current temperature: {:.1}°C", val)
                    } else {
                        "No temperature data available".to_string()
                    };
                    info!("Temperature: {}", temp_msg);
                }
                _ => {
                    warn!("Invalid MQTT action: {}", cmd.action);
                }
            }
        }
        ConsoleCommandType::PlaceBlock(cmd) => {
            info!("Executing place block command: {} at ({}, {}, {})", cmd.block_type, cmd.x, cmd.y, cmd.z);

            let block_type = match cmd.block_type.as_str() {
                "grass" => BlockType::Grass,
                "dirt" => BlockType::Dirt,
                "stone" => BlockType::Stone,
                "quartz_block" => BlockType::QuartzBlock,
                "glass_pane" => BlockType::GlassPane,
                "cyan_terracotta" => BlockType::CyanTerracotta,
                "water" => BlockType::Water,
                _ => {
                    warn!("Invalid block type: {}", cmd.block_type);
                    return;
                }
            };

            voxel_world.set_block(IVec3::new(cmd.x, cmd.y, cmd.z), block_type);

            // Spawn the block visually
            let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
            let material = create_block_material(block_type, asset_server, materials);

            commands.spawn((
                Mesh3d(cube_mesh),
                MeshMaterial3d(material),
                Transform::from_translation(Vec3::new(cmd.x as f32, cmd.y as f32, cmd.z as f32)),
                VoxelBlock {
                    position: IVec3::new(cmd.x, cmd.y, cmd.z),
                },
            ));

            info!("Placed {} block at ({}, {}, {})", cmd.block_type, cmd.x, cmd.y, cmd.z);
        }
        ConsoleCommandType::RemoveBlock(cmd) => {
            info!("Executing remove block command at ({}, {}, {})", cmd.x, cmd.y, cmd.z);

            let position = IVec3::new(cmd.x, cmd.y, cmd.z);
            if voxel_world.remove_block(&position).is_some() {
                // Remove the block entity
                for (entity, block) in block_query.iter() {
                    if block.position == position {
                        commands.entity(entity).despawn();
                    }
                }
                info!("Removed block at ({}, {}, {})", cmd.x, cmd.y, cmd.z);
            } else {
                warn!("No block found at ({}, {}, {})", cmd.x, cmd.y, cmd.z);
            }
        }
        ConsoleCommandType::Wall(cmd) => {
            info!("Executing wall command: {} from ({}, {}, {}) to ({}, {}, {})",
                cmd.block_type, cmd.x1, cmd.y1, cmd.z1, cmd.x2, cmd.y2, cmd.z2);

            let block_type = match cmd.block_type.as_str() {
                "grass" => BlockType::Grass,
                "dirt" => BlockType::Dirt,
                "stone" => BlockType::Stone,
                "quartz_block" => BlockType::QuartzBlock,
                "glass_pane" => BlockType::GlassPane,
                "cyan_terracotta" => BlockType::CyanTerracotta,
                "water" => BlockType::Water,
                _ => {
                    warn!("Invalid block type: {}", cmd.block_type);
                    return;
                }
            };

            let material = create_block_material(block_type, asset_server, materials);
            let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));

            let mut blocks_added = 0;
            for x in cmd.x1..=cmd.x2 {
                for y in cmd.y1..=cmd.y2 {
                    for z in cmd.z1..=cmd.z2 {
                        voxel_world.set_block(IVec3::new(x, y, z), block_type);
                        blocks_added += 1;

                        commands.spawn((
                            Mesh3d(cube_mesh.clone()),
                            MeshMaterial3d(material.clone()),
                            Transform::from_translation(Vec3::new(x as f32, y as f32, z as f32)),
                            VoxelBlock {
                                position: IVec3::new(x, y, z),
                            },
                        ));
                    }
                }
            }

            info!("Created wall of {} blocks from ({}, {}, {}) to ({}, {}, {})",
                blocks_added, cmd.x1, cmd.y1, cmd.z1, cmd.x2, cmd.y2, cmd.z2);
        }
        ConsoleCommandType::Teleport(cmd) => {
            info!("Executing teleport command to ({}, {}, {})", cmd.x, cmd.y, cmd.z);

            if let Ok((mut transform, _)) = camera_query.single_mut() {
                transform.translation = Vec3::new(cmd.x, cmd.y, cmd.z);
                info!("Teleported to ({:.1}, {:.1}, {:.1})", cmd.x, cmd.y, cmd.z);
            } else {
                warn!("Could not find camera to teleport");
            }
        }
        ConsoleCommandType::Look(cmd) => {
            info!("Executing look command: yaw={}, pitch={}", cmd.yaw, cmd.pitch);

            if let Ok((mut transform, mut camera_controller)) = camera_query.single_mut() {
                let yaw_rad = cmd.yaw.to_radians();
                let pitch_rad = cmd.pitch.to_radians();

                camera_controller.yaw = yaw_rad;
                camera_controller.pitch = pitch_rad.clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);

                transform.rotation = Quat::from_euler(
                    bevy::math::EulerRot::ZYX,
                    0.0,
                    camera_controller.yaw,
                    camera_controller.pitch,
                );

                info!("Set look angles to yaw: {:.1}°, pitch: {:.1}°", cmd.yaw, cmd.pitch);
            } else {
                warn!("Could not find camera to set look direction");
            }
        }
        ConsoleCommandType::Give(cmd) => {
            info!("Executing give command: {} x{}", cmd.item_type, cmd.count);

            let item_type = match cmd.item_type.as_str() {
                "grass" => ItemType::Block(BlockType::Grass),
                "dirt" => ItemType::Block(BlockType::Dirt),
                "stone" => ItemType::Block(BlockType::Stone),
                "quartz_block" => ItemType::Block(BlockType::QuartzBlock),
                "glass_pane" => ItemType::Block(BlockType::GlassPane),
                "cyan_terracotta" => ItemType::Block(BlockType::CyanTerracotta),
                "water" => ItemType::Block(BlockType::Water),
                _ => {
                    warn!("Invalid item type: {}", cmd.item_type);
                    return;
                }
            };

            inventory.add_items(item_type, cmd.count);
            info!("Added {} x {}", cmd.count, cmd.item_type);
        }
        ConsoleCommandType::List(_) => {
            info!("Executing list command");

            let device_count = device_query.iter().count();
            if device_count == 0 {
                info!("No connected devices found.");
            } else {
                info!("Connected devices ({}):", device_count);
                for (device, transform) in device_query.iter() {
                    let pos = transform.translation;
                    info!("- ID: {} | Type: {} | Position: ({:.2}, {:.2}, {:.2})",
                        device.device_id,
                        device.device_type,
                        pos.x,
                        pos.y,
                        pos.z
                    );
                }
            }
        }
        ConsoleCommandType::SaveMap(cmd) => {
            info!("Save map command: {} (web - using localStorage)", cmd.filename);
            // Implementation would save to localStorage
            save_map_to_local_storage(&cmd.filename, voxel_world);
        }
        ConsoleCommandType::LoadMap(cmd) => {
            info!("Load map command: {} (web - using localStorage)", cmd.filename);
            // Implementation would load from localStorage
            load_map_from_local_storage(&cmd.filename, voxel_world, commands, meshes, materials, asset_server, block_query);
        }
        ConsoleCommandType::Spawn(cmd) => {
            info!("Spawn command: {} at ({}, {}, {}) (web mode - acknowledgement only)",
                cmd.device_id, cmd.x, cmd.y, cmd.z);
            // In web version, just acknowledge - no direct MQTT
        }
    }
}

/// Helper function to create block materials
fn create_block_material(
    block_type: BlockType,
    asset_server: &AssetServer,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Handle<StandardMaterial> {
    match block_type {
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
                BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                _ => "textures/stone.webp", // fallback
            };
            let texture: Handle<Image> = asset_server.load(texture_path);
            materials.add(StandardMaterial {
                base_color_texture: Some(texture),
                ..default()
            })
        }
    }
}

/// Web-specific save functionality using localStorage
#[cfg(target_arch = "wasm32")]
fn save_map_to_local_storage(filename: &str, voxel_world: &VoxelWorld) {
    if let Ok(world_data) = serde_json::to_string(&voxel_world.blocks) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(local_storage)) = window.local_storage() {
                let storage_key = format!("iotcraft_map_{}", filename);
                match local_storage.set_item(&storage_key, &world_data) {
                    Ok(_) => {
                        info!("Map '{}' saved to browser storage with {} blocks", filename, voxel_world.blocks.len());
                    }
                    Err(_) => {
                        warn!("Failed to save map '{}' to localStorage", filename);
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn save_map_to_local_storage(_filename: &str, _voxel_world: &VoxelWorld) {
    // No-op for non-web targets
}

/// Web-specific load functionality using localStorage
#[cfg(target_arch = "wasm32")]
fn load_map_from_local_storage(
    filename: &str,
    voxel_world: &mut VoxelWorld,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &AssetServer,
    block_query: &Query<(Entity, &VoxelBlock)>,
) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(local_storage)) = window.local_storage() {
            let storage_key = format!("iotcraft_map_{}", filename);
            match local_storage.get_item(&storage_key) {
                Ok(Some(world_data)) => {
                    match serde_json::from_str(&world_data) {
                        Ok(loaded_blocks) => {
                            // Clear existing blocks
                            for (entity, _) in block_query.iter() {
                                commands.entity(entity).despawn();
                            }

                            voxel_world.blocks = loaded_blocks;

                            // Spawn all blocks
                            let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
                            for (position, block_type) in voxel_world.blocks.iter() {
                                let material = create_block_material(*block_type, asset_server, materials);

                                commands.spawn((
                                    Mesh3d(cube_mesh.clone()),
                                    MeshMaterial3d(material),
                                    Transform::from_translation(Vec3::new(
                                        position.x as f32,
                                        position.y as f32,
                                        position.z as f32,
                                    )),
                                    VoxelBlock { position: *position },
                                ));
                            }

                            info!("Map '{}' loaded from browser storage with {} blocks", filename, voxel_world.blocks.len());
                        }
                        Err(_) => {
                            warn!("Failed to parse stored map data for '{}'", filename);
                        }
                    }
                }
                Ok(None) => {
                    warn!("Map '{}' not found in browser storage", filename);
                }
                Err(_) => {
                    warn!("Failed to access browser storage");
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn load_map_from_local_storage(
    _filename: &str,
    _voxel_world: &mut VoxelWorld,
    _commands: &mut Commands,
    _meshes: &mut ResMut<Assets<Mesh>>,
    _materials: &mut ResMut<Assets<StandardMaterial>>,
    _asset_server: &AssetServer,
    _block_query: &Query<(Entity, &VoxelBlock)>,
) {
    // No-op for non-web targets
}

/// Update blink system
fn update_blink_system(
    time: Res<Time>,
    mut blink_state: ResMut<BlinkState>,
) {
    if blink_state.blinking {
        blink_state.timer.tick(time.delta());
        if blink_state.timer.finished() {
            blink_state.light_state = !blink_state.light_state;
        }
    }
}

/// Handle escape key to close console
fn handle_escape_key(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_state: ResMut<NextState<GameState>>,
    current_state: Res<State<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) && *current_state.get() == GameState::ConsoleOpen {
        game_state.set(GameState::InGame);
        info!("Console closed (web version)");
    }
}

/// Parse a console command string into a ConsoleCommandType
pub fn parse_console_command(command: &str) -> Option<ConsoleCommandType> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    match parts[0] {
        "blink" => {
            if parts.len() == 2 {
                Some(ConsoleCommandType::Blink(BlinkCommand {
                    action: parts[1].to_string(),
                }))
            } else {
                None
            }
        }
        "mqtt" => {
            if parts.len() == 2 {
                Some(ConsoleCommandType::Mqtt(MqttCommand {
                    action: parts[1].to_string(),
                }))
            } else {
                None
            }
        }
        "place" => {
            if parts.len() == 5 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[2].parse::<i32>(),
                    parts[3].parse::<i32>(),
                    parts[4].parse::<i32>(),
                ) {
                    Some(ConsoleCommandType::PlaceBlock(PlaceBlockCommand {
                        block_type: parts[1].to_string(),
                        x,
                        y,
                        z,
                    }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "remove" => {
            if parts.len() == 4 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<i32>(),
                    parts[2].parse::<i32>(),
                    parts[3].parse::<i32>(),
                ) {
                    Some(ConsoleCommandType::RemoveBlock(RemoveBlockCommand { x, y, z }))
                } else {
                    None
                }
            } else {
                None
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
                    Some(ConsoleCommandType::Wall(WallCommand {
                        block_type: parts[1].to_string(),
                        x1,
                        y1,
                        z1,
                        x2,
                        y2,
                        z2,
                    }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "tp" | "teleport" => {
            if parts.len() == 4 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                ) {
                    Some(ConsoleCommandType::Teleport(TeleportCommand { x, y, z }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "look" => {
            if parts.len() == 3 {
                if let (Ok(yaw), Ok(pitch)) = (
                    parts[1].parse::<f32>(),
                    parts[2].parse::<f32>(),
                ) {
                    Some(ConsoleCommandType::Look(LookCommand { yaw, pitch }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "give" => {
            if parts.len() == 3 {
                if let Ok(count) = parts[2].parse::<u32>() {
                    Some(ConsoleCommandType::Give(GiveCommand {
                        item_type: parts[1].to_string(),
                        count,
                    }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        "list" => Some(ConsoleCommandType::List(ListCommand {})),
        "save_map" => {
            if parts.len() == 2 {
                Some(ConsoleCommandType::SaveMap(SaveMapCommand {
                    filename: parts[1].to_string(),
                }))
            } else {
                None
            }
        }
        "load_map" => {
            if parts.len() == 2 {
                Some(ConsoleCommandType::LoadMap(LoadMapCommand {
                    filename: parts[1].to_string(),
                }))
            } else {
                None
            }
        }
        "spawn" => {
            if parts.len() == 5 {
                if let (Ok(x), Ok(y), Ok(z)) = (
                    parts[2].parse::<f32>(),
                    parts[3].parse::<f32>(),
                    parts[4].parse::<f32>(),
                ) {
                    Some(ConsoleCommandType::Spawn(SpawnCommand {
                        device_id: parts[1].to_string(),
                        x,
                        y,
                        z,
                    }))
                } else {
                    None
                }
            } else {
                None
            }
        }
        _ => None,
    }
}
