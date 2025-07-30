use crate::environment::BlockType;
use bevy::prelude::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Maximum number of items that can be held in one inventory slot
pub const MAX_STACK_SIZE: u32 = 64;

/// Maximum number of inventory slots
pub const INVENTORY_SIZE: usize = 36; // 9x4 like Minecraft

/// Represents an item that can be held in inventory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemType {
    Block(BlockType),
    // Future items like tools, etc. can be added here
}

impl ItemType {
    /// Get the display name for this item type
    pub fn display_name(&self) -> &'static str {
        match self {
            ItemType::Block(block_type) => match block_type {
                BlockType::Grass => "Grass Block",
                BlockType::Dirt => "Dirt Block",
                BlockType::Stone => "Stone Block",
                BlockType::QuartzBlock => "Quartz Block",
                BlockType::GlassPane => "Glass Pane",
                BlockType::CyanTerracotta => "Cyan Terracotta",
            },
        }
    }
}

/// Represents a stack of items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_type: ItemType,
    pub count: u32,
}

impl ItemStack {
    pub fn new(item_type: ItemType, count: u32) -> Self {
        Self {
            item_type,
            count: count.min(MAX_STACK_SIZE),
        }
    }

    /// Try to add more items to this stack, returns the amount that couldn't be added
    pub fn add(&mut self, count: u32) -> u32 {
        let can_add = MAX_STACK_SIZE - self.count;
        let to_add = count.min(can_add);
        self.count += to_add;
        count - to_add
    }

    /// Remove items from this stack, returns the amount actually removed
    pub fn remove(&mut self, count: u32) -> u32 {
        let to_remove = count.min(self.count);
        self.count -= to_remove;
        to_remove
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Player's inventory system
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInventory {
    pub slots: Vec<Option<ItemStack>>,
    pub selected_slot: usize,
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerInventory {
    pub fn new() -> Self {
        Self {
            slots: vec![None; INVENTORY_SIZE],
            selected_slot: 0,
        }
    }
    
    /// Ensure inventory has the correct number of slots
    pub fn ensure_proper_size(&mut self) {
        if self.slots.len() != INVENTORY_SIZE {
            self.slots.resize(INVENTORY_SIZE, None);
        }
    }

    /// Add items to inventory, returns the amount that couldn't be added
    pub fn add_items(&mut self, item_type: ItemType, count: u32) -> u32 {
        let mut remaining = count;

        // First, try to add to existing stacks of the same type
        for slot in &mut self.slots {
            if let Some(stack) = slot {
                if stack.item_type == item_type {
                    remaining = stack.add(remaining);
                    if remaining == 0 {
                        return 0;
                    }
                }
            }
        }

        // If there's still remaining, create new stacks in empty slots
        for slot in &mut self.slots {
            if slot.is_none() && remaining > 0 {
                let to_add = remaining.min(MAX_STACK_SIZE);
                *slot = Some(ItemStack::new(item_type, to_add));
                remaining -= to_add;
            }
        }

        remaining
    }

    /// Remove items from inventory, returns the amount actually removed
    pub fn remove_items(&mut self, item_type: ItemType, count: u32) -> u32 {
        let mut remaining = count;

        for slot in &mut self.slots {
            if let Some(stack) = slot {
                if stack.item_type == item_type {
                    let removed = stack.remove(remaining);
                    remaining -= removed;

                    if stack.is_empty() {
                        *slot = None;
                    }

                    if remaining == 0 {
                        break;
                    }
                }
            }
        }

        count - remaining
    }

    /// Get the currently selected item
    pub fn get_selected_item(&self) -> Option<&ItemStack> {
        self.slots.get(self.selected_slot)?.as_ref()
    }

    /// Get a mutable reference to the currently selected item
    pub fn get_selected_item_mut(&mut self) -> Option<&mut ItemStack> {
        self.slots.get_mut(self.selected_slot)?.as_mut()
    }

    /// Count total items of a specific type
    pub fn count_items(&self, item_type: ItemType) -> u32 {
        self.slots
            .iter()
            .filter_map(|slot| slot.as_ref())
            .filter(|stack| stack.item_type == item_type)
            .map(|stack| stack.count)
            .sum()
    }

    /// Select a different inventory slot
    pub fn select_slot(&mut self, slot: usize) {
        if slot < INVENTORY_SIZE {
            self.selected_slot = slot;
        }
    }

    /// Clear the currently selected item slot
    pub fn clear_selected_item(&mut self) {
        if self.selected_slot < self.slots.len() {
            self.slots[self.selected_slot] = None;
        }
    }
}

/// Event for giving items to the player
#[derive(Event)]
pub struct GiveItemEvent {
    pub item_type: ItemType,
    pub count: u32,
}

/// Event for when player tries to place a block
#[derive(Event)]
pub struct PlaceBlockEvent {
    pub position: IVec3,
    pub block_type: BlockType,
}

/// Event for when player tries to break a block
#[derive(Event)]
pub struct BreakBlockEvent {
    pub position: IVec3,
}
