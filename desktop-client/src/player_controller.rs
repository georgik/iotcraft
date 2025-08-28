// Import GameState - use desktop UI system for both desktop and web
use crate::ui::GameState;
use bevy::prelude::*;

/// Plugin for player controller functionality
pub struct PlayerControllerPlugin;

impl Plugin for PlayerControllerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerMode::Walking).add_systems(
            Update,
            (
                handle_mode_switch,
                player_movement,
                force_walking_mode_on_world_start,
            ),
        );
    }
}

/// Simulate a simple raycast hit result for voxel-based ground checking
#[derive(Debug, Copy, Clone)]
struct VoxelRayHit {
    distance: f32,
}

/// Check for ground in the voxel world by casting a ray downward
/// Returns Some(hit) if ground is found, None otherwise
fn check_voxel_ground(
    ray_origin: Vec3,
    ray_direction: Dir3,
    max_distance: f32,
    voxel_world: &crate::environment::VoxelWorld,
) -> Option<VoxelRayHit> {
    let step_size = 0.05; // Very fine-grained steps for accurate collision
    let ray_dir = ray_direction.as_vec3();

    let mut current_distance = 0.0;
    while current_distance <= max_distance {
        let check_position = ray_origin + ray_dir * current_distance;
        let voxel_pos = IVec3::new(
            check_position.x.floor() as i32,
            check_position.y.floor() as i32,
            check_position.z.floor() as i32,
        );

        // Check if there's a voxel block at this position
        if voxel_world.is_block_at(voxel_pos) {
            // Found a block! Return hit information
            return Some(VoxelRayHit {
                distance: current_distance,
            });
        }

        current_distance += step_size;
    }

    None // No ground found within max distance
}

