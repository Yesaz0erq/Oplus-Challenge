use bevy::prelude::*;

use crate::equipment::ItemId;

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
        Self { slots: vec![None; slot_count] }
    }

    pub fn slot_count(&self) -> usize { self.slots.len() }

    pub fn try_add(&mut self, id: ItemId, mut count: u32, max_stack: u32) -> u32 {
        // 1) 先叠加到已有 stack
        for slot in self.slots.iter_mut() {
            if let Some(s) = slot.as_mut() {
                if s.id == id && s.count < max_stack && count > 0 {
                    let can = (max_stack - s.count).min(count);
                    s.count += can;
                    count -= can;
                }
            }
        }
        // 2) 再塞进空格
        for slot in self.slots.iter_mut() {
            if slot.is_none() && count > 0 {
                let put = max_stack.min(count);
                *slot = Some(ItemStack { id, count: put });
                count -= put;
            }
        }
        count // 返回剩余放不下的数量
    }

    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a >= self.slots.len() || b >= self.slots.len() { return; }
        self.slots.swap(a, b);
    }
}

impl ItemId {
    pub fn display_name(self) -> &'static str {
        match self {
            ItemId::RustySword => "生锈短剑",
            ItemId::MagicWand => "法杖",
            ItemId::HunterBow => "猎弓",
        }
    }

    /// 返回 assets/ 下图标路径，例如 "items/rusty_sword.png"
    pub fn icon_path(self) -> &'static str {
        match self {
            ItemId::RustySword => "items/rusty_sword.png",
            ItemId::MagicWand => "items/magic_wand.png",
            ItemId::HunterBow => "items/hunter_bow.png",
        }
    }
}
