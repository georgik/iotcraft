use crate::ui::GameState;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Plugin for player controller functionality
pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerMode::Walking).add_systems(
            Update,
            (
                handle_mode_switch,
                setup_player_physics.run_if(resource_changed::<PlayerMode>),
                player_movement,
            ),
        );
    }
}

/// Player movement modes
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerMode {
    Flying,  // Spectator/creative mode - no physics, free movement
    Walking, // Survival mode - physics enabled, gravity, collision
}

impl Default for PlayerMode {
    fn default() -> Self {
        Self::Walking
    }
}

/// Component to mark the player's physics body
#[derive(Component)]
pub struct PlayerPhysicsBody;

/// Component for player movement configuration
#[derive(Component)]
pub struct PlayerMovement {
    pub walk_speed: f32,
    pub run_speed: f32,
    pub jump_force: f32,
    pub ground_friction: f32,
    pub air_control: f32,
    pub gravity_scale: f32,
    pub is_grounded: bool,
    pub ground_check_distance: f32,
    pub last_spacebar_press: Option<f64>, // Track last spacebar press for double-tap detection
    pub double_tap_window: f64,           // Window for double-tap detection (seconds)
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            walk_speed: 5.0,
            run_speed: 12.0,
            jump_force: 8.0,
            ground_friction: 0.9,
            air_control: 0.3,
            gravity_scale: 1.0,
            is_grounded: false,
            ground_check_distance: 1.2, // Slightly more generous ground detection
            last_spacebar_press: None,
            double_tap_window: 0.25, // 250ms window for double-tap (more precise)
        }
    }
}

/// System to handle mode switching
fn handle_mode_switch(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_mode: ResMut<PlayerMode>,
    game_state: Res<State<GameState>>,
) {
    // Only handle mode switch in game
    if *game_state.get() != GameState::InGame {
        return;
    }

    // Use F4 to toggle between flying and walking mode (like Minecraft spectator toggle)
    if keyboard_input.just_pressed(KeyCode::F4) {
        *player_mode = match *player_mode {
            PlayerMode::Flying => PlayerMode::Walking,
            PlayerMode::Walking => PlayerMode::Flying,
        };

        info!("Switched to {:?} mode", *player_mode);
    }
}

/// System to setup player physics based on current mode
fn setup_player_physics(
    mut commands: Commands,
    player_mode: Res<PlayerMode>,
    camera_query: Query<Entity, With<Camera>>,
) {
    let Ok(camera_entity) = camera_query.single() else {
        return;
    };

    match *player_mode {
        PlayerMode::Flying => {
            // Remove physics components from camera if they exist
            commands
                .entity(camera_entity)
                .remove::<RigidBody>()
                .remove::<Collider>()
                .remove::<PlayerMovement>()
                .remove::<PlayerPhysicsBody>()
                .remove::<LinearVelocity>() // Clear any residual velocity from physics mode
                .remove::<AngularVelocity>() // Clear any residual angular velocity
                .remove::<GravityScale>()
                .remove::<LockedAxes>()
                .remove::<Mass>()
                .remove::<Restitution>()
                .remove::<Friction>()
                .remove::<LinearDamping>()
                .remove::<AngularDamping>()
                .remove::<ColliderDensity>();
            info!("Configured camera for flying mode - cleared physics components");
        }
        PlayerMode::Walking => {
            // Add physics components to camera with optimized settings
            commands.entity(camera_entity).insert((
                RigidBody::Dynamic,
                Collider::capsule(0.4, 1.6), // Player capsule: radius 0.4, height 1.6 (wider to prevent falling through gaps)
                PlayerMovement::default(),
                PlayerPhysicsBody,
                GravityScale(1.5),           // Stronger gravity for snappy movement
                LockedAxes::ROTATION_LOCKED, // Prevent player from rotating physically
                Mass(70.0),                  // 70kg player
                Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
                Friction::new(0.9).with_combine_rule(CoefficientCombine::Max), // Higher friction for stable walking
                LinearDamping(0.9),   // Damping to prevent sliding
                AngularDamping(10.0), // High angular damping to prevent unwanted rotation
                // Optimize collision detection
                ColliderDensity(1.0),
            ));
            info!("Configured camera for walking mode with physics");
        }
    }
}

/// System to handle player movement based on current mode
fn player_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_mode: ResMut<PlayerMode>,
    mut camera_query: Query<
        (
            &mut Transform,
            Option<&mut LinearVelocity>,
            Option<&mut PlayerMovement>,
        ),
        With<Camera>,
    >,
    spatial_query: SpatialQuery,
    game_state: Res<State<GameState>>,
) {
    if *game_state.get() != GameState::InGame {
        return;
    }

    let Ok((transform, linear_velocity, player_movement)) = camera_query.single_mut() else {
        return;
    };

    match *player_mode {
        PlayerMode::Flying => {
            // Use existing flying movement logic (keep the existing camera controller behavior)
            // This is handled by the existing camera controller system
        }
        PlayerMode::Walking => {
            if let (Some(mut velocity), Some(mut movement)) = (linear_velocity, player_movement) {
                // Check for double spacebar to toggle flight mode
                let (mode_changed, should_skip_jump) = handle_double_spacebar_flight_toggle(
                    &time,
                    &keyboard_input,
                    &mut movement,
                    &mut player_mode,
                );

                if mode_changed {
                    return; // Mode changed, let the next frame handle the new mode
                }

                handle_walking_movement(
                    &time,
                    &keyboard_input,
                    &transform,
                    &mut velocity,
                    &mut movement,
                    &spatial_query,
                    should_skip_jump,
                );
            }
        }
    }
}

