use crate::equipment::ItemId;
use bevy::prelude::*;

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
    /// 创建容量为 slot_count 的背包
    pub fn new(slot_count: usize) -> Self {
        Self {
            slots: vec![None; slot_count],
        }
    }

    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// 尝试把 count 个物品放进背包（会优先叠加），返回剩余放不下的数量
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

        for slot in self.slots.iter_mut() {
            if slot.is_none() && count > 0 {
                let put = max_stack.min(count);
                *slot = Some(ItemStack { id, count: put });
                count -= put;
            }
        }

        count
    }

    /// 从背包中移除一个指定 ItemId（找到任意一个计数>0 的堆并减一）
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

    /// 交换两个索引
    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a >= self.slots.len() || b >= self.slots.len() {
            return;
        }
        self.slots.swap(a, b);
    }
}
