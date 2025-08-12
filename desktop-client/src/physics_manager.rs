use crate::environment::{CUBE_SIZE, VoxelBlock};
use crate::player_controller::PlayerMode;
use avian3d::prelude::*;
use bevy::prelude::*;

/// Plugin for managing physics optimization
pub struct PhysicsManagerPlugin;

impl Plugin for PhysicsManagerPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhysicsConfig::default())
            .add_systems(Startup, configure_physics_settings)
            .add_systems(
                Update,
                (
                    manage_block_physics_distance_based,
                    prevent_player_fall_through_world,
                    // Only run when physics is enabled
                )
                    .run_if(|mode: Res<PlayerMode>| *mode == PlayerMode::Walking),
            );
    }
}

/// Configuration for physics optimization
#[derive(Resource)]
pub struct PhysicsConfig {
    /// Maximum number of static colliders to keep active
    pub max_static_colliders: usize,
    /// Distance from player to keep colliders active
    pub collider_distance: f32,
    /// Minimum Y position before teleporting player up
    pub world_floor: f32,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            max_static_colliders: 500, // Increased to prevent falling through blocks
            collider_distance: 25.0,   // Increased radius for better collision coverage
            world_floor: -20.0,        // Don't let player fall below -20 as requested
        }
    }
}

/// Marker component for blocks that have physics colliders
#[derive(Component)]
pub struct HasPhysicsCollider;

/// Configure physics settings for optimal performance
fn configure_physics_settings(mut commands: Commands) {
    // Configure physics for better performance with static world
    commands.insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0))); // Normal gravity strength

    info!("Physics configured for optimal voxel world performance");
}

/// System to prevent player from falling through the world
fn prevent_player_fall_through_world(
    mut player_query: Query<(&mut Transform, Option<&mut LinearVelocity>), With<Camera>>,
    physics_config: Res<PhysicsConfig>,
    voxel_world: Res<crate::environment::VoxelWorld>,
) {
    if let Ok((mut transform, velocity)) = player_query.single_mut() {
        if transform.translation.y < physics_config.world_floor {
            // Find a safe spawn position above solid ground
            let safe_position = find_safe_spawn_position(&voxel_world, IVec3::new(0, 0, 0));
            transform.translation = safe_position;

            // Reset velocity to prevent continuous falling
            if let Some(mut vel) = velocity {
                vel.0 = Vec3::ZERO;
                info!(
                    "Player teleported to safe coordinates {:?} and velocity reset",
                    safe_position
                );
            } else {
                info!(
                    "Player teleported to safe coordinates {:?} from falling through world",
                    safe_position
                );
            }
        }
    }
}

/// System to manage which blocks have physics colliders based on player position
/// Only runs in walking mode for optimal performance
fn manage_block_physics_distance_based(
    mut commands: Commands,
    physics_config: Res<PhysicsConfig>,
    camera_query: Query<&Transform, With<Camera>>,
    // Blocks with colliders
    blocks_with_physics: Query<(Entity, &Transform), (With<VoxelBlock>, With<HasPhysicsCollider>)>,
    // Blocks without colliders
    blocks_without_physics: Query<
        (Entity, &Transform),
        (With<VoxelBlock>, Without<HasPhysicsCollider>),
    >,
) {
    // Handle multiple camera entities by taking the first one
    let camera_entities: Vec<_> = camera_query.iter().collect();
    if camera_entities.is_empty() {
        return;
    }
    let camera_transform = camera_entities[0];
    let player_pos = camera_transform.translation;

    // Remove colliders from blocks that are too far away
    for (entity, transform) in &blocks_with_physics {
        let distance = player_pos.distance(transform.translation);
        if distance > physics_config.collider_distance {
            commands
                .entity(entity)
                .remove::<RigidBody>()
                .remove::<Collider>()
                .remove::<HasPhysicsCollider>();
        }
    }

    // Add colliders to nearby blocks (up to our limit)
    let mut nearby_blocks: Vec<_> = blocks_without_physics
        .iter()
        .filter_map(|(entity, transform)| {
            let distance = player_pos.distance(transform.translation);
            if distance <= physics_config.collider_distance {
                Some((entity, distance))
            } else {
                None
            }
        })
        .collect();

    // Sort by distance (closest first) for best performance
    nearby_blocks.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Add colliders to the closest blocks (up to our limit)
    let current_collider_count = blocks_with_physics.iter().count();
    let remaining_slots = physics_config
        .max_static_colliders
        .saturating_sub(current_collider_count);

    // Debug logging for physics management
    static mut LAST_PHYSICS_DEBUG: f64 = 0.0;
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    unsafe {
        if current_time - LAST_PHYSICS_DEBUG > 3.0 {
            LAST_PHYSICS_DEBUG = current_time;
            info!(
                "Physics manager: player_pos={:?}, current_colliders={}, remaining_slots={}, nearby_blocks={}",
                player_pos,
                current_collider_count,
                remaining_slots,
                nearby_blocks.len()
            );
        }
    }

    let added_count = nearby_blocks.len().min(remaining_slots);
    for (entity, distance) in nearby_blocks.into_iter().take(remaining_slots) {
        commands.entity(entity).insert((
            RigidBody::Static,
            // Use half-extents for cuboid colliders (CUBE_SIZE is 1.0, so half-extents are 0.5)
            Collider::cuboid(CUBE_SIZE / 2.0, CUBE_SIZE / 2.0, CUBE_SIZE / 2.0),
            // Optimize static colliders for performance
            ColliderDensity(1.0), // Explicit density for consistent behavior
            Friction::new(0.7).with_combine_rule(CoefficientCombine::Max),
            Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
            HasPhysicsCollider,
        ));
    }

    // Debug: Log when colliders are added
    if added_count > 0 {
        unsafe {
            if current_time - LAST_PHYSICS_DEBUG > 3.0 {
                info!(
                    "Added {} physics colliders to blocks near player",
                    added_count
                );
            }
        }
    }
}

