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
pub struct DeviceEntity {
    pub device_id: String,
    pub device_type: String,
}

// ============ MULTIPLAYER COMPONENTS & RESOURCES ============

/// Component to mark remote player entities
#[derive(Component)]
pub struct RemotePlayer;

// Multiplayer types are imported from either web MQTT or desktop multiplayer modules

/// Multiplayer connection status
#[derive(Resource, Default)]
pub struct MultiplayerConnectionStatus {
    pub connection_available: bool,
}

/// Timer for position updates (10 Hz)
#[derive(Resource)]
pub struct PositionTimer {
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
pub struct WorldId(pub String);

impl Default for WorldId {
    fn default() -> Self {
        Self("default".to_string())
    }
}

// ============ TOUCH CONTROL SYSTEMS ============

/// Setup touch control areas based on screen size
pub fn setup_touch_areas(mut touch_state: ResMut<TouchInputState>, windows: Query<&Window>) {
    if let Ok(window) = windows.single() {
        let window_width = window.resolution.width();
        let window_height = window.resolution.height();

        // Virtual joystick center (bottom-left quarter)
        touch_state.joystick_center = Vec2::new(window_width * 0.15, window_height * 0.85);

        // Look area starts from center of screen to the right
        touch_state.look_area_min_x = window_width * 0.5;

        info!(
            "📱 Touch areas initialized: joystick at {:?}, look area x >= {:.1}",
            touch_state.joystick_center, touch_state.look_area_min_x
        );

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!(
            "📱 Touch controls ready: Movement (left side), Look (right side), Joystick center: {:?}",
            touch_state.joystick_center
        ).into());
    }
}

