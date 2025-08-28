// Minimal debug version for iPad crash diagnosis
use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn debug_set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let mut msg = String::new();

        if let Some(payload) = info.payload().downcast_ref::<&str>() {
            msg.push_str(&format!("🚨 DEBUG PANIC: {}", payload));
        } else if let Some(payload) = info.payload().downcast_ref::<String>() {
            msg.push_str(&format!("🚨 DEBUG PANIC: {}", payload));
        } else {
            msg.push_str("🚨 DEBUG PANIC: (no message)");
        }

        if let Some(location) = info.location() {
            msg.push_str(&format!(
                " at {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            ));
        }

        // Log to console
        web_sys::console::error_1(&msg.clone().into());

        // Try to alert the user
        if let Some(window) = web_sys::window() {
            let _ = window.alert_with_message(&format!("CRASH DETECTED: {}", msg));
        }
    }));

    web_sys::console::log_1(&"🚨 DEBUG panic hook initialized".into());
}

/// Minimal debug app - just the essentials
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn debug_start() {
    debug_set_panic_hook();
    web_sys::console::log_1(&"🚨 Starting DEBUG version - minimal components only".into());

    // Create app with full scene but enhanced debugging
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "IoTCraft DEBUG - iPad Crash Diagnosis".to_string(),
                        resolution: bevy::window::WindowResolution::new(1280, 720),
                        canvas: Some("#canvas".to_owned()),
                        fit_canvas_to_parent: true,
                        prevent_default_event_handling: false,
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
        .insert_resource(crate::config::MqttConfig::from_web_env())
        .insert_resource(crate::profile::load_or_create_profile_with_override(None))
        .add_plugins(crate::web_menu::WebMenuPlugin)
        .add_plugins(crate::mqtt::MqttPlugin) // MQTT connection working!
        .add_plugins(crate::player_avatar::PlayerAvatarPlugin) // Add avatar animations
        .insert_resource(ClearColor(Color::srgb(0.53, 0.81, 0.92)))
        .insert_resource(crate::lib_gradual::CameraController::new())
        .insert_resource(crate::lib_gradual::TouchInputState::default())
        // Multiplayer resources
        .insert_resource(crate::lib_gradual::WorldId::default())
        .insert_resource(crate::lib_gradual::MultiplayerConnectionStatus::default())
        .insert_resource(crate::lib_gradual::PositionTimer::default())
        .insert_resource(crate::lib_gradual::SceneSetupGuard(false))
        .insert_resource(DebugState::default())
        .add_systems(
            Startup,
            (
                // Use the guarded scene setup to prevent duplicates
                crate::lib_gradual::setup_basic_scene_once,
                crate::lib_gradual::setup_multiplayer_connections,
                crate::lib_gradual::setup_touch_areas,
                crate::lib_gradual::setup_touch_ui,
            ),
        )
        .add_systems(
            Update,
            (
                crate::lib_gradual::rotate_cube,
                crate::lib_gradual::touch_camera_control_system
                    .run_if(in_state(crate::web_menu::WebGameState::InGame)),
                crate::lib_gradual::update_touch_ui
                    .run_if(in_state(crate::web_menu::WebGameState::InGame)),
                crate::lib_gradual::process_device_announcements,
                crate::lib_gradual::update_position_timer,
                crate::lib_gradual::publish_local_pose,
                crate::lib_gradual::apply_remote_poses,
                crate::lib_gradual::log_fps,
                debug_touch_handler, // Our debug touch handler
                debug_logger,        // Our debug logger
            ),
        )
        .run();
}

#[derive(Resource, Default)]
struct DebugState {
    touch_count: usize,
    last_touch_time: f64,
}

fn setup_full_debug_scene(
    commands: Commands,
    meshes: ResMut<Assets<Mesh>>,
    materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    web_sys::console::log_1(&"🚨 DEBUG: Setting up FULL IoTCraft scene...".into());

    // Use the exact same scene setup as the main app
    crate::lib_gradual::setup_basic_scene(commands, meshes, materials, asset_server);

    web_sys::console::log_1(&"🚨 DEBUG: Full scene setup complete".into());
}

// Keep the original minimal debug setup for reference (unused)
fn _debug_setup_minimal(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    web_sys::console::log_1(&"🚨 DEBUG: Setting up minimal scene".into());

    // Add camera
    commands.spawn(Camera3d::default());

    // Add light
    commands.spawn(DirectionalLight::default());

    // Add single cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
        Transform::from_xyz(0.0, 0.0, -5.0),
    ));

    web_sys::console::log_1(&"🚨 DEBUG: Minimal scene setup complete".into());
}

fn debug_touch_handler(
    mut touch_events: EventReader<TouchInput>,
    mut debug_state: ResMut<DebugState>,
    time: Res<Time>,
) {
    for touch in touch_events.read() {
        debug_state.touch_count += 1;
        debug_state.last_touch_time = time.elapsed_secs_f64();

        let msg = format!(
            "🚨 DEBUG TOUCH #{}: Phase: {:?}, ID: {}, Pos: ({:.1}, {:.1})",
            debug_state.touch_count, touch.phase, touch.id, touch.position.x, touch.position.y
        );

        web_sys::console::log_1(&msg.into());

        // Test if this causes the crash
        match touch.phase {
            bevy::input::touch::TouchPhase::Started => {
                web_sys::console::log_1(&"🚨 Touch STARTED - checking if this crashes...".into());
            }
            bevy::input::touch::TouchPhase::Ended => {
                web_sys::console::log_1(&"🚨 Touch ENDED - checking if this crashes...".into());
            }
            _ => {}
        }
    }
}

fn debug_logger(debug_state: Res<DebugState>, time: Res<Time>, mut last_log: Local<f64>) {
    let current_time = time.elapsed_secs_f64();

    // Log every 2 seconds to confirm app is still running
    if current_time - *last_log >= 2.0 {
        let msg = format!(
            "🚨 DEBUG: App running OK. Time: {:.1}s, Touches: {}, Last touch: {:.1}s ago",
            current_time,
            debug_state.touch_count,
            current_time - debug_state.last_touch_time
        );
        web_sys::console::log_1(&msg.into());
        *last_log = current_time;
    }
}
