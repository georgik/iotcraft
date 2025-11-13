//! Source: https://github.com/bevyengine/bevy/blob/main/examples/helpers/camera_controller.rs
//! A freecam-style camera controller plugin.
//! To use in your own application:
//! - Copy the code for the [`CameraControllerPlugin`] and add the plugin to your App.
//! - Attach the [`CameraController`] component to an entity with a [`Camera3d`].
//!
//! Unlike other examples, which demonstrate an application, this demonstrates a plugin library.

use crate::player_controller::PlayerMode;
use crate::ui::GameState;
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit},
    prelude::*,
    // window::CursorGrabMode, // Unused in Bevy 0.17
};
use std::{f32::consts::*, fmt};

/// Resource to track the previous player mode for camera velocity management
#[derive(Resource, Default)]
struct PreviousPlayerMode(Option<PlayerMode>);

/// A freecam-style camera controller plugin.
pub struct CameraControllerPlugin;

impl Plugin for CameraControllerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PreviousPlayerMode::default())
            .add_systems(
                Update,
                (sync_camera_controller_with_transform, run_camera_controller).chain(),
            );
    }
}

/// Based on Valorant's default sensitivity, not entirely sure why it is exactly 1.0 / 180.0,
/// but I'm guessing it is a misunderstanding between degrees/radians and then sticking with
/// it because it felt nice.
pub const RADIANS_PER_DOT: f32 = 1.0 / 180.0;

/// Camera controller [`Component`].
#[derive(Component)]
pub struct CameraController {
    /// Enables this [`CameraController`] when `true`.
    pub enabled: bool,
    /// Indicates if this controller has been initialized by the [`CameraControllerPlugin`].
    pub initialized: bool,
    /// Multiplier for pitch and yaw rotation speed.
    pub sensitivity: f32,
    /// Gamepad right stick X axis value for camera rotation
    pub gamepad_look_x: f32,
    /// Gamepad right stick Y axis value for camera rotation
    pub gamepad_look_y: f32,
    /// [`KeyCode`] for forward translation.
    pub key_forward: KeyCode,
    /// [`KeyCode`] for backward translation.
    pub key_back: KeyCode,
    /// [`KeyCode`] for left translation.
    pub key_left: KeyCode,
    /// [`KeyCode`] for right translation.
    pub key_right: KeyCode,
    /// [`KeyCode`] for up translation.
    pub key_up: KeyCode,
    /// [`KeyCode`] for down translation.
    pub key_down: KeyCode,
    /// [`KeyCode`] to use [`run_speed`](CameraController::run_speed) instead of
    /// [`walk_speed`](CameraController::walk_speed) for translation.
    pub key_run: KeyCode,
    /// [`MouseButton`] for grabbing the mouse focus.
    pub mouse_key_cursor_grab: MouseButton,
    /// [`KeyCode`] for grabbing the keyboard focus.
    pub keyboard_key_toggle_cursor_grab: KeyCode,
    /// Multiplier for unmodified translation speed.
    pub walk_speed: f32,
    /// Multiplier for running translation speed.
    pub run_speed: f32,
    /// Multiplier for how the mouse scroll wheel modifies [`walk_speed`](CameraController::walk_speed)
    /// and [`run_speed`](CameraController::run_speed).
    pub scroll_factor: f32,
    /// Friction factor used to exponentially decay [`velocity`](CameraController::velocity) over time.
    pub friction: f32,
    /// This [`CameraController`]'s pitch rotation.
    pub pitch: f32,
    /// This [`CameraController`]'s yaw rotation.
    pub yaw: f32,
    /// This [`CameraController`]'s translation velocity.
    pub velocity: Vec3,
    /// Flag to ignore the next mouse motion delta after cursor re-grab to prevent camera jump
    pub ignore_next_mouse_delta: bool,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            enabled: true,
            initialized: false,
            sensitivity: 1.0,
            gamepad_look_x: 0.0,
            gamepad_look_y: 0.0,
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_up: KeyCode::KeyE,
            key_down: KeyCode::KeyQ,
            key_run: KeyCode::ShiftLeft,
            mouse_key_cursor_grab: MouseButton::Left,
            keyboard_key_toggle_cursor_grab: KeyCode::F1, // Disabled to use custom state management
            walk_speed: 5.0,
            run_speed: 15.0,
            scroll_factor: 0.1,
            friction: 0.5,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::ZERO,
            ignore_next_mouse_delta: false,
        }
    }
}

impl fmt::Display for CameraController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "
Freecam Controls:
    Mouse\t- Move camera orientation
    Scroll\t- Adjust movement speed
    {:?}\t- Hold to grab cursor
    {:?}\t- Toggle cursor grab
    {:?} & {:?}\t- Fly forward & backwards
    {:?} & {:?}\t- Fly sideways left & right
    {:?} & {:?}\t- Fly up & down
    {:?}\t- Fly faster while held",
            self.mouse_key_cursor_grab,
            self.keyboard_key_toggle_cursor_grab,
            self.key_forward,
            self.key_back,
            self.key_left,
            self.key_right,
            self.key_up,
            self.key_down,
            self.key_run,
        )
    }
}

