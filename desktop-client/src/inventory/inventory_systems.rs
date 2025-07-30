use crate::console::GiveCommand;
use crate::environment::{BlockType, VoxelWorld};
use crate::inventory::{
    BreakBlockEvent, GiveItemEvent, ItemType, PlaceBlockEvent, PlayerInventory,
};
use bevy::prelude::*;
use bevy_console::{ConsoleCommand, reply};

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
            if let ItemType::Block(block_type) = selected_item.item_type {
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
                            block_type,
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
}

/// System to handle block breaking
pub fn break_block_system(
    mut events: EventReader<BreakBlockEvent>,
    mut voxel_world: ResMut<VoxelWorld>,
) {
    for event in events.read() {
        voxel_world.remove_block(&event.position);
    }
}

/// Console command handler for giving items to player
pub fn handle_give_command(
    mut log: ConsoleCommand<GiveCommand>,
    mut inventory: ResMut<PlayerInventory>,
) {
    if let Some(Ok(GiveCommand { item_type, count })) = log.take() {
        let block_type = match item_type.as_str() {
            "grass" => BlockType::Grass,
            "dirt" => BlockType::Dirt,
            "stone" => BlockType::Stone,
            "quartz_block" => BlockType::QuartzBlock,
            "glass_pane" => BlockType::GlassPane,
            "cyan_terracotta" => BlockType::CyanTerracotta,
            _ => {
                reply!(log, "Invalid item type: {}", item_type);
                return;
            }
        };

        let item_type = ItemType::Block(block_type);
        let remainder = inventory.add_items(item_type, count);

        if remainder == 0 {
            reply!(log, "Gave {} {} to player", count, item_type.display_name());
        } else {
            let given = count - remainder;
            reply!(
                log,
                "Gave {} {} to player ({} couldn't fit in inventory)",
                given,
                item_type.display_name(),
                remainder
            );
        }
    }
}
