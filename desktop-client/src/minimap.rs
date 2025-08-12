use crate::environment::{BlockType, VoxelWorld};
use crate::ui::GameState;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, poll_once};
use std::collections::HashMap;

/// Plugin for minimap/radar functionality
pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                toggle_minimap,
                start_minimap_texture_generation,
                finish_minimap_texture_generation,
                update_minimap_visibility,
            )
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(OnEnter(GameState::InGame), setup_minimap)
        .add_systems(OnExit(GameState::InGame), cleanup_minimap);
    }
}

/// Different minimap display modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinimapMode {
    PlayerOriented, // Map rotates with player view direction (default)
    FixedNorth,     // North is always up
    Hidden,         // Minimap is off
}

impl Default for MinimapMode {
    fn default() -> Self {
        Self::PlayerOriented // Start with player-oriented mode (performance is good now)
    }
}

/// Resource to track minimap state
#[derive(Resource, Default)]
pub struct MinimapState {
    pub mode: MinimapMode,
    pub size: f32,  // Size of the minimap as percentage of screen
    pub scale: f32, // How many world units to show
}

/// Component to mark the minimap camera
#[derive(Component)]
pub struct MinimapCamera;

/// Component to mark the minimap UI
#[derive(Component)]
pub struct MinimapUI;

/// Component to mark the player indicator on minimap
#[derive(Component)]
pub struct MinimapPlayerIndicator;

/// Component to mark the minimap texture that needs updating
#[derive(Component)]
pub struct MinimapTexture {
    pub image_handle: Handle<Image>,
    pub last_update: f64,
    pub update_interval: f64,
    pub last_player_pos: Vec3, // Track player position to throttle updates
}

/// Component to track async minimap generation tasks
#[derive(Component)]
pub struct MinimapGenerationTask {
    pub task: Task<Vec<u8>>,
    pub player_pos: Vec3,
    pub player_rotation: Option<f32>,
}

/// Get color for different block types
fn get_block_color(block_type: BlockType) -> [u8; 4] {
    match block_type {
        BlockType::Grass => [34, 139, 34, 255],   // Forest green
        BlockType::Dirt => [139, 69, 19, 255],    // Saddle brown
        BlockType::Stone => [128, 128, 128, 255], // Gray
        BlockType::QuartzBlock => [245, 245, 220, 255], // Beige
        BlockType::GlassPane => [173, 216, 230, 255], // Light blue (semi-transparent look)
        BlockType::CyanTerracotta => [72, 209, 204, 255], // Medium turquoise
    }
}

/// Generate a 2D minimap texture from the voxel world (now async-compatible)
fn generate_minimap_texture_sync(
    blocks: HashMap<IVec3, BlockType>, // Pass owned data for async
    player_pos: Vec3,
    player_rotation: Option<f32>, // Player's yaw rotation in radians (None for fixed north mode)
    texture_size: u32,
    world_radius: i32,
) -> Vec<u8> {
    let mut pixels = vec![0u8; (texture_size * texture_size * 4) as usize];
    let half_size = texture_size as f32 / 2.0;
    let scale = world_radius as f32 / half_size;

    // Fill background with dark blue (water/void color)
    for i in (0..pixels.len()).step_by(4) {
        pixels[i] = 25; // R
        pixels[i + 1] = 25; // G  
        pixels[i + 2] = 112; // B (dark blue)
        pixels[i + 3] = 255; // A
    }

    // Pre-compute rotation values if needed (major performance optimization)
    let (cos_r, sin_r) = if let Some(rotation) = player_rotation {
        (rotation.cos(), rotation.sin())
    } else {
        (1.0, 0.0) // No rotation for fixed north mode
    };

    // Render blocks around player position
    for y in 0..texture_size {
        for x in 0..texture_size {
            // Get local coordinates relative to texture center
            let local_x = (x as f32) - half_size;
            let local_y = (y as f32) - half_size;

            // Apply rotation (now using pre-computed values)
            let (world_x, world_z) = if player_rotation.is_some() {
                // Rotate local coordinates
                let rotated_x = local_x * cos_r - local_y * sin_r;
                let rotated_z = local_x * sin_r + local_y * cos_r;

                // Convert to world coordinates
                (
                    player_pos.x + rotated_x * scale,
                    player_pos.z + rotated_z * scale,
                )
            } else {
                // Fixed north mode - no rotation
                (
                    player_pos.x + local_x * scale,
                    player_pos.z + local_y * scale,
                )
            };

            // Find the topmost block at this X,Z position
            let mut highest_block: Option<(i32, BlockType)> = None;

            // Check fewer Y levels to find the highest block (performance optimization)
            for check_y in (-5..=20).rev() {
                // Check from top to bottom (reduced range)
                let block_pos = IVec3::new(world_x.round() as i32, check_y, world_z.round() as i32);

                if let Some(block_type) = blocks.get(&block_pos) {
                    highest_block = Some((check_y, *block_type));
                    break;
                }
            }

            // Set pixel color based on the highest block found
            if let Some((_height, block_type)) = highest_block {
                let color = get_block_color(block_type);
                let pixel_index = ((y * texture_size + x) * 4) as usize;

                if pixel_index + 3 < pixels.len() {
                    pixels[pixel_index] = color[0]; // R
                    pixels[pixel_index + 1] = color[1]; // G
                    pixels[pixel_index + 2] = color[2]; // B
                    pixels[pixel_index + 3] = color[3]; // A
                }
            }
        }
    }

    pixels
}