/// Enhanced camera control system with continuous touch support for mobile devices
pub fn touch_camera_control_system(
    time: Res<Time>,
    mut camera_controller: ResMut<CameraController>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    mut windows: Query<&mut Window>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut touch_events: EventReader<TouchInput>,
    mut touch_state: ResMut<TouchInputState>,
) {
    if !camera_controller.enabled {
        return;
    }

    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let mut velocity = Vec3::ZERO;
    let dt = time.delta_secs();

    // ============ PROCESS TOUCH EVENTS ============

    for touch in touch_events.read() {
        let touch_pos = touch.position;

        match touch.phase {
            bevy::input::touch::TouchPhase::Started => {
                // Dynamically update screen split based on current window size
                if let Ok(window) = windows.single() {
                    let current_width = window.resolution.width();
                    touch_state.look_area_min_x = current_width * 0.5;
                    touch_state.screen_width = current_width;
                    touch_state.screen_height = window.resolution.height();
                }

                // Determine touch area: left half = movement, right half = look
                let is_left_side = touch_pos.x < touch_state.look_area_min_x;

                info!(
                    "📱 Touch at {:?}, screen_width={:.1}, split_x={:.1}, is_left={}",
                    touch_pos, touch_state.screen_width, touch_state.look_area_min_x, is_left_side
                );

                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!(
                    "📱 Touch debug: pos=({:.1},{:.1}), screen={:.1}x{:.1}, split_x={:.1}, left_side={}", 
                    touch_pos.x, touch_pos.y, touch_state.screen_width, touch_state.screen_height,
                    touch_state.look_area_min_x, is_left_side
                ).into());

                if is_left_side {
                    // Movement touch (left side)
                    if touch_state.move_touch_id.is_none() {
                        touch_state.move_touch_id = Some(touch.id);
                        touch_state.joystick_active = true;
                        // Update joystick center to where user touched (floating joystick)
                        touch_state.joystick_center = touch_pos;
                        touch_state.joystick_offset = Vec2::ZERO;
                        info!("📱 Movement joystick activated at {:?}", touch_pos);

                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(&"📱 Movement joystick activated".into());
                    }
                } else {
                    // Look touch (right side) - NO joystick visual, just camera rotation
                    if touch_state.look_touch_id.is_none() {
                        touch_state.look_touch_id = Some(touch.id);
                        touch_state.last_look_position = Some(touch_pos);
                        touch_state.look_delta_accumulator = Vec2::ZERO;
                        info!(
                            "🎮 Look control started at {:?} (no joystick visual)",
                            touch_pos
                        );

                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(
                            &"🎮 Look control activated - camera rotation only".into(),
                        );
                    }
                }
            }
            bevy::input::touch::TouchPhase::Moved => {
                if Some(touch.id) == touch_state.move_touch_id {
                    // Update joystick offset from center
                    touch_state.joystick_offset = touch_pos - touch_state.joystick_center;
                } else if Some(touch.id) == touch_state.look_touch_id {
                    // Accumulate look delta for smooth rotation
                    if let Some(last_pos) = touch_state.last_look_position {
                        let delta = touch_pos - last_pos;
                        touch_state.look_delta_accumulator += delta;
                        touch_state.last_look_position = Some(touch_pos);

                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(
                            &format!(
                                "🎮 Look move: delta=({:.2},{:.2}), accumulator=({:.2},{:.2})",
                                delta.x,
                                delta.y,
                                touch_state.look_delta_accumulator.x,
                                touch_state.look_delta_accumulator.y
                            )
                            .into(),
                        );
                    }
                }
            }
            bevy::input::touch::TouchPhase::Ended | bevy::input::touch::TouchPhase::Canceled => {
                let phase_name = if matches!(touch.phase, bevy::input::touch::TouchPhase::Ended) {
                    "ENDED"
                } else {
                    "CANCELED"
                };

                if Some(touch.id) == touch_state.move_touch_id {
                    // Deactivate movement joystick
                    touch_state.move_touch_id = None;
                    touch_state.joystick_active = false;
                    touch_state.joystick_offset = Vec2::ZERO;
                    info!("📱 Movement joystick deactivated ({})", phase_name);

                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::log_1(
                        &format!("📱 Movement joystick deactivated ({})", phase_name).into(),
                    );
                } else if Some(touch.id) == touch_state.look_touch_id {
                    // Deactivate look control
                    touch_state.look_touch_id = None;
                    touch_state.last_look_position = None;
                    touch_state.look_delta_accumulator = Vec2::ZERO;
                    info!("📱 Look control deactivated ({})", phase_name);

                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::log_1(
                        &format!("📱 Look control deactivated ({})", phase_name).into(),
                    );
                } else {
                    // Unknown touch ended - log for debugging
                    #[cfg(target_arch = "wasm32")]
                    web_sys::console::log_1(
                        &format!(
                            "📱 Unknown touch {} - ID: {}, Active Move: {:?}, Active Look: {:?}",
                            phase_name,
                            touch.id,
                            touch_state.move_touch_id,
                            touch_state.look_touch_id
                        )
                        .into(),
                    );
                }
            }
        }
    }

    // ============ CONTINUOUS TOUCH MOVEMENT ============

    if touch_state.joystick_active {
        let joystick_distance = touch_state.joystick_offset.length();
        let max_distance = 80.0; // Maximum joystick radius
        let dead_zone = 15.0; // Dead zone radius

        if joystick_distance > dead_zone {
            // Calculate movement direction and strength
            let normalized_offset = touch_state.joystick_offset / joystick_distance;
            let movement_strength =
                ((joystick_distance - dead_zone) / (max_distance - dead_zone)).min(1.0);

            // Apply movement based on joystick direction
            // Note: Y is inverted for touch (up = negative Y)
            let forward_input = -normalized_offset.y; // Invert Y for forward/back
            let right_input = normalized_offset.x;

            // Add movement velocity
            velocity += camera_transform.forward().as_vec3() * forward_input * movement_strength;
            velocity += camera_transform.right().as_vec3() * right_input * movement_strength;

            // Visual feedback - clamp joystick to max distance
            if joystick_distance > max_distance {
                touch_state.joystick_offset = normalized_offset * max_distance;
            }
        }
    }

    // ============ CONTINUOUS LOOK ROTATION ============

    if touch_state.look_delta_accumulator != Vec2::ZERO {
        let sensitivity = camera_controller.touch_sensitivity * 0.0025; // Reduced sensitivity by 75% total

        // Apply accumulated rotation to camera controller
        let old_yaw = camera_controller.yaw;
        let old_pitch = camera_controller.pitch;

        camera_controller.yaw -= touch_state.look_delta_accumulator.x * sensitivity;
        camera_controller.pitch -= touch_state.look_delta_accumulator.y * sensitivity;

        // Clamp pitch to prevent over-rotation
        camera_controller.pitch = camera_controller.pitch.clamp(
            -std::f32::consts::FRAC_PI_2 * 0.9,
            std::f32::consts::FRAC_PI_2 * 0.9,
        );

        // Smooth decay of accumulated delta for natural feel
        touch_state.look_delta_accumulator *= 0.7; // Faster decay for better responsiveness

        // Clear very small deltas to prevent jitter
        if touch_state.look_delta_accumulator.length() < 0.1 {
            touch_state.look_delta_accumulator = Vec2::ZERO;
        }

        info!(
            "🎮 Look control: yaw={:.3} (Δ{:.3}), pitch={:.3} (Δ{:.3}), delta={:?}",
            camera_controller.yaw,
            camera_controller.yaw - old_yaw,
            camera_controller.pitch,
            camera_controller.pitch - old_pitch,
            touch_state.look_delta_accumulator
        );

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!(
                "🎮 Camera rotation: yaw={:.3}, pitch={:.3}",
                camera_controller.yaw, camera_controller.pitch
            )
            .into(),
        );
    }

    // ============ KEYBOARD INPUT (for desktop compatibility) ============

    // Arrow key camera rotation (backup for mouse look)
    let rotation_speed = 64.0f32; // degrees per frame when held (4x original for web browser)
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
    }

    // Handle WASD movement (desktop)
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

    // ============ MOUSE INPUT (for desktop compatibility) ============

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
        }
    }

    // ============ APPLY TRANSFORMS ============

    // Apply rotation changes from all input sources
    camera_transform.rotation = Quat::from_euler(
        EulerRot::ZYX,
        0.0,
        camera_controller.yaw,
        camera_controller.pitch,
    );

    // Apply movement
    if velocity != Vec3::ZERO {
        camera_transform.translation += velocity.normalize() * camera_controller.speed * dt;
    }

    // Handle mouse capture (escape key is handled by menu system)
    for mut window in &mut windows {
        if mouse_button_input.just_pressed(MouseButton::Left) && window.focused {
            safe_set_cursor_grab_mode(&mut window, bevy::window::CursorGrabMode::Locked, false);
            info!(
                "Mouse captured for camera control. Use WASD to move, mouse to look around. Touch controls: left side = move, right side = look."
            );
        }
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

/// Detect if we're running on a mobile/touch device
#[cfg(target_arch = "wasm32")]
fn is_mobile_device() -> bool {
    if let Some(window) = web_sys::window() {
        let navigator = window.navigator();
        if let Ok(user_agent) = navigator.user_agent() {
            let ua = user_agent.to_lowercase();
            let is_mobile = ua.contains("mobile")
                || ua.contains("android")
                || ua.contains("iphone")
                || ua.contains("ipad")
                || ua.contains("ipod")
                || ua.contains("blackberry")
                || ua.contains("webos");
            if is_mobile {
                return true;
            }
        }

        // Also check for touch support
        if let Ok(has_touch) = js_sys::Reflect::has(&window, &"ontouchstart".into()) {
            if has_touch {
                return true;
            }
        }
    }
    false
}

#[cfg(not(target_arch = "wasm32"))]
fn is_mobile_device() -> bool {
    false // Desktop platforms are never mobile
}

/// Safely set cursor grab mode, avoiding requestPointerLock on mobile devices
pub fn safe_set_cursor_grab_mode(
    window: &mut Window,
    grab_mode: bevy::window::CursorGrabMode,
    visible: bool,
) {
    if grab_mode == bevy::window::CursorGrabMode::Locked && is_mobile_device() {
        info!("📱 Skipping cursor lock on mobile device - using touch controls instead");
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &"📱 Skipping cursor lock on mobile device - using touch controls instead".into(),
        );
        return;
    }

    window.cursor_options.grab_mode = grab_mode;
    window.cursor_options.visible = visible;

    match grab_mode {
        bevy::window::CursorGrabMode::Locked => {
            info!("🖱️ Mouse cursor locked for camera control");
        }
        bevy::window::CursorGrabMode::None => {
            info!("🖱️ Mouse cursor released");
        }
        _ => {}
    }
}

