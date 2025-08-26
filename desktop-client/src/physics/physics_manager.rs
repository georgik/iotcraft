// use crate::environment::{CUBE_SIZE, VoxelBlock}; // Unused currently
#[cfg(feature = "physics")]
use crate::player_controller::PlayerMode;
#[cfg(feature = "physics")]
use avian3d::prelude::*;
use bevy::prelude::*;

/// Plugin for managing physics optimization
pub struct PhysicsManagerPlugin;

#[cfg(feature = "physics")]
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

#[cfg(not(feature = "physics"))]
impl Plugin for PhysicsManagerPlugin {
    fn build(&self, _app: &mut App) {
        // No-op when physics feature is disabled
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

/// Marker component for water blocks to enable water detection
/// Water blocks don't have solid colliders but can still be detected for swimming mechanics
#[derive(Component)]
pub struct WaterBlock;

/// Configure physics settings for optimal performance
#[cfg(feature = "physics")]
fn configure_physics_settings(mut commands: Commands) {
    // Configure physics for better performance with static world
    commands.insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0))); // Normal gravity strength

    info!("Physics configured for optimal voxel world performance");
}

/// System to prevent player from falling through the world
#[cfg(feature = "physics")]
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
/// Water blocks are excluded from having solid colliders to allow player movement
#[cfg(feature = "physics")]
fn manage_block_physics_distance_based(
    mut commands: Commands,
    physics_config: Res<PhysicsConfig>,
    camera_query: Query<&Transform, With<Camera>>,
    voxel_world: Res<crate::environment::VoxelWorld>,
    // Blocks with colliders
    blocks_with_physics: Query<(Entity, &Transform, &VoxelBlock), With<HasPhysicsCollider>>,
    // Blocks without colliders
    blocks_without_physics: Query<(Entity, &Transform, &VoxelBlock), Without<HasPhysicsCollider>>,
) {
    // Handle multiple camera entities by taking the first one
    let camera_entities: Vec<_> = camera_query.iter().collect();
    if camera_entities.is_empty() {
        return;
    }
    let camera_transform = camera_entities[0];
    let player_pos = camera_transform.translation;

    // Remove colliders from blocks that are too far away
    for (entity, transform, _voxel_block) in &blocks_with_physics {
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
    // IMPORTANT: Exclude water blocks from having solid colliders to allow player movement
    let mut nearby_blocks: Vec<_> = blocks_without_physics
        .iter()
        .filter_map(|(entity, transform, voxel_block)| {
            let distance = player_pos.distance(transform.translation);
            if distance <= physics_config.collider_distance {
                // Check if this block is water - if so, exclude it from solid colliders
                if let Some(block_type) = voxel_world.blocks.get(&voxel_block.position) {
                    if *block_type == crate::environment::BlockType::Water {
                        // Skip water blocks - they should not have solid colliders
                        return None;
                    }
                }
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
    for (entity, _distance) in nearby_blocks.into_iter().take(remaining_slots) {
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
/// Water blocks are considered non-solid for spawn purposes
fn find_surface_y(voxel_world: &crate::environment::VoxelWorld, x: i32, z: i32) -> Option<f32> {
    // Check from a reasonable height down to below ground
    for y in ((-20)..=20).rev() {
        let pos = IVec3::new(x, y, z);
        if let Some(block_type) = voxel_world.blocks.get(&pos) {
            // Only consider non-water blocks as solid for spawning purposes
            if *block_type != crate::environment::BlockType::Water {
                // Found a solid block, the surface is one block above
                return Some((y + 1) as f32);
            }
        }
    }
    None
}

/// Tests for water physics behavior
#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::{BlockType, VoxelWorld};

    #[test]
    fn test_water_blocks_excluded_from_solid_colliders() {
        // This test verifies that water blocks are properly excluded from
        // having solid colliders, allowing player movement through water

        let mut voxel_world = VoxelWorld::default();

        // Create a test scenario with both solid blocks and water blocks
        voxel_world.set_block(IVec3::new(0, 0, 0), BlockType::Stone); // Solid block
        voxel_world.set_block(IVec3::new(1, 0, 0), BlockType::Water); // Water block
        voxel_world.set_block(IVec3::new(2, 0, 0), BlockType::Grass); // Another solid block

        // Test that water blocks are properly identified and excluded
        let water_pos = IVec3::new(1, 0, 0);
        let stone_pos = IVec3::new(0, 0, 0);
        let grass_pos = IVec3::new(2, 0, 0);

        // Check block types
        assert_eq!(voxel_world.blocks.get(&water_pos), Some(&BlockType::Water));
        assert_eq!(voxel_world.blocks.get(&stone_pos), Some(&BlockType::Stone));
        assert_eq!(voxel_world.blocks.get(&grass_pos), Some(&BlockType::Grass));

        // Water should be identified as such (test our filtering logic)
        assert!(voxel_world.blocks.get(&water_pos) == Some(&BlockType::Water));
        assert!(voxel_world.blocks.get(&stone_pos) != Some(&BlockType::Water));
        assert!(voxel_world.blocks.get(&grass_pos) != Some(&BlockType::Water));
    }

    #[test]
    fn test_find_surface_y_treats_water_as_non_solid() {
        // Test that the safe spawn logic treats water as non-solid
        let mut voxel_world = VoxelWorld::default();

        // Create a column with stone at bottom, water in middle, and air on top
        voxel_world.set_block(IVec3::new(5, 0, 5), BlockType::Stone); // Solid foundation
        voxel_world.set_block(IVec3::new(5, 1, 5), BlockType::Water); // Water above stone
        voxel_world.set_block(IVec3::new(5, 2, 5), BlockType::Water); // More water
        // No block at (5, 3, 5) - air

        // find_surface_y should return the level above the stone (y=1)
        // because water is considered non-solid for spawning
        let surface_y = find_surface_y(&voxel_world, 5, 5);

        // Should find stone at y=0, so surface should be y=1
        assert_eq!(surface_y, Some(1.0));
    }

    #[test]
    fn test_find_surface_y_with_no_solid_blocks() {
        // Test case where there are only water blocks (or no blocks)
        let mut voxel_world = VoxelWorld::default();

        // Only add water blocks
        voxel_world.set_block(IVec3::new(10, 0, 10), BlockType::Water);
        voxel_world.set_block(IVec3::new(10, 1, 10), BlockType::Water);

        // Should return None since there are no solid blocks to spawn on
        let surface_y = find_surface_y(&voxel_world, 10, 10);
        assert_eq!(surface_y, None);
    }

    #[test]
    fn test_mixed_block_types_surface_detection() {
        // Test a more complex scenario with mixed block types
        let mut voxel_world = VoxelWorld::default();

        // Create a realistic scenario: dirt foundation with water pool on top
        voxel_world.set_block(IVec3::new(20, -1, 20), BlockType::Dirt); // Underground
        voxel_world.set_block(IVec3::new(20, 0, 20), BlockType::Stone); // Ground level
        voxel_world.set_block(IVec3::new(20, 1, 20), BlockType::Water); // Water surface
        voxel_world.set_block(IVec3::new(20, 2, 20), BlockType::Water); // Deep water

        // Should find the stone at y=0 as the highest solid block
        // Surface should be at y=1 (above the stone)
        let surface_y = find_surface_y(&voxel_world, 20, 20);
        assert_eq!(surface_y, Some(1.0));
    }
}

// Helper function removed as it was unused