/// System to set up the minimap when entering game
fn setup_minimap(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Insert minimap state resource with player-oriented mode as default
    commands.insert_resource(MinimapState {
        mode: MinimapMode::PlayerOriented, // Start with player-oriented mode (performance is good now)
        size: 0.2,                         // 20% of screen size
        scale: 50.0,                       // Show 50x50 world units
    });

    // Create initial minimap texture (will be updated dynamically)
    let size = 128; // Minimap texture resolution (reduced for performance)
    let mut initial_pixels = Vec::new();
    for _ in 0..(size * size) {
        initial_pixels.extend_from_slice(&[25u8, 25u8, 112u8, 255u8]); // Dark blue background
    }

    let minimap_image = Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        initial_pixels,
        TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD
            | bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );

    let minimap_image_handle = images.add(minimap_image);

    // Create minimap UI overlay
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0), // Position in top-right corner
                top: Val::Px(10.0),
                width: Val::Px(200.0), // UI size
                height: Val::Px(200.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BorderColor(Color::WHITE),
            BackgroundColor(Color::NONE),
            Visibility::Visible, // Start visible (performance is good now)
            MinimapUI,
            Name::new("Minimap UI"),
        ))
        .with_children(|parent| {
            // Minimap image display with texture component for updates
            parent.spawn((
                ImageNode::new(minimap_image_handle.clone()),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                MinimapTexture {
                    image_handle: minimap_image_handle,
                    last_update: 0.0,
                    update_interval: 0.8, // Update every 0.8 seconds (increased frequency)
                    last_player_pos: Vec3::ZERO,
                },
            ));

            // Player indicator (center dot)
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    width: Val::Px(6.0),
                    height: Val::Px(6.0),
                    margin: UiRect {
                        left: Val::Px(-3.0), // Center the dot
                        top: Val::Px(-3.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(Color::srgb(1.0, 1.0, 0.0)), // Yellow player dot
                MinimapPlayerIndicator,
            ));
        });

    info!("2D Minimap system initialized - press M to toggle");
}

/// System to clean up minimap when exiting game
fn cleanup_minimap(
    mut commands: Commands,
    minimap_cameras: Query<Entity, With<MinimapCamera>>,
    minimap_ui: Query<Entity, With<MinimapUI>>,
) {
    // Remove minimap camera
    for entity in minimap_cameras.iter() {
        commands.entity(entity).despawn();
    }

    // Remove minimap UI
    for entity in minimap_ui.iter() {
        commands.entity(entity).despawn();
    }

    // Remove minimap state resource
    commands.remove_resource::<MinimapState>();
}

/// System to toggle minimap modes (Player-Oriented -> Fixed North -> Hidden -> ...)
fn toggle_minimap(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut minimap_state: ResMut<MinimapState>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        let (new_mode, mode_name) = match minimap_state.mode {
            MinimapMode::PlayerOriented => (MinimapMode::FixedNorth, "Fixed North"),
            MinimapMode::FixedNorth => (MinimapMode::Hidden, "Hidden"),
            MinimapMode::Hidden => (MinimapMode::PlayerOriented, "Player Oriented"),
        };

        minimap_state.mode = new_mode;
        info!("Minimap mode: {} (M to cycle)", mode_name);
    }
}

