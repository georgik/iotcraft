//! Remote Block Synchronization System
//!
//! This module handles incoming `BlockChangeEvent` events with `source: Remote` and
//! creates/removes visual block entities accordingly. This is the missing piece that
//! makes real-time block placement/removal visible to other players.

use crate::environment::{CUBE_SIZE, VoxelBlock, VoxelWorld};
use crate::multiplayer::{BlockChangeEvent, BlockChangeSource, BlockChangeType};
use bevy::prelude::*;

/// System to handle remote block changes from other players
/// Listens to BlockChangeEvent events with source: Remote and creates/removes visual entities
pub fn handle_remote_block_changes(
    mut commands: Commands,
    mut block_change_events: EventReader<BlockChangeEvent>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    existing_blocks: Query<(Entity, &VoxelBlock)>,
) {
    for event in block_change_events.read() {
        // Only handle remote block changes to prevent infinite loops
        match event.source {
            BlockChangeSource::Local => {
                // Skip local changes - they're already handled by inventory systems
                continue;
            }
            BlockChangeSource::Remote => {
                info!(
                    "🧱 Processing remote block change from {}: {:?}",
                    event.player_name, event.change_type
                );
            }
        }

        match &event.change_type {
            BlockChangeType::Placed {
                x,
                y,
                z,
                block_type,
            } => {
                let position = IVec3::new(*x, *y, *z);

                // Add block to voxel world data
                voxel_world.set_block(position, *block_type);

                // Create visual representation
                let cube_mesh = meshes.add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));

                let material = match block_type {
                    crate::environment::BlockType::Water => materials.add(StandardMaterial {
                        base_color: Color::srgba(0.0, 0.35, 0.9, 0.6),
                        alpha_mode: AlphaMode::Blend,
                        ..default()
                    }),
                    _ => {
                        let texture_path = match block_type {
                            crate::environment::BlockType::Grass => "textures/grass.webp",
                            crate::environment::BlockType::Dirt => "textures/dirt.webp",
                            crate::environment::BlockType::Stone => "textures/stone.webp",
                            crate::environment::BlockType::QuartzBlock => {
                                "textures/quartz_block.webp"
                            }
                            crate::environment::BlockType::GlassPane => "textures/glass_pane.webp",
                            crate::environment::BlockType::CyanTerracotta => {
                                "textures/cyan_terracotta.webp"
                            }
                            _ => "textures/dirt.webp", // fallback
                        };
                        let texture = asset_server.load(texture_path);
                        materials.add(StandardMaterial {
                            base_color_texture: Some(texture),
                            ..default()
                        })
                    }
                };

                commands.spawn((
                    Mesh3d(cube_mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(Vec3::new(*x as f32, *y as f32, *z as f32)),
                    VoxelBlock { position },
                    Name::new(format!("RemoteBlock-{}-{}-{}", x, y, z)),
                ));

                info!(
                    "✅ Placed remote block {:?} at ({}, {}, {}) from player {}",
                    block_type, x, y, z, event.player_name
                );
            }
            BlockChangeType::Removed { x, y, z } => {
                let position = IVec3::new(*x, *y, *z);

                // Remove from voxel world data
                voxel_world.remove_block(&position);

                // Remove visual representation by finding the entity at this position
                for (entity, block) in existing_blocks.iter() {
                    if block.position == position {
                        commands.entity(entity).despawn();
                        info!(
                            "🗑️ Despawned remote block entity at position ({}, {}, {})",
                            x, y, z
                        );
                        break;
                    }
                }

                info!(
                    "🗑️ Removed remote block at ({}, {}, {}) from player {}",
                    x, y, z, event.player_name
                );
            }
        }
    }
}