/// Set up enhanced panic hook for better error reporting in web console
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let mut msg = String::new();

        if let Some(payload) = info.payload().downcast_ref::<&str>() {
            msg.push_str(&format!("💥 PANIC: {}", payload));
        } else if let Some(payload) = info.payload().downcast_ref::<String>() {
            msg.push_str(&format!("💥 PANIC: {}", payload));
        } else {
            msg.push_str("💥 PANIC: (no message)");
        }

        if let Some(location) = info.location() {
            msg.push_str(&format!(
                " at {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            ));
        }

        // Log to both console and web_sys
        eprintln!("{}", msg);
        web_sys::console::error_1(&msg.clone().into());

        // Try to show error in the UI if possible
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(element) = document.get_element_by_id("errorMessage") {
                    element.set_text_content(Some(&format!("WASM Panic: {}", msg)));
                }
                if let Some(error_div) = document.get_element_by_id("error") {
                    if let Some(html_element) = error_div.dyn_ref::<web_sys::HtmlElement>() {
                        let _ = html_element.style().set_property("display", "block");
                    }
                }
            }
        }
    }));

    web_sys::console::log_1(&"Enhanced panic hook initialized for IoTCraft".into());
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
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "IoTCraft Desktop Client - Web Version".to_string(),
                        resolution: (1280.0, 720.0).into(),
                        canvas: Some("#canvas".to_owned()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false, // Allow browser events for better iPad compatibility
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    watch_for_changes_override: Some(false), // Disable asset watching for web
                    ..default()
                }),
        )
        // Insert resources BEFORE adding plugins that depend on them
        .insert_resource(MqttConfig::from_web_env())
        .insert_resource(crate::profile::load_or_create_profile_with_override(None))
        .add_plugins(WebMenuPlugin)
        .add_plugins(MqttPlugin) // MQTT connection working!
        .add_plugins(crate::player_avatar::PlayerAvatarPlugin) // Add avatar animations
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .insert_resource(CameraController::new())
        .insert_resource(TouchInputState::default())
        // Multiplayer resources
        .insert_resource(WorldId::default())
        .insert_resource(MultiplayerConnectionStatus::default())
        .insert_resource(PositionTimer::default())
        .add_systems(
            Startup,
            (
                setup_basic_scene,
                setup_multiplayer_connections,
                setup_touch_areas,
                setup_touch_ui,
            ),
        )
        .add_systems(
            Update,
            (
                rotate_cube,
                touch_camera_control_system.run_if(in_state(WebGameState::InGame)),
                update_touch_ui.run_if(in_state(WebGameState::InGame)),
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
pub struct DemoCube;

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
    pub touch_sensitivity: f32,
}

/// Touch input state for mobile controls with joystick behavior
#[derive(Resource, Default)]
pub struct TouchInputState {
    /// Current touch for look control (camera rotation)
    look_touch_id: Option<u64>,
    last_look_position: Option<Vec2>,
    /// Current touch for movement control
    move_touch_id: Option<u64>,
    /// Current joystick position relative to center
    joystick_offset: Vec2,
    /// Virtual joystick center (dynamically set where user touches)
    joystick_center: Vec2,
    /// Look area (right side of screen)
    look_area_min_x: f32,
    /// Whether joystick is currently active
    joystick_active: bool,
    /// Accumulated look delta for smooth rotation with decay
    look_delta_accumulator: Vec2,
    /// Screen dimensions for UI positioning
    screen_width: f32,
    screen_height: f32,
}

/// UI components for touch controls
#[derive(Component)]
struct TouchControlsUI;

#[derive(Component)]
pub struct JoystickBase;

#[derive(Component)]
pub struct JoystickKnob;

#[derive(Component)]
struct TouchZoneIndicator;

#[derive(Component)]
struct TouchInstructions;

impl CameraController {
    pub fn new() -> Self {
        Self {
            enabled: false, // Start disabled - menu system will enable it
            sensitivity: 2.0,
            speed: 5.0,
            yaw: 0.0,
            pitch: 0.0,
            touch_sensitivity: 3.0, // Higher sensitivity for touch
        }
    }
}

pub fn setup_basic_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    info!("Setting up enhanced IoTCraft world scene...");

    // Add a camera positioned like in the original desktop client with explicit order 0
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0, // Render first (3D scene)
            ..default()
        },
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
pub fn rotate_cube(time: Res<Time>, mut query: Query<&mut Transform, With<DemoCube>>) {
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
            safe_set_cursor_grab_mode(&mut window, bevy::window::CursorGrabMode::Locked, false);
            info!(
                "Mouse captured for camera control. Use WASD to move, mouse to look around. Press Escape to open menu."
            );
        }
    }
}