/// Find a safe spawn position above solid ground
fn find_safe_spawn_position(
    voxel_world: &crate::environment::VoxelWorld,
    preferred_pos: IVec3,
) -> Vec3 {
    // Try the preferred position first
    if let Some(safe_y) = find_surface_y(voxel_world, preferred_pos.x, preferred_pos.z) {
        return Vec3::new(preferred_pos.x as f32, safe_y + 2.0, preferred_pos.z as f32);
    }

    // If preferred position doesn't work, try nearby positions in a spiral pattern
    for radius in 1i32..=10i32 {
        for x_offset in -radius..=radius {
            for z_offset in -radius..=radius {
                // Only check the perimeter of the current radius
                if x_offset.abs() == radius || z_offset.abs() == radius {
                    let test_x = preferred_pos.x + x_offset;
                    let test_z = preferred_pos.z + z_offset;

                    if let Some(safe_y) = find_surface_y(voxel_world, test_x, test_z) {
                        return Vec3::new(test_x as f32, safe_y + 2.0, test_z as f32);
                    }
                }
            }
        }
    }

    // Fallback to a safe position in the air above spawn
    Vec3::new(0.0, 10.0, 0.0)
}

/// Find the Y coordinate of the highest solid block at the given X,Z coordinates
fn find_surface_y(voxel_world: &crate::environment::VoxelWorld, x: i32, z: i32) -> Option<f32> {
    // Check from a reasonable height down to below ground
    for y in ((-20)..=20).rev() {
        let pos = IVec3::new(x, y, z);
        if voxel_world.is_block_at(pos) {
            // Found a solid block, the surface is one block above
            return Some((y + 1) as f32);
        }
    }
    None
}

/// Helper function to add physics collider to a block entity (used by block creation systems)
pub fn add_physics_to_block_if_needed(
    commands: &mut Commands,
    entity: Entity,
    player_mode: &PlayerMode,
    player_position: Option<Vec3>,
    block_position: Vec3,
    max_distance: f32,
) {
    // Only add physics in walking mode
    if !matches!(*player_mode, PlayerMode::Walking) {
        return;
    }

    // Check distance if player position is provided
    if let Some(player_pos) = player_position {
        let distance = player_pos.distance(block_position);
        if distance > max_distance {
            return;
        }
    }

    // Add physics components
    commands.entity(entity).insert((
        RigidBody::Static,
        // Use half-extents for cuboid colliders (CUBE_SIZE is 1.0, so half-extents are 0.5)
        Collider::cuboid(CUBE_SIZE / 2.0, CUBE_SIZE / 2.0, CUBE_SIZE / 2.0),
        HasPhysicsCollider,
    ));
}
