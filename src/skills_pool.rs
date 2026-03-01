use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkillId {
    Dash,
    Slash,
    Fireball,
    LightWave,
}

impl SkillId {
    pub const fn to_u32(self) -> u32 {
        match self {
            SkillId::Dash => 1,
            SkillId::Slash => 2,
            SkillId::Fireball => 3,
            SkillId::LightWave => 4,
        }
    }

    pub const fn from_u32(v: u32) -> Option<Self> {
        match v {
            1 => Some(SkillId::Dash),
            2 => Some(SkillId::Slash),
            3 => Some(SkillId::Fireball),
            4 => Some(SkillId::LightWave),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SkillRarity {
    Common,
}

#[derive(Clone, Copy, Debug)]
pub struct SkillDef {
    pub id: SkillId,
    pub damage: f32,
    pub cooldown: f32,
    pub rarity: SkillRarity,
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
            SkillId::Dash => SkillDef {
                id,
                damage: 0.0,
                cooldown: 3.0,
                rarity: SkillRarity::Common,
            },
            SkillId::Slash => SkillDef {
                id,
                damage: 60.0,
                cooldown: 6.0,
                rarity: SkillRarity::Common,
            },
            SkillId::Fireball => SkillDef {
                id,
                damage: 42.0,
                cooldown: 5.5,
                rarity: SkillRarity::Common,
            },
            SkillId::LightWave => SkillDef {
                id,
                damage: 32.0,
                cooldown: 8.0,
                rarity: SkillRarity::Common,
            },
        }
    }

    pub fn next_non_dash(&mut self) -> SkillId {
        let list = [SkillId::Slash, SkillId::Fireball, SkillId::LightWave];
        let id = list[self.next_other % list.len()];
        self.next_other = self.next_other.wrapping_add(1);
        id
    }
}

pub struct SkillPoolPlugin;

impl Plugin for SkillPoolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkillPool>()
            .add_systems(Startup, validate_skill_defs);
    }
}

fn validate_skill_defs(pool: Res<SkillPool>) {
    for id in [
        SkillId::Dash,
        SkillId::Slash,
        SkillId::Fireball,
        SkillId::LightWave,
    ] {
        let def = pool.def(id);
        let _ = def.id;
    }
}
