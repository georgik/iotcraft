// Inventory system implementations using parameter bundles
// Refactored from various files to use parameter bundles for Bevy compliance

use bevy::prelude::*;
use log::info;

use crate::environment::{BlockType, CUBE_SIZE, VoxelBlock};
use crate::inventory::ItemType;

use super::inventory_params::{
    BlockBreakingParams, BlockPlacementParams, BlockVisualsParams,
    ComprehensiveBlockPlacementParams, ConsoleAwareInventoryInputParams, InventoryInputParams,
    ItemGivingParams, MultiplayerBlockBreakingSyncParams, MultiplayerBlockSyncParams,
};

/// System to handle inventory input (console feature enabled version)
#[cfg(feature = "console")]
pub fn handle_inventory_input_bundled(mut params: ConsoleAwareInventoryInputParams) {
    // Don't handle input when console is open or in any menu state
    let console_open = params
        .console_manager
        .as_ref()
        .map(|manager| manager.console.is_visible())
        .unwrap_or(false);

    #[cfg(not(target_arch = "wasm32"))]
    let in_game_state =
        *params.base_params.game_state.get() == crate::ui::main_menu::GameState::InGame;
    #[cfg(target_arch = "wasm32")]
    let in_game_state = *params.base_params.game_state.get() == crate::ui::GameState::InGame;

    if console_open || !in_game_state {
        return;
    }

    handle_inventory_input_core(&mut params.base_params);
}

/// System to handle inventory input (console feature disabled version)
#[cfg(not(feature = "console"))]
pub fn handle_inventory_input_bundled(mut params: InventoryInputParams) {
    // Don't handle input when in any menu state (console not available to check)
    #[cfg(not(target_arch = "wasm32"))]
    let in_game_state = *params.game_state.get() == crate::ui::main_menu::GameState::InGame;
    #[cfg(target_arch = "wasm32")]
    let in_game_state = *params.game_state.get() == crate::ui::GameState::InGame;

    if !in_game_state {
        return;
    }

    handle_inventory_input_core(&mut params);
}

/// Core inventory input handling logic (shared between console and non-console versions)
fn handle_inventory_input_core(params: &mut InventoryInputParams) {
    // Handle mouse wheel for inventory slot switching
    if params.accumulated_mouse_scroll.delta.y != 0.0 {
        let current_slot = params.inventory.selected_slot;
        let new_slot = if params.accumulated_mouse_scroll.delta.y > 0.0 {
            // Scroll up - previous slot (wraps around)
            if current_slot == 0 {
                8
            } else {
                current_slot - 1
            }
        } else {
            // Scroll down - next slot (wraps around)
            if current_slot == 8 {
                0
            } else {
                current_slot + 1
            }
        };

        if new_slot != current_slot {
            params.inventory.select_slot(new_slot);
            info!("Selected inventory slot {}", new_slot + 1);
        }
    }

    // Handle number keys 1-9 for slot selection
    let key_mappings = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ];

    for (key, slot) in key_mappings {
        if params.keyboard_input.just_pressed(key) {
            params.inventory.select_slot(slot);
            info!("Selected inventory slot {}", slot + 1);
            break;
        }
    }
}

/// System to handle giving items to inventory using parameter bundles
pub fn give_item_system_bundled(mut params: ItemGivingParams) {
    for event in params.give_events.read() {
        let remainder = params.inventory.add_items(event.item_type, event.count);
        if remainder > 0 {
            info!(
                "Inventory full! Couldn't add {} {}",
                remainder,
                event.item_type.display_name()
            );
        }
    }
}

