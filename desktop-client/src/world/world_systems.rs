use bevy::prelude::*;
use log::{error, info, warn};
use std::fs;
use std::path::Path;

use super::world_types::*;
use crate::camera_controllers::CameraController;
use crate::environment::VoxelWorld;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<LoadWorldEvent>()
            .add_event::<SaveWorldEvent>()
            .add_event::<CreateWorldEvent>()
            .init_resource::<DiscoveredWorlds>()
            .add_systems(Startup, discover_worlds)
            .add_systems(
                Update,
                (
                    handle_load_world_events,
                    handle_create_world_events,
                    handle_save_world_events,
                ),
            );
    }
}

/// Discovers all saved worlds on startup
fn discover_worlds(mut discovered_worlds: ResMut<DiscoveredWorlds>) {
    let worlds_dir = get_worlds_directory();

    info!("Discovering worlds in directory: {:?}", worlds_dir);

    // Create worlds directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&worlds_dir) {
        error!("Failed to create worlds directory: {}", e);
        return;
    }

    let mut worlds = Vec::new();

    // Read directory contents
    match fs::read_dir(&worlds_dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(world_info) = load_world_info(&path) {
                            info!("Discovered world: {}", world_info.name);
                            worlds.push(world_info);
                        }
                    }
                }
            }
        }
        Err(e) => error!("Failed to read worlds directory: {}", e),
    }

    // Sort worlds by last played (most recent first)
    worlds.sort_by(|a, b| b.metadata.last_played.cmp(&a.metadata.last_played));

    discovered_worlds.worlds = worlds;
    info!("Discovered {} worlds", discovered_worlds.worlds.len());
}

/// Gets the worlds directory path
fn get_worlds_directory() -> std::path::PathBuf {
    let mut path = dirs::document_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    path.push("IOTCraft");
    path.push("worlds");
    path
}

/// Loads world info from a directory
fn load_world_info(world_path: &Path) -> Option<WorldInfo> {
    let metadata_path = world_path.join("metadata.json");

    if !metadata_path.exists() {
        return None;
    }

    match fs::read_to_string(&metadata_path) {
        Ok(content) => match serde_json::from_str::<WorldMetadata>(&content) {
            Ok(metadata) => Some(WorldInfo {
                name: metadata.name.clone(),
                path: world_path.to_path_buf(),
                metadata,
            }),
            Err(e) => {
                warn!("Failed to parse metadata for world {:?}: {}", world_path, e);
                None
            }
        },
        Err(e) => {
            warn!("Failed to read metadata for world {:?}: {}", world_path, e);
            None
        }
    }
}

