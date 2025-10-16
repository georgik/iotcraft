use bevy::prelude::*;
use log::{error, info, warn};
use std::fs;
use std::path::Path;

use super::world_types::*;
use crate::camera_controllers::CameraController;
use crate::environment::VoxelWorld;
use crate::script::script_types::PendingCommands;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<LoadWorldEvent>()
            .add_event::<SaveWorldEvent>()
            .add_event::<CreateWorldEvent>()
            .add_event::<DeleteWorldEvent>()
            .init_resource::<DiscoveredWorlds>()
            .add_systems(Startup, discover_worlds)
            // World management systems should run early in Update to ensure
            // world state changes are processed before other game logic
            .add_systems(
                Update,
                (
                    handle_load_world_events,
                    handle_create_world_events,
                    handle_save_world_events,
                    handle_delete_world_events,
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
#[cfg(not(target_arch = "wasm32"))]
pub fn get_worlds_directory() -> std::path::PathBuf {
    let mut path = dirs::document_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    path.push("IOTCraft");
    path.push("worlds");
    path
}

/// For web, we'll use a virtual directory concept
#[cfg(target_arch = "wasm32")]
pub fn get_worlds_directory() -> std::path::PathBuf {
    // Return a dummy path for web - we'll handle storage differently
    std::path::PathBuf::from("web_worlds")
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

/// Load web-compatible template scripts for WASM builds
#[cfg(target_arch = "wasm32")]
fn load_web_template(
    template_name: &str,
    pending_commands: &mut ResMut<PendingCommands>,
    world_name: &str,
) {
    info!(
        "Loading web template '{}' for world {}",
        template_name, world_name
    );

    let script_commands = match template_name {
        "creative" => get_creative_template(),
        "default" => get_default_template(),
        "medieval" => get_medieval_template(),
        "modern" => get_modern_template(),
        _ => {
            warn!(
                "Unknown template '{}', falling back to default",
                template_name
            );
            get_default_template()
        }
    };

    info!(
        "Executing web template '{}' with {} commands for world {}",
        template_name,
        script_commands.len(),
        world_name
    );

    pending_commands.commands.extend(script_commands);
}

/// Get creative template script commands
#[cfg(target_arch = "wasm32")]
fn get_creative_template() -> Vec<String> {
    vec![
        "# Creative Template - Expansive world for unlimited creativity".to_string(),
        "tp 0 50 0".to_string(),
        "look 25 0".to_string(),
        "".to_string(),
        "# Create a massive flat creative platform (200x200)".to_string(),
        "wall grass -100 10 -100 100 10 100".to_string(),
        "".to_string(),
        "# Create material showcase areas".to_string(),
        "# Stone section".to_string(),
        "wall stone -80 11 -80 -60 15 -60".to_string(),
        "".to_string(),
        "# Dirt hills for terraforming practice".to_string(),
        "wall dirt -40 11 -40 -20 20 -20".to_string(),
        "wall grass -40 21 -40 -20 21 -20".to_string(),
        "".to_string(),
        "# Quartz showcase building".to_string(),
        "wall quartz_block 0 11 -50 20 25 -30".to_string(),
        "wall glass_pane 5 16 -45 15 20 -35".to_string(),
        "".to_string(),
        "# Water features for landscaping".to_string(),
        "wall water 30 11 -30 50 11 -10".to_string(),
        "".to_string(),
        "# Colorful terracotta section".to_string(),
        "wall cyan_terracotta 60 11 -20 80 11 0".to_string(),
        "".to_string(),
        "# Starting inventory with creative materials".to_string(),
        "give grass 200".to_string(),
        "give dirt 200".to_string(),
        "give stone 200".to_string(),
        "give quartz_block 100".to_string(),
        "give glass_pane 100".to_string(),
        "give cyan_terracotta 100".to_string(),
        "give water 50".to_string(),
    ]
}

/// Get default template script commands
#[cfg(target_arch = "wasm32")]
fn get_default_template() -> Vec<String> {
    vec![
        "# Default Template - Basic world for new players".to_string(),
        "tp -8 3 15".to_string(),
        "look -34 -3".to_string(),
        "".to_string(),
        "# Create a large grass plain (80x80 area)".to_string(),
        "wall grass -40 0 -40 40 0 40".to_string(),
        "".to_string(),
        "# Create some hills around the plain".to_string(),
        "wall dirt 15 1 15 20 3 20".to_string(),
        "wall grass 15 4 15 20 4 20".to_string(),
        "".to_string(),
        "wall dirt -20 1 -20 -15 4 -15".to_string(),
        "wall grass -20 5 -20 -15 5 -15".to_string(),
        "".to_string(),
        "# Create a small structure".to_string(),
        "wall stone 3 1 3 7 3 7".to_string(),
        "wall stone 4 4 4 6 4 6".to_string(),
        "place quartz_block 5 5 5".to_string(),
        "".to_string(),
        "# Basic starting inventory".to_string(),
        "give grass 64".to_string(),
        "give dirt 32".to_string(),
        "give stone 32".to_string(),
        "give quartz_block 16".to_string(),
    ]
}

/// Get medieval template script commands
#[cfg(target_arch = "wasm32")]
fn get_medieval_template() -> Vec<String> {
    vec![
        "# Medieval Template - Castle and village setting".to_string(),
        "tp -10 8 25".to_string(),
        "look -15 -8".to_string(),
        "".to_string(),
        "# Create base terrain".to_string(),
        "wall grass -50 0 -50 50 0 50".to_string(),
        "wall dirt -50 -1 -50 50 -1 50".to_string(),
        "".to_string(),
        "# Build castle foundation".to_string(),
        "wall stone -20 1 -20 20 1 20".to_string(),
        "wall stone -20 1 -20 20 5 -15".to_string(),
        "wall stone -20 1 15 20 5 20".to_string(),
        "wall stone -20 1 -20 -15 5 20".to_string(),
        "wall stone 15 1 -20 20 5 20".to_string(),
        "".to_string(),
        "# Castle towers".to_string(),
        "wall stone -18 6 -18 -15 12 -15".to_string(),
        "wall stone 15 6 -18 18 12 -15".to_string(),
        "wall stone -18 6 15 -15 12 18".to_string(),
        "wall stone 15 6 15 18 12 18".to_string(),
        "".to_string(),
        "# Castle gate and courtyard".to_string(),
        "wall stone -2 2 -20 2 4 -15".to_string(),
        "wall grass -15 2 -15 15 2 15".to_string(),
        "".to_string(),
        "# Village houses".to_string(),
        "wall stone -35 1 5 -30 3 10".to_string(),
        "wall stone -35 4 7 -30 4 8".to_string(),
        "wall stone 30 1 5 35 3 10".to_string(),
        "wall stone 30 4 7 35 4 8".to_string(),
        "".to_string(),
        "# Fields and farms".to_string(),
        "wall dirt -45 1 -10 -25 1 5".to_string(),
        "wall dirt 25 1 -10 45 1 5".to_string(),
        "".to_string(),
        "# Decorative elements".to_string(),
        "place quartz_block 0 6 0".to_string(),
        "place glass_pane -1 3 -17".to_string(),
        "place glass_pane 1 3 -17".to_string(),
        "".to_string(),
        "# Medieval starter inventory".to_string(),
        "give stone 128".to_string(),
        "give dirt 64".to_string(),
        "give grass 32".to_string(),
        "give quartz_block 16".to_string(),
        "give glass_pane 8".to_string(),
    ]
}

/// Get modern template script commands
#[cfg(target_arch = "wasm32")]
fn get_modern_template() -> Vec<String> {
    vec![
        "# Modern Template - Contemporary city setting".to_string(),
        "tp 0 15 30".to_string(),
        "look -10 0".to_string(),
        "".to_string(),
        "# Create modern city base".to_string(),
        "wall stone -60 0 -60 60 0 60".to_string(),
        "".to_string(),
        "# Modern skyscraper foundations".to_string(),
        "wall quartz_block -25 1 -25 -15 1 -15".to_string(),
        "wall quartz_block -25 2 -25 -15 20 -15".to_string(),
        "".to_string(),
        "wall quartz_block 15 1 -25 25 1 -15".to_string(),
        "wall quartz_block 15 2 -25 25 15 -15".to_string(),
        "".to_string(),
        "wall quartz_block -10 1 10 10 1 20".to_string(),
        "wall quartz_block -10 2 10 10 25 20".to_string(),
        "".to_string(),
        "# Glass windows for buildings".to_string(),
        "wall glass_pane -23 3 -23 -17 18 -17".to_string(),
        "wall glass_pane 17 3 -23 23 13 -17".to_string(),
        "wall glass_pane -8 3 12 8 23 18".to_string(),
        "".to_string(),
        "# Modern plaza with decorative elements".to_string(),
        "wall cyan_terracotta -5 1 -5 5 1 5".to_string(),
        "place quartz_block 0 2 0".to_string(),
        "".to_string(),
        "# Urban green spaces".to_string(),
        "wall grass -40 1 30 -25 1 45".to_string(),
        "wall grass 25 1 30 40 1 45".to_string(),
        "".to_string(),
        "# Water features - modern fountains".to_string(),
        "wall water -35 2 35 -30 2 40".to_string(),
        "wall water 30 2 35 35 2 40".to_string(),
        "".to_string(),
        "# Roads and pathways".to_string(),
        "wall stone -60 1 -2 60 1 2".to_string(),
        "wall stone -2 1 -60 2 1 60".to_string(),
        "".to_string(),
        "# Modern builder inventory".to_string(),
        "give quartz_block 200".to_string(),
        "give glass_pane 100".to_string(),
        "give stone 150".to_string(),
        "give cyan_terracotta 50".to_string(),
        "give water 20".to_string(),
        "give grass 32".to_string(),
    ]
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
    shared_materials: Option<Res<crate::shared_materials::SharedBlockMaterials>>,
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

                                // Spawn visual blocks for all loaded blocks using shared materials
                                let mut spawned_blocks = 0;
                                for (pos, block_type) in voxel_world.blocks.iter() {
                                    let cube_mesh = meshes.add(Cuboid::new(
                                        crate::environment::CUBE_SIZE,
                                        crate::environment::CUBE_SIZE,
                                        crate::environment::CUBE_SIZE,
                                    ));

                                    // Use SharedBlockMaterials if available, otherwise fall back to creating new materials
                                    let material = if let Some(shared_materials) =
                                        shared_materials.as_ref()
                                    {
                                        shared_materials.get_material(*block_type)
                                            .unwrap_or_else(|| {
                                                // Fallback to creating individual material
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
                                                materials.add(StandardMaterial {
                                                    base_color_texture: Some(texture),
                                                    ..default()
                                                })
                                            })
                                    } else {
                                        // Fallback when shared materials not available
                                        let texture_path = match block_type {
                                            crate::environment::BlockType::Grass => {
                                                "textures/grass.webp"
                                            }
                                            crate::environment::BlockType::Dirt => {
                                                "textures/dirt.webp"
                                            }
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
                                            crate::environment::BlockType::Water => {
                                                "textures/water.webp"
                                            }
                                        };
                                        let texture: Handle<Image> =
                                            asset_server.load(texture_path);
                                        materials.add(StandardMaterial {
                                            base_color_texture: Some(texture),
                                            ..default()
                                        })
                                    };

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
                    &existing_blocks_query,
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
    mut pending_commands: ResMut<PendingCommands>,
    existing_blocks_query: Query<Entity, With<crate::environment::VoxelBlock>>,
) {
    for event in create_events.read() {
        info!("Creating new world: {}", event.world_name);

        let worlds_dir = get_worlds_directory();
        let world_path = worlds_dir.join(&event.world_name);

        // Create world directory (skip on WASM where filesystem operations aren't supported)
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Err(e) = fs::create_dir_all(&world_path) {
                error!(
                    "Failed to create world directory for {}: {}",
                    event.world_name, e
                );
                continue;
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            // On WASM, we can't create directories, but we can still create the world
            info!(
                "Skipping directory creation on WASM platform for world: {}",
                event.world_name
            );
        }

        // Create metadata
        let metadata = WorldMetadata {
            name: event.world_name.clone(),
            description: event.description.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            last_played: chrono::Utc::now().to_rfc3339(),
            version: "1.0.0".to_string(),
        };

        // Save metadata (skip on WASM where filesystem operations aren't supported)
        #[cfg(not(target_arch = "wasm32"))]
        {
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
        }
        #[cfg(target_arch = "wasm32")]
        {
            // On WASM, we can't save to filesystem, but metadata is still used in memory
            info!(
                "Skipping metadata file save on WASM platform for world: {}",
                event.world_name
            );
        }

        // Create empty world
        create_empty_world(
            &event.world_name,
            &event.description,
            &mut voxel_world,
            &mut commands,
            &world_path,
            &existing_blocks_query,
        );

        // Add to discovered worlds
        discovered_worlds.worlds.push(WorldInfo {
            name: event.world_name.clone(),
            path: world_path,
            metadata,
        });

        // Execute world template script based on event template or default
        let template_name = event.template.as_deref().unwrap_or("default");

        #[cfg(target_arch = "wasm32")]
        {
            // Use web-compatible template loading
            load_web_template(template_name, &mut pending_commands, &event.world_name);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Use filesystem-based template loading for desktop
            let template_path = format!("scripts/world_templates/{}.txt", template_name);

            info!(
                "Loading world template '{}' from path: {}",
                template_name, template_path
            );

            if std::path::Path::new(&template_path).exists() {
                match fs::read_to_string(&template_path) {
                    Ok(content) => {
                        let script_commands = content
                            .lines()
                            .map(|line| line.trim())
                            .filter(|line| !line.is_empty() && !line.starts_with('#'))
                            .map(|line| line.to_string())
                            .collect::<Vec<String>>();

                        info!(
                            "Executing world template '{}' with {} commands for world {}",
                            template_name,
                            script_commands.len(),
                            event.world_name
                        );

                        // Debug: show first few commands being added
                        for (i, cmd) in script_commands.iter().take(5).enumerate() {
                            info!("  Template command {}: '{}'", i, cmd);
                        }

                        pending_commands.commands.extend(script_commands);
                    }
                    Err(e) => {
                        error!("Failed to read world template '{}': {}", template_name, e);
                    }
                }
            } else {
                // Fallback to legacy path if template doesn't exist
                let fallback_script_path = "scripts/new_world.txt";
                if std::path::Path::new(fallback_script_path).exists() {
                    info!(
                        "Template '{}' not found, falling back to legacy script: {}",
                        template_name, fallback_script_path
                    );
                    match fs::read_to_string(fallback_script_path) {
                        Ok(content) => {
                            let script_commands = content
                                .lines()
                                .map(|line| line.trim())
                                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                                .map(|line| line.to_string())
                                .collect::<Vec<String>>();

                            info!(
                                "Executing fallback world script with {} commands for world {}",
                                script_commands.len(),
                                event.world_name
                            );
                            pending_commands.commands.extend(script_commands);
                        }
                        Err(e) => {
                            error!("Failed to read fallback world script: {}", e);
                        }
                    }
                } else {
                    warn!(
                        "Neither template '{}' at '{}' nor fallback script at '{}' found, world will be empty",
                        template_name, template_path, fallback_script_path
                    );
                }
            }
        }

        info!("Successfully created new world: {}", event.world_name);
    }
}

/// Creates an empty world and optionally executes a script
fn create_empty_world(
    world_name: &str,
    description: &str,
    voxel_world: &mut VoxelWorld,
    commands: &mut Commands,
    world_path: &Path,
    existing_blocks_query: &Query<Entity, With<crate::environment::VoxelBlock>>,
) {
    // Clear existing 3D block entities from the scene
    let mut cleared_entities = 0usize;
    for entity in existing_blocks_query.iter() {
        commands.entity(entity).despawn();
        cleared_entities += 1;
    }
    info!(
        "Cleared {} existing block entities before creating new world",
        cleared_entities
    );

    // Clear existing blocks in voxel storage
    voxel_world.blocks.clear();

    // Reset scene setup guard to allow template scripts to execute
    #[cfg(target_arch = "wasm32")]
    {
        // For WASM builds, reset the scene setup guard so that template scripts can execute
        // This allows new worlds to properly run their template commands instead of being blocked
        commands.insert_resource(crate::SceneSetupGuard(false));
        info!("Reset scene setup guard to allow template script execution for new world");
    }

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

/// Handles world deletion events
fn handle_delete_world_events(
    mut delete_events: EventReader<DeleteWorldEvent>,
    mut discovered_worlds: ResMut<DiscoveredWorlds>,
    _commands: Commands,
    mut game_state: ResMut<NextState<crate::ui::GameState>>,
) {
    for event in delete_events.read() {
        info!("Deleting world: {}", event.world_name);

        // Find the world in discovered worlds
        if let Some(world_index) = discovered_worlds
            .worlds
            .iter()
            .position(|w| w.name == event.world_name)
        {
            let world_info = &discovered_worlds.worlds[world_index];
            let world_path = &world_info.path;

            // Remove the world directory and all its contents
            match std::fs::remove_dir_all(world_path) {
                Ok(_) => {
                    info!("Successfully deleted world directory: {:?}", world_path);
                    // Remove from discovered worlds list
                    discovered_worlds.worlds.remove(world_index);
                    info!(
                        "Removed world '{}' from discovered worlds list",
                        event.world_name
                    );

                    // Refresh the world selection menu by going back to main menu and returning
                    // This ensures the UI updates immediately
                    game_state.set(crate::ui::GameState::MainMenu);
                }
                Err(e) => {
                    error!("Failed to delete world directory {:?}: {}", world_path, e);
                }
            }
        } else {
            warn!("World '{}' not found for deletion", event.world_name);
        }
    }
}
