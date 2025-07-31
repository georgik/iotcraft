use crate::environment::BlockType;
use bevy::prelude::*;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_stack_add_and_remove() {
        let item_type = ItemType::Block(BlockType::Grass);
        let mut stack = ItemStack::new(item_type, 10);

        // Test adding
        assert_eq!(stack.add(5), 0); // Should successfully add all 5
        assert_eq!(stack.count, 15);
        assert_eq!(stack.add(60), 11); // Can only add 49 (64 max capacity)
        assert_eq!(stack.count, 64);

        // Test removing
        assert_eq!(stack.remove(10), 10); // Should successfully remove 10
        assert_eq!(stack.count, 54);
        assert_eq!(stack.remove(60), 54); // Can only remove 54
        assert!(stack.is_empty());
    }

    #[test]
    fn test_inventory_add_items() {
        let mut inventory = PlayerInventory::new();
        let item_type = ItemType::Block(BlockType::Stone);

        // Test adding items that fit completely
        assert_eq!(inventory.add_items(item_type, 64), 0); // Should add all 64 (fills 1 slot)
        assert_eq!(inventory.add_items(item_type, 100), 0); // Should add all 100 (fills 1 more slot + partial)

        // Test adding items that exceed inventory capacity
        // Inventory has 36 slots * 64 max per slot = 2304 total capacity
        // We've added 164 items, so 2304 - 164 = 2140 items can still fit
        let remaining_capacity = (36 * 64) - 164;
        let excess_items = remaining_capacity + 50; // Try to add 50 more than capacity
        assert_eq!(inventory.add_items(item_type, excess_items), 50); // Should return 50 excess

        // Test selecting item
        inventory.select_slot(0);
        assert!(inventory.get_selected_item().is_some());

        // Test clear selected slot
        inventory.clear_selected_item();
        assert!(inventory.get_selected_item().is_none());
    }

    #[test]
    fn test_inventory_select_slot() {
        let mut inventory = PlayerInventory::new();
        inventory.select_slot(5);
        assert_eq!(inventory.selected_slot, 5);
        inventory.select_slot(40); // Out of bounds, should not change
        assert_eq!(inventory.selected_slot, 5);
    }
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

    /// Get the currently selected item
    pub fn get_selected_item(&self) -> Option<&ItemStack> {
        self.slots.get(self.selected_slot)?.as_ref()
    }

    /// Get a mutable reference to the currently selected item
    pub fn get_selected_item_mut(&mut self) -> Option<&mut ItemStack> {
        self.slots.get_mut(self.selected_slot)?.as_mut()
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
}

/// Event for when player tries to break a block
#[derive(Event)]
pub struct BreakBlockEvent {
    pub position: IVec3,
}
