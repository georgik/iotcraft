// IoTCraft Desktop Client - Web Version (Gradual Build)
use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Web menu system
use crate::web_menu::{WebGameState, WebMenuPlugin};

// MQTT plugin and related modules
use crate::config::MqttConfig;
use crate::mqtt::MqttPlugin;

// Device types for web (simplified from desktop version)
use crate::mqtt::web::DeviceAnnouncementReceiver;
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Import web MQTT multiplayer types
#[cfg(target_arch = "wasm32")]
use crate::mqtt::web::{PoseMessage, PoseReceiver, PoseSender};

// Import desktop multiplayer types for non-WASM
#[cfg(not(target_arch = "wasm32"))]
use crate::multiplayer::{PoseMessage, PoseReceiver, PoseSender};

/// Device types available in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DeviceType {
    Lamp,
    Door,
    Sensor,
}

impl DeviceType {
    /// Convert from string representation
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "lamp" => Some(DeviceType::Lamp),
            "door" => Some(DeviceType::Door),
            "sensor" => Some(DeviceType::Sensor),
            _ => None,
        }
    }

    /// Convert to string representation
    fn as_str(&self) -> &'static str {
        match self {
            DeviceType::Lamp => "lamp",
            DeviceType::Door => "door",
            DeviceType::Sensor => "sensor",
        }
    }

    /// Get the mesh dimensions for this device type (width, height, depth)
    fn mesh_dimensions(&self) -> (f32, f32, f32) {
        match self {
            DeviceType::Lamp => (1.0, 1.0, 1.0),
            DeviceType::Door => (0.2, 2.0, 1.0),
            DeviceType::Sensor => (1.0, 1.0, 1.0),
        }
    }
}

/// Web-compatible device entity component
#[derive(Component)]
struct DeviceEntity {
    pub device_id: String,
    pub device_type: String,
}

// ============ MULTIPLAYER COMPONENTS & RESOURCES ============

/// Component to mark remote player entities
#[derive(Component)]
struct RemotePlayer;

/// Component to store player avatar information
#[derive(Component)]
struct PlayerAvatar {
    pub player_id: String,
    pub player_name: String,
}

// Multiplayer types are imported from either web MQTT or desktop multiplayer modules

/// Multiplayer connection status
#[derive(Resource, Default)]
struct MultiplayerConnectionStatus {
    pub connection_available: bool,
}

/// Timer for position updates (10 Hz)
#[derive(Resource)]
struct PositionTimer {
    timer: Timer,
    last_position: Option<Vec3>,
}

impl Default for PositionTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.1, TimerMode::Repeating), // 10 Hz
            last_position: None,
        }
    }
}

/// World ID resource
#[derive(Resource, Debug, Clone)]
struct WorldId(pub String);

impl Default for WorldId {
    fn default() -> Self {
        Self("default".to_string())
    }
}

fn now_ts() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        // For WASM, use JavaScript Date.now() which returns milliseconds since epoch
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64
    }
}

/// Set up panic hook for better error reporting in web console
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
    web_sys::console::log_1(&"Panic hook initialized for IoTCraft".into());
}

/// Manual initialization function for WASM
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run() {
    set_panic_hook();
    start();
}

/// Main entry point for WASM (called by HTML)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn main() {
    set_panic_hook();
    start();
}