/// Check if a position would collide with a voxel block
/// Uses smart collision detection that prevents camera from entering cubes while avoiding false positives
fn check_voxel_collision(
    position: Vec3,
    player_height: f32,
    player_radius: f32,
    voxel_world: &crate::environment::VoxelWorld,
) -> bool {
    // Player dimensions
    let player_center = position;
    let player_bottom = player_center - Vec3::new(0.0, player_height * 0.5, 0.0);
    let player_top = player_center + Vec3::new(0.0, player_height * 0.5, 0.0);
    let player_mid = player_center; // Eye level (camera position)

    // Use smaller buffer to prevent getting stuck - reduced from 0.3 to 0.15
    let collision_radius = player_radius + 0.15;

    // Only check critical collision points to avoid false positives
    let mut positions_to_check = Vec::new();

    // Check center positions at different heights
    positions_to_check.push(player_center);

    // Check 4 cardinal directions at mid-height (most important for horizontal movement)
    let cardinal_angles: [f32; 4] = [0.0, 90.0, 180.0, 270.0]; // N, E, S, W
    for angle_deg in cardinal_angles {
        let angle_rad = angle_deg.to_radians();
        let x_offset = collision_radius * angle_rad.cos();
        let z_offset = collision_radius * angle_rad.sin();
        positions_to_check.push(player_mid + Vec3::new(x_offset, 0.0, z_offset));
    }

    // Check corners at bottom level (for ground collision)
    let corner_positions = [
        player_bottom + Vec3::new(player_radius * 0.7, 0.0, player_radius * 0.7),
        player_bottom + Vec3::new(-player_radius * 0.7, 0.0, player_radius * 0.7),
        player_bottom + Vec3::new(player_radius * 0.7, 0.0, -player_radius * 0.7),
        player_bottom + Vec3::new(-player_radius * 0.7, 0.0, -player_radius * 0.7),
    ];
    positions_to_check.extend_from_slice(&corner_positions);

    // Check top corners (for ceiling collision)
    let top_corner_positions = [
        player_top + Vec3::new(player_radius * 0.5, 0.0, player_radius * 0.5),
        player_top + Vec3::new(-player_radius * 0.5, 0.0, player_radius * 0.5),
        player_top + Vec3::new(player_radius * 0.5, 0.0, -player_radius * 0.5),
        player_top + Vec3::new(-player_radius * 0.5, 0.0, -player_radius * 0.5),
    ];
    positions_to_check.extend_from_slice(&top_corner_positions);

    // Check all positions for collisions
    for check_pos in positions_to_check {
        let voxel_pos = IVec3::new(
            check_pos.x.floor() as i32,
            check_pos.y.floor() as i32,
            check_pos.z.floor() as i32,
        );

        if voxel_world.is_block_at(voxel_pos) {
            return true; // Collision detected
        }
    }

    false // No collision
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

/// Component for player movement configuration
#[derive(Component)]
pub struct PlayerMovement {
    pub walk_speed: f32,
    pub run_speed: f32,
    pub jump_force: f32,
    pub gravity_scale: f32, // repurposed for vertical velocity
    pub is_grounded: bool,
    pub ground_check_distance: f32,
    pub last_spacebar_press: Option<f64>,
    pub double_tap_window: f64,
}

impl Default for PlayerMovement {
    fn default() -> Self {
        Self {
            walk_speed: 5.0,
            run_speed: 12.0,
            jump_force: 5.5, // Set to 5.5 for ~1.5 cube height jump (cube size = 1.0)
            gravity_scale: 0.0,
            is_grounded: true,           // Assume grounded initially
            ground_check_distance: 10.0, // Increased from 1.2 to 10.0 for better ground detection
            last_spacebar_press: None,
            double_tap_window: 0.25,
        }
    }
}

/// System to handle mode switching with F4 key
fn handle_mode_switch(
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

/// System to handle player movement based on current mode
fn player_movement(
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
                "Player movement system running: mode={:?}, has_movement_component={}, game_state={:?}",
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

/// System to force walking mode when the world starts and ensure component is always added
fn force_walking_mode_on_world_start(
    mut player_mode: ResMut<PlayerMode>,
    mut commands: Commands,
    camera_query: Query<(Entity, Option<&PlayerMovement>), With<Camera>>,
    game_state: Res<State<GameState>>,
    time: Res<Time>, // Use Bevy's Time instead of SystemTime
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

/// Physics-free walking movement for menu state - only applies gravity, no input processing
/// Uses the same voxel-based collision system as the main gameplay to ensure consistency
pub fn handle_physics_free_walking_movement_menu_only(
    time: &Res<Time>,
    mut transform: Mut<Transform>,
    movement: &mut PlayerMovement,
    voxel_world: &Res<crate::environment::VoxelWorld>,
) {
    let dt = time.delta_secs();

    // Early return if VoxelWorld is empty - wait for world generation to complete
    if voxel_world.blocks.is_empty() {
        // Debug: Log every few seconds to see what's happening while waiting for world generation
        static mut LAST_EMPTY_WORLD_DEBUG: f64 = 0.0;
        let current_time = time.elapsed_secs_f64();
        unsafe {
            if current_time - LAST_EMPTY_WORLD_DEBUG > 1.0 {
                LAST_EMPTY_WORLD_DEBUG = current_time;
                info!(
                    "Menu mode: Waiting for world generation - VoxelWorld is empty, skipping collision detection"
                );
            }
        }
        // Keep player floating at a reasonable height until world loads
        if transform.translation.y < 5.0 {
            transform.translation.y = 5.0;
        }
        movement.is_grounded = false;
        movement.gravity_scale = 0.0; // Disable gravity until world loads
        return;
    }

    // Player dimensions - same as in the main gameplay function
    let player_height = 1.8;
    let player_radius = 0.3;
    let capsule_half_height = player_height * 0.5;

    // STEP 1: Ground detection using voxel-based collision (same as gameplay)
    let ray_origin = transform.translation + Vec3::new(0.0, -capsule_half_height, 0.0);
    let ray_direction = Dir3::NEG_Y;
    let max_distance = movement.ground_check_distance;

    let ground_check = check_voxel_ground(ray_origin, ray_direction, max_distance, voxel_world);

    // Update grounded state
    if let Some(ground_hit) = ground_check {
        let ground_point = ray_origin + (ray_direction.as_vec3() * ground_hit.distance);
        let ground_y = ground_point.y;
        let player_feet_y = transform.translation.y - capsule_half_height;

        let ground_tolerance = 0.15;
        movement.is_grounded = (player_feet_y - ground_y).abs() <= ground_tolerance;
    } else {
        movement.is_grounded = false;
    }

    // STEP 2: Apply gravity and vertical movement with collision detection (same as gameplay)
    let mut new_y_position = transform.translation.y;

    if !movement.is_grounded {
        // Apply gravity
        movement.gravity_scale += -9.81 * dt;
    } else {
        // Reset gravity when grounded
        if movement.gravity_scale < 0.0 {
            movement.gravity_scale = 0.0;
        }
    }

    // Apply vertical movement with collision checking
    if movement.gravity_scale != 0.0 {
        new_y_position += movement.gravity_scale * dt;

        let new_position = Vec3::new(
            transform.translation.x,
            new_y_position,
            transform.translation.z,
        );

        // Check for collision with new Y position
        if !check_voxel_collision(new_position, player_height, player_radius, voxel_world) {
            // Safe to move vertically
            transform.translation.y = new_y_position;
        } else {
            // Collision detected in Y direction
            if movement.gravity_scale < 0.0 {
                // Falling into ground - stop falling and set grounded
                movement.gravity_scale = 0.0;
                movement.is_grounded = true;

                // Find the exact ground level and place player on top
                if let Some(ground_hit) = ground_check {
                    let ground_point = ray_origin + (ray_direction.as_vec3() * ground_hit.distance);
                    let ground_y = ground_point.y;
                    transform.translation.y = ground_y + capsule_half_height + 0.05; // Small buffer
                }
            } else if movement.gravity_scale > 0.0 {
                // Hitting ceiling while jumping - stop upward movement
                movement.gravity_scale = 0.0;
            }
        }
    }

    // STEP 3: Final ground collision correction
    if let Some(ground_hit) = ground_check {
        let ground_point = ray_origin + (ray_direction.as_vec3() * ground_hit.distance);
        let ground_y = ground_point.y;
        let player_feet_y = transform.translation.y - capsule_half_height;

        // If player is inside or below the ground, push them up
        if player_feet_y <= ground_y {
            transform.translation.y = ground_y + capsule_half_height + 0.01;
            movement.gravity_scale = 0.0;
            movement.is_grounded = true;
        }
    }

    // STEP 4: Final collision check - if we're somehow still in a collision state, stabilize position
    if check_voxel_collision(
        transform.translation,
        player_height,
        player_radius,
        voxel_world,
    ) {
        // Find the nearest safe position above ground
        if let Some(ground_hit) = ground_check {
            let ground_point = ray_origin + (ray_direction.as_vec3() * ground_hit.distance);
            let ground_y = ground_point.y;
            transform.translation.y = ground_y + capsule_half_height + 0.1;
            movement.gravity_scale = 0.0;
            movement.is_grounded = true;

            // Rate-limit collision correction logging to prevent spam
            static mut LAST_CORRECTION_LOG: f64 = 0.0;
            let current_time = time.elapsed_secs_f64();
            unsafe {
                if current_time - LAST_CORRECTION_LOG > 1.0 {
                    LAST_CORRECTION_LOG = current_time;
                    info!("Menu mode: Corrected player position to avoid collision");
                }
            }
        }
    }
}

/// Physics-free walking movement that simulates gravity and collision via Transform manipulation
pub fn handle_physics_free_walking_movement(
    time: &Res<Time>,
    keyboard_input: &Res<ButtonInput<KeyCode>>,
    mut transform: Mut<Transform>,
    movement: &mut PlayerMovement,
    voxel_world: &Res<crate::environment::VoxelWorld>,
    should_skip_jump: bool,
) {
    let dt = time.delta_secs();

    // Early return if VoxelWorld is empty - wait for world generation to complete
    if voxel_world.blocks.is_empty() {
        // Debug: Log every few seconds to see what's happening while waiting for world generation
        static mut LAST_EMPTY_WORLD_DEBUG: f64 = 0.0;
        let current_time = time.elapsed_secs_f64();
        unsafe {
            if current_time - LAST_EMPTY_WORLD_DEBUG > 1.0 {
                LAST_EMPTY_WORLD_DEBUG = current_time;
                info!(
                    "Waiting for world generation - VoxelWorld is empty, skipping collision detection"
                );
            }
        }
        // Keep player floating at a reasonable height until world loads
        if transform.translation.y < 5.0 {
            transform.translation.y = 5.0;
        }
        movement.is_grounded = false;
        movement.gravity_scale = 0.0; // Disable gravity until world loads
        return;
    }

    // Debug: Log every few seconds to see what's happening
    static mut LAST_DEBUG: f64 = 0.0;
    let current_time = time.elapsed_secs_f64();
    unsafe {
        if current_time - LAST_DEBUG > 2.0 {
            LAST_DEBUG = current_time;
            info!(
                "Walking mode debug: pos={:?}, grounded={}, gravity={}, world_blocks={}",
                transform.translation,
                movement.is_grounded,
                movement.gravity_scale,
                voxel_world.blocks.len()
            );
        }
    }

    // Player dimensions
    let player_height = 1.8;
    let player_radius = 0.3;
    let capsule_half_height = player_height * 0.5;

    // Store original position for collision rollback
    let original_position = transform.translation;

    // STEP 1: Ground detection and grounded state update
    let ray_origin = transform.translation + Vec3::new(0.0, -capsule_half_height, 0.0);
    let ray_direction = Dir3::NEG_Y;
    let max_distance = movement.ground_check_distance;

    let ground_check = check_voxel_ground(ray_origin, ray_direction, max_distance, voxel_world);

    // Update grounded state
    let _was_grounded = movement.is_grounded;
    if let Some(ground_hit) = ground_check {
        let ground_point = ray_origin + (ray_direction.as_vec3() * ground_hit.distance);
        let ground_y = ground_point.y;
        let player_feet_y = transform.translation.y - capsule_half_height;

        let ground_tolerance = 0.15;
        movement.is_grounded = (player_feet_y - ground_y).abs() <= ground_tolerance;
    } else {
        movement.is_grounded = false;
    }

    // Debug ground check and collision detection
    if keyboard_input.just_pressed(KeyCode::KeyG) {
        info!("=== DEBUG COLLISION SYSTEM ===");
        info!(
            "Ground check: grounded={}, pos={:?}",
            movement.is_grounded, transform.translation
        );
        info!(
            "Ray origin: {:?}, direction: {:?}, max_distance: {}",
            ray_origin, ray_direction, max_distance
        );

        if let Some(ground_hit) = ground_check {
            info!("Ground hit: distance={}", ground_hit.distance);
        } else {
            info!("No ground found within {} units", max_distance);
        }

        // Test collision detection at current position
        let collision_at_current = check_voxel_collision(
            transform.translation,
            player_height,
            player_radius,
            voxel_world,
        );
        info!("Collision at current position: {}", collision_at_current);

        // Test some specific voxel positions around the player
        let player_voxel = IVec3::new(
            transform.translation.x.floor() as i32,
            transform.translation.y.floor() as i32,
            transform.translation.z.floor() as i32,
        );
        info!("Player voxel position: {:?}", player_voxel);

        // Debug voxel world info
        info!("VoxelWorld total blocks: {}", voxel_world.blocks.len());

        // Show a few sample blocks from the voxel world
        let mut sample_count = 0;
        for (pos, block_type) in voxel_world.blocks.iter() {
            info!("Sample block: {:?} -> {:?}", pos, block_type);
            sample_count += 1;
            if sample_count >= 5 {
                break;
            }
        }

        // Check blocks around player position
        for y_offset in -10..3 {
            // Increased range to check more blocks below
            for x_offset in -1..2 {
                for z_offset in -1..2 {
                    let check_pos = player_voxel + IVec3::new(x_offset, y_offset, z_offset);
                    let has_block = voxel_world.is_block_at(check_pos);
                    if has_block {
                        info!("Block found at {:?}", check_pos);
                    }
                }
            }
        }

        // Specifically check the ground level where blocks should be
        let ground_positions = [
            IVec3::new(15, 0, 15),   // Should be grass from "wall grass -15 0 -15 15 0 15"
            IVec3::new(0, 0, 0),     // Center of grass area
            IVec3::new(-15, 0, -15), // Corner of grass area
            IVec3::new(15, 0, -15),  // Another corner
        ];

        for pos in ground_positions {
            let has_block = voxel_world.is_block_at(pos);
            info!("Ground check at {:?}: has_block={}", pos, has_block);
        }
        info!("=== END DEBUG ===");
    }

    // STEP 2: Handle horizontal movement with collision detection
    let mut movement_input = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::KeyW) {
        movement_input += transform.forward().as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        movement_input -= transform.forward().as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        movement_input -= transform.right().as_vec3();
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        movement_input += transform.right().as_vec3();
    }

    // Normalize and apply horizontal movement
    if movement_input.length() > 0.0 {
        movement_input = movement_input.normalize();

        let current_speed = if keyboard_input.pressed(KeyCode::ShiftLeft) {
            movement.run_speed
        } else {
            movement.walk_speed
        };

        let horizontal_movement = movement_input * current_speed * dt;
        let new_horizontal_position = Vec3::new(
            transform.translation.x + horizontal_movement.x,
            transform.translation.y, // Don't change Y yet
            transform.translation.z + horizontal_movement.z,
        );

        // Check if new horizontal position would cause collision
        if !check_voxel_collision(
            new_horizontal_position,
            player_height,
            player_radius,
            voxel_world,
        ) {
            // Safe to move horizontally
            transform.translation.x = new_horizontal_position.x;
            transform.translation.z = new_horizontal_position.z;
        } else {
            // Collision detected, try moving in individual axes
            let x_only_pos = Vec3::new(
                transform.translation.x + horizontal_movement.x,
                transform.translation.y,
                transform.translation.z,
            );
            let z_only_pos = Vec3::new(
                transform.translation.x,
                transform.translation.y,
                transform.translation.z + horizontal_movement.z,
            );

            // Try X movement only
            if !check_voxel_collision(x_only_pos, player_height, player_radius, voxel_world) {
                transform.translation.x = x_only_pos.x;
            }

            // Try Z movement only
            if !check_voxel_collision(z_only_pos, player_height, player_radius, voxel_world) {
                transform.translation.z = z_only_pos.z;
            }
        }
    }

    // STEP 3: Handle jumping
    if keyboard_input.just_pressed(KeyCode::Space) && movement.is_grounded && !should_skip_jump {
        movement.gravity_scale = movement.jump_force;
        movement.is_grounded = false;
        info!("Player jumped with velocity: {}!", movement.jump_force);
    }

    // STEP 4: Apply gravity and vertical movement with collision detection
    let mut new_y_position = transform.translation.y;

    if !movement.is_grounded {
        // Apply gravity
        movement.gravity_scale += -9.81 * dt;
    } else {
        // Reset gravity when grounded
        if movement.gravity_scale < 0.0 {
            movement.gravity_scale = 0.0;
        }
    }

    // Apply vertical movement
    if movement.gravity_scale != 0.0 {
        new_y_position += movement.gravity_scale * dt;

        let new_position = Vec3::new(
            transform.translation.x,
            new_y_position,
            transform.translation.z,
        );

        // Check for collision with new Y position
        if !check_voxel_collision(new_position, player_height, player_radius, voxel_world) {
            // Safe to move vertically
            transform.translation.y = new_y_position;
        } else {
            // Collision detected in Y direction
            if movement.gravity_scale < 0.0 {
                // Falling into ground - stop falling and set grounded
                movement.gravity_scale = 0.0;
                movement.is_grounded = true;

                // Find the exact ground level and place player on top
                if let Some(ground_hit) = ground_check {
                    let ground_point = ray_origin + (ray_direction.as_vec3() * ground_hit.distance);
                    let ground_y = ground_point.y;
                    transform.translation.y = ground_y + capsule_half_height + 0.05; // Small buffer
                }
            } else if movement.gravity_scale > 0.0 {
                // Hitting ceiling while jumping - stop upward movement
                movement.gravity_scale = 0.0;
            }
        }
    }

    // STEP 5: Final ground collision correction
    if let Some(ground_hit) = ground_check {
        let ground_point = ray_origin + (ray_direction.as_vec3() * ground_hit.distance);
        let ground_y = ground_point.y;
        let player_feet_y = transform.translation.y - capsule_half_height;

        // If player is inside or below the ground, push them up
        if player_feet_y <= ground_y {
            transform.translation.y = ground_y + capsule_half_height + 0.01;
            movement.gravity_scale = 0.0;
            movement.is_grounded = true;
        }
    }

    // STEP 6: Final collision check - if we're somehow still in a collision state, revert
    if check_voxel_collision(
        transform.translation,
        player_height,
        player_radius,
        voxel_world,
    ) {
        // Something went wrong, revert to original position
        info!(
            "Emergency collision revert from {:?} to {:?}",
            transform.translation, original_position
        );
        transform.translation = original_position;
        movement.gravity_scale = 0.0;
        movement.is_grounded = true;
    }
}
