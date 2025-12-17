use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkillId {
    Dash,
    Slash,
}

#[derive(Clone, Copy, Debug)]
pub struct SkillDef {
    pub id: SkillId,
    pub name: &'static str,
    pub cooldown: f32,
}

#[derive(Resource, Debug)]
pub struct SkillPool {
    next_other: usize,
}

impl Default for SkillPool {
    fn default() -> Self {
        Self { next_other: 0 }
    }
}

impl SkillPool {
    pub fn def(&self, id: SkillId) -> SkillDef {
        match id {
            SkillId::Dash => SkillDef { id, name: "Dash", cooldown: 3.0 },
            SkillId::Slash => SkillDef { id, name: "Slash", cooldown: 6.0 },
        }
    }

    pub fn next_non_dash(&mut self) -> SkillId {
        let list = [SkillId::Slash];
        let id = list[self.next_other % list.len()];
        self.next_other = self.next_other.wrapping_add(1);
        id
    }
}

pub struct SkillPoolPlugin;

impl Plugin for SkillPoolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkillPool>();
    }
}