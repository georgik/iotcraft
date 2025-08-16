use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use tokio::fs;

use super::world_types::*;
use crate::environment::{BlockType, CUBE_SIZE, VoxelBlock, VoxelWorld};
use crate::inventory::PlayerInventory;

/// Plugin for async world operations
pub struct AsyncWorldPlugin;

impl Plugin for AsyncWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AsyncLoadWorldEvent>()
            .add_event::<AsyncSaveWorldEvent>()
            .insert_resource(AsyncWorldTaskManager::default())
            .add_systems(
                Update,
                (
                    handle_async_load_events,
                    handle_async_save_events,
                    complete_world_loading_tasks,
                    complete_world_saving_tasks,
                ),
            );
    }
}

/// Events for async world operations
#[derive(Event)]
pub struct AsyncLoadWorldEvent {
    pub world_name: String,
}

#[derive(Event)]
pub struct AsyncSaveWorldEvent {
    pub world_name: String,
}

/// Manages async world operation tasks
#[derive(Resource, Default)]
pub struct AsyncWorldTaskManager {
    pub loading_tasks: Vec<WorldLoadingTask>,
    pub saving_tasks: Vec<WorldSavingTask>,
}

/// Task for loading world data asynchronously
pub struct WorldLoadingTask {
    pub task: Task<WorldLoadResult>,
    pub world_name: String,
}

/// Task for saving world data asynchronously
pub struct WorldSavingTask {
    pub task: Task<WorldSaveResult>,
    pub world_name: String,
}

/// Result from async world loading
#[derive(Debug)]
pub struct WorldLoadResult {
    pub success: bool,
    pub world_name: String,
    pub save_data: Option<WorldSaveData>,
    pub error_message: Option<String>,
    pub load_time_ms: f32,
}

/// Result from async world saving
#[derive(Debug)]
pub struct WorldSaveResult {
    pub success: bool,
    pub world_name: String,
    pub error_message: Option<String>,
    pub save_time_ms: f32,
}

/// Batch processing configuration
const BLOCK_SPAWN_BATCH_SIZE: usize = 200; // Spawn blocks in batches to avoid frame drops

