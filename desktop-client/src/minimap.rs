use crate::ui::GameState;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;

/// Plugin for minimap/radar functionality
pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                toggle_minimap,
                update_minimap_camera,
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

/// System to set up the minimap when entering game
fn setup_minimap(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Insert minimap state resource
    commands.insert_resource(MinimapState {
        enabled: false, // Start disabled
        size: 0.2,      // 20% of screen size
        scale: 50.0,    // Show 50x50 world units
    });

    // Create a render target image for the minimap
    let size = 256; // Minimap texture resolution
    let mut minimap_image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &[0, 0, 0, 255], // Black background
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    );

    // Enable the image as a render target
    minimap_image.texture_descriptor.usage |=
        bevy::render::render_resource::TextureUsages::RENDER_ATTACHMENT;

    let minimap_image_handle = images.add(minimap_image);

    // Create minimap camera (top-down orthographic)
    commands.spawn((
        Camera3d::default(),
        Camera {
            // Render to our custom image instead of the main window
            target: RenderTarget::Image(minimap_image_handle.clone().into()),
            order: 1,         // Render after main camera
            is_active: false, // Start inactive
            ..default()
        },
        Transform::from_xyz(0.0, 100.0, 0.0) // High above the world
            .looking_at(Vec3::ZERO, Vec3::Z), // Look down, Z is "north"
        Projection::from(OrthographicProjection {
            scale: 25.0, // Show 50x50 world units (25 radius)
            ..OrthographicProjection::default_3d()
        }),
        MinimapCamera,
        Name::new("Minimap Camera"),
    ));

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
            // Minimap image display
            parent.spawn((
                ImageNode::new(minimap_image_handle),
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
            ));

            // Player indicator (center dot)
            parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    top: Val::Percent(50.0),
                    width: Val::Px(4.0),
                    height: Val::Px(4.0),
                    margin: UiRect {
                        left: Val::Px(-2.0), // Center the dot
                        top: Val::Px(-2.0),
                        ..default()
                    },
                    ..default()
                },
                BackgroundColor(Color::srgb(1.0, 0.0, 0.0)), // Red player dot
                MinimapPlayerIndicator,
            ));
        });

    info!("Minimap system initialized - press M to toggle");
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

/// System to update minimap camera position to follow player
fn update_minimap_camera(
    mut minimap_camera_query: Query<&mut Transform, (With<MinimapCamera>, Without<Camera>)>,
    player_camera_query: Query<&Transform, (With<Camera>, Without<MinimapCamera>)>,
    minimap_state: Res<MinimapState>,
) {
    if !minimap_state.enabled {
        return;
    }

    // Get player camera position
    if let Ok(player_transform) = player_camera_query.single() {
        // Update minimap camera to follow player
        if let Ok(mut minimap_transform) = minimap_camera_query.single_mut() {
            // Position minimap camera above player
            minimap_transform.translation = Vec3::new(
                player_transform.translation.x,
                player_transform.translation.y + 100.0, // 100 units above player
                player_transform.translation.z,
            );
        }
    }
}

/// System to update minimap visibility based on state
fn update_minimap_visibility(
    mut minimap_camera_query: Query<&mut Camera, With<MinimapCamera>>,
    mut minimap_ui_query: Query<&mut Visibility, With<MinimapUI>>,
    minimap_state: Res<MinimapState>,
) {
    // Update camera active state
    if let Ok(mut camera) = minimap_camera_query.single_mut() {
        camera.is_active = minimap_state.enabled;
    }

    // Update UI visibility
    if let Ok(mut visibility) = minimap_ui_query.single_mut() {
        *visibility = if minimap_state.enabled {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
