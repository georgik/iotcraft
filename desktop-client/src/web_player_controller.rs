use crate::ui::GameState;
use bevy::prelude::*;

/// Re-export the player controller types from desktop for reuse
pub use crate::player_controller::{PlayerMode, PlayerMovement};

/// Web-compatible player controller plugin
pub struct WebPlayerControllerPlugin;

impl Plugin for WebPlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerMode::Walking).add_systems(
            Update,
            (
                enable_gravity_after_world_populated_web,
                handle_mode_switch_web,
                player_movement_web,
                force_walking_mode_on_world_start_web,
            ),
        );
    }
}

/// System to handle mode switching with F4 key (web version)
fn handle_mode_switch_web(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_mode: ResMut<PlayerMode>,
    game_state: Res<State<GameState>>,
) {
    if !matches!(
        *game_state.get(),
        GameState::InGame | GameState::GameplayMenu
    ) {
        return;
    }

    if keyboard_input.just_pressed(KeyCode::F4) {
        *player_mode = match *player_mode {
            PlayerMode::Flying => PlayerMode::Walking,
            PlayerMode::Walking => PlayerMode::Flying,
        };
        info!("Switched to {:?} mode", *player_mode);
    }
}

/// System to handle player movement based on current mode (web version)
fn player_movement_web(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_mode: ResMut<PlayerMode>,
    mut commands: Commands,
    mut camera_query: Query<(Entity, &mut Transform, Option<&mut PlayerMovement>), With<Camera>>,
    voxel_world: Res<crate::environment::VoxelWorld>,
    game_state: Res<State<GameState>>,
) {
    // Only run when actually in gameplay (either actively playing or with the gameplay menu open)
    if !matches!(
        *game_state.get(),
        GameState::InGame | GameState::GameplayMenu
    ) {
        return;
    }

    // Handle multiple camera entities by taking the first one
    let camera_entities: Vec<_> = camera_query.iter_mut().collect();
    if camera_entities.is_empty() {
        return;
    }

    let (camera_entity, transform, player_movement) = camera_entities.into_iter().next().unwrap();

    // Debug logging every few seconds to understand what's happening
    static mut LAST_SYSTEM_DEBUG: f64 = 0.0;
    let current_time = time.elapsed_secs_f64();
    unsafe {
        if current_time - LAST_SYSTEM_DEBUG > 3.0 {
            LAST_SYSTEM_DEBUG = current_time;
            info!(
                "Web player movement system running: mode={:?}, has_movement_component={}, game_state={:?}",
                *player_mode,
                player_movement.is_some(),
                *game_state.get()
            );
        }
    }

    match *player_mode {
        PlayerMode::Flying => {
            // Remove PlayerMovement component if it exists
            if player_movement.is_some() {
                commands.entity(camera_entity).remove::<PlayerMovement>();
                info!("Removed PlayerMovement for flying mode");
            }
            // Use existing flying movement logic (keep the existing camera controller behavior)
            // This is handled by the existing camera controller system
        }
        PlayerMode::Walking => {
            // First ensure PlayerMovement component exists - add it if missing regardless of game state
            if player_movement.is_none() {
                let mut movement = PlayerMovement::default();
                movement.is_grounded = false; // Start not grounded to apply gravity
                commands.entity(camera_entity).insert(movement);
                info!(
                    "Added PlayerMovement component for walking mode in {:?} state",
                    *game_state.get()
                );
                return; // Return early to let the component be added, next frame will process movement
            }

            // Now we know the component exists, get it and apply physics/input based on game state
            if let Some(mut movement) = player_movement {
                if *game_state.get() == GameState::InGame {
                    // When actively playing: full movement with input processing, gravity, and physics
                    // Debug log to confirm we're processing InGame state
                    static mut LAST_INGAME_DEBUG: f64 = 0.0;
                    unsafe {
                        if current_time - LAST_INGAME_DEBUG > 3.0 {
                            LAST_INGAME_DEBUG = current_time;
                            info!(
                                "Processing InGame walking mode: grounded={}, gravity={}",
                                movement.is_grounded, movement.gravity_scale
                            );
                        }
                    }

                    let (mode_changed, should_skip_jump) = handle_double_spacebar_flight_toggle(
                        &time,
                        &keyboard_input,
                        &mut movement,
                        &mut player_mode,
                    );

                    if mode_changed {
                        return; // Mode changed, let the next frame handle the new mode
                    }

                    handle_physics_free_walking_movement(
                        &time,
                        &keyboard_input,
                        transform,
                        &mut movement,
                        &voxel_world,
                        should_skip_jump,
                    );
                } else if *game_state.get() == GameState::GameplayMenu {
                    // When in gameplay menu: only gravity, no input processing
                    handle_physics_free_walking_movement_menu_only(
                        &time,
                        transform,
                        &mut movement,
                        &voxel_world,
                    );
                }
            }
        }
    }
}