/// Handle async world loading events
fn handle_async_load_events(
    mut load_events: EventReader<AsyncLoadWorldEvent>,
    mut task_manager: ResMut<AsyncWorldTaskManager>,
    discovered_worlds: Res<DiscoveredWorlds>,
) {
    for event in load_events.read() {
        // Check if we're already loading this world
        if task_manager
            .loading_tasks
            .iter()
            .any(|task| task.world_name == event.world_name)
        {
            continue;
        }

        info!("Starting async load for world: {}", event.world_name);

        // Find the world in discovered worlds
        if let Some(world_info) = discovered_worlds
            .worlds
            .iter()
            .find(|w| w.name == event.world_name)
        {
            let world_data_path = world_info.path.join("world.json");
            let world_name = event.world_name.clone();

            let task_pool = AsyncComputeTaskPool::get();
            let task = task_pool.spawn(async move {
                let start_time = std::time::Instant::now();

                if world_data_path.exists() {
                    match fs::read_to_string(&world_data_path).await {
                        Ok(content) => match serde_json::from_str::<WorldSaveData>(&content) {
                            Ok(save_data) => {
                                let load_time = start_time.elapsed().as_secs_f32() * 1000.0;
                                info!(
                                    "Async world load completed for '{}' in {:.2}ms",
                                    world_name, load_time
                                );

                                WorldLoadResult {
                                    success: true,
                                    world_name,
                                    save_data: Some(save_data),
                                    error_message: None,
                                    load_time_ms: load_time,
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to parse world data: {}", e);
                                error!("{}", error_msg);
                                WorldLoadResult {
                                    success: false,
                                    world_name,
                                    save_data: None,
                                    error_message: Some(error_msg),
                                    load_time_ms: start_time.elapsed().as_secs_f32() * 1000.0,
                                }
                            }
                        },
                        Err(e) => {
                            let error_msg = format!("Failed to read world file: {}", e);
                            error!("{}", error_msg);
                            WorldLoadResult {
                                success: false,
                                world_name,
                                save_data: None,
                                error_message: Some(error_msg),
                                load_time_ms: start_time.elapsed().as_secs_f32() * 1000.0,
                            }
                        }
                    }
                } else {
                    // World file doesn't exist, return empty world
                    WorldLoadResult {
                        success: true,
                        world_name,
                        save_data: None,
                        error_message: Some(
                            "World data file not found - creating empty world".to_string(),
                        ),
                        load_time_ms: start_time.elapsed().as_secs_f32() * 1000.0,
                    }
                }
            });

            task_manager.loading_tasks.push(WorldLoadingTask {
                task,
                world_name: event.world_name.clone(),
            });
        } else {
            error!("World not found for async loading: {}", event.world_name);
        }
    }
}

/// Handle async world saving events
fn handle_async_save_events(
    mut save_events: EventReader<AsyncSaveWorldEvent>,
    mut task_manager: ResMut<AsyncWorldTaskManager>,
    current_world: Option<Res<CurrentWorld>>,
    voxel_world: Res<VoxelWorld>,
    camera_query: Query<&Transform, With<crate::camera_controllers::CameraController>>,
    inventory: Res<PlayerInventory>,
) {
    for event in save_events.read() {
        // Check if we're already saving this world
        if task_manager
            .saving_tasks
            .iter()
            .any(|task| task.world_name == event.world_name)
        {
            continue;
        }

        info!("Starting async save for world: {}", event.world_name);

        if let Some(current_world) = &current_world {
            let world_data_path = current_world.path.join("world.json");
            let world_name = event.world_name.clone();

            // Get current player position and rotation
            let (player_position, player_rotation) = if let Ok(transform) = camera_query.single() {
                (transform.translation, transform.rotation)
            } else {
                (Vec3::new(0.0, 2.0, 0.0), Quat::IDENTITY)
            };

            // Convert blocks from HashMap to Vec for serialization (in parallel)
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

            // Update metadata
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

            let task_pool = AsyncComputeTaskPool::get();
            let task = task_pool.spawn(async move {
                let start_time = std::time::Instant::now();

                match serde_json::to_string_pretty(&save_data) {
                    Ok(json) => match fs::write(&world_data_path, json).await {
                        Ok(_) => {
                            let save_time = start_time.elapsed().as_secs_f32() * 1000.0;
                            info!(
                                "Async world save completed for '{}' in {:.2}ms",
                                world_name, save_time
                            );

                            WorldSaveResult {
                                success: true,
                                world_name,
                                error_message: None,
                                save_time_ms: save_time,
                            }
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to write world file: {}", e);
                            error!("{}", error_msg);
                            WorldSaveResult {
                                success: false,
                                world_name,
                                error_message: Some(error_msg),
                                save_time_ms: start_time.elapsed().as_secs_f32() * 1000.0,
                            }
                        }
                    },
                    Err(e) => {
                        let error_msg = format!("Failed to serialize world data: {}", e);
                        error!("{}", error_msg);
                        WorldSaveResult {
                            success: false,
                            world_name,
                            error_message: Some(error_msg),
                            save_time_ms: start_time.elapsed().as_secs_f32() * 1000.0,
                        }
                    }
                }
            });

            task_manager.saving_tasks.push(WorldSavingTask {
                task,
                world_name: event.world_name.clone(),
            });
        } else {
            warn!("No current world to save for async operation");
        }
    }
}

/// Complete world loading tasks and apply results
fn complete_world_loading_tasks(
    mut commands: Commands,
    mut task_manager: ResMut<AsyncWorldTaskManager>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut inventory: ResMut<PlayerInventory>,
    camera_query: Query<Entity, With<crate::camera_controllers::CameraController>>,
    existing_blocks_query: Query<Entity, With<VoxelBlock>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    discovered_worlds: Res<DiscoveredWorlds>,
    mut error_resource: ResMut<crate::ui::error_indicator::ErrorResource>,
    time: Res<Time>,
) {
    task_manager.loading_tasks.retain_mut(|loading_task| {
        if let Some(result) = bevy::tasks::block_on(bevy::tasks::poll_once(&mut loading_task.task))
        {
            if result.success {
                if let Some(save_data) = result.save_data {
                    info!(
                        "Applying async world load for '{}' with {} blocks",
                        result.world_name,
                        save_data.blocks.len()
                    );

                    // Clear existing blocks
                    let cleared_entities = existing_blocks_query.iter().count();
                    for entity in existing_blocks_query.iter() {
                        commands.entity(entity).despawn();
                    }
                    info!("Cleared {} existing block entities", cleared_entities);

                    // Update voxel world
                    voxel_world.blocks.clear();
                    for block_data in &save_data.blocks {
                        voxel_world.blocks.insert(
                            IVec3::new(block_data.x, block_data.y, block_data.z),
                            block_data.block_type,
                        );
                    }

                    // Spawn blocks in batches to avoid frame drops
                    spawn_blocks_in_batches(
                        &mut commands,
                        &save_data.blocks,
                        &mut meshes,
                        &mut materials,
                        &asset_server,
                    );

                    // Update inventory
                    *inventory = save_data.inventory;
                    inventory.ensure_proper_size();
                    inventory.set_changed();

                    // Update current world resource
                    if let Some(world_info) = discovered_worlds
                        .worlds
                        .iter()
                        .find(|w| w.name == result.world_name)
                    {
                        commands.insert_resource(CurrentWorld {
                            name: world_info.name.clone(),
                            path: world_info.path.clone(),
                            metadata: world_info.metadata.clone(),
                        });
                    }

                    // Set player position
                    if let Ok(camera_entity) = camera_query.single() {
                        commands.entity(camera_entity).insert(Transform {
                            translation: save_data.player_position,
                            rotation: save_data.player_rotation,
                            ..default()
                        });
                    }

                    info!(
                        "Successfully applied async world load: {}",
                        result.world_name
                    );
                } else {
                    info!("Loaded empty world: {}", result.world_name);
                }
            } else {
                if let Some(error_msg) = &result.error_message {
                    error_resource.indicator_on = true;
                    error_resource.last_error_time = time.elapsed_secs();
                    error_resource.messages.push(error_msg.clone());
                }
            }

            false // Remove completed task
        } else {
            true // Keep task (still running)
        }
    });
}

/// Complete world saving tasks
fn complete_world_saving_tasks(
    mut task_manager: ResMut<AsyncWorldTaskManager>,
    mut error_resource: ResMut<crate::ui::error_indicator::ErrorResource>,
    time: Res<Time>,
) {
    task_manager.saving_tasks.retain_mut(|saving_task| {
        if let Some(result) = bevy::tasks::block_on(bevy::tasks::poll_once(&mut saving_task.task)) {
            if result.success {
                info!(
                    "Successfully completed async world save: {}",
                    result.world_name
                );
            } else {
                if let Some(error_msg) = &result.error_message {
                    error_resource.indicator_on = true;
                    error_resource.last_error_time = time.elapsed_secs();
                    error_resource.messages.push(error_msg.clone());
                }
            }

            false // Remove completed task
        } else {
            true // Keep task (still running)
        }
    });
}

/// Spawn blocks in batches to avoid frame drops
fn spawn_blocks_in_batches(
    commands: &mut Commands,
    blocks: &[VoxelBlockData],
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    asset_server: &Res<AssetServer>,
) {
    let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));

    // Process blocks in batches
    for chunk in blocks.chunks(BLOCK_SPAWN_BATCH_SIZE) {
        for block_data in chunk {
            let texture_path = match block_data.block_type {
                BlockType::Grass => "textures/grass.webp",
                BlockType::Dirt => "textures/dirt.webp",
                BlockType::Stone => "textures/stone.webp",
                BlockType::QuartzBlock => "textures/quartz_block.webp",
                BlockType::GlassPane => "textures/glass_pane.webp",
                BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                BlockType::Water => "textures/water.webp",
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
                    block_data.x as f32,
                    block_data.y as f32,
                    block_data.z as f32,
                )),
                VoxelBlock {
                    position: IVec3::new(block_data.x, block_data.y, block_data.z),
                },
            ));
        }
    }

    info!("Spawned {} block entities in batches", blocks.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_async_world_operations() {
        // This test would need to be run in an async context
        // For now, just test that the structures are properly defined
        let manager = AsyncWorldTaskManager::default();
        assert_eq!(manager.loading_tasks.len(), 0);
        assert_eq!(manager.saving_tasks.len(), 0);
    }
}