/// Handles world loading events
fn handle_load_world_events(
    mut load_events: EventReader<LoadWorldEvent>,
    discovered_worlds: Res<DiscoveredWorlds>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut commands: Commands,
    camera_query: Query<Entity, With<CameraController>>,
    mut inventory: ResMut<crate::inventory::PlayerInventory>,
    existing_blocks_query: Query<Entity, With<crate::environment::VoxelBlock>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut error_resource: ResMut<crate::ui::error_indicator::ErrorResource>,
    time: Res<Time>,
) {
    for event in load_events.read() {
        info!("Loading world: {}", event.world_name);

        // Find the world in discovered worlds
        if let Some(world_info) = discovered_worlds
            .worlds
            .iter()
            .find(|w| w.name == event.world_name)
        {
            let world_data_path = world_info.path.join("world.json");

            if world_data_path.exists() {
                info!("Loading world data from: {:?}", world_data_path);
                match fs::read_to_string(&world_data_path) {
                    Ok(content) => {
                        match serde_json::from_str::<WorldSaveData>(&content) {
                            Ok(save_data) => {
                                info!(
                                    "Loaded world save data with {} blocks",
                                    save_data.blocks.len()
                                );
                                for (i, block_data) in save_data.blocks.iter().take(5).enumerate() {
                                    info!(
                                        "  Block {}: {:?} at ({}, {}, {})",
                                        i,
                                        block_data.block_type,
                                        block_data.x,
                                        block_data.y,
                                        block_data.z
                                    );
                                }

                                // Clear existing voxel blocks from the scene
                                let cleared_entities = existing_blocks_query.iter().count();
                                for entity in existing_blocks_query.iter() {
                                    commands.entity(entity).despawn();
                                }
                                info!(
                                    "Cleared {} existing block entities from scene",
                                    cleared_entities
                                );

                                // Clear and convert blocks into voxel world
                                voxel_world.blocks.clear();
                                for block_data in save_data.blocks {
                                    voxel_world.blocks.insert(
                                        IVec3::new(block_data.x, block_data.y, block_data.z),
                                        block_data.block_type,
                                    );
                                }
                                info!("Loaded {} blocks into VoxelWorld", voxel_world.blocks.len());

                                // Spawn visual blocks for all loaded blocks
                                let mut spawned_blocks = 0;
                                for (pos, block_type) in voxel_world.blocks.iter() {
                                    let cube_mesh = meshes.add(Cuboid::new(
                                        crate::environment::CUBE_SIZE,
                                        crate::environment::CUBE_SIZE,
                                        crate::environment::CUBE_SIZE,
                                    ));
                                    let texture_path = match block_type {
                                        crate::environment::BlockType::Grass => {
                                            "textures/grass.webp"
                                        }
                                        crate::environment::BlockType::Dirt => "textures/dirt.webp",
                                        crate::environment::BlockType::Stone => {
                                            "textures/stone.webp"
                                        }
                                        crate::environment::BlockType::QuartzBlock => {
                                            "textures/quartz_block.webp"
                                        }
                                        crate::environment::BlockType::GlassPane => {
                                            "textures/glass_pane.webp"
                                        }
                                        crate::environment::BlockType::CyanTerracotta => {
                                            "textures/cyan_terracotta.webp"
                                        }
                                    };
                                    let texture: Handle<Image> = asset_server.load(texture_path);
                                    let material = materials.add(StandardMaterial {
                                        base_color_texture: Some(texture),
                                        ..default()
                                    });

                                    commands.spawn((
                                        Mesh3d(cube_mesh),
                                        MeshMaterial3d(material),
                                        Transform::from_translation(pos.as_vec3()),
                                        crate::environment::VoxelBlock { position: *pos },
                                    ));
                                    spawned_blocks += 1;
                                }
                                info!("Spawned {} visual block entities", spawned_blocks);

                                // Load inventory data and force change detection
                                *inventory = save_data.inventory;
                                inventory.ensure_proper_size();
                                inventory.set_changed();

                                // Update current world resource
                                commands.insert_resource(CurrentWorld {
                                    name: world_info.name.clone(),
                                    path: world_info.path.clone(),
                                    metadata: world_info.metadata.clone(),
                                });

                                // Set player position if camera exists
                                if let Ok(camera_entity) = camera_query.single() {
                                    commands.entity(camera_entity).insert(Transform {
                                        translation: save_data.player_position,
                                        rotation: save_data.player_rotation,
                                        ..default()
                                    });
                                }

                                info!("Successfully loaded world: {}", event.world_name);
                            }
                            Err(e) => {
                                let error_message = format!(
                                    "Failed to parse world data for {}: {}",
                                    event.world_name, e
                                );
                                error!("{}", error_message);

                                // Trigger error indicator
                                error_resource.indicator_on = true;
                                error_resource.last_error_time = time.elapsed_secs();
                                error_resource.messages.push(error_message);
                            }
                        }
                    }
                    Err(e) => {
                        let error_message =
                            format!("Failed to read world data for {}: {}", event.world_name, e);
                        error!("{}", error_message);

                        // Trigger error indicator
                        error_resource.indicator_on = true;
                        error_resource.last_error_time = time.elapsed_secs();
                        error_resource.messages.push(error_message);
                    }
                }
            } else {
                warn!("World data file not found for: {}", event.world_name);
                // Create empty world if no data exists
                create_empty_world(
                    &event.world_name,
                    &world_info.metadata.description,
                    &mut voxel_world,
                    &mut commands,
                    &world_info.path,
                );
            }
        } else {
            error!("World not found: {}", event.world_name);
        }
    }
}

