use crate::environment::{BlockType, CUBE_SIZE, VoxelBlock, VoxelWorld};
use crate::player_controller::PlayerMode;
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use rayon::prelude::*;

/// Plugin for parallel physics management
pub struct ParallelPhysicsPlugin;

impl Plugin for ParallelPhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhysicsConfig::default())
            .insert_resource(PhysicsTaskManager::default())
            .add_systems(Startup, configure_parallel_physics)
            .add_systems(
                Update,
                (
                    start_parallel_physics_update,
                    apply_physics_updates,
                    prevent_player_fall_through_world,
                )
                    .run_if(|mode: Res<PlayerMode>| *mode == PlayerMode::Walking),
            );
    }
}

/// Configuration for parallel physics optimization
#[derive(Resource, Clone)]
pub struct PhysicsConfig {
    /// Maximum number of static colliders to keep active
    pub max_static_colliders: usize,
    /// Distance from player to keep colliders active
    pub collider_distance: f32,
    /// Minimum Y position before teleporting player up
    pub world_floor: f32,
    /// Number of blocks to process per batch
    pub batch_size: usize,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            max_static_colliders: 500,
            collider_distance: 25.0,
            world_floor: -20.0,
            batch_size: 100, // Process blocks in batches for better performance
        }
    }
}

/// Manages async physics computation tasks
#[derive(Resource, Default)]
pub struct PhysicsTaskManager {
    pub active_tasks: Vec<PhysicsUpdateTask>,
}

/// Component for tracking async physics update tasks
#[derive(Component)]
pub struct PhysicsUpdateTask {
    pub task: Task<PhysicsUpdateResult>,
    pub player_position: Vec3,
}

/// Result from parallel physics computation
#[derive(Debug)]
pub struct PhysicsUpdateResult {
    pub entities_to_add_colliders: Vec<Entity>,
    pub entities_to_remove_colliders: Vec<Entity>,
    pub computation_time_ms: f32,
}

/// Data structure for parallel physics computation
#[derive(Clone)]
pub struct PhysicsBlockData {
    pub entity: Entity,
    pub position: Vec3,
    pub block_type: BlockType,
    pub has_collider: bool,
}

/// Marker component for blocks that have physics colliders
#[derive(Component)]
pub struct HasPhysicsCollider;

/// Marker component for water blocks
#[derive(Component)]
pub struct WaterBlock;

/// Configure physics settings for optimal parallel performance
fn configure_parallel_physics(mut commands: Commands) {
    commands.insert_resource(Gravity(Vec3::new(0.0, -9.81, 0.0)));
    info!("Parallel physics configured for optimal performance");
}

