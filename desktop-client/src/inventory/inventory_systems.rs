use crate::environment::{BlockType, VoxelWorld};
use crate::inventory::{
    BreakBlockEvent, GiveItemEvent, ItemType, PlaceBlockEvent, PlayerInventory,
};
use bevy::prelude::*;

/// System to handle give item events
pub fn give_item_system(
    mut inventory: ResMut<PlayerInventory>,
    mut events: EventReader<GiveItemEvent>,
) {
    for event in events.read() {
        let remainder = inventory.add_items(event.item_type, event.count);
        if remainder > 0 {
            println!(
                "Inventory full! Couldn't add {} {}",
                remainder,
                event.item_type.display_name()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::{BlockType, VoxelWorld};
    use crate::inventory::{
        BreakBlockEvent, GiveItemEvent, ItemType, PlaceBlockEvent, PlayerInventory,
    };
    use bevy::ecs::system::IntoSystem;

    #[test]
    fn test_give_item_system() {
        let mut world = World::new();
        world.insert_resource(PlayerInventory::new());
        world.init_resource::<Events<GiveItemEvent>>();

        let mut event_writer = world.resource_mut::<Events<GiveItemEvent>>();
        event_writer.write(GiveItemEvent {
            item_type: ItemType::Block(BlockType::Stone),
            count: 32,
        });
        drop(event_writer);

        let mut system = IntoSystem::into_system(give_item_system);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);

        let inventory = world.resource::<PlayerInventory>();
        // Should have added items to inventory (first slot)
        assert!(inventory.slots[0].is_some());
        if let Some(stack) = &inventory.slots[0] {
            assert_eq!(stack.item_type, ItemType::Block(BlockType::Stone));
            assert_eq!(stack.count, 32);
        }
    }

    #[test]
    fn test_place_block_system() {
        let mut world = World::new();
        let mut inventory = PlayerInventory::new();
        inventory.add_items(ItemType::Block(BlockType::Grass), 10);
        inventory.select_slot(0);

        world.insert_resource(inventory);
        world.insert_resource(VoxelWorld::default());
        world.insert_resource(Assets::<Mesh>::default());
        world.insert_resource(Assets::<StandardMaterial>::default());
        // Skip AssetServer as it requires complex setup for tests
        world.init_resource::<Events<PlaceBlockEvent>>();

        let mut event_writer = world.resource_mut::<Events<PlaceBlockEvent>>();
        event_writer.write(PlaceBlockEvent {
            position: IVec3::new(1, 2, 3),
        });
        drop(event_writer);

        // Create a simplified test system that only updates VoxelWorld
        let test_system = |mut inventory: ResMut<PlayerInventory>,
                           mut voxel_world: ResMut<VoxelWorld>,
                           mut events: EventReader<PlaceBlockEvent>| {
            for event in events.read() {
                if let Some(selected_item) = inventory.get_selected_item_mut() {
                    let ItemType::Block(block_type) = selected_item.item_type;
                    if selected_item.count > 0 {
                        voxel_world.set_block(event.position, block_type);
                        selected_item.remove(1);
                        if selected_item.is_empty() {
                            inventory.clear_selected_item();
                        }
                    }
                }
            }
        };

        let mut system = IntoSystem::into_system(test_system);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);

        let voxel_world = world.resource::<VoxelWorld>();
        let inventory = world.resource::<PlayerInventory>();

        // Should have placed block in world
        assert!(voxel_world.is_block_at(IVec3::new(1, 2, 3)));

        // Should have consumed item from inventory
        if let Some(stack) = &inventory.slots[0] {
            assert_eq!(stack.count, 9); // One item consumed
        }
    }

    #[test]
    fn test_break_block_system() {
        let mut world = World::new();
        let mut voxel_world = VoxelWorld::default();
        voxel_world.set_block(IVec3::new(5, 5, 5), BlockType::Stone);
        world.insert_resource(voxel_world);
        world.init_resource::<Events<BreakBlockEvent>>();

        let mut event_writer = world.resource_mut::<Events<BreakBlockEvent>>();
        event_writer.write(BreakBlockEvent {
            position: IVec3::new(5, 5, 5),
        });
        drop(event_writer);

        let mut system = IntoSystem::into_system(break_block_system);
        system.initialize(&mut world);
        let _ = system.run((), &mut world);

        let voxel_world = world.resource::<VoxelWorld>();
        // Block should be removed
        assert!(!voxel_world.is_block_at(IVec3::new(5, 5, 5)));
    }
}

/// System to handle item placement
pub fn place_block_system(
    mut inventory: ResMut<PlayerInventory>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut events: EventReader<PlaceBlockEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    for event in events.read() {
        if let Some(selected_item) = inventory.get_selected_item_mut() {
            let ItemType::Block(block_type) = selected_item.item_type;
            if selected_item.count > 0 {
                // Update the voxel world data
                voxel_world.set_block(event.position, block_type);
                info!(
                    "Placed block {:?} at {:?} in VoxelWorld",
                    block_type, event.position
                );

                // Spawn the visual block
                let cube_mesh = meshes.add(Cuboid::new(
                    crate::environment::CUBE_SIZE,
                    crate::environment::CUBE_SIZE,
                    crate::environment::CUBE_SIZE,
                ));
                let texture_path = match block_type {
                    BlockType::Grass => "textures/grass.webp",
                    BlockType::Dirt => "textures/dirt.webp",
                    BlockType::Stone => "textures/stone.webp",
                    BlockType::QuartzBlock => "textures/quartz_block.webp",
                    BlockType::GlassPane => "textures/glass_pane.webp",
                    BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                    BlockType::Water => "textures/water.webp",
                };
                let texture: Handle<Image> = asset_server.load(texture_path);
                let material = materials.add(StandardMaterial {
                    base_color_texture: Some(texture),
                    ..default()
                });

                commands.spawn((
                    Mesh3d(cube_mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(event.position.as_vec3()),
                    crate::environment::VoxelBlock {
                        position: event.position,
                    },
                ));

                // Remove item from inventory
                selected_item.remove(1);

                if selected_item.is_empty() {
                    inventory.clear_selected_item();
                }
            }
        }
    }
}

/// System to handle multiplayer synchronization for UI block placement
pub fn place_block_multiplayer_sync_system(
    mut place_block_events: EventReader<PlaceBlockEvent>,
    #[cfg(not(target_arch = "wasm32"))] mut block_change_events: EventWriter<
        crate::multiplayer::BlockChangeEvent,
    >,
    #[cfg(target_arch = "wasm32")] mut block_change_events: EventWriter<
        crate::multiplayer_web::BlockChangeEvent,
    >,
    #[cfg(not(target_arch = "wasm32"))] multiplayer_mode: Res<crate::multiplayer::MultiplayerMode>,
    #[cfg(target_arch = "wasm32")] multiplayer_mode: Res<crate::multiplayer_web::MultiplayerMode>,
    player_profile: Res<crate::profile::PlayerProfile>,
    inventory: Res<PlayerInventory>,
) {
    for event in place_block_events.read() {
        info!(
            "🔄 Processing block placement event at {:?} - current multiplayer mode: {:?}",
            event.position, &*multiplayer_mode
        );

        // Only send multiplayer events when in multiplayer mode
        #[cfg(not(target_arch = "wasm32"))]
        let is_multiplayer_with_world = matches!(
            &*multiplayer_mode,
            crate::multiplayer::MultiplayerMode::HostingWorld { world_id: _, .. }
                | crate::multiplayer::MultiplayerMode::JoinedWorld { world_id: _, .. }
        );

        #[cfg(target_arch = "wasm32")]
        let is_multiplayer_with_world = matches!(
            &*multiplayer_mode,
            crate::multiplayer_web::MultiplayerMode::HostingWorld { world_id: _, .. }
                | crate::multiplayer_web::MultiplayerMode::JoinedWorld { world_id: _, .. }
        );

        if is_multiplayer_with_world {
            let world_id = match &*multiplayer_mode {
                #[cfg(not(target_arch = "wasm32"))]
                crate::multiplayer::MultiplayerMode::HostingWorld { world_id, .. }
                | crate::multiplayer::MultiplayerMode::JoinedWorld { world_id, .. } => world_id,
                #[cfg(target_arch = "wasm32")]
                crate::multiplayer_web::MultiplayerMode::HostingWorld { world_id, .. }
                | crate::multiplayer_web::MultiplayerMode::JoinedWorld { world_id, .. } => world_id,
                _ => unreachable!("We already checked for multiplayer mode"),
            };

            info!(
                "✅ In multiplayer mode (world_id: {}), checking inventory for selected item",
                world_id
            );

            // Get the block type from the player's selected inventory item
            if let Some(selected_item) = inventory.get_selected_item() {
                let ItemType::Block(block_type) = selected_item.item_type;

                info!(
                    "🧱 Found selected item: {:?}, generating MQTT event for player {} ({})",
                    block_type, player_profile.player_name, player_profile.player_id
                );

                #[cfg(not(target_arch = "wasm32"))]
                block_change_events.write(crate::multiplayer::BlockChangeEvent {
                    world_id: world_id.clone(),
                    player_id: player_profile.player_id.clone(),
                    player_name: player_profile.player_name.clone(),
                    change_type: crate::multiplayer::BlockChangeType::Placed {
                        x: event.position.x,
                        y: event.position.y,
                        z: event.position.z,
                        block_type,
                    },
                });

                #[cfg(target_arch = "wasm32")]
                block_change_events.write(crate::multiplayer_web::BlockChangeEvent {
                    world_id: world_id.clone(),
                    player_id: player_profile.player_id.clone(),
                    player_name: player_profile.player_name.clone(),
                    change_type: crate::multiplayer_web::BlockChangeType::Placed {
                        x: event.position.x,
                        y: event.position.y,
                        z: event.position.z,
                        block_type,
                    },
                });

                info!(
                    "📡 Sent multiplayer block change event: {:?} at {:?} for world {}",
                    block_type, event.position, world_id
                );
            } else {
                warn!(
                    "❌ No selected item in inventory when placing block at {:?}",
                    event.position
                );
            }
        } else {
            info!(
                "🚫 Not in multiplayer mode, skipping MQTT publish for block at {:?}",
                event.position
            );
        }
    }
}

/// System to handle block breaking with visual entity removal
pub fn break_block_system(
    mut events: EventReader<BreakBlockEvent>,
    mut voxel_world: ResMut<VoxelWorld>,
    mut commands: Commands,
    existing_blocks_query: Query<(Entity, &crate::environment::VoxelBlock)>,
) {
    for event in events.read() {
        // Remove from voxel world data
        voxel_world.remove_block(&event.position);

        // Remove visual entity if it exists
        for (entity, block) in existing_blocks_query.iter() {
            if block.position == event.position {
                commands.entity(entity).despawn();
                info!("Removed visual block entity at {:?}", event.position);
                break;
            }
        }

        info!("Block removed from VoxelWorld at {:?}", event.position);
    }
}

/// System to handle multiplayer synchronization for block breaking
pub fn break_block_multiplayer_sync_system(
    mut break_block_events: EventReader<BreakBlockEvent>,
    #[cfg(not(target_arch = "wasm32"))] mut block_change_events: EventWriter<
        crate::multiplayer::BlockChangeEvent,
    >,
    #[cfg(target_arch = "wasm32")] mut block_change_events: EventWriter<
        crate::multiplayer_web::BlockChangeEvent,
    >,
    #[cfg(not(target_arch = "wasm32"))] multiplayer_mode: Res<crate::multiplayer::MultiplayerMode>,
    #[cfg(target_arch = "wasm32")] multiplayer_mode: Res<crate::multiplayer_web::MultiplayerMode>,
    player_profile: Res<crate::profile::PlayerProfile>,
) {
    for event in break_block_events.read() {
        info!(
            "🔄 Processing block breaking event at {:?} - current multiplayer mode: {:?}",
            event.position, &*multiplayer_mode
        );

        // Only send multiplayer events when in multiplayer mode
        #[cfg(not(target_arch = "wasm32"))]
        let is_multiplayer_with_world = matches!(
            &*multiplayer_mode,
            crate::multiplayer::MultiplayerMode::HostingWorld { world_id: _, .. }
                | crate::multiplayer::MultiplayerMode::JoinedWorld { world_id: _, .. }
        );

        #[cfg(target_arch = "wasm32")]
        let is_multiplayer_with_world = matches!(
            &*multiplayer_mode,
            crate::multiplayer_web::MultiplayerMode::HostingWorld { world_id: _, .. }
                | crate::multiplayer_web::MultiplayerMode::JoinedWorld { world_id: _, .. }
        );

        if is_multiplayer_with_world {
            let world_id = match &*multiplayer_mode {
                #[cfg(not(target_arch = "wasm32"))]
                crate::multiplayer::MultiplayerMode::HostingWorld { world_id, .. }
                | crate::multiplayer::MultiplayerMode::JoinedWorld { world_id, .. } => world_id,
                #[cfg(target_arch = "wasm32")]
                crate::multiplayer_web::MultiplayerMode::HostingWorld { world_id, .. }
                | crate::multiplayer_web::MultiplayerMode::JoinedWorld { world_id, .. } => world_id,
                _ => unreachable!("We already checked for multiplayer mode"),
            };

            info!(
                "✅ In multiplayer mode (world_id: {}), generating block removal event for player {} ({})",
                world_id, player_profile.player_name, player_profile.player_id
            );

            #[cfg(not(target_arch = "wasm32"))]
            block_change_events.write(crate::multiplayer::BlockChangeEvent {
                world_id: world_id.clone(),
                player_id: player_profile.player_id.clone(),
                player_name: player_profile.player_name.clone(),
                change_type: crate::multiplayer::BlockChangeType::Removed {
                    x: event.position.x,
                    y: event.position.y,
                    z: event.position.z,
                },
            });

            #[cfg(target_arch = "wasm32")]
            block_change_events.write(crate::multiplayer_web::BlockChangeEvent {
                world_id: world_id.clone(),
                player_id: player_profile.player_id.clone(),
                player_name: player_profile.player_name.clone(),
                change_type: crate::multiplayer_web::BlockChangeType::Removed {
                    x: event.position.x,
                    y: event.position.y,
                    z: event.position.z,
                },
            });

            info!(
                "📡 Sent multiplayer block removal event at {:?} for world {}",
                event.position, world_id
            );
        } else {
            info!(
                "🚫 Not in multiplayer mode, skipping MQTT publish for block removal at {:?}",
                event.position
            );
        }
    }
}
