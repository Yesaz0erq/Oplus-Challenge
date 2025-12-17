use bevy::prelude::*;

pub fn despawn_with_children(commands: &mut Commands, children_q: &Query<&Children>, entity: Entity) {
    if let Ok(children) = children_q.get(entity) {
        for &child in children.iter() {
            despawn_with_children(commands, children_q, child);
        }
    }
    commands.entity(entity).despawn();
}