/// Handle movement in walking mode
fn handle_walking_movement(
    time: &Res<Time>,
    keyboard_input: &Res<ButtonInput<KeyCode>>,
    transform: &Transform,
    velocity: &mut LinearVelocity,
    movement: &mut PlayerMovement,
    spatial_query: &SpatialQuery,
    should_skip_jump: bool,
) {
    let dt = time.delta_secs();

    // Ground check - cast a ray downward to check if player is on ground
    // Start the ray from slightly inside the player capsule to avoid issues with floating point precision
    let ray_origin = transform.translation + Vec3::new(0.0, -0.7, 0.0); // Start from bottom of player capsule
    let ray_direction = Dir3::NEG_Y;
    let max_distance = movement.ground_check_distance - 0.7; // Adjust for starting position

    // Use a more robust ground check that excludes the player entity itself
    let ground_check = spatial_query.cast_ray(
        ray_origin,
        ray_direction,
        max_distance,
        true, // solid: whether to include solid bodies
        &SpatialQueryFilter::default(),
    );

    movement.is_grounded = ground_check.is_some();

    // Debug ground check occasionally
    if keyboard_input.just_pressed(KeyCode::KeyG) {
        info!(
            "Ground check: {:?} at {:?} distance {}",
            movement.is_grounded, ray_origin, max_distance
        );
    }

    // Handle input
    let mut movement_input = Vec3::ZERO;

    // Forward/backward movement (fixed direction)
    if keyboard_input.pressed(KeyCode::KeyW) {
        movement_input += transform.forward().as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        movement_input -= transform.forward().as_vec3();
    }

    // Left/right movement
    if keyboard_input.pressed(KeyCode::KeyA) {
        movement_input -= transform.right().as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        movement_input += transform.right().as_vec3();
    }

    // Normalize horizontal movement
    if movement_input.length() > 0.0 {
        movement_input = movement_input.normalize();
    }

    // Apply movement speed
    let current_speed = if keyboard_input.pressed(KeyCode::ShiftLeft) {
        movement.run_speed
    } else {
        movement.walk_speed
    };

    // Calculate horizontal movement
    let control_factor = if movement.is_grounded {
        1.0
    } else {
        movement.air_control
    };

    let target_horizontal_velocity = movement_input * current_speed;
    let horizontal_velocity = Vec3::new(velocity.x, 0.0, velocity.z);

    // Apply movement with control factor
    let velocity_change =
        (target_horizontal_velocity - horizontal_velocity) * control_factor * dt * 10.0;
    velocity.x += velocity_change.x;
    velocity.z += velocity_change.z;

    // Apply friction when grounded
    if movement.is_grounded && movement_input.length() == 0.0 {
        velocity.x *= 1.0 - (movement.ground_friction * dt);
        velocity.z *= 1.0 - (movement.ground_friction * dt);
    }

    // Limit falling speed to prevent excessive velocity buildup
    const MAX_FALL_SPEED: f32 = -20.0;
    if velocity.y < MAX_FALL_SPEED {
        velocity.y = MAX_FALL_SPEED;
    }

    // Handle jumping - but don't jump if double spacebar was just processed or if not grounded
    if keyboard_input.just_pressed(KeyCode::Space) && movement.is_grounded && !should_skip_jump {
        velocity.y = movement.jump_force;
        info!("Player jumped!");
    }
}

/// Handle double spacebar press to toggle flight mode (similar to creative mode mechanics)
/// Returns (mode_changed, should_skip_jump)
fn handle_double_spacebar_flight_toggle(
    time: &Res<Time>,
    keyboard_input: &Res<ButtonInput<KeyCode>>,
    movement: &mut PlayerMovement,
    player_mode: &mut ResMut<PlayerMode>,
) -> (bool, bool) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        let current_time = time.elapsed_secs_f64();

        if let Some(last_press) = movement.last_spacebar_press {
            let time_diff = current_time - last_press;
            if time_diff <= movement.double_tap_window {
                // Double tap detected! Toggle flight mode
                **player_mode = PlayerMode::Flying;
                info!(
                    "Double spacebar detected - switching to Flying mode! ({}ms apart)",
                    (time_diff * 1000.0) as u32
                );
                movement.last_spacebar_press = None; // Reset to prevent triple-tap issues
                return (true, true); // Mode changed, skip jump
            }
        }

        // Only set last press time if player is grounded (to prevent air-tap detection)
        if movement.is_grounded {
            movement.last_spacebar_press = Some(current_time);
            return (false, false); // No mode change, allow jump if grounded
        } else {
            // Don't register spacebar press when in air to prevent air jumping
            return (false, true); // No mode change, skip jump (prevent air jump)
        }
    }

    (false, false) // No spacebar press
}