/// Extract yaw rotation from transform quaternion
fn extract_yaw_from_transform(transform: &Transform) -> f32 {
    // Convert quaternion to Euler angles and extract yaw (Y rotation)
    let (yaw, _pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
    yaw
}

/// System to start async minimap texture generation
fn start_minimap_texture_generation(
    mut commands: Commands,
    mut minimap_textures: Query<(Entity, &mut MinimapTexture), Without<MinimapGenerationTask>>,
    player_camera_query: Query<&Transform, With<Camera>>,
    minimap_state: Res<MinimapState>,
    voxel_world: Res<VoxelWorld>,
    time: Res<Time>,
) {
    // Skip if minimap is hidden
    if minimap_state.mode == MinimapMode::Hidden {
        return;
    }

    let current_time = time.elapsed_secs_f64();

    // Get player position and rotation
    let (player_pos, player_rotation) = if let Ok(player_transform) = player_camera_query.single() {
        let pos = player_transform.translation;
        let rotation = match minimap_state.mode {
            MinimapMode::PlayerOriented => Some(extract_yaw_from_transform(player_transform)),
            MinimapMode::FixedNorth => None,
            MinimapMode::Hidden => return, // Already handled above
        };
        (pos, rotation)
    } else {
        return;
    };

    // Start async tasks for each minimap texture that needs updating
    for (entity, mut minimap_texture) in minimap_textures.iter_mut() {
        // Check if it's time to update
        if current_time - minimap_texture.last_update < minimap_texture.update_interval {
            continue;
        }

        // Only update if player moved significantly (performance optimization)
        let movement_threshold = 3.0; // Only update if player moved 3+ units
        if minimap_texture.last_player_pos.distance(player_pos) < movement_threshold {
            continue;
        }

        minimap_texture.last_update = current_time;
        minimap_texture.last_player_pos = player_pos;

        // Clone the blocks data for the async task (only relevant blocks for performance)
        let player_x = player_pos.x as i32;
        let player_z = player_pos.z as i32;
        let world_radius = 25i32;

        let relevant_blocks: HashMap<IVec3, BlockType> = voxel_world
            .blocks
            .iter()
            .filter(|(pos, _)| {
                let dx = (pos.x - player_x).abs();
                let dz = (pos.z - player_z).abs();
                dx <= world_radius && dz <= world_radius && pos.y >= -5 && pos.y <= 20
            })
            .map(|(pos, block_type)| (*pos, *block_type))
            .collect();

        info!(
            "Starting async minimap generation with {} relevant blocks",
            relevant_blocks.len()
        );

        // Spawn async task on compute thread pool
        let task_pool = AsyncComputeTaskPool::get();
        let texture_size = 128u32;

        let task = task_pool.spawn(async move {
            generate_minimap_texture_sync(
                relevant_blocks,
                player_pos,
                player_rotation,
                texture_size,
                world_radius,
            )
        });

        // Add the task component to track completion
        commands.entity(entity).insert(MinimapGenerationTask {
            task,
            player_pos,
            player_rotation,
        });
    }
}

/// System to finish async minimap texture generation and apply results
fn finish_minimap_texture_generation(
    mut commands: Commands,
    mut query: Query<(Entity, &MinimapTexture, &mut MinimapGenerationTask)>,
    mut images: ResMut<Assets<Image>>,
    minimap_state: Res<MinimapState>,
    time: Res<Time>,
) {
    let current_time = time.elapsed_secs_f64();

    for (entity, minimap_texture, mut task_component) in query.iter_mut() {
        // Check if the async task is complete
        if let Some(texture_data) = block_on(poll_once(&mut task_component.task)) {
            // Apply the generated texture data
            if let Some(image) = images.get_mut(&minimap_texture.image_handle) {
                image.data = Some(texture_data);

                // Debug logging
                static mut LAST_DEBUG_TIME: f64 = 0.0;
                unsafe {
                    if current_time - LAST_DEBUG_TIME > 3.0 {
                        LAST_DEBUG_TIME = current_time;
                        let mode_str = match minimap_state.mode {
                            MinimapMode::PlayerOriented => "Player-Oriented",
                            MinimapMode::FixedNorth => "Fixed North",
                            MinimapMode::Hidden => "Hidden",
                        };
                        info!(
                            "Minimap texture completed (async) - Mode: {}, Player at {:?}",
                            mode_str, task_component.player_pos
                        );
                    }
                }
            }

            // Remove the task component since it's complete
            commands.entity(entity).remove::<MinimapGenerationTask>();
        }
    }
}

/// System to update minimap visibility based on state
fn update_minimap_visibility(
    mut minimap_ui_query: Query<&mut Visibility, With<MinimapUI>>,
    minimap_state: Res<MinimapState>,
) {
    // Update UI visibility based on mode
    if let Ok(mut visibility) = minimap_ui_query.single_mut() {
        *visibility = if minimap_state.mode == MinimapMode::Hidden {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}
