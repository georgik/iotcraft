// IoTCraft Desktop Client - Web Version (Gradual Build)
use crate::script::script_systems::ScriptPlugin;
use crate::script::script_types::PendingCommands;
use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Desktop UI system adapted for web
use crate::ui::{GameState, MainMenuPlugin};

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

// Imports for inventory and interaction system
use crate::environment::{BlockType, VoxelWorld};
use crate::inventory::{BreakBlockEvent, ItemType, PlaceBlockEvent, PlayerInventory};

// WASM-compatible GhostBlockState (matches desktop behavior)
#[derive(Resource, Default)]
struct WebGhostBlockState {
    pub target_block_position: Option<IVec3>, // Position of existing block that would be broken (highlighted)
    pub placement_position: Option<IVec3>, // Position where new block would be placed (adjacent to target)
    pub can_place: bool,
}

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

/// Guard to prevent duplicate scene setup
#[derive(Resource)]
pub struct SceneSetupGuard(pub bool);

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
    mut camera_controller: Option<ResMut<CameraController>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<Camera3d>>,
    windows: Query<&mut Window>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut touch_events: EventReader<TouchInput>,
    mut touch_state: ResMut<TouchInputState>,
    // Check for UI interactions to avoid conflicts
    ui_interactions: Query<&Interaction, Changed<Interaction>>,
    game_state: Res<State<GameState>>,
) {
    // Only run in InGame state to avoid conflicts with menu systems
    if **game_state != GameState::InGame {
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!(
                "📱 Touch control system: Not in InGame state, current state: {:?}",
                **game_state
            )
            .into(),
        );
        return;
    }

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&"📱 Touch control system: Running in InGame state".into());

    // Check if any UI elements are currently being interacted with
    let ui_is_active = ui_interactions
        .iter()
        .any(|interaction| !matches!(interaction, Interaction::None));

    if ui_is_active {
        // Don't process touch events if UI is being used
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &"⚠️ Touch control system: UI is active, clearing touch events".into(),
        );
        touch_events.clear(); // Clear the events to prevent processing
        return;
    }

    #[cfg(target_arch = "wasm32")]
    {
        let touch_count = touch_events.len();
        if touch_count > 0 {
            web_sys::console::log_1(
                &format!(
                    "📱 Touch control system: Processing {} touch events, UI active: {}",
                    touch_count, ui_is_active
                )
                .into(),
            );
        }
    }
    // Check if camera controller exists and is enabled
    let Some(ref controller) = camera_controller else {
        return; // No camera controller resource available
    };

    if !controller.enabled {
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
        if let Some(ref mut controller) = camera_controller {
            let sensitivity = controller.touch_sensitivity * 0.0025; // Reduced sensitivity by 75% total

            // Apply accumulated rotation to camera controller
            let old_yaw = controller.yaw;
            let old_pitch = controller.pitch;

            controller.yaw -= touch_state.look_delta_accumulator.x * sensitivity;
            controller.pitch -= touch_state.look_delta_accumulator.y * sensitivity;

            // Clamp pitch to prevent over-rotation
            controller.pitch = controller.pitch.clamp(
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
                controller.yaw,
                controller.yaw - old_yaw,
                controller.pitch,
                controller.pitch - old_pitch,
                touch_state.look_delta_accumulator
            );

            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(
                &format!(
                    "🎮 Camera rotation: yaw={:.3}, pitch={:.3}",
                    controller.yaw, controller.pitch
                )
                .into(),
            );
        }
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
        if let Some(ref mut controller) = camera_controller {
            controller.yaw += yaw_change.to_radians() * dt;
            controller.pitch = (controller.pitch + pitch_change.to_radians() * dt)
                .clamp(-std::f32::consts::PI / 2.0, std::f32::consts::PI / 2.0);
        }
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
            if let Some(ref mut controller) = camera_controller {
                let delta_x = delta.x * controller.sensitivity * dt;
                let delta_y = delta.y * controller.sensitivity * dt;

                controller.yaw -= delta_x * 0.01;
                controller.pitch -= delta_y * 0.01;

                // Clamp pitch
                controller.pitch = controller.pitch.clamp(
                    -std::f32::consts::FRAC_PI_2 * 0.9,
                    std::f32::consts::FRAC_PI_2 * 0.9,
                );
            }
        }
    }

    // ============ APPLY TRANSFORMS ============

    // Apply rotation changes from all input sources
    if let Some(ref controller) = camera_controller {
        camera_transform.rotation =
            Quat::from_euler(EulerRot::ZYX, 0.0, controller.yaw, controller.pitch);
    }

    // Apply movement
    if velocity != Vec3::ZERO {
        let speed = camera_controller.as_ref().map(|c| c.speed).unwrap_or(5.0);
        camera_transform.translation += velocity.normalize() * speed * dt;
    }

    // Handle mouse capture (escape key is handled by menu system)
    // Note: Cursor options are now managed separately, web version doesn't need active cursor management
    for window in &windows {
        if mouse_button_input.just_pressed(MouseButton::Left) && window.focused {
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
    _window: &mut Window,
    cursor_options: Option<&mut bevy::window::CursorOptions>,
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

    if let Some(cursor_opts) = cursor_options {
        cursor_opts.grab_mode = grab_mode;
        cursor_opts.visible = visible;
    }

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
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "IoTCraft Desktop Client - Web Version".to_string(),
                    resolution: bevy::window::WindowResolution::new(1280, 720),
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
    .insert_resource(crate::profile::load_or_create_profile_with_override(None));

    // Initialize fonts resource immediately after AssetServer is available (same as desktop)
    // We need to do this in a way that ensures AssetServer exists
    app.world_mut()
        .resource_scope(|world, asset_server: Mut<AssetServer>| {
            let fonts = crate::fonts::Fonts::new(&asset_server);
            world.insert_resource(fonts);
        });

    app // Add desktop fonts and localization systems
        .add_plugins(crate::fonts::FontPlugin)
        .add_plugins(crate::localization::LocalizationPlugin)
        // Add desktop UI system
        .add_plugins(MainMenuPlugin)
        .add_plugins(MqttPlugin) // MQTT connection working!
        .add_plugins(crate::player_avatar::PlayerAvatarPlugin) // Add avatar animations
        .add_plugins(crate::console::ConsolePlugin) // Add full desktop console (with T key)
        .add_plugins(crate::web_player_controller::WebPlayerControllerPlugin) // Add web player controller with gravity and fly mode
        .add_plugins(crate::inventory::InventoryPlugin) // Add inventory system
        .add_plugins(crate::ui::InventoryUiPlugin) // Add inventory UI (hotbar)
        // Add error indicator plugin for ErrorResource (used by world systems)
        .add_plugins(crate::ui::error_indicator::ErrorIndicatorPlugin)
        // Note: EnvironmentPlugin disabled for web - comprehensive scene handled by setup_basic_scene_once
        .add_plugins(crate::multiplayer_web::WebMultiplayerPlugin) // Add web multiplayer for block sync
        // Add world plugin for world management (DiscoveredWorlds resource)
        .add_plugins(crate::world::WorldPlugin)
        // Add minimap plugin (same as desktop)
        .add_plugins(crate::minimap::MinimapPlugin)
        // Note: OnlineWorlds resource initialized by MainMenuPlugin for WASM compatibility
        // Add script system for unified world creation
        .add_plugins(ScriptPlugin)
        // Add desktop camera controller and player controller plugins
        .add_plugins(crate::camera_controllers::CameraControllerPlugin)
        .add_plugins(crate::player_controller::PlayerControllerPlugin)
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .insert_resource(TouchInputState::default())
        // Multiplayer resources
        .insert_resource(WorldId::default())
        .insert_resource(MultiplayerConnectionStatus::default())
        .insert_resource(PositionTimer::default())
        // Initialize GameState - required for handle_escape_key and other state-dependent systems
        .init_state::<GameState>()
        // VoxelWorld resource needed for multiplayer block synchronization
        .insert_resource(crate::environment::VoxelWorld::default())
        // Console resources for desktop console integration
        .insert_resource(crate::console::BlinkState::default())
        // MCP event needed by execute_pending_commands
        // Prevent duplicate scene setup - only one startup system should handle it
        .insert_resource(SceneSetupGuard(false))
        // WASM-specific ghost block state for inventory interaction
        .insert_resource(WebGhostBlockState::default())
        .add_systems(
            Startup,
            (
                setup_basic_scene_once, // Use guarded version to prevent duplicates
                setup_multiplayer_connections,
                setup_touch_areas,
                setup_touch_ui,
            ),
        )
        .add_systems(
            Update,
            (
                rotate_cube,
                manage_camera_controller_based_on_player_mode.run_if(in_state(GameState::InGame)),
                touch_camera_control_system.run_if(in_state(GameState::InGame)),
                update_touch_ui.run_if(in_state(GameState::InGame)),
                process_device_announcements,
                update_position_timer,
                publish_local_pose,
                apply_remote_poses,
                execute_pending_commands_web_wrapper, // Execute script commands to populate VoxelWorld
                sync_block_visuals_web, // Sync VoxelWorld blocks to visual entities (desktop pattern)
                populate_initial_inventory_web, // Add some blocks to inventory for testing
                // Inventory and interaction systems for WASM
                crate::inventory::handle_inventory_input_bundled
                    .run_if(in_state(GameState::InGame)),
                handle_block_interaction_input_web.run_if(in_state(GameState::InGame)),
                // Ghost block preview and crosshair systems for WASM
                update_ghost_block_preview_web.run_if(in_state(GameState::InGame)),
                draw_crosshair_web.run_if(in_state(GameState::InGame)),
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
#[derive(Component, Resource, Default)]
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
    mut voxel_world: ResMut<crate::environment::VoxelWorld>,
) {
    info!("Setting up enhanced IoTCraft world scene...");

    // Add a camera positioned like in the original desktop client with explicit order 0
    // Use the desktop camera controller component instead of simplified resource
    commands.spawn((
        Camera3d::default(),
        Camera {
            order: 0, // Render first (3D scene)
            ..default()
        },
        Transform::from_xyz(-8.0, 3.0, 15.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        crate::camera_controllers::CameraController {
            enabled: false, // Start disabled - only enable in Flying mode
            ..Default::default()
        }, // Add desktop camera controller component
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
            let position = IVec3::new(x, 0, z);
            // Add to VoxelWorld for collision detection
            voxel_world.set_block(position, crate::environment::BlockType::Grass);

            commands.spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(grass_material.clone()),
                Transform::from_translation(Vec3::new(x as f32, 0.0, z as f32)),
                crate::environment::VoxelBlock { position },
                Ground,
            ));
        }
    }

    // Create rolling hills - Hill 1 (dirt base)
    info!("Building rolling hills...");
    for x in -10..=-5 {
        for y in 1..=2 {
            for z in -10..=-5 {
                let position = IVec3::new(x, y, z);
                // Add to VoxelWorld for collision detection
                voxel_world.set_block(position, crate::environment::BlockType::Dirt);

                commands.spawn((
                    Mesh3d(cube_mesh.clone()),
                    MeshMaterial3d(dirt_material.clone()),
                    Transform::from_translation(Vec3::new(x as f32, y as f32, z as f32)),
                    crate::environment::VoxelBlock { position },
                ));
            }
        }
    }
    // Grass top of hill 1
    for x in -10..=-5 {
        for z in -10..=-5 {
            let position = IVec3::new(x, 0, z); // Note: y=0 for the grass top since it replaces ground
            // Add to VoxelWorld for collision detection
            voxel_world.set_block(position, crate::environment::BlockType::Grass);

            commands.spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(grass_material.clone()),
                Transform::from_translation(Vec3::new(x as f32, 3.0, z as f32)),
                crate::environment::VoxelBlock {
                    position: IVec3::new(x, 3, z),
                },
            ));
        }
    }

    // Hill 2 (dirt base)
    for x in 5..=10 {
        for y in 1..=3 {
            for z in 5..=10 {
                let position = IVec3::new(x, y, z);
                // Add to VoxelWorld for collision detection
                voxel_world.set_block(position, crate::environment::BlockType::Dirt);

                commands.spawn((
                    Mesh3d(cube_mesh.clone()),
                    MeshMaterial3d(dirt_material.clone()),
                    Transform::from_translation(Vec3::new(x as f32, y as f32, z as f32)),
                    crate::environment::VoxelBlock { position },
                ));
            }
        }
    }
    // Grass top of hill 2
    for x in 5..=10 {
        for z in 5..=10 {
            let position = IVec3::new(x, 4, z);
            // Add to VoxelWorld for collision detection
            voxel_world.set_block(position, crate::environment::BlockType::Grass);

            commands.spawn((
                Mesh3d(cube_mesh.clone()),
                MeshMaterial3d(grass_material.clone()),
                Transform::from_translation(Vec3::new(x as f32, 4.0, z as f32)),
                crate::environment::VoxelBlock { position },
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

/// Enable camera controller when entering the game
pub fn enable_camera_controller(
    mut camera_query: Query<&mut crate::camera_controllers::CameraController, With<Camera>>,
) {
    if let Ok(mut camera_controller) = camera_query.single_mut() {
        camera_controller.enabled = true;
        info!("📹 Desktop camera controller enabled for game");

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &"📹 Desktop camera controller enabled - WASD, mouse and touch controls ready".into(),
        );
    }
}

/// Guarded version of scene setup to prevent duplicates
pub fn setup_basic_scene_once(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut setup_guard: ResMut<SceneSetupGuard>,
    voxel_world: ResMut<crate::environment::VoxelWorld>,
) {
    // Only set up scene once
    if setup_guard.0 {
        info!("Scene already set up, skipping duplicate setup");
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&"🚫 Scene already set up, skipping duplicate setup".into());
        return;
    }

    setup_guard.0 = true;
    info!("🎬 Setting up scene (guarded) - first time only");
    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&"🎬 Setting up scene (guarded) - first time only".into());

    setup_basic_scene(commands, meshes, materials, asset_server, voxel_world);
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
    windows: Query<&mut Window>,
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
    // Note: Cursor options are now managed separately, web version doesn't need active cursor management
    for window in &windows {
        if mouse_button_input.just_pressed(MouseButton::Left) && window.focused {
            info!(
                "Mouse captured for camera control. Use WASD to move, mouse to look around. Press Escape to open menu."
            );
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

/// Web wrapper for execute_pending_commands that handles missing resources gracefully
fn execute_pending_commands_web_wrapper(
    mut pending_commands: ResMut<PendingCommands>,
    mut blink_state: ResMut<crate::console::BlinkState>,
    temperature: Res<crate::mqtt::TemperatureResource>,
    _mqtt_config: Res<MqttConfig>,
    mut voxel_world: ResMut<crate::environment::VoxelWorld>,
    _commands: Commands,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
    _asset_server: Res<AssetServer>,
    _query: Query<(Entity, &crate::environment::VoxelBlock)>,
    _device_query: Query<(&DeviceEntity, &Transform), Without<Camera>>,
    mut inventory: ResMut<crate::inventory::PlayerInventory>,
    mut camera_query: Query<
        (
            &mut Transform,
            &mut crate::camera_controllers::CameraController,
        ),
        With<Camera>,
    >,
    mut setup_guard: ResMut<SceneSetupGuard>,
) {
    // Debug logging for guard state
    if !pending_commands.commands.is_empty() {
        info!(
            "Web: Checking SceneSetupGuard state: {}, pending commands: {}",
            setup_guard.0,
            pending_commands.commands.len()
        );
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!(
                "Web: SceneSetupGuard: {}, pending commands: {}",
                setup_guard.0,
                pending_commands.commands.len()
            )
            .into(),
        );
    }

    // If the hardcoded scene has been set up, don't execute scripts to avoid conflicts
    // UNLESS this is template script execution for a new world (commands > 20 usually means template)
    if setup_guard.0 && pending_commands.commands.len() < 20 {
        // Clear commands to prevent accumulation, but don't execute them
        if !pending_commands.commands.is_empty() {
            info!(
                "Web: Skipping {} script commands because hardcoded scene is active",
                pending_commands.commands.len()
            );
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(
                &format!(
                    "Web: Skipping {} script commands because hardcoded scene is active",
                    pending_commands.commands.len()
                )
                .into(),
            );
            pending_commands.commands.clear();
        }
        return;
    } else if setup_guard.0 && pending_commands.commands.len() >= 20 {
        // This looks like a template script - reset the guard to allow execution
        info!(
            "Web: Detected template script with {} commands, resetting SceneSetupGuard to allow execution",
            pending_commands.commands.len()
        );
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!(
                "Web: Template detected - {} commands, resetting guard",
                pending_commands.commands.len()
            )
            .into(),
        );
        setup_guard.0 = false;
    }

    // Simplified web implementation of command execution
    // Process pending commands with basic functionality for web client

    // Detect if this is a large batch (template) to reduce logging verbosity
    let total_commands = pending_commands.commands.len();
    let is_large_batch = total_commands > 10;

    if is_large_batch {
        info!(
            "Web: Executing batch of {} template commands (reduced logging mode)",
            total_commands
        );
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!(
                "Web: Executing batch of {} template commands (reduced logging mode)",
                total_commands
            )
            .into(),
        );
    }

    for (idx, command) in pending_commands.commands.drain(..).enumerate() {
        // Only log individual commands if not a large batch, or show progress every 100 commands
        if !is_large_batch || idx % 100 == 0 {
            info!("Web: Executing queued command: {}", command);

            #[cfg(target_arch = "wasm32")]
            if is_large_batch {
                web_sys::console::log_1(
                    &format!("Web: Progress {}/{}: {}", idx + 1, total_commands, command).into(),
                );
            } else {
                web_sys::console::log_1(&format!("Web: Executing command: {}", command).into());
            }
        }

        // Parse command string and dispatch to appropriate handler
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "blink" => {
                if parts.len() == 2 {
                    let action = parts[1];
                    match action {
                        "start" => {
                            blink_state.blinking = true;
                            info!("Web: Blink started via script");
                        }
                        "stop" => {
                            blink_state.blinking = false;
                            info!("Web: Blink stopped via script");
                        }
                        _ => {
                            info!("Web: Usage: blink [start|stop]");
                        }
                    }
                }
            }
            "mqtt" => {
                if parts.len() == 2 {
                    let action = parts[1];
                    match action {
                        "status" => {
                            let status = if temperature.value.is_some() {
                                "Connected to MQTT broker"
                            } else {
                                "Connecting to MQTT broker..."
                            };
                            info!("Web: MQTT status: {}", status);
                        }
                        "temp" => {
                            let temp_msg = if let Some(val) = temperature.value {
                                format!("Current temperature: {:.1}°C", val)
                            } else {
                                "No temperature data available".to_string()
                            };
                            info!("Web: {}", temp_msg);
                        }
                        _ => {
                            info!("Web: Usage: mqtt [status|temp]");
                        }
                    }
                }
            }
            "place" => {
                if parts.len() == 5 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        parts[2].parse::<i32>(),
                        parts[3].parse::<i32>(),
                        parts[4].parse::<i32>(),
                    ) {
                        let block_type_str = parts[1];
                        if let Some(block_type) = parse_block_type(block_type_str) {
                            // Use desktop pattern: only store in VoxelWorld, let sync system create visuals
                            let position = IVec3::new(x, y, z);
                            voxel_world.set_block(position, block_type);

                            if !is_large_batch {
                                info!(
                                    "Web: Placed {} block at ({}, {}, {}) - stored in VoxelWorld, visual will be created by sync system",
                                    block_type_str, x, y, z
                                );
                            }
                        }
                    }
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
                        let block_type_str = parts[1];
                        if let Some(block_type) = parse_block_type(block_type_str) {
                            // Create wall by filling area between two points
                            let min_x = x1.min(x2);
                            let max_x = x1.max(x2);
                            let min_y = y1.min(y2);
                            let max_y = y1.max(y2);
                            let min_z = z1.min(z2);
                            let max_z = z1.max(z2);

                            let mut blocks_added = 0;
                            for x in min_x..=max_x {
                                for y in min_y..=max_y {
                                    for z in min_z..=max_z {
                                        // Use desktop pattern: only store in VoxelWorld
                                        let position = IVec3::new(x, y, z);
                                        voxel_world.set_block(position, block_type);
                                        blocks_added += 1;
                                    }
                                }
                            }

                            info!(
                                "Web: Created {} wall from ({},{},{}) to ({},{},{}) with {} blocks - stored in VoxelWorld, visuals will be created by sync system",
                                block_type_str, x1, y1, z1, x2, y2, z2, blocks_added
                            );

                            #[cfg(target_arch = "wasm32")]
                            web_sys::console::log_1(
                                &format!(
                                    "Web: Created {} wall from ({},{},{}) to ({},{},{}) with {} blocks - stored in VoxelWorld",
                                    block_type_str, x1, y1, z1, x2, y2, z2, blocks_added
                                )
                                .into(),
                            );
                        }
                    }
                }
            }
            "tp" | "teleport" => {
                if parts.len() == 4 {
                    if let (Ok(x), Ok(y), Ok(z)) = (
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                        parts[3].parse::<f32>(),
                    ) {
                        // Teleport camera to position
                        for (mut camera_transform, _) in camera_query.iter_mut() {
                            camera_transform.translation = Vec3::new(x, y, z);
                            info!("Web: Teleported camera to ({}, {}, {})", x, y, z);
                            break; // Only teleport the first camera
                        }
                    }
                }
            }
            "look" => {
                if parts.len() == 3 {
                    if let (Ok(yaw), Ok(pitch)) = (parts[1].parse::<f32>(), parts[2].parse::<f32>())
                    {
                        // Set camera rotation
                        for (mut camera_transform, _) in camera_query.iter_mut() {
                            let yaw_rad = yaw.to_radians();
                            let pitch_rad = pitch.to_radians();
                            camera_transform.rotation =
                                Quat::from_euler(EulerRot::YXZ, yaw_rad, pitch_rad, 0.0);
                            info!("Web: Set camera look to yaw={}, pitch={}", yaw, pitch);
                            break; // Only affect the first camera
                        }
                    }
                }
            }
            "give" => {
                if parts.len() == 3 {
                    if let Ok(count) = parts[2].parse::<u32>() {
                        let item_type = parts[1];
                        if let Some(block_type) = parse_block_type(item_type) {
                            let item_type = crate::inventory::ItemType::Block(block_type);
                            let remaining = inventory.add_items(item_type, count);
                            if remaining > 0 {
                                info!(
                                    "Web: Gave {} {} to player inventory ({} couldn't fit)",
                                    count - remaining,
                                    parts[1],
                                    remaining
                                );
                            } else {
                                info!("Web: Gave {} {} to player inventory", count, parts[1]);
                            }
                        }
                    }
                }
            }
            _ => {
                if !is_large_batch {
                    info!("Web: Unknown command: {}", parts[0]);
                }
            }
        }
    }

    // Show completion summary for large batches
    if is_large_batch {
        info!(
            "Web: Completed execution of {} template commands. World now has {} blocks.",
            total_commands,
            voxel_world.blocks.len()
        );
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!(
                "✅ Web: Template execution complete! {} commands processed, {} blocks in world",
                total_commands,
                voxel_world.blocks.len()
            )
            .into(),
        );
    }
}