/// Start the IoTCraft application - simplified for web
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start() {
    // Get build timestamp from compile-time environment variables
    let build_timestamp = env!(
        "BUILD_TIMESTAMP",
        "Set BUILD_TIMESTAMP environment variable during build"
    );
    let start_message = format!(
        "Starting IoTCraft Desktop Client (Web Version) - Build: {}",
        build_timestamp
    );
    web_sys::console::log_1(&start_message.into());

    // Initialize the Bevy app with basic plugins
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "IoTCraft Desktop Client - Web Version".to_string(),
                resolution: (1280.0, 720.0).into(),
                canvas: Some("#canvas".to_owned()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        // Insert resources BEFORE adding plugins that depend on them
        .insert_resource(MqttConfig {
            host: "localhost".to_string(),
            port: 1883,
        })
        .insert_resource(crate::profile::load_or_create_profile_with_override(None))
        .add_plugins(WebMenuPlugin)
        .add_plugins(MqttPlugin) // MQTT connection working!
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .insert_resource(CameraController::new())
        // Multiplayer resources
        .insert_resource(WorldId::default())
        .insert_resource(MultiplayerConnectionStatus::default())
        .insert_resource(PositionTimer::default())
        .add_systems(Startup, (setup_basic_scene, setup_multiplayer_connections))
        .add_systems(
            Update,
            (
                rotate_cube,
                camera_control_system.run_if(in_state(WebGameState::InGame)),
                process_device_announcements,
                update_position_timer,
                publish_local_pose,
                apply_remote_poses,
                log_fps,
            ),
        )
        .run();
}

/// Basic scene components
#[derive(Component)]
struct DemoCube;

#[derive(Component)]
struct Ground;

/// Component for MQTT-spawned devices in web client
#[derive(Component)]
struct WebMqttDevice {
    pub device_id: String,
    pub device_type: String,
    pub is_on: bool,
}

/// Simple camera controller for web
#[derive(Resource, Default)]
pub struct CameraController {
    pub enabled: bool,
    pub sensitivity: f32,
    pub speed: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl CameraController {
    fn new() -> Self {
        Self {
            enabled: false, // Start disabled - menu system will enable it
            sensitivity: 2.0,
            speed: 5.0,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

fn setup_basic_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    info!("Setting up enhanced IoTCraft world scene...");

    // Add a camera positioned like in the original desktop client
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-8.0, 3.0, 15.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    // Add a directional light with shadows like the original
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            1.0,
            -std::f32::consts::FRAC_PI_4,
        )),
    ));

    // Create materials for different block types - ensure asset paths are correct for web
    let grass_texture = asset_server.load("textures/grass.webp");
    let dirt_texture = asset_server.load("textures/dirt.webp");
    let stone_texture = asset_server.load("textures/stone.webp");
    let quartz_texture = asset_server.load("textures/quartz_block.webp");
    let glass_texture = asset_server.load("textures/glass_pane.webp");
    let cyan_terracotta_texture = asset_server.load("textures/cyan_terracotta.webp");
    let esp_logo_texture = asset_server.load("textures/espressif.webp");

    // Log asset loading for debugging
    info!(
        "Loading textures from base path. If textures don't appear, check browser console for 404 errors."
    );

    let grass_material = materials.add(StandardMaterial {
        base_color_texture: Some(grass_texture.clone()),
        ..default()
    });
    let dirt_material = materials.add(StandardMaterial {
        base_color_texture: Some(dirt_texture),
        ..default()
    });
    let stone_material = materials.add(StandardMaterial {
        base_color_texture: Some(stone_texture),
        ..default()
    });
    let quartz_material = materials.add(StandardMaterial {
        base_color_texture: Some(quartz_texture),
        ..default()
    });
    let glass_material = materials.add(StandardMaterial {
        base_color_texture: Some(glass_texture),
        ..default()
    });
    let cyan_material = materials.add(StandardMaterial {
        base_color_texture: Some(cyan_terracotta_texture),
        ..default()
    });
    let water_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.0, 0.35, 0.9, 0.6),
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let esp_logo_material = materials.add(StandardMaterial {
        base_color_texture: Some(esp_logo_texture),
        base_color: Color::WHITE,
        ..default()
    });

    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let _block_size = 1.0;

    // Build the world based on background_world.txt script
    // Create a grass base (-15 to 15 in x and z)
    info!("Building grass terrain...");
    for x in -15..=15 {
        for z in -15..=15 {
            commands.spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(grass_material.clone()),
                Transform::from_translation(Vec3::new(x as f32, 0.0, z as f32)),
                Ground,
            ));
        }
    }

    // Create rolling hills - Hill 1 (dirt base)
    info!("Building rolling hills...");
    for x in -10..=-5 {
        for y in 1..=2 {
            for z in -10..=-5 {
                commands.spawn((
                    Mesh3d(cube_mesh.clone()),
                    MeshMaterial3d(dirt_material.clone()),
                    Transform::from_translation(Vec3::new(x as f32, y as f32, z as f32)),
                ));
            }
        }
    }
    // Grass top of hill 1
    for x in -10..=-5 {
        for z in -10..=-5 {
            commands.spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(grass_material.clone()),
                Transform::from_translation(Vec3::new(x as f32, 3.0, z as f32)),
            ));
        }
    }

    // Hill 2 (dirt base)
    for x in 5..=10 {
        for y in 1..=3 {
            for z in 5..=10 {
                commands.spawn((
                    Mesh3d(cube_mesh.clone()),
                    MeshMaterial3d(dirt_material.clone()),
                    Transform::from_translation(Vec3::new(x as f32, y as f32, z as f32)),
                ));
            }
        }
    }
    // Grass top of hill 2
    for x in 5..=10 {
        for z in 5..=10 {
            commands.spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(grass_material.clone()),
                Transform::from_translation(Vec3::new(x as f32, 4.0, z as f32)),
            ));
        }
    }

    // Add variety blocks for visual interest
    info!("Adding decorative elements...");
    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(stone_material.clone()),
        Transform::from_translation(Vec3::new(-8.0, 1.0, 8.0)),
    ));

    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(quartz_material.clone()),
        Transform::from_translation(Vec3::new(8.0, 1.0, -8.0)),
    ));

    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(glass_material.clone()),
        Transform::from_translation(Vec3::new(0.0, 1.0, 12.0)),
    ));

    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(cyan_material.clone()),
        Transform::from_translation(Vec3::new(12.0, 1.0, 0.0)),
    ));

    // Create a small tower for interest (stone tower with quartz top)
    info!("Building central tower...");
    for y in 1..=5 {
        commands.spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(stone_material.clone()),
            Transform::from_translation(Vec3::new(0.0, y as f32, 0.0)),
        ));
    }
    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(quartz_material.clone()),
        Transform::from_translation(Vec3::new(0.0, 6.0, 0.0)),
    ));

    // Add a spinning Espressif logo cube (like the original)
    commands.spawn((
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(esp_logo_material),
        Transform::from_translation(Vec3::new(3.0, 6.5, 2.0)),
        DemoCube, // This will make it rotate
    ));

    // Create a small water pond in one corner
    info!("Adding water features...");
    // Water pond walls (stone)
    for x in 21..=26 {
        // North and south walls
        commands.spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(stone_material.clone()),
            Transform::from_translation(Vec3::new(x as f32, 1.0, -26.0)),
        ));
        commands.spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(stone_material.clone()),
            Transform::from_translation(Vec3::new(x as f32, 1.0, -21.0)),
        ));
    }
    for z in -26..=-21 {
        // East and west walls
        commands.spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(stone_material.clone()),
            Transform::from_translation(Vec3::new(21.0, 1.0, z as f32)),
        ));
        commands.spawn((
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(stone_material.clone()),
            Transform::from_translation(Vec3::new(26.0, 1.0, z as f32)),
        ));
    }

    // Water inside the pond
    for x in 22..=25 {
        for z in -25..=-22 {
            commands.spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(water_material.clone()),
                Transform::from_translation(Vec3::new(x as f32, 1.0, z as f32)),
            ));
        }
    }

    // Add some IoT devices around the scene
    info!("Placing IoT devices...");
    let device_material = materials.add(Color::srgb(1.0, 0.7, 0.2));
    let small_cube = meshes.add(Cuboid::new(0.5, 0.5, 0.5));

    // Place devices at strategic locations
    let device_locations = [
        Vec3::new(-7.0, 1.25, -7.0),
        Vec3::new(7.0, 1.25, 7.0),
        Vec3::new(-5.0, 1.25, 10.0),
        Vec3::new(10.0, 1.25, -5.0),
        Vec3::new(2.0, 1.25, 2.0),
        Vec3::new(-3.0, 1.25, -3.0),
    ];

    for location in device_locations {
        commands.spawn((
            Mesh3d(small_cube.clone()),
            MeshMaterial3d(device_material.clone()),
            Transform::from_translation(location),
        ));
    }

    // Add a thermometer-like indicator
    let thermo_mesh = meshes.add(Cuboid::new(0.2, 5.0, 0.2));
    let thermo_material = materials.add(Color::srgb(0.2, 0.2, 0.2));
    commands.spawn((
        Mesh3d(thermo_mesh),
        MeshMaterial3d(thermo_material),
        Transform::from_translation(Vec3::new(-3.0, 2.5, 2.0)),
    ));

    info!(
        "IoTCraft Enhanced Web Scene completed! Total blocks: ~700+ | Features: Terrain, Hills, Water, Devices, Tower"
    );
}
fn rotate_cube(time: Res<Time>, mut query: Query<&mut Transform, With<DemoCube>>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() * 0.5);
    }
}

