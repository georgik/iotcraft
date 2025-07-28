use crate::environment::BlockType;
use crate::inventory::{ItemType, PlayerInventory};
use bevy::prelude::*;

pub struct InventoryUiPlugin;

impl Plugin for InventoryUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_inventory_ui)
            .add_systems(Update, update_inventory_ui);
    }
}

#[derive(Component)]
struct InventoryUiRoot;

/// Initializes the inventory UI
fn setup_inventory_ui(mut commands: Commands) {
    // Add a root node as a container with a background
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            bottom: Val::Px(10.0),
            width: Val::Percent(100.0),
            height: Val::Px(70.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Row,
            ..default()
        },
        BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
        InventoryUiRoot,
    ));
}

/// Updates the inventory UI to show the current items
fn update_inventory_ui(
    inventory: Res<PlayerInventory>,
    ui_root_query: Query<Entity, With<InventoryUiRoot>>,
    children_query: Query<&Children>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if inventory.is_changed() {
        if let Ok(ui_root_entity) = ui_root_query.single() {
            // Clear existing children
            if let Ok(children) = children_query.get(ui_root_entity) {
                for child in children.iter() {
                    commands.entity(child).despawn();
                }
            }

            // Add inventory items as children
            commands.entity(ui_root_entity).with_children(|parent| {
                // Show only the first 9 slots for a simple hotbar
                for (index, slot) in inventory.slots.iter().take(9).enumerate() {
                    let is_selected = index == inventory.selected_slot;
                    let slot_color = if is_selected {
                        Color::srgba(1.0, 1.0, 0.0, 0.8) // Yellow for selected
                    } else {
                        Color::srgba(0.5, 0.5, 0.5, 0.8) // Gray for others
                    };

                    // Create slot with item if available
                    if let Some(item_stack) = slot {
                        let texture_path = match &item_stack.item_type {
                            ItemType::Block(block_type) => match block_type {
                                BlockType::Grass => "textures/grass.webp",
                                BlockType::Dirt => "textures/dirt.webp",
                                BlockType::Stone => "textures/stone.webp",
                                BlockType::QuartzBlock => "textures/quartz_block.webp",
                                BlockType::GlassPane => "textures/glass_pane.webp",
                                BlockType::CyanTerracotta => "textures/cyan_terracotta.webp",
                            },
                        };

                        // Slot with item image
                        parent
                            .spawn((
                                Node {
                                    width: Val::Px(60.0),
                                    height: Val::Px(60.0),
                                    margin: UiRect::all(Val::Px(2.0)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(slot_color),
                            ))
                            .with_children(|slot_parent| {
                                // Item image
                                slot_parent.spawn((
                                    ImageNode::new(asset_server.load(texture_path)),
                                    Node {
                                        width: Val::Px(50.0),
                                        height: Val::Px(50.0),
                                        ..default()
                                    },
                                ));

                                // Item count text (if more than 1)
                                if item_stack.count > 1 {
                                    slot_parent.spawn((
                                        Text::new(item_stack.count.to_string()),
                                        TextColor(Color::WHITE),
                                        Node {
                                            position_type: PositionType::Absolute,
                                            right: Val::Px(5.0),
                                            bottom: Val::Px(5.0),
                                            ..default()
                                        },
                                    ));
                                }
                            });
                    } else {
                        // Empty slot
                        parent.spawn((
                            Node {
                                width: Val::Px(60.0),
                                height: Val::Px(60.0),
                                margin: UiRect::all(Val::Px(2.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(slot_color),
                        ));
                    }
                }
            });
        }
    }
}
