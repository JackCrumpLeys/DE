use bevy::prelude::*;
use de_core::baseset::GameSet;
use de_core::gamestate::GameState;
use de_core::objects::{MovableSolid, StaticSolid};
use de_core::projection::ToFlat;
use kiddo::float::distance::Manhattan;
use kiddo::float::kdtree::KdTree;
use std::collections::HashMap;
use std::time::Instant;

pub(crate) struct KdTreePlugin;

impl Plugin for KdTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup.in_schedule(OnEnter(GameState::Playing)))
            .add_system(
                insert
                    .in_base_set(GameSet::PreUpdate)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_system(
                update
                    .in_base_set(GameSet::PreUpdate)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

#[derive(Debug, Resource)]
pub struct EntityKdTree {
    tree: KdTree<f32, u64, 2, 512, u32>,
    entity_to_last_loc: HashMap<Entity, ([f32; 2])>,
}

impl Default for EntityKdTree {
    fn default() -> Self {
        Self {
            tree: KdTree::new(),
            entity_to_last_loc: HashMap::new(),
        }
    }
}

impl EntityKdTree {
    /// Returns the entities within a given radius of a point.
    /// The distance is the Manhattan distance. (Not accurate, but fast and correctly ordered)
    pub fn radius(&self, point: &[f32; 2], radius: f32) -> Vec<(f32, Entity)> {
        self
            .tree
            .within::<Manhattan>(point, radius)
            .iter()
            .map(|nn| (nn.distance, Entity::from_bits(nn.item)))
            .collect()
    }
}

#[derive(Component, Debug, Clone, Copy)]
struct TrackedByKdTree;

/// This system iterates over all not yet TrackedByKdTree entities and adds them.
fn insert(
    mut commands: Commands,
    mut tree: ResMut<EntityKdTree>,
    query: Query<
        (Entity, &Transform),
        (
            Without<TrackedByKdTree>,
            Or<(With<StaticSolid>, With<MovableSolid>)>,
        ),
    >,
) {
    for (entity, transform) in query.iter() {
        if !tree.entity_to_last_loc.contains_key(&entity) {
            let translation = *transform.translation.to_flat().as_ref();
            tree.entity_to_last_loc.insert(entity, translation);
            tree.tree.add(&translation, entity.to_bits());
            commands.entity(entity).insert(TrackedByKdTree);
        }
    }
}

fn setup(mut commands: Commands) {
    commands.insert_resource(EntityKdTree::default());
}

fn update(
    mut entity_kd_tree: ResMut<EntityKdTree>,
    query: Query<(Entity, &Transform), (With<TrackedByKdTree>, Changed<Transform>)>,
) {
    let time = Instant::now();
    for (entity, transform) in query.iter() {
        let last_known_coords = *entity_kd_tree.entity_to_last_loc.get(&entity).unwrap();
        let coords = *transform.translation.to_flat().as_ref();
        if last_known_coords == coords {
            continue;
        }

        entity_kd_tree
            .tree
            .remove(&last_known_coords, entity.to_bits());
        entity_kd_tree
            .tree
            .add(&coords, entity.to_bits());

        *entity_kd_tree.entity_to_last_loc.get_mut(&entity).unwrap() =
            coords;
    }
    // println!("Update took: {:?}", time.elapsed());
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiddo::distance_metric::DistanceMetric;

    #[test]
    fn test() {
        let mut tree = EntityKdTree::default();

        tree.tree.add(&[1.0, 2.0], Entity::from_raw(1).to_bits());
        tree.tree.add(&[2.0, 1.0], Entity::from_raw(2).to_bits());
        tree.tree.add(&[3.0, 2.0], Entity::from_raw(3).to_bits());
        tree.tree.add(&[4.0, 1.0], Entity::from_raw(4).to_bits());
        tree.tree.add(&[5.0, 2.0], Entity::from_raw(5).to_bits());
        tree.tree.add(&[6.0, 1.0], Entity::from_raw(6).to_bits());
        dbg!(Manhattan::dist(&[0.0, 0.0], &[1.0, 2.0]));

        let result = dbg!(tree.radius(&[0.0, 0.0], 5.0, 2));
        assert_eq!(
            result,
            vec![(3.0, Entity::from_raw(2)), (3.0, Entity::from_raw(1))]
        );

        println!("{:?}", result);
    }
}