/// Camera control system for web
fn camera_control_system(
    time: Res<Time>,
    mut camera_controller: ResMut<CameraController>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    mut windows: Query<&mut Window>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut cursor_moved_events: EventReader<CursorMoved>,
) {
    if !camera_controller.enabled {
        return;
    }

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let mut velocity = Vec3::ZERO;
    let dt = time.delta_secs();

    // Arrow key camera rotation (backup for mouse look)
    let rotation_speed = 16.0f32; // degrees per frame when held (doubled from 8.0)
    let mut yaw_change = 0.0f32;
    let mut pitch_change = 0.0f32;

    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        yaw_change += rotation_speed;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) {
        yaw_change -= rotation_speed;
    }
    if keyboard_input.pressed(KeyCode::ArrowUp) {
        pitch_change += rotation_speed;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        pitch_change -= rotation_speed;
    }

    // Apply arrow key rotation
    if yaw_change != 0.0 || pitch_change != 0.0 {
        camera_controller.yaw += yaw_change.to_radians() * dt;
        camera_controller.pitch = (camera_controller.pitch + pitch_change.to_radians() * dt)
            .clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);

        camera_transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            camera_controller.yaw,
            camera_controller.pitch,
        );
    }

    // Handle WASD movement
    if keyboard_input.pressed(KeyCode::KeyW) {
        velocity += camera_transform.forward().as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        velocity -= camera_transform.forward().as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        velocity -= camera_transform.right().as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        velocity += camera_transform.right().as_vec3();
    }
    if keyboard_input.pressed(KeyCode::Space) {
        velocity += Vec3::Y;
    }
    if keyboard_input.pressed(KeyCode::ControlLeft) || keyboard_input.pressed(KeyCode::ShiftLeft) {
        velocity -= Vec3::Y;
    }

    // Apply movement
    if velocity != Vec3::ZERO {
        camera_transform.translation += velocity.normalize() * camera_controller.speed * dt;
    }

    // Handle mouse look (only if we have delta information)
    for cursor_event in cursor_moved_events.read() {
        if let Some(delta) = cursor_event.delta {
            let delta_x = delta.x * camera_controller.sensitivity * dt;
            let delta_y = delta.y * camera_controller.sensitivity * dt;

            camera_controller.yaw -= delta_x * 0.01;
            camera_controller.pitch -= delta_y * 0.01;

            // Clamp pitch
            camera_controller.pitch = camera_controller.pitch.clamp(
                -std::f32::consts::FRAC_PI_2 * 0.9,
                std::f32::consts::FRAC_PI_2 * 0.9,
            );

            // Apply rotation
            camera_transform.rotation = Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                camera_controller.yaw,
                camera_controller.pitch,
            );
        }
    }

    // Handle mouse capture (escape key is handled by menu system)
    for mut window in &mut windows {
        if mouse_button_input.just_pressed(MouseButton::Left) && window.focused {
            window.cursor_options.grab_mode = bevy::window::CursorGrabMode::Locked;
            window.cursor_options.visible = false;
            info!(
                "Mouse captured for camera control. Use WASD to move, mouse to look around. Press Escape to open menu."
            );
        }
    }
}