/// System to sync camera controller's yaw/pitch with transform changes (e.g., from world loading)
fn sync_camera_controller_with_transform(
    mut query: Query<(&Transform, &mut CameraController), (With<Camera>, Changed<Transform>)>,
) {
    for (transform, mut controller) in query.iter_mut() {
        // Only sync if the controller is initialized and the transform changed externally
        if controller.initialized {
            let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);

            // Check if this is a significant change (not just from our own mouse input)
            let yaw_diff = (controller.yaw - yaw).abs();
            let pitch_diff = (controller.pitch - pitch).abs();

            // If the difference is significant, it means the transform was changed externally
            // (e.g., by world loading), so we need to sync our internal state
            if yaw_diff > 0.01 || pitch_diff > 0.01 {
                info!(
                    "Syncing camera controller with external transform change: yaw {} -> {}, pitch {} -> {}",
                    controller.yaw, yaw, controller.pitch, pitch
                );
                controller.yaw = yaw;
                controller.pitch = pitch;

                // Also set the ignore flag to prevent the next mouse delta from causing issues
                controller.ignore_next_mouse_delta = true;
                info!("Set ignore_next_mouse_delta flag due to external transform change");
            }
        }
    }
}

fn run_camera_controller(
    time: Res<Time>,
    _windows: Query<&Window>,
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    accumulated_mouse_scroll: Res<AccumulatedMouseScroll>,
    key_input: Res<ButtonInput<KeyCode>>,
    game_state: Res<State<GameState>>,
    player_mode: Res<PlayerMode>,
    mut previous_mode: ResMut<PreviousPlayerMode>,
    mut gamepad_events: EventReader<crate::input::GamepadInputEvent>,
    gamepads: Query<&bevy::input::gamepad::Gamepad>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera>>,
) {
    let dt = time.delta_secs();

    let Ok((mut transform, mut controller)) = query.single_mut() else {
        return;
    };

    if !controller.initialized {
        let (yaw, pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
        controller.yaw = yaw;
        controller.pitch = pitch;
        controller.initialized = true;

        // Start with mouse not captured (main menu will handle initial state)

        // Don't capture mouse on startup - main menu handles cursor state
        // In Bevy 0.17, cursor options are managed separately from Window
        // This initialization is handled by the main menu system

        info!("{}", *controller);
    }
    if !controller.enabled {
        return;
    }

    let mut scroll = 0.0;

    let amount = match accumulated_mouse_scroll.unit {
        MouseScrollUnit::Line => accumulated_mouse_scroll.delta.y,
        MouseScrollUnit::Pixel => accumulated_mouse_scroll.delta.y / 16.0,
    };
    scroll += amount;
    controller.walk_speed += scroll * controller.scroll_factor * controller.walk_speed;
    controller.run_speed = controller.walk_speed * 3.0;

    // Handle key input
    let mut axis_input = Vec3::ZERO;
    if key_input.pressed(controller.key_forward) {
        axis_input.z += 1.0;
    }
    if key_input.pressed(controller.key_back) {
        axis_input.z -= 1.0;
    }
    if key_input.pressed(controller.key_right) {
        axis_input.x += 1.0;
    }
    if key_input.pressed(controller.key_left) {
        axis_input.x -= 1.0;
    }
    if key_input.pressed(controller.key_up) {
        axis_input.y += 1.0;
    }
    if key_input.pressed(controller.key_down) {
        axis_input.y -= 1.0;
    }

    // Cursor management is handled by the game state system, not the camera controller
    // This ensures no conflicts with our main menu and ESC key handling

    // Handle mouse input - Apply rotation when in game state and cursor is actually grabbed
    // In Bevy 0.17, we need to check cursor state differently since it's managed by a separate system
    // For now, assume cursor is grabbed when in game state (this is managed by main menu system)
    let is_cursor_grabbed = *game_state.get() == GameState::InGame;

    if accumulated_mouse_motion.delta != Vec2::ZERO
        && *game_state.get() == GameState::InGame
        && is_cursor_grabbed
    {
        // Check if we should ignore this mouse delta (first frame after cursor re-grab)
        if controller.ignore_next_mouse_delta {
            // Reset the flag and skip this mouse input to prevent camera jump
            controller.ignore_next_mouse_delta = false;
            info!(
                "Ignoring mouse delta after cursor re-grab to prevent jump: {:?}",
                accumulated_mouse_motion.delta
            );
        } else {
            // Apply look update
            controller.pitch = (controller.pitch
                - accumulated_mouse_motion.delta.y * RADIANS_PER_DOT * controller.sensitivity)
                .clamp(-PI / 2., PI / 2.);
            controller.yaw -=
                accumulated_mouse_motion.delta.x * RADIANS_PER_DOT * controller.sensitivity;
            transform.rotation =
                Quat::from_euler(EulerRot::ZYX, 0.0, controller.yaw, controller.pitch);
        }
    }

    // Handle gamepad input for camera rotation
    if *game_state.get() == GameState::InGame {
        let gamepad_look_speed = 3.0 * controller.sensitivity; // Adjustable multiplier for gamepad
        let deadzone = 0.15; // Deadzone to prevent drift

        // Reset gamepad look values each frame
        controller.gamepad_look_x = 0.0;
        controller.gamepad_look_y = 0.0;

        // Check gamepad state directly every frame (fixes stick return-to-center issue)
        for gamepad in gamepads.iter() {
            use bevy::input::gamepad::GamepadAxis;

            // Check right stick X axis (horizontal look)
            if let Some(x_value) = gamepad.get(GamepadAxis::RightStickX) {
                if x_value.abs() > deadzone {
                    controller.gamepad_look_x = x_value;
                }
            }

            // Check right stick Y axis (vertical look)
            if let Some(y_value) = gamepad.get(GamepadAxis::RightStickY) {
                if y_value.abs() > deadzone {
                    // Invert Y axis for more intuitive control (typical gamepad behavior)
                    controller.gamepad_look_y = -y_value;
                }
            }
        }

        // Process gamepad axis events for additional responsiveness
        for event in gamepad_events.read() {
            if let crate::input::GamepadInputEvent::AxisMoved { value, action, .. } = event {
                match action {
                    crate::input::GameAxisAction::LookHorizontal => {
                        if value.abs() > deadzone {
                            controller.gamepad_look_x = *value;
                        }
                    }
                    crate::input::GameAxisAction::LookVertical => {
                        if value.abs() > deadzone {
                            // Invert Y axis for more intuitive control (typical gamepad behavior)
                            controller.gamepad_look_y = -*value;
                        }
                    }
                    _ => {} // Ignore other axis events in camera controller
                }
            }
        }

        // Debug logging for camera rotation
        static mut LAST_CAMERA_DEBUG: f64 = 0.0;
        unsafe {
            let current_time = time.elapsed_secs_f64();
            if (controller.gamepad_look_x.abs() > 0.01 || controller.gamepad_look_y.abs() > 0.01)
                && current_time - LAST_CAMERA_DEBUG > 1.0
            {
                LAST_CAMERA_DEBUG = current_time;
                info!(
                    "Camera rotation: look_x={:.3}, look_y={:.3}",
                    controller.gamepad_look_x, controller.gamepad_look_y
                );
            }
        }

        // Apply gamepad camera rotation
        if controller.gamepad_look_x.abs() > 0.0 || controller.gamepad_look_y.abs() > 0.0 {
            controller.yaw -= controller.gamepad_look_x * gamepad_look_speed * dt;
            controller.pitch = (controller.pitch
                + controller.gamepad_look_y * gamepad_look_speed * dt)
                .clamp(-PI / 2., PI / 2.);
            transform.rotation =
                Quat::from_euler(EulerRot::ZYX, 0.0, controller.yaw, controller.pitch);
        }
    }

    // Clear velocity when first entering flying mode to prevent physics carryover
    // This prevents the "seeing only sky" issue when transitioning from walking to flying
    if previous_mode.0 != Some(*player_mode) && *player_mode == PlayerMode::Flying {
        controller.velocity = Vec3::ZERO;
        info!("Cleared camera velocity when switching to flying mode");
    }

    // Also clear velocity when switching from flying to walking to prevent initial physics issues
    if previous_mode.0 == Some(PlayerMode::Flying) && *player_mode == PlayerMode::Walking {
        controller.velocity = Vec3::ZERO;
        info!("Cleared camera velocity when switching from flying to walking mode");
    }

    previous_mode.0 = Some(*player_mode);

    // Only handle movement in flying mode - walking mode movement is handled by the physics system
    if *player_mode == PlayerMode::Flying {
        // Update velocity
        if axis_input != Vec3::ZERO {
            let max_speed = if key_input.pressed(controller.key_run) {
                controller.run_speed
            } else {
                controller.walk_speed
            };
            controller.velocity = axis_input.normalize() * max_speed;
        } else {
            let friction = controller.friction.clamp(0.0, 1.0);
            controller.velocity *= 1.0 - friction;
            if controller.velocity.length_squared() < 1e-6 {
                controller.velocity = Vec3::ZERO;
            }
        }

        // Apply movement update - Now uses updated rotation from mouse input
        if controller.velocity != Vec3::ZERO {
            let forward = *transform.forward();
            let right = *transform.right();
            transform.translation += controller.velocity.x * dt * right
                + controller.velocity.y * dt * Vec3::Y
                + controller.velocity.z * dt * forward;
        }
    }
}