/// System to prevent player from falling through the world
fn prevent_player_fall_through_world(
    mut player_query: Query<(&mut Transform, Option<&mut LinearVelocity>), With<Camera>>,
    physics_config: Res<PhysicsConfig>,
    voxel_world: Res<VoxelWorld>,
) {
    if let Ok((mut transform, velocity)) = player_query.single_mut() {
        if transform.translation.y < physics_config.world_floor {
            let safe_position = find_safe_spawn_position(&voxel_world, IVec3::new(0, 0, 0));
            transform.translation = safe_position;

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

/// Start parallel physics update computation
fn start_parallel_physics_update(
    mut commands: Commands,
    mut task_manager: ResMut<PhysicsTaskManager>,
    physics_config: Res<PhysicsConfig>,
    camera_query: Query<&Transform, With<Camera>>,
    voxel_world: Res<VoxelWorld>,
    blocks_with_physics: Query<(Entity, &Transform, &VoxelBlock), With<HasPhysicsCollider>>,
    blocks_without_physics: Query<(Entity, &Transform, &VoxelBlock), Without<HasPhysicsCollider>>,
) {
    // Don't start new tasks if we already have one running
    if !task_manager.active_tasks.is_empty() {
        return;
    }

    let camera_entities: Vec<_> = camera_query.iter().collect();
    if camera_entities.is_empty() {
        return;
    }
    let player_pos = camera_entities[0].translation;

    // Collect all block data for parallel processing
    let mut all_blocks: Vec<PhysicsBlockData> = Vec::new();

    // Add blocks that currently have colliders
    for (entity, transform, voxel_block) in &blocks_with_physics {
        if let Some(block_type) = voxel_world.blocks.get(&voxel_block.position) {
            all_blocks.push(PhysicsBlockData {
                entity,
                position: transform.translation,
                block_type: *block_type,
                has_collider: true,
            });
        }
    }

    // Add blocks that don't have colliders
    for (entity, transform, voxel_block) in &blocks_without_physics {
        if let Some(block_type) = voxel_world.blocks.get(&voxel_block.position) {
            all_blocks.push(PhysicsBlockData {
                entity,
                position: transform.translation,
                block_type: *block_type,
                has_collider: false,
            });
        }
    }

    let task_pool = AsyncComputeTaskPool::get();
    let config = (*physics_config).clone();

    let task = task_pool.spawn(async move {
        let start_time = std::time::Instant::now();

        // Use parallel processing with rayon for high-performance computation
        let results: Vec<_> = all_blocks
            .par_chunks(config.batch_size)
            .flat_map(|chunk| {
                chunk.par_iter().filter_map(|block_data| {
                    let distance = player_pos.distance(block_data.position);

                    // Exclude water blocks from having solid colliders
                    let should_have_collider = distance <= config.collider_distance
                        && block_data.block_type != BlockType::Water;

                    match (block_data.has_collider, should_have_collider) {
                        (true, false) => Some((block_data.entity, false)), // Remove collider
                        (false, true) => Some((block_data.entity, true)),  // Add collider
                        _ => None,                                         // No change needed
                    }
                })
            })
            .collect();

        // Split results into add/remove lists
        let mut entities_to_add_colliders = Vec::new();
        let mut entities_to_remove_colliders = Vec::new();

        for (entity, should_add) in results {
            if should_add {
                entities_to_add_colliders.push(entity);
            } else {
                entities_to_remove_colliders.push(entity);
            }
        }

        // Limit the number of colliders we add to stay within budget
        let current_colliders = all_blocks.iter().filter(|b| b.has_collider).count();
        let remaining_slots = config
            .max_static_colliders
            .saturating_sub(current_colliders - entities_to_remove_colliders.len());

        entities_to_add_colliders.truncate(remaining_slots);

        let computation_time = start_time.elapsed().as_secs_f32() * 1000.0;

        PhysicsUpdateResult {
            entities_to_add_colliders,
            entities_to_remove_colliders,
            computation_time_ms: computation_time,
        }
    });

    // Store the task for later completion
    task_manager.active_tasks.push(PhysicsUpdateTask {
        task,
        player_position: player_pos,
    });
}

/// Apply the results from parallel physics computation
fn apply_physics_updates(mut commands: Commands, mut task_manager: ResMut<PhysicsTaskManager>) {
    // Check completed tasks
    task_manager.active_tasks.retain_mut(|task| {
        if let Some(result) = bevy::tasks::block_on(bevy::tasks::poll_once(&mut task.task)) {
            // Apply the computed physics updates
            let add_count = result.entities_to_add_colliders.len();
            let remove_count = result.entities_to_remove_colliders.len();

            for entity in &result.entities_to_remove_colliders {
                commands
                    .entity(*entity)
                    .remove::<RigidBody>()
                    .remove::<Collider>()
                    .remove::<HasPhysicsCollider>();
            }

            for entity in &result.entities_to_add_colliders {
                commands.entity(*entity).insert((
                    RigidBody::Static,
                    Collider::cuboid(CUBE_SIZE / 2.0, CUBE_SIZE / 2.0, CUBE_SIZE / 2.0),
                    ColliderDensity(1.0),
                    Friction::new(0.7).with_combine_rule(CoefficientCombine::Max),
                    Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
                    HasPhysicsCollider,
                ));
            }

            // Log performance metrics
            debug!(
                "Parallel physics update completed: +{} -{} colliders in {:.2}ms",
                add_count, remove_count, result.computation_time_ms
            );

            false // Remove this completed task
        } else {
            true // Keep this task (still running)
        }
    });
}

/// Find a safe spawn position above solid ground (optimized version)
fn find_safe_spawn_position(voxel_world: &VoxelWorld, preferred_pos: IVec3) -> Vec3 {
    // Use parallel search for better performance
    let search_positions: Vec<IVec3> = (1i32..=10i32)
        .flat_map(|radius| {
            (-radius..=radius).flat_map(move |x_offset| {
                (-radius..=radius).filter_map(move |z_offset| {
                    if x_offset.abs() == radius || z_offset.abs() == radius {
                        Some(IVec3::new(
                            preferred_pos.x + x_offset,
                            preferred_pos.y,
                            preferred_pos.z + z_offset,
                        ))
                    } else {
                        None
                    }
                })
            })
        })
        .collect();

    // Try preferred position first
    if let Some(safe_y) = find_surface_y(voxel_world, preferred_pos.x, preferred_pos.z) {
        return Vec3::new(preferred_pos.x as f32, safe_y + 2.0, preferred_pos.z as f32);
    }

    // Use parallel search to find safe position faster
    search_positions
        .par_iter()
        .find_map_first(|pos| {
            find_surface_y(voxel_world, pos.x, pos.z)
                .map(|safe_y| Vec3::new(pos.x as f32, safe_y + 2.0, pos.z as f32))
        })
        .unwrap_or_else(|| Vec3::new(0.0, 10.0, 0.0))
}

/// Find the Y coordinate of the highest solid block at given X,Z coordinates
fn find_surface_y(voxel_world: &VoxelWorld, x: i32, z: i32) -> Option<f32> {
    for y in (-20..=20).rev() {
        let pos = IVec3::new(x, y, z);
        if let Some(block_type) = voxel_world.blocks.get(&pos) {
            if *block_type != BlockType::Water {
                return Some((y + 1) as f32);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::VoxelWorld;

    #[test]
    fn test_parallel_physics_performance() {
        // Create a large world for testing parallel performance
        let mut voxel_world = VoxelWorld::default();

        // Add many blocks to test parallel processing
        for x in -50..50 {
            for z in -50..50 {
                for y in 0..5 {
                    voxel_world.set_block(IVec3::new(x, y, z), BlockType::Stone);
                }
            }
        }

        // Test that we can find a safe spawn position efficiently
        let start_time = std::time::Instant::now();
        let safe_pos = find_safe_spawn_position(&voxel_world, IVec3::new(0, 0, 0));
        let elapsed = start_time.elapsed();

        assert!(safe_pos.y > 5.0); // Should be above the blocks
        assert!(elapsed.as_millis() < 100); // Should be fast even with many blocks
    }
}
