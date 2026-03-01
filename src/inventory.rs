use crate::equipment::ItemId;
use bevy::prelude::*;

pub const INVENTORY_PAGE_SLOT_COUNT: usize = 24;

#[derive(Clone, Copy, Debug)]
pub struct ItemStack {
    pub id: ItemId,
    pub count: u32,
}

#[derive(Component)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
}

impl Inventory {
    pub fn new(slot_count: usize) -> Self {
        Self {
            slots: vec![None; slot_count],
        }
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub fn try_add(&mut self, id: ItemId, mut count: u32, max_stack: u32) -> u32 {
        for slot in self.slots.iter_mut() {
            if let Some(s) = slot.as_mut() {
                if s.id == id && s.count < max_stack && count > 0 {
                    let can = (max_stack - s.count).min(count);
                    s.count += can;
                    count -= can;
                }
            }
        }

        while count > 0 {
            let mut inserted_any = false;

            for slot in self.slots.iter_mut() {
                if slot.is_none() && count > 0 {
                    let put = max_stack.min(count);
                    *slot = Some(ItemStack { id, count: put });
                    count -= put;
                    inserted_any = true;
                }
            }

            if count == 0 {
                break;
            }

            if !inserted_any {
                // Auto-expand by one UI page when all current pages are full.
                self.slots
                    .extend(std::iter::repeat_n(None, INVENTORY_PAGE_SLOT_COUNT));
            }
        }

        count
    }

    pub fn try_remove_one(&mut self, id: ItemId) -> bool {
        for slot in self.slots.iter_mut() {
            if let Some(s) = slot {
                if s.id == id && s.count > 0 {
                    s.count -= 1;
                    if s.count == 0 {
                        *slot = None;
                    }
                    return true;
                }
            }
        }
        false
    }

    #[allow(dead_code)]
    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a >= self.slots.len() || b >= self.slots.len() {
            return;
        }
        self.slots.swap(a, b);
    }
}