/// Process device announcements received via MQTT and spawn devices visually
pub fn process_device_announcements(
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
pub fn log_fps(time: Res<Time>, mut timer: Local<Timer>) {
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
pub fn setup_multiplayer_connections(
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
pub fn update_position_timer(mut timer: ResMut<PositionTimer>, time: Res<Time>) {
    timer.timer.tick(time.delta());
}

/// Publish local player pose (simplified for web)
pub fn publish_local_pose(
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
pub fn apply_remote_poses(
    profile: Res<crate::profile::PlayerProfile>,
    pose_receiver: Option<Res<PoseReceiver>>,
    mut commands: Commands,
    mut remote_players: Query<
        (&mut Transform, &crate::player_avatar::PlayerAvatar),
        With<RemotePlayer>,
    >,
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
        info!(
            "📡 Web: Processing pose message from {}: {:?}",
            msg.player_name, msg.pos
        );

        // Ignore our own messages
        if msg.player_id == profile.player_id {
            info!(
                "📡 Web: Ignoring our own pose message from {}",
                msg.player_name
            );
            continue;
        }

        // Try to update existing remote player avatar
        let mut updated = false;
        for (mut transform, player_avatar) in remote_players.iter_mut() {
            if player_avatar.player_id == msg.player_id {
                let old_pos = transform.translation;
                transform.translation = Vec3::new(msg.pos[0], msg.pos[1], msg.pos[2]);
                transform.rotation = Quat::from_rotation_y(msg.yaw);
                updated = true;
                info!(
                    "👤 Web: Updated existing avatar for {} from {:?} to {:?}",
                    msg.player_name, old_pos, transform.translation
                );
                break;
            }
        }

        // Spawn new remote player avatar if not found
        if !updated {
            let position = Vec3::new(msg.pos[0], msg.pos[1], msg.pos[2]);

            // Use the unified desktop avatar spawning function
            let avatar_entity = crate::player_avatar::spawn_player_avatar(
                &mut commands,
                &mut meshes,
                &mut materials,
                position,
                msg.player_id.clone(),
                msg.player_name.clone(),
            );

            // Add RemotePlayer marker to distinguish from local player
            commands.entity(avatar_entity).insert(RemotePlayer);

            info!(
                "👤 Web: New remote player joined using unified avatar: {} ({})",
                msg.player_name, msg.player_id
            );
            web_sys::console::log_1(
                &format!(
                    "👤 Web: New remote player joined using unified avatar: {} ({})",
                    msg.player_name, msg.player_id
                )
                .into(),
            );
        }
    }
}

// ============ TOUCH UI SYSTEMS ============

/// Setup touch control UI with virtual joystick and touch areas
pub fn setup_touch_ui(
    mut commands: Commands,
    mut touch_state: ResMut<TouchInputState>,
    windows: Query<&Window>,
    asset_server: Res<AssetServer>,
) {
    if let Ok(window) = windows.single() {
        let window_width = window.resolution.width();
        let window_height = window.resolution.height();

        // Store screen dimensions
        touch_state.screen_width = window_width;
        touch_state.screen_height = window_height;

        info!(
            "📱 Setting up touch UI for {}x{} screen",
            window_width, window_height
        );

        // Create UI camera for 2D overlay with higher order to render on top
        commands.spawn((
            Camera2d,
            Camera {
                order: 1, // Render after the 3D camera (order 0)
                ..default()
            },
        ));

        // Create touch control overlay container
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                TouchControlsUI,
            ))
            .with_children(|parent| {
                // Build info display in bottom-right corner
                parent
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            right: Val::Px(10.0),
                            bottom: Val::Px(10.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
                        BorderRadius::all(Val::Px(5.0)),
                    ))
                    .with_children(|info_parent| {
                        let build_timestamp = env!(
                            "BUILD_TIMESTAMP",
                            "Set BUILD_TIMESTAMP environment variable during build"
                        );
                        info_parent.spawn((
                            Text::new(format!("Build: {}", build_timestamp)),
                            TextFont {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 12.0,
                                ..default()
                            },
                            TextColor(Color::srgba(0.8, 0.8, 0.8, 0.9)),
                        ));
                    });

                // Virtual joystick base (initially hidden)
                parent
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(100.0), // Will be updated dynamically
                            bottom: Val::Px(150.0),
                            width: Val::Px(120.0),
                            height: Val::Px(120.0),
                            border: UiRect::all(Val::Px(3.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.4)),
                        BorderColor(Color::srgba(0.8, 0.8, 0.8, 0.6)),
                        BorderRadius::all(Val::Px(60.0)), // Circular
                        Visibility::Hidden,               // Start hidden
                        JoystickBase,
                    ))
                    .with_children(|base_parent| {
                        // Virtual joystick knob
                        base_parent.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: Val::Px(35.0), // Centered in base
                                top: Val::Px(35.0),
                                width: Val::Px(50.0),
                                height: Val::Px(50.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.9, 0.9, 0.9, 0.8)),
                            BorderRadius::all(Val::Px(25.0)), // Circular
                            JoystickKnob,
                        ));
                    });
            });

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &"📱 Touch UI setup complete with virtual joystick and zone indicators".into(),
        );
    }
}