/// Process device announcements received via MQTT and spawn devices visually
fn process_device_announcements(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    device_receiver: Option<Res<DeviceAnnouncementReceiver>>,
    existing_devices: Query<(Entity, &DeviceEntity)>,
) {
    let Some(receiver) = device_receiver else {
        return; // No DeviceAnnouncementReceiver resource available yet
    };

    let Ok(rx) = receiver.0.lock() else {
        return;
    };

    // Process all available device announcements
    while let Ok(device_msg) = rx.try_recv() {
        web_sys::console::log_1(
            &format!("Web: Processing device announcement: {}", device_msg).into(),
        );
        info!("Web: Processing device announcement: {}", device_msg);

        // Parse the JSON device announcement
        if let Ok(device_data) = serde_json::from_str::<Value>(&device_msg) {
            if let (Some(device_id), Some(device_type_str), Some(state), Some(location)) = (
                device_data["device_id"].as_str(),
                device_data["device_type"].as_str(),
                device_data["state"].as_str(),
                device_data["location"].as_object(),
            ) {
                match state {
                    "online" => {
                        // Handle device registration/online announcement
                        if let Some(device_type) = DeviceType::from_str(device_type_str) {
                            // Check if device already exists
                            let device_exists = existing_devices
                                .iter()
                                .any(|(_, dev)| dev.device_id == device_id);

                            if !device_exists {
                                info!(
                                    "🔌 Web: Registering new device: {} ({})",
                                    device_id, device_type_str
                                );

                                // Extract location coordinates
                                let x = location["x"].as_f64().unwrap_or(0.0) as f32;
                                let y = location["y"].as_f64().unwrap_or(0.5) as f32;
                                let z = location["z"].as_f64().unwrap_or(0.0) as f32;

                                // Choose material based on device type
                                let material = match device_type {
                                    DeviceType::Lamp => {
                                        let lamp_texture: Handle<Image> =
                                            asset_server.load("textures/lamp.webp");
                                        materials.add(StandardMaterial {
                                            base_color_texture: Some(lamp_texture),
                                            base_color: Color::srgb(0.2, 0.2, 0.2),
                                            ..default()
                                        })
                                    }
                                    DeviceType::Door => {
                                        let door_texture: Handle<Image> =
                                            asset_server.load("textures/door.webp");
                                        materials.add(StandardMaterial {
                                            base_color_texture: Some(door_texture),
                                            base_color: Color::srgb(0.8, 0.6, 0.4), // Wood-like brown when closed
                                            ..default()
                                        })
                                    }
                                    DeviceType::Sensor => materials.add(StandardMaterial {
                                        base_color: Color::srgb(0.2, 0.8, 1.0),
                                        ..default()
                                    }),
                                };

                                // Create mesh based on device type dimensions
                                let (width, height, depth) = device_type.mesh_dimensions();
                                let mesh = meshes.add(Cuboid::new(width, height, depth));

                                // Spawn the device entity
                                let _device_entity = commands.spawn((
                                    Mesh3d(mesh),
                                    MeshMaterial3d(material),
                                    Transform::from_translation(Vec3::new(x, y, z)),
                                    DeviceEntity {
                                        device_id: device_id.to_string(),
                                        device_type: device_type.as_str().to_string(),
                                    },
                                    Name::new(format!("Web-Device-{}", device_id)),
                                ));

                                info!(
                                    "✅ Web: Spawned device: {} of type {} at ({}, {}, {})",
                                    device_id,
                                    device_type.as_str(),
                                    x,
                                    y,
                                    z
                                );
                                web_sys::console::log_1(
                                    &format!(
                                        "✅ Web: Spawned device: {} of type {} at ({}, {}, {})",
                                        device_id,
                                        device_type.as_str(),
                                        x,
                                        y,
                                        z
                                    )
                                    .into(),
                                );
                            } else {
                                info!(
                                    "⚠️ Web: Device {} already registered, ignoring duplicate announcement",
                                    device_id
                                );
                            }
                        } else {
                            info!("❓ Web: Unknown device type: {}", device_type_str);
                        }
                    }
                    "offline" => {
                        // Handle device deregistration/offline announcement
                        info!(
                            "🔌 Web: Device {} going offline, removing from world",
                            device_id
                        );

                        // Find and despawn the device entity
                        for (entity, device_entity) in existing_devices.iter() {
                            if device_entity.device_id == device_id {
                                commands.entity(entity).despawn();
                                info!("🗑️ Web: Removed device {} from 3D world", device_id);
                                break;
                            }
                        }
                    }
                    _ => {
                        info!(
                            "❓ Web: Unknown device state: {} for device {}",
                            state, device_id
                        );
                    }
                }
            } else {
                info!("⚠️ Web: Invalid device announcement format: missing required fields");
            }
        } else {
            warn!(
                "Web: Failed to parse device announcement JSON: {}",
                device_msg
            );
        }
    }
}

