use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;

use super::environment_types::*;
use crate::camera_controllers::CameraController;
use crate::console::BlinkCube;
use crate::mqtt::TemperatureResource;
use crate::script::script_types::PendingCommands;

pub struct EnvironmentPlugin;

/// Resource to ensure background world setup only runs once
#[derive(Resource, Default)]
struct BackgroundWorldSetupComplete(bool);

impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(VoxelWorld::default())
            .insert_resource(BackgroundWorldSetupComplete::default())
            .add_systems(Startup, setup_environment)
            .add_systems(
                Update,
                (
                    setup_background_world
                        .run_if(|setup_complete: Res<BackgroundWorldSetupComplete>| {
                            !setup_complete.0
                        })
                        .run_if(resource_exists::<PendingCommands>),
                    blinking_system,
                    rotate_logo_system,
                    update_thermometer_material,
                    update_thermometer_scale,
                ),
            );
    }
}

/// Setup the initial environment including voxel terrain
fn setup_environment(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // grass texture for ground
    let grass_texture: Handle<Image> = asset_server.load("textures/grass.webp");
    let grass_material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(grass_texture),
        ..default()
    });

    // generate flat grass terrain using voxels
    voxel_world.generate_flat_terrain(10, 0);

    // cube mesh for voxel blocks
    let voxel_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));

    // spawn voxel blocks
    for (position, block_type) in voxel_world.blocks.iter() {
        let material = match block_type {
            BlockType::Grass => grass_material_handle.clone(),
            _ => grass_material_handle.clone(), // Placeholder for other materials
        };
        commands.spawn((
            Mesh3d(voxel_mesh.clone()),
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

    // block with Espressif logo texture
    let block_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
    let esp_logo_texture: Handle<Image> = asset_server.load("textures/espressif.webp");
    let esp_logo_material = materials.add(StandardMaterial {
        base_color_texture: Some(esp_logo_texture),
        base_color: Color::WHITE,
        ..default()
    });
    commands.spawn((
        Mesh3d(block_mesh),
        MeshMaterial3d(esp_logo_material),
        Transform::from_translation(Vec3::new(3.0, 6.5, 2.0)),
        Visibility::default(),
        LogoCube,
    ));

    // thermometer 3D indicator
    let thermo_mesh = meshes.add(Cuboid::new(0.2, 5.0, 0.2));
    let thermo_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.2),
        ..default()
    });
    commands.spawn((
        Mesh3d(thermo_mesh),
        MeshMaterial3d(thermo_handle.clone()),
        Transform::from_translation(Vec3::new(-3.0, 2.5, 2.0)),
        Thermometer,
    ));
    commands.insert_resource(ThermometerMaterial(thermo_handle));

    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_translation(Vec3::ONE).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(15.0, 5.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        CameraController::default(),
    ));
}

fn blinking_system(
    time: Res<Time>,
    mut blink_state: ResMut<crate::console::BlinkState>,
    query: Query<&MeshMaterial3d<StandardMaterial>, With<BlinkCube>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if blink_state.blinking {
        blink_state.timer.tick(time.delta());
        if blink_state.timer.just_finished() {
            for mesh_material in &query {
                let handle = mesh_material.clone();
                if let Some(mat) = materials.get_mut(&handle) {
                    if mat.base_color == Color::WHITE {
                        blink_state.light_state = false;
                        mat.base_color = Color::srgb(0.2, 0.2, 0.2);
                    } else {
                        blink_state.light_state = true;
                        mat.base_color = Color::WHITE;
                    }
                }
            }
        }
    }
}

fn rotate_logo_system(time: Res<Time>, mut query: Query<&mut Transform, With<LogoCube>>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() * 0.5);
        transform.rotate_x(time.delta_secs() * 0.5);
    }
}

fn update_thermometer_material(
    temp: Res<TemperatureResource>,
    thermo: Res<ThermometerMaterial>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if let Some(mat) = materials.get_mut(&thermo.0) {
        if temp.value.is_some() {
            mat.base_color = Color::srgb(1.0, 0.0, 0.0);
        } else {
            mat.base_color = Color::srgb(0.2, 0.2, 0.2);
        }
    }
}

fn update_thermometer_scale(
    temp: Res<TemperatureResource>,
    mut query: Query<&mut Transform, With<Thermometer>>,
) {
    if let Some(value) = temp.value {
        for mut transform in &mut query {
            let scale_y = (value / 100.0).clamp(0.1, 2.0);
            transform.scale = Vec3::new(1.0, scale_y, 1.0);
        }
    }
}

/// Setup background world by executing the background world script
fn setup_background_world(
    mut pending_commands: ResMut<PendingCommands>,
    mut setup_complete: ResMut<BackgroundWorldSetupComplete>,
) {
    // Execute background world script if it exists
    let background_script_path = "scripts/background_world.txt";
    if std::path::Path::new(background_script_path).exists() {
        match std::fs::read_to_string(background_script_path) {
            Ok(content) => {
                let commands = content
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .map(|line| line.to_string())
                    .collect::<Vec<String>>();

                info!(
                    "Executing background world script with {} commands",
                    commands.len()
                );
                pending_commands.commands.extend(commands);
            }
            Err(e) => {
                error!("Failed to read background world script: {}", e);
            }
        }
    } else {
        info!(
            "Background world script not found at {}, using default terrain",
            background_script_path
        );
    }

    // Mark setup as complete so this system won't run again
    setup_complete.0 = true;
}
