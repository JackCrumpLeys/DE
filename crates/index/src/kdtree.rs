use bevy::prelude::*;
use de_core::baseset::GameSet;
use de_core::gamestate::GameState;
use de_core::objects::{MovableSolid, StaticSolid};
use kiddo::float::distance::SquaredEuclidean;
use kiddo::float::kdtree::KdTree;
use std::collections::HashMap;
use de_core::projection::ToFlat;

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
    tree: KdTree<f32, u64, 2, 100_000, u32>,
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
    pub fn radius(&self, point: &[f32; 2], radius: f32) -> Vec<(f32, Entity)> {
        dbg!(self.tree
            .within::<SquaredEuclidean>(point, radius)
            .iter()
            .map(|nn| (nn.distance, Entity::from_bits(nn.item)))
            .collect())
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
    for (entity, transform) in query.iter() {
        let last_known_coords = *entity_kd_tree.entity_to_last_loc.get(&entity).unwrap();

        entity_kd_tree.tree.remove(&last_known_coords, entity.to_bits());
        entity_kd_tree.tree.add(
            transform.translation.to_flat().as_ref(),
            entity.to_bits(),
        );

        *entity_kd_tree.entity_to_last_loc.get_mut(&entity).unwrap() = *transform.translation.to_flat().as_ref();
    }
}