/// Web-compatible FPS logging
#[cfg(target_arch = "wasm32")]
fn log_fps(time: Res<Time>, mut timer: Local<Timer>) {
    // Initialize timer to log every 10 seconds (less frequent for web)
    if timer.duration() == std::time::Duration::ZERO {
        *timer = Timer::from_seconds(10.0, TimerMode::Repeating);
    }

    if timer.tick(time.delta()).just_finished() {
        let fps = 1.0 / time.delta_secs();
        web_sys::console::log_1(&format!("IoTCraft Web FPS: {:.1}", fps).into());
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn log_fps(_time: Res<Time>) {
    // No-op for non-wasm targets
}

// ============ MULTIPLAYER SYSTEMS ============

/// Setup multiplayer connections (simplified for web)
fn setup_multiplayer_connections(
    mut commands: Commands,
    profile: Res<crate::profile::PlayerProfile>,
) {
    info!("🌐 Initializing web multiplayer...");

    // Enable multiplayer - PoseReceiver and PoseSender are already set up by the web MQTT plugin
    // We just need to enable the connection status
    commands.insert_resource(MultiplayerConnectionStatus {
        connection_available: true,
    });

    info!(
        "🌐 Web multiplayer initialized for player {}",
        profile.player_name
    );
    web_sys::console::log_1(
        &format!(
            "🌐 Web multiplayer initialized for player {}",
            profile.player_name
        )
        .into(),
    );
}

/// Update position timer
fn update_position_timer(mut timer: ResMut<PositionTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

/// Publish local player pose (simplified for web)
fn publish_local_pose(
    profile: Res<crate::profile::PlayerProfile>,
    mut timer: ResMut<PositionTimer>,
    pose_sender: Option<Res<PoseSender>>,
    camera_query: Query<&Transform, With<Camera3d>>,
    connection_status: Res<MultiplayerConnectionStatus>,
) {
    if !timer.timer.just_finished() {
        return;
    }

    // Don't publish poses if multiplayer is disabled
    if !connection_status.connection_available {
        return;
    }

    let Some(sender) = pose_sender else {
        return;
    };

    let Ok(transform) = camera_query.single() else {
        return;
    };

    let current_position = transform.translation;

    // Always send initial pose, then only if position changed significantly
    let should_send = match timer.last_position {
        Some(last_pos) => current_position.distance(last_pos) > 0.01, // 1cm threshold
        None => true,                                                 // Always send first time
    };

    if !should_send {
        return;
    }

    timer.last_position = Some(current_position);

    // Extract yaw/pitch from rotation
    let forward = transform.forward();
    let yaw = forward.x.atan2(forward.z);
    let pitch = forward.y.asin();

    let msg = PoseMessage {
        player_id: profile.player_id.clone(),
        player_name: profile.player_name.clone(),
        pos: [current_position.x, current_position.y, current_position.z],
        yaw,
        pitch,
        ts: now_ts(),
    };

    // Send pose message to MQTT system for publishing
    if let Ok(tx) = sender.0.lock() {
        if let Err(_) = tx.send(msg.clone()) {
            error!("Failed to send pose message to MQTT publisher");
        } else {
            info!(
                "📡 Web: Publishing pose for {} at [{:.2}, {:.2}, {:.2}]",
                msg.player_name, msg.pos[0], msg.pos[1], msg.pos[2]
            );
        }
    }
}

/// Apply remote poses to spawn/update player avatars
fn apply_remote_poses(
    profile: Res<crate::profile::PlayerProfile>,
    pose_receiver: Option<Res<PoseReceiver>>,
    mut commands: Commands,
    mut remote_players: Query<(&mut Transform, &PlayerAvatar), With<RemotePlayer>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    connection_status: Res<MultiplayerConnectionStatus>,
) {
    // Don't process remote poses if multiplayer is disabled
    if !connection_status.connection_available {
        return;
    }

    let Some(receiver) = pose_receiver else {
        return;
    };

    let Ok(rx) = receiver.0.lock() else {
        return;
    };

    // Process all available messages
    while let Ok(msg) = rx.try_recv() {
        // Ignore our own messages
        if msg.player_id == profile.player_id {
            continue;
        }

        // Try to update existing remote player avatar
        let mut updated = false;
        for (mut transform, player_avatar) in remote_players.iter_mut() {
            if player_avatar.player_id == msg.player_id {
                transform.translation = Vec3::new(msg.pos[0], msg.pos[1], msg.pos[2]);
                transform.rotation = Quat::from_rotation_y(msg.yaw);
                updated = true;
                break;
            }
        }

        // Spawn new remote player avatar if not found
        if !updated {
            let position = Vec3::new(msg.pos[0], msg.pos[1], msg.pos[2]);
            spawn_simple_player_avatar(
                &mut commands,
                &mut meshes,
                &mut materials,
                position,
                msg.player_id.clone(),
                msg.player_name.clone(),
            );

            info!(
                "👤 Web: New remote player joined: {} ({})",
                msg.player_name, msg.player_id
            );
            web_sys::console::log_1(
                &format!(
                    "👤 Web: New remote player joined: {} ({})",
                    msg.player_name, msg.player_id
                )
                .into(),
            );
        }
    }
}

/// Spawn a simple player avatar for web (simplified version of desktop avatar)
fn spawn_simple_player_avatar(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    player_id: String,
    player_name: String,
) {
    // Create a simple colorful avatar (simplified from desktop version)
    let head_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.96, 0.8, 0.69), // Skin color
        ..default()
    });

    let body_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.4, 0.8), // Blue shirt
        ..default()
    });

    // Simple avatar dimensions
    let head_size = 0.4;
    let body_width = 0.35;
    let body_height = 0.6;
    let body_depth = 0.2;

    let head_mesh = meshes.add(Cuboid::new(head_size, head_size, head_size));
    let body_mesh = meshes.add(Cuboid::new(body_width, body_height, body_depth));

    // Spawn the main avatar entity
    let avatar_entity = commands
        .spawn((
            Transform::from_translation(position),
            GlobalTransform::default(),
            PlayerAvatar {
                player_id: player_id.clone(),
                player_name: player_name.clone(),
            },
            RemotePlayer,
            Name::new(format!("WebPlayerAvatar-{}", player_name)),
            Visibility::default(),
        ))
        .id();

    // Spawn head
    let head_entity = commands
        .spawn((
            Mesh3d(head_mesh),
            MeshMaterial3d(head_material),
            Transform::from_translation(Vec3::new(0.0, body_height / 2.0 + head_size / 2.0, 0.0)),
            Name::new("Head"),
        ))
        .id();

    // Spawn body
    let body_entity = commands
        .spawn((
            Mesh3d(body_mesh),
            MeshMaterial3d(body_material),
            Transform::from_translation(Vec3::ZERO),
            Name::new("Body"),
        ))
        .id();

    // Set up parent-child relationships
    commands
        .entity(avatar_entity)
        .add_children(&[head_entity, body_entity]);

    info!(
        "👤 Web: Spawned simple avatar for player {} at {:?}",
        player_name, position
    );
}
