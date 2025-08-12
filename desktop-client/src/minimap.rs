use crate::environment::{BlockType, VoxelWorld};
use crate::ui::GameState;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Plugin for minimap/radar functionality
pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                toggle_minimap,
                update_minimap_texture,
                update_minimap_visibility,
            )
                .run_if(in_state(GameState::InGame)),
        )
        .add_systems(OnEnter(GameState::InGame), setup_minimap)
        .add_systems(OnExit(GameState::InGame), cleanup_minimap);
    }
}

/// Resource to track minimap state
#[derive(Resource, Default)]
pub struct MinimapState {
    pub enabled: bool,
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

/// Generate a 2D minimap texture from the voxel world
fn generate_minimap_texture(
    voxel_world: &VoxelWorld,
    player_pos: Vec3,
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

    // Render blocks around player position
    for y in 0..texture_size {
        for x in 0..texture_size {
            // Convert texture coordinates to world coordinates
            let world_x = player_pos.x + (x as f32 - half_size) * scale;
            let world_z = player_pos.z + (y as f32 - half_size) * scale;

            // Find the topmost block at this X,Z position
            let mut highest_block: Option<(i32, BlockType)> = None;

            // Check multiple Y levels to find the highest block
            for check_y in (-10..=50).rev() {
                // Check from top to bottom
                let block_pos = IVec3::new(world_x.round() as i32, check_y, world_z.round() as i32);

                if let Some(block_type) = voxel_world.blocks.get(&block_pos) {
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
    // Insert minimap state resource
    commands.insert_resource(MinimapState {
        enabled: false, // Start disabled
        size: 0.2,      // 20% of screen size
        scale: 50.0,    // Show 50x50 world units
    });

    // Create initial minimap texture (will be updated dynamically)
    let size = 256; // Minimap texture resolution
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
            Visibility::Hidden, // Start hidden
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
                    update_interval: 0.5, // Update every 0.5 seconds
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

/// System to toggle minimap visibility
fn toggle_minimap(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut minimap_state: ResMut<MinimapState>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyM) {
        minimap_state.enabled = !minimap_state.enabled;
        info!(
            "Minimap {} (M to toggle)",
            if minimap_state.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

/// System to update minimap texture based on player position and world state
fn update_minimap_texture(
    mut minimap_textures: Query<&mut MinimapTexture>,
    player_camera_query: Query<&Transform, With<Camera>>,
    minimap_state: Res<MinimapState>,
    voxel_world: Res<VoxelWorld>,
    time: Res<Time>,
    mut images: ResMut<Assets<Image>>,
) {
    if !minimap_state.enabled {
        return;
    }

    let current_time = time.elapsed_secs_f64();

    // Get player position
    let player_pos = if let Ok(player_transform) = player_camera_query.single() {
        player_transform.translation
    } else {
        return;
    };

    // Update each minimap texture
    for mut minimap_texture in minimap_textures.iter_mut() {
        // Check if it's time to update
        if current_time - minimap_texture.last_update < minimap_texture.update_interval {
            continue;
        }

        minimap_texture.last_update = current_time;

        // Generate new texture data
        let texture_size = 256u32;
        let world_radius = 30i32; // Show 60x60 world units around player
        let new_pixels =
            generate_minimap_texture(&voxel_world, player_pos, texture_size, world_radius);

        // Update the image asset
        if let Some(image) = images.get_mut(&minimap_texture.image_handle) {
            image.data = Some(new_pixels);

            // Debug logging
            static mut LAST_DEBUG_TIME: f64 = 0.0;
            unsafe {
                if current_time - LAST_DEBUG_TIME > 3.0 {
                    LAST_DEBUG_TIME = current_time;
                    info!(
                        "Minimap texture updated - Player at {:?}, World has {} blocks",
                        player_pos,
                        voxel_world.blocks.len()
                    );
                }
            }
        }
    }
}

/// System to update minimap visibility based on state
fn update_minimap_visibility(
    mut minimap_ui_query: Query<&mut Visibility, With<MinimapUI>>,
    minimap_state: Res<MinimapState>,
) {
    // Update UI visibility
    if let Ok(mut visibility) = minimap_ui_query.single_mut() {
        *visibility = if minimap_state.enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