/// Handles world saving events
fn handle_save_world_events(
    mut save_events: EventReader<SaveWorldEvent>,
    current_world: Option<Res<CurrentWorld>>,
    voxel_world: Res<VoxelWorld>,
    camera_query: Query<&Transform, With<CameraController>>,
    inventory: Res<crate::inventory::PlayerInventory>,
) {
    for event in save_events.read() {
        info!("Saving world: {}", event.world_name);

        if let Some(current_world) = &current_world {
            let world_data_path = current_world.path.join("world.json");

            // Get current player position
            let (player_position, player_rotation) = if let Ok(transform) = camera_query.single() {
                (transform.translation, transform.rotation)
            } else {
                (Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY)
            };

            // Convert blocks from HashMap to Vec for serialization
            info!(
                "VoxelWorld contains {} blocks before saving",
                voxel_world.blocks.len()
            );
            for (pos, block_type) in voxel_world.blocks.iter().take(5) {
                info!("  Block at {:?}: {:?}", pos, block_type);
            }

            let blocks: Vec<VoxelBlockData> = voxel_world
                .blocks
                .iter()
                .map(|(pos, block_type)| VoxelBlockData {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                    block_type: *block_type,
                })
                .collect();

            info!("Converted {} blocks for serialization", blocks.len());

            // Update metadata with current timestamp
            let mut updated_metadata = current_world.metadata.clone();
            updated_metadata.last_played = chrono::Utc::now().to_rfc3339();

            // Create save data
            let save_data = WorldSaveData {
                metadata: updated_metadata,
                blocks,
                player_position,
                player_rotation,
                inventory: (*inventory).clone(),
            };

            // Serialize and save
            info!("Saving world data to: {:?}", world_data_path);
            match serde_json::to_string_pretty(&save_data) {
                Ok(json) => {
                    if let Err(e) = fs::write(&world_data_path, json) {
                        error!("Failed to write world data for {}: {}", event.world_name, e);
                    } else {
                        info!("Successfully saved world: {}", event.world_name);
                    }
                }
                Err(e) => error!(
                    "Failed to serialize world data for {}: {}",
                    event.world_name, e
                ),
            }
        } else {
            warn!("No current world to save");
        }
    }
}

/// Handles create world events
fn handle_create_world_events(
    mut create_events: EventReader<CreateWorldEvent>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut commands: Commands,
    mut discovered_worlds: ResMut<DiscoveredWorlds>,
) {
    for event in create_events.read() {
        info!("Creating new world: {}", event.world_name);

        let worlds_dir = get_worlds_directory();
        let world_path = worlds_dir.join(&event.world_name);

        // Create world directory
        if let Err(e) = fs::create_dir_all(&world_path) {
            error!(
                "Failed to create world directory for {}: {}",
                event.world_name, e
            );
            continue;
        }

        // Create metadata
        let metadata = WorldMetadata {
            name: event.world_name.clone(),
            description: event.description.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_played: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
        };

        // Save metadata
        let metadata_path = world_path.join("metadata.json");
        match serde_json::to_string_pretty(&metadata) {
            Ok(json) => {
                if let Err(e) = fs::write(&metadata_path, json) {
                    error!("Failed to write metadata for {}: {}", event.world_name, e);
                    continue;
                }
            }
            Err(e) => {
                error!(
                    "Failed to serialize metadata for {}: {}",
                    event.world_name, e
                );
                continue;
            }
        }

        // Create empty world
        create_empty_world(
            &event.world_name,
            &event.description,
            &mut voxel_world,
            &mut commands,
            &world_path,
        );

        // Add to discovered worlds
        discovered_worlds.worlds.push(WorldInfo {
            name: event.world_name.clone(),
            path: world_path,
            metadata,
        });

        info!("Successfully created new world: {}", event.world_name);
    }
}

/// Creates an empty world
fn create_empty_world(
    world_name: &str,
    description: &str,
    voxel_world: &mut VoxelWorld,
    commands: &mut Commands,
    world_path: &Path,
) {
    // Clear existing blocks
    voxel_world.blocks.clear();

    // Set current world
    commands.insert_resource(CurrentWorld {
        name: world_name.to_string(),
        path: world_path.to_path_buf(),
        metadata: WorldMetadata {
            name: world_name.to_string(),
            description: description.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_played: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
        },
    });
}