/// System to handle block placement using parameter bundles (placement only)
pub fn place_block_system_bundled(
    mut placement_params: BlockPlacementParams,
    mut visuals_params: BlockVisualsParams,
) {
    for event in placement_params.place_events.read() {
        if let Some(selected_item) = placement_params.inventory.get_selected_item_mut() {
            let ItemType::Block(block_type) = selected_item.item_type;
            if selected_item.count > 0 {
                // Update the voxel world data
                placement_params
                    .voxel_world
                    .set_block(event.position, block_type);
                info!(
                    "Placed block {:?} at {:?} in VoxelWorld",
                    block_type, event.position
                );

                // Spawn the visual block
                let cube_mesh = visuals_params
                    .meshes
                    .add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
                let texture_path = match block_type {
                    BlockType::Grass => "textures/grass.webp",
                    BlockType::Dirt => "textures/dirt.webp",
                    BlockType::Stone => "textures/stone.webp",
                    BlockType::QuartzBlock => "textures/quartz_block.webp",
                    BlockType::GlassPane => "textures/glass_pane.webp",
                    BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                    BlockType::Water => "textures/water.webp",
                };
                let texture: Handle<Image> = visuals_params.asset_server.load(texture_path);
                let material = visuals_params.materials.add(StandardMaterial {
                    base_color_texture: Some(texture),
                    ..default()
                });

                placement_params.commands.spawn((
                    Mesh3d(cube_mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(event.position.as_vec3()),
                    VoxelBlock {
                        position: event.position,
                    },
                ));

                // Remove item from inventory
                selected_item.remove(1);

                if selected_item.is_empty() {
                    placement_params.inventory.clear_selected_item();
                }
            }
        }
    }
}

/// System to handle comprehensive block placement (includes multiplayer sync)
pub fn comprehensive_place_block_system_bundled(mut params: ComprehensiveBlockPlacementParams) {
    // First handle the placement and visual creation
    for event in params.placement.place_events.read() {
        if let Some(selected_item) = params.placement.inventory.get_selected_item_mut() {
            let ItemType::Block(block_type) = selected_item.item_type;
            if selected_item.count > 0 {
                // Update the voxel world data
                params
                    .placement
                    .voxel_world
                    .set_block(event.position, block_type);
                info!(
                    "Placed block {:?} at {:?} in VoxelWorld",
                    block_type, event.position
                );

                // Spawn the visual block
                let cube_mesh = params
                    .visuals
                    .meshes
                    .add(Cuboid::new(CUBE_SIZE, CUBE_SIZE, CUBE_SIZE));
                let texture_path = match block_type {
                    BlockType::Grass => "textures/grass.webp",
                    BlockType::Dirt => "textures/dirt.webp",
                    BlockType::Stone => "textures/stone.webp",
                    BlockType::QuartzBlock => "textures/quartz_block.webp",
                    BlockType::GlassPane => "textures/glass_pane.webp",
                    BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                    BlockType::Water => "textures/water.webp",
                };
                let texture: Handle<Image> = params.visuals.asset_server.load(texture_path);
                let material = params.visuals.materials.add(StandardMaterial {
                    base_color_texture: Some(texture),
                    ..default()
                });

                params.placement.commands.spawn((
                    Mesh3d(cube_mesh),
                    MeshMaterial3d(material),
                    Transform::from_translation(event.position.as_vec3()),
                    VoxelBlock {
                        position: event.position,
                    },
                ));

                // Remove item from inventory
                selected_item.remove(1);

                if selected_item.is_empty() {
                    params.placement.inventory.clear_selected_item();
                }
            }
        }
    }

    // Then handle multiplayer sync
    handle_multiplayer_block_sync(&mut params.sync);
}

/// System to handle multiplayer block synchronization using parameter bundles
pub fn place_block_multiplayer_sync_system_bundled(mut params: MultiplayerBlockSyncParams) {
    handle_multiplayer_block_sync(&mut params);
}

/// Core multiplayer sync logic (shared between comprehensive and standalone systems)
fn handle_multiplayer_block_sync(params: &mut MultiplayerBlockSyncParams) {
    for event in params.place_events.read() {
        info!(
            "🔄 Processing block placement event at {:?} - current multiplayer mode: {:?}",
            event.position, &*params.multiplayer_mode
        );

        // Only send multiplayer events when in multiplayer mode
        #[cfg(not(target_arch = "wasm32"))]
        let is_multiplayer_with_world = matches!(
            &*params.multiplayer_mode,
            crate::multiplayer::MultiplayerMode::HostingWorld { world_id: _, .. }
                | crate::multiplayer::MultiplayerMode::JoinedWorld { world_id: _, .. }
        );

        #[cfg(target_arch = "wasm32")]
        let is_multiplayer_with_world = matches!(
            &*params.multiplayer_mode,
            crate::multiplayer_web::MultiplayerMode::HostingWorld { world_id: _, .. }
                | crate::multiplayer_web::MultiplayerMode::JoinedWorld { world_id: _, .. }
        );

        if is_multiplayer_with_world {
            let world_id = match &*params.multiplayer_mode {
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
            if let Some(selected_item) = params.inventory.get_selected_item() {
                let ItemType::Block(block_type) = selected_item.item_type;

                info!(
                    "🧱 Found selected item: {:?}, generating MQTT event for player {} ({})",
                    block_type, params.player_profile.player_name, params.player_profile.player_id
                );

                #[cfg(not(target_arch = "wasm32"))]
                params
                    .block_change_events
                    .write(crate::multiplayer::BlockChangeEvent {
                        world_id: world_id.clone(),
                        player_id: params.player_profile.player_id.clone(),
                        player_name: params.player_profile.player_name.clone(),
                        change_type: crate::multiplayer::BlockChangeType::Placed {
                            x: event.position.x,
                            y: event.position.y,
                            z: event.position.z,
                            block_type,
                        },
                        source: crate::multiplayer::BlockChangeSource::Local,
                    });

                #[cfg(target_arch = "wasm32")]
                params
                    .block_change_events
                    .write(crate::multiplayer_web::BlockChangeEvent {
                        world_id: world_id.clone(),
                        player_id: params.player_profile.player_id.clone(),
                        player_name: params.player_profile.player_name.clone(),
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
                log::warn!(
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

/// System to handle block breaking using parameter bundles with visual entity removal
pub fn break_block_system_bundled(mut params: BlockBreakingParams) {
    for event in params.break_events.read() {
        // Remove from voxel world data
        params.voxel_world.remove_block(&event.position);

        // Remove visual entity if it exists
        for (entity, block) in params.existing_blocks_query.iter() {
            if block.position == event.position {
                params.commands.entity(entity).despawn();
                info!("Removed visual block entity at {:?}", event.position);
                break;
            }
        }

        info!("Block removed from VoxelWorld at {:?}", event.position);
    }
}

/// System to handle multiplayer block breaking synchronization using parameter bundles
pub fn break_block_multiplayer_sync_system_bundled(mut params: MultiplayerBlockBreakingSyncParams) {
    handle_multiplayer_block_breaking_sync(&mut params);
}

/// Core multiplayer block breaking sync logic (shared between comprehensive and standalone systems)
fn handle_multiplayer_block_breaking_sync(params: &mut MultiplayerBlockBreakingSyncParams) {
    for event in params.break_events.read() {
        info!(
            "🔄 Processing block breaking event at {:?} - current multiplayer mode: {:?}",
            event.position, &*params.multiplayer_mode
        );

        // Only send multiplayer events when in multiplayer mode
        #[cfg(not(target_arch = "wasm32"))]
        let is_multiplayer_with_world = matches!(
            &*params.multiplayer_mode,
            crate::multiplayer::MultiplayerMode::HostingWorld { world_id: _, .. }
                | crate::multiplayer::MultiplayerMode::JoinedWorld { world_id: _, .. }
        );

        #[cfg(target_arch = "wasm32")]
        let is_multiplayer_with_world = matches!(
            &*params.multiplayer_mode,
            crate::multiplayer_web::MultiplayerMode::HostingWorld { world_id: _, .. }
                | crate::multiplayer_web::MultiplayerMode::JoinedWorld { world_id: _, .. }
        );

        if is_multiplayer_with_world {
            let world_id = match &*params.multiplayer_mode {
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
                world_id, params.player_profile.player_name, params.player_profile.player_id
            );

            #[cfg(not(target_arch = "wasm32"))]
            params
                .block_change_events
                .write(crate::multiplayer::BlockChangeEvent {
                    world_id: world_id.clone(),
                    player_id: params.player_profile.player_id.clone(),
                    player_name: params.player_profile.player_name.clone(),
                    change_type: crate::multiplayer::BlockChangeType::Removed {
                        x: event.position.x,
                        y: event.position.y,
                        z: event.position.z,
                    },
                    source: crate::multiplayer::BlockChangeSource::Local,
                });

            #[cfg(target_arch = "wasm32")]
            params
                .block_change_events
                .write(crate::multiplayer_web::BlockChangeEvent {
                    world_id: world_id.clone(),
                    player_id: params.player_profile.player_id.clone(),
                    player_name: params.player_profile.player_name.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{MinimalPlugins, app::App};

    #[test]
    fn test_give_item_system_bundled_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_systems(Update, give_item_system_bundled);
        // Compilation test only
    }

    #[test]
    fn test_place_block_system_bundled_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_systems(Update, place_block_system_bundled);
        // Compilation test only
    }

    #[test]
    fn test_break_block_system_bundled_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_systems(Update, break_block_system_bundled);
        // Compilation test only
    }

    #[test]
    fn test_handle_inventory_input_bundled_compiles() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        app.add_systems(Update, handle_inventory_input_bundled);
        // Compilation test only
    }
}