/// Parse block type from string
fn parse_block_type(block_type_str: &str) -> Option<crate::environment::BlockType> {
    match block_type_str {
        "grass" => Some(crate::environment::BlockType::Grass),
        "dirt" => Some(crate::environment::BlockType::Dirt),
        "stone" => Some(crate::environment::BlockType::Stone),
        "quartz_block" => Some(crate::environment::BlockType::QuartzBlock),
        "glass_pane" => Some(crate::environment::BlockType::GlassPane),
        "cyan_terracotta" => Some(crate::environment::BlockType::CyanTerracotta),
        "water" => Some(crate::environment::BlockType::Water),
        _ => None,
    }
}

/// System to synchronize visual block entities with VoxelWorld data for WASM
/// This ensures blocks added via template scripts get visual representation
/// Same as desktop version but specifically for WASM context
fn sync_block_visuals_web(
    voxel_world: Res<crate::environment::VoxelWorld>,
    existing_blocks_query: Query<&crate::environment::VoxelBlock>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Only run this sync when voxel world changes to avoid performance issues
    if !voxel_world.is_changed() {
        return;
    }

    // Get all existing visual block positions
    let existing_positions: std::collections::HashSet<bevy::math::IVec3> = existing_blocks_query
        .iter()
        .map(|block| block.position)
        .collect();

    // Create visual entities for blocks that don't have them
    let mut created_visuals = 0;
    for (pos, block_type) in voxel_world.blocks.iter() {
        if !existing_positions.contains(pos) {
            // Create visual entity for this block
            let cube_mesh = meshes.add(bevy::math::primitives::Cuboid::new(
                crate::environment::CUBE_SIZE,
                crate::environment::CUBE_SIZE,
                crate::environment::CUBE_SIZE,
            ));

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
            let material = materials.add(StandardMaterial {
                base_color_texture: Some(texture),
                ..default()
            });

            commands.spawn((
                Mesh3d(cube_mesh),
                MeshMaterial3d(material),
                Transform::from_translation(pos.as_vec3()),
                crate::environment::VoxelBlock { position: *pos },
                Name::new(format!("WebSyncBlock-{}-{}-{}", pos.x, pos.y, pos.z)),
            ));

            created_visuals += 1;
        }
    }

    if created_visuals > 0 {
        info!(
            "Web: Synced {} visual entities for VoxelWorld blocks",
            created_visuals
        );

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(
            &format!(
                "Web: Synced {} visual entities for VoxelWorld blocks (total blocks: {})",
                created_visuals,
                voxel_world.blocks.len()
            )
            .into(),
        );
    }
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
                        BorderColor::all(Color::srgba(0.8, 0.8, 0.8, 0.6)),
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

/// Manage camera controller enabled state based on player mode
pub fn manage_camera_controller_based_on_player_mode(
    current_mode: Option<Res<crate::player_controller::PlayerMode>>,
    mut camera_controller_query: Query<
        &mut crate::camera_controllers::CameraController,
        With<Camera3d>,
    >,
) {
    let Some(player_mode) = current_mode else {
        return; // No player mode resource available yet
    };

    if let Ok(mut camera_controller) = camera_controller_query.single_mut() {
        let should_be_enabled =
            matches!(*player_mode, crate::player_controller::PlayerMode::Flying);

        if camera_controller.enabled != should_be_enabled {
            camera_controller.enabled = should_be_enabled;

            let mode_name = match *player_mode {
                crate::player_controller::PlayerMode::Flying => "Flying",
                crate::player_controller::PlayerMode::Walking => "Walking",
            };

            if should_be_enabled {
                info!("📹 Camera controller enabled for {} mode", mode_name);
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(
                    &format!("📹 Camera controller enabled for {} mode", mode_name).into(),
                );
            } else {
                info!(
                    "📹 Camera controller disabled for {} mode - player controller handling movement",
                    mode_name
                );
                #[cfg(target_arch = "wasm32")]
                web_sys::console::log_1(&format!("📹 Camera controller disabled for {} mode - player controller handling movement", mode_name).into());
            }
        }
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
        if let Ok((mut base_style, mut base_visibility)) = joystick_base_query.single_mut() {
            if touch_state.joystick_active {
                // Show joystick and position it at touch center
                *base_visibility = Visibility::Visible;
                base_style.left = Val::Px(touch_state.joystick_center.x - 60.0); // Center the 120px base
                base_style.bottom = Val::Px(window_height - touch_state.joystick_center.y - 60.0); // Flip Y coordinate

                // Update knob position within base
                if let Ok(mut knob_style) = joystick_knob_query.single_mut() {
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

/// Handle block interaction input (placing and breaking blocks) for WASM version
/// Mimics desktop behavior with raycasting from camera center
pub fn handle_block_interaction_input_web(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    game_state: Res<State<GameState>>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    voxel_world: Res<VoxelWorld>,
    mut place_block_events: EventWriter<PlaceBlockEvent>,
    mut break_block_events: EventWriter<BreakBlockEvent>,
    player_inventory: Res<PlayerInventory>,
    ghost_block_state: Option<Res<WebGhostBlockState>>,
) {
    // Only process input in InGame state
    if *game_state.get() != GameState::InGame {
        return;
    }

    // Get camera transform for raycasting
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };

    // Raycast from camera center (mimicking desktop crosshair behavior)
    let ray_origin = camera_transform.translation();
    let ray_direction = camera_transform.forward().as_vec3(); // Camera forward direction

    // Find intersection with voxel world (max distance ~10 blocks)
    const MAX_DISTANCE: f32 = 10.0;
    let mut ray_pos = ray_origin;
    let ray_step = ray_direction.normalize() * 0.1; // Small step size for accuracy

    for _ in 0..(MAX_DISTANCE / 0.1) as i32 {
        ray_pos += ray_step;

        // Convert world position to voxel coordinates
        let voxel_position = IVec3::new(
            ray_pos.x.floor() as i32,
            ray_pos.y.floor() as i32,
            ray_pos.z.floor() as i32,
        );

        // Check if there's a block at this position
        if voxel_world.is_block_at(voxel_position) {
            // Left click - break the block that's currently highlighted by the ghost system
            // This ensures perfect alignment between the wireframe highlight and block breaking
            if mouse_button_input.just_pressed(MouseButton::Left) {
                if let Some(ghost_state) = &ghost_block_state {
                    if let Some(target_position) = ghost_state.target_block_position {
                        break_block_events.write(BreakBlockEvent {
                            position: target_position,
                        });

                        #[cfg(target_arch = "wasm32")]
                        web_sys::console::log_1(
                            &format!("🔨 Breaking highlighted block at {:?}", target_position)
                                .into(),
                        );
                    }
                }
            }

            // Right click - place block (use ghost state logic like desktop)
            if mouse_button_input.just_pressed(MouseButton::Right) {
                // Get the currently selected item from inventory
                if let Some(selected_item) = player_inventory.get_selected_item() {
                    let ItemType::Block(block_type) = selected_item.item_type;
                    // Use ghost block state placement position (adjacent to highlighted block, like desktop)
                    if let Some(ghost_state) = &ghost_block_state {
                        if let Some(placement_position) = ghost_state.placement_position {
                            if ghost_state.can_place {
                                place_block_events.write(PlaceBlockEvent {
                                    position: placement_position,
                                });

                                #[cfg(target_arch = "wasm32")]
                                web_sys::console::log_1(
                                    &format!(
                                        "🧱 Placing {:?} block at {:?} (adjacent to highlighted block)",
                                        block_type, placement_position
                                    )
                                    .into(),
                                );
                            }
                        }
                    }
                }
            }

            break; // Stop raycasting after finding the first block
        }
    }
}

/// Updates ghost block preview for WASM (adapted from desktop interaction system)
pub fn update_ghost_block_preview_web(
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window>,
    voxel_world: Res<VoxelWorld>,
    mut ghost_state: ResMut<WebGhostBlockState>,
    game_state: Res<State<GameState>>,
) {
    // Only process in InGame state
    if *game_state.get() != GameState::InGame {
        ghost_state.target_block_position = None;
        ghost_state.placement_position = None;
        return;
    }

    let Ok(window) = windows.single() else {
        ghost_state.target_block_position = None;
        ghost_state.placement_position = None;
        return;
    };

    let Ok((camera, camera_transform)) = camera_query.single() else {
        ghost_state.target_block_position = None;
        ghost_state.placement_position = None;
        return;
    };

    // Use screen center as cursor position (like desktop when cursor is grabbed)
    let cursor_position = Vec2::new(window.width() / 2.0, window.height() / 2.0);

    // Use the camera component directly for raycasting
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        ghost_state.target_block_position = None;
        ghost_state.placement_position = None;
        return;
    };

    // Define interaction distance range - start from a minimum distance to avoid placing too close
    let min_distance = 2.0; // Minimum distance from camera
    let max_distance = 8.0; // Maximum reach distance
    let step_size = 0.1; // Fine-grained raycast steps

    ghost_state.target_block_position = None;
    ghost_state.placement_position = None;
    ghost_state.can_place = false;

    // Perform precise raycast to find the first existing block (Minecraft-style targeting)
    let mut current_distance = min_distance;
    let mut last_empty_pos = None;

    while current_distance <= max_distance {
        let check_position = (ray.origin + ray.direction * current_distance).as_ivec3();

        if voxel_world.is_block_at(check_position) {
            // Found the first existing block - this is our target for highlighting (would be broken on left-click)
            ghost_state.target_block_position = Some(check_position);

            // Try to place on top face first (Minecraft-like behavior)
            let top_face_pos = check_position + IVec3::new(0, 1, 0);
            if !voxel_world.is_block_at(top_face_pos) {
                // Top face is available - place there
                ghost_state.placement_position = Some(top_face_pos);
                ghost_state.can_place = true;
            } else if let Some(empty_pos) = last_empty_pos {
                // Top face blocked - fall back to closest face (ray approach)
                if !voxel_world.is_block_at(empty_pos) {
                    ghost_state.placement_position = Some(empty_pos);
                    ghost_state.can_place = true;
                }
            }
            break;
        }

        // Track the last empty position
        last_empty_pos = Some(check_position);
        current_distance += step_size;
    }
}

/// Draw crosshair and ghost block wireframe for WASM (adapted from desktop interaction system)
pub fn draw_crosshair_web(
    mut gizmos: Gizmos,
    game_state: Res<State<GameState>>,
    ghost_state: Res<WebGhostBlockState>,
    inventory: Res<PlayerInventory>,
    windows: Query<&Window>,
) {
    // Only draw in InGame state
    if *game_state.get() != GameState::InGame {
        return;
    }

    // Get window to determine screen center
    let Ok(window) = windows.single() else {
        return;
    };

    let screen_center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
    let crosshair_size = 10.0;

    // Draw crosshair at screen center
    gizmos.line_2d(
        screen_center + Vec2::new(-crosshair_size, 0.0),
        screen_center + Vec2::new(crosshair_size, 0.0),
        Color::WHITE,
    );
    gizmos.line_2d(
        screen_center + Vec2::new(0.0, -crosshair_size),
        screen_center + Vec2::new(0.0, crosshair_size),
        Color::WHITE,
    );

    // Draw ghost block wireframe around the target block (the one that would be broken)
    // This provides visual feedback like in Minecraft/Luanti
    if let Some(_selected_item) = inventory.get_selected_item() {
        if let Some(target_pos) = ghost_state.target_block_position {
            if ghost_state.can_place {
                // Convert voxel coordinates to world position (block corner)
                // Voxel coordinates are integers representing cube corners, same as block rendering
                let position = target_pos.as_vec3();
                let color = Color::srgba(0.2, 1.0, 0.2, 0.5); // Semi-transparent green

                // Draw wireframe cube around the block center
                let half_size = 0.5;
                let corners = [
                    position + Vec3::new(-half_size, -half_size, -half_size),
                    position + Vec3::new(half_size, -half_size, -half_size),
                    position + Vec3::new(half_size, half_size, -half_size),
                    position + Vec3::new(-half_size, half_size, -half_size),
                    position + Vec3::new(-half_size, -half_size, half_size),
                    position + Vec3::new(half_size, -half_size, half_size),
                    position + Vec3::new(half_size, half_size, half_size),
                    position + Vec3::new(-half_size, half_size, half_size),
                ];

                // Bottom face
                gizmos.line(corners[0], corners[1], color);
                gizmos.line(corners[1], corners[2], color);
                gizmos.line(corners[2], corners[3], color);
                gizmos.line(corners[3], corners[0], color);

                // Top face
                gizmos.line(corners[4], corners[5], color);
                gizmos.line(corners[5], corners[6], color);
                gizmos.line(corners[6], corners[7], color);
                gizmos.line(corners[7], corners[4], color);

                // Vertical edges
                gizmos.line(corners[0], corners[4], color);
                gizmos.line(corners[1], corners[5], color);
                gizmos.line(corners[2], corners[6], color);
                gizmos.line(corners[3], corners[7], color);
            }
        }
    }
}

/// Populate initial inventory items for WASM testing
pub fn populate_initial_inventory_web(
    mut inventory: ResMut<PlayerInventory>,
    mut populated: Local<bool>,
) {
    // Only populate once
    if *populated {
        return;
    }

    // Add some basic blocks for testing
    inventory.add_items(ItemType::Block(BlockType::Grass), 64);
    inventory.add_items(ItemType::Block(BlockType::Dirt), 64);
    inventory.add_items(ItemType::Block(BlockType::Stone), 32);
    inventory.add_items(ItemType::Block(BlockType::QuartzBlock), 16);
    inventory.add_items(ItemType::Block(BlockType::GlassPane), 16);
    inventory.add_items(ItemType::Block(BlockType::CyanTerracotta), 8);
    inventory.add_items(ItemType::Block(BlockType::Water), 8);

    // Select first slot by default
    inventory.select_slot(0);

    *populated = true;

    #[cfg(target_arch = "wasm32")]
    web_sys::console::log_1(&"✅ Initial inventory populated with building blocks".into());
}
