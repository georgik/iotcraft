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
            max_static_colliders: 200, // Reduced for better performance
            collider_distance: 20.0,   // Reduced radius for better performance
            world_floor: -3.0,         // Don't let player fall below -3
        }
    }
}

/// Marker component for blocks that have physics colliders
#[derive(Component)]
pub struct HasPhysicsCollider;

/// Configure physics settings for optimal performance
fn configure_physics_settings(mut commands: Commands) {
    // Configure physics for better performance with static world
    commands.insert_resource(Gravity(Vec3::new(0.0, -9.81 * 2.0, 0.0))); // Stronger gravity for snappy feel

    info!("Physics configured for optimal voxel world performance");
}

/// System to prevent player from falling through the world
fn prevent_player_fall_through_world(
    mut player_query: Query<&mut Transform, With<Camera>>,
    physics_config: Res<PhysicsConfig>,
) {
    if let Ok(mut transform) = player_query.single_mut() {
        if transform.translation.y < physics_config.world_floor {
            transform.translation.y = physics_config.world_floor + 5.0; // Teleport 5 units above floor
            info!("Player teleported back up from falling through world");
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
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
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

    for (entity, _distance) in nearby_blocks.into_iter().take(remaining_slots) {
        commands.entity(entity).insert((
            RigidBody::Static,
            Collider::cuboid(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE),
            // Optimize static colliders for performance
            ColliderDensity(1.0), // Explicit density for consistent behavior
            Friction::new(0.7).with_combine_rule(CoefficientCombine::Max),
            Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
            HasPhysicsCollider,
        ));
    }
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
        Collider::cuboid(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE),
        HasPhysicsCollider,
    ));
}