/// Update touch UI based on current touch state
pub fn update_touch_ui(
    touch_state: Res<TouchInputState>,
    mut joystick_base_query: Query<
        (&mut Node, &mut Visibility),
        (With<JoystickBase>, Without<JoystickKnob>),
    >,
    mut joystick_knob_query: Query<&mut Node, (With<JoystickKnob>, Without<JoystickBase>)>,
    windows: Query<&Window>,
) {
    if let Ok(window) = windows.single() {
        let window_height = window.resolution.height();

        // Update joystick base position and visibility
        if let Ok((mut base_style, mut base_visibility)) = joystick_base_query.get_single_mut() {
            if touch_state.joystick_active {
                // Show joystick and position it at touch center
                *base_visibility = Visibility::Visible;
                base_style.left = Val::Px(touch_state.joystick_center.x - 60.0); // Center the 120px base
                base_style.bottom = Val::Px(window_height - touch_state.joystick_center.y - 60.0); // Flip Y coordinate

                // Update knob position within base
                if let Ok(mut knob_style) = joystick_knob_query.get_single_mut() {
                    // Clamp joystick offset to base radius (60px)
                    let max_offset = 35.0; // Half the base radius minus knob radius
                    let clamped_offset = if touch_state.joystick_offset.length() > max_offset {
                        touch_state.joystick_offset.normalize() * max_offset
                    } else {
                        touch_state.joystick_offset
                    };

                    // Position knob relative to base center (35px from edge)
                    knob_style.left = Val::Px(35.0 + clamped_offset.x);
                    knob_style.top = Val::Px(35.0 - clamped_offset.y); // Invert Y for UI coordinate system
                }
            } else {
                // Hide joystick when not active
                *base_visibility = Visibility::Hidden;
            }
        }
    }
}