/// System to force walking mode when the world starts and ensure component is always added (web version)
fn force_walking_mode_on_world_start_web(
    mut player_mode: ResMut<PlayerMode>,
    mut commands: Commands,
    camera_query: Query<(Entity, Option<&PlayerMovement>), With<Camera>>,
    game_state: Res<State<GameState>>,
    time: Res<Time>, // Use Bevy's Time instead of SystemTime for WASM compatibility
) {
    // When the game is in InGame state, ensure we're in walking mode and have the component
    if *game_state.get() == GameState::InGame {
        static mut WORLD_STARTED: bool = false;
        static mut LAST_GAME_STATE: Option<GameState> = None;

        unsafe {
            // Check if we just transitioned to InGame state
            if LAST_GAME_STATE != Some(GameState::InGame) && *game_state.get() == GameState::InGame
            {
                if !WORLD_STARTED {
                    info!(
                        "World started - forcing Walking mode (was {:?})",
                        *player_mode
                    );
                    *player_mode = PlayerMode::Walking;
                    WORLD_STARTED = true;
                }
            }

            // ALWAYS ensure PlayerMovement component exists when in InGame with Walking mode
            if *player_mode == PlayerMode::Walking {
                // Handle multiple camera entities by taking the first one
                let camera_entities: Vec<_> = camera_query.iter().collect();

                if camera_entities.is_empty() {
                    static mut LAST_NO_CAMERA_DEBUG: f64 = 0.0;
                    let current_time = time.elapsed_secs_f64();
                    if current_time - LAST_NO_CAMERA_DEBUG > 5.0 {
                        LAST_NO_CAMERA_DEBUG = current_time;
                        info!("No camera entities found in InGame state");
                    }
                } else {
                    // Use the first camera entity (usually the main player camera)
                    let (camera_entity, movement_component) = camera_entities[0];

                    if movement_component.is_none() {
                        let mut movement = PlayerMovement::default();
                        movement.is_grounded = false; // Start not grounded to apply gravity
                        commands.entity(camera_entity).insert(movement);
                        info!(
                            "Added PlayerMovement component for InGame Walking mode (found {} camera entities)",
                            camera_entities.len()
                        );
                    } else {
                        // Debug: Component already exists
                        static mut LAST_EXISTS_DEBUG: f64 = 0.0;
                        let current_time = time.elapsed_secs_f64();
                        if current_time - LAST_EXISTS_DEBUG > 5.0 {
                            LAST_EXISTS_DEBUG = current_time;
                            info!(
                                "PlayerMovement component already exists in InGame state (found {} camera entities)",
                                camera_entities.len()
                            );
                        }
                    }
                }
            }

            LAST_GAME_STATE = Some(game_state.get().clone());
        }
    }
}

// Re-use the helper functions from the desktop player controller module

/// Enable gravity after the voxel world has been populated (web version)
fn enable_gravity_after_world_populated_web(
    voxel_world: Res<crate::environment::VoxelWorld>,
    mut camera_query: Query<(&mut Transform, &mut PlayerMovement), With<Camera>>,
) {
    if voxel_world.blocks.is_empty() {
        return; // World not ready yet
    }

    if let Ok((mut transform, mut movement)) = camera_query.single_mut() {
        if !movement.gravity_initialized {
            movement.is_grounded = false; // allow gravity to start affecting the player
            // Kickstart gravity slightly so it begins falling next frame
            if movement.gravity_scale >= 0.0 {
                movement.gravity_scale = -0.1;
            }
            movement.gravity_initialized = true;
            // Ensure the player is not below the ground unexpectedly
            if transform.translation.y < 1.5 {
                transform.translation.y = 3.0;
            }
            info!(
                "Web: Gravity initialized after world population ({} blocks)",
                voxel_world.blocks.len()
            );
            #[cfg(target_arch = "wasm32")]
            web_sys::console::log_1(
                &format!(
                    "Web: Gravity initialized after world population ({} blocks)",
                    voxel_world.blocks.len()
                )
                .into(),
            );
        }
    }
}

/// Handle double spacebar press to toggle flight mode (reused from desktop)
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

/// Re-use physics functions from desktop player controller
fn handle_physics_free_walking_movement(
    time: &Res<Time>,
    keyboard_input: &Res<ButtonInput<KeyCode>>,
    mut transform: Mut<Transform>,
    movement: &mut PlayerMovement,
    voxel_world: &Res<crate::environment::VoxelWorld>,
    should_skip_jump: bool,
) {
    // Call the desktop implementation
    crate::player_controller::handle_physics_free_walking_movement(
        time,
        keyboard_input,
        transform,
        movement,
        voxel_world,
        should_skip_jump,
    );
}

fn handle_physics_free_walking_movement_menu_only(
    time: &Res<Time>,
    mut transform: Mut<Transform>,
    movement: &mut PlayerMovement,
    voxel_world: &Res<crate::environment::VoxelWorld>,
) {
    // Call the desktop implementation
    crate::player_controller::handle_physics_free_walking_movement_menu_only(
        time,
        transform,
        movement,
        voxel_world,
    );
}
