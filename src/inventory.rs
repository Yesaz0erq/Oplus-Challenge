// src/inventory.rs
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
    /// 新建背包，slot_count 格
    pub fn new(slot_count: usize) -> Self {
        Self { slots: vec![None; slot_count] }
    }

    pub fn slot_count(&self) -> usize { self.slots.len() }

    /// 尝试按叠加和空格放下若干个物品，返回剩余未放下的数量
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

    /// 从背包里移除一件特定 ItemId（找到任意一个 count>0 的堆并减一），成功返回 true
    pub fn try_remove_one(&mut self, id: ItemId) -> bool {
        if let Some(i) = self.slots.iter().position(|s| s.map(|ss| ss.id == id && ss.count > 0).unwrap_or(false)) {
            if let Some(mut s) = self.slots[i] {
                s.count -= 1;
                if s.count == 0 {
                    self.slots[i] = None;
                } else {
                    self.slots[i] = Some(s);
                }
                return true;
            }
        }
        false
    }

    /// 直接按索引交换两个格子
    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a >= self.slots.len() || b >= self.slots.len() { return; }
        self.slots.swap(a, b);
    }
}